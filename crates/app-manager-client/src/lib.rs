//! Typed outbound App Manager Desktop Agent client.
//!
//! P8 deliberately exposes no generic remote-execution primitive. The client
//! accepts only the closed command enum declared in this module and talks to
//! the control plane through HTTPS initiated by the Windows desktop process.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(target_os = "windows")]
use std::{
    fs,
    io::Write,
    path::PathBuf,
    process::{Command, Stdio},
};

pub const APP_ID: &str = "pc-manager";
pub const PLATFORM: &str = "windows";
pub const DEVICE_TYPE: &str = "desktop-native";
pub const AGENT_PROTOCOL: &str = "pc-manager-agent/v1";
const DEFAULT_HEARTBEAT_SECONDS: u64 = 60;
const MAX_BACKOFF_SECONDS: u64 = 300;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AgentCommandType {
    CheckUpdate,
    RunHealthScan,
    RefreshDeviceStatus,
    DisableLicense,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentCommand {
    pub schema: String,
    pub command_id: String,
    #[serde(rename = "type")]
    pub command_type: AgentCommandType,
    pub issued_at: String,
    pub expires_at: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppManagerState {
    pub configured: bool,
    pub online: bool,
    pub endpoint: Option<String>,
    pub device_id: Option<String>,
    pub device_code: Option<String>,
    pub approval_state: String,
    pub app_version: String,
    pub release_channel: String,
    pub entitlement_state: String,
    pub update_policy: String,
    pub last_success_epoch_ms: Option<u64>,
    pub retry_after_seconds: u64,
    pub message: String,
}

impl AppManagerState {
    #[must_use]
    pub fn unconfigured() -> Self {
        Self {
            configured: false,
            online: false,
            endpoint: None,
            device_id: None,
            device_code: None,
            approval_state: "unconfigured".to_string(),
            app_version: env!("CARGO_PKG_VERSION").to_string(),
            release_channel: release_channel(),
            entitlement_state: "unknown".to_string(),
            update_policy: "unknown".to_string(),
            last_success_epoch_ms: None,
            retry_after_seconds: DEFAULT_HEARTBEAT_SECONDS,
            message: "App Manager endpoint is not configured.".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncBatch {
    pub state: AppManagerState,
    pub commands: Vec<AgentCommand>,
    pub heartbeat_interval_seconds: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppManagerError {
    pub code: String,
    pub message: String,
    pub recoverable: bool,
}

impl AppManagerError {
    #[must_use]
    pub fn new(code: impl Into<String>, message: impl Into<String>, recoverable: bool) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            recoverable,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PublicKeyJwk {
    kty: String,
    crv: String,
    x: String,
    y: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GatewayDevice {
    device_id: String,
    device_code: String,
    status: String,
    app_version: String,
    release_channel: String,
    entitlement_state: String,
    update_policy: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GatewayPolicy {
    release_channel: String,
    entitlement_state: String,
    update_policy: String,
}

#[derive(Debug, Deserialize)]
struct RegisterResponse {
    device: GatewayDevice,
    policy: GatewayPolicy,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ChallengeResponse {
    challenge: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct HeartbeatResponse {
    device: GatewayDevice,
    policy: GatewayPolicy,
    heartbeat_interval_seconds: u64,
    #[serde(default)]
    commands: Vec<AgentCommand>,
}

#[must_use]
pub fn release_channel() -> String {
    if let Ok(value) = std::env::var("PC_MANAGER_RELEASE_CHANNEL") {
        let normalized = value.trim().to_ascii_lowercase();
        if matches!(normalized.as_str(), "stable" | "preview" | "development") {
            return normalized;
        }
    }

    if cfg!(debug_assertions) {
        "development".to_string()
    } else {
        "stable".to_string()
    }
}

#[must_use]
pub fn next_backoff_seconds(consecutive_failures: u32) -> u64 {
    if consecutive_failures == 0 {
        return DEFAULT_HEARTBEAT_SECONDS;
    }
    let shift = consecutive_failures.saturating_sub(1).min(4);
    (30_u64.saturating_mul(1_u64 << shift)).min(MAX_BACKOFF_SECONDS)
}

pub fn normalize_endpoint(value: &str) -> Result<String, AppManagerError> {
    let trimmed = value.trim().trim_end_matches('/');
    let lower = trimmed.to_ascii_lowercase();
    let secure = lower.starts_with("https://");
    let loopback = lower.starts_with("http://127.0.0.1:")
        || lower.starts_with("http://localhost:")
        || lower == "http://127.0.0.1"
        || lower == "http://localhost";

    if (!secure && !loopback)
        || trimmed.len() > 240
        || trimmed.contains(char::is_whitespace)
        || trimmed.contains('#')
        || trimmed.contains('?')
    {
        return Err(AppManagerError::new(
            "invalid_app_manager_endpoint",
            "App Manager must use HTTPS, except loopback HTTP during local development.",
            true,
        ));
    }

    let after_scheme = trimmed
        .split_once("://")
        .map(|(_, rest)| rest)
        .unwrap_or_default();
    if after_scheme.is_empty() || after_scheme.contains('/') {
        return Err(AppManagerError::new(
            "invalid_app_manager_endpoint",
            "App Manager endpoint must be an origin without a path.",
            true,
        ));
    }

    Ok(trimmed.to_string())
}

#[cfg(target_os = "windows")]
fn app_data_dir() -> Result<PathBuf, AppManagerError> {
    let root = std::env::var_os("LOCALAPPDATA").ok_or_else(|| {
        AppManagerError::new(
            "local_app_data_unavailable",
            "LOCALAPPDATA is unavailable for PC Manager state.",
            true,
        )
    })?;
    Ok(PathBuf::from(root).join("PCManager"))
}

#[cfg(target_os = "windows")]
fn endpoint_file() -> Result<PathBuf, AppManagerError> {
    Ok(app_data_dir()?.join("app-manager-endpoint.txt"))
}

pub fn configured_endpoint() -> Result<Option<String>, AppManagerError> {
    if let Ok(value) = std::env::var("PC_MANAGER_APP_MANAGER_BASE_URL") {
        if !value.trim().is_empty() {
            return normalize_endpoint(&value).map(Some);
        }
    }

    #[cfg(target_os = "windows")]
    {
        let path = endpoint_file()?;
        if !path.exists() {
            return Ok(None);
        }
        let value = fs::read_to_string(path).map_err(|error| {
            AppManagerError::new(
                "app_manager_endpoint_read_failed",
                format!("Unable to read App Manager endpoint: {error}"),
                true,
            )
        })?;
        if value.trim().is_empty() {
            return Ok(None);
        }
        return normalize_endpoint(&value).map(Some);
    }

    #[cfg(not(target_os = "windows"))]
    {
        Ok(None)
    }
}

pub fn set_configured_endpoint(value: Option<&str>) -> Result<Option<String>, AppManagerError> {
    #[cfg(target_os = "windows")]
    {
        let path = endpoint_file()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|error| {
                AppManagerError::new(
                    "app_manager_config_dir_failed",
                    format!("Unable to create PC Manager config directory: {error}"),
                    true,
                )
            })?;
        }

        match value.map(str::trim).filter(|value| !value.is_empty()) {
            Some(raw) => {
                let endpoint = normalize_endpoint(raw)?;
                fs::write(&path, format!("{endpoint}\n")).map_err(|error| {
                    AppManagerError::new(
                        "app_manager_endpoint_write_failed",
                        format!("Unable to save App Manager endpoint: {error}"),
                        true,
                    )
                })?;
                Ok(Some(endpoint))
            }
            None => {
                if path.exists() {
                    fs::remove_file(path).map_err(|error| {
                        AppManagerError::new(
                            "app_manager_endpoint_remove_failed",
                            format!("Unable to clear App Manager endpoint: {error}"),
                            true,
                        )
                    })?;
                }
                Ok(None)
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = value;
        Err(AppManagerError::new(
            "unsupported_platform",
            "App Manager endpoint persistence is implemented for Windows only.",
            false,
        ))
    }
}

#[cfg(target_os = "windows")]
const IDENTITY_SCRIPT: &str = r#"
$ErrorActionPreference = 'Stop'
$keyName = 'PCManagerDesktopAgentP8'
$provider = [System.Security.Cryptography.CngProvider]::MicrosoftSoftwareKeyStorageProvider
try {
  $key = [System.Security.Cryptography.CngKey]::Open($keyName, $provider)
} catch {
  $parameters = New-Object System.Security.Cryptography.CngKeyCreationParameters
  $parameters.Provider = $provider
  $parameters.KeyUsage = [System.Security.Cryptography.CngKeyUsages]::Signing
  $key = [System.Security.Cryptography.CngKey]::Create(
    [System.Security.Cryptography.CngAlgorithm]::ECDsaP256,
    $keyName,
    $parameters
  )
}
$ecdsa = New-Object System.Security.Cryptography.ECDsaCng($key)
$params = $ecdsa.ExportParameters($false)
function B64Url([byte[]]$bytes) {
  [Convert]::ToBase64String($bytes).TrimEnd('=').Replace('+','-').Replace('/','_')
}
[pscustomobject]@{
  kty = 'EC'
  crv = 'P-256'
  x = B64Url $params.Q.X
  y = B64Url $params.Q.Y
} | ConvertTo-Json -Compress
"#;

#[cfg(target_os = "windows")]
const SIGN_SCRIPT: &str = r#"
$ErrorActionPreference = 'Stop'
$keyName = 'PCManagerDesktopAgentP8'
$provider = [System.Security.Cryptography.CngProvider]::MicrosoftSoftwareKeyStorageProvider
$key = [System.Security.Cryptography.CngKey]::Open($keyName, $provider)
$ecdsa = New-Object System.Security.Cryptography.ECDsaCng($key)
$data = [Text.Encoding]::UTF8.GetBytes($env:PC_MANAGER_AGENT_MESSAGE)
$signature = $ecdsa.SignData($data, [System.Security.Cryptography.HashAlgorithmName]::SHA256)
[Convert]::ToBase64String($signature).TrimEnd('=').Replace('+','-').Replace('/','_')
"#;

#[cfg(target_os = "windows")]
fn run_powershell(script: &str, message: Option<&str>) -> Result<String, AppManagerError> {
    let mut command = Command::new("powershell.exe");
    command.args([
        "-NoLogo",
        "-NoProfile",
        "-NonInteractive",
        "-ExecutionPolicy",
        "Bypass",
        "-Command",
        script,
    ]);
    if let Some(value) = message {
        command.env("PC_MANAGER_AGENT_MESSAGE", value);
    }

    let output = command.output().map_err(|error| {
        AppManagerError::new(
            "powershell_unavailable",
            format!("Unable to start the fixed PC Manager identity provider: {error}"),
            true,
        )
    })?;

    if !output.status.success() {
        return Err(AppManagerError::new(
            "agent_identity_provider_failed",
            String::from_utf8_lossy(&output.stderr)
                .trim()
                .chars()
                .take(500)
                .collect::<String>(),
            true,
        ));
    }

    String::from_utf8(output.stdout)
        .map(|value| value.trim().to_string())
        .map_err(|error| {
            AppManagerError::new(
                "agent_identity_encoding_failed",
                format!("PC Manager identity output was invalid UTF-8: {error}"),
                true,
            )
        })
}

#[cfg(target_os = "windows")]
fn ensure_identity() -> Result<PublicKeyJwk, AppManagerError> {
    let json = run_powershell(IDENTITY_SCRIPT, None)?;
    serde_json::from_str(&json).map_err(|error| {
        AppManagerError::new(
            "agent_identity_payload_failed",
            format!("Unable to parse PC Manager public identity: {error}"),
            true,
        )
    })
}

#[cfg(target_os = "windows")]
fn sign_message(message: &str) -> Result<String, AppManagerError> {
    let signature = run_powershell(SIGN_SCRIPT, Some(message))?;
    if signature.len() < 80 || signature.len() > 160 {
        return Err(AppManagerError::new(
            "agent_signature_invalid",
            "The Windows CNG signature provider returned an invalid signature.",
            true,
        ));
    }
    Ok(signature)
}

#[cfg(target_os = "windows")]
fn post_json(endpoint: &str, payload: &Value) -> Result<Value, AppManagerError> {
    let url = format!("{endpoint}/api/desktop-agent");
    let body = serde_json::to_vec(payload).map_err(|error| {
        AppManagerError::new(
            "agent_request_encode_failed",
            format!("Unable to encode App Manager request: {error}"),
            true,
        )
    })?;

    let mut child = Command::new("curl.exe")
        .args([
            "--silent",
            "--show-error",
            "--max-time",
            "20",
            "--header",
            "Content-Type: application/json",
            "--data-binary",
            "@-",
            "--url",
            &url,
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| {
            AppManagerError::new(
                "agent_transport_unavailable",
                format!("Unable to start Windows HTTPS transport: {error}"),
                true,
            )
        })?;

    if let Some(stdin) = child.stdin.as_mut() {
        stdin.write_all(&body).map_err(|error| {
            AppManagerError::new(
                "agent_transport_write_failed",
                format!("Unable to send App Manager request: {error}"),
                true,
            )
        })?;
    }

    let output = child.wait_with_output().map_err(|error| {
        AppManagerError::new(
            "agent_transport_failed",
            format!("App Manager request failed: {error}"),
            true,
        )
    })?;
    if !output.status.success() {
        return Err(AppManagerError::new(
            "agent_transport_failed",
            String::from_utf8_lossy(&output.stderr).trim().chars().take(500).collect::<String>(),
            true,
        ));
    }

    let value: Value = serde_json::from_slice(&output.stdout).map_err(|error| {
        AppManagerError::new(
            "agent_response_invalid",
            format!("App Manager returned invalid JSON: {error}"),
            true,
        )
    })?;

    if value.get("ok").and_then(Value::as_bool) == Some(false) {
        return Err(AppManagerError::new(
            value
                .get("code")
                .and_then(Value::as_str)
                .unwrap_or("agent_gateway_error"),
            value
                .get("error")
                .and_then(Value::as_str)
                .unwrap_or("App Manager rejected the request."),
            true,
        ));
    }

    Ok(value)
}

fn now_epoch_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or_default()
}

#[cfg(target_os = "windows")]
fn base_state(endpoint: String, device: &GatewayDevice, policy: &GatewayPolicy) -> AppManagerState {
    AppManagerState {
        configured: true,
        online: true,
        endpoint: Some(endpoint),
        device_id: Some(device.device_id.clone()),
        device_code: Some(device.device_code.clone()),
        approval_state: device.status.clone(),
        app_version: device.app_version.clone(),
        release_channel: policy.release_channel.clone(),
        entitlement_state: policy.entitlement_state.clone(),
        update_policy: policy.update_policy.clone(),
        last_success_epoch_ms: Some(now_epoch_ms()),
        retry_after_seconds: DEFAULT_HEARTBEAT_SECONDS,
        message: match device.status.as_str() {
            "approved" => "Connected to App Manager.".to_string(),
            "pending" => "Registered and waiting for App Manager approval.".to_string(),
            "blocked" => "This PC Manager device is blocked by App Manager.".to_string(),
            _ => "App Manager returned an unknown approval state.".to_string(),
        },
    }
}

pub fn sync_once() -> Result<SyncBatch, AppManagerError> {
    let Some(endpoint) = configured_endpoint()? else {
        return Ok(SyncBatch {
            state: AppManagerState::unconfigured(),
            commands: Vec::new(),
            heartbeat_interval_seconds: DEFAULT_HEARTBEAT_SECONDS,
        });
    };

    #[cfg(target_os = "windows")]
    {
        let public_key = ensure_identity()?;
        let register_value = post_json(
            &endpoint,
            &json!({
                "action": "register",
                "protocol": AGENT_PROTOCOL,
                "appId": APP_ID,
                "platform": PLATFORM,
                "deviceType": DEVICE_TYPE,
                "appVersion": env!("CARGO_PKG_VERSION"),
                "releaseChannel": release_channel(),
                "publicKey": public_key,
            }),
        )?;
        let register: RegisterResponse =
            serde_json::from_value(register_value).map_err(|error| {
                AppManagerError::new(
                    "agent_register_response_invalid",
                    format!("Unable to parse App Manager registration: {error}"),
                    true,
                )
            })?;

        let state = base_state(endpoint.clone(), &register.device, &register.policy);
        if register.device.status != "approved" {
            return Ok(SyncBatch {
                state,
                commands: Vec::new(),
                heartbeat_interval_seconds: DEFAULT_HEARTBEAT_SECONDS,
            });
        }

        let challenge_value = post_json(
            &endpoint,
            &json!({
                "action": "challenge",
                "protocol": AGENT_PROTOCOL,
                "appId": APP_ID,
                "deviceId": register.device.device_id,
            }),
        )?;
        let challenge: ChallengeResponse =
            serde_json::from_value(challenge_value).map_err(|error| {
                AppManagerError::new(
                    "agent_challenge_response_invalid",
                    format!("Unable to parse App Manager challenge: {error}"),
                    true,
                )
            })?;
        let signed_message = format!(
            "{AGENT_PROTOCOL}:heartbeat:{}:{}",
            register.device.device_id, challenge.challenge
        );
        let signature = sign_message(&signed_message)?;
        let heartbeat_value = post_json(
            &endpoint,
            &json!({
                "action": "heartbeat",
                "protocol": AGENT_PROTOCOL,
                "appId": APP_ID,
                "deviceId": register.device.device_id,
                "challenge": challenge.challenge,
                "signature": signature,
                "appVersion": env!("CARGO_PKG_VERSION"),
            }),
        )?;
        let heartbeat: HeartbeatResponse =
            serde_json::from_value(heartbeat_value).map_err(|error| {
                AppManagerError::new(
                    "agent_heartbeat_response_invalid",
                    format!("Unable to parse App Manager heartbeat: {error}"),
                    true,
                )
            })?;

        let mut state = base_state(endpoint, &heartbeat.device, &heartbeat.policy);
        state.retry_after_seconds = heartbeat
            .heartbeat_interval_seconds
            .clamp(30, MAX_BACKOFF_SECONDS);
        Ok(SyncBatch {
            state,
            commands: heartbeat.commands,
            heartbeat_interval_seconds: heartbeat
                .heartbeat_interval_seconds
                .clamp(30, MAX_BACKOFF_SECONDS),
        })
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = endpoint;
        Err(AppManagerError::new(
            "unsupported_platform",
            "App Manager agent synchronization is implemented for Windows only.",
            false,
        ))
    }
}

pub fn acknowledge_command(
    device_id: &str,
    command_id: &str,
    outcome: &str,
    result: Value,
) -> Result<(), AppManagerError> {
    let Some(endpoint) = configured_endpoint()? else {
        return Err(AppManagerError::new(
            "app_manager_unconfigured",
            "App Manager endpoint is not configured.",
            true,
        ));
    };

    #[cfg(target_os = "windows")]
    {
        let challenge_value = post_json(
            &endpoint,
            &json!({
                "action": "challenge",
                "protocol": AGENT_PROTOCOL,
                "appId": APP_ID,
                "deviceId": device_id,
            }),
        )?;
        let challenge: ChallengeResponse =
            serde_json::from_value(challenge_value).map_err(|error| {
                AppManagerError::new(
                    "agent_challenge_response_invalid",
                    format!("Unable to parse App Manager challenge: {error}"),
                    true,
                )
            })?;
        let signed_message = format!("{AGENT_PROTOCOL}:ack:{device_id}:{}", challenge.challenge);
        let signature = sign_message(&signed_message)?;
        post_json(
            &endpoint,
            &json!({
                "action": "ack",
                "protocol": AGENT_PROTOCOL,
                "appId": APP_ID,
                "deviceId": device_id,
                "challenge": challenge.challenge,
                "signature": signature,
                "commandId": command_id,
                "outcome": outcome,
                "result": result,
            }),
        )?;
        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = (endpoint, device_id, command_id, outcome, result);
        Err(AppManagerError::new(
            "unsupported_platform",
            "App Manager command acknowledgement is implemented for Windows only.",
            false,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::{
        next_backoff_seconds, normalize_endpoint, AgentCommand, AgentCommandType, AGENT_PROTOCOL,
    };

    #[test]
    fn endpoint_policy_is_https_except_loopback_development() {
        assert_eq!(
            normalize_endpoint("https://manager.example.test/").unwrap(),
            "https://manager.example.test"
        );
        assert!(normalize_endpoint("http://127.0.0.1:3000").is_ok());
        assert!(normalize_endpoint("http://localhost:3000").is_ok());
        assert!(normalize_endpoint("http://manager.example.test").is_err());
        assert!(normalize_endpoint("https://manager.example.test/api").is_err());
    }

    #[test]
    fn retry_backoff_is_bounded() {
        assert_eq!(next_backoff_seconds(0), 60);
        assert_eq!(next_backoff_seconds(1), 30);
        assert_eq!(next_backoff_seconds(2), 60);
        assert_eq!(next_backoff_seconds(3), 120);
        assert_eq!(next_backoff_seconds(10), 300);
    }

    #[test]
    fn command_envelope_rejects_unknown_remote_execution_types() {
        let valid = serde_json::from_str::<AgentCommand>(
            r#"{"schema":"pc-manager-command/v1","commandId":"00000000-0000-4000-8000-000000000001","type":"RUN_HEALTH_SCAN","issuedAt":"2026-10-06T00:00:00Z","expiresAt":1}"#,
        )
        .unwrap();
        assert_eq!(valid.command_type, AgentCommandType::RunHealthScan);

        let arbitrary = serde_json::from_str::<AgentCommand>(
            r#"{"schema":"pc-manager-command/v1","commandId":"00000000-0000-4000-8000-000000000001","type":"RUN_POWERSHELL","issuedAt":"2026-10-06T00:00:00Z","expiresAt":1}"#,
        );
        assert!(arbitrary.is_err());
        assert_eq!(AGENT_PROTOCOL, "pc-manager-agent/v1");
    }
}

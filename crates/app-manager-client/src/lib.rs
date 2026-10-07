//! Typed outbound App Manager Desktop Agent client.
//!
//! P8 deliberately keeps App Manager transport inside this crate. The desktop
//! application never opens an inbound port. On Windows, a fixed application-
//! authored PowerShell provider uses a persisted CNG P-256 signing key and
//! HTTPS requests. No server value is ever executed as PowerShell source.

use serde::{Deserialize, Serialize};

pub const APP_ID: &str = "pc-manager";
pub const PLATFORM: &str = "windows";
pub const DEVICE_TYPE: &str = "desktop-native";
pub const PROTOCOL: &str = "application-management.desktop-agent/v1";
pub const GATEWAY_PATH: &str = "/api/desktop-agent";

const DEFAULT_HEARTBEAT_SECONDS: u64 = 60;
const MAX_HEARTBEAT_SECONDS: u64 = 300;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DeviceApprovalState {
    Pending,
    Approved,
    Blocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReleaseChannel {
    Dev,
    Beta,
    Stable,
}

impl ReleaseChannel {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Dev => "dev",
            Self::Beta => "beta",
            Self::Stable => "stable",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EntitlementState {
    Unknown,
    Active,
    Trial,
    Disabled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdatePolicy {
    #[serde(default = "default_release_channel")]
    pub channel: ReleaseChannel,
    #[serde(default = "default_true")]
    pub auto_check: bool,
    #[serde(default)]
    pub minimum_version: Option<String>,
}

impl Default for UpdatePolicy {
    fn default() -> Self {
        Self {
            channel: ReleaseChannel::Stable,
            auto_check: true,
            minimum_version: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentDeviceState {
    pub device_id: String,
    pub device_code: String,
    pub app_id: String,
    pub platform: String,
    pub device_type: String,
    pub status: DeviceApprovalState,
    pub online: bool,
    pub app_version: String,
    pub release_channel: ReleaseChannel,
    pub entitlement_state: EntitlementState,
    #[serde(default)]
    pub update_policy: UpdatePolicy,
    pub created_at: String,
    pub approved_at: Option<String>,
    pub blocked_at: Option<String>,
    pub last_seen_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RemoteCommand {
    CheckUpdate {
        #[serde(rename = "commandId")]
        command_id: String,
        #[serde(default)]
        payload: serde_json::Value,
    },
    RunHealthScan {
        #[serde(rename = "commandId")]
        command_id: String,
        #[serde(default)]
        payload: serde_json::Value,
    },
    RefreshDeviceStatus {
        #[serde(rename = "commandId")]
        command_id: String,
        #[serde(default)]
        payload: serde_json::Value,
    },
    DisableLicense {
        #[serde(rename = "commandId")]
        command_id: String,
        #[serde(default)]
        payload: serde_json::Value,
    },
}

impl RemoteCommand {
    #[must_use]
    pub fn command_id(&self) -> &str {
        match self {
            Self::CheckUpdate { command_id, .. }
            | Self::RunHealthScan { command_id, .. }
            | Self::RefreshDeviceStatus { command_id, .. }
            | Self::DisableLicense { command_id, .. } => command_id,
        }
    }

    #[must_use]
    pub const fn kind(&self) -> &'static str {
        match self {
            Self::CheckUpdate { .. } => "CHECK_UPDATE",
            Self::RunHealthScan { .. } => "RUN_HEALTH_SCAN",
            Self::RefreshDeviceStatus { .. } => "REFRESH_DEVICE_STATUS",
            Self::DisableLicense { .. } => "DISABLE_LICENSE",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HeartbeatResponse {
    pub protocol: String,
    pub server_time: String,
    #[serde(default = "default_heartbeat_seconds")]
    pub heartbeat_after_seconds: u64,
    pub device: AgentDeviceState,
    #[serde(default)]
    pub commands: Vec<RemoteCommand>,
    pub challenge: String,
    pub expires_at: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandResultAck {
    pub protocol: String,
    pub accepted: bool,
    pub replayed: bool,
    pub challenge: String,
    pub expires_at: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientError {
    pub code: String,
    pub message: String,
    pub recoverable: bool,
}

impl ClientError {
    #[must_use]
    pub fn new(code: impl Into<String>, message: impl Into<String>, recoverable: bool) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            recoverable,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientConfig {
    pub configured: bool,
    pub origin: Option<String>,
    pub release_channel: ReleaseChannel,
    pub app_version: String,
}

#[derive(Debug, Clone)]
pub struct AppManagerClient {
    origin: Option<String>,
    release_channel: ReleaseChannel,
    app_version: String,
}

impl AppManagerClient {
    #[must_use]
    pub fn from_environment(app_version: impl Into<String>) -> Self {
        let origin = std::env::var("PC_MANAGER_APP_MANAGER_ORIGIN")
            .ok()
            .and_then(|value| normalize_origin(&value).ok());
        let release_channel = std::env::var("PC_MANAGER_RELEASE_CHANNEL")
            .ok()
            .as_deref()
            .and_then(parse_release_channel)
            .unwrap_or(ReleaseChannel::Stable);

        Self {
            origin,
            release_channel,
            app_version: app_version.into(),
        }
    }

    #[must_use]
    pub fn config(&self) -> ClientConfig {
        ClientConfig {
            configured: self.origin.is_some(),
            origin: self.origin.clone(),
            release_channel: self.release_channel,
            app_version: self.app_version.clone(),
        }
    }

    pub fn heartbeat(&self) -> Result<HeartbeatResponse, ClientError> {
        let origin = self.origin.as_deref().ok_or_else(|| {
            ClientError::new(
                "gateway_not_configured",
                "App Manager gateway is not configured. Local PC Manager features remain available.",
                true,
            )
        })?;

        #[cfg(target_os = "windows")]
        {
            let script = windows_heartbeat_script();
            let raw = run_windows_provider(
                &script,
                origin,
                &self.app_version,
                self.release_channel,
                None,
            )?;
            let response: HeartbeatResponse = serde_json::from_str(&raw).map_err(|error| {
                ClientError::new(
                    "gateway_response_invalid",
                    format!("App Manager returned an invalid heartbeat payload: {error}"),
                    true,
                )
            })?;
            validate_heartbeat(&response)?;
            Ok(response)
        }

        #[cfg(not(target_os = "windows"))]
        {
            let _ = origin;
            Err(ClientError::new(
                "unsupported_platform",
                "App Manager desktop-agent transport is available on Windows only.",
                false,
            ))
        }
    }

    pub fn submit_result(
        &self,
        device_id: &str,
        command_id: &str,
        succeeded: bool,
        result: &serde_json::Value,
    ) -> Result<CommandResultAck, ClientError> {
        let origin = self.origin.as_deref().ok_or_else(|| {
            ClientError::new(
                "gateway_not_configured",
                "App Manager gateway is not configured.",
                true,
            )
        })?;
        validate_device_id(device_id)?;
        validate_command_id(command_id)?;
        let compact = serde_json::to_string(result).map_err(|error| {
            ClientError::new(
                "command_result_invalid",
                format!("Unable to serialize the command result: {error}"),
                false,
            )
        })?;
        if compact.len() > 4_096 {
            return Err(ClientError::new(
                "command_result_too_large",
                "Remote command result exceeds the 4 KB privacy-safe limit.",
                false,
            ));
        }

        #[cfg(target_os = "windows")]
        {
            let envelope = serde_json::json!({
                "deviceId": device_id,
                "commandId": command_id,
                "status": if succeeded { "completed" } else { "failed" },
                "result": result,
            });
            let script = windows_result_script();
            let raw = run_windows_provider(
                &script,
                origin,
                &self.app_version,
                self.release_channel,
                Some(&envelope.to_string()),
            )?;
            let response: CommandResultAck = serde_json::from_str(&raw).map_err(|error| {
                ClientError::new(
                    "gateway_response_invalid",
                    format!("App Manager returned an invalid result acknowledgement: {error}"),
                    true,
                )
            })?;
            if response.protocol != PROTOCOL || !response.accepted {
                return Err(ClientError::new(
                    "gateway_result_rejected",
                    "App Manager did not accept the typed command result.",
                    true,
                ));
            }
            Ok(response)
        }

        #[cfg(not(target_os = "windows"))]
        {
            let _ = (origin, compact, succeeded);
            Err(ClientError::new(
                "unsupported_platform",
                "App Manager desktop-agent transport is available on Windows only.",
                false,
            ))
        }
    }
}

#[must_use]
pub fn retry_delay_seconds(failure_count: u32) -> u64 {
    match failure_count {
        0 => 5,
        1 => 15,
        2 => 30,
        3 => 60,
        4 => 120,
        _ => 300,
    }
}

#[must_use]
pub fn bounded_heartbeat_seconds(value: u64) -> u64 {
    value.clamp(30, MAX_HEARTBEAT_SECONDS)
}

fn default_heartbeat_seconds() -> u64 {
    DEFAULT_HEARTBEAT_SECONDS
}

fn default_release_channel() -> ReleaseChannel {
    ReleaseChannel::Stable
}

fn default_true() -> bool {
    true
}

fn parse_release_channel(value: &str) -> Option<ReleaseChannel> {
    match value.trim().to_ascii_lowercase().as_str() {
        "dev" => Some(ReleaseChannel::Dev),
        "beta" => Some(ReleaseChannel::Beta),
        "stable" => Some(ReleaseChannel::Stable),
        _ => None,
    }
}

fn normalize_origin(value: &str) -> Result<String, ClientError> {
    let origin = value.trim().trim_end_matches('/');
    if origin.is_empty()
        || origin.contains('@')
        || origin.contains('?')
        || origin.contains('#')
        || origin[8.min(origin.len())..].contains('/')
    {
        return Err(ClientError::new(
            "gateway_origin_invalid",
            "App Manager origin must be a bare HTTPS origin.",
            false,
        ));
    }

    if origin.starts_with("https://") {
        return Ok(origin.to_string());
    }

    let allow_local = std::env::var("PC_MANAGER_ALLOW_INSECURE_LOCAL_GATEWAY")
        .ok()
        .as_deref()
        == Some("1");
    if allow_local
        && (origin.starts_with("http://127.0.0.1:") || origin.starts_with("http://localhost:"))
    {
        return Ok(origin.to_string());
    }

    Err(ClientError::new(
        "gateway_origin_insecure",
        "App Manager requires HTTPS. Plain HTTP is limited to explicitly enabled loopback development.",
        false,
    ))
}

fn validate_device_id(value: &str) -> Result<(), ClientError> {
    if value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        Ok(())
    } else {
        Err(ClientError::new(
            "device_id_invalid",
            "App Manager device ID is invalid.",
            false,
        ))
    }
}

fn validate_command_id(value: &str) -> Result<(), ClientError> {
    let valid = value.len() == 36
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() || byte == b'-');
    if valid {
        Ok(())
    } else {
        Err(ClientError::new(
            "command_id_invalid",
            "Remote command ID is invalid.",
            false,
        ))
    }
}

#[cfg(target_os = "windows")]
fn validate_heartbeat(value: &HeartbeatResponse) -> Result<(), ClientError> {
    if value.protocol != PROTOCOL {
        return Err(ClientError::new(
            "gateway_protocol_mismatch",
            "App Manager desktop-agent protocol is incompatible.",
            false,
        ));
    }
    validate_device_id(&value.device.device_id)?;
    if value.device.app_id != APP_ID
        || value.device.platform != PLATFORM
        || value.device.device_type != DEVICE_TYPE
    {
        return Err(ClientError::new(
            "gateway_identity_mismatch",
            "App Manager returned state for a different application identity.",
            false,
        ));
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn run_windows_provider(
    script: &str,
    origin: &str,
    app_version: &str,
    release_channel: ReleaseChannel,
    result_envelope: Option<&str>,
) -> Result<String, ClientError> {
    use std::process::Command;

    let mut command = Command::new("powershell.exe");
    command
        .args([
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            script,
        ])
        .env("PCM_GATEWAY_ORIGIN", origin)
        .env("PCM_APP_VERSION", app_version)
        .env("PCM_RELEASE_CHANNEL", release_channel.as_str());
    if let Some(value) = result_envelope {
        command.env("PCM_COMMAND_RESULT", value);
    }

    let output = command.output().map_err(|error| {
        ClientError::new(
            "gateway_provider_unavailable",
            format!("Unable to start the fixed App Manager Windows provider: {error}"),
            true,
        )
    })?;
    if !output.status.success() {
        let detail = String::from_utf8_lossy(&output.stderr);
        return Err(ClientError::new(
            "gateway_request_failed",
            if detail.trim().is_empty() {
                "App Manager request failed. Local PC Manager features remain available."
                    .to_string()
            } else {
                format!("App Manager request failed: {}", detail.trim())
            },
            true,
        ));
    }

    String::from_utf8(output.stdout).map_err(|error| {
        ClientError::new(
            "gateway_response_encoding",
            format!("App Manager response was not valid UTF-8: {error}"),
            true,
        )
    })
}

#[cfg(target_os = "windows")]
const WINDOWS_KEY_HELPERS: &str = r#"
$ErrorActionPreference = 'Stop'
$origin = $env:PCM_GATEWAY_ORIGIN
$gateway = "$origin/api/desktop-agent"
$keyName = 'PCManager.AppManager.DeviceKey.v1'
$provider = [System.Security.Cryptography.CngProvider]::MicrosoftSoftwareKeyStorageProvider
if ([System.Security.Cryptography.CngKey]::Exists($keyName, $provider)) {
  $key = [System.Security.Cryptography.CngKey]::Open($keyName, $provider)
} else {
  $parameters = [System.Security.Cryptography.CngKeyCreationParameters]::new()
  $parameters.Provider = $provider
  $parameters.KeyUsage = [System.Security.Cryptography.CngKeyUsages]::Signing
  $key = [System.Security.Cryptography.CngKey]::Create(
    [System.Security.Cryptography.CngAlgorithm]::EcdsaP256,
    $keyName,
    $parameters
  )
}
$ecdsa = [System.Security.Cryptography.ECDsaCng]::new($key)
$public = $ecdsa.ExportParameters($false)
function To-Base64Url([byte[]]$bytes) {
  [Convert]::ToBase64String($bytes).TrimEnd('=').Replace('+','-').Replace('/','_')
}
function Sign-AgentMessage([string]$deviceId, [string]$challenge, [string]$action) {
  $message = "pc-manager-agent/v1:$deviceId:$challenge:$action"
  $bytes = [Text.Encoding]::UTF8.GetBytes($message)
  $signature = $ecdsa.SignData($bytes, [Security.Cryptography.HashAlgorithmName]::SHA256)
  To-Base64Url $signature
}
$publicKey = @{
  kty = 'EC'
  crv = 'P-256'
  x = To-Base64Url $public.Q.X
  y = To-Base64Url $public.Q.Y
}
$registerPayload = @{
  action = 'register'
  appId = 'pc-manager'
  platform = 'windows'
  deviceType = 'desktop-native'
  appVersion = $env:PCM_APP_VERSION
  releaseChannel = $env:PCM_RELEASE_CHANNEL
  publicKey = $publicKey
}
$register = Invoke-RestMethod -Method Post -Uri $gateway -ContentType 'application/json' -Body ($registerPayload | ConvertTo-Json -Depth 8 -Compress)
$deviceId = [string]$register.device.deviceId
$challenge = [string]$register.challenge
"#;

#[cfg(target_os = "windows")]
const WINDOWS_HEARTBEAT_BODY: &str = r#"
$heartbeatPayload = @{
  action = 'heartbeat'
  deviceId = $deviceId
  proof = @{
    challenge = $challenge
    signature = Sign-AgentMessage $deviceId $challenge 'heartbeat'
  }
  telemetry = @{
    appId = 'pc-manager'
    platform = 'windows'
    deviceType = 'desktop-native'
    appVersion = $env:PCM_APP_VERSION
    releaseChannel = $env:PCM_RELEASE_CHANNEL
  }
}
$response = Invoke-RestMethod -Method Post -Uri $gateway -ContentType 'application/json' -Body ($heartbeatPayload | ConvertTo-Json -Depth 8 -Compress)
$response | ConvertTo-Json -Depth 12 -Compress
"#;

#[cfg(target_os = "windows")]
const WINDOWS_RESULT_BODY: &str = r#"
$commandResult = $env:PCM_COMMAND_RESULT | ConvertFrom-Json
$resultPayload = @{
  action = 'result'
  deviceId = $deviceId
  commandId = [string]$commandResult.commandId
  status = [string]$commandResult.status
  result = $commandResult.result
  proof = @{
    challenge = $challenge
    signature = Sign-AgentMessage $deviceId $challenge 'result'
  }
}
$response = Invoke-RestMethod -Method Post -Uri $gateway -ContentType 'application/json' -Body ($resultPayload | ConvertTo-Json -Depth 10 -Compress)
$response | ConvertTo-Json -Depth 8 -Compress
"#;

#[cfg(target_os = "windows")]
fn provider_script(body: &str) -> String {
    format!("{WINDOWS_KEY_HELPERS}\n{body}")
}

#[cfg(target_os = "windows")]
fn windows_heartbeat_script() -> String {
    provider_script(WINDOWS_HEARTBEAT_BODY)
}

#[cfg(target_os = "windows")]
fn windows_result_script() -> String {
    provider_script(WINDOWS_RESULT_BODY)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_deserialization_is_allow_listed() {
        let allowed = serde_json::from_str::<RemoteCommand>(
            r#"{"commandId":"00000000-0000-0000-0000-000000000001","type":"RUN_HEALTH_SCAN","payload":{}}"#,
        )
        .expect("allowed command should parse");
        assert_eq!(allowed.kind(), "RUN_HEALTH_SCAN");

        let rejected = serde_json::from_str::<RemoteCommand>(
            r#"{"commandId":"00000000-0000-0000-0000-000000000002","type":"RUN_COMMAND","payload":{"command":"whoami"}}"#,
        );
        assert!(rejected.is_err());
    }

    #[test]
    fn retry_backoff_is_bounded() {
        assert_eq!(retry_delay_seconds(0), 5);
        assert_eq!(retry_delay_seconds(1), 15);
        assert_eq!(retry_delay_seconds(2), 30);
        assert_eq!(retry_delay_seconds(3), 60);
        assert_eq!(retry_delay_seconds(4), 120);
        assert_eq!(retry_delay_seconds(25), 300);
    }

    #[test]
    fn heartbeat_interval_is_bounded() {
        assert_eq!(bounded_heartbeat_seconds(1), 30);
        assert_eq!(bounded_heartbeat_seconds(60), 60);
        assert_eq!(bounded_heartbeat_seconds(999), 300);
    }

    #[test]
    fn production_origin_requires_https() {
        std::env::remove_var("PC_MANAGER_ALLOW_INSECURE_LOCAL_GATEWAY");
        assert!(normalize_origin("https://example.com").is_ok());
        assert!(normalize_origin("http://example.com").is_err());
        assert!(normalize_origin("https://example.com/path").is_err());
    }
}

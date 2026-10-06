use pc_core::{
    AppsError, InstalledAppEntry, InstalledAppSource, UninstallKind, UninstallLaunchResult,
    UninstallRequest,
};
use sha2::{Digest, Sha256};
#[cfg(target_os = "windows")]
use std::collections::BTreeMap;

#[cfg(target_os = "windows")]
use serde::Deserialize;
#[cfg(target_os = "windows")]
use std::process::Command;

#[derive(Debug, Clone)]
struct NativeInstalledApp {
    public: InstalledAppEntry,
    uninstall_string: Option<String>,
    product_code: Option<String>,
}

#[cfg(target_os = "windows")]
const APPS_SCRIPT: &str = r#"
$ErrorActionPreference = 'Stop'
$locations = @(
  @{ Path = 'HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\*'; Source = 'currentUser' },
  @{ Path = 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\*'; Source = 'localMachine64' },
  @{ Path = 'HKLM:\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\*'; Source = 'localMachine32' }
)

$apps = @()
foreach ($location in $locations) {
  $items = @(Get-ItemProperty -Path $location.Path -ErrorAction SilentlyContinue)
  foreach ($item in $items) {
    if ([string]::IsNullOrWhiteSpace([string]$item.DisplayName)) { continue }
    $apps += [pscustomobject]@{
      displayName = [string]$item.DisplayName
      displayVersion = if ($null -eq $item.DisplayVersion) { $null } else { [string]$item.DisplayVersion }
      publisher = if ($null -eq $item.Publisher) { $null } else { [string]$item.Publisher }
      installLocation = if ($null -eq $item.InstallLocation) { $null } else { [string]$item.InstallLocation }
      installDate = if ($null -eq $item.InstallDate) { $null } else { [string]$item.InstallDate }
      estimatedSizeKb = if ($null -eq $item.EstimatedSize) { $null } else { [uint64]$item.EstimatedSize }
      uninstallString = if ($null -eq $item.UninstallString) { $null } else { [string]$item.UninstallString }
      windowsInstaller = if ($null -eq $item.WindowsInstaller) { 0 } else { [int]$item.WindowsInstaller }
      registryIdentity = "$($location.Source)|$($item.PSChildName)"
      productCode = [string]$item.PSChildName
      source = $location.Source
    }
  }
}

[pscustomobject]@{ apps = @($apps) } | ConvertTo-Json -Depth 5 -Compress
"#;

pub fn list_installed_app_entries() -> Result<Vec<InstalledAppEntry>, AppsError> {
    Ok(discover_native_apps()?
        .into_iter()
        .map(|entry| entry.public)
        .collect())
}

pub fn launch_standard_uninstall(
    request: &UninstallRequest,
) -> Result<UninstallLaunchResult, AppsError> {
    if request.app_id.trim().is_empty() {
        return Err(AppsError::new(
            "app_id_missing",
            "The application identifier is missing.",
            false,
        ));
    }

    let entry = discover_native_apps()?
        .into_iter()
        .find(|entry| entry.public.id == request.app_id)
        .ok_or_else(|| {
            AppsError::new(
                "app_not_found",
                "The selected application is no longer present. Refresh the Apps page.",
                true,
            )
        })?;

    if !entry.public.can_uninstall {
        return Err(AppsError::new(
            "uninstall_unavailable",
            "This application does not expose a supported standard uninstall flow.",
            false,
        ));
    }

    launch_native_uninstaller(&entry)?;

    Ok(UninstallLaunchResult {
        app_id: entry.public.id,
        display_name: entry.public.display_name,
        launched: true,
        message: if entry.public.requires_elevation {
            "The application's standard uninstall flow was opened. Windows may request administrator approval.".to_string()
        } else {
            "The application's standard uninstall flow was opened. Finish the vendor uninstall UI, then refresh Apps.".to_string()
        },
    })
}

#[cfg(target_os = "windows")]
fn discover_native_apps() -> Result<Vec<NativeInstalledApp>, AppsError> {
    let output = Command::new("powershell.exe")
        .args([
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            APPS_SCRIPT,
        ])
        .output()
        .map_err(|error| {
            AppsError::new(
                "apps_provider_unavailable",
                format!("Unable to start the Windows Apps provider: {error}"),
                true,
            )
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(AppsError::new(
            "apps_provider_failed",
            if stderr.trim().is_empty() {
                "The Windows Apps provider exited with an error.".to_string()
            } else {
                format!("The Windows Apps provider failed: {}", stderr.trim())
            },
            true,
        ));
    }

    let stdout = String::from_utf8(output.stdout).map_err(|error| {
        AppsError::new(
            "apps_provider_encoding",
            format!("The Apps provider returned invalid UTF-8: {error}"),
            true,
        )
    })?;
    let payload: RawAppPayload = serde_json::from_str(stdout.trim()).map_err(|error| {
        AppsError::new(
            "apps_provider_payload",
            format!("Unable to parse the Apps inventory: {error}"),
            true,
        )
    })?;

    Ok(map_raw_apps(payload.apps))
}

#[cfg(not(target_os = "windows"))]
fn discover_native_apps() -> Result<Vec<NativeInstalledApp>, AppsError> {
    Err(AppsError::new(
        "unsupported_platform",
        "Application management is currently implemented for Windows only.",
        false,
    ))
}

#[cfg(target_os = "windows")]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawAppPayload {
    #[serde(default)]
    apps: Vec<RawApp>,
}

#[cfg(target_os = "windows")]
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawApp {
    display_name: Option<String>,
    display_version: Option<String>,
    publisher: Option<String>,
    install_location: Option<String>,
    install_date: Option<String>,
    estimated_size_kb: Option<u64>,
    uninstall_string: Option<String>,
    windows_installer: Option<u32>,
    registry_identity: Option<String>,
    product_code: Option<String>,
    source: Option<String>,
}

#[cfg(target_os = "windows")]
fn map_raw_apps(apps: Vec<RawApp>) -> Vec<NativeInstalledApp> {
    let mut mapped = BTreeMap::new();

    for raw in apps {
        let Some(display_name) = non_empty(raw.display_name) else {
            continue;
        };
        let Some(identity) = non_empty(raw.registry_identity) else {
            continue;
        };
        let Some(source) = raw.source.as_deref().and_then(parse_source) else {
            continue;
        };

        let windows_installer = raw.windows_installer.unwrap_or_default() != 0;
        let product_code =
            non_empty(raw.product_code).filter(|value| looks_like_product_code(value));
        let uninstall_string = non_empty(raw.uninstall_string);
        let uninstall_kind = if windows_installer && product_code.is_some() {
            UninstallKind::Msi
        } else if uninstall_string
            .as_deref()
            .and_then(parse_registered_uninstall)
            .is_some()
        {
            UninstallKind::Executable
        } else {
            UninstallKind::Unavailable
        };
        let can_uninstall = uninstall_kind != UninstallKind::Unavailable;
        let requires_elevation = source != InstalledAppSource::CurrentUser;
        let id = installed_app_id(source, &identity);

        let public = InstalledAppEntry {
            id: id.clone(),
            display_name,
            publisher: non_empty(raw.publisher),
            version: non_empty(raw.display_version),
            install_location: non_empty(raw.install_location),
            install_date: non_empty(raw.install_date),
            estimated_size_bytes: raw.estimated_size_kb.map(|kb| kb.saturating_mul(1024)),
            source,
            can_uninstall,
            requires_elevation,
            uninstall_kind,
            detail: match (can_uninstall, requires_elevation) {
                (true, true) => {
                    "Standard uninstall available; Windows may request elevation.".to_string()
                }
                (true, false) => "Standard uninstall available for the current user.".to_string(),
                (false, _) => "No supported standard uninstall command is registered.".to_string(),
            },
        };

        mapped.entry(id).or_insert(NativeInstalledApp {
            public,
            uninstall_string,
            product_code,
        });
    }

    mapped.into_values().collect()
}

fn installed_app_id(source: InstalledAppSource, identity: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"pc-manager-installed-app-v1:");
    hasher.update(format!("{source:?}|{}", identity.trim().to_ascii_lowercase()).as_bytes());
    let digest = hasher.finalize();
    format!(
        "app-{}",
        digest[..16]
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    )
}

#[cfg(target_os = "windows")]
fn parse_source(value: &str) -> Option<InstalledAppSource> {
    match value {
        "currentUser" => Some(InstalledAppSource::CurrentUser),
        "localMachine64" => Some(InstalledAppSource::LocalMachine64),
        "localMachine32" => Some(InstalledAppSource::LocalMachine32),
        _ => None,
    }
}

#[cfg(target_os = "windows")]
fn non_empty(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let trimmed = value.trim();
        (!trimmed.is_empty()).then(|| trimmed.to_string())
    })
}

fn looks_like_product_code(value: &str) -> bool {
    let value = value.trim();
    value.len() == 38
        && value.starts_with('{')
        && value.ends_with('}')
        && value.chars().enumerate().all(|(index, ch)| match index {
            0 | 37 => matches!(ch, '{' | '}'),
            9 | 14 | 19 | 24 => ch == '-',
            _ => ch.is_ascii_hexdigit(),
        })
}

#[cfg(target_os = "windows")]
fn launch_native_uninstaller(entry: &NativeInstalledApp) -> Result<(), AppsError> {
    match entry.public.uninstall_kind {
        UninstallKind::Msi => {
            let product_code = entry.product_code.as_deref().ok_or_else(|| {
                AppsError::new(
                    "msi_product_code_missing",
                    "The MSI product code is no longer available.",
                    true,
                )
            })?;
            Command::new("msiexec.exe")
                .args(["/x", product_code])
                .spawn()
                .map(|_| ())
                .map_err(|error| {
                    AppsError::new(
                        "uninstall_launch_failed",
                        format!("Unable to open the MSI uninstall flow: {error}"),
                        true,
                    )
                })
        }
        UninstallKind::Executable => {
            let command = entry.uninstall_string.as_deref().ok_or_else(|| {
                AppsError::new(
                    "uninstall_command_missing",
                    "The registered uninstall command is no longer available.",
                    true,
                )
            })?;
            let (executable, args) = parse_registered_uninstall(command).ok_or_else(|| {
                AppsError::new(
                    "uninstall_command_unsupported",
                    "The registered uninstall command could not be parsed safely.",
                    false,
                )
            })?;
            if args.iter().any(|arg| is_silent_flag(arg)) {
                return Err(AppsError::new(
                    "silent_uninstall_rejected",
                    "The registered uninstall command is silent-only, so PC Manager will not launch it in V1.",
                    false,
                ));
            }

            Command::new(expand_environment_tokens(&executable))
                .args(args)
                .spawn()
                .map(|_| ())
                .map_err(|error| {
                    AppsError::new(
                        "uninstall_launch_failed",
                        format!(
                            "Unable to open the application's standard uninstall flow: {error}"
                        ),
                        true,
                    )
                })
        }
        UninstallKind::Unavailable => Err(AppsError::new(
            "uninstall_unavailable",
            "No supported standard uninstall flow is registered.",
            false,
        )),
    }
}

#[cfg(not(target_os = "windows"))]
fn launch_native_uninstaller(_entry: &NativeInstalledApp) -> Result<(), AppsError> {
    Err(AppsError::new(
        "unsupported_platform",
        "Application uninstall is currently implemented for Windows only.",
        false,
    ))
}

fn parse_registered_uninstall(command: &str) -> Option<(String, Vec<String>)> {
    let command = command.trim();
    if command.is_empty() {
        return None;
    }

    let (executable, rest) = if let Some(stripped) = command.strip_prefix('"') {
        let end = stripped.find('"')?;
        (
            stripped[..end].trim().to_string(),
            stripped[end + 1..].trim(),
        )
    } else {
        let lower = command.to_ascii_lowercase();
        let end = lower.find(".exe")?.saturating_add(4);
        (command[..end].trim().to_string(), command[end..].trim())
    };

    if executable.is_empty() {
        return None;
    }

    Some((executable, tokenize_windows_arguments(rest)))
}

fn tokenize_windows_arguments(input: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '"' => in_quotes = !in_quotes,
            '\\' if chars.peek() == Some(&'"') => {
                current.push('"');
                chars.next();
            }
            ch if ch.is_whitespace() && !in_quotes => {
                if !current.is_empty() {
                    args.push(std::mem::take(&mut current));
                }
            }
            _ => current.push(ch),
        }
    }

    if !current.is_empty() {
        args.push(current);
    }

    args
}

fn is_silent_flag(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "/quiet" | "/qn" | "/q" | "/s" | "-s" | "--silent" | "/silent"
    )
}

#[cfg(target_os = "windows")]
fn expand_environment_tokens(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut remaining = input;

    while let Some(start) = remaining.find('%') {
        result.push_str(&remaining[..start]);
        let after_start = &remaining[start + 1..];
        let Some(end) = after_start.find('%') else {
            result.push_str(&remaining[start..]);
            return result;
        };
        let name = &after_start[..end];
        if let Ok(value) = std::env::var(name) {
            result.push_str(&value);
        } else {
            result.push('%');
            result.push_str(name);
            result.push('%');
        }
        remaining = &after_start[end + 1..];
    }

    result.push_str(remaining);
    result
}

#[cfg(test)]
mod tests {
    use super::{
        installed_app_id, is_silent_flag, looks_like_product_code, parse_registered_uninstall,
    };
    use pc_core::InstalledAppSource;

    #[test]
    fn installed_app_ids_are_stable_and_opaque() {
        let first = installed_app_id(
            InstalledAppSource::CurrentUser,
            "currentUser|Example.Product",
        );
        let second = installed_app_id(
            InstalledAppSource::CurrentUser,
            "currentuser|example.product",
        );

        assert_eq!(first, second);
        assert!(first.starts_with("app-"));
        assert!(!first.to_ascii_lowercase().contains("example"));
    }

    #[test]
    fn quoted_registered_command_is_split_without_shell_injection() {
        let (executable, args) = parse_registered_uninstall(
            r#""C:\Program Files\Example\uninstall.exe" /remove "Example App""#,
        )
        .expect("registered command should parse");

        assert_eq!(executable, r"C:\Program Files\Example\uninstall.exe");
        assert_eq!(args, vec!["/remove", "Example App"]);
    }

    #[test]
    fn unquoted_executable_with_spaces_is_recognized_through_exe_suffix() {
        let (executable, args) =
            parse_registered_uninstall(r"C:\Program Files\Example\uninstall.exe /remove")
                .expect("registered command should parse");

        assert_eq!(executable, r"C:\Program Files\Example\uninstall.exe");
        assert_eq!(args, vec!["/remove"]);
    }

    #[test]
    fn silent_flags_are_rejected_explicitly() {
        assert!(is_silent_flag("/quiet"));
        assert!(is_silent_flag("/QN"));
        assert!(is_silent_flag("--silent"));
        assert!(!is_silent_flag("/remove"));
    }

    #[test]
    fn msi_product_code_shape_is_strict() {
        assert!(looks_like_product_code(
            "{12345678-1234-ABCD-9876-1234567890AB}"
        ));
        assert!(!looks_like_product_code("not-a-product-code"));
    }
}

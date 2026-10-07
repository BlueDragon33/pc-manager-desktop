use pc_core::{DriverCenterError, DriverCenterSnapshot, DriverSettingsLaunchResult};
#[cfg(any(target_os = "windows", test))]
use pc_core::{DriverCenterWarning, DriverInventoryEntry, DriverUpdateCandidate};
#[cfg(any(target_os = "windows", test))]
use serde::Deserialize;
#[cfg(any(target_os = "windows", test))]
use sha2::{Digest, Sha256};

#[cfg(target_os = "windows")]
use std::process::Command;
#[cfg(any(target_os = "windows", test))]
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(any(target_os = "windows", test))]
const DRIVER_CENTER_PROVIDER: &str = "Windows Update Agent";
#[cfg(target_os = "windows")]
const OPTIONAL_UPDATES_URI: &str = "ms-settings:windowsupdate-optionalupdates";
#[cfg(any(target_os = "windows", test))]
const MAX_INSTALLED_DRIVERS: usize = 500;
#[cfg(any(target_os = "windows", test))]
const MAX_AVAILABLE_UPDATES: usize = 100;

#[cfg(target_os = "windows")]
const DRIVER_CENTER_SCRIPT: &str = r#"
$ErrorActionPreference = 'Stop'
$OutputEncoding = [Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false)
$warnings = [System.Collections.Generic.List[object]]::new()
$installedDrivers = @()
$availableUpdates = @()
$providerAvailable = $false

try {
    $installedDrivers = @(
        Get-CimInstance -ClassName Win32_PnPSignedDriver -ErrorAction Stop |
            Where-Object { $_.DeviceName } |
            ForEach-Object {
                $driverDate = $null
                if ($_.DriverDate) {
                    try { $driverDate = ([DateTime]$_.DriverDate).ToUniversalTime().ToString('o') } catch {}
                }

                [pscustomobject]@{
                    deviceId = [string]$_.DeviceID
                    deviceName = [string]$_.DeviceName
                    manufacturer = if ($_.Manufacturer) { [string]$_.Manufacturer } else { $null }
                    provider = if ($_.DriverProviderName) { [string]$_.DriverProviderName } else { $null }
                    version = if ($_.DriverVersion) { [string]$_.DriverVersion } else { $null }
                    driverDate = $driverDate
                    className = if ($_.DeviceClass) { [string]$_.DeviceClass } else { $null }
                    signer = if ($_.Signer) { [string]$_.Signer } else { $null }
                    infName = if ($_.InfName) { [string]$_.InfName } else { $null }
                    isSigned = if ($null -ne $_.IsSigned) { [bool]$_.IsSigned } else { $null }
                }
            }
    )
} catch {
    $warnings.Add([pscustomobject]@{
        code = 'installed_driver_inventory_failed'
        message = ('Installed driver inventory could not be read: ' + $_.Exception.Message)
    })
}

try {
    $session = New-Object -ComObject Microsoft.Update.Session
    $searcher = $session.CreateUpdateSearcher()
    $search = $searcher.Search("IsInstalled=0 and IsHidden=0 and Type='Driver'")
    $providerAvailable = $true

    $availableUpdates = @(
        for ($index = 0; $index -lt $search.Updates.Count; $index++) {
            $update = $search.Updates.Item($index)
            $driverDate = $null
            try {
                if ($update.DriverVerDate) {
                    $driverDate = ([DateTime]$update.DriverVerDate).ToUniversalTime().ToString('o')
                }
            } catch {}

            $updateId = ''
            $revisionNumber = 0
            $hardwareId = ''
            $manufacturer = $null
            $provider = $null
            $model = $null
            $className = $null
            try {
                $updateId = [string]$update.Identity.UpdateID
                $revisionNumber = [int]$update.Identity.RevisionNumber
            } catch {}
            try { $hardwareId = [string]$update.DriverHardwareID } catch {}
            try { if ($update.DriverManufacturer) { $manufacturer = [string]$update.DriverManufacturer } } catch {}
            try { if ($update.DriverProvider) { $provider = [string]$update.DriverProvider } } catch {}
            try { if ($update.DriverModel) { $model = [string]$update.DriverModel } } catch {}
            try { if ($update.DriverClass) { $className = [string]$update.DriverClass } } catch {}

            [pscustomobject]@{
                updateId = $updateId
                revisionNumber = $revisionNumber
                title = [string]$update.Title
                hardwareId = $hardwareId
                manufacturer = $manufacturer
                provider = $provider
                model = $model
                className = $className
                driverDate = $driverDate
                downloaded = [bool]$update.IsDownloaded
                eulaAccepted = [bool]$update.EulaAccepted
                rebootRequired = [bool]$update.RebootRequired
            }
        }
    )
} catch {
    $warnings.Add([pscustomobject]@{
        code = 'windows_update_driver_scan_failed'
        message = ('Windows Update driver discovery is unavailable: ' + $_.Exception.Message)
    })
}

[pscustomobject]@{
    providerAvailable = $providerAvailable
    installedDrivers = @($installedDrivers)
    availableUpdates = @($availableUpdates)
    warnings = @($warnings)
} | ConvertTo-Json -Depth 7 -Compress
"#;

#[cfg(any(target_os = "windows", test))]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawDriverCenterSnapshot {
    #[serde(default)]
    provider_available: bool,
    #[serde(default)]
    installed_drivers: Vec<RawInstalledDriver>,
    #[serde(default)]
    available_updates: Vec<RawDriverUpdate>,
    #[serde(default)]
    warnings: Vec<DriverCenterWarning>,
}

#[cfg(any(target_os = "windows", test))]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawInstalledDriver {
    #[serde(default)]
    device_id: String,
    device_name: String,
    manufacturer: Option<String>,
    provider: Option<String>,
    version: Option<String>,
    driver_date: Option<String>,
    class_name: Option<String>,
    signer: Option<String>,
    inf_name: Option<String>,
    is_signed: Option<bool>,
}

#[cfg(any(target_os = "windows", test))]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawDriverUpdate {
    #[serde(default)]
    update_id: String,
    #[serde(default)]
    revision_number: i64,
    title: String,
    #[serde(default)]
    hardware_id: String,
    manufacturer: Option<String>,
    provider: Option<String>,
    model: Option<String>,
    class_name: Option<String>,
    driver_date: Option<String>,
    #[serde(default)]
    downloaded: bool,
    #[serde(default)]
    eula_accepted: bool,
    #[serde(default)]
    reboot_required: bool,
}

#[cfg(any(target_os = "windows", test))]
fn opaque_id(namespace: &[u8], parts: &[&str]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(namespace);
    for part in parts {
        hasher.update(b"|");
        hasher.update(part.as_bytes());
    }
    let digest = hasher.finalize();
    digest[..16]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>()
}

#[cfg(any(target_os = "windows", test))]
fn normalize_optional(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let trimmed = value.trim();
        (!trimmed.is_empty()).then(|| trimmed.to_string())
    })
}

#[cfg(any(target_os = "windows", test))]
fn generated_epoch_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or_default()
}

#[cfg(any(target_os = "windows", test))]
fn snapshot_from_raw(mut raw: RawDriverCenterSnapshot) -> DriverCenterSnapshot {
    let mut warnings = raw.warnings;

    raw.installed_drivers.sort_by(|left, right| {
        left.device_name
            .to_ascii_lowercase()
            .cmp(&right.device_name.to_ascii_lowercase())
    });
    if raw.installed_drivers.len() > MAX_INSTALLED_DRIVERS {
        raw.installed_drivers.truncate(MAX_INSTALLED_DRIVERS);
        warnings.push(DriverCenterWarning {
            code: "installed_driver_inventory_truncated".to_string(),
            message: format!(
                "Installed driver inventory was limited to {MAX_INSTALLED_DRIVERS} entries."
            ),
        });
    }

    raw.available_updates.sort_by(|left, right| {
        left.title
            .to_ascii_lowercase()
            .cmp(&right.title.to_ascii_lowercase())
    });
    if raw.available_updates.len() > MAX_AVAILABLE_UPDATES {
        raw.available_updates.truncate(MAX_AVAILABLE_UPDATES);
        warnings.push(DriverCenterWarning {
            code: "driver_update_inventory_truncated".to_string(),
            message: format!(
                "Windows Update results were limited to {MAX_AVAILABLE_UPDATES} driver updates."
            ),
        });
    }

    let installed_drivers = raw
        .installed_drivers
        .into_iter()
        .map(|driver| DriverInventoryEntry {
            id: opaque_id(
                b"pc-manager-driver-inventory-v1",
                &[
                    &driver.device_id,
                    driver.inf_name.as_deref().unwrap_or_default(),
                    driver.version.as_deref().unwrap_or_default(),
                ],
            ),
            device_name: driver.device_name.trim().to_string(),
            manufacturer: normalize_optional(driver.manufacturer),
            provider: normalize_optional(driver.provider),
            version: normalize_optional(driver.version),
            driver_date: normalize_optional(driver.driver_date),
            class_name: normalize_optional(driver.class_name),
            signer: normalize_optional(driver.signer),
            inf_name: normalize_optional(driver.inf_name),
            is_signed: driver.is_signed,
        })
        .filter(|driver| !driver.device_name.is_empty())
        .collect::<Vec<_>>();

    let available_updates = raw
        .available_updates
        .into_iter()
        .filter(|update| !update.title.trim().is_empty())
        .map(|update| {
            let revision = update.revision_number.to_string();
            DriverUpdateCandidate {
                id: opaque_id(
                    b"pc-manager-driver-update-v1",
                    &[&update.update_id, &revision, &update.hardware_id, &update.title],
                ),
                title: update.title.trim().to_string(),
                manufacturer: normalize_optional(update.manufacturer),
                provider: normalize_optional(update.provider),
                model: normalize_optional(update.model),
                class_name: normalize_optional(update.class_name),
                driver_date: normalize_optional(update.driver_date),
                downloaded: update.downloaded,
                eula_accepted: update.eula_accepted,
                reboot_required: update.reboot_required,
                source: "Windows Update".to_string(),
                recommendation:
                    "Windows Update reports this driver as applicable to this PC. Review it in Windows Settings; PC Manager does not recommend a driver merely because it is old."
                        .to_string(),
            }
        })
        .collect::<Vec<_>>();

    DriverCenterSnapshot {
        provider: DRIVER_CENTER_PROVIDER.to_string(),
        provider_available: raw.provider_available,
        source_detail: if raw.provider_available {
            "Applicable driver updates are reported by the local Windows Update Agent; PC Manager does not download or install driver packages directly."
                .to_string()
        } else {
            "Windows Update driver discovery is unavailable. Installed driver inventory may still be shown, but PC Manager will not invent update recommendations."
                .to_string()
        },
        installed_drivers,
        available_updates,
        warnings,
        generated_epoch_ms: generated_epoch_ms(),
    }
}

#[cfg(target_os = "windows")]
pub fn scan_driver_center() -> Result<DriverCenterSnapshot, DriverCenterError> {
    let output = Command::new("powershell.exe")
        .args([
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            DRIVER_CENTER_SCRIPT,
        ])
        .output()
        .map_err(|error| {
            DriverCenterError::new(
                "driver_center_provider_unavailable",
                format!("The Windows driver provider could not be started: {error}"),
                true,
            )
        })?;

    if !output.status.success() {
        let detail = String::from_utf8_lossy(&output.stderr)
            .split_whitespace()
            .take(48)
            .collect::<Vec<_>>()
            .join(" ");
        return Err(DriverCenterError::new(
            "driver_center_scan_failed",
            if detail.is_empty() {
                "The Windows driver scan failed without structured output.".to_string()
            } else {
                format!("The Windows driver scan failed: {detail}")
            },
            true,
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let start = stdout.find('{').ok_or_else(|| {
        DriverCenterError::new(
            "driver_center_output_invalid",
            "The Windows driver provider returned no structured result.",
            true,
        )
    })?;
    let end = stdout.rfind('}').ok_or_else(|| {
        DriverCenterError::new(
            "driver_center_output_invalid",
            "The Windows driver provider returned incomplete structured output.",
            true,
        )
    })?;

    let raw: RawDriverCenterSnapshot =
        serde_json::from_str(&stdout[start..=end]).map_err(|error| {
            DriverCenterError::new(
                "driver_center_output_invalid",
                format!("The Windows driver result could not be parsed: {error}"),
                true,
            )
        })?;

    Ok(snapshot_from_raw(raw))
}

#[cfg(not(target_os = "windows"))]
pub fn scan_driver_center() -> Result<DriverCenterSnapshot, DriverCenterError> {
    Err(DriverCenterError::new(
        "unsupported_platform",
        "Driver Center is currently implemented for Windows only.",
        false,
    ))
}

#[cfg(target_os = "windows")]
pub fn open_driver_update_settings() -> Result<DriverSettingsLaunchResult, DriverCenterError> {
    Command::new("explorer.exe")
        .arg(OPTIONAL_UPDATES_URI)
        .spawn()
        .map_err(|error| {
            DriverCenterError::new(
                "driver_settings_launch_failed",
                format!("Windows Optional updates could not be opened: {error}"),
                true,
            )
        })?;

    Ok(DriverSettingsLaunchResult {
        launched: true,
        target: "Windows Update > Optional updates".to_string(),
        message:
            "Windows Settings was opened. Review driver updates there; PC Manager did not download or execute a driver installer."
                .to_string(),
    })
}

#[cfg(not(target_os = "windows"))]
pub fn open_driver_update_settings() -> Result<DriverSettingsLaunchResult, DriverCenterError> {
    Err(DriverCenterError::new(
        "unsupported_platform",
        "Windows Optional updates can only be opened on Windows.",
        false,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_raw() -> RawDriverCenterSnapshot {
        serde_json::from_str(
            r#"{
                "providerAvailable": true,
                "installedDrivers": [
                    {
                        "deviceId": "PCI\\VEN_1234&DEV_ABCD",
                        "deviceName": "Example Adapter",
                        "manufacturer": "Example Corp",
                        "provider": "Example Corp",
                        "version": "1.2.3.4",
                        "driverDate": "2025-01-02T00:00:00.0000000Z",
                        "className": "NET",
                        "signer": "Microsoft Windows Hardware Compatibility Publisher",
                        "infName": "oem42.inf",
                        "isSigned": true
                    }
                ],
                "availableUpdates": [
                    {
                        "updateId": "11111111-2222-3333-4444-555555555555",
                        "revisionNumber": 7,
                        "title": "Example Corp - Net - 2.0.0.0",
                        "hardwareId": "PCI\\VEN_1234&DEV_ABCD",
                        "manufacturer": "Example Corp",
                        "provider": "Example Corp",
                        "model": "Example Adapter",
                        "className": "Net",
                        "driverDate": "2026-01-02T00:00:00.0000000Z",
                        "downloaded": false,
                        "eulaAccepted": false,
                        "rebootRequired": false
                    }
                ],
                "warnings": []
            }"#,
        )
        .expect("sample should parse")
    }

    #[test]
    fn builds_opaque_ids_without_exposing_device_identity() {
        let snapshot = snapshot_from_raw(sample_raw());
        assert_eq!(snapshot.installed_drivers.len(), 1);
        assert_eq!(snapshot.available_updates.len(), 1);
        assert_eq!(snapshot.installed_drivers[0].id.len(), 32);
        assert_eq!(snapshot.available_updates[0].id.len(), 32);
        assert!(!snapshot.installed_drivers[0].id.contains("VEN_1234"));
        assert!(!snapshot.available_updates[0].id.contains("11111111"));
    }

    #[test]
    fn recommendation_uses_applicability_not_driver_age() {
        let snapshot = snapshot_from_raw(sample_raw());
        let reason = &snapshot.available_updates[0].recommendation;
        assert!(reason.contains("applicable to this PC"));
        assert!(reason.contains("does not recommend a driver merely because it is old"));
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn windows_driver_provider_smoke_is_read_only() {
        match scan_driver_center() {
            Ok(snapshot) => {
                assert_eq!(snapshot.provider, DRIVER_CENTER_PROVIDER);
                assert!(snapshot.installed_drivers.len() <= MAX_INSTALLED_DRIVERS);
                assert!(snapshot.available_updates.len() <= MAX_AVAILABLE_UPDATES);
            }
            Err(error) => {
                assert!(
                    error.code.starts_with("driver_center_"),
                    "unexpected Driver Center error: {}",
                    error.code
                );
            }
        }
    }

    #[test]
    fn unavailable_windows_update_does_not_invent_candidates() {
        let raw: RawDriverCenterSnapshot = serde_json::from_str(
            r#"{
                "providerAvailable": false,
                "installedDrivers": [],
                "availableUpdates": [],
                "warnings": [{"code":"windows_update_driver_scan_failed","message":"disabled"}]
            }"#,
        )
        .expect("sample should parse");

        let snapshot = snapshot_from_raw(raw);
        assert!(!snapshot.provider_available);
        assert!(snapshot.available_updates.is_empty());
        assert_eq!(snapshot.warnings.len(), 1);
    }
}

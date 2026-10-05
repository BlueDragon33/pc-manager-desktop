//! Windows-specific system integration for PC Manager Desktop.
//!
//! Windows integration remains narrowly scoped. P4B adds user-confirmed cleanup execution
//! only for candidates produced by the native allow-listed cleanup scanner.

mod cleanup;
pub use cleanup::{execute_cleanup_plan, scan_cleanup_candidates};

#[cfg(any(target_os = "windows", test))]
use pc_core::InstalledApplication;
#[cfg(target_os = "windows")]
use pc_core::{
    CpuSummary, DeviceIdentity, DiskVolume, InventoryWarning, MemorySummary, NetworkAdapter,
    OperatingSystemSummary, ProcessEntry, ProcessSummary, StartupItem,
};
use pc_core::{InventoryError, SystemInventory};
#[cfg(target_os = "windows")]
use serde::Deserialize;
use sha2::{Digest, Sha256};
#[cfg(any(target_os = "windows", test))]
use std::collections::BTreeMap;
#[cfg(target_os = "windows")]
use std::time::{SystemTime, UNIX_EPOCH};

/// Returns this crate's stable component name.
#[must_use]
pub const fn component_name() -> &'static str {
    "pc-windows"
}

/// Derives a one-way local identifier without exposing the raw Windows MachineGuid.
#[must_use]
pub fn derive_device_id(machine_guid: &str) -> String {
    let normalized = machine_guid.trim().to_ascii_lowercase();
    let mut hasher = Sha256::new();
    hasher.update(b"pc-manager-device-id-v1:");
    hasher.update(normalized.as_bytes());
    let digest = hasher.finalize();

    digest[..16]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>()
}

#[cfg(target_os = "windows")]
pub fn collect_system_inventory() -> Result<SystemInventory, InventoryError> {
    windows::collect()
}

#[cfg(not(target_os = "windows"))]
pub fn collect_system_inventory() -> Result<SystemInventory, InventoryError> {
    Err(InventoryError::new(
        "unsupported_platform",
        "System inventory is currently implemented for Windows only.",
        false,
    ))
}

#[cfg(target_os = "windows")]
mod windows {
    use super::*;
    use std::process::Command;

    const INVENTORY_SCRIPT: &str = r#"
$ErrorActionPreference = 'Stop'
$warnings = [System.Collections.Generic.List[object]]::new()

function Try-Read([string]$source, [scriptblock]$action, $fallback) {
  try { & $action }
  catch {
    $warnings.Add([pscustomobject]@{ source = $source; message = $_.Exception.Message })
    $fallback
  }
}

$os = Try-Read 'operatingSystem' {
  Get-CimInstance Win32_OperatingSystem |
    Select-Object -First 1 Caption, Version, BuildNumber, OSArchitecture, TotalVisibleMemorySize, FreePhysicalMemory
} $null

$cpu = @(Try-Read 'cpu' {
  @(Get-CimInstance Win32_Processor |
    Select-Object Name, NumberOfCores, NumberOfLogicalProcessors)
} @())

$volumes = @(Try-Read 'volumes' {
  @(Get-CimInstance Win32_LogicalDisk -Filter "DriveType=3" |
    Select-Object DeviceID, VolumeName, FileSystem, Size, FreeSpace)
} @())

$processes = @(Try-Read 'processes' {
  @(Get-Process |
    Sort-Object WorkingSet64 -Descending |
    Select-Object -First 10 Id, ProcessName, WorkingSet64)
} @())

$processCount = Try-Read 'processCount' { @(Get-Process).Count } 0

$installedApps = @(Try-Read 'installedApplications' {
  $paths = @(
    'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\*',
    'HKLM:\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\*',
    'HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\*'
  )
  @($paths | ForEach-Object {
    Get-ItemProperty -Path $_ -ErrorAction SilentlyContinue |
      Where-Object { -not [string]::IsNullOrWhiteSpace($_.DisplayName) } |
      Select-Object DisplayName, DisplayVersion, Publisher
  })
} @())

$startupItems = @(Try-Read 'startup' {
  @(Get-CimInstance Win32_StartupCommand |
    Select-Object Name, Command, Location)
} @())

$networkAdapters = @(Try-Read 'networkAdapters' {
  @(Get-CimInstance Win32_NetworkAdapter |
    Where-Object { $_.NetConnectionID -or $_.NetEnabled -ne $null } |
    Select-Object Name, NetConnectionID, NetEnabled, Speed)
} @())

$machineGuid = Try-Read 'deviceIdentity' {
  Get-ItemPropertyValue -Path 'HKLM:\SOFTWARE\Microsoft\Cryptography' -Name MachineGuid
} $null

[pscustomobject]@{
  hostname = $env:COMPUTERNAME
  machineGuid = $machineGuid
  operatingSystem = $os
  cpu = $cpu
  volumes = $volumes
  processes = $processes
  processCount = [int]$processCount
  installedApplications = $installedApps
  startupItems = $startupItems
  networkAdapters = $networkAdapters
  warnings = @($warnings)
} | ConvertTo-Json -Depth 6 -Compress
"#;

    pub(super) fn collect() -> Result<SystemInventory, InventoryError> {
        let output = Command::new("powershell.exe")
            .args([
                "-NoLogo",
                "-NoProfile",
                "-NonInteractive",
                "-ExecutionPolicy",
                "Bypass",
                "-Command",
                INVENTORY_SCRIPT,
            ])
            .output()
            .map_err(|error| {
                InventoryError::new(
                    "powershell_unavailable",
                    format!("Unable to start the Windows inventory provider: {error}"),
                    true,
                )
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(InventoryError::new(
                "inventory_provider_failed",
                if stderr.trim().is_empty() {
                    "The Windows inventory provider exited with an error.".to_string()
                } else {
                    format!("The Windows inventory provider failed: {}", stderr.trim())
                },
                true,
            ));
        }

        let stdout = String::from_utf8(output.stdout).map_err(|error| {
            InventoryError::new(
                "invalid_inventory_encoding",
                format!("Inventory output was not valid UTF-8: {error}"),
                true,
            )
        })?;

        let raw: RawInventory = serde_json::from_str(stdout.trim()).map_err(|error| {
            InventoryError::new(
                "invalid_inventory_payload",
                format!("Unable to parse the Windows inventory payload: {error}"),
                true,
            )
        })?;

        Ok(map_inventory(raw))
    }
}

#[cfg(target_os = "windows")]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawInventory {
    hostname: Option<String>,
    machine_guid: Option<String>,
    operating_system: Option<RawOperatingSystem>,
    #[serde(default)]
    cpu: Vec<RawCpu>,
    #[serde(default)]
    volumes: Vec<RawVolume>,
    #[serde(default)]
    processes: Vec<RawProcess>,
    #[serde(default)]
    process_count: usize,
    #[serde(default)]
    installed_applications: Vec<RawInstalledApplication>,
    #[serde(default)]
    startup_items: Vec<RawStartupItem>,
    #[serde(default)]
    network_adapters: Vec<RawNetworkAdapter>,
    #[serde(default)]
    warnings: Vec<RawWarning>,
}

#[cfg(target_os = "windows")]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct RawOperatingSystem {
    caption: Option<String>,
    version: Option<String>,
    build_number: Option<String>,
    #[serde(rename = "OSArchitecture")]
    os_architecture: Option<String>,
    total_visible_memory_size: Option<u64>,
    free_physical_memory: Option<u64>,
}

#[cfg(target_os = "windows")]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct RawCpu {
    name: Option<String>,
    number_of_cores: Option<u32>,
    number_of_logical_processors: Option<u32>,
}

#[cfg(target_os = "windows")]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct RawVolume {
    #[serde(rename = "DeviceID")]
    device_id: Option<String>,
    volume_name: Option<String>,
    file_system: Option<String>,
    size: Option<u64>,
    free_space: Option<u64>,
}

#[cfg(target_os = "windows")]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct RawProcess {
    id: Option<u32>,
    process_name: Option<String>,
    working_set64: Option<u64>,
}

#[cfg(any(target_os = "windows", test))]
#[derive(Debug, Clone)]
#[cfg_attr(target_os = "windows", derive(Deserialize))]
#[cfg_attr(target_os = "windows", serde(rename_all = "PascalCase"))]
struct RawInstalledApplication {
    display_name: Option<String>,
    display_version: Option<String>,
    publisher: Option<String>,
}

#[cfg(target_os = "windows")]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct RawStartupItem {
    name: Option<String>,
    command: Option<String>,
    location: Option<String>,
}

#[cfg(target_os = "windows")]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct RawNetworkAdapter {
    name: Option<String>,
    #[serde(rename = "NetConnectionID")]
    net_connection_id: Option<String>,
    net_enabled: Option<bool>,
    speed: Option<u64>,
}

#[cfg(target_os = "windows")]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawWarning {
    source: String,
    message: String,
}

#[cfg(target_os = "windows")]
fn map_inventory(raw: RawInventory) -> SystemInventory {
    let mut warnings = raw
        .warnings
        .into_iter()
        .map(|warning| InventoryWarning {
            source: warning.source,
            message: warning.message,
        })
        .collect::<Vec<_>>();

    let memory = raw
        .operating_system
        .as_ref()
        .map(|os| MemorySummary {
            total_bytes: os
                .total_visible_memory_size
                .unwrap_or_default()
                .saturating_mul(1024),
            available_bytes: os
                .free_physical_memory
                .unwrap_or_default()
                .saturating_mul(1024),
        })
        .unwrap_or(MemorySummary {
            total_bytes: 0,
            available_bytes: 0,
        });

    let operating_system = match raw.operating_system {
        Some(os) => OperatingSystemSummary {
            name: non_empty(os.caption).unwrap_or_else(|| "Windows".to_string()),
            version: non_empty(os.version).unwrap_or_else(|| "Unknown".to_string()),
            build_number: non_empty(os.build_number),
            architecture: non_empty(os.os_architecture)
                .unwrap_or_else(|| std::env::consts::ARCH.to_string()),
        },
        None => {
            warnings.push(InventoryWarning {
                source: "operatingSystem".to_string(),
                message: "Operating-system details were unavailable.".to_string(),
            });
            OperatingSystemSummary {
                name: "Windows".to_string(),
                version: "Unknown".to_string(),
                build_number: None,
                architecture: std::env::consts::ARCH.to_string(),
            }
        }
    };

    let cpu = CpuSummary {
        brand: raw
            .cpu
            .iter()
            .find_map(|entry| non_empty(entry.name.clone()))
            .unwrap_or_else(|| "Unknown CPU".to_string()),
        physical_cores: sum_optional(raw.cpu.iter().map(|entry| entry.number_of_cores)),
        logical_cores: raw
            .cpu
            .iter()
            .filter_map(|entry| entry.number_of_logical_processors)
            .sum(),
        architecture: operating_system.architecture.clone(),
    };

    let local_device_id = raw
        .machine_guid
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .map(derive_device_id);

    if local_device_id.is_none() {
        warnings.push(InventoryWarning {
            source: "deviceIdentity".to_string(),
            message: "A stable local device identifier could not be derived.".to_string(),
        });
    }

    let volumes = raw
        .volumes
        .into_iter()
        .filter_map(|volume| {
            let name = non_empty(volume.device_id)?;
            Some(DiskVolume {
                name,
                label: non_empty(volume.volume_name),
                file_system: non_empty(volume.file_system),
                total_bytes: volume.size.unwrap_or_default(),
                available_bytes: volume.free_space.unwrap_or_default(),
            })
        })
        .collect();

    let top_memory = raw
        .processes
        .into_iter()
        .filter_map(|process| {
            Some(ProcessEntry {
                pid: process.id?,
                name: non_empty(process.process_name)?,
                memory_bytes: process.working_set64.unwrap_or_default(),
            })
        })
        .collect();

    let installed_applications = deduplicate_apps(raw.installed_applications);

    let startup_items = raw
        .startup_items
        .into_iter()
        .filter_map(|item| {
            Some(StartupItem {
                name: non_empty(item.name)?,
                source: "Windows startup inventory".to_string(),
                location: non_empty(item.location),
                command: non_empty(item.command),
            })
        })
        .collect();

    let network_adapters = raw
        .network_adapters
        .into_iter()
        .filter_map(|adapter| {
            let description = non_empty(adapter.name);
            let name =
                non_empty(adapter.net_connection_id.clone()).or_else(|| description.clone())?;

            Some(NetworkAdapter {
                name,
                description,
                enabled: adapter.net_enabled,
                link_speed_bps: adapter.speed,
            })
        })
        .collect();

    SystemInventory {
        collected_at_epoch_ms: now_epoch_ms(),
        operating_system,
        device: DeviceIdentity {
            hostname: non_empty(raw.hostname).unwrap_or_else(|| "Unknown".to_string()),
            local_device_id,
        },
        cpu,
        memory,
        volumes,
        processes: ProcessSummary {
            total_count: raw.process_count,
            top_memory,
        },
        installed_applications,
        startup_items,
        network_adapters,
        warnings,
    }
}

#[cfg(any(target_os = "windows", test))]
fn deduplicate_apps(apps: Vec<RawInstalledApplication>) -> Vec<InstalledApplication> {
    let mut deduplicated = BTreeMap::new();

    for app in apps {
        let Some(name) = non_empty(app.display_name) else {
            continue;
        };
        let version = non_empty(app.display_version);
        let publisher = non_empty(app.publisher);
        let key = format!(
            "{}|{}|{}",
            name.to_ascii_lowercase(),
            version.clone().unwrap_or_default().to_ascii_lowercase(),
            publisher.clone().unwrap_or_default().to_ascii_lowercase()
        );

        deduplicated.entry(key).or_insert(InstalledApplication {
            name,
            version,
            publisher,
        });
    }

    deduplicated.into_values().collect()
}

#[cfg(any(target_os = "windows", test))]
fn non_empty(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let trimmed = value.trim();
        (!trimmed.is_empty()).then(|| trimmed.to_string())
    })
}

#[cfg(any(target_os = "windows", test))]
fn sum_optional(values: impl Iterator<Item = Option<u32>>) -> Option<u32> {
    let mut found = false;
    let mut total = 0_u32;

    for value in values.flatten() {
        found = true;
        total = total.saturating_add(value);
    }

    found.then_some(total)
}

#[cfg(target_os = "windows")]
fn now_epoch_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::{deduplicate_apps, derive_device_id, sum_optional, RawInstalledApplication};

    #[test]
    fn device_id_is_stable_and_does_not_echo_machine_guid() {
        let first = derive_device_id("  ABCD-1234  ");
        let second = derive_device_id("abcd-1234");

        assert_eq!(first, second);
        assert_eq!(first.len(), 32);
        assert!(!first.contains("abcd"));
    }

    #[test]
    fn duplicate_application_records_are_collapsed() {
        let apps = vec![
            RawInstalledApplication {
                display_name: Some("Example App".to_string()),
                display_version: Some("1.0".to_string()),
                publisher: Some("Example".to_string()),
            },
            RawInstalledApplication {
                display_name: Some("Example App".to_string()),
                display_version: Some("1.0".to_string()),
                publisher: Some("Example".to_string()),
            },
        ];

        let mapped = deduplicate_apps(apps);
        assert_eq!(mapped.len(), 1);
        assert_eq!(mapped[0].name, "Example App");
    }

    #[test]
    fn optional_cpu_counts_remain_optional_when_unavailable() {
        assert_eq!(sum_optional([None, None].into_iter()), None);
        assert_eq!(sum_optional([Some(4), Some(6)].into_iter()), Some(10));
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn windows_inventory_provider_returns_real_machine_data() {
        let inventory =
            super::collect_system_inventory().expect("Windows inventory provider should run");

        assert!(!inventory.device.hostname.trim().is_empty());
        assert!(!inventory.operating_system.name.trim().is_empty());
        assert!(inventory.memory.total_bytes > 0);
        assert!(inventory.cpu.logical_cores > 0);
        assert!(!inventory.volumes.is_empty());
    }
}

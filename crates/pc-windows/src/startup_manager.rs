use pc_core::{
    StartupChangeRequest, StartupEntry, StartupError, StartupImpact, StartupOperationRecord,
    StartupSourceType,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
#[cfg(target_os = "windows")]
use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
#[cfg(target_os = "windows")]
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

const STARTUP_AUDIT_FILE: &str = "startup-operations.jsonl";

#[derive(Debug, Clone)]
struct NativeStartupEntry {
    public: StartupEntry,
    #[cfg(target_os = "windows")]
    registry_key: Option<String>,
    #[cfg(target_os = "windows")]
    registry_value_name: Option<String>,
    startup_path: Option<PathBuf>,
    #[cfg(target_os = "windows")]
    scheduled_task_name: Option<String>,
    #[cfg(target_os = "windows")]
    scheduled_task_path: Option<String>,
}

#[cfg(target_os = "windows")]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawStartupEntry {
    source_type: String,
    identity: String,
    name: String,
    command: String,
    enabled: bool,
    can_change: bool,
    requires_elevation: bool,
    detail: String,
    registry_key: Option<String>,
    registry_value_name: Option<String>,
    startup_path: Option<String>,
    scheduled_task_name: Option<String>,
    scheduled_task_path: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DisabledFolderMeta {
    entry_id: String,
    name: String,
    original_path: String,
    disabled_path: String,
    command: String,
}

#[cfg(target_os = "windows")]
const STARTUP_INVENTORY_SCRIPT: &str = r#"
$ErrorActionPreference = 'Stop'
$entries = [System.Collections.Generic.List[object]]::new()

function Add-RegistryEntries(
  [string]$sourceType,
  [string]$keyPath,
  [bool]$canChange,
  [bool]$requiresElevation,
  [string]$label
) {
  if (-not (Test-Path -LiteralPath $keyPath)) { return }

  $item = Get-ItemProperty -LiteralPath $keyPath -ErrorAction Stop
  foreach ($property in $item.PSObject.Properties) {
    if ($property.Name -like 'PS*') { continue }
    if ($property.Value -isnot [string]) { continue }
    $entries.Add([pscustomobject]@{
      sourceType = $sourceType
      identity = "$keyPath|$($property.Name)"
      name = $property.Name
      command = [string]$property.Value
      enabled = $true
      canChange = $canChange
      requiresElevation = $requiresElevation
      detail = $label
      registryKey = $keyPath
      registryValueName = $property.Name
      startupPath = $null
      scheduledTaskName = $null
      scheduledTaskPath = $null
    })
  }
}

Add-RegistryEntries 'registryCurrentUserRun' 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run' $true $false 'Current user Run'
Add-RegistryEntries 'registryCurrentUserRunOnce' 'HKCU:\Software\Microsoft\Windows\CurrentVersion\RunOnce' $true $false 'Current user RunOnce'
Add-RegistryEntries 'registryLocalMachineRun' 'HKLM:\Software\Microsoft\Windows\CurrentVersion\Run' $false $true 'All users Run'
Add-RegistryEntries 'registryLocalMachineRunOnce' 'HKLM:\Software\Microsoft\Windows\CurrentVersion\RunOnce' $false $true 'All users RunOnce'
Add-RegistryEntries 'registryLocalMachineRun' 'HKLM:\Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Run' $false $true 'All users Run (32-bit)'
Add-RegistryEntries 'registryLocalMachineRunOnce' 'HKLM:\Software\WOW6432Node\Microsoft\Windows\CurrentVersion\RunOnce' $false $true 'All users RunOnce (32-bit)'

$backupRoot = 'HKCU:\Software\PCManager\StartupBackup'
if (Test-Path -LiteralPath $backupRoot) {
  foreach ($backupKey in Get-ChildItem -LiteralPath $backupRoot -ErrorAction SilentlyContinue) {
    try {
      $meta = Get-ItemProperty -LiteralPath $backupKey.PSPath -ErrorAction Stop
      if ($meta.SourceType -and $meta.OriginalKey -and $meta.Name -and $meta.Command) {
        $entries.Add([pscustomobject]@{
          sourceType = [string]$meta.SourceType
          identity = "$($meta.OriginalKey)|$($meta.Name)"
          name = [string]$meta.Name
          command = [string]$meta.Command
          enabled = $false
          canChange = $true
          requiresElevation = $false
          detail = 'Disabled by PC Manager; original registry value preserved'
          registryKey = [string]$meta.OriginalKey
          registryValueName = [string]$meta.Name
          startupPath = $null
          scheduledTaskName = $null
          scheduledTaskPath = $null
        })
      }
    } catch {}
  }
}

$userStartup = [Environment]::GetFolderPath('Startup')
$commonStartup = [Environment]::GetFolderPath('CommonStartup')

foreach ($folderSpec in @(
  [pscustomobject]@{ SourceType='userStartupFolder'; Path=$userStartup; CanChange=$true; RequiresElevation=$false; Label='Current user Startup folder' },
  [pscustomobject]@{ SourceType='commonStartupFolder'; Path=$commonStartup; CanChange=$false; RequiresElevation=$true; Label='All users Startup folder' }
)) {
  if ([string]::IsNullOrWhiteSpace($folderSpec.Path)) { continue }
  if (-not (Test-Path -LiteralPath $folderSpec.Path)) { continue }

  foreach ($file in Get-ChildItem -LiteralPath $folderSpec.Path -File -Force -ErrorAction SilentlyContinue) {
    $entries.Add([pscustomobject]@{
      sourceType = $folderSpec.SourceType
      identity = $file.FullName
      name = $file.Name
      command = $file.FullName
      enabled = $true
      canChange = $folderSpec.CanChange
      requiresElevation = $folderSpec.RequiresElevation
      detail = $folderSpec.Label
      registryKey = $null
      registryValueName = $null
      startupPath = $file.FullName
      scheduledTaskName = $null
      scheduledTaskPath = $null
    })
  }
}

try {
  foreach ($task in Get-ScheduledTask -ErrorAction Stop) {
    if ($task.TaskPath -like '\Microsoft\Windows\*') { continue }

    $hasStartupTrigger = $false
    foreach ($trigger in @($task.Triggers)) {
      $kind = $trigger.CimClass.CimClassName
      if ($kind -eq 'MSFT_TaskLogonTrigger' -or $kind -eq 'MSFT_TaskBootTrigger') {
        $hasStartupTrigger = $true
        break
      }
    }
    if (-not $hasStartupTrigger) { continue }

    $requiresElevation = $false
    try {
      $requiresElevation = ([string]$task.Principal.RunLevel -eq 'Highest')
    } catch {}

    $commandSummary = (@($task.Actions) | ForEach-Object {
      if ($_.Execute) {
        if ($_.Arguments) { "$($_.Execute) $($_.Arguments)" } else { [string]$_.Execute }
      }
    }) -join '; '

    $entries.Add([pscustomobject]@{
      sourceType = 'scheduledTask'
      identity = "$($task.TaskPath)|$($task.TaskName)"
      name = $task.TaskName
      command = $commandSummary
      enabled = ([string]$task.State -ne 'Disabled')
      canChange = (-not $requiresElevation)
      requiresElevation = $requiresElevation
      detail = "Scheduled Task $($task.TaskPath)"
      registryKey = $null
      registryValueName = $null
      startupPath = $null
      scheduledTaskName = $task.TaskName
      scheduledTaskPath = $task.TaskPath
    })
  }
} catch {}

@($entries) | ConvertTo-Json -Depth 6 -Compress
"#;

pub fn list_startup_entries() -> Result<Vec<StartupEntry>, StartupError> {
    Ok(discover_native_entries()?
        .into_iter()
        .map(|entry| entry.public)
        .collect())
}

pub fn set_startup_entry_enabled(
    request: &StartupChangeRequest,
) -> Result<StartupOperationRecord, StartupError> {
    let entries = discover_native_entries()?;
    let entry = entries
        .into_iter()
        .find(|entry| entry.public.id == request.entry_id)
        .ok_or_else(|| {
            StartupError::new(
                "startup_entry_not_found",
                "The startup entry changed or no longer exists. Refresh the Startup page.",
                true,
            )
        })?;

    if !entry.public.can_change {
        return Err(StartupError::new(
            "startup_entry_read_only",
            if entry.public.requires_elevation {
                "This startup entry requires elevation and is read-only until the privileged service is available."
            } else {
                "This startup entry is read-only in the current implementation."
            },
            false,
        ));
    }

    let previous_enabled = entry.public.enabled;
    if previous_enabled == request.enabled {
        return Err(StartupError::new(
            "startup_state_unchanged",
            "The startup entry is already in the requested state.",
            true,
        ));
    }

    let result = match entry.public.source_type {
        StartupSourceType::RegistryCurrentUserRun
        | StartupSourceType::RegistryCurrentUserRunOnce => {
            set_current_user_registry_enabled(&entry, request.enabled)
        }
        StartupSourceType::UserStartupFolder => {
            set_user_startup_file_enabled(&entry, request.enabled)
        }
        StartupSourceType::ScheduledTask => set_scheduled_task_enabled(&entry, request.enabled),
        StartupSourceType::RegistryLocalMachineRun
        | StartupSourceType::RegistryLocalMachineRunOnce
        | StartupSourceType::CommonStartupFolder => Err(StartupError::new(
            "startup_entry_requires_elevation",
            "This startup source is not changed by the unprivileged desktop process.",
            false,
        )),
    };

    let (success, message) = match result {
        Ok(message) => (true, message),
        Err(error) => {
            let record = operation_record(
                &entry,
                previous_enabled,
                request.enabled,
                false,
                error.message.clone(),
            );
            append_operation(&record)?;
            return Err(error);
        }
    };

    let record = operation_record(&entry, previous_enabled, request.enabled, success, message);
    append_operation(&record)?;
    Ok(record)
}

pub fn list_startup_operations() -> Result<Vec<StartupOperationRecord>, StartupError> {
    let path = startup_audit_path()?;
    if !path.exists() {
        return Ok(Vec::new());
    }

    let file = fs::File::open(&path).map_err(|error| {
        StartupError::new(
            "startup_audit_open_failed",
            format!("Unable to open Startup Manager history: {error}"),
            true,
        )
    })?;

    let mut records = Vec::new();
    for line in BufReader::new(file).lines() {
        let line = match line {
            Ok(line) if !line.trim().is_empty() => line,
            Ok(_) => continue,
            Err(_) => continue,
        };
        if let Ok(record) = serde_json::from_str::<StartupOperationRecord>(&line) {
            records.push(record);
        }
    }
    records.sort_by(|left, right| right.completed_at_epoch_ms.cmp(&left.completed_at_epoch_ms));
    Ok(records)
}

fn discover_native_entries() -> Result<Vec<NativeStartupEntry>, StartupError> {
    #[cfg(target_os = "windows")]
    {
        let output = Command::new("powershell.exe")
            .args([
                "-NoLogo",
                "-NoProfile",
                "-NonInteractive",
                "-ExecutionPolicy",
                "Bypass",
                "-Command",
                STARTUP_INVENTORY_SCRIPT,
            ])
            .output()
            .map_err(|error| {
                StartupError::new(
                    "startup_provider_unavailable",
                    format!("Unable to start the Startup Manager provider: {error}"),
                    true,
                )
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(StartupError::new(
                "startup_provider_failed",
                if stderr.trim().is_empty() {
                    "The Startup Manager provider exited with an error.".to_string()
                } else {
                    format!("The Startup Manager provider failed: {}", stderr.trim())
                },
                true,
            ));
        }

        let stdout = String::from_utf8(output.stdout).map_err(|error| {
            StartupError::new(
                "startup_provider_encoding",
                format!("Startup Manager output was not valid UTF-8: {error}"),
                true,
            )
        })?;

        let trimmed = stdout.trim();
        let raw: Vec<RawStartupEntry> = if trimmed.is_empty() || trimmed == "null" {
            Vec::new()
        } else {
            let parsed = if trimmed.starts_with('[') {
                serde_json::from_str(trimmed)
            } else {
                serde_json::from_str::<RawStartupEntry>(trimmed).map(|entry| vec![entry])
            };

            parsed.map_err(|error| {
                StartupError::new(
                    "startup_provider_payload",
                    format!("Unable to parse Startup Manager data: {error}"),
                    true,
                )
            })?
        };

        let mut entries = raw
            .into_iter()
            .filter_map(map_raw_entry)
            .collect::<Vec<_>>();
        entries.extend(discover_disabled_startup_files()?);

        let mut deduplicated = BTreeMap::new();
        for entry in entries {
            deduplicated.entry(entry.public.id.clone()).or_insert(entry);
        }
        Ok(deduplicated.into_values().collect())
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err(StartupError::new(
            "unsupported_platform",
            "Startup Manager is currently implemented for Windows only.",
            false,
        ))
    }
}

#[cfg(target_os = "windows")]
fn map_raw_entry(raw: RawStartupEntry) -> Option<NativeStartupEntry> {
    let source_type = parse_source_type(&raw.source_type)?;
    let identity = raw.identity.trim().to_string();
    if identity.is_empty() || raw.name.trim().is_empty() {
        return None;
    }

    let id = startup_entry_id(source_type, &identity);

    Some(NativeStartupEntry {
        public: StartupEntry {
            id,
            name: raw.name.trim().to_string(),
            source_type,
            source_label: source_label(source_type).to_string(),
            command: raw.command.trim().to_string(),
            publisher: None,
            enabled: raw.enabled,
            can_change: raw.can_change,
            requires_elevation: raw.requires_elevation,
            impact: StartupImpact::Unknown,
            impact_evidence: None,
            detail: raw.detail,
        },
        registry_key: raw.registry_key,
        registry_value_name: raw.registry_value_name,
        startup_path: raw.startup_path.map(PathBuf::from),
        scheduled_task_name: raw.scheduled_task_name,
        scheduled_task_path: raw.scheduled_task_path,
    })
}

#[cfg(any(target_os = "windows", test))]
fn parse_source_type(value: &str) -> Option<StartupSourceType> {
    match value {
        "registryCurrentUserRun" => Some(StartupSourceType::RegistryCurrentUserRun),
        "registryCurrentUserRunOnce" => Some(StartupSourceType::RegistryCurrentUserRunOnce),
        "registryLocalMachineRun" => Some(StartupSourceType::RegistryLocalMachineRun),
        "registryLocalMachineRunOnce" => Some(StartupSourceType::RegistryLocalMachineRunOnce),
        "userStartupFolder" => Some(StartupSourceType::UserStartupFolder),
        "commonStartupFolder" => Some(StartupSourceType::CommonStartupFolder),
        "scheduledTask" => Some(StartupSourceType::ScheduledTask),
        _ => None,
    }
}

#[cfg(target_os = "windows")]
fn source_label(source: StartupSourceType) -> &'static str {
    match source {
        StartupSourceType::RegistryCurrentUserRun => "Current user Run",
        StartupSourceType::RegistryCurrentUserRunOnce => "Current user RunOnce",
        StartupSourceType::RegistryLocalMachineRun => "All users Run",
        StartupSourceType::RegistryLocalMachineRunOnce => "All users RunOnce",
        StartupSourceType::UserStartupFolder => "User Startup folder",
        StartupSourceType::CommonStartupFolder => "All users Startup folder",
        StartupSourceType::ScheduledTask => "Scheduled Task",
    }
}

#[cfg(any(target_os = "windows", test))]
fn startup_entry_id(source: StartupSourceType, identity: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"pc-manager-startup-v1:");
    hasher.update(format!("{source:?}|{}", identity.to_ascii_lowercase()).as_bytes());
    let digest = hasher.finalize();
    format!(
        "startup-{}",
        digest[..12]
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    )
}

fn set_current_user_registry_enabled(
    entry: &NativeStartupEntry,
    enabled: bool,
) -> Result<String, StartupError> {
    #[cfg(target_os = "windows")]
    {
        let key = entry.registry_key.as_deref().ok_or_else(|| {
            StartupError::new(
                "startup_registry_metadata_missing",
                "Registry metadata is missing for this startup entry.",
                true,
            )
        })?;
        let value_name = entry.registry_value_name.as_deref().ok_or_else(|| {
            StartupError::new(
                "startup_registry_metadata_missing",
                "Registry value metadata is missing for this startup entry.",
                true,
            )
        })?;

        if !is_supported_hkcu_key(key) {
            return Err(StartupError::new(
                "startup_registry_source_rejected",
                "The registry source is outside the Startup Manager allow-list.",
                false,
            ));
        }

        let backup_key = format!(
            "HKCU:\\Software\\PCManager\\StartupBackup\\{}",
            entry.public.id
        );

        let script = if enabled {
            r#"
$ErrorActionPreference='Stop'
$key=$env:PCM_REG_KEY
$name=$env:PCM_REG_NAME
$backup=$env:PCM_BACKUP_KEY
if (-not (Test-Path -LiteralPath $backup)) { throw 'Backup entry is missing.' }
$meta=Get-ItemProperty -LiteralPath $backup
if ([string]$meta.OriginalKey -ne $key -or [string]$meta.Name -ne $name) { throw 'Backup metadata does not match the discovered startup entry.' }
New-Item -Path $key -Force | Out-Null
Set-ItemProperty -LiteralPath $key -Name $name -Value ([string]$meta.Command) -Type String
Remove-Item -LiteralPath $backup -Recurse -Force
"#
        } else {
            r#"
$ErrorActionPreference='Stop'
$key=$env:PCM_REG_KEY
$name=$env:PCM_REG_NAME
$backup=$env:PCM_BACKUP_KEY
$current=(Get-ItemProperty -LiteralPath $key -Name $name -ErrorAction Stop).$name
New-Item -Path $backup -Force | Out-Null
Set-ItemProperty -LiteralPath $backup -Name 'SourceType' -Value $env:PCM_SOURCE_TYPE -Type String
Set-ItemProperty -LiteralPath $backup -Name 'OriginalKey' -Value $key -Type String
Set-ItemProperty -LiteralPath $backup -Name 'Name' -Value $name -Type String
Set-ItemProperty -LiteralPath $backup -Name 'Command' -Value ([string]$current) -Type String
Remove-ItemProperty -LiteralPath $key -Name $name -ErrorAction Stop
"#
        };

        let output = Command::new("powershell.exe")
            .args([
                "-NoLogo",
                "-NoProfile",
                "-NonInteractive",
                "-ExecutionPolicy",
                "Bypass",
                "-Command",
                script,
            ])
            .env("PCM_REG_KEY", key)
            .env("PCM_REG_NAME", value_name)
            .env("PCM_BACKUP_KEY", &backup_key)
            .env(
                "PCM_SOURCE_TYPE",
                source_type_wire(entry.public.source_type),
            )
            .output()
            .map_err(|error| {
                StartupError::new(
                    "startup_registry_provider_unavailable",
                    format!("Unable to start the registry startup provider: {error}"),
                    true,
                )
            })?;

        ensure_provider_success(output, "registry startup change")?;
        Ok(if enabled {
            "Startup registry entry restored from PC Manager backup.".to_string()
        } else {
            "Startup registry entry disabled and preserved in PC Manager backup.".to_string()
        })
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = (entry, enabled);
        Err(StartupError::new(
            "unsupported_platform",
            "Registry startup changes are Windows-only.",
            false,
        ))
    }
}

fn set_user_startup_file_enabled(
    entry: &NativeStartupEntry,
    enabled: bool,
) -> Result<String, StartupError> {
    let root = user_startup_root()?;
    let disabled_root = disabled_startup_root()?;
    let entry_dir = disabled_root.join(&entry.public.id);

    if enabled {
        let meta_path = entry_dir.join("meta.json");
        let meta_bytes = fs::read(&meta_path).map_err(|error| {
            StartupError::new(
                "startup_backup_missing",
                format!("Disabled Startup metadata is unavailable: {error}"),
                true,
            )
        })?;
        let meta: DisabledFolderMeta = serde_json::from_slice(&meta_bytes).map_err(|error| {
            StartupError::new(
                "startup_backup_invalid",
                format!("Disabled Startup metadata is invalid: {error}"),
                false,
            )
        })?;

        if meta.entry_id != entry.public.id {
            return Err(StartupError::new(
                "startup_backup_mismatch",
                "Disabled Startup metadata does not match the selected entry.",
                false,
            ));
        }

        let original = PathBuf::from(&meta.original_path);
        let disabled = PathBuf::from(&meta.disabled_path);
        verify_direct_child(&root, &original)?;
        verify_direct_child(&entry_dir, &disabled)?;

        if original.exists() {
            return Err(StartupError::new(
                "startup_destination_exists",
                "The original Startup location already contains an item with this name.",
                true,
            ));
        }

        fs::rename(&disabled, &original).map_err(|error| {
            StartupError::new(
                "startup_restore_failed",
                format!("Unable to restore the Startup item: {error}"),
                true,
            )
        })?;
        let _ = fs::remove_file(&meta_path);
        let _ = fs::remove_dir(&entry_dir);

        Ok("Startup folder item restored to its original user Startup folder.".to_string())
    } else {
        let original = entry.startup_path.as_ref().ok_or_else(|| {
            StartupError::new(
                "startup_path_missing",
                "Startup folder metadata is missing.",
                true,
            )
        })?;
        verify_direct_child(&root, original)?;
        reject_reparse_or_symlink(original)?;

        fs::create_dir_all(&entry_dir).map_err(|error| {
            StartupError::new(
                "startup_backup_create_failed",
                format!("Unable to create the disabled Startup store: {error}"),
                true,
            )
        })?;

        let file_name = original.file_name().ok_or_else(|| {
            StartupError::new(
                "startup_filename_missing",
                "The Startup item has no filename.",
                false,
            )
        })?;
        let disabled = entry_dir.join(file_name);
        if disabled.exists() {
            return Err(StartupError::new(
                "startup_backup_exists",
                "A disabled copy already exists. Refresh before trying again.",
                true,
            ));
        }

        fs::rename(original, &disabled).map_err(|error| {
            StartupError::new(
                "startup_disable_failed",
                format!("Unable to disable the Startup folder item: {error}"),
                true,
            )
        })?;

        let meta = DisabledFolderMeta {
            entry_id: entry.public.id.clone(),
            name: entry.public.name.clone(),
            original_path: original.to_string_lossy().to_string(),
            disabled_path: disabled.to_string_lossy().to_string(),
            command: entry.public.command.clone(),
        };
        let bytes = serde_json::to_vec_pretty(&meta).map_err(|error| {
            StartupError::new(
                "startup_backup_encode_failed",
                format!("Unable to encode disabled Startup metadata: {error}"),
                true,
            )
        })?;
        if let Err(error) = fs::write(entry_dir.join("meta.json"), bytes) {
            let _ = fs::rename(&disabled, original);
            return Err(StartupError::new(
                "startup_backup_write_failed",
                format!("Unable to save disabled Startup metadata: {error}"),
                true,
            ));
        }

        Ok("Startup folder item moved into the PC Manager disabled-startup store.".to_string())
    }
}

fn set_scheduled_task_enabled(
    entry: &NativeStartupEntry,
    enabled: bool,
) -> Result<String, StartupError> {
    #[cfg(target_os = "windows")]
    {
        let task_name = entry.scheduled_task_name.as_deref().ok_or_else(|| {
            StartupError::new(
                "startup_task_metadata_missing",
                "Scheduled Task name is missing.",
                true,
            )
        })?;
        let task_path = entry.scheduled_task_path.as_deref().ok_or_else(|| {
            StartupError::new(
                "startup_task_metadata_missing",
                "Scheduled Task path is missing.",
                true,
            )
        })?;

        if task_path.starts_with("\\Microsoft\\Windows\\") {
            return Err(StartupError::new(
                "startup_task_system_rejected",
                "Windows system Scheduled Tasks are outside the Startup Manager allow-list.",
                false,
            ));
        }

        let script = if enabled {
            "Enable-ScheduledTask -TaskName $env:PCM_TASK_NAME -TaskPath $env:PCM_TASK_PATH -ErrorAction Stop | Out-Null"
        } else {
            "Disable-ScheduledTask -TaskName $env:PCM_TASK_NAME -TaskPath $env:PCM_TASK_PATH -ErrorAction Stop | Out-Null"
        };

        let output = Command::new("powershell.exe")
            .args([
                "-NoLogo",
                "-NoProfile",
                "-NonInteractive",
                "-ExecutionPolicy",
                "Bypass",
                "-Command",
                script,
            ])
            .env("PCM_TASK_NAME", task_name)
            .env("PCM_TASK_PATH", task_path)
            .output()
            .map_err(|error| {
                StartupError::new(
                    "startup_task_provider_unavailable",
                    format!("Unable to start the Scheduled Task provider: {error}"),
                    true,
                )
            })?;

        ensure_provider_success(output, "Scheduled Task change")?;
        Ok(if enabled {
            "Scheduled Task startup entry enabled.".to_string()
        } else {
            "Scheduled Task startup entry disabled.".to_string()
        })
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = (entry, enabled);
        Err(StartupError::new(
            "unsupported_platform",
            "Scheduled Task changes are Windows-only.",
            false,
        ))
    }
}

#[cfg(target_os = "windows")]
fn discover_disabled_startup_files() -> Result<Vec<NativeStartupEntry>, StartupError> {
    let root = disabled_startup_root()?;
    if !root.exists() {
        return Ok(Vec::new());
    }

    let mut entries = Vec::new();
    let dirs = fs::read_dir(&root).map_err(|error| {
        StartupError::new(
            "startup_disabled_store_read_failed",
            format!("Unable to read the disabled Startup store: {error}"),
            true,
        )
    })?;

    for dir in dirs.flatten() {
        let meta_path = dir.path().join("meta.json");
        let Ok(bytes) = fs::read(&meta_path) else {
            continue;
        };
        let Ok(meta) = serde_json::from_slice::<DisabledFolderMeta>(&bytes) else {
            continue;
        };

        let original = PathBuf::from(&meta.original_path);
        let expected_root = user_startup_root()?;
        if verify_direct_child(&expected_root, &original).is_err() {
            continue;
        }

        let identity = meta.original_path.clone();
        let id = startup_entry_id(StartupSourceType::UserStartupFolder, &identity);
        if id != meta.entry_id {
            continue;
        }

        entries.push(NativeStartupEntry {
            public: StartupEntry {
                id,
                name: meta.name,
                source_type: StartupSourceType::UserStartupFolder,
                source_label: source_label(StartupSourceType::UserStartupFolder).to_string(),
                command: meta.command,
                publisher: None,
                enabled: false,
                can_change: true,
                requires_elevation: false,
                impact: StartupImpact::Unknown,
                impact_evidence: None,
                detail: "Disabled by PC Manager; original Startup folder location preserved."
                    .to_string(),
            },
            #[cfg(target_os = "windows")]
            registry_key: None,
            #[cfg(target_os = "windows")]
            registry_value_name: None,
            startup_path: Some(original),
            #[cfg(target_os = "windows")]
            scheduled_task_name: None,
            #[cfg(target_os = "windows")]
            scheduled_task_path: None,
        });
    }

    Ok(entries)
}

fn user_startup_root() -> Result<PathBuf, StartupError> {
    let roaming = std::env::var_os("APPDATA").ok_or_else(|| {
        StartupError::new(
            "startup_appdata_missing",
            "APPDATA is unavailable for the current Windows user.",
            false,
        )
    })?;
    Ok(PathBuf::from(roaming)
        .join("Microsoft")
        .join("Windows")
        .join("Start Menu")
        .join("Programs")
        .join("Startup"))
}

fn disabled_startup_root() -> Result<PathBuf, StartupError> {
    let local = std::env::var_os("LOCALAPPDATA").ok_or_else(|| {
        StartupError::new(
            "startup_localappdata_missing",
            "LOCALAPPDATA is unavailable for the current Windows user.",
            false,
        )
    })?;
    Ok(PathBuf::from(local)
        .join("PCManager")
        .join("DisabledStartup"))
}

fn verify_direct_child(root: &Path, candidate: &Path) -> Result<(), StartupError> {
    let candidate_parent = candidate.parent().ok_or_else(|| {
        StartupError::new(
            "startup_path_invalid",
            "The Startup item has no parent directory.",
            false,
        )
    })?;

    let normalized_root = normalize_path(root);
    let normalized_parent = normalize_path(candidate_parent);
    if normalized_root != normalized_parent {
        return Err(StartupError::new(
            "startup_path_outside_root",
            "The Startup item is outside the approved Startup folder.",
            false,
        ));
    }
    Ok(())
}

fn normalize_path(path: &Path) -> String {
    path.to_string_lossy()
        .replace('/', "\\")
        .trim_end_matches('\\')
        .to_ascii_lowercase()
}

fn reject_reparse_or_symlink(path: &Path) -> Result<(), StartupError> {
    let metadata = fs::symlink_metadata(path).map_err(|error| {
        StartupError::new(
            "startup_path_inspect_failed",
            format!("Unable to inspect the Startup item: {error}"),
            true,
        )
    })?;

    if metadata.file_type().is_symlink() {
        return Err(StartupError::new(
            "startup_reparse_rejected",
            "Symbolic links are not changed by Startup Manager.",
            false,
        ));
    }

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0400;
        if (metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT) != 0 {
            return Err(StartupError::new(
                "startup_reparse_rejected",
                "Windows reparse points are not changed by Startup Manager.",
                false,
            ));
        }
    }

    Ok(())
}

#[cfg(any(target_os = "windows", test))]
fn is_supported_hkcu_key(key: &str) -> bool {
    matches!(
        key.to_ascii_lowercase().as_str(),
        "hkcu:\\software\\microsoft\\windows\\currentversion\\run"
            | "hkcu:\\software\\microsoft\\windows\\currentversion\\runonce"
    )
}

#[cfg(target_os = "windows")]
fn source_type_wire(source: StartupSourceType) -> &'static str {
    match source {
        StartupSourceType::RegistryCurrentUserRun => "registryCurrentUserRun",
        StartupSourceType::RegistryCurrentUserRunOnce => "registryCurrentUserRunOnce",
        StartupSourceType::RegistryLocalMachineRun => "registryLocalMachineRun",
        StartupSourceType::RegistryLocalMachineRunOnce => "registryLocalMachineRunOnce",
        StartupSourceType::UserStartupFolder => "userStartupFolder",
        StartupSourceType::CommonStartupFolder => "commonStartupFolder",
        StartupSourceType::ScheduledTask => "scheduledTask",
    }
}

#[cfg(target_os = "windows")]
fn ensure_provider_success(output: std::process::Output, action: &str) -> Result<(), StartupError> {
    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    Err(StartupError::new(
        "startup_change_failed",
        if stderr.trim().is_empty() {
            format!("The {action} failed.")
        } else {
            format!("The {action} failed: {}", stderr.trim())
        },
        true,
    ))
}

fn operation_record(
    entry: &NativeStartupEntry,
    previous_enabled: bool,
    new_enabled: bool,
    success: bool,
    message: String,
) -> StartupOperationRecord {
    let completed_at_epoch_ms = now_epoch_ms();
    StartupOperationRecord {
        operation_id: format!(
            "startup-op-{}-{}",
            completed_at_epoch_ms,
            &entry.public.id["startup-".len()..]
        ),
        entry_id: entry.public.id.clone(),
        display_name: entry.public.name.clone(),
        source_type: entry.public.source_type,
        previous_enabled,
        new_enabled,
        success,
        rollback_available: matches!(
            entry.public.source_type,
            StartupSourceType::RegistryCurrentUserRun
                | StartupSourceType::RegistryCurrentUserRunOnce
                | StartupSourceType::UserStartupFolder
                | StartupSourceType::ScheduledTask
        ),
        completed_at_epoch_ms,
        message,
    }
}

fn startup_audit_path() -> Result<PathBuf, StartupError> {
    let local = std::env::var_os("LOCALAPPDATA").ok_or_else(|| {
        StartupError::new(
            "startup_localappdata_missing",
            "LOCALAPPDATA is unavailable for Startup Manager audit storage.",
            false,
        )
    })?;
    Ok(PathBuf::from(local)
        .join("PCManager")
        .join("operations")
        .join(STARTUP_AUDIT_FILE))
}

fn append_operation(record: &StartupOperationRecord) -> Result<(), StartupError> {
    let path = startup_audit_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            StartupError::new(
                "startup_audit_directory_failed",
                format!("Unable to create Startup Manager audit storage: {error}"),
                true,
            )
        })?;
    }

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|error| {
            StartupError::new(
                "startup_audit_open_failed",
                format!("Unable to open Startup Manager audit storage: {error}"),
                true,
            )
        })?;

    let line = serde_json::to_string(record).map_err(|error| {
        StartupError::new(
            "startup_audit_encode_failed",
            format!("Unable to encode Startup Manager audit record: {error}"),
            true,
        )
    })?;
    writeln!(file, "{line}").map_err(|error| {
        StartupError::new(
            "startup_audit_write_failed",
            format!("Unable to write Startup Manager audit record: {error}"),
            true,
        )
    })
}

fn now_epoch_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::{is_supported_hkcu_key, parse_source_type, startup_entry_id, verify_direct_child};
    use pc_core::StartupSourceType;
    use std::path::Path;

    #[test]
    fn startup_entry_ids_are_stable_and_opaque() {
        let first = startup_entry_id(
            StartupSourceType::RegistryCurrentUserRun,
            r"HKCU:\Software\Microsoft\Windows\CurrentVersion\Run|Example",
        );
        let second = startup_entry_id(
            StartupSourceType::RegistryCurrentUserRun,
            r"hkcu:\software\microsoft\windows\currentversion\run|example",
        );

        assert_eq!(first, second);
        assert!(first.starts_with("startup-"));
        assert!(!first.to_ascii_lowercase().contains("example"));
    }

    #[test]
    fn only_supported_current_user_run_keys_are_mutable() {
        assert!(is_supported_hkcu_key(
            r"HKCU:\Software\Microsoft\Windows\CurrentVersion\Run"
        ));
        assert!(is_supported_hkcu_key(
            r"HKCU:\Software\Microsoft\Windows\CurrentVersion\RunOnce"
        ));
        assert!(!is_supported_hkcu_key(
            r"HKCU:\Software\Microsoft\Windows\CurrentVersion\Policies\Explorer\Run"
        ));
    }

    #[test]
    fn source_mapping_is_explicit() {
        assert_eq!(
            parse_source_type("scheduledTask"),
            Some(StartupSourceType::ScheduledTask)
        );
        assert_eq!(parse_source_type("anythingElse"), None);
    }

    #[test]
    fn startup_folder_candidate_must_be_direct_child() {
        assert!(verify_direct_child(
            Path::new("/tmp/startup"),
            Path::new("/tmp/startup/example.lnk")
        )
        .is_ok());
        assert!(verify_direct_child(
            Path::new("/tmp/startup"),
            Path::new("/tmp/documents/example.lnk")
        )
        .is_err());
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn windows_startup_provider_returns_without_persistent_changes() {
        let entries = super::list_startup_entries().expect("startup inventory should run");
        for entry in entries {
            assert!(!entry.id.trim().is_empty());
            assert!(!entry.name.trim().is_empty());
        }
    }
}

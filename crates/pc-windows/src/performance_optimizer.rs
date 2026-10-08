use pc_core::{PerformanceOptimizerError, PerformanceServiceEvidence};
#[cfg(target_os = "windows")]
use serde::Deserialize;
#[cfg(target_os = "windows")]
use std::process::Command;

#[cfg(any(target_os = "windows", test))]
const PERFORMANCE_SERVICE_SCRIPT: &str = r#"
$ErrorActionPreference = 'Stop'
$rows = [System.Collections.Generic.List[object]]::new()

$services = @(Get-CimInstance Win32_Service -ErrorAction Stop |
  Where-Object { $_.StartMode -eq 'Auto' -and $_.State -eq 'Running' } |
  Select-Object -First 250 Name, DisplayName, State, StartMode, ProcessId, PathName)

foreach ($service in $services) {
  $processName = $null
  if ([uint32]$service.ProcessId -gt 0) {
    try {
      $processName = [string](Get-Process -Id ([uint32]$service.ProcessId) -ErrorAction Stop |
        Select-Object -First 1 -ExpandProperty ProcessName)
    } catch {}
  }

  $pathName = if ($null -eq $service.PathName) { '' } else { [string]$service.PathName }
  $expandedPath = [Environment]::ExpandEnvironmentVariables($pathName).Trim().TrimStart('"')
  $windowsSystemPath = $false
  if (-not [string]::IsNullOrWhiteSpace($expandedPath) -and -not [string]::IsNullOrWhiteSpace($env:windir)) {
    $windowsSystemPath = $expandedPath.StartsWith(
      $env:windir,
      [System.StringComparison]::OrdinalIgnoreCase
    )
  }

  $rows.Add([pscustomobject]@{
    name = [string]$service.Name
    displayName = [string]$service.DisplayName
    state = [string]$service.State
    startMode = [string]$service.StartMode
    processName = $processName
    windowsSystemPath = [bool]$windowsSystemPath
  })
}

ConvertTo-Json -InputObject $rows.ToArray() -Depth 4 -Compress
"#;

#[cfg(target_os = "windows")]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawPerformanceService {
    name: String,
    display_name: String,
    state: String,
    start_mode: String,
    process_name: Option<String>,
    windows_system_path: bool,
}

pub fn list_performance_services(
) -> Result<Vec<PerformanceServiceEvidence>, PerformanceOptimizerError> {
    #[cfg(target_os = "windows")]
    {
        let output = Command::new("powershell.exe")
            .args([
                "-NoLogo",
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                PERFORMANCE_SERVICE_SCRIPT,
            ])
            .output()
            .map_err(|error| {
                PerformanceOptimizerError::new(
                    "performance_service_provider_unavailable",
                    format!("Unable to start the Windows service evidence provider: {error}"),
                    true,
                )
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(PerformanceOptimizerError::new(
                "performance_service_provider_failed",
                if stderr.trim().is_empty() {
                    "The Windows service evidence provider exited with an error.".to_string()
                } else {
                    format!(
                        "The Windows service evidence provider failed: {}",
                        stderr.trim()
                    )
                },
                true,
            ));
        }

        let stdout = String::from_utf8(output.stdout).map_err(|error| {
            PerformanceOptimizerError::new(
                "performance_service_provider_encoding",
                format!("Service evidence output was not valid UTF-8: {error}"),
                true,
            )
        })?;
        if stdout.trim().is_empty() {
            return Ok(Vec::new());
        }

        let raw: Vec<RawPerformanceService> =
            serde_json::from_str(stdout.trim()).map_err(|error| {
                PerformanceOptimizerError::new(
                    "performance_service_provider_payload",
                    format!("Unable to parse Windows service evidence: {error}"),
                    true,
                )
            })?;

        Ok(raw
            .into_iter()
            .filter(|service| !service.name.trim().is_empty())
            .map(|service| PerformanceServiceEvidence {
                name: service.name,
                display_name: service.display_name,
                state: service.state,
                start_mode: service.start_mode,
                process_name: service.process_name.and_then(|name| {
                    let trimmed = name.trim();
                    (!trimmed.is_empty()).then(|| trimmed.to_string())
                }),
                windows_system_path: service.windows_system_path,
            })
            .collect())
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err(PerformanceOptimizerError::new(
            "unsupported_platform",
            "Performance service evidence is currently implemented for Windows only.",
            false,
        ))
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn provider_is_read_only_by_contract() {
        let script = super::PERFORMANCE_SERVICE_SCRIPT;
        assert!(script.contains("Get-CimInstance Win32_Service"));
        assert!(!script.contains("Stop-Service"));
        assert!(!script.contains("Set-Service"));
        assert!(!script.contains("sc.exe"));
        assert!(script.contains("ConvertTo-Json -InputObject $rows.ToArray()"));
    }
}

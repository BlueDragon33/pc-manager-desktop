//! Low-overhead Windows resource monitoring for PC Manager Desktop.
//!
//! P7 samples only on explicit UI requests. There is no permanent background
//! polling loop in this crate.

use serde::{Deserialize, Serialize};

#[cfg(target_os = "windows")]
use std::process::Command;
#[cfg(target_os = "windows")]
use std::time::Instant;
#[cfg(any(target_os = "windows", test))]
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PercentMetric {
    pub available: bool,
    pub value: Option<f64>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryMetric {
    pub available: bool,
    pub total_bytes: Option<u64>,
    pub used_bytes: Option<u64>,
    pub available_bytes: Option<u64>,
    pub used_percent: Option<f64>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IoMetric {
    pub available: bool,
    pub read_bytes_per_sec: Option<u64>,
    pub write_bytes_per_sec: Option<u64>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkMetric {
    pub available: bool,
    pub receive_bytes_per_sec: Option<u64>,
    pub send_bytes_per_sec: Option<u64>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GpuMetric {
    pub available: bool,
    pub utilization_percent: Option<f64>,
    pub metric_label: String,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SensorMetric {
    pub name: String,
    pub available: bool,
    pub value_celsius: Option<f64>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessUsage {
    pub pid: u32,
    pub name: String,
    pub cpu_percent: Option<f64>,
    pub memory_bytes: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MonitorSnapshot {
    pub collected_at_epoch_ms: u64,
    pub sample_duration_ms: u64,
    pub cpu: PercentMetric,
    pub memory: MemoryMetric,
    pub disk: IoMetric,
    pub network: NetworkMetric,
    pub gpu: GpuMetric,
    pub sensors: Vec<SensorMetric>,
    pub top_processes: Vec<ProcessUsage>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MonitorError {
    pub code: String,
    pub message: String,
    pub recoverable: bool,
}

impl MonitorError {
    #[must_use]
    pub fn new(code: impl Into<String>, message: impl Into<String>, recoverable: bool) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            recoverable,
        }
    }
}

#[cfg(target_os = "windows")]
const MONITOR_SCRIPT: &str = r#"
$ErrorActionPreference = 'Stop'

function Try-Read([scriptblock]$Action, $Fallback) {
  try { & $Action } catch { $Fallback }
}

$cpu = Try-Read {
  $value = Get-CimInstance Win32_PerfFormattedData_PerfOS_Processor -Filter "Name='_Total'" |
    Select-Object -First 1 -ExpandProperty PercentProcessorTime
  if ($null -eq $value) { $null } else { [double]$value }
} $null

$os = Try-Read {
  Get-CimInstance Win32_OperatingSystem |
    Select-Object -First 1 TotalVisibleMemorySize, FreePhysicalMemory
} $null

$disk = Try-Read {
  Get-CimInstance Win32_PerfFormattedData_PerfDisk_PhysicalDisk -Filter "Name='_Total'" |
    Select-Object -First 1 DiskReadBytesPersec, DiskWriteBytesPersec
} $null

$networkRows = @(Try-Read {
  @(Get-CimInstance Win32_PerfFormattedData_Tcpip_NetworkInterface |
    Select-Object BytesReceivedPersec, BytesSentPersec)
} @())
$networkReceive = $null
$networkSend = $null
if ($networkRows.Count -gt 0) {
  [uint64]$networkReceive = ($networkRows | Measure-Object -Property BytesReceivedPersec -Sum).Sum
  [uint64]$networkSend = ($networkRows | Measure-Object -Property BytesSentPersec -Sum).Sum
}

$processes = @(Try-Read {
  @(Get-CimInstance Win32_PerfFormattedData_PerfProc_Process |
    Where-Object { $_.IDProcess -gt 0 -and $_.Name -ne '_Total' -and $_.Name -ne 'Idle' } |
    Sort-Object PercentProcessorTime -Descending |
    Select-Object -First 10 Name, IDProcess, PercentProcessorTime, WorkingSetPrivate)
} @())

$gpuRows = @(Try-Read {
  @(Get-CimInstance Win32_PerfFormattedData_GPUPerformanceCounters_GPUEngine |
    Where-Object { $_.Name -match 'engtype_3D' } |
    Select-Object UtilizationPercentage)
} @())
$gpuValue = $null
if ($gpuRows.Count -gt 0) {
  $gpuValue = [double](($gpuRows | Measure-Object -Property UtilizationPercentage -Maximum).Maximum)
}

[pscustomobject]@{
  cpuPercent = $cpu
  totalMemoryKb = if ($null -eq $os) { $null } else { [uint64]$os.TotalVisibleMemorySize }
  freeMemoryKb = if ($null -eq $os) { $null } else { [uint64]$os.FreePhysicalMemory }
  diskReadBytesPersec = if ($null -eq $disk) { $null } else { [uint64]$disk.DiskReadBytesPersec }
  diskWriteBytesPersec = if ($null -eq $disk) { $null } else { [uint64]$disk.DiskWriteBytesPersec }
  networkReceiveBytesPersec = $networkReceive
  networkSendBytesPersec = $networkSend
  gpu3dPeakPercent = $gpuValue
  processes = @($processes | ForEach-Object {
    [pscustomobject]@{
      name = [string]$_.Name
      pid = [uint32]$_.IDProcess
      cpuPercent = if ($null -eq $_.PercentProcessorTime) { $null } else { [double]$_.PercentProcessorTime }
      memoryBytes = if ($null -eq $_.WorkingSetPrivate) { $null } else { [uint64]$_.WorkingSetPrivate }
    }
  })
} | ConvertTo-Json -Depth 5 -Compress
"#;

#[cfg(any(target_os = "windows", test))]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawMonitorSnapshot {
    cpu_percent: Option<f64>,
    total_memory_kb: Option<u64>,
    free_memory_kb: Option<u64>,
    disk_read_bytes_persec: Option<u64>,
    disk_write_bytes_persec: Option<u64>,
    network_receive_bytes_persec: Option<u64>,
    network_send_bytes_persec: Option<u64>,
    gpu3d_peak_percent: Option<f64>,
    #[serde(default)]
    processes: Vec<RawProcessUsage>,
}

#[cfg(any(target_os = "windows", test))]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawProcessUsage {
    name: String,
    pid: u32,
    cpu_percent: Option<f64>,
    memory_bytes: Option<u64>,
}

#[must_use]
pub fn normalize_percent(value: f64) -> f64 {
    if value.is_finite() {
        value.clamp(0.0, 100.0)
    } else {
        0.0
    }
}

pub fn sample_monitor() -> Result<MonitorSnapshot, MonitorError> {
    #[cfg(target_os = "windows")]
    {
        let started = Instant::now();
        let output = Command::new("powershell.exe")
            .args([
                "-NoLogo",
                "-NoProfile",
                "-NonInteractive",
                "-ExecutionPolicy",
                "Bypass",
                "-Command",
                MONITOR_SCRIPT,
            ])
            .output()
            .map_err(|error| {
                MonitorError::new(
                    "monitor_provider_unavailable",
                    format!("Unable to start the Windows monitor provider: {error}"),
                    true,
                )
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(MonitorError::new(
                "monitor_provider_failed",
                if stderr.trim().is_empty() {
                    "The Windows monitor provider exited with an error.".to_string()
                } else {
                    format!("The Windows monitor provider failed: {}", stderr.trim())
                },
                true,
            ));
        }

        let stdout = String::from_utf8(output.stdout).map_err(|error| {
            MonitorError::new(
                "monitor_provider_encoding",
                format!("Monitor output was not valid UTF-8: {error}"),
                true,
            )
        })?;
        let raw: RawMonitorSnapshot = serde_json::from_str(stdout.trim()).map_err(|error| {
            MonitorError::new(
                "monitor_provider_payload",
                format!("Unable to parse the Windows monitor payload: {error}"),
                true,
            )
        })?;

        Ok(map_raw_snapshot(raw, started.elapsed().as_millis() as u64))
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err(MonitorError::new(
            "unsupported_platform",
            "System Monitor sampling is currently implemented for Windows only.",
            false,
        ))
    }
}

#[cfg(any(target_os = "windows", test))]
fn map_raw_snapshot(raw: RawMonitorSnapshot, sample_duration_ms: u64) -> MonitorSnapshot {
    let cpu = match raw.cpu_percent {
        Some(value) => PercentMetric {
            available: true,
            value: Some(normalize_percent(value)),
            reason: None,
        },
        None => PercentMetric {
            available: false,
            value: None,
            reason: Some("Windows CPU performance data is unavailable.".to_string()),
        },
    };

    let memory = match (raw.total_memory_kb, raw.free_memory_kb) {
        (Some(total_kb), Some(free_kb)) if total_kb > 0 => {
            let total_bytes = total_kb.saturating_mul(1024);
            let available_bytes = free_kb.saturating_mul(1024).min(total_bytes);
            let used_bytes = total_bytes.saturating_sub(available_bytes);
            MemoryMetric {
                available: true,
                total_bytes: Some(total_bytes),
                used_bytes: Some(used_bytes),
                available_bytes: Some(available_bytes),
                used_percent: Some(normalize_percent(
                    used_bytes as f64 * 100.0 / total_bytes as f64,
                )),
                reason: None,
            }
        }
        _ => MemoryMetric {
            available: false,
            total_bytes: None,
            used_bytes: None,
            available_bytes: None,
            used_percent: None,
            reason: Some("Windows memory telemetry is unavailable.".to_string()),
        },
    };

    let disk_available =
        raw.disk_read_bytes_persec.is_some() || raw.disk_write_bytes_persec.is_some();
    let network_available =
        raw.network_receive_bytes_persec.is_some() || raw.network_send_bytes_persec.is_some();

    MonitorSnapshot {
        collected_at_epoch_ms: now_epoch_ms(),
        sample_duration_ms,
        cpu,
        memory,
        disk: IoMetric {
            available: disk_available,
            read_bytes_per_sec: raw.disk_read_bytes_persec,
            write_bytes_per_sec: raw.disk_write_bytes_persec,
            reason: (!disk_available)
                .then(|| "Aggregate Windows physical-disk throughput is unavailable.".to_string()),
        },
        network: NetworkMetric {
            available: network_available,
            receive_bytes_per_sec: raw.network_receive_bytes_persec,
            send_bytes_per_sec: raw.network_send_bytes_persec,
            reason: (!network_available)
                .then(|| "Windows network-interface throughput is unavailable.".to_string()),
        },
        gpu: match raw.gpu3d_peak_percent {
            Some(value) => GpuMetric {
                available: true,
                utilization_percent: Some(normalize_percent(value)),
                metric_label: "Peak 3D engine activity".to_string(),
                reason: None,
            },
            None => GpuMetric {
                available: false,
                utilization_percent: None,
                metric_label: "GPU activity".to_string(),
                reason: Some(
                    "A reliable Windows GPU 3D performance source is unavailable on this system."
                        .to_string(),
                ),
            },
        },
        sensors: vec![SensorMetric {
            name: "Temperature".to_string(),
            available: false,
            value_celsius: None,
            reason: Some(
                "P7 does not guess temperatures when Windows exposes no trustworthy generic sensor source."
                    .to_string(),
            ),
        }],
        top_processes: raw
            .processes
            .into_iter()
            .filter(|process| process.pid > 0 && !process.name.trim().is_empty())
            .take(10)
            .map(|process| ProcessUsage {
                pid: process.pid,
                name: process.name,
                cpu_percent: process.cpu_percent.map(normalize_percent),
                memory_bytes: process.memory_bytes,
            })
            .collect(),
    }
}

#[cfg(any(target_os = "windows", test))]
fn now_epoch_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or_default()
}

/// Returns this crate's stable component name.
#[must_use]
pub const fn component_name() -> &'static str {
    "pc-monitor"
}

#[cfg(test)]
mod tests {
    use super::{map_raw_snapshot, normalize_percent, RawMonitorSnapshot};

    #[test]
    fn percentages_are_bounded() {
        assert_eq!(normalize_percent(-5.0), 0.0);
        assert_eq!(normalize_percent(42.5), 42.5);
        assert_eq!(normalize_percent(150.0), 100.0);
        assert_eq!(normalize_percent(f64::NAN), 0.0);
    }

    #[test]
    fn unavailable_sources_are_not_fabricated_as_zero() {
        let snapshot = map_raw_snapshot(
            RawMonitorSnapshot {
                cpu_percent: None,
                total_memory_kb: None,
                free_memory_kb: None,
                disk_read_bytes_persec: None,
                disk_write_bytes_persec: None,
                network_receive_bytes_persec: None,
                network_send_bytes_persec: None,
                gpu3d_peak_percent: None,
                processes: Vec::new(),
            },
            12,
        );

        assert!(!snapshot.cpu.available);
        assert_eq!(snapshot.cpu.value, None);
        assert!(!snapshot.gpu.available);
        assert!(snapshot.gpu.utilization_percent.is_none());
        assert!(!snapshot.sensors[0].available);
        assert_eq!(snapshot.sample_duration_ms, 12);
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn windows_monitor_provider_returns_real_sample() {
        let snapshot = super::sample_monitor().expect("Windows monitor provider should run");
        println!(
            "P7 monitor provider sample duration: {} ms",
            snapshot.sample_duration_ms
        );
        assert!(snapshot.cpu.available);
        assert!(snapshot.memory.available);
    }
}

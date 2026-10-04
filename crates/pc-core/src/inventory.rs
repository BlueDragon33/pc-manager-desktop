use serde::Serialize;

/// Complete read-only inventory snapshot returned to the desktop UI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemInventory {
    pub collected_at_epoch_ms: u64,
    pub operating_system: OperatingSystemSummary,
    pub device: DeviceIdentity,
    pub cpu: CpuSummary,
    pub memory: MemorySummary,
    pub volumes: Vec<DiskVolume>,
    pub processes: ProcessSummary,
    pub installed_applications: Vec<InstalledApplication>,
    pub startup_items: Vec<StartupItem>,
    pub network_adapters: Vec<NetworkAdapter>,
    pub warnings: Vec<InventoryWarning>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OperatingSystemSummary {
    pub name: String,
    pub version: String,
    pub build_number: Option<String>,
    pub architecture: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceIdentity {
    pub hostname: String,
    /// One-way identifier derived locally from the Windows MachineGuid.
    pub local_device_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CpuSummary {
    pub brand: String,
    pub physical_cores: Option<u32>,
    pub logical_cores: u32,
    pub architecture: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemorySummary {
    pub total_bytes: u64,
    pub available_bytes: u64,
}

impl MemorySummary {
    #[must_use]
    pub fn used_bytes(&self) -> u64 {
        self.total_bytes.saturating_sub(self.available_bytes)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiskVolume {
    pub name: String,
    pub label: Option<String>,
    pub file_system: Option<String>,
    pub total_bytes: u64,
    pub available_bytes: u64,
}

impl DiskVolume {
    #[must_use]
    pub fn used_bytes(&self) -> u64 {
        self.total_bytes.saturating_sub(self.available_bytes)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessSummary {
    pub total_count: usize,
    pub top_memory: Vec<ProcessEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessEntry {
    pub pid: u32,
    pub name: String,
    pub memory_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledApplication {
    pub name: String,
    pub version: Option<String>,
    pub publisher: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StartupItem {
    pub name: String,
    pub source: String,
    pub location: Option<String>,
    pub command: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkAdapter {
    pub name: String,
    pub description: Option<String>,
    pub enabled: Option<bool>,
    pub link_speed_bps: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InventoryWarning {
    pub source: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InventoryError {
    pub code: String,
    pub message: String,
    pub recoverable: bool,
}

impl InventoryError {
    #[must_use]
    pub fn new(code: impl Into<String>, message: impl Into<String>, recoverable: bool) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            recoverable,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{DiskVolume, MemorySummary};

    #[test]
    fn used_memory_saturates_on_inconsistent_source_data() {
        let memory = MemorySummary {
            total_bytes: 8,
            available_bytes: 10,
        };

        assert_eq!(memory.used_bytes(), 0);
    }

    #[test]
    fn used_volume_bytes_are_calculated_safely() {
        let volume = DiskVolume {
            name: "C:".to_string(),
            label: None,
            file_system: Some("NTFS".to_string()),
            total_bytes: 1_000,
            available_bytes: 250,
        };

        assert_eq!(volume.used_bytes(), 750);
    }
}

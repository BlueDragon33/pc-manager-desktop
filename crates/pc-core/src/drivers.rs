use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DriverInventoryEntry {
    pub id: String,
    pub device_name: String,
    pub manufacturer: Option<String>,
    pub provider: Option<String>,
    pub version: Option<String>,
    pub driver_date: Option<String>,
    pub class_name: Option<String>,
    pub signer: Option<String>,
    pub inf_name: Option<String>,
    pub is_signed: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DriverUpdateCandidate {
    pub id: String,
    pub title: String,
    pub manufacturer: Option<String>,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub class_name: Option<String>,
    pub driver_date: Option<String>,
    pub downloaded: bool,
    pub eula_accepted: bool,
    pub reboot_required: bool,
    pub source: String,
    pub recommendation: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DriverCenterWarning {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DriverCenterSnapshot {
    pub provider: String,
    pub provider_available: bool,
    pub source_detail: String,
    pub installed_drivers: Vec<DriverInventoryEntry>,
    pub available_updates: Vec<DriverUpdateCandidate>,
    pub warnings: Vec<DriverCenterWarning>,
    pub generated_epoch_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DriverCenterError {
    pub code: String,
    pub message: String,
    pub recoverable: bool,
}

impl DriverCenterError {
    #[must_use]
    pub fn new(code: impl Into<String>, message: impl Into<String>, recoverable: bool) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            recoverable,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DriverSettingsLaunchResult {
    pub launched: bool,
    pub target: String,
    pub message: String,
}

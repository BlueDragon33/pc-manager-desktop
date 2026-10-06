use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StartupSourceType {
    RegistryCurrentUserRun,
    RegistryCurrentUserRunOnce,
    RegistryLocalMachineRun,
    RegistryLocalMachineRunOnce,
    UserStartupFolder,
    CommonStartupFolder,
    ScheduledTask,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StartupImpact {
    Unknown,
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartupEntry {
    pub id: String,
    pub name: String,
    pub source_type: StartupSourceType,
    pub source_label: String,
    pub command: String,
    pub publisher: Option<String>,
    pub enabled: bool,
    pub can_change: bool,
    pub requires_elevation: bool,
    pub impact: StartupImpact,
    pub impact_evidence: Option<String>,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartupChangeRequest {
    pub entry_id: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartupOperationRecord {
    pub operation_id: String,
    pub entry_id: String,
    pub display_name: String,
    pub source_type: StartupSourceType,
    pub previous_enabled: bool,
    pub new_enabled: bool,
    pub success: bool,
    pub rollback_available: bool,
    pub completed_at_epoch_ms: u64,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StartupError {
    pub code: String,
    pub message: String,
    pub recoverable: bool,
}

impl StartupError {
    #[must_use]
    pub fn new(code: impl Into<String>, message: impl Into<String>, recoverable: bool) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            recoverable,
        }
    }
}

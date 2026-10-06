use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum InstalledAppSource {
    CurrentUser,
    LocalMachine64,
    LocalMachine32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum UninstallKind {
    Msi,
    Executable,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledAppEntry {
    pub id: String,
    pub display_name: String,
    pub publisher: Option<String>,
    pub version: Option<String>,
    pub install_location: Option<String>,
    pub install_date: Option<String>,
    pub estimated_size_bytes: Option<u64>,
    pub source: InstalledAppSource,
    pub can_uninstall: bool,
    pub requires_elevation: bool,
    pub uninstall_kind: UninstallKind,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UninstallRequest {
    pub app_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UninstallLaunchResult {
    pub app_id: String,
    pub display_name: String,
    pub launched: bool,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppsError {
    pub code: String,
    pub message: String,
    pub recoverable: bool,
}

impl AppsError {
    #[must_use]
    pub fn new(code: impl Into<String>, message: impl Into<String>, recoverable: bool) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            recoverable,
        }
    }
}

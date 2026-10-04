use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum CleanupCategory {
    WindowsTemp,
    BrowserCache,
    ApplicationCache,
    RecycleBin,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupScanOptions {
    #[serde(default)]
    pub include_recycle_bin: bool,
}

impl Default for CleanupScanOptions {
    fn default() -> Self {
        Self {
            include_recycle_bin: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupWarning {
    pub provider_id: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupProviderSummary {
    pub provider_id: String,
    pub display_name: String,
    pub category: CleanupCategory,
    pub enabled_by_default: bool,
    pub available: bool,
    pub file_count: u64,
    pub bytes: u64,
    pub reversible: bool,
    pub description: String,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupScanSummary {
    pub plan_id: String,
    pub collected_at_epoch_ms: u64,
    pub total_bytes: u64,
    pub total_files: u64,
    pub providers: Vec<CleanupProviderSummary>,
    pub warnings: Vec<CleanupWarning>,
    pub execution_available: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CleanupPlanItem {
    pub provider_id: String,
    pub provider_root: String,
    pub path: String,
    pub bytes: u64,
    pub modified_at_epoch_ms: Option<u64>,
    pub reversible: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CleanupPlan {
    pub plan_id: String,
    pub collected_at_epoch_ms: u64,
    pub items: Vec<CleanupPlanItem>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupError {
    pub code: String,
    pub message: String,
    pub recoverable: bool,
}

impl CleanupError {
    #[must_use]
    pub fn new(code: impl Into<String>, message: impl Into<String>, recoverable: bool) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            recoverable,
        }
    }
}

#[must_use]
pub fn summarize_cleanup_plan(
    plan_id: String,
    collected_at_epoch_ms: u64,
    providers: Vec<CleanupProviderSummary>,
    warnings: Vec<CleanupWarning>,
) -> CleanupScanSummary {
    let total_bytes = providers.iter().map(|provider| provider.bytes).sum();
    let total_files = providers.iter().map(|provider| provider.file_count).sum();

    CleanupScanSummary {
        plan_id,
        collected_at_epoch_ms,
        total_bytes,
        total_files,
        providers,
        warnings,
        execution_available: false,
    }
}

#[cfg(test)]
mod tests {
    use super::{summarize_cleanup_plan, CleanupCategory, CleanupProviderSummary, CleanupWarning};

    #[test]
    fn scan_summary_aggregates_exact_provider_totals() {
        let summary = summarize_cleanup_plan(
            "cleanup-1".to_string(),
            100,
            vec![
                CleanupProviderSummary {
                    provider_id: "windows-temp".to_string(),
                    display_name: "Windows user temp".to_string(),
                    category: CleanupCategory::WindowsTemp,
                    enabled_by_default: true,
                    available: true,
                    file_count: 3,
                    bytes: 120,
                    reversible: false,
                    description: "test".to_string(),
                    warnings: vec![],
                },
                CleanupProviderSummary {
                    provider_id: "browser-edge".to_string(),
                    display_name: "Microsoft Edge cache".to_string(),
                    category: CleanupCategory::BrowserCache,
                    enabled_by_default: true,
                    available: true,
                    file_count: 2,
                    bytes: 80,
                    reversible: false,
                    description: "test".to_string(),
                    warnings: vec![],
                },
            ],
            vec![CleanupWarning {
                provider_id: "browser-edge".to_string(),
                message: "one item could not be read".to_string(),
            }],
        );

        assert_eq!(summary.total_files, 5);
        assert_eq!(summary.total_bytes, 200);
        assert!(!summary.execution_available);
        assert_eq!(summary.warnings.len(), 1);
    }
}

//! Platform-neutral domain models for PC Manager Desktop.

mod apps;
mod cleanup;
mod filesystem;
mod health;
mod inventory;
mod operation;
mod startup;

pub use apps::{
    AppsError, InstalledAppEntry, InstalledAppSource, UninstallKind, UninstallLaunchResult,
    UninstallRequest,
};
pub use cleanup::{
    summarize_cleanup_plan, CleanupCategory, CleanupError, CleanupPlan, CleanupPlanItem,
    CleanupPlanStore, CleanupProviderSummary, CleanupScanOptions, CleanupScanSummary,
    CleanupWarning,
};
pub use filesystem::{
    DuplicateDeleteError, DuplicateDeleteRequest, DuplicateDeleteResult, DuplicateFile,
    DuplicateGroup, DuplicatePlanItem, DuplicateScanOptions, DuplicateScanPlan,
    DuplicateScanPlanStore, DuplicateScanSummary, FilesystemError, FilesystemWarning,
    ScanRootSelection, StorageFileEntry, StorageFolderAggregate, StorageScanOptions,
    StorageScanSummary, StorageTypeAggregate,
};
pub use health::{
    evaluate_health, HealthCategory, HealthCategoryResult, HealthFinding, HealthReport,
    HealthSeverity, HealthStatus, RiskLevel,
};
pub use inventory::{
    CpuSummary, DeviceIdentity, DiskVolume, InstalledApplication, InventoryError, InventoryWarning,
    MemorySummary, NetworkAdapter, OperatingSystemSummary, ProcessEntry, ProcessSummary,
    StartupItem, SystemInventory,
};
pub use operation::{
    CleanupExecutionError, CleanupOperationRecord, CleanupProviderExecutionResult,
};
pub use startup::{
    StartupChangeRequest, StartupEntry, StartupError, StartupImpact, StartupOperationRecord,
    StartupSourceType,
};

use serde::Serialize;

/// Stable application identity exposed through the native bridge.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub app_id: &'static str,
    pub platform: &'static str,
    pub device_type: &'static str,
    pub phase: &'static str,
}

impl AppInfo {
    /// Returns the immutable identity for the current implementation phase.
    #[must_use]
    pub const fn current() -> Self {
        Self {
            app_id: "pc-manager",
            platform: "windows",
            device_type: "desktop-native",
            phase: "P7",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::AppInfo;

    #[test]
    fn current_identity_is_stable() {
        let info = AppInfo::current();

        assert_eq!(info.app_id, "pc-manager");
        assert_eq!(info.platform, "windows");
        assert_eq!(info.device_type, "desktop-native");
        assert_eq!(info.phase, "P7");
    }
}

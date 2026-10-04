//! Platform-neutral domain models for PC Manager Desktop.

mod cleanup;
mod health;
mod inventory;

pub use cleanup::{
    summarize_cleanup_plan, CleanupCategory, CleanupError, CleanupPlan, CleanupPlanItem,
    CleanupProviderSummary, CleanupScanOptions, CleanupScanSummary, CleanupWarning,
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
            phase: "P4A",
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
        assert_eq!(info.phase, "P4A");
    }
}

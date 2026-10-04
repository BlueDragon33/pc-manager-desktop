//! Platform-neutral domain models for PC Manager Desktop.

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
    /// Returns the immutable identity used during the P0 foundation phase.
    #[must_use]
    pub const fn p0() -> Self {
        Self {
            app_id: "pc-manager",
            platform: "windows",
            device_type: "desktop-native",
            phase: "P0",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::AppInfo;

    #[test]
    fn p0_identity_is_stable() {
        let info = AppInfo::p0();

        assert_eq!(info.app_id, "pc-manager");
        assert_eq!(info.platform, "windows");
        assert_eq!(info.device_type, "desktop-native");
        assert_eq!(info.phase, "P0");
    }
}

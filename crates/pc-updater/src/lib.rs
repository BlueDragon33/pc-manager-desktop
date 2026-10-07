//! Release-channel and update-selection policy for PC Manager Desktop.
//!
//! P9 owns packaging metadata and policy. Download/install orchestration remains
//! deliberately separate from P10 Software Updater.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReleaseChannel { Dev, Beta, Stable }

impl ReleaseChannel {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self { Self::Dev => "dev", Self::Beta => "beta", Self::Stable => "stable" }
    }

    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "dev" => Some(Self::Dev),
            "beta" => Some(Self::Beta),
            "stable" => Some(Self::Stable),
            _ => None,
        }
    }

    #[must_use]
    pub const fn accepts(self, offered: Self) -> bool {
        match self {
            Self::Dev => true,
            Self::Beta => matches!(offered, Self::Beta | Self::Stable),
            Self::Stable => matches!(offered, Self::Stable),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateDecision { UpdateAvailable, Current, DowngradeBlocked, ChannelBlocked, InvalidVersion }

#[must_use]
pub fn decide_update(
    installed_version: &str,
    installed_channel: ReleaseChannel,
    offered_version: &str,
    offered_channel: ReleaseChannel,
) -> UpdateDecision {
    if !installed_channel.accepts(offered_channel) { return UpdateDecision::ChannelBlocked; }
    let Some(installed) = parse_version(installed_version) else { return UpdateDecision::InvalidVersion; };
    let Some(offered) = parse_version(offered_version) else { return UpdateDecision::InvalidVersion; };
    match offered.cmp(&installed) {
        std::cmp::Ordering::Greater => UpdateDecision::UpdateAvailable,
        std::cmp::Ordering::Equal => UpdateDecision::Current,
        std::cmp::Ordering::Less => UpdateDecision::DowngradeBlocked,
    }
}

#[must_use]
pub fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn parse_version(value: &str) -> Option<Vec<u64>> {
    let core = value.trim().trim_start_matches('v').split(['-', '+']).next()?;
    let parts = core.split('.').map(str::parse::<u64>).collect::<Result<Vec<_>, _>>().ok()?;
    (!parts.is_empty() && parts.len() <= 4).then_some(parts)
}

#[must_use]
pub const fn component_name() -> &'static str { "pc-updater" }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stable_never_accepts_prerelease_channels() {
        assert!(ReleaseChannel::Stable.accepts(ReleaseChannel::Stable));
        assert!(!ReleaseChannel::Stable.accepts(ReleaseChannel::Beta));
        assert!(!ReleaseChannel::Stable.accepts(ReleaseChannel::Dev));
    }

    #[test]
    fn version_policy_blocks_downgrades() {
        assert_eq!(decide_update("1.2.3", ReleaseChannel::Stable, "1.3.0", ReleaseChannel::Stable), UpdateDecision::UpdateAvailable);
        assert_eq!(decide_update("1.2.3", ReleaseChannel::Stable, "1.2.3", ReleaseChannel::Stable), UpdateDecision::Current);
        assert_eq!(decide_update("1.2.3", ReleaseChannel::Stable, "1.2.2", ReleaseChannel::Stable), UpdateDecision::DowngradeBlocked);
    }

    #[test]
    fn checksum_shape_is_strict() {
        assert!(valid_sha256(&"a".repeat(64)));
        assert!(!valid_sha256("abc"));
        assert!(!valid_sha256(&"z".repeat(64)));
    }
}

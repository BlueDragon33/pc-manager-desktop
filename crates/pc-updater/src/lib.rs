//! Trusted software-update discovery and launch policy for PC Manager Desktop.
//!
//! P9 owns PC Manager's own release metadata. P10 adds a conservative third-party
//! software update provider. V1 delegates discovery and installation to the official
//! Microsoft WinGet community source and never accepts arbitrary URLs or command lines.

use serde::{Deserialize, Serialize};
#[cfg(any(target_os = "windows", test))]
use sha2::{Digest, Sha256};
#[cfg(any(target_os = "windows", test))]
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(target_os = "windows")]
use std::process::Command;

#[cfg(any(target_os = "windows", test))]
const OFFICIAL_WINGET_SOURCE_ID: &str = "Microsoft.Winget.Source_8wekyb3d8bbwe";
#[cfg(any(target_os = "windows", test))]
const OFFICIAL_WINGET_SOURCE_ARG: &str = "https://cdn.winget.microsoft.com/cache";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReleaseChannel {
    Dev,
    Beta,
    Stable,
}

impl ReleaseChannel {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Dev => "dev",
            Self::Beta => "beta",
            Self::Stable => "stable",
        }
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
pub enum UpdateDecision {
    UpdateAvailable,
    Current,
    DowngradeBlocked,
    ChannelBlocked,
    InvalidVersion,
}

#[must_use]
pub fn decide_update(
    installed_version: &str,
    installed_channel: ReleaseChannel,
    offered_version: &str,
    offered_channel: ReleaseChannel,
) -> UpdateDecision {
    if !installed_channel.accepts(offered_channel) {
        return UpdateDecision::ChannelBlocked;
    }
    let Some(installed) = parse_version(installed_version) else {
        return UpdateDecision::InvalidVersion;
    };
    let Some(offered) = parse_version(offered_version) else {
        return UpdateDecision::InvalidVersion;
    };
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
    let core = value
        .trim()
        .trim_start_matches('v')
        .split(['-', '+'])
        .next()?;
    let parts = core
        .split('.')
        .map(str::parse::<u64>)
        .collect::<Result<Vec<_>, _>>()
        .ok()?;
    (!parts.is_empty() && parts.len() <= 4).then_some(parts)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SoftwareUpdateCandidate {
    pub id: String,
    pub name: String,
    pub package_id: String,
    pub installed_version: String,
    pub available_version: String,
    pub source: String,
    pub source_verified: bool,
    pub publisher: Option<String>,
    pub publisher_verification: String,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SoftwareUpdateScan {
    pub scan_id: String,
    pub provider: String,
    pub provider_available: bool,
    pub source_verified: bool,
    pub source_detail: String,
    pub candidates: Vec<SoftwareUpdateCandidate>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SoftwareUpdateLaunchResult {
    pub candidate_id: String,
    pub package_id: String,
    pub display_name: String,
    pub launched: bool,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SoftwareUpdateError {
    pub code: String,
    pub message: String,
    pub recoverable: bool,
}

impl SoftwareUpdateError {
    #[must_use]
    pub fn new(code: impl Into<String>, message: impl Into<String>, recoverable: bool) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            recoverable,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PlannedSoftwareUpdate {
    candidate_id: String,
    name: String,
    package_id: String,
    installed_version: String,
    available_version: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SoftwareUpdatePlan {
    scan_id: String,
    candidates: Vec<PlannedSoftwareUpdate>,
}

#[derive(Debug, Default)]
pub struct SoftwareUpdatePlanStore {
    plan: Option<SoftwareUpdatePlan>,
}

impl SoftwareUpdatePlanStore {
    pub fn replace(&mut self, plan: SoftwareUpdatePlan) {
        self.plan = Some(plan);
    }

    #[must_use]
    pub fn get_by_scan_id(&self, scan_id: &str) -> Option<&SoftwareUpdatePlan> {
        self.plan.as_ref().filter(|plan| plan.scan_id == scan_id)
    }
}

#[cfg(any(target_os = "windows", test))]
#[derive(Debug, Clone, PartialEq, Eq)]
struct TrustedWingetSource {
    name: String,
    identifier: String,
    argument: String,
}

#[cfg(any(target_os = "windows", test))]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct WingetSourceExport {
    name: String,
    identifier: String,
    #[serde(rename = "Arg")]
    argument: String,
    #[serde(default)]
    trust_level: Vec<String>,
}

#[cfg(any(target_os = "windows", test))]
#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedUpgradeRow {
    name: String,
    package_id: String,
    installed_version: String,
    available_version: String,
}

#[cfg(target_os = "windows")]
fn run_winget(args: &[&str]) -> Result<std::process::Output, SoftwareUpdateError> {
    Command::new("winget.exe").args(args).output().map_err(|error| {
        SoftwareUpdateError::new(
            "winget_unavailable",
            format!("Windows Package Manager could not be started: {error}"),
            true,
        )
    })
}

#[cfg(target_os = "windows")]
fn bounded_provider_message(value: &[u8]) -> String {
    let text = String::from_utf8_lossy(value);
    let compact = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if compact.chars().count() > 320 {
        format!("{}…", compact.chars().take(320).collect::<String>())
    } else {
        compact
    }
}

#[cfg(any(target_os = "windows", test))]
fn parse_trusted_source(stdout: &str) -> Result<TrustedWingetSource, SoftwareUpdateError> {
    let start = stdout.find('{').ok_or_else(|| {
        SoftwareUpdateError::new(
            "winget_source_invalid",
            "WinGet did not return structured source metadata.",
            true,
        )
    })?;
    let end = stdout.rfind('}').ok_or_else(|| {
        SoftwareUpdateError::new(
            "winget_source_invalid",
            "WinGet source metadata was incomplete.",
            true,
        )
    })?;

    let raw: WingetSourceExport = serde_json::from_str(&stdout[start..=end]).map_err(|error| {
        SoftwareUpdateError::new(
            "winget_source_invalid",
            format!("WinGet source metadata could not be parsed: {error}"),
            true,
        )
    })?;

    let trusted = raw
        .trust_level
        .iter()
        .any(|value| value.eq_ignore_ascii_case("Trusted"));
    let official_argument = raw
        .argument
        .trim_end_matches('/')
        .eq_ignore_ascii_case(OFFICIAL_WINGET_SOURCE_ARG);

    if !raw.name.eq_ignore_ascii_case("winget")
        || raw.identifier != OFFICIAL_WINGET_SOURCE_ID
        || !official_argument
        || !trusted
    {
        return Err(SoftwareUpdateError::new(
            "winget_source_untrusted",
            "The configured WinGet source does not match PC Manager's official trusted-source policy.",
            false,
        ));
    }

    Ok(TrustedWingetSource {
        name: raw.name,
        identifier: raw.identifier,
        argument: raw.argument,
    })
}

#[cfg(any(target_os = "windows", test))]
fn valid_package_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 200
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_' | b'+')
        })
}

#[cfg(any(target_os = "windows", test))]
fn is_separator_line(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.len() >= 10
        && trimmed.bytes().all(|byte| byte == b'-' || byte == b' ')
        && trimmed.bytes().filter(|byte| *byte == b'-').count() >= 10
}

#[cfg(any(target_os = "windows", test))]
fn parse_upgrade_rows(stdout: &str) -> Vec<ParsedUpgradeRow> {
    let normalized = stdout.replace('\r', "");
    let mut in_table = false;
    let mut rows = Vec::new();

    for line in normalized.lines() {
        if !in_table {
            if is_separator_line(line) {
                in_table = true;
            }
            continue;
        }

        let parts = line.split_whitespace().collect::<Vec<_>>();
        if parts.len() < 5 {
            if !rows.is_empty() {
                break;
            }
            continue;
        }

        let source = parts[parts.len() - 1];
        if !source.eq_ignore_ascii_case("winget") {
            if !rows.is_empty() {
                break;
            }
            continue;
        }

        let available_version = parts[parts.len() - 2];
        let installed_version = parts[parts.len() - 3];
        let package_id = parts[parts.len() - 4];
        let name = parts[..parts.len() - 4].join(" ");

        if name.is_empty()
            || !valid_package_id(package_id)
            || installed_version.eq_ignore_ascii_case("unknown")
            || available_version.eq_ignore_ascii_case("unknown")
        {
            continue;
        }

        rows.push(ParsedUpgradeRow {
            name,
            package_id: package_id.to_string(),
            installed_version: installed_version.to_string(),
            available_version: available_version.to_string(),
        });
    }

    rows
}

#[cfg(any(target_os = "windows", test))]
fn candidate_id(row: &ParsedUpgradeRow) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"pc-manager-software-update-v1:");
    hasher.update(row.package_id.as_bytes());
    hasher.update(b"|");
    hasher.update(row.installed_version.as_bytes());
    hasher.update(b"|");
    hasher.update(row.available_version.as_bytes());
    let digest = hasher.finalize();
    digest[..16]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>()
}

#[cfg(any(target_os = "windows", test))]
fn new_scan_id(candidates: &[PlannedSoftwareUpdate]) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let mut hasher = Sha256::new();
    hasher.update(b"pc-manager-software-update-scan-v1:");
    hasher.update(now.to_le_bytes());
    for candidate in candidates {
        hasher.update(candidate.candidate_id.as_bytes());
    }
    let digest = hasher.finalize();
    digest[..16]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>()
}

#[cfg(target_os = "windows")]
fn verify_winget_source() -> Result<TrustedWingetSource, SoftwareUpdateError> {
    let output = run_winget(&["source", "export", "winget", "--disable-interactivity"])?;
    if !output.status.success() {
        return Err(SoftwareUpdateError::new(
            "winget_source_probe_failed",
            {
                let detail = bounded_provider_message(&output.stderr);
                if detail.is_empty() {
                    "WinGet could not export the configured winget source.".to_string()
                } else {
                    format!("WinGet source verification failed: {detail}")
                }
            },
            true,
        ));
    }
    let stdout = String::from_utf8(output.stdout).map_err(|error| {
        SoftwareUpdateError::new(
            "winget_source_encoding",
            format!("WinGet source metadata was not valid UTF-8: {error}"),
            true,
        )
    })?;
    parse_trusted_source(&stdout)
}

#[cfg(target_os = "windows")]
fn discover_rows() -> Result<Vec<ParsedUpgradeRow>, SoftwareUpdateError> {
    let output = run_winget(&[
        "upgrade",
        "--source",
        "winget",
        "--accept-source-agreements",
        "--disable-interactivity",
    ])?;

    if !output.status.success() {
        return Err(SoftwareUpdateError::new(
            "winget_upgrade_scan_failed",
            {
                let detail = bounded_provider_message(&output.stderr);
                if detail.is_empty() {
                    "WinGet could not enumerate software updates.".to_string()
                } else {
                    format!("WinGet update discovery failed: {detail}")
                }
            },
            true,
        ));
    }

    let stdout = String::from_utf8(output.stdout).map_err(|error| {
        SoftwareUpdateError::new(
            "winget_upgrade_encoding",
            format!("WinGet update output was not valid UTF-8: {error}"),
            true,
        )
    })?;
    Ok(parse_upgrade_rows(&stdout))
}

#[cfg(target_os = "windows")]
pub fn scan_software_updates(
) -> Result<(SoftwareUpdatePlan, SoftwareUpdateScan), SoftwareUpdateError> {
    let source = verify_winget_source()?;
    let rows = discover_rows()?;

    let planned = rows
        .iter()
        .map(|row| PlannedSoftwareUpdate {
            candidate_id: candidate_id(row),
            name: row.name.clone(),
            package_id: row.package_id.clone(),
            installed_version: row.installed_version.clone(),
            available_version: row.available_version.clone(),
        })
        .collect::<Vec<_>>();
    let scan_id = new_scan_id(&planned);

    let candidates = rows
        .iter()
        .map(|row| SoftwareUpdateCandidate {
            id: candidate_id(row),
            name: row.name.clone(),
            package_id: row.package_id.clone(),
            installed_version: row.installed_version.clone(),
            available_version: row.available_version.clone(),
            source: source.name.clone(),
            source_verified: true,
            publisher: None,
            publisher_verification:
                "Unavailable in P10 V1: WinGet source/package identity is verified, but PC Manager does not independently attest publisher metadata yet."
                    .to_string(),
            detail:
                "Eligible through the verified official WinGet source. Installer hash verification remains owned by WinGet."
                    .to_string(),
        })
        .collect();

    Ok((
        SoftwareUpdatePlan {
            scan_id: scan_id.clone(),
            candidates: planned,
        },
        SoftwareUpdateScan {
            scan_id,
            provider: "Windows Package Manager (WinGet)".to_string(),
            provider_available: true,
            source_verified: true,
            source_detail: format!(
                "Official source verified: {} · {}",
                source.identifier, source.argument
            ),
            candidates,
            warnings: Vec::new(),
        },
    ))
}

#[cfg(not(target_os = "windows"))]
pub fn scan_software_updates(
) -> Result<(SoftwareUpdatePlan, SoftwareUpdateScan), SoftwareUpdateError> {
    Err(SoftwareUpdateError::new(
        "unsupported_platform",
        "Software update discovery is currently implemented for Windows only.",
        false,
    ))
}

#[cfg(target_os = "windows")]
pub fn launch_software_update(
    plan: &SoftwareUpdatePlan,
    candidate_id: &str,
) -> Result<SoftwareUpdateLaunchResult, SoftwareUpdateError> {
    let planned = plan
        .candidates
        .iter()
        .find(|candidate| candidate.candidate_id == candidate_id)
        .ok_or_else(|| {
            SoftwareUpdateError::new(
                "software_update_candidate_not_found",
                "The selected update is missing or stale. Scan again before updating.",
                true,
            )
        })?
        .clone();

    verify_winget_source()?;
    let current = discover_rows()?;
    let still_valid = current.iter().any(|candidate| {
        candidate.package_id == planned.package_id
            && candidate.installed_version == planned.installed_version
            && candidate.available_version == planned.available_version
    });

    if !still_valid {
        return Err(SoftwareUpdateError::new(
            "software_update_candidate_changed",
            "The available update changed after the scan. PC Manager refused to launch a stale update; scan again.",
            true,
        ));
    }

    Command::new("winget.exe")
        .args([
            "upgrade",
            "--id",
            planned.package_id.as_str(),
            "--exact",
            "--source",
            "winget",
            "--interactive",
            "--accept-source-agreements",
            "--disable-interactivity",
        ])
        .spawn()
        .map_err(|error| {
            SoftwareUpdateError::new(
                "software_update_launch_failed",
                format!("WinGet could not launch the selected update: {error}"),
                true,
            )
        })?;

    Ok(SoftwareUpdateLaunchResult {
        candidate_id: planned.candidate_id,
        package_id: planned.package_id,
        display_name: planned.name,
        launched: true,
        message:
            "The verified WinGet update flow was launched. PC Manager did not supply a download URL, silent flag, hash bypass, force flag, or custom installer arguments."
                .to_string(),
    })
}

#[cfg(not(target_os = "windows"))]
pub fn launch_software_update(
    _plan: &SoftwareUpdatePlan,
    _candidate_id: &str,
) -> Result<SoftwareUpdateLaunchResult, SoftwareUpdateError> {
    Err(SoftwareUpdateError::new(
        "unsupported_platform",
        "Software update launch is currently implemented for Windows only.",
        false,
    ))
}

#[must_use]
pub const fn component_name() -> &'static str {
    "pc-updater"
}

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
        assert_eq!(
            decide_update(
                "1.2.3",
                ReleaseChannel::Stable,
                "1.3.0",
                ReleaseChannel::Stable
            ),
            UpdateDecision::UpdateAvailable
        );
        assert_eq!(
            decide_update(
                "1.2.3",
                ReleaseChannel::Stable,
                "1.2.3",
                ReleaseChannel::Stable
            ),
            UpdateDecision::Current
        );
        assert_eq!(
            decide_update(
                "1.2.3",
                ReleaseChannel::Stable,
                "1.2.2",
                ReleaseChannel::Stable
            ),
            UpdateDecision::DowngradeBlocked
        );
    }

    #[test]
    fn checksum_shape_is_strict() {
        assert!(valid_sha256(&"a".repeat(64)));
        assert!(!valid_sha256("abc"));
        assert!(!valid_sha256(&"z".repeat(64)));
    }

    #[test]
    fn official_winget_source_must_match_identity_origin_and_trust() {
        let source = parse_trusted_source(
            r#"{"Arg":"https://cdn.winget.microsoft.com/cache","Identifier":"Microsoft.Winget.Source_8wekyb3d8bbwe","Name":"winget","TrustLevel":["Trusted","StoreOrigin"]}"#,
        )
        .expect("official source should be trusted");
        assert_eq!(source.name, "winget");

        let error = parse_trusted_source(
            r#"{"Arg":"https://example.invalid/cache","Identifier":"Microsoft.Winget.Source_8wekyb3d8bbwe","Name":"winget","TrustLevel":["Trusted"]}"#,
        )
        .expect_err("different origin must fail closed");
        assert_eq!(error.code, "winget_source_untrusted");
    }

    #[test]
    fn upgrade_table_parser_keeps_names_with_spaces() {
        let rows = parse_upgrade_rows(
            "Name                          Id                         Version   Available Source\n\
             ------------------------------------------------------------------------------\n\
             Microsoft PowerToys           Microsoft.PowerToys        0.90.0    0.91.0    winget\n\
             Visual Studio Code            Microsoft.VisualStudioCode 1.100.0   1.101.0   winget\n\
             2 upgrades available.\n",
        );

        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].name, "Microsoft PowerToys");
        assert_eq!(rows[0].package_id, "Microsoft.PowerToys");
        assert_eq!(rows[1].available_version, "1.101.0");
    }

    #[test]
    fn malformed_or_empty_upgrade_output_returns_no_candidates() {
        assert!(parse_upgrade_rows("No applicable upgrade found.").is_empty());
        assert!(parse_upgrade_rows("----\nnot enough columns").is_empty());
    }

    #[test]
    fn candidate_ids_are_opaque_and_change_with_version_transition() {
        let first = ParsedUpgradeRow {
            name: "Example App".to_string(),
            package_id: "Example.App".to_string(),
            installed_version: "1.0.0".to_string(),
            available_version: "2.0.0".to_string(),
        };
        let mut second = first.clone();
        second.available_version = "2.1.0".to_string();

        let first_id = candidate_id(&first);
        let second_id = candidate_id(&second);
        assert_eq!(first_id.len(), 32);
        assert_ne!(first_id, second_id);
        assert!(!first_id.contains("Example"));
    }

    #[test]
    #[cfg(target_os = "windows")]
    #[test]
    fn winget_provider_smoke_is_read_only_when_winget_is_present() {
        if Command::new("winget.exe").arg("--version").output().is_err() {
            return;
        }

        match scan_software_updates() {
            Ok((_plan, scan)) => {
                assert!(scan.provider_available);
                assert!(scan.source_verified);
                assert!(scan
                    .candidates
                    .iter()
                    .all(|candidate| candidate.source_verified));
            }
            Err(error) => {
                assert!(
                    error.code.starts_with("winget_"),
                    "unexpected provider error: {}",
                    error.code
                );
            }
        }
    }

    #[test]
    fn plan_store_rejects_different_scan_id() {
        let row = ParsedUpgradeRow {
            name: "Example App".to_string(),
            package_id: "Example.App".to_string(),
            installed_version: "1.0.0".to_string(),
            available_version: "2.0.0".to_string(),
        };
        let candidate = PlannedSoftwareUpdate {
            candidate_id: candidate_id(&row),
            name: row.name,
            package_id: row.package_id,
            installed_version: row.installed_version,
            available_version: row.available_version,
        };
        let scan_id = new_scan_id(std::slice::from_ref(&candidate));
        let mut store = SoftwareUpdatePlanStore::default();
        store.replace(SoftwareUpdatePlan {
            scan_id: scan_id.clone(),
            candidates: vec![candidate],
        });

        assert!(store.get_by_scan_id(&scan_id).is_some());
        assert!(store.get_by_scan_id("stale-scan").is_none());
    }
}

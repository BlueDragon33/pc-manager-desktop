//! Update policy and artifact verification for PC Manager Desktop.
//!
//! P9 deliberately keeps installer execution out of this crate. It validates
//! release policy and downloaded artifact integrity before a future updater
//! hands a verified package to the OS-specific installation layer.

use semver::Version;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::Read;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReleaseChannel {
    Dev,
    Beta,
    Stable,
}

impl ReleaseChannel {
    #[must_use]
    pub const fn accepts(self, candidate: Self) -> bool {
        match self {
            Self::Stable => matches!(candidate, Self::Stable),
            Self::Beta => matches!(candidate, Self::Stable | Self::Beta),
            Self::Dev => true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateMetadata {
    pub version: String,
    pub channel: ReleaseChannel,
    pub mandatory: bool,
    pub sha256: String,
    pub artifact_url: String,
    #[serde(default)]
    pub release_notes: Vec<String>,
    pub published_at: String,
    pub platform: String,
    pub architecture: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateDecision {
    UpToDate,
    Available { mandatory: bool },
    ChannelNotAllowed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateError {
    pub code: &'static str,
    pub message: String,
}

impl UpdateError {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

pub fn evaluate_update(
    current_version: &str,
    enrolled_channel: ReleaseChannel,
    metadata: &UpdateMetadata,
) -> Result<UpdateDecision, UpdateError> {
    validate_metadata(metadata)?;

    if !enrolled_channel.accepts(metadata.channel) {
        return Ok(UpdateDecision::ChannelNotAllowed);
    }

    let current = Version::parse(current_version).map_err(|error| {
        UpdateError::new(
            "invalid_current_version",
            format!("Current application version is invalid: {error}"),
        )
    })?;
    let candidate = Version::parse(&metadata.version).map_err(|error| {
        UpdateError::new(
            "invalid_update_version",
            format!("Update metadata version is invalid: {error}"),
        )
    })?;

    if candidate <= current {
        Ok(UpdateDecision::UpToDate)
    } else {
        Ok(UpdateDecision::Available {
            mandatory: metadata.mandatory,
        })
    }
}

pub fn validate_metadata(metadata: &UpdateMetadata) -> Result<(), UpdateError> {
    Version::parse(&metadata.version).map_err(|error| {
        UpdateError::new(
            "invalid_update_version",
            format!("Update metadata version is invalid: {error}"),
        )
    })?;

    if metadata.sha256.len() != 64
        || !metadata
            .sha256
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
    {
        return Err(UpdateError::new(
            "invalid_update_checksum",
            "Update metadata must contain a 64-character SHA-256 digest.",
        ));
    }

    if metadata.platform != "windows" || metadata.architecture != "x86_64" {
        return Err(UpdateError::new(
            "unsupported_update_target",
            "Update package does not target Windows x86_64.",
        ));
    }

    if !metadata.artifact_url.starts_with("https://") {
        return Err(UpdateError::new(
            "insecure_update_url",
            "Production update artifacts must use HTTPS.",
        ));
    }

    if metadata.published_at.trim().is_empty() {
        return Err(UpdateError::new(
            "missing_update_timestamp",
            "Update metadata is missing its publication timestamp.",
        ));
    }

    Ok(())
}

pub fn verify_file_sha256(
    path: impl AsRef<Path>,
    expected_sha256: &str,
) -> Result<(), UpdateError> {
    let expected = expected_sha256.trim().to_ascii_lowercase();
    if expected.len() != 64 || !expected.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(UpdateError::new(
            "invalid_expected_checksum",
            "Expected checksum is not a valid SHA-256 digest.",
        ));
    }

    let path = path.as_ref();
    let mut file = File::open(path).map_err(|error| {
        UpdateError::new(
            "update_artifact_unreadable",
            format!("Unable to open update artifact: {error}"),
        )
    })?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];

    loop {
        let read = file.read(&mut buffer).map_err(|error| {
            UpdateError::new(
                "update_artifact_read_failed",
                format!("Unable to read update artifact: {error}"),
            )
        })?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }

    let actual = format!("{:x}", hasher.finalize());
    if actual == expected {
        Ok(())
    } else {
        Err(UpdateError::new(
            "update_checksum_mismatch",
            format!("Update artifact checksum mismatch: expected {expected}, got {actual}."),
        ))
    }
}

/// Returns this crate's stable component name.
#[must_use]
pub const fn component_name() -> &'static str {
    "pc-updater"
}

#[cfg(test)]
mod tests {
    use super::{
        evaluate_update, validate_metadata, verify_file_sha256, ReleaseChannel, UpdateDecision,
        UpdateMetadata,
    };
    use std::fs;

    fn metadata(channel: ReleaseChannel, version: &str) -> UpdateMetadata {
        UpdateMetadata {
            version: version.to_string(),
            channel,
            mandatory: false,
            sha256: "a".repeat(64),
            artifact_url: "https://example.invalid/pc-manager.exe".to_string(),
            release_notes: vec!["test".to_string()],
            published_at: "2026-10-07T00:00:00Z".to_string(),
            platform: "windows".to_string(),
            architecture: "x86_64".to_string(),
        }
    }

    #[test]
    fn channel_policy_is_monotonic() {
        assert!(ReleaseChannel::Stable.accepts(ReleaseChannel::Stable));
        assert!(!ReleaseChannel::Stable.accepts(ReleaseChannel::Beta));
        assert!(ReleaseChannel::Beta.accepts(ReleaseChannel::Stable));
        assert!(ReleaseChannel::Beta.accepts(ReleaseChannel::Beta));
        assert!(!ReleaseChannel::Beta.accepts(ReleaseChannel::Dev));
        assert!(ReleaseChannel::Dev.accepts(ReleaseChannel::Dev));
    }

    #[test]
    fn newer_version_is_available_only_on_allowed_channel() {
        assert_eq!(
            evaluate_update("0.1.0", ReleaseChannel::Stable, &metadata(ReleaseChannel::Stable, "0.2.0"))
                .expect("valid metadata"),
            UpdateDecision::Available { mandatory: false }
        );
        assert_eq!(
            evaluate_update("0.1.0", ReleaseChannel::Stable, &metadata(ReleaseChannel::Beta, "0.2.0"))
                .expect("valid metadata"),
            UpdateDecision::ChannelNotAllowed
        );
    }

    #[test]
    fn metadata_rejects_insecure_artifact_url() {
        let mut value = metadata(ReleaseChannel::Stable, "0.2.0");
        value.artifact_url = "http://example.invalid/update.exe".to_string();
        assert_eq!(
            validate_metadata(&value).expect_err("http must be rejected").code,
            "insecure_update_url"
        );
    }

    #[test]
    fn checksum_verification_fails_closed() {
        let path = std::env::temp_dir().join(format!(
            "pc-manager-updater-test-{}-{}.bin",
            std::process::id(),
            "checksum"
        ));
        fs::write(&path, b"pc-manager").expect("write fixture");
        let correct =
            "5971fff264e4a6a2e315d3f868c066af1dbcf342a7c1098fd7515021357104c0";
        verify_file_sha256(&path, correct).expect("checksum should match");
        let error = verify_file_sha256(&path, &"0".repeat(64))
            .expect_err("wrong checksum must fail");
        assert_eq!(error.code, "update_checksum_mismatch");
        let _ = fs::remove_file(path);
    }
}

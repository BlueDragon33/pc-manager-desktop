//! Update policy and verification layer.
//!
//! Network and installer behavior are intentionally absent in P0.

/// Returns this crate's stable component name.
#[must_use]
pub const fn component_name() -> &'static str {
    "pc-updater"
}

//! Low-overhead resource monitoring abstractions.
//!
//! Sampling implementations are introduced in P7.

/// Returns this crate's stable component name.
#[must_use]
pub const fn component_name() -> &'static str {
    "pc-monitor"
}

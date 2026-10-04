//! Windows-specific system integration for PC Manager Desktop.
//!
//! P0 intentionally contains no system mutation code.

/// Returns this crate's stable component name.
#[must_use]
pub const fn component_name() -> &'static str {
    "pc-windows"
}

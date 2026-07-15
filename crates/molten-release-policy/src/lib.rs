//! Repository shell for Molten release dependency policy checks.
//!
//! Pure admission decisions remain in `molten-core`; the binary shell owns
//! Nickel execution, repository reads, archive measurement, and rendering.

pub const BINARY_NAME: &str = "molten-release-policy";

//! Snapshot tests for core error message formatting.
//!
//! Ensures error messages remain stable and actionable.
//! Run `cargo insta review` to approve changes.

use aspen_core::VaultError;

#[test]
fn snapshot_system_prefix_reserved() {
    let err = VaultError::SystemPrefixReserved;
    insta::assert_snapshot!("system_prefix_reserved", err.to_string());
}

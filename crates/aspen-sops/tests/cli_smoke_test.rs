//! CLI smoke tests for aspen-sops binary.
//!
//! These verify the CLI parses arguments correctly without requiring
//! a running Aspen cluster.

use std::process::Command;

fn aspen_sops() -> Command {
    Command::new(env!("CARGO_BIN_EXE_aspen-sops"))
}

#[test]
fn test_help_exits_zero() {
    let output = aspen_sops().arg("--help").output().expect("failed to run aspen-sops");
    assert!(output.status.success(), "aspen-sops --help should exit 0");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("SOPS backend"), "help should mention SOPS backend");
}

#[test]
fn test_encrypt_help() {
    let output = aspen_sops().args(["encrypt", "--help"]).output().expect("failed to run aspen-sops");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("cluster-ticket") || stdout.contains("CLUSTER_TICKET"));
    assert!(stdout.contains("transit-key"));
}

#[test]
fn test_decrypt_help() {
    let output = aspen_sops().args(["decrypt", "--help"]).output().expect("failed to run aspen-sops");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("extract") || stdout.contains("EXTRACT"));
}

#[test]
fn test_edit_help() {
    let output = aspen_sops().args(["edit", "--help"]).output().expect("failed to run aspen-sops");
    assert!(output.status.success());
}

#[test]
fn test_rotate_help() {
    let output = aspen_sops().args(["rotate", "--help"]).output().expect("failed to run aspen-sops");
    assert!(output.status.success());
}

#[test]
fn test_update_keys_help() {
    let output = aspen_sops().args(["update-keys", "--help"]).output().expect("failed to run aspen-sops");
    assert!(output.status.success());
}

#[test]
fn test_unknown_command_fails() {
    let output = aspen_sops().arg("nonexistent").output().expect("failed to run aspen-sops");
    assert!(!output.status.success(), "unknown command should fail");
}

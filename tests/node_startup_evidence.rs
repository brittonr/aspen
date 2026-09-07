#![cfg(target_os = "linux")]
use std::os::fd::AsRawFd;
use std::process::Command;

// r[verify molten.startup_evidence.scope]
#[test]
fn cli_denies_invalid_policy_without_creating_bundle_or_state() {
    let root = cap_tempfile::tempdir(cap_tempfile::ambient_authority()).unwrap();
    let path = std::fs::read_link(format!("/proc/self/fd/{}", root.as_raw_fd())).unwrap();
    root.write("policy.json", b"{\"schema\":\"wrong\"}").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_molten"))
        .current_dir(&path)
        .args(["node", "startup-evidence", "verify", "--policy"])
        .arg(path.join("policy.json"))
        .arg("--bundle")
        .arg(path.join("not-created"))
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("startup-evidence-policy-json"));
    assert_eq!(root.entries().unwrap().count(), 1);
    assert!(output.stdout.is_empty());
}

#[test]
fn cli_checks_descriptor_identity_before_decode_or_member_reads() {
    let root = cap_tempfile::tempdir(cap_tempfile::ambient_authority()).unwrap();
    let path = std::fs::read_link(format!("/proc/self/fd/{}", root.as_raw_fd())).unwrap();
    let hash = "a".repeat(64);
    let policy = serde_json::json!({"schema":"molten.node-startup-cohort.v1", "descriptor_blake3":hash,
        "cohort":{"source_revision":"a".repeat(40), "source_inventory_blake3":hash, "executable_blake3":hash,
        "build_rustc_blake3":hash, "build_toolchain":"nightly-2026-05-26",
        "octet_revision":"c9b06bcf565c51d4a77d210e61b69ae51db9df25", "octet_cli_blake3":hash,
        "octet_driver_blake3":hash,"octet_lints_blake3":hash,"octet_rustc_blake3":hash,
        "octet_toolchain":"nightly-2026-03-21-x86_64-unknown-linux-gnu"}});
    root.write("policy.json", serde_json::to_vec(&policy).unwrap()).unwrap();
    root.write("bundle.json", b"malformed unapproved descriptor").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_molten"))
        .current_dir(&path)
        .args(["node", "startup-evidence", "verify", "--policy"])
        .arg(path.join("policy.json"))
        .arg("--bundle")
        .arg(&path)
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("startup-evidence-descriptor-identity"));
    assert!(output.stdout.is_empty());
    assert_eq!(root.entries().unwrap().count(), 2);
}

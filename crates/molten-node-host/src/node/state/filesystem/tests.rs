use cap_std::fs::PermissionsExt;

use super::*;

#[test]
fn actual_failed_open_preserves_typed_permission_denial() {
    // r[verify molten.testing.nixos_vm_fault_injection.storage_state_faults]
    let root = cap_tempfile::tempdir(cap_tempfile::ambient_authority()).expect("temporary node root");
    write_regular_file(&root, std::path::Path::new("allowed.preserves"), b"allowed").expect("positive writable root");
    root.set_permissions(".", cap_std::fs::Permissions::from_mode(0o500))
        .expect("deny directory writes");
    let result = write_regular_file(&root, std::path::Path::new("health-receipt.preserves"), b"health");
    root.set_permissions(".", cap_std::fs::Permissions::from_mode(0o700))
        .expect("restore test-owned root");
    let effective_uid = std::fs::read_to_string("/proc/self/status")
        .expect("process status")
        .lines()
        .find_map(|line| line.strip_prefix("Uid:"))
        .and_then(|uids| uids.split_whitespace().nth(1))
        .expect("effective UID")
        .parse::<u32>()
        .expect("numeric UID");
    if effective_uid == 0 && result.is_ok() {
        // Root may bypass mode bits; that path is not an observed denial.
        return;
    }
    assert!(matches!(
        result,
        Err(crate::error::Failure::NodeStateWriteOpenDenied { path, errno: 13 })
            if path == "health-receipt.preserves"
    ));
    assert!(root.symlink_metadata("health-receipt.preserves").is_err(), "failed open created no target");
}

use super::*;
use crate::quality::startup_snapshot::tests::fixture;
use crate::quality::startup_snapshot::tests::repin_test_data;
use crate::test_support::process_workspace;

fn write_fixture(root: &Path, policy_path: &Path) {
    let (mut policy, mut descriptor, members) = fixture();
    descriptor.cohort.executable_blake3 = measure_current_executable().unwrap();
    repin_test_data(&mut policy, &mut descriptor, &members);
    for (member, bytes) in descriptor.members.iter().zip(&members) {
        std::fs::write(root.join(member.role.filename()), bytes).unwrap();
    }
    std::fs::write(root.join("bundle.json"), serde_json::to_vec(&descriptor).unwrap()).unwrap();
    std::fs::write(policy_path, serde_json::to_vec(&policy).unwrap()).unwrap();
}

// r[verify molten.startup_evidence.scope]
#[test]
fn verification_only_report_never_authorizes_startup() {
    let root = process_workspace("startup-verify").unwrap();
    let policy_dir = process_workspace("startup-policy").unwrap();
    let policy = policy_dir.join("cohort.json");
    write_fixture(&root, &policy);
    let before: Vec<_> = std::fs::read_dir(&root).unwrap().map(|entry| entry.unwrap().file_name()).collect();
    let report = verify(&policy, &root).unwrap();
    assert_eq!(report.disposition, "verification-only");
    assert_eq!(report.verified_members, 10);
    assert!(!report.startup_authorized);
    assert!(!report.execution_established);
    let after: Vec<_> = std::fs::read_dir(&root).unwrap().map(|entry| entry.unwrap().file_name()).collect();
    assert_eq!(before, after, "read-only verification wrote to bundle");
}

#[test]
fn wrong_descriptor_is_rejected_before_missing_members() {
    let root = process_workspace("startup-descriptor").unwrap();
    let (policy, _, _) = fixture();
    let policy_path = root.join("policy.json");
    std::fs::write(&policy_path, serde_json::to_vec(&policy).unwrap()).unwrap();
    std::fs::write(root.join("bundle.json"), b"not admitted JSON").unwrap();
    assert!(verify(&policy_path, &root).unwrap_err().to_string().contains("descriptor-identity"));
    assert_eq!(std::fs::read_dir(&root).unwrap().count(), 2);
}

#[test]
fn wrong_runtime_binary_is_rejected_before_member_reads() {
    let root = process_workspace("startup-binary").unwrap();
    let (policy, descriptor, _) = fixture();
    let policy_path = root.join("policy.json");
    std::fs::write(&policy_path, serde_json::to_vec(&policy).unwrap()).unwrap();
    std::fs::write(root.join("bundle.json"), serde_json::to_vec(&descriptor).unwrap()).unwrap();
    assert!(verify(&policy_path, &root).unwrap_err().to_string().contains("Executable"));
}

#[test]
fn unknown_fields_cannot_add_authority_or_paths() {
    let (policy, descriptor, _) = fixture();
    let mut p = serde_json::to_value(policy).unwrap();
    p["trust_anything"] = true.into();
    assert!(serde_json::from_value::<TrustedCohort>(p).is_err());
    let mut d = serde_json::to_value(descriptor).unwrap();
    d["members"][0]["path"] = "../secret".into();
    assert!(serde_json::from_value::<Descriptor>(d).is_err());
}

#[cfg(unix)]
#[test]
fn symlinks_directories_and_oversized_leaves_fail_closed() {
    let root = process_workspace("startup-leaves").unwrap();
    let outside = process_workspace("startup-outside").unwrap();
    std::fs::write(outside.join("outside"), b"secret").unwrap();
    std::os::unix::fs::symlink(outside.join("outside"), root.join("link")).unwrap();
    std::fs::create_dir(root.join("directory")).unwrap();
    std::fs::write(root.join("large"), b"1234").unwrap();
    let cap = Dir::open_ambient_dir(&root, cap_std::ambient_authority()).unwrap();
    for name in ["link", "directory", "large"] {
        assert!(read_regular(&cap, Path::new(name), 3).unwrap_err().to_string().contains("not-bounded-regular"));
    }
    assert_eq!(std::fs::read(outside.join("outside")).unwrap(), b"secret");
}

// r[verify molten.startup_evidence.scope]
#[test]
fn admission_rejects_relative_paths_and_missing_policy() {
    let root = process_workspace("startup-admit-relative").unwrap();
    let error = admit_startup_source_gate(Path::new("policy.json"), &root).unwrap_err();
    assert!(error.to_string().contains("paths-must-be-absolute"));
    let missing = process_workspace("startup-admit-missing").unwrap();
    let policy = process_workspace("startup-admit-missing-policy").unwrap().join("cohort.json");
    assert!(admit_startup_source_gate(&policy, &missing).is_err());
    assert_eq!(std::fs::read_dir(&missing).unwrap().count(), 0, "failed admission wrote no state");
}

#[test]
fn verified_fixture_cannot_authorize_lifecycle_startup() {
    let root = process_workspace("startup-admit").unwrap();
    let policy_dir = process_workspace("startup-admit-policy").unwrap();
    let policy = policy_dir.join("cohort.json");
    write_fixture(&root, &policy);
    let (cap, plan) = load_plan(&policy, &root).unwrap();
    let gate = verified_source_gate(&cap, &plan).unwrap();
    assert!(
        admit_startup_source_gate(&policy, &root)
            .unwrap_err()
            .to_string()
            .contains("real-cohort-not-approved")
    );
    assert!(gate.receipt_ref.starts_with("blake3:"), "receipt ref {}", gate.receipt_ref);
    let text = crate::preserves_rail::to_text(&gate.receipt_value).unwrap();
    assert!(text.contains("pass"), "receipt value: {text}");
    let state = root.join("must-not-create-state");
    let paths = crate::node_daemon::StartupEvidencePaths {
        policy: &policy,
        bundle: &root,
    };
    let error = crate::node_daemon::run_local(&crate::node_daemon::RunInput {
        state_root: &state,
        startup_evidence: Some(paths),
    })
    .unwrap_err();
    assert!(error.to_string().contains("real-cohort-not-approved"));
    assert!(!state.exists());
    assert!(crate::node_daemon::run_local_source_gate_for_serve(Some(paths)).is_err());
    assert!(crate::node_daemon::run_local_source_gate_for_serve(None).is_err());
}

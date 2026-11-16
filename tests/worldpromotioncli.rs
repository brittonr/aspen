#![feature(register_tool)]
#![register_tool(tigerstyle)]
#![allow(
    tigerstyle::non_trait_imports,
    reason = "the integration test names each external type and command at the CLI boundary"
)]
#![allow(
    tigerstyle::no_unwrap,
    reason = "fixture setup fails fast and the test name identifies the exercised path"
)]

mod worldpromotionsupport;

use molten_core::world_promotion::WorldAttemptOutcome;
use molten_core::world_promotion::WorldAttemptRecord;
use molten_core::world_promotion::WorldReleaseAttemptRef;
use molten_core::world_promotion::WorldReleaseState;
use worldpromotionsupport::*;

#[test]
fn public_promote_command_commits_head_reservations_and_receipt() {
    let temporary = tempfile::tempdir().expect("temporary root");
    let root = DirectoryRoot::open(temporary.path()).expect("state root");
    root.create_layout().expect("layout");
    let policy_ref = WorldHeadPolicyRef::new(reference("policy")).expect("policy");
    seed_head(&root, &policy_ref);
    let (signer_hex, request_path, authority_path, closure_path) = fixtures(temporary.path(), &policy_ref);
    let stdout = run_promote(temporary.path(), &request_path, &authority_path, &closure_path, &signer_hex);
    assert!(stdout.contains("persistence="), "stdout={stdout}");

    let storage = root.namespace(NamespaceKind::Storage).expect("storage");
    let store = LocalWorldPromotionStore::open(&storage).expect("promotion store");
    assert_eq!(store.list_reservations().expect("reservations").len(), 1);
    let head = store.read_head(&WorldBranchId::new("release").expect("branch")).expect("head").expect("head");
    assert_eq!(head.head.as_str(), reference("candidate"));
    let receipts = storage
        .existing_subdir(&RelativePath::parse("world-promotion/receipts").expect("path"))
        .expect("dir")
        .expect("receipt directory");
    assert_eq!(receipts.list_entries().expect("entries").len(), 1);
}

#[test]
fn reconcile_command_classifies_an_interrupted_attempt_once() {
    let temporary = tempfile::tempdir().expect("temporary root");
    let root = DirectoryRoot::open(temporary.path()).expect("state root");
    root.create_layout().expect("layout");
    let policy_ref = WorldHeadPolicyRef::new(reference("policy")).expect("policy");
    seed_head(&root, &policy_ref);
    let (signer_hex, request_path, authority_path, closure_path) = fixtures(temporary.path(), &policy_ref);
    run_promote(temporary.path(), &request_path, &authority_path, &closure_path, &signer_hex);

    let storage = root.namespace(NamespaceKind::Storage).expect("storage");
    let attempt_ref = WorldReleaseAttemptRef::new(reference("interrupted-attempt")).expect("attempt ref");
    {
        // A previous process stored the attempt, then stopped before recording the dispatcher result.
        let mut store = LocalWorldPromotionStore::open(&storage).expect("promotion store");
        let reservation_ref = store.list_reservations().expect("reservations")[0].reservation_ref.clone();
        let mut reservation = store.claim_reservation(&reservation_ref).expect("claim").expect("reservation");
        reservation.state = WorldReleaseState::Attempting;
        store.update_reservation(&reservation).expect("attempting reservation");
        store
            .store_attempt(&WorldAttemptRecord {
                reservation_ref,
                attempt_ref: attempt_ref.clone(),
                state: WorldReleaseState::Attempting,
                observation_ref: None,
                outcome: WorldAttemptOutcome::Unknown,
                external_completion_proven: false,
            })
            .expect("attempting attempt");
    }

    let first = run_reconcile(temporary.path());
    assert!(first.contains("recovered_interrupted_attempts=1\n"), "stdout={first}");
    assert!(first.contains(&format!("recovered_attempt={attempt_ref}\n")), "stdout={first}");
    let second = run_reconcile(temporary.path());
    assert!(second.contains("recovered_interrupted_attempts=0\n"), "stdout={second}");

    let store = LocalWorldPromotionStore::open(&storage).expect("promotion store");
    let attempt = store.read_attempt(&attempt_ref).expect("attempt read").expect("attempt");
    assert_eq!(attempt.state, WorldReleaseState::Uncertain);
    assert_eq!(attempt.outcome, WorldAttemptOutcome::Unknown);
    assert_eq!(store.list_reservations().expect("reservations")[0].state, WorldReleaseState::Uncertain);
    let receipts = storage
        .existing_subdir(&RelativePath::parse("world-promotion/receipts").expect("path"))
        .expect("dir")
        .expect("receipt directory");
    assert_eq!(receipts.list_entries().expect("entries").len(), 2);
}

fn run_reconcile(root: &std::path::Path) -> String {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_molten"))
        .args([
            "world-promotion",
            "reconcile",
            "--state-root",
            root.to_str().expect("path"),
        ])
        .output()
        .expect("reconcile command");
    assert!(
        output.status.success(),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("utf-8 stdout")
}

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

#[path = "worldpromotioncli/actions.rs"]
mod actions;
#[path = "worldpromotioncli/support.rs"]
mod support;

use molten_core::world_promotion::WorldAttemptOutcome;
use molten_core::world_promotion::WorldAttemptRecord;
use molten_core::world_promotion::WorldReleaseAttemptRef;
use molten_core::world_promotion::WorldReleaseState;
use support::*;
use worldpromotionsupport::*;

#[test]
fn public_promote_command_commits_head_reservations_and_receipt() {
    let temporary = tempfile::tempdir().expect("temporary root");
    let root = DirectoryRoot::open(temporary.path()).expect("state root");
    root.create_layout().expect("layout");
    let policy_ref = WorldHeadPolicyRef::new(reference("policy")).expect("policy");
    seed(&root, &policy_ref);
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
    seed(&root, &policy_ref);
    let (signer_hex, request_path, authority_path, closure_path) = fixtures(temporary.path(), &policy_ref);
    run_promote(temporary.path(), &request_path, &authority_path, &closure_path, &signer_hex);

    let storage = root.namespace(NamespaceKind::Storage).expect("storage");
    let (_, attempt_ref) = interrupt_attempt(&storage);

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

// r[verify molten.promotion_identity.restart_binding]
// r[verify molten.world_promotion.reconciliation]
#[test]
fn resubmitted_promotion_after_a_lost_result_replays_without_a_second_publication() {
    let promoted = Promoted::new();
    let before = promoted.store().list_reservations().expect("reservations");
    // The caller lost the first result; a new process resubmits the same logical operation.
    let stdout = run_promote(
        promoted.temporary.path(),
        &promoted.request_path,
        &promoted.authority_path,
        &promoted.closure_path,
        &promoted.signer_hex,
    );
    assert!(stdout.contains("replayed=true\n"), "stdout={stdout}");
    assert!(stdout.contains("external_effects_completed=false\n"), "stdout={stdout}");
    {
        let store = promoted.store();
        assert_eq!(store.list_reservations().expect("reservations"), before);
        let head = store.read_head(&WorldBranchId::new("release").expect("branch")).expect("head").expect("head");
        assert_eq!(head.head.as_str(), reference("candidate"));
        assert_eq!(head.generation, 2);
    }

    // A different request under the same logical operation is a conflict, not a new operation.
    let mut conflicting = read(&promoted.request_path);
    conflicting["candidate_head"] = serde_json::Value::String(reference("other-candidate"));
    let conflicting_path = promoted.temporary.path().join("conflicting-request.json");
    write(&conflicting_path, conflicting);
    let output = run_promote_output(
        promoted.temporary.path(),
        &conflicting_path,
        &promoted.authority_path,
        &promoted.closure_path,
        &promoted.signer_hex,
    );
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("promotion identity conflict"), "stderr={stderr}");
    assert_eq!(promoted.store().list_reservations().expect("reservations"), before);
}

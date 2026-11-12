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

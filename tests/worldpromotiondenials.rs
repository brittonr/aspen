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
fn stale_revoked_and_incomplete_promotions_leave_head_and_outbox_unchanged() {
    let temporary = tempfile::tempdir().expect("temporary root");
    let root = DirectoryRoot::open(temporary.path()).expect("state root");
    root.create_layout().expect("layout");
    let policy_ref = WorldHeadPolicyRef::new(reference("policy")).expect("policy");
    seed_head(&root, &policy_ref);
    let (signer_hex, request_path, authority_path, closure_path) = fixtures(temporary.path(), &policy_ref);

    let mut stale = read(&request_path);
    stale["expected_generation"] = serde_json::Value::from(2);
    write(&request_path, stale);
    let output = run_promote_output(temporary.path(), &request_path, &authority_path, &closure_path, &signer_hex);
    assert!(!output.status.success());
    assert_unchanged(&root);

    let (signer_hex, request_path, authority_path, closure_path) = fixtures(temporary.path(), &policy_ref);
    resign_authority(&authority_path, false);
    let output = run_promote_output(temporary.path(), &request_path, &authority_path, &closure_path, &signer_hex);
    assert!(!output.status.success());
    assert_unchanged(&root);

    let (signer_hex, request_path, authority_path, closure_path) = fixtures(temporary.path(), &policy_ref);
    resign_closure(&closure_path, false);
    let output = run_promote_output(temporary.path(), &request_path, &authority_path, &closure_path, &signer_hex);
    assert!(!output.status.success());
    assert_unchanged(&root);
}

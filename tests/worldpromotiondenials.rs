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
    seed(&root, &policy_ref);
    let (signer_hex, request_path, authority_path, closure_path) = fixtures(temporary.path(), &policy_ref);

    let mut stale = read(&request_path);
    stale["expected_generation"] = serde_json::Value::from(2);
    write(&request_path, stale);
    let output = run_promote_output(temporary.path(), &request_path, &authority_path, &closure_path, &signer_hex);
    assert!(!output.status.success());
    head::assert_unchanged(&root);

    let (signer_hex, request_path, authority_path, closure_path) = fixtures(temporary.path(), &policy_ref);
    signed::resign_authority(&authority_path, false);
    let output = run_promote_output(temporary.path(), &request_path, &authority_path, &closure_path, &signer_hex);
    assert!(!output.status.success());
    head::assert_unchanged(&root);

    let (signer_hex, request_path, authority_path, closure_path) = fixtures(temporary.path(), &policy_ref);
    signed::resign_closure(&closure_path, false);
    let output = run_promote_output(temporary.path(), &request_path, &authority_path, &closure_path, &signer_hex);
    assert!(!output.status.success());
    head::assert_unchanged(&root);
}

fn assert_material_denied(output: &std::process::Output) {
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("IntentMaterialUnavailable"), "stderr={stderr}");
}

// r[verify molten.world_promotion.transaction]
#[test]
fn missing_or_mismatched_intent_material_leaves_head_and_outbox_unchanged() {
    let temporary = tempfile::tempdir().expect("temporary root");
    let root = DirectoryRoot::open(temporary.path()).expect("state root");
    root.create_layout().expect("layout");
    let policy_ref = WorldHeadPolicyRef::new(reference("policy")).expect("policy");
    seed(&root, &policy_ref);

    // The signed closure names the material, but nothing was published.
    let (signer_hex, request_path, authority_path, closure_path) =
        signed::fixtures_naming_material(temporary.path(), &policy_ref, "adapter");
    let output = run_promote_output(temporary.path(), &request_path, &authority_path, &closure_path, &signer_hex);
    assert_material_denied(&output);
    head::assert_unchanged(&root);

    // The bound material exists but names another adapter than the request declares.
    let (signer_hex, request_path, authority_path, closure_path) =
        signed::fixtures_naming_material(temporary.path(), &policy_ref, "other-adapter");
    publish_intent(temporary.path(), "other-adapter");
    let output = run_promote_output(temporary.path(), &request_path, &authority_path, &closure_path, &signer_hex);
    assert_material_denied(&output);
    head::assert_unchanged(&root);

    // Publishing the matching material lets the same signed inputs promote.
    let (signer_hex, request_path, authority_path, closure_path) = fixtures(temporary.path(), &policy_ref);
    let stdout = run_promote(temporary.path(), &request_path, &authority_path, &closure_path, &signer_hex);
    assert!(stdout.contains("replayed=false\n"), "stdout={stdout}");
}

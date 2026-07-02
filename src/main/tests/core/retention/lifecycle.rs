fn pin_unpin_and_tombstone(dir: &Path, fixture: &RetentionFixture) {
    let pin_out = dir.join("pin.preserves");
    run_retention_command(RetentionCommand::Pin(cli_retention::command::base::Pin {
        root: fixture.root.clone(),
        object_ref: fixture.object_ref.clone(),
        object_kind: "encrypted-ref".to_string(),
        retention_class: molten::retention::CLASS_PRIVATE_SECRET_REF.to_string(),
        source: molten::retention::SOURCE_SECRET_REDACTION.to_string(),
        reason: "reveal audit pending".to_string(),
        owner_ref: fixture.owner_ref.clone(),
        expiry_ref: None,
        policy_refs: vec![fixture.policy_ref.clone()],
        evidence_refs: vec![fixture.evidence_ref.clone()],
        has_authority: true,
        pin_out: Some(pin_out.clone()),
        receipt_out: Some(dir.join("pin-receipt.preserves")),
    }))
    .expect("pin retention object");
    let pin = molten::retention::parse_pin(&read_preserves_file(&pin_out).expect("read pin")).expect("parse pin");
    deny_pinned_delete(dir, fixture);
    unpin_retention_object(dir, fixture, pin.pin_ref);
    tombstone_retention_object(dir, fixture);
}

fn deny_pinned_delete(dir: &Path, fixture: &RetentionFixture) {
    let denied_receipt = dir.join("delete-denied.preserves");
    run_retention_command(retention_check(
        fixture,
        molten::retention::ACTION_DELETE,
        Some(denied_receipt.clone()),
    ))
    .expect("deny pinned delete");
    let denied = molten::retention::parse_retention_receipt(
        &read_preserves_file(&denied_receipt).expect("read denied receipt"),
    )
    .expect("parse denied receipt");
    assert_eq!(denied.decision, "deny");
}

fn unpin_retention_object(dir: &Path, fixture: &RetentionFixture, pin_ref: String) {
    run_retention_command(RetentionCommand::Unpin(cli_retention::command::base::Unpin {
        root: fixture.root.clone(),
        pin_ref,
        requester_ref: fixture.owner_ref.clone(),
        policy_refs: vec![fixture.policy_ref.clone()],
        evidence_refs: vec![fixture.evidence_ref.clone()],
        has_authority: true,
        receipt_out: Some(dir.join("unpin-receipt.preserves")),
    }))
    .expect("unpin retention object");
}

fn tombstone_retention_object(dir: &Path, fixture: &RetentionFixture) {
    let tombstone_receipt = dir.join("tombstone-receipt.preserves");
    run_retention_command(retention_check(
        fixture,
        molten::retention::ACTION_TOMBSTONE,
        Some(tombstone_receipt.clone()),
    ))
    .expect("tombstone retention object");
    let tombstone = molten::retention::parse_retention_receipt(
        &read_preserves_file(&tombstone_receipt).expect("read tombstone receipt"),
    )
    .expect("parse tombstone receipt");
    assert_eq!(tombstone.decision, "pass");
    assert!(tombstone.tombstone_ref.is_some());
}

fn retention_check(fixture: &RetentionFixture, action: &str, receipt_out: Option<PathBuf>) -> RetentionCommand {
    RetentionCommand::Check(cli_retention::command::ops::Check {
        root: fixture.root.clone(),
        object_ref: fixture.object_ref.clone(),
        object_kind: "encrypted-ref".to_string(),
        retention_class: molten::retention::CLASS_PRIVATE_SECRET_REF.to_string(),
        action: action.to_string(),
        requester_ref: fixture.owner_ref.clone(),
        is_reference_index_complete: true,
        retained_refs: Vec::new(),
        remote_refs: Vec::new(),
        policy_refs: vec![fixture.policy_ref.clone()],
        evidence_refs: vec![fixture.evidence_ref.clone()],
        has_delete_authority: true,
        has_remote_gc_clearance: true,
        receipt_out,
    })
}

fn run_retention_fixture(dir: &Path) {
    let fixture_out = dir.join("fixture");
    run_retention_command(RetentionCommand::RunFixture(cli_retention::command::ops::RunFixture {
        out: fixture_out.clone(),
    }))
    .expect("retention fixture");
    assert!(fixture_out.join("tombstone.preserves").exists());
}

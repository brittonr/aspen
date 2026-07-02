fn remote_clearance_roundtrip(dir: &Path, fixture: &RetentionFixture) -> ClearanceFixture {
    let clearance = write_remote_clearance(dir, fixture);
    let response_out = respond_to_clearance(
        dir,
        fixture,
        &clearance,
        vec![cli_synthetic_ref("retention-peer-evidence").expect("peer evidence ref")],
        Vec::new(),
        "remote-clearance-response.preserves",
    );
    let import_out = import_clearance(dir, fixture, &clearance, response_out, "remote-clearance-import.preserves");
    let imported = molten::retention::parse_retention_remote_gc_clearance_import(
        &read_preserves_file(&import_out).expect("read clearance import"),
    )
    .expect("parse clearance import");
    assert_eq!(imported.decision, "pass");
    assert!(imported.clearance_ref.is_some());
    clearance
}

fn write_remote_clearance(dir: &Path, fixture: &RetentionFixture) -> ClearanceFixture {
    let clearance_out = dir.join("remote-clearance.preserves");
    let remote_ref = cli_synthetic_ref("retention-remote").expect("remote ref");
    let peer_ref = cli_synthetic_ref("retention-peer").expect("peer ref");
    run_retention_command(RetentionCommand::Clearance(cli_retention::command::base::Record {
        root: fixture.root.clone(),
        decision: "pass".to_string(),
        requester_ref: fixture.owner_ref.clone(),
        peer_ref: peer_ref.clone(),
        object_ref: fixture.object_ref.clone(),
        object_kind: "encrypted-ref".to_string(),
        retention_class: molten::retention::CLASS_PRIVATE_SECRET_REF.to_string(),
        action: molten::retention::ACTION_DELETE.to_string(),
        remote_ref: remote_ref.clone(),
        policy_ref: fixture.policy_ref.clone(),
        authority_ref: fixture.authority_ref.clone(),
        evidence_refs: vec![fixture.evidence_ref.clone()],
        retained_refs: Vec::new(),
        is_stale: false,
        revoked_refs: Vec::new(),
        diagnostics: Vec::new(),
        out: Some(clearance_out.clone()),
    }))
    .expect("retention remote clearance");
    show_retention_artifact(clearance_out, "show retention remote clearance");
    let request_out = write_clearance_request(dir, fixture, &remote_ref, &peer_ref);
    ClearanceFixture {
        request_out,
        remote_ref,
        peer_ref,
    }
}

fn write_clearance_request(dir: &Path, fixture: &RetentionFixture, remote_ref: &str, peer_ref: &str) -> PathBuf {
    let request_out = dir.join("remote-clearance-request.preserves");
    run_retention_command(RetentionCommand::ClearanceRequest(cli_retention::command::base::Request {
        root: fixture.root.clone(),
        requester_ref: fixture.owner_ref.clone(),
        peer_ref: peer_ref.to_string(),
        object_ref: fixture.object_ref.clone(),
        object_kind: "encrypted-ref".to_string(),
        retention_class: molten::retention::CLASS_PRIVATE_SECRET_REF.to_string(),
        action: molten::retention::ACTION_DELETE.to_string(),
        remote_ref: remote_ref.to_string(),
        policy_ref: fixture.policy_ref.clone(),
        authority_ref: fixture.authority_ref.clone(),
        evidence_refs: vec![fixture.evidence_ref.clone()],
        out: Some(request_out.clone()),
    }))
    .expect("retention remote clearance request");
    show_retention_artifact(request_out.clone(), "show retention remote clearance request");
    request_out
}

fn respond_to_clearance(
    dir: &Path,
    fixture: &RetentionFixture,
    clearance: &ClearanceFixture,
    evidence_refs: Vec<String>,
    retained_refs: Vec<String>,
    file_name: &str,
) -> PathBuf {
    let response_out = dir.join(file_name);
    run_retention_command(RetentionCommand::ClearanceRespond(cli_retention::command::base::Respond {
        root: fixture.root.clone(),
        request: clearance.request_out.clone(),
        evidence_refs,
        retained_refs,
        is_stale: false,
        revoked_refs: Vec::new(),
        diagnostics: Vec::new(),
        out: Some(response_out.clone()),
    }))
    .expect("retention remote clearance response");
    show_retention_artifact(response_out.clone(), "show retention remote clearance response");
    response_out
}

fn import_clearance(
    dir: &Path,
    fixture: &RetentionFixture,
    clearance: &ClearanceFixture,
    response: PathBuf,
    file_name: &str,
) -> PathBuf {
    let import_out = dir.join(file_name);
    run_retention_command(RetentionCommand::ClearanceImport(cli_retention::command::base::Import {
        root: fixture.root.clone(),
        request: clearance.request_out.clone(),
        response,
        expected_peer_ref: Some(clearance.peer_ref.clone()),
        expected_remote_ref: Some(clearance.remote_ref.clone()),
        out: Some(import_out.clone()),
    }))
    .expect("retention remote clearance import");
    show_retention_artifact(import_out.clone(), "show retention remote clearance import");
    import_out
}

fn retained_clearance_is_denied(dir: &Path, fixture: &RetentionFixture, clearance: ClearanceFixture) {
    let retained_refs = vec![cli_synthetic_ref("retention-remote-retained").expect("remote retained ref")];
    let response = respond_to_clearance(
        dir,
        fixture,
        &clearance,
        Vec::new(),
        retained_refs,
        "remote-clearance-retained-response.preserves",
    );
    let import_out = import_clearance(dir, fixture, &clearance, response, "remote-clearance-retained-import.preserves");
    let imported = molten::retention::parse_retention_remote_gc_clearance_import(
        &read_preserves_file(&import_out).expect("read retained clearance import"),
    )
    .expect("parse retained clearance import");
    assert_eq!(imported.decision, "deny");
    assert!(imported.clearance_ref.is_none());
    assert!(imported.diagnostics.iter().any(|diagnostic| diagnostic.contains("retained")));
}

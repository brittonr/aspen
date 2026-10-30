
#[test]
fn cli_node_control_request_and_deny_receipt_work() -> CliResult<()> {
    let dir = temp_dir("cli-node-control")?;
    let request = dir.join("node-control-request.preserves");
    let receipt = dir.join("node-control-receipt.preserves");
    let provenance = dir.join("node-control-provenance.preserves");
    let payload_ref = test_ref("node-control-payload")?;
    let startup_ref = test_ref("node-startup")?;

    let provenance_out = molten_cmd()
        .args(["test", "node", "provenance-fixture", "--artifact-ref"])
        .arg(&payload_ref)
        .args(["--out"])
        .arg(&provenance)
        .output()?;
    assert_success(&provenance_out, "node provenance fixture");
    assert_eq!(molten::ledger::artifact_kind(&read_preserves(&provenance)?), "provenance-record");

    let request_out = molten_cmd()
        .args(["test", "node", "control-request", "--operation", "gate", "--payload"])
        .arg(&payload_ref)
        .args(["--out"])
        .arg(&request)
        .output()?;
    assert_success(&request_out, "node control request");
    assert_eq!(molten::ledger::artifact_kind(&read_preserves(&request)?), "node-control-request");

    let deny = molten_cmd()
        .args(["test", "node", "control-deny"])
        .arg(&request)
        .args(["--startup"])
        .arg(&startup_ref)
        .args(["--diagnostic", "missing authority/resource", "--receipt-out"])
        .arg(&receipt)
        .output()?;
    assert_success(&deny, "node control deny");
    let receipt_value = read_preserves(&receipt)?;
    assert_eq!(molten::ledger::artifact_kind(&receipt_value), "node-control-receipt");
    let text = molten::preserves_rail::to_text(&receipt_value)?;
    assert!(text.contains("missing authority/resource"));
    Ok(())
}

#[test]
#[ignore = "requires independently provisioned MOLTEN_TEST_SOURCE_TRUST signed producer output"]
fn cli_node_control_ingress_build_publish_deliver_work() -> CliResult<()> {
    let dir = temp_dir("cli-node-control-ingress")?;
    let trust = std::env::var_os(SOURCE_TRUST_ENV);
    let state_root = dir.join("node-state");
    let request = dir.join("request.preserves");
    let envelope = dir.join("ingress-envelope.preserves");
    let publish_receipt = dir.join("ingress-publish.preserves");
    let deliver_receipt = dir.join("ingress-deliver.preserves");
    let loop_receipt = dir.join("loop.preserves");
    let authority_ref = test_ref("ingress-authority")?;
    let policy_ref = test_ref("ingress-policy")?;
    let resource_ref = test_ref("ingress-resource")?;
    let bootstrap_ref = test_ref("ingress-bootstrap")?;

    start_state(trust.as_deref(), &state_root, "node:cli-ingress", "node ingress init", "node ingress run")?;
    write_status_request(&request, &authority_ref, &policy_ref, &resource_ref, "node ingress request")?;
    let envelope_ref = write_envelope(EnvelopeArgs {
        path: &envelope,
        request: &request,
        bootstrap_ref: &bootstrap_ref,
        authority_ref: &authority_ref,
        policy_ref: &policy_ref,
        resource_ref: &resource_ref,
        label: "node ingress build",
    })?;
    publish_envelope(&state_root, &envelope, &publish_receipt, "node ingress publish")?;
    deliver_envelope(&state_root, &envelope_ref, &deliver_receipt, "node ingress deliver")?;
    let loop_out = run_once(trust.as_deref(), &state_root, &loop_receipt, "node ingress loop")?;
    assert!(stdout(&loop_out).contains("processed=1"));
    Ok(())
}

#[test]
#[ignore = "requires independently provisioned MOLTEN_TEST_SOURCE_TRUST signed producer output"]
fn cli_node_live_ingress_loopback_enqueues_request() -> CliResult<()> {
    let dir = temp_dir("cli-node-live-ingress")?;
    let trust = std::env::var_os(SOURCE_TRUST_ENV);
    let state_root = dir.join("node-state");
    let request = dir.join("status.preserves");
    let publish_receipt = dir.join("publish.preserves");
    let receive_receipt = dir.join("receive.preserves");
    let authority_grant = dir.join("authority-grant.preserves");
    let live_ticket = dir.join("live-ticket.preserves");
    let peer_admission = dir.join("peer-admission.preserves");
    let policy_ref = test_ref("live-policy")?;
    let resource_ref = test_ref("live-resource")?;

    start_state(trust.as_deref(), &state_root, "node:cli-live", "node live init", "node live run")?;
    let authority_ref = grant_fixture(GrantArgs {
        root: &state_root,
        grant: &authority_grant,
        peer: "peer:cli-live",
        node: "node:cli-live",
        policy_ref: &policy_ref,
        label: "node live authority grant",
    })?;
    ticket_export(&state_root, &live_ticket, &policy_ref, "node live ticket export")?;
    let bootstrap_ref = peer_admit(AdmitArgs {
        root: &state_root,
        receipt: &peer_admission,
        peer: "peer:cli-live",
        policy_ref: &policy_ref,
        ticket: &live_ticket,
        label: "node live peer admit",
    })?;
    write_status_request(&request, &authority_ref, &policy_ref, &resource_ref, "node live status request")?;
    run_loopback(LoopbackArgs {
        root: &state_root,
        request: &request,
        publish: &publish_receipt,
        receive: &receive_receipt,
        bootstrap_ref: &bootstrap_ref,
        authority_ref: &authority_ref,
        policy_ref: &policy_ref,
        resource_ref: &resource_ref,
    })?;
    Ok(())
}

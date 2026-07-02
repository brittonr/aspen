
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
fn cli_node_control_ingress_build_publish_deliver_work() -> CliResult<()> {
    let dir = temp_dir("cli-node-control-ingress")?;
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

    start_state(&state_root, "node:cli-ingress", "node ingress init", "node ingress run")?;
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
    let loop_out = run_once(&state_root, &loop_receipt, "node ingress loop")?;
    assert!(stdout(&loop_out).contains("processed=1"));
    Ok(())
}

#[test]
fn cli_node_live_ingress_loopback_enqueues_request() -> CliResult<()> {
    let dir = temp_dir("cli-node-live-ingress")?;
    let state_root = dir.join("node-state");
    let request = dir.join("status.preserves");
    let publish_receipt = dir.join("publish.preserves");
    let receive_receipt = dir.join("receive.preserves");
    let authority_grant = dir.join("authority-grant.preserves");
    let live_ticket = dir.join("live-ticket.preserves");
    let peer_admission = dir.join("peer-admission.preserves");
    let policy_ref = test_ref("live-policy")?;
    let resource_ref = test_ref("live-resource")?;

    start_state(&state_root, "node:cli-live", "node live init", "node live run")?;
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

#[test]
fn cli_node_live_ticket_and_authority_import_receipts_work() -> CliResult<()> {
    let dir = temp_dir("cli-node-live-import")?;
    let receiver_root = dir.join("receiver-node");
    let sender_root = dir.join("sender-node");
    let bundle_sender_root = dir.join("bundle-sender-node");
    let bundle_apply_root = dir.join("bundle-apply-node");
    let authority_grant = dir.join("authority-grant.preserves");
    let live_ticket = dir.join("live-ticket.preserves");
    let peer_admission = dir.join("peer-admission.preserves");
    let missing_import_request = dir.join("missing-import-request.preserves");
    let missing_import_send_receipt = dir.join("missing-import-send.preserves");
    let workflow_bundle = dir.join("workflow-bundle.preserves");
    let bundle_export = dir.join("workflow-bundle-export.preserves");
    let bundle_verify = dir.join("workflow-bundle-verify.preserves");
    let bundle_gate = dir.join("workflow-bundle-gate.preserves");
    let bundle_apply = dir.join("workflow-bundle-apply.preserves");
    let bundle_reconcile = dir.join("workflow-bundle-reconcile.preserves");
    let bundle_ack = dir.join("workflow-bundle-ack.preserves");
    let bundle_ack_export = dir.join("workflow-bundle-ack-export.preserves");
    let bundle_ack_import = dir.join("workflow-bundle-ack-import.preserves");
    let bundle_protocol_gate = dir.join("workflow-bundle-protocol-gate.preserves");
    let bundle_import = dir.join("workflow-bundle-import.preserves");
    let bundle_import_send_receipt = dir.join("bundle-import-send.preserves");
    let ticket_import = dir.join("ticket-import.preserves");
    let grant_import = dir.join("grant-import.preserves");
    let policy_ref = test_ref("live-import-policy")?;
    let resource_ref = test_ref("live-import-resource")?;
    let case = LiveImportCase {
        receiver_root: &receiver_root,
        sender_root: &sender_root,
        bundle_sender_root: &bundle_sender_root,
        bundle_apply_root: &bundle_apply_root,
        authority_grant: &authority_grant,
        live_ticket: &live_ticket,
        peer_admission: &peer_admission,
        missing_import_request: &missing_import_request,
        missing_import_send_receipt: &missing_import_send_receipt,
        workflow_bundle: &workflow_bundle,
        bundle_export: &bundle_export,
        bundle_verify: &bundle_verify,
        bundle_gate: &bundle_gate,
        bundle_apply: &bundle_apply,
        bundle_reconcile: &bundle_reconcile,
        bundle_ack: &bundle_ack,
        bundle_ack_export: &bundle_ack_export,
        bundle_ack_import: &bundle_ack_import,
        bundle_protocol_gate: &bundle_protocol_gate,
        bundle_import: &bundle_import,
        bundle_import_send_receipt: &bundle_import_send_receipt,
        ticket_import: &ticket_import,
        grant_import: &grant_import,
        policy_ref: policy_ref.as_str(),
        resource_ref: resource_ref.as_str(),
    };

    let refs = prepare_live_import_case(&case)?;
    expect_missing_imports(&case, &refs)?;
    export_and_verify_case(&case)?;
    gate_and_apply_case(&case)?;
    review_missing_receiver(&case)?;
    import_case_and_retry(&case, &refs)?;
    import_ticket_and_grant(&case)?;
    Ok(())
}

struct LiveImportCase<'a> {
    receiver_root: &'a std::path::Path,
    sender_root: &'a std::path::Path,
    bundle_sender_root: &'a std::path::Path,
    bundle_apply_root: &'a std::path::Path,
    authority_grant: &'a std::path::Path,
    live_ticket: &'a std::path::Path,
    peer_admission: &'a std::path::Path,
    missing_import_request: &'a std::path::Path,
    missing_import_send_receipt: &'a std::path::Path,
    workflow_bundle: &'a std::path::Path,
    bundle_export: &'a std::path::Path,
    bundle_verify: &'a std::path::Path,
    bundle_gate: &'a std::path::Path,
    bundle_apply: &'a std::path::Path,
    bundle_reconcile: &'a std::path::Path,
    bundle_ack: &'a std::path::Path,
    bundle_ack_export: &'a std::path::Path,
    bundle_ack_import: &'a std::path::Path,
    bundle_protocol_gate: &'a std::path::Path,
    bundle_import: &'a std::path::Path,
    bundle_import_send_receipt: &'a std::path::Path,
    ticket_import: &'a std::path::Path,
    grant_import: &'a std::path::Path,
    policy_ref: &'a str,
    resource_ref: &'a str,
}

struct LiveImportRefs {
    authority_ref: String,
    bootstrap_ref: String,
}

fn init_node(root: &std::path::Path, node_id: &str, label: &str) -> CliResult<()> {
    assert_success(
        &molten_cmd()
            .args(["test", "node", "init", "--state-root"])
            .arg(root)
            .args(["--node-id", node_id])
            .output()?,
        label,
    );
    Ok(())
}

fn prepare_live_import_case(case: &LiveImportCase<'_>) -> CliResult<LiveImportRefs> {
    start_state(case.receiver_root, "node:cli-live-import", "receiver init", "receiver run")?;
    init_node(case.sender_root, "node:cli-live-import-sender", "sender init")?;
    init_node(case.bundle_sender_root, "node:cli-live-bundle-sender", "bundle sender init")?;
    init_node(case.bundle_apply_root, "node:cli-live-bundle-apply", "bundle apply init")?;
    let authority_ref = grant_fixture(GrantArgs {
        root: case.receiver_root,
        grant: case.authority_grant,
        peer: "peer:cli-live-import",
        node: "node:cli-live-import",
        policy_ref: case.policy_ref,
        label: "receiver authority grant",
    })?;
    ticket_export(case.receiver_root, case.live_ticket, case.policy_ref, "receiver live ticket")?;
    let bootstrap_ref = peer_admit(AdmitArgs {
        root: case.receiver_root,
        receipt: case.peer_admission,
        peer: "peer:cli-live-import",
        policy_ref: case.policy_ref,
        ticket: case.live_ticket,
        label: "receiver peer admit",
    })?;
    write_status_request(
        case.missing_import_request,
        &authority_ref,
        case.policy_ref,
        case.resource_ref,
        "missing import request",
    )?;
    Ok(LiveImportRefs {
        authority_ref,
        bootstrap_ref,
    })
}

fn expect_missing_imports(case: &LiveImportCase<'_>, refs: &LiveImportRefs) -> CliResult<()> {
    let output = molten_cmd()
        .args(["test", "node", "control-ingress-live-send", "--state-root"])
        .arg(case.sender_root)
        .arg(case.missing_import_request)
        .arg(case.live_ticket)
        .args([
            "--from-peer",
            "peer:cli-live-import",
            "--expected-topic",
            "wrong-topic",
            "--peer-bootstrap",
        ])
        .arg(&refs.bootstrap_ref)
        .args(["--authority"])
        .arg(&refs.authority_ref)
        .args(["--policy"])
        .arg(case.policy_ref)
        .args(["--resource"])
        .arg(case.resource_ref)
        .args(["--receipt-out"])
        .arg(case.missing_import_send_receipt)
        .output()?;
    assert_success(&output, "live send missing imports deny receipt");
    let text = molten::preserves_rail::to_text(&read_preserves(case.missing_import_send_receipt)?)?;
    assert!(text.contains("live-ticket-import"));
    assert!(text.contains("authority-grant-import"));
    assert!(text.contains("ticket topic node-control does not match expected wrong-topic"));
    assert!(text.contains("receiver-ticket-expected"));
    assert!(text.contains("sender-state-root-evidence"));
    Ok(())
}

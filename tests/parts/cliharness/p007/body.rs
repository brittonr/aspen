
fn import_ticket_and_grant(case: &LiveImportCase<'_>) -> CliResult<()> {
    let ticket = molten_cmd()
        .args(["test", "node", "live-ticket-import", "--state-root"])
        .arg(case.sender_root)
        .arg(case.live_ticket)
        .args(["--peer-admission"])
        .arg(case.peer_admission)
        .args([
            "--expected-node",
            "node:cli-live-import",
            "--expected-topic",
            "node-control",
            "--expected-peer",
            "peer:cli-live-import",
            "--receipt-out",
        ])
        .arg(case.ticket_import)
        .output()?;
    assert_success(&ticket, "sender live ticket import");
    assert!(stdout(&ticket).contains("decision=pass"));
    assert_eq!(
        molten::ledger::artifact_kind(&read_preserves(case.ticket_import)?),
        "node-control-live-ticket-import-receipt"
    );

    let grant = molten_cmd()
        .args(["test", "node", "authority-grant-import", "--state-root"])
        .arg(case.sender_root)
        .arg(case.authority_grant)
        .args([
            "--peer",
            "peer:cli-live-import",
            "--node",
            "node:cli-live-import",
            "--operation",
            "status",
            "--receipt-out",
        ])
        .arg(case.grant_import)
        .output()?;
    assert_success(&grant, "sender authority grant import");
    assert!(stdout(&grant).contains("decision=pass"));
    assert_eq!(
        molten::ledger::artifact_kind(&read_preserves(case.grant_import)?),
        "node-control-authority-grant-import-receipt"
    );
    Ok(())
}

#[test]
fn cli_node_live_send_denies_offline_ticket_without_addresses() -> CliResult<()> {
    let dir = temp_dir("cli-node-live-send")?;
    let state_root = dir.join("node-state");
    let request = dir.join("status.preserves");
    let ticket = dir.join("live-ticket.preserves");
    let send_receipt = dir.join("send.preserves");
    let operation_mismatch_receipt = dir.join("operation-mismatch.preserves");
    let transport_receipt = dir.join("transport.preserves");
    let authority_grant = dir.join("authority-grant.preserves");
    let peer_admission = dir.join("peer-admission.preserves");
    let service_receipt = dir.join("service.preserves");
    let workflow_receipt = dir.join("workflow.preserves");
    let policy_ref = test_ref("live-send-policy")?;
    let resource_ref = test_ref("live-send-resource")?;
    let wrong_operation_ref = test_ref("wrong-live-send-operation")?;

    start_state(&state_root, "node:cli-live-send", "node live send init", "node live send run")?;
    let authority_ref = grant_fixture(GrantArgs {
        root: &state_root,
        grant: &authority_grant,
        peer: "peer:cli-live-send",
        node: "node:cli-live-send",
        policy_ref: &policy_ref,
        label: "node live send authority grant",
    })?;
    write_status_request(&request, &authority_ref, &policy_ref, &resource_ref, "node live send request")?;
    ticket_export(&state_root, &ticket, &policy_ref, "node live send ticket")?;
    let bootstrap_ref = peer_admit(AdmitArgs {
        root: &state_root,
        receipt: &peer_admission,
        peer: "peer:cli-live-send",
        policy_ref: &policy_ref,
        ticket: &ticket,
        label: "node live send peer admit",
    })?;
    let send_args = SendArgs {
        root: &state_root,
        request: &request,
        ticket: &ticket,
        peer: "peer:cli-live-send",
        bootstrap_ref: &bootstrap_ref,
        authority_ref: &authority_ref,
        policy_ref: &policy_ref,
        resource_ref: &resource_ref,
    };
    expect_no_address(&send_args, &transport_receipt, &send_receipt)?;
    expect_mismatch(&send_args, &wrong_operation_ref, &operation_mismatch_receipt)?;
    assert_success(
        &molten_cmd()
            .args(["test", "node", "serve", "--state-root"])
            .arg(&state_root)
            .args(["--max-ticks", "1", "--receipt-out"])
            .arg(&service_receipt)
            .output()?,
        "node live send service receipt",
    );
    expect_missing_receive(BundleArgs {
        root: &state_root,
        ticket: &ticket,
        peer_admission: &peer_admission,
        authority_grant: &authority_grant,
        send_receipt: &send_receipt,
        service_receipt: &service_receipt,
        receipt: &workflow_receipt,
    })?;
    Ok(())
}

#[test]
fn cli_node_serve_live_iroh_empty_listener_receipt() -> CliResult<()> {
    let dir = temp_dir("cli-node-serve-live")?;
    let state_root = dir.join("node-state");
    let listener_receipt = dir.join("listener.preserves");
    let service_receipt = dir.join("service.preserves");
    let live_ticket = dir.join("live-ticket.preserves");
    assert_success(
        &molten_cmd()
            .args(["test", "node", "init", "--state-root"])
            .arg(&state_root)
            .args(["--node-id", "node:cli-serve-live"])
            .output()?,
        "node serve live init",
    );
    assert_success(
        &molten_cmd().args(["test", "node", "run", "--state-root"]).arg(&state_root).output()?,
        "node serve live run",
    );
    let served = molten_cmd()
        .args(["test", "node", "serve", "--state-root"])
        .arg(&state_root)
        .args(["--live-iroh", "--live-max-events", "0", "--service-receipt-out"])
        .arg(&service_receipt)
        .args(["--live-ticket-out"])
        .arg(&live_ticket)
        .args(["--receipt-out"])
        .arg(&listener_receipt)
        .output()?;
    assert_success(&served, "node serve live listener");
    assert!(stdout(&served).contains("node serve live-iroh"));
    assert_eq!(
        molten::ledger::artifact_kind(&read_preserves(&listener_receipt)?),
        "node-control-live-listener-receipt"
    );
    assert_eq!(
        molten::ledger::artifact_kind(&read_preserves(&service_receipt)?),
        "node-control-service-run-receipt"
    );
    assert_eq!(molten::ledger::artifact_kind(&read_preserves(&live_ticket)?), "node-control-live-ticket");
    Ok(())
}

#[test]
fn cli_node_supervisor_policy_fixture_and_serve_receipt() -> CliResult<()> {
    let dir = temp_dir("cli-node-supervisor-policy")?;
    let state_root = dir.join("node-state");
    let policy = dir.join("supervisor-policy.preserves");
    let service_receipt = dir.join("service.preserves");
    let policy_ref = test_ref("supervisor-policy")?;

    assert_success(
        &molten_cmd()
            .args(["test", "node", "init", "--state-root"])
            .arg(&state_root)
            .args(["--node-id", "node:cli-supervisor"])
            .output()?,
        "node supervisor init",
    );
    assert_success(
        &molten_cmd().args(["test", "node", "run", "--state-root"]).arg(&state_root).output()?,
        "node supervisor run",
    );
    assert_success(
        &molten_cmd()
            .args(["test", "node", "supervisor-policy-fixture", "--state-root"])
            .arg(&state_root)
            .args([
                "--max-restarts",
                "1",
                "--restart-window-ticks",
                "2",
                "--heartbeat-timeout-ticks",
                "2",
                "--shutdown-drain-ticks",
                "2",
                "--allow-stale-lock-recovery",
                "--policy",
            ])
            .arg(&policy_ref)
            .args(["--out"])
            .arg(&policy)
            .output()?,
        "node supervisor policy fixture",
    );
    assert_eq!(molten::ledger::artifact_kind(&read_preserves(&policy)?), "node-control-supervisor-policy");
    let served = molten_cmd()
        .args(["test", "node", "serve", "--state-root"])
        .arg(&state_root)
        .args(["--max-ticks", "1", "--supervisor-policy"])
        .arg(&policy)
        .args(["--receipt-out"])
        .arg(&service_receipt)
        .output()?;
    assert_success(&served, "node supervisor serve");
    let service_value = read_preserves(&service_receipt)?;
    assert_eq!(molten::ledger::artifact_kind(&service_value), "node-control-service-run-receipt");
    let service_text = molten::preserves_rail::to_text(&service_value)?;
    assert!(service_text.contains("supervisor-policy"));
    assert!(service_text.contains("supervisor-receipts"));
    Ok(())
}

#[test]
fn cli_node_serve_drains_shutdown_request() -> CliResult<()> {
    let dir = temp_dir("cli-node-serve")?;
    let state_root = dir.join("node-state");
    let request = dir.join("shutdown.preserves");
    let queue_receipt = dir.join("queue.preserves");
    let service_receipt = dir.join("service.preserves");
    let authority_ref = test_ref("serve-authority")?;
    let policy_ref = test_ref("serve-policy")?;
    let resource_ref = test_ref("serve-resource")?;

    assert_success(
        &molten_cmd()
            .args(["test", "node", "init", "--state-root"])
            .arg(&state_root)
            .args(["--node-id", "node:cli-serve"])
            .output()?,
        "node serve init",
    );
    assert_success(
        &molten_cmd().args(["test", "node", "run", "--state-root"]).arg(&state_root).output()?,
        "node serve run",
    );
    assert_success(
        &molten_cmd()
            .args([
                "test",
                "node",
                "control-request",
                "--operation",
                "shutdown",
                "--authority",
            ])
            .arg(&authority_ref)
            .args(["--policy"])
            .arg(&policy_ref)
            .args(["--resource"])
            .arg(&resource_ref)
            .args(["--out"])
            .arg(&request)
            .output()?,
        "node serve shutdown request",
    );
    assert_success(
        &molten_cmd()
            .args(["test", "node", "control-submit", "--state-root"])
            .arg(&state_root)
            .arg(&request)
            .args(["--receipt-out"])
            .arg(&queue_receipt)
            .output()?,
        "node serve submit",
    );
    let served = molten_cmd()
        .args(["test", "node", "serve", "--state-root"])
        .arg(&state_root)
        .args(["--max-ticks", "2", "--max-requests-per-tick", "1", "--receipt-out"])
        .arg(&service_receipt)
        .output()?;
    assert_success(&served, "node serve");
    assert!(stdout(&served).contains("node serve decision=pass"));
    assert!(stdout(&served).contains("stopped=yes"));
    assert_eq!(
        molten::ledger::artifact_kind(&read_preserves(&service_receipt)?),
        "node-control-service-run-receipt"
    );
    Ok(())
}

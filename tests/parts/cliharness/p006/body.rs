
fn export_and_verify_case(case: &LiveImportCase<'_>) -> CliResult<()> {
    let exported = molten_cmd()
        .args(["test", "node", "live-workflow-bundle-export", "--ticket"])
        .arg(case.live_ticket)
        .args(["--peer-admission"])
        .arg(case.peer_admission)
        .args(["--authority-grant"])
        .arg(case.authority_grant)
        .args(["--receipt"])
        .arg(case.missing_import_send_receipt)
        .args(["--out"])
        .arg(case.workflow_bundle)
        .args(["--receipt-out"])
        .arg(case.bundle_export)
        .output()?;
    assert_success(&exported, "live workflow bundle export");
    assert!(stdout(&exported).contains("decision=pass"));
    assert_eq!(
        molten::ledger::artifact_kind(&read_preserves(case.workflow_bundle)?),
        "node-control-live-workflow-bundle"
    );
    assert_eq!(
        molten::ledger::artifact_kind(&read_preserves(case.bundle_export)?),
        "node-control-live-workflow-bundle-export-receipt"
    );

    let verified = molten_cmd()
        .args(["test", "node", "live-workflow-bundle-verify"])
        .arg(case.workflow_bundle)
        .args([
            "--expected-node",
            "node:cli-live-import",
            "--expected-topic",
            "node-control",
            "--expected-peer",
            "peer:cli-live-import",
            "--operation",
            "status",
            "--receipt-out",
        ])
        .arg(case.bundle_verify)
        .output()?;
    assert_success(&verified, "live workflow bundle verify");
    assert!(stdout(&verified).contains("decision=pass"));
    assert_eq!(
        molten::ledger::artifact_kind(&read_preserves(case.bundle_verify)?),
        "node-control-live-workflow-bundle-verify-receipt"
    );
    let text = molten::preserves_rail::to_text(&read_preserves(case.bundle_verify)?)?;
    assert!(text.contains("verify-receipt-is-not-authority"));
    Ok(())
}

fn gate_and_apply_case(case: &LiveImportCase<'_>) -> CliResult<()> {
    let gated = molten_cmd()
        .args(["test", "node", "live-workflow-bundle-gate"])
        .arg(case.workflow_bundle)
        .args(["--verify-receipt"])
        .arg(case.bundle_verify)
        .args([
            "--require-verify-receipt",
            "--expected-node",
            "node:cli-live-import",
            "--expected-topic",
            "node-control",
            "--expected-peer",
            "peer:cli-live-import",
            "--operation",
            "status",
            "--receipt-out",
        ])
        .arg(case.bundle_gate)
        .output()?;
    assert_success(&gated, "live workflow bundle gate");
    assert!(stdout(&gated).contains("decision=pass"));
    assert!(stdout(&gated).contains("next-step=import-bundle"));
    assert_eq!(
        molten::ledger::artifact_kind(&read_preserves(case.bundle_gate)?),
        "node-control-live-workflow-bundle-gate-receipt"
    );
    let gate_text = molten::preserves_rail::to_text(&read_preserves(case.bundle_gate)?)?;
    assert!(gate_text.contains("gate-receipt-is-not-authority"));

    let applied = molten_cmd()
        .args(["test", "node", "live-workflow-bundle-apply", "--state-root"])
        .arg(case.bundle_apply_root)
        .arg(case.workflow_bundle)
        .args(["--gate-receipt"])
        .arg(case.bundle_gate)
        .args([
            "--require-gate-receipt",
            "--expected-node",
            "node:cli-live-import",
            "--expected-topic",
            "node-control",
            "--expected-peer",
            "peer:cli-live-import",
            "--operation",
            "status",
            "--receipt-out",
        ])
        .arg(case.bundle_apply)
        .output()?;
    assert_success(&applied, "live workflow bundle apply");
    assert!(stdout(&applied).contains("decision=pass"));
    assert!(stdout(&applied).contains("next-step=dry-run-or-send-request"));
    assert_eq!(
        molten::ledger::artifact_kind(&read_preserves(case.bundle_apply)?),
        "node-control-live-workflow-bundle-apply-receipt"
    );
    let apply_text = molten::preserves_rail::to_text(&read_preserves(case.bundle_apply)?)?;
    assert!(apply_text.contains("apply-receipt-is-not-authority"));
    Ok(())
}

fn review_missing_receiver(case: &LiveImportCase<'_>) -> CliResult<()> {
    reconcile_missing_receiver(case)?;
    export_missing_ack(case)?;
    import_missing_ack(case)?;
    gate_missing_protocol(case)?;
    Ok(())
}

fn reconcile_missing_receiver(case: &LiveImportCase<'_>) -> CliResult<()> {
    let output = molten_cmd()
        .args(["test", "node", "live-workflow-bundle-reconcile"])
        .arg(case.bundle_apply)
        .args(["--receipt-out"])
        .arg(case.bundle_reconcile)
        .output()?;
    assert_success(&output, "live workflow bundle reconcile missing receiver");
    assert!(stdout(&output).contains("decision=deny"));
    assert!(stdout(&output).contains("next-step=wait-or-import-receiver-ingress"));
    assert_eq!(
        molten::ledger::artifact_kind(&read_preserves(case.bundle_reconcile)?),
        "node-control-live-workflow-bundle-reconcile-receipt"
    );
    let text = molten::preserves_rail::to_text(&read_preserves(case.bundle_reconcile)?)?;
    assert!(text.contains("reconcile-receipt-is-not-authority"));
    Ok(())
}

fn export_missing_ack(case: &LiveImportCase<'_>) -> CliResult<()> {
    let output = molten_cmd()
        .args(["test", "node", "live-workflow-bundle-ack-export"])
        .arg(case.bundle_apply)
        .args(["--reconcile-receipt"])
        .arg(case.bundle_reconcile)
        .args(["--out"])
        .arg(case.bundle_ack)
        .args(["--receipt-out"])
        .arg(case.bundle_ack_export)
        .output()?;
    assert_success(&output, "live workflow bundle ack export missing receiver");
    assert!(stdout(&output).contains("decision=deny"));
    assert!(stdout(&output).contains("next-step=collect-receiver-evidence"));
    assert_eq!(
        molten::ledger::artifact_kind(&read_preserves(case.bundle_ack)?),
        "node-control-live-workflow-bundle-ack"
    );
    assert_eq!(
        molten::ledger::artifact_kind(&read_preserves(case.bundle_ack_export)?),
        "node-control-live-workflow-bundle-ack-export-receipt"
    );
    let text = molten::preserves_rail::to_text(&read_preserves(case.bundle_ack)?)?;
    assert!(text.contains("ack-bundle-is-not-authority"));
    Ok(())
}

fn import_missing_ack(case: &LiveImportCase<'_>) -> CliResult<()> {
    let output = molten_cmd()
        .args(["test", "node", "live-workflow-bundle-ack-import", "--state-root"])
        .arg(case.bundle_sender_root)
        .arg(case.bundle_ack)
        .args(["--receipt-out"])
        .arg(case.bundle_ack_import)
        .output()?;
    assert_success(&output, "live workflow bundle ack import missing receiver");
    assert!(stdout(&output).contains("decision=deny"));
    assert_eq!(
        molten::ledger::artifact_kind(&read_preserves(case.bundle_ack_import)?),
        "node-control-live-workflow-bundle-ack-import-receipt"
    );
    let text = molten::preserves_rail::to_text(&read_preserves(case.bundle_ack_import)?)?;
    assert!(text.contains("ack-import-is-not-authority"));
    Ok(())
}

fn gate_missing_protocol(case: &LiveImportCase<'_>) -> CliResult<()> {
    let output = molten_cmd()
        .args(["test", "node", "live-workflow-bundle-protocol-gate"])
        .arg(case.workflow_bundle)
        .args(["--gate-receipt"])
        .arg(case.bundle_gate)
        .args(["--apply-receipt"])
        .arg(case.bundle_apply)
        .args(["--reconcile-receipt"])
        .arg(case.bundle_reconcile)
        .args(["--ack"])
        .arg(case.bundle_ack)
        .args(["--receipt-out"])
        .arg(case.bundle_protocol_gate)
        .output()?;
    assert_success(&output, "live workflow bundle protocol gate missing receiver");
    assert!(stdout(&output).contains("decision=deny"));
    assert_eq!(
        molten::ledger::artifact_kind(&read_preserves(case.bundle_protocol_gate)?),
        "protocol-session-gate-receipt"
    );
    let text = molten::preserves_rail::to_text(&read_preserves(case.bundle_protocol_gate)?)?;
    assert!(text.contains("ack receiver decision deny"));
    assert!(text.contains("protocol-session-gate-is-not-authority"));
    Ok(())
}

fn import_case_and_retry(case: &LiveImportCase<'_>, refs: &LiveImportRefs) -> CliResult<()> {
    let imported = molten_cmd()
        .args(["test", "node", "live-workflow-bundle-import", "--state-root"])
        .arg(case.bundle_sender_root)
        .arg(case.workflow_bundle)
        .args([
            "--expected-node",
            "node:cli-live-import",
            "--expected-topic",
            "node-control",
            "--expected-peer",
            "peer:cli-live-import",
            "--operation",
            "status",
            "--receipt-out",
        ])
        .arg(case.bundle_import)
        .output()?;
    assert_success(&imported, "live workflow bundle import");
    assert!(stdout(&imported).contains("decision=pass"));
    assert_eq!(
        molten::ledger::artifact_kind(&read_preserves(case.bundle_import)?),
        "node-control-live-workflow-bundle-import-receipt"
    );

    let sent = molten_cmd()
        .args(["test", "node", "control-ingress-live-send", "--state-root"])
        .arg(case.bundle_sender_root)
        .arg(case.missing_import_request)
        .arg(case.live_ticket)
        .args([
            "--from-peer",
            "peer:cli-live-import",
            "--expected-topic",
            "node-control",
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
        .arg(case.bundle_import_send_receipt)
        .output()?;
    assert_success(&sent, "live send after workflow bundle import");
    let text = molten::preserves_rail::to_text(&read_preserves(case.bundle_import_send_receipt)?)?;
    assert!(text.contains("ticket has no endpoint addresses"));
    assert!(!text.contains("authority-grant-import"));
    assert!(!text.contains("peer admission unavailable in sender state root"));
    assert!(!text.contains("authority grant unavailable in sender state root"));
    Ok(())
}

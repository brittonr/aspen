pub(crate) fn reconcile(input: super::super::command::live::Reconcile) -> molten::error::Result<()> {
    let super::super::command::live::Reconcile {
        apply_receipt,
        send_receipt,
        ingress_receipt,
        queue_receipt,
        control_receipt,
        expected_envelope,
        expected_operation,
        expected_request,
        receipt_out,
    } = input;
    let apply_receipt_value = super::super::core::read_preserves_file(&apply_receipt)?;
    let send_receipt_value = read_optional(send_receipt.as_ref())?;
    let ingress_receipt_value = read_optional(ingress_receipt.as_ref())?;
    let queue_receipt_value = read_optional(queue_receipt.as_ref())?;
    let control_receipt_value = read_optional(control_receipt.as_ref())?;
    let reconciled = molten::node_daemon::reconcile_node_control_live_workflow_bundle(
        &molten::node_daemon::ControlLiveWorkflowBundleReconcileInput {
            apply_receipt_value: &apply_receipt_value,
            send_receipt_value: send_receipt_value.as_ref(),
            ingress_receipt_value: ingress_receipt_value.as_ref(),
            queue_receipt_value: queue_receipt_value.as_ref(),
            control_receipt_value: control_receipt_value.as_ref(),
            expected_envelope_ref: expected_envelope.as_deref(),
            expected_operation_ref: expected_operation.as_deref(),
            expected_request_ref: expected_request.as_deref(),
        },
    )?;
    super::super::core::emit_named_receipt(
        receipt_out.as_ref(),
        "node control live workflow bundle reconcile receipt",
        &reconciled.receipt_value,
    )?;
    println!(
        "node live workflow bundle reconcile decision={} bundle={} apply={} ingress={} queue={} control={} diagnostics={}",
        reconciled.decision,
        reconciled.bundle_ref,
        reconciled.apply_receipt_ref,
        reconciled.ingress_receipt_ref.as_deref().unwrap_or("none"),
        reconciled.queue_receipt_ref.as_deref().unwrap_or("none"),
        reconciled.control_receipt_ref.as_deref().unwrap_or("none"),
        reconciled.diagnostics.len()
    );
    print_reconcile_next_step(&reconciled);
    Ok(())
}

pub(crate) fn export(input: super::super::command::live::AckExport) -> molten::error::Result<()> {
    let super::super::command::live::AckExport {
        apply_receipt,
        send_receipt,
        ingress_receipt,
        queue_receipt,
        control_receipt,
        reconcile_receipt,
        out,
        receipt_out,
    } = input;
    let apply_receipt_value = super::super::core::read_preserves_file(&apply_receipt)?;
    let send_receipt_value = read_optional(send_receipt.as_ref())?;
    let ingress_receipt_value = read_optional(ingress_receipt.as_ref())?;
    let queue_receipt_value = read_optional(queue_receipt.as_ref())?;
    let control_receipt_value = read_optional(control_receipt.as_ref())?;
    let reconcile_receipt_value = super::super::core::read_preserves_file(&reconcile_receipt)?;
    let exported = molten::node_daemon::export_node_control_live_workflow_bundle_ack(
        &molten::node_daemon::ControlLiveWorkflowBundleAckExportInput {
            apply_receipt_value: &apply_receipt_value,
            send_receipt_value: send_receipt_value.as_ref(),
            ingress_receipt_value: ingress_receipt_value.as_ref(),
            queue_receipt_value: queue_receipt_value.as_ref(),
            control_receipt_value: control_receipt_value.as_ref(),
            reconcile_receipt_value: &reconcile_receipt_value,
        },
    )?;
    super::super::core::write_file(&out, &molten::preserves_rail::to_text(&exported.ack.ack_value)?)?;
    super::super::core::emit_named_receipt(
        receipt_out.as_ref(),
        "node control live workflow bundle ack export receipt",
        &exported.receipt_value,
    )?;
    println!(
        "node live workflow bundle ack export decision={} ack={} bundle={} receiver_decision={} diagnostics={}",
        exported.decision,
        exported.ack.ack_ref,
        exported.ack.bundle_ref,
        exported.receiver_decision,
        exported.diagnostics.len()
    );
    print_export_next_step(&exported);
    Ok(())
}

pub(crate) fn import(input: super::super::command::live::AckImport) -> molten::error::Result<()> {
    let super::super::command::live::AckImport {
        state_root,
        ack,
        expected_bundle,
        expected_envelope,
        expected_operation,
        expected_request,
        receipt_out,
    } = input;
    let ack_value = super::super::core::read_preserves_file(&ack)?;
    let imported = molten::node_daemon::import_node_control_live_workflow_bundle_ack(
        &molten::node_daemon::ControlLiveWorkflowBundleAckImportInput {
            state_root: &state_root,
            ack_value: &ack_value,
            expected_bundle_ref: expected_bundle.as_deref(),
            expected_envelope_ref: expected_envelope.as_deref(),
            expected_operation_ref: expected_operation.as_deref(),
            expected_request_ref: expected_request.as_deref(),
        },
    )?;
    super::super::core::emit_named_receipt(
        receipt_out.as_ref(),
        "node control live workflow bundle ack import receipt",
        &imported.receipt_value,
    )?;
    println!(
        "node live workflow bundle ack import decision={} ack={} bundle={} imported={} receiver_decision={} diagnostics={}",
        imported.decision,
        imported.ack_ref,
        imported.bundle_ref,
        imported.imported_refs.len(),
        imported.receiver_decision,
        imported.diagnostics.len()
    );
    print_import_next_step(&imported);
    Ok(())
}

pub(crate) fn protocol_gate(input: super::super::command::live::ProtocolGate) -> molten::error::Result<()> {
    let super::super::command::live::ProtocolGate {
        bundle,
        gate_receipt,
        apply_receipt,
        reconcile_receipt,
        ack,
        expected_envelope,
        expected_operation,
        expected_request,
        receipt_out,
    } = input;
    let bundle_value = super::super::core::read_preserves_file(&bundle)?;
    let gate_receipt_value = super::super::core::read_preserves_file(&gate_receipt)?;
    let apply_receipt_value = super::super::core::read_preserves_file(&apply_receipt)?;
    let reconcile_receipt_value = super::super::core::read_preserves_file(&reconcile_receipt)?;
    let ack_value = super::super::core::read_preserves_file(&ack)?;
    let gated = molten::node_daemon::gate_node_control_live_workflow_protocol(
        &molten::node_daemon::ControlLiveWorkflowProtocolGateInput {
            bundle_value: &bundle_value,
            gate_receipt_value: &gate_receipt_value,
            apply_receipt_value: &apply_receipt_value,
            reconcile_receipt_value: &reconcile_receipt_value,
            ack_value: &ack_value,
            expected_envelope_ref: expected_envelope.as_deref(),
            expected_operation_ref: expected_operation.as_deref(),
            expected_request_ref: expected_request.as_deref(),
        },
    )?;
    super::super::core::emit_named_receipt(
        receipt_out.as_ref(),
        "node control live workflow protocol gate receipt",
        &gated.receipt_value,
    )?;
    println!(
        "node live workflow protocol gate decision={} receipt={} protocol={} session={} operations={} messages={} diagnostics={}",
        gated.decision,
        gated.receipt_ref,
        gated.protocol_ref,
        gated.session_id,
        gated.operation_count,
        gated.message_count,
        gated.diagnostics.len()
    );
    print_protocol_gate_next_step(&gated);
    Ok(())
}

fn read_optional(path: Option<&std::path::PathBuf>) -> molten::error::Result<Option<preserves::IOValue>> {
    path.map(|path| super::super::core::read_preserves_file(path)).transpose()
}

fn print_export_next_step(exported: &molten::node_daemon::ControlLiveWorkflowBundleAckExport) {
    if exported.decision != "pass" {
        println!(
            "next-step=collect-receiver-evidence command=\"molten node live-workflow-bundle-reconcile ... --ingress-receipt <receipt> --queue-receipt <receipt>\""
        );
        return;
    }
    println!(
        "next-step=import-ack command=\"molten node live-workflow-bundle-ack-import --state-root <sender> <ack>\""
    );
}

fn print_import_next_step(imported: &molten::node_daemon::ControlLiveWorkflowBundleAckImport) {
    if imported.decision != "pass" {
        println!("next-step=inspect-ack-diagnostics command=\"molten node show <ack-import-receipt>\"");
        return;
    }
    if imported.receiver_decision == "pass" {
        println!("next-step=inspect-receiver-control command=\"molten node show <control-receipt>\"");
    } else {
        println!("next-step=inspect-receiver-denial command=\"molten node show <reconcile-receipt>\"");
    }
}

fn print_protocol_gate_next_step(gated: &molten::node_daemon::ControlLiveWorkflowProtocolGate) {
    if gated.decision == "pass" {
        println!("next-step=archive-workflow-protocol command=\"molten node show <protocol-gate-receipt>\"");
    } else {
        println!(
            "next-step=inspect-workflow-protocol-diagnostics command=\"molten node show <protocol-gate-receipt>\""
        );
    }
}

fn print_reconcile_next_step(reconciled: &molten::node_daemon::ControlLiveWorkflowBundleReconcile) {
    if reconciled.decision == "pass" {
        if reconciled.control_receipt_ref.is_some() {
            println!("next-step=inspect-control-receipt command=\"molten node show <control-receipt>\"");
        } else {
            println!("next-step=run-receiver-control-loop command=\"molten node run-loop --state-root <receiver>\"");
        }
        return;
    }
    let has_missing_ingress = reconciled
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.contains("requires receiver ingress receipt"));
    if has_missing_ingress {
        println!(
            "next-step=wait-or-import-receiver-ingress command=\"molten node live-workflow-bundle-reconcile ... --ingress-receipt <receipt>\""
        );
        return;
    }
    let has_control_denial =
        reconciled.diagnostics.iter().any(|diagnostic| diagnostic.contains("receiver control receipt"));
    if has_control_denial {
        println!("next-step=inspect-receiver-denial command=\"molten node show <control-receipt>\"");
        return;
    }
    println!("next-step=inspect-reconcile-diagnostics command=\"molten node show <reconcile-receipt>\"");
}

pub(crate) fn run(input: super::super::command::live::Bundle) -> molten::error::Result<()> {
    let super::super::command::live::Bundle {
        state_root,
        ticket,
        peer_admission,
        authority_grant,
        send_receipt,
        receive_receipts,
        listener_receipt,
        service_receipt,
        receipt_out,
    } = input;
    let ticket_value = super::super::core::read_preserves_file(&ticket)?;
    let peer_admission_value = super::super::core::read_preserves_file(&peer_admission)?;
    let authority_grant_value = super::super::core::read_preserves_file(&authority_grant)?;
    let send_receipt_value = super::super::core::read_preserves_file(&send_receipt)?;
    let receive_values = receive_receipts
        .iter()
        .map(|path| super::super::core::read_preserves_file(path))
        .collect::<molten::error::Result<Vec<_>>>()?;
    let receive_value_refs = receive_values.iter().collect::<Vec<_>>();
    let listener_value =
        listener_receipt.as_ref().map(|path| super::super::core::read_preserves_file(path)).transpose()?;
    let service_receipt_value = super::super::core::read_preserves_file(&service_receipt)?;
    let workflow =
        molten::node_daemon::control_live_workflow_receipt(&molten::node_daemon::ControlLiveWorkflowInput {
            state_root: state_root.as_deref(),
            receiver_ticket_value: &ticket_value,
            peer_admission_value: &peer_admission_value,
            authority_grant_value: &authority_grant_value,
            send_receipt_value: &send_receipt_value,
            receive_receipt_values: &receive_value_refs,
            listener_receipt_value: listener_value.as_ref(),
            service_receipt_value: &service_receipt_value,
        })?;
    super::super::core::emit_named_receipt(
        receipt_out.as_ref(),
        "node control live workflow receipt",
        &workflow.receipt_value,
    )?;
    println!(
        "node live workflow bundle decision={} receipt={} diagnostics={}",
        workflow.decision,
        workflow.receipt_ref,
        workflow.diagnostics.len()
    );
    Ok(())
}

pub(crate) fn export(input: super::super::command::live::Export) -> molten::error::Result<()> {
    let super::super::command::live::Export {
        ticket,
        peer_admission,
        authority_grant,
        receipt_values,
        out,
        receipt_out,
    } = input;
    let ticket_value = super::super::core::read_preserves_file(&ticket)?;
    let peer_admission_value = super::super::core::read_preserves_file(&peer_admission)?;
    let authority_grant_value = super::super::core::read_preserves_file(&authority_grant)?;
    let receipt_values = receipt_values
        .iter()
        .map(|path| super::super::core::read_preserves_file(path))
        .collect::<molten::error::Result<Vec<_>>>()?;
    let receipt_value_refs = receipt_values.iter().collect::<Vec<_>>();
    let exported = molten::node_daemon::export_control_live_workflow_bundle(
        &molten::node_daemon::ControlLiveWorkflowBundleExportInput {
            receiver_ticket_value: &ticket_value,
            peer_admission_value: &peer_admission_value,
            authority_grant_value: &authority_grant_value,
            receipt_values: &receipt_value_refs,
        },
    )?;
    super::super::core::write_file(&out, &molten::preserves_rail::to_text(&exported.bundle.bundle_value)?)?;
    super::super::core::emit_named_receipt(
        receipt_out.as_ref(),
        "node control live workflow bundle export receipt",
        &exported.receipt_value,
    )?;
    println!(
        "node live workflow bundle export decision={} bundle={} ticket={} admission={} grant={} diagnostics={}",
        exported.decision,
        exported.bundle.bundle_ref,
        exported.bundle.ticket_ref,
        exported.bundle.peer_admission_ref,
        exported.bundle.authority_grant_ref,
        exported.diagnostics.len()
    );
    Ok(())
}

pub(crate) fn verify(input: super::super::command::live::Verify) -> molten::error::Result<()> {
    let super::super::command::live::Verify {
        bundle,
        expected_node,
        expected_topic,
        expected_endpoint,
        expected_peer,
        operations,
        target_scope,
        resource_scope,
        as_of_sequence,
        as_of_epoch,
        receipt_out,
    } = input;
    let bundle_value = super::super::core::read_preserves_file(&bundle)?;
    let verified = molten::node_daemon::verify_control_live_workflow_bundle(
        &molten::node_daemon::ControlLiveWorkflowBundleVerifyInput {
            bundle_value: &bundle_value,
            expected_node: expected_node.as_deref(),
            expected_topic: expected_topic.as_deref(),
            expected_endpoint: expected_endpoint.as_deref(),
            expected_peer: expected_peer.as_deref(),
            expected_operations: &operations,
            expected_target_scope: target_scope.as_deref(),
            expected_resource_scope: resource_scope.as_deref(),
            as_of_sequence,
            as_of_epoch,
        },
    )?;
    super::super::core::emit_named_receipt(
        receipt_out.as_ref(),
        "node control live workflow bundle verify receipt",
        &verified.receipt_value,
    )?;
    println!(
        "node live workflow bundle verify decision={} bundle={} ticket={} admission={} grant={} diagnostics={}",
        verified.decision,
        verified.bundle_ref,
        verified.ticket_ref.as_deref().unwrap_or("none"),
        verified.peer_admission_ref.as_deref().unwrap_or("none"),
        verified.authority_grant_ref.as_deref().unwrap_or("none"),
        verified.diagnostics.len()
    );
    Ok(())
}

pub(crate) fn import(input: super::super::command::live::Import) -> molten::error::Result<()> {
    let super::super::command::live::Import {
        state_root,
        bundle,
        expected_node,
        expected_topic,
        expected_endpoint,
        expected_peer,
        operations,
        target_scope,
        resource_scope,
        as_of_sequence,
        as_of_epoch,
        receipt_out,
    } = input;
    let bundle_value = super::super::core::read_preserves_file(&bundle)?;
    let imported = molten::node_daemon::import_control_live_workflow_bundle(
        &molten::node_daemon::ControlLiveWorkflowBundleImportInput {
            state_root: &state_root,
            bundle_value: &bundle_value,
            expected_node: expected_node.as_deref(),
            expected_topic: expected_topic.as_deref(),
            expected_endpoint: expected_endpoint.as_deref(),
            expected_peer: expected_peer.as_deref(),
            expected_operations: &operations,
            expected_target_scope: target_scope.as_deref(),
            expected_resource_scope: resource_scope.as_deref(),
            as_of_sequence,
            as_of_epoch,
        },
    )?;
    super::super::core::emit_named_receipt(
        receipt_out.as_ref(),
        "node control live workflow bundle import receipt",
        &imported.receipt_value,
    )?;
    println!(
        "node live workflow bundle import decision={} bundle={} ticket_import={} authority_import={} imported={} diagnostics={}",
        imported.decision,
        imported.bundle_ref,
        imported.ticket_import_ref.as_deref().unwrap_or("none"),
        imported.authority_import_ref.as_deref().unwrap_or("none"),
        imported.imported_refs.len(),
        imported.diagnostics.len()
    );
    Ok(())
}

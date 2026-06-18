pub(crate) fn import(args: super::command::live::ImportWorkflow) -> molten::error::Result<()> {
    let super::command::live::ImportWorkflow {
        root,
        request,
        response,
        request_control,
        request_send_receipt,
        request_receive_receipt,
        request_ingress_ref,
        response_control,
        response_send_receipt,
        response_receive_receipt,
        response_ingress_ref,
        expected_peer_ref,
        expected_remote_ref,
        import_out,
        receipt_out,
    } = args;
    let request_value = super::io::read_preserves_file(&request)?;
    let response_value = super::io::read_preserves_file(&response)?;
    let request_control_value = super::io::read_preserves_file(&request_control)?;
    let request_send_receipt_value = super::io::read_preserves_file(&request_send_receipt)?;
    let request_receive_receipt_value = super::io::read_preserves_file(&request_receive_receipt)?;
    let response_control_value = super::io::read_preserves_file(&response_control)?;
    let response_send_receipt_value = super::io::read_preserves_file(&response_send_receipt)?;
    let response_receive_receipt_value = super::io::read_preserves_file(&response_receive_receipt)?;
    let imported = molten::retention::import_retention_remote_gc_clearance_live_workflow(
        molten::retention::RetentionRemoteGcClearanceLiveImportWorkflowInput {
            root: &root,
            request_value: &request_value,
            response_value: &response_value,
            request_control_value: &request_control_value,
            request_send_receipt_value: &request_send_receipt_value,
            request_receive_receipt_value: &request_receive_receipt_value,
            request_ingress_ref: &request_ingress_ref,
            response_control_value: &response_control_value,
            response_send_receipt_value: &response_send_receipt_value,
            response_receive_receipt_value: &response_receive_receipt_value,
            response_ingress_ref: &response_ingress_ref,
            expected_peer_ref: expected_peer_ref.as_deref(),
            expected_remote_ref: expected_remote_ref.as_deref(),
        },
    )?;
    super::io::write_optional_preserves(import_out.as_ref(), &imported.import.value)?;
    let is_written_to_file = super::io::write_optional_preserves(receipt_out.as_ref(), &imported.workflow.value)?;
    super::io::print_or_log_summary(
        is_written_to_file,
        &format!(
            "retention remote clearance live import-workflow ref={} decision={} import={} clearance={} request-send={} response-send={} diagnostics={}",
            imported.workflow.workflow_ref,
            imported.workflow.decision,
            imported.import.import_ref,
            imported.import.clearance_ref.as_deref().unwrap_or("none"),
            imported.request_send_receipt_ref,
            imported.response_send_receipt_ref,
            imported.workflow.diagnostics.len()
        ),
    );
    Ok(())
}

pub(crate) fn loopback(args: super::command::live::Loopback) -> molten::error::Result<()> {
    let runtime = runtime()?;
    let live = runtime.block_on(molten::retention::run_retention_remote_gc_clearance_live_loopback(
        molten::retention::RetentionRemoteGcClearanceLiveLoopbackInput {
            root: &args.root,
            requester_node_root: &args.requester_node_root,
            peer_node_root: &args.peer_node_root,
            requester_node_id: &args.requester_node_id,
            peer_node_id: &args.peer_node_id,
            topic: &args.topic,
            request_sequence: args.request_sequence,
            response_sequence: args.response_sequence,
            requester_ref: &args.requester_ref,
            peer_ref: &args.peer_ref,
            object_ref: &args.object_ref,
            object_kind: &args.object_kind,
            retention_class: &args.retention_class,
            action: &args.action,
            remote_ref: &args.remote_ref,
            policy_ref: &args.policy_ref,
            authority_ref: &args.authority_ref,
            retention_evidence_refs: &args.retention_evidence_refs,
            response_evidence_refs: &args.response_evidence_refs,
            retained_refs: &args.retained_refs,
            is_current: !args.is_stale,
            revoked_refs: &args.revoked_refs,
            response_diagnostics: &args.diagnostics,
            request_peer_bootstrap_refs: &args.request_peer_bootstrap_refs,
            request_authority_refs: &args.request_authority_refs,
            request_policy_refs: &args.request_policy_refs,
            request_resource_refs: &args.request_resource_refs,
            request_transport_evidence_refs: &args.request_transport_evidence_refs,
            response_peer_bootstrap_refs: &args.response_peer_bootstrap_refs,
            response_authority_refs: &args.response_authority_refs,
            response_policy_refs: &args.response_policy_refs,
            response_resource_refs: &args.response_resource_refs,
            response_transport_evidence_refs: &args.response_transport_evidence_refs,
        },
    ))?;
    write_optional_value(args.request_out.as_ref(), &live.request.value)?;
    write_optional_value(args.response_out.as_ref(), &live.response.value)?;
    write_optional_value(args.import_out.as_ref(), &live.import.value)?;
    emit_summary(args.receipt_out.as_ref(), Summary {
        value: &live.workflow.value,
        workflow_ref: &live.workflow.workflow_ref,
        decision: &live.workflow.decision,
        request_ref: &live.request.request_ref,
        response_ref: &live.response.response_ref,
        import_ref: &live.import.import_ref,
        clearance_ref: live.import.clearance_ref.as_deref(),
        diagnostics: live.workflow.diagnostics.len(),
    })
}

struct Summary<'a> {
    value: &'a preserves::IOValue,
    workflow_ref: &'a str,
    decision: &'a str,
    request_ref: &'a str,
    response_ref: &'a str,
    import_ref: &'a str,
    clearance_ref: Option<&'a str>,
    diagnostics: usize,
}

fn runtime() -> molten::error::Result<tokio::runtime::Runtime> {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(molten::error::MoltenError::from)
}

fn emit_summary(path: Option<&std::path::PathBuf>, summary: Summary<'_>) -> molten::error::Result<()> {
    let is_written_to_file = super::io::write_optional_preserves(path, summary.value)?;
    super::io::print_or_log_summary(
        is_written_to_file,
        &format!(
            "retention remote clearance live workflow ref={} decision={} request={} response={} import={} clearance={} diagnostics={}",
            summary.workflow_ref,
            summary.decision,
            summary.request_ref,
            summary.response_ref,
            summary.import_ref,
            summary.clearance_ref.unwrap_or("none"),
            summary.diagnostics
        ),
    );
    Ok(())
}

fn write_optional_value(path: Option<&std::path::PathBuf>, value: &preserves::IOValue) -> molten::error::Result<()> {
    if let Some(path) = path {
        super::io::write_file(path, &molten::preserves_rail::to_text(value)?)?;
    }
    Ok(())
}

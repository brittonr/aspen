pub(crate) fn request(args: super::command::live::RequestSend) -> molten::error::Result<()> {
    let ticket_value = super::io::read_preserves_file(&args.peer_ticket)?;
    let runtime = runtime()?;
    let sent = runtime.block_on(molten::retention::send_remote_gc_clearance_live_request(
        molten::retention::RemoteGcClearanceLiveRequestSendInput {
            root: &args.root,
            requester_node_root: args.requester_node_root.as_deref(),
            peer_ticket_value: &ticket_value,
            requester_node_id: &args.requester_node_id,
            peer_node_id: &args.peer_node_id,
            topic: &args.topic,
            sequence: args.sequence,
            max_attempts: args.max_attempts,
            join_timeout_ms: args.join_timeout_ms,
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
            peer_bootstrap_refs: &args.peer_bootstrap_refs,
            authority_refs: &args.authority_refs,
            policy_refs: &args.policy_refs,
            resource_refs: &args.resource_refs,
            transport_evidence_refs: &args.transport_evidence_refs,
        },
    ))?;
    super::io::write_optional_preserves(args.request_out.as_ref(), &sent.request.value)?;
    super::io::write_optional_preserves(args.control_out.as_ref(), &sent.control_value)?;
    write_transport_receipt(args.transport_receipt_out.as_ref(), sent.send.transport_receipt_value.as_ref())?;
    emit_summary(args.receipt_out.as_ref(), &sent.send.send_receipt_value, Summary {
        operation: "request-send",
        artifact: "request",
        artifact_ref: &sent.request.request_ref,
        control_ref: &sent.control_ref,
        send_ref: &sent.send.send_receipt_ref,
        transport_ref: sent.send.transport_receipt_ref.as_deref(),
    })
}

pub(crate) fn response(args: super::command::live::ResponseSend) -> molten::error::Result<()> {
    let ticket_value = super::io::read_preserves_file(&args.requester_ticket)?;
    let request_value = super::io::read_preserves_file(&args.request)?;
    let runtime = runtime()?;
    let sent = runtime.block_on(molten::retention::send_remote_gc_clearance_live_response(
        molten::retention::RemoteGcClearanceLiveResponseSendInput {
            root: &args.root,
            peer_node_root: args.peer_node_root.as_deref(),
            requester_ticket_value: &ticket_value,
            request_value: &request_value,
            peer_node_id: &args.peer_node_id,
            requester_node_id: &args.requester_node_id,
            topic: &args.topic,
            sequence: args.sequence,
            max_attempts: args.max_attempts,
            join_timeout_ms: args.join_timeout_ms,
            response_evidence_refs: &args.response_evidence_refs,
            retained_refs: &args.retained_refs,
            is_current: !args.is_stale,
            revoked_refs: &args.revoked_refs,
            response_diagnostics: &args.diagnostics,
            peer_bootstrap_refs: &args.peer_bootstrap_refs,
            authority_refs: &args.authority_refs,
            policy_refs: &args.policy_refs,
            resource_refs: &args.resource_refs,
            transport_evidence_refs: &args.transport_evidence_refs,
        },
    ))?;
    super::io::write_optional_preserves(args.response_out.as_ref(), &sent.response.value)?;
    super::io::write_optional_preserves(args.control_out.as_ref(), &sent.control_value)?;
    write_transport_receipt(args.transport_receipt_out.as_ref(), sent.send.transport_receipt_value.as_ref())?;
    emit_summary(args.receipt_out.as_ref(), &sent.send.send_receipt_value, Summary {
        operation: "response-send",
        artifact: "response",
        artifact_ref: &sent.response.response_ref,
        control_ref: &sent.control_ref,
        send_ref: &sent.send.send_receipt_ref,
        transport_ref: sent.send.transport_receipt_ref.as_deref(),
    })
}

struct Summary<'a> {
    operation: &'static str,
    artifact: &'static str,
    artifact_ref: &'a str,
    control_ref: &'a str,
    send_ref: &'a str,
    transport_ref: Option<&'a str>,
}

fn runtime() -> molten::error::Result<tokio::runtime::Runtime> {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(molten::error::MoltenError::from)
}

fn emit_summary(
    path: Option<&std::path::PathBuf>,
    receipt: &preserves::IOValue,
    summary: Summary<'_>,
) -> molten::error::Result<()> {
    let is_written_to_file = super::io::write_optional_preserves(path, receipt)?;
    let diagnostics = molten::node_daemon::parse_control_live_send_receipt(receipt)?.diagnostics.len();
    super::io::print_or_log_summary(
        is_written_to_file,
        &format!(
            "retention remote clearance live {} {}={} control={} send={} transport={} diagnostics={}",
            summary.operation,
            summary.artifact,
            summary.artifact_ref,
            summary.control_ref,
            summary.send_ref,
            summary.transport_ref.unwrap_or("none"),
            diagnostics
        ),
    );
    Ok(())
}

fn write_transport_receipt(
    path: Option<&std::path::PathBuf>,
    value: Option<&preserves::IOValue>,
) -> molten::error::Result<()> {
    if let Some(path) = path
        && let Some(value) = value
    {
        super::io::write_file(path, &molten::preserves_rail::to_text(value)?)?;
    }
    Ok(())
}

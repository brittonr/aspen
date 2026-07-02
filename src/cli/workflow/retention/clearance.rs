pub(crate) fn record(args: super::command::base::Record) -> molten::error::Result<()> {
    let super::command::base::Record {
        root,
        decision,
        requester_ref,
        peer_ref,
        object_ref,
        object_kind,
        retention_class,
        action,
        remote_ref,
        policy_ref,
        authority_ref,
        evidence_refs,
        retained_refs,
        is_stale,
        revoked_refs,
        diagnostics,
        out,
    } = args;
    let clearance = molten::retention::store_remote_gc_clearance(&root, &molten::retention::RemoteGcClearanceInput {
        decision: &decision,
        requester_ref: &requester_ref,
        peer_ref: &peer_ref,
        object_ref: &object_ref,
        object_kind: &object_kind,
        retention_class: &retention_class,
        action: &action,
        remote_ref: &remote_ref,
        policy_ref: &policy_ref,
        authority_ref: &authority_ref,
        evidence_refs: &evidence_refs,
        retained_refs: &retained_refs,
        is_current: !is_stale,
        revoked_refs: &revoked_refs,
        diagnostics: &diagnostics,
    })?;
    let is_written_to_file = super::io::write_optional_preserves(out.as_ref(), &clearance.value)?;
    super::io::print_or_log_summary(
        is_written_to_file,
        &format!(
            "retention remote clearance ref={} peer={} remote={} decision={}",
            clearance.clearance_ref, clearance.peer_ref, clearance.remote_ref, clearance.decision
        ),
    );
    Ok(())
}

pub(crate) fn request(args: super::command::base::Request) -> molten::error::Result<()> {
    let super::command::base::Request {
        root,
        requester_ref,
        peer_ref,
        object_ref,
        object_kind,
        retention_class,
        action,
        remote_ref,
        policy_ref,
        authority_ref,
        evidence_refs,
        out,
    } = args;
    let request = molten::retention::store_retention_remote_gc_clearance_request(
        &root,
        &molten::retention::RemoteGcClearanceRequestInput {
            requester_ref: &requester_ref,
            peer_ref: &peer_ref,
            object_ref: &object_ref,
            object_kind: &object_kind,
            retention_class: &retention_class,
            action: &action,
            remote_ref: &remote_ref,
            policy_ref: &policy_ref,
            authority_ref: &authority_ref,
            evidence_refs: &evidence_refs,
        },
    )?;
    let is_written_to_file = super::io::write_optional_preserves(out.as_ref(), &request.value)?;
    super::io::print_or_log_summary(
        is_written_to_file,
        &format!(
            "retention remote clearance request ref={} peer={} remote={} object={}",
            request.request_ref, request.peer_ref, request.remote_ref, request.object_ref
        ),
    );
    Ok(())
}

pub(crate) fn respond(args: super::command::base::Respond) -> molten::error::Result<()> {
    let super::command::base::Respond {
        root,
        request,
        evidence_refs,
        retained_refs,
        is_stale,
        revoked_refs,
        diagnostics,
        out,
    } = args;
    let request_value = super::io::read_preserves_file(&request)?;
    let response = molten::retention::store_retention_remote_gc_clearance_response(
        molten::retention::RemoteGcClearanceResponseInput {
            root: &root,
            request_value: &request_value,
            evidence_refs: &evidence_refs,
            retained_refs: &retained_refs,
            is_current: !is_stale,
            revoked_refs: &revoked_refs,
            diagnostics: &diagnostics,
        },
    )?;
    let is_written_to_file = super::io::write_optional_preserves(out.as_ref(), &response.value)?;
    super::io::print_or_log_summary(
        is_written_to_file,
        &format!(
            "retention remote clearance response ref={} decision={} request={} clearance={}",
            response.response_ref, response.decision, response.request_ref, response.clearance_ref
        ),
    );
    Ok(())
}

pub(crate) fn import(args: super::command::base::Import) -> molten::error::Result<()> {
    let super::command::base::Import {
        root,
        request,
        response,
        expected_peer_ref,
        expected_remote_ref,
        out,
    } = args;
    let request_value = super::io::read_preserves_file(&request)?;
    let response_value = super::io::read_preserves_file(&response)?;
    let import =
        molten::retention::import_remote_gc_clearance_response(molten::retention::RemoteGcClearanceImportInput {
            root: &root,
            request_value: &request_value,
            response_value: &response_value,
            expected_peer_ref: expected_peer_ref.as_deref(),
            expected_remote_ref: expected_remote_ref.as_deref(),
        })?;
    let is_written_to_file = super::io::write_optional_preserves(out.as_ref(), &import.value)?;
    super::io::print_or_log_summary(
        is_written_to_file,
        &format!(
            "retention remote clearance import ref={} decision={} clearance={}",
            import.import_ref,
            import.decision,
            import.clearance_ref.as_deref().unwrap_or("none")
        ),
    );
    Ok(())
}

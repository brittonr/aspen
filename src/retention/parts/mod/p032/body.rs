
pub async fn run_remote_gc_clearance_live_loopback(
    input: RemoteGcClearanceLiveLoopbackInput<'_>,
) -> Result<RemoteGcClearanceLiveLoopback> {
    let retention_root = open_capability_retention_root(input.root)?;
    ensure_store_with_root(&retention_root)?;
    validate_remote_gc_clearance_live_loopback_input(&input)?;
    let request = store_remote_gc_clearance_request_with_root(&retention_root, &RemoteGcClearanceRequestInput {
        requester_ref: input.requester_ref,
        peer_ref: input.peer_ref,
        object_ref: input.object_ref,
        object_kind: input.object_kind,
        retention_class: input.retention_class,
        action: input.action,
        remote_ref: input.remote_ref,
        policy_ref: input.policy_ref,
        authority_ref: input.authority_ref,
        evidence_refs: input.retention_evidence_refs,
    })?;
    let request_control_evidence = request_evidence(&input, &request.request_ref)?;
    let (request_control_ref, request_control_value) =
        request_control(&input, &request.request_ref, &request_control_evidence)?;
    let request_live = request_leg(&input, &request_control_value, &request_control_evidence).await?;

    let response = store_remote_gc_clearance_response_with_root(RemoteGcClearanceResponseInput {
        root: &retention_root,
        request_value: &request.value,
        evidence_refs: input.response_evidence_refs,
        retained_refs: input.retained_refs,
        is_current: input.is_current,
        revoked_refs: input.revoked_refs,
        diagnostics: input.response_diagnostics,
    })?;
    let response_control_evidence = response_evidence(&input, &request.request_ref, &response.response_ref)?;
    let (response_control_ref, response_control_value) =
        response_control(&input, &request.request_ref, &response.response_ref, &response_control_evidence)?;
    let response_live = response_leg(&input, &response_control_value, &response_control_evidence).await?;

    let import = import_remote_gc_clearance_response_with_root(RemoteGcClearanceImportInput {
        root: &retention_root,
        request_value: &request.value,
        response_value: &response.value,
        expected_peer_ref: Some(input.peer_ref),
        expected_remote_ref: Some(input.remote_ref),
    })?;
    let transport_diagnostics = transport_notes(&request_live, &response_live)?;
    let workflow_value = loopback_value(&LoopbackValueInput {
        request_value: &request.value,
        response_value: &response.value,
        import_value: &import.value,
        request_control_ref: &request_control_ref,
        response_control_ref: &response_control_ref,
        request_live: &request_live,
        response_live: &response_live,
        transport_diagnostics: &transport_diagnostics,
    })?;
    let workflow = store_remote_gc_clearance_live_workflow_with_root(&retention_root, &workflow_value)?;
    Ok(RemoteGcClearanceLiveLoopback {
        request,
        response,
        import,
        workflow,
        request_publish_receipt_value: request_live.publish_receipt_value,
        request_receive_receipt_value: request_live.receive_receipt_value,
        response_publish_receipt_value: response_live.publish_receipt_value,
        response_receive_receipt_value: response_live.receive_receipt_value,
    })
}

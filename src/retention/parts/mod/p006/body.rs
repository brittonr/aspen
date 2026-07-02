
pub async fn send_remote_gc_clearance_live_request(
    input: RemoteGcClearanceLiveRequestSendInput<'_>,
) -> Result<RemoteGcClearanceLiveRequestSend> {
    ensure_store(input.root)?;
    validate_remote_gc_clearance_live_request_send_input(&input)?;
    let request = store_remote_gc_clearance_request(input.root, &RemoteGcClearanceRequestInput {
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
    let control_evidence = refs_with_extra(
        input.transport_evidence_refs,
        std::slice::from_ref(&request.request_ref),
        "retention live request transport evidence ref",
    )?;
    let (control_ref, control_value) = remote_clearance_live_control_request_value(&LiveControlRequestInput {
        target_ref: &request.request_ref,
        payload_ref: None,
        authority_refs: input.authority_refs,
        policy_refs: input.policy_refs,
        resource_refs: input.resource_refs,
        evidence_refs: &control_evidence,
    })?;
    let send = crate::node_daemon::send_control_live_ingress(&crate::node_daemon::ControlLiveSendInput {
        state_root: input.requester_node_root,
        request_value: &control_value,
        receiver_ticket_value: input.peer_ticket_value,
        from_peer: input.requester_node_id,
        sequence: input.sequence,
        expected_operation_ref: None,
        expected_receiver_node: Some(input.peer_node_id),
        expected_topic: Some(input.topic),
        expected_endpoint: None,
        max_attempts: input.max_attempts,
        peer_bootstrap_refs: input.peer_bootstrap_refs,
        authority_refs: input.authority_refs,
        policy_refs: input.policy_refs,
        resource_refs: input.resource_refs,
        evidence_refs: &control_evidence,
        join_timeout_ms: input.join_timeout_ms,
    })
    .await?;
    Ok(RemoteGcClearanceLiveRequestSend {
        request,
        control_ref,
        control_value,
        send,
    })
}

pub async fn send_remote_gc_clearance_live_response(
    input: RemoteGcClearanceLiveResponseSendInput<'_>,
) -> Result<RemoteGcClearanceLiveResponseSend> {
    ensure_store(input.root)?;
    validate_remote_gc_clearance_live_response_send_input(&input)?;
    let request = parse_remote_gc_clearance_request(input.request_value)?;
    let response = store_remote_gc_clearance_response(RemoteGcClearanceResponseInput {
        root: input.root,
        request_value: input.request_value,
        evidence_refs: input.response_evidence_refs,
        retained_refs: input.retained_refs,
        is_current: input.is_current,
        revoked_refs: input.revoked_refs,
        diagnostics: input.response_diagnostics,
    })?;
    let control_evidence = refs_with_extra(
        input.transport_evidence_refs,
        &[request.request_ref.clone(), response.response_ref.clone()],
        "retention live response transport evidence ref",
    )?;
    let (control_ref, control_value) = remote_clearance_live_control_request_value(&LiveControlRequestInput {
        target_ref: &response.response_ref,
        payload_ref: Some(&request.request_ref),
        authority_refs: input.authority_refs,
        policy_refs: input.policy_refs,
        resource_refs: input.resource_refs,
        evidence_refs: &control_evidence,
    })?;
    let send = crate::node_daemon::send_control_live_ingress(&crate::node_daemon::ControlLiveSendInput {
        state_root: input.peer_node_root,
        request_value: &control_value,
        receiver_ticket_value: input.requester_ticket_value,
        from_peer: input.peer_node_id,
        sequence: input.sequence,
        expected_operation_ref: None,
        expected_receiver_node: Some(input.requester_node_id),
        expected_topic: Some(input.topic),
        expected_endpoint: None,
        max_attempts: input.max_attempts,
        peer_bootstrap_refs: input.peer_bootstrap_refs,
        authority_refs: input.authority_refs,
        policy_refs: input.policy_refs,
        resource_refs: input.resource_refs,
        evidence_refs: &control_evidence,
        join_timeout_ms: input.join_timeout_ms,
    })
    .await?;
    Ok(RemoteGcClearanceLiveResponseSend {
        response,
        control_ref,
        control_value,
        send,
    })
}

pub fn import_remote_gc_clearance_live_workflow(
    input: RemoteGcClearanceLiveImportWorkflowInput<'_>,
) -> Result<RemoteGcClearanceLiveImportWorkflow> {
    ensure_store(input.root)?;
    validate_remote_gc_clearance_live_import_workflow_input(&input)?;
    let request = parse_remote_gc_clearance_request(input.request_value)?;
    let response_ref = crate::preserves_rail::canonical_hash(input.response_value)?;
    let import = import_remote_gc_clearance_response(RemoteGcClearanceImportInput {
        root: input.root,
        request_value: input.request_value,
        response_value: input.response_value,
        expected_peer_ref: input.expected_peer_ref,
        expected_remote_ref: input.expected_remote_ref,
    })?;
    let request_control = crate::node_runtime::parse_control_request(input.request_control_value)?;
    let response_control = crate::node_runtime::parse_control_request(input.response_control_value)?;
    let request_control_ref = crate::preserves_rail::canonical_hash(input.request_control_value)?;
    let response_control_ref = crate::preserves_rail::canonical_hash(input.response_control_value)?;
    let request_send = crate::node_daemon::parse_control_live_send_receipt(input.request_send_receipt_value)?;
    let response_send = crate::node_daemon::parse_control_live_send_receipt(input.response_send_receipt_value)?;
    let request_receive = parse_node_live_transport_receipt(input.request_receive_receipt_value)?;
    let response_receive = parse_node_live_transport_receipt(input.response_receive_receipt_value)?;
    let diagnostics = live_import_diagnostics(LiveImportDiagnosticsInput {
        request: &request,
        response_ref: &response_ref,
        request_control: &request_control,
        response_control: &response_control,
        request_send: &request_send,
        response_send: &response_send,
        request_receive: &request_receive,
        response_receive: &response_receive,
        request_ingress_ref: input.request_ingress_ref,
        response_ingress_ref: input.response_ingress_ref,
    })?;
    let request_publish_ref = live_send_publish_ref(&request_send);
    let response_publish_ref = live_send_publish_ref(&response_send);
    let request_receive_ref = request_receive.receipt_ref.clone();
    let response_receive_ref = response_receive.receipt_ref.clone();
    let workflow_value = remote_gc_clearance_live_workflow_value(&RemoteGcClearanceLiveWorkflowValueInput {
        request_value: input.request_value,
        response_value: input.response_value,
        import_value: &import.value,
        request_control_ref: &request_control_ref,
        request_publish_ref: &request_publish_ref,
        request_receive_ref: &request_receive_ref,
        request_ingress_ref: input.request_ingress_ref,
        response_control_ref: &response_control_ref,
        response_publish_ref: &response_publish_ref,
        response_receive_ref: &response_receive_ref,
        response_ingress_ref: input.response_ingress_ref,
        transport_diagnostics: &diagnostics,
    })?;
    let workflow = store_remote_gc_clearance_live_workflow(input.root, &workflow_value)?;
    Ok(RemoteGcClearanceLiveImportWorkflow {
        import,
        workflow,
        request_send_receipt_ref: request_send.receipt_ref,
        response_send_receipt_ref: response_send.receipt_ref,
    })
}

struct LiveImportDiagnosticsInput<'a> {
    request: &'a RemoteGcClearanceRequest,
    response_ref: &'a str,
    request_control: &'a crate::node_runtime::ControlRequest,
    response_control: &'a crate::node_runtime::ControlRequest,
    request_send: &'a crate::node_daemon::ControlLiveSendReceipt,
    response_send: &'a crate::node_daemon::ControlLiveSendReceipt,
    request_receive: &'a NodeLiveTransportReceipt,
    response_receive: &'a NodeLiveTransportReceipt,
    request_ingress_ref: &'a str,
    response_ingress_ref: &'a str,
}

fn live_import_diagnostics(input: LiveImportDiagnosticsInput<'_>) -> Result<Vec<String>> {
    let mut diagnostics = live_import_request_diagnostics(&input)?;
    extend_bounded(
        &mut diagnostics,
        live_import_response_diagnostics(&input)?,
        MAX_RETENTION_DIAGNOSTICS,
        "retention live workflow diagnostics",
    )?;
    Ok(diagnostics)
}

fn live_import_request_diagnostics(input: &LiveImportDiagnosticsInput<'_>) -> Result<Vec<String>> {
    let mut diagnostics = Vec::new();
    extend_bounded(
        &mut diagnostics,
        node_live_control_diagnostics("request-control", input.request_control, &input.request.request_ref, None),
        MAX_RETENTION_DIAGNOSTICS,
        "retention live workflow diagnostics",
    )?;
    extend_bounded(
        &mut diagnostics,
        node_live_send_diagnostics("request-send", input.request_send),
        MAX_RETENTION_DIAGNOSTICS,
        "retention live workflow diagnostics",
    )?;
    extend_bounded(
        &mut diagnostics,
        node_live_transport_diagnostics_from("request-receive", input.request_receive)?,
        MAX_RETENTION_DIAGNOSTICS,
        "retention live workflow diagnostics",
    )?;
    extend_bounded(
        &mut diagnostics,
        node_live_receive_binding_diagnostics(
            "request-receive",
            input.request_send,
            input.request_receive,
            input.request_ingress_ref,
        ),
        MAX_RETENTION_DIAGNOSTICS,
        "retention live workflow diagnostics",
    )?;
    Ok(diagnostics)
}

fn live_import_response_diagnostics(input: &LiveImportDiagnosticsInput<'_>) -> Result<Vec<String>> {
    let mut diagnostics = Vec::new();
    extend_bounded(
        &mut diagnostics,
        node_live_control_diagnostics(
            "response-control",
            input.response_control,
            input.response_ref,
            Some(&input.request.request_ref),
        ),
        MAX_RETENTION_DIAGNOSTICS,
        "retention live workflow diagnostics",
    )?;
    extend_bounded(
        &mut diagnostics,
        node_live_send_diagnostics("response-send", input.response_send),
        MAX_RETENTION_DIAGNOSTICS,
        "retention live workflow diagnostics",
    )?;
    extend_bounded(
        &mut diagnostics,
        node_live_transport_diagnostics_from("response-receive", input.response_receive)?,
        MAX_RETENTION_DIAGNOSTICS,
        "retention live workflow diagnostics",
    )?;
    extend_bounded(
        &mut diagnostics,
        node_live_receive_binding_diagnostics(
            "response-receive",
            input.response_send,
            input.response_receive,
            input.response_ingress_ref,
        ),
        MAX_RETENTION_DIAGNOSTICS,
        "retention live workflow diagnostics",
    )?;
    Ok(diagnostics)
}

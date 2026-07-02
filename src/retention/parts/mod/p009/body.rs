
struct LoopbackValueInput<'a> {
    request_value: &'a IoValue,
    response_value: &'a IoValue,
    import_value: &'a IoValue,
    request_control_ref: &'a str,
    response_control_ref: &'a str,
    request_live: &'a crate::node_daemon::ControlLiveLoopback,
    response_live: &'a crate::node_daemon::ControlLiveLoopback,
    transport_diagnostics: &'a [String],
}

fn request_evidence(input: &RemoteGcClearanceLiveLoopbackInput<'_>, request_ref: &str) -> Result<Vec<String>> {
    let extra_refs = [request_ref.to_string()];
    refs_with_extra(input.request_transport_evidence_refs, &extra_refs, "retention live request transport evidence ref")
}

fn response_evidence(
    input: &RemoteGcClearanceLiveLoopbackInput<'_>,
    request_ref: &str,
    response_ref: &str,
) -> Result<Vec<String>> {
    let extra_refs = [request_ref.to_string(), response_ref.to_string()];
    refs_with_extra(
        input.response_transport_evidence_refs,
        &extra_refs,
        "retention live response transport evidence ref",
    )
}

fn request_control(
    input: &RemoteGcClearanceLiveLoopbackInput<'_>,
    request_ref: &str,
    evidence_refs: &[String],
) -> Result<(String, IoValue)> {
    remote_clearance_live_control_request_value(&LiveControlRequestInput {
        target_ref: request_ref,
        payload_ref: None,
        authority_refs: input.request_authority_refs,
        policy_refs: input.request_policy_refs,
        resource_refs: input.request_resource_refs,
        evidence_refs,
    })
}

fn response_control(
    input: &RemoteGcClearanceLiveLoopbackInput<'_>,
    request_ref: &str,
    response_ref: &str,
    evidence_refs: &[String],
) -> Result<(String, IoValue)> {
    remote_clearance_live_control_request_value(&LiveControlRequestInput {
        target_ref: response_ref,
        payload_ref: Some(request_ref),
        authority_refs: input.response_authority_refs,
        policy_refs: input.response_policy_refs,
        resource_refs: input.response_resource_refs,
        evidence_refs,
    })
}

async fn request_leg(
    input: &RemoteGcClearanceLiveLoopbackInput<'_>,
    control_value: &IoValue,
    evidence_refs: &[String],
) -> Result<crate::node_daemon::ControlLiveLoopback> {
    crate::node_daemon::control_live_iroh_loopback(&crate::node_daemon::ControlLiveLoopbackInput {
        state_root: input.peer_node_root,
        request_value: control_value,
        from_peer: input.requester_node_id,
        to_node: input.peer_node_id,
        topic: input.topic,
        sequence: input.request_sequence,
        peer_bootstrap_refs: input.request_peer_bootstrap_refs,
        authority_refs: input.request_authority_refs,
        policy_refs: input.request_policy_refs,
        resource_refs: input.request_resource_refs,
        evidence_refs,
    })
    .await
}

async fn response_leg(
    input: &RemoteGcClearanceLiveLoopbackInput<'_>,
    control_value: &IoValue,
    evidence_refs: &[String],
) -> Result<crate::node_daemon::ControlLiveLoopback> {
    crate::node_daemon::control_live_iroh_loopback(&crate::node_daemon::ControlLiveLoopbackInput {
        state_root: input.requester_node_root,
        request_value: control_value,
        from_peer: input.peer_node_id,
        to_node: input.requester_node_id,
        topic: input.topic,
        sequence: input.response_sequence,
        peer_bootstrap_refs: input.response_peer_bootstrap_refs,
        authority_refs: input.response_authority_refs,
        policy_refs: input.response_policy_refs,
        resource_refs: input.response_resource_refs,
        evidence_refs,
    })
    .await
}

fn transport_notes(
    request_live: &crate::node_daemon::ControlLiveLoopback,
    response_live: &crate::node_daemon::ControlLiveLoopback,
) -> Result<Vec<String>> {
    let mut diagnostics = Vec::new();
    extend_bounded(
        &mut diagnostics,
        node_live_transport_diagnostics("request-publish", &request_live.publish_receipt_value)?,
        MAX_RETENTION_DIAGNOSTICS,
        "retention live workflow diagnostics",
    )?;
    extend_bounded(
        &mut diagnostics,
        node_live_transport_diagnostics("request-receive", &request_live.receive_receipt_value)?,
        MAX_RETENTION_DIAGNOSTICS,
        "retention live workflow diagnostics",
    )?;
    extend_bounded(
        &mut diagnostics,
        node_live_transport_diagnostics("response-publish", &response_live.publish_receipt_value)?,
        MAX_RETENTION_DIAGNOSTICS,
        "retention live workflow diagnostics",
    )?;
    extend_bounded(
        &mut diagnostics,
        node_live_transport_diagnostics("response-receive", &response_live.receive_receipt_value)?,
        MAX_RETENTION_DIAGNOSTICS,
        "retention live workflow diagnostics",
    )?;
    Ok(diagnostics)
}

fn loopback_value(input: &LoopbackValueInput<'_>) -> Result<IoValue> {
    remote_gc_clearance_live_workflow_value(&RemoteGcClearanceLiveWorkflowValueInput {
        request_value: input.request_value,
        response_value: input.response_value,
        import_value: input.import_value,
        request_control_ref: input.request_control_ref,
        request_publish_ref: &input.request_live.publish_receipt_ref,
        request_receive_ref: &input.request_live.receive_receipt_ref,
        request_ingress_ref: &input.request_live.ingress_receipt_ref,
        response_control_ref: input.response_control_ref,
        response_publish_ref: &input.response_live.publish_receipt_ref,
        response_receive_ref: &input.response_live.receive_receipt_ref,
        response_ingress_ref: &input.response_live.ingress_receipt_ref,
        transport_diagnostics: input.transport_diagnostics,
    })
}

fn gate_scope<'a>(input: &'a GcPlanInput<'a>) -> AdmissionScope<'a> {
    AdmissionScope {
        requester_ref: input.evidence.requester_ref.as_deref(),
        object_ref: input.object_ref,
        object_kind: input.object_kind,
        retention_class: input.retention_class,
        action: input.action,
    }
}

fn gate_admissions(input: &GcPlanInput<'_>, scope: &AdmissionScope<'_>) -> Result<GateAdmissions> {
    let policy = admit_evidence_refs(AdmissionRefsInput {
        root: input.root,
        refs: &input.evidence.policy_refs,
        expected_kind: ADMISSION_KIND_POLICY,
        scope,
        required_remote_refs: &[],
    })?;
    let authority = admit_evidence_refs(AdmissionRefsInput {
        root: input.root,
        refs: &input.evidence.authority_refs,
        expected_kind: ADMISSION_KIND_AUTHORITY,
        scope,
        required_remote_refs: &[],
    })?;
    let supporting = admit_evidence_refs(AdmissionRefsInput {
        root: input.root,
        refs: &input.evidence.evidence_refs,
        expected_kind: ADMISSION_KIND_SUPPORTING_EVIDENCE,
        scope,
        required_remote_refs: &[],
    })?;
    let reference_index = admit_evidence_refs(AdmissionRefsInput {
        root: input.root,
        refs: &input.evidence.reference_index_refs,
        expected_kind: ADMISSION_KIND_REFERENCE_INDEX,
        scope,
        required_remote_refs: &[],
    })?;
    let remote_gc = admit_evidence_refs(AdmissionRefsInput {
        root: input.root,
        refs: &input.evidence.remote_gc_refs,
        expected_kind: ADMISSION_KIND_REMOTE_GC,
        scope,
        required_remote_refs: &input.evidence.remote_refs,
    })?;
    Ok(GateAdmissions {
        policy,
        authority,
        supporting,
        reference_index,
        remote_gc,
    })
}

fn gate_remote_clearance(input: &GcPlanInput<'_>, scope: &AdmissionScope<'_>) -> Result<RemoteClearanceRefsResult> {
    admit_remote_clearance_refs(RemoteClearanceRefsInput {
        root: input.root,
        refs: &input.evidence.remote_clearance_refs,
        scope,
        required_remote_refs: &input.evidence.remote_refs,
        required_peer_refs: &input.evidence.remote_peer_refs,
        policy_refs: &input.evidence.policy_refs,
        authority_refs: &input.evidence.authority_refs,
    })
}

fn has_all_refs(required: &[String], admitted: &[String]) -> bool {
    required.is_empty() || required.iter().all(|reference| admitted.iter().any(|candidate| candidate == reference))
}

fn is_clearance_complete(
    input: &GcPlanInput<'_>,
    remote_gc: &AdmissionRefsResult,
    remote_clearance: &RemoteClearanceRefsResult,
) -> bool {
    let has_local_plan = has_all_refs(&input.evidence.remote_refs, &remote_gc.remote_refs);
    let has_remote_refs = has_all_refs(&input.evidence.remote_refs, &remote_clearance.remote_refs);
    let has_remote_peers = has_all_refs(&input.evidence.remote_peer_refs, &remote_clearance.peer_refs);
    has_local_plan && has_remote_refs && has_remote_peers
}

fn gate_inputs<'a>(input: &'a GcPlanInput<'a>) -> Result<GateInputs<'a>> {
    let scope = gate_scope(input);
    let admissions = gate_admissions(input, &scope)?;
    let remote_clearance = gate_remote_clearance(input, &scope)?;
    let has_remote_gc_clearance = is_clearance_complete(input, &admissions.remote_gc, &remote_clearance);
    let has_delete_authority = is_destructive_action(input.action)
        && !admissions.authority.admitted_refs.is_empty()
        && !admissions.policy.admitted_refs.is_empty()
        && !admissions.supporting.admitted_refs.is_empty()
        && (!input.evidence.is_reference_index_complete || !admissions.reference_index.admitted_refs.is_empty())
        && has_remote_gc_clearance;
    Ok(GateInputs {
        input,
        policy: admissions.policy,
        authority: admissions.authority,
        supporting: admissions.supporting,
        reference_index: admissions.reference_index,
        remote_gc: admissions.remote_gc,
        remote_clearance,
        has_delete_authority,
        has_remote_gc_clearance,
    })
}

fn retention_plan_gates(input: &GateInputs<'_>, index: &ReferenceIndex) -> Result<Vec<PlanGate>> {
    let mut gates = Vec::new();
    push_access_gates(&mut gates, input)?;
    push_index_gates(&mut gates, input, index)?;
    push_external_gates(&mut gates, input)?;
    Ok(gates)
}

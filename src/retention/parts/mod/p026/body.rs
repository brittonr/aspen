
fn push_notes<S, I>(values: &mut S, entries: I) -> Result<()>
where
    S: VecSink<String>,
    I: IntoIterator<Item = (bool, &'static str)>,
{
    for (is_active, note) in entries {
        if is_active {
            push_bounded(values, note.to_string(), MAX_RETENTION_DIAGNOSTICS, "retention diagnostics")?;
        }
    }
    Ok(())
}

fn class_profile_diagnostics(input: &ClassProfileInput) -> Result<Vec<String>> {
    let mut diagnostics = Vec::new();
    if input.class_name == CLASS_PRIVATE_SECRET_REF && !input.has_secret_redaction_hook {
        push_bounded(
            &mut diagnostics,
            "private-secret-redaction-hook-missing".to_string(),
            MAX_RETENTION_DIAGNOSTICS,
            "retention diagnostics",
        )?;
    }
    if !input.has_remote_gc_plan {
        push_bounded(
            &mut diagnostics,
            "remote-gc-plan-not-declared".to_string(),
            MAX_RETENTION_DIAGNOSTICS,
            "retention diagnostics",
        )?;
    }
    Ok(diagnostics)
}

fn is_destructive_action(action: &str) -> bool {
    matches!(action, ACTION_DELETE | ACTION_TOMBSTONE | ACTION_REDACT | ACTION_COMPACT)
}

fn validate_class_profile_input(input: &ClassProfileInput) -> Result<()> {
    validate_class(&input.class_name)?;
    require_ref(&input.deletion_authority_ref, "retention deletion authority ref")?;
    validate_refs(&input.policy_refs, "retention class policy ref")?;
    if input.policy_refs.is_empty() {
        return Err(MoltenError::invalid_harness("retention class profile requires policy refs"));
    }
    if input.maximum_age_seconds.is_some_and(|maximum| maximum < input.minimum_age_seconds) {
        return Err(MoltenError::invalid_harness("retention maximum age cannot be below minimum age"));
    }
    Ok(())
}

fn validate_pin_input(input: &PinInput) -> Result<()> {
    require_ref(&input.object_ref, "retention pin object ref")?;
    validate_name(&input.object_kind, "retention pin object kind")?;
    validate_class(&input.retention_class)?;
    validate_pin_source(&input.source)?;
    validate_name(&input.reason, "retention pin reason")?;
    require_ref(&input.owner_ref, "retention pin owner ref")?;
    if let Some(expiry) = input.expiry_ref.as_deref() {
        require_ref(expiry, "retention pin expiry ref")?;
    }
    validate_refs(&input.policy_refs, "retention pin policy ref")?;
    validate_refs(&input.evidence_refs, "retention pin evidence ref")?;
    if input.policy_refs.is_empty() {
        return Err(MoltenError::invalid_harness("retention pin requires policy refs"));
    }
    Ok(())
}

fn validate_reference_index_input(input: &ReferenceIndexInput) -> Result<()> {
    require_ref(&input.object_ref, "retention index object ref")?;
    validate_name(&input.object_kind, "retention index object kind")?;
    validate_refs(&input.pin_refs, "retention index pin ref")?;
    validate_refs(&input.retained_refs, "retention index retained ref")?;
    validate_refs(&input.tombstone_refs, "retention index tombstone ref")?;
    validate_refs(&input.remote_refs, "retention index remote ref")?;
    Ok(())
}

fn validate_receipt_build_input(input: &ReceiptBuildInput<'_>) -> Result<()> {
    if input.decision != "pass" && input.decision != "deny" {
        return Err(MoltenError::invalid_harness("retention receipt decision must be pass or deny"));
    }
    validate_action(input.action)?;
    require_ref(input.object_ref, "retention receipt object ref")?;
    validate_name(input.object_kind, "retention receipt object kind")?;
    validate_class(input.retention_class)?;
    require_ref(input.requester_ref, "retention receipt requester ref")?;
    require_ref(input.index_ref, "retention receipt index ref")?;
    validate_refs(input.pin_refs, "retention receipt pin ref")?;
    validate_refs(input.retained_refs, "retention receipt retained ref")?;
    validate_refs(input.remote_refs, "retention receipt remote ref")?;
    validate_refs(input.policy_refs, "retention receipt policy ref")?;
    validate_refs(input.evidence_refs, "retention receipt evidence ref")?;
    if let Some(tombstone_ref) = input.tombstone_ref {
        require_ref(tombstone_ref, "retention receipt tombstone ref")?;
    }
    ensure_count_at_most(input.diagnostics.len(), MAX_RETENTION_DIAGNOSTICS, "retention receipt diagnostics")
}

fn validate_class(value: &str) -> Result<()> {
    if RETENTION_CLASSES.iter().any(|class| class == &value) {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("unsupported retention class {value}")))
    }
}

fn validate_pin_source(value: &str) -> Result<()> {
    if PIN_SOURCES.iter().any(|source| source == &value) {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("unsupported retention pin source {value}")))
    }
}

fn validate_action(value: &str) -> Result<()> {
    if RETENTION_ACTIONS.iter().any(|action| action == &value) {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("unsupported retention action {value}")))
    }
}

fn validate_admission_kind(value: &str) -> Result<()> {
    if ADMISSION_KINDS.iter().any(|kind| kind == &value) {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("unsupported retention admission kind {value}")))
    }
}

fn validate_decision(value: &str) -> Result<()> {
    if matches!(value, "pass" | "deny") {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("unsupported retention admission decision {value}")))
    }
}

fn validate_evidence_admission_input(input: &EvidenceAdmissionInput<'_>) -> Result<()> {
    validate_admission_kind(input.kind)?;
    validate_decision(input.decision)?;
    require_ref(input.requester_ref, "retention admission requester ref")?;
    require_ref(input.object_ref, "retention admission object ref")?;
    validate_name(input.object_kind, "retention admission object kind")?;
    validate_class(input.retention_class)?;
    validate_action(input.action)?;
    validate_refs(input.bound_refs, "retention admission bound ref")?;
    validate_refs(input.retained_refs, "retention admission retained ref")?;
    validate_refs(input.remote_refs, "retention admission remote ref")?;
    validate_refs(input.revoked_refs, "retention admission revoked ref")?;
    ensure_count_at_most(input.diagnostics.len(), MAX_RETENTION_DIAGNOSTICS, "retention admission diagnostics")
}

fn validate_remote_gc_clearance_input(input: &RemoteGcClearanceInput<'_>) -> Result<()> {
    validate_decision(input.decision)?;
    require_ref(input.requester_ref, "retention remote clearance requester ref")?;
    require_ref(input.peer_ref, "retention remote clearance peer ref")?;
    require_ref(input.object_ref, "retention remote clearance object ref")?;
    validate_name(input.object_kind, "retention remote clearance object kind")?;
    validate_class(input.retention_class)?;
    validate_action(input.action)?;
    require_ref(input.remote_ref, "retention remote clearance remote ref")?;
    require_ref(input.policy_ref, "retention remote clearance policy ref")?;
    require_ref(input.authority_ref, "retention remote clearance authority ref")?;
    validate_refs(input.evidence_refs, "retention remote clearance evidence ref")?;
    validate_refs(input.retained_refs, "retention remote clearance retained ref")?;
    validate_refs(input.revoked_refs, "retention remote clearance revoked ref")?;
    ensure_count_at_most(input.diagnostics.len(), MAX_RETENTION_DIAGNOSTICS, "retention remote clearance diagnostics")
}

fn validate_remote_gc_clearance_request_input(input: &RemoteGcClearanceRequestInput<'_>) -> Result<()> {
    require_ref(input.requester_ref, "retention remote clearance request requester ref")?;
    require_ref(input.peer_ref, "retention remote clearance request peer ref")?;
    require_ref(input.object_ref, "retention remote clearance request object ref")?;
    validate_name(input.object_kind, "retention remote clearance request object kind")?;
    validate_class(input.retention_class)?;
    validate_action(input.action)?;
    require_ref(input.remote_ref, "retention remote clearance request remote ref")?;
    require_ref(input.policy_ref, "retention remote clearance request policy ref")?;
    require_ref(input.authority_ref, "retention remote clearance request authority ref")?;
    validate_refs(input.evidence_refs, "retention remote clearance request evidence ref")
}

fn validate_remote_gc_clearance_request(request: &RemoteGcClearanceRequest) -> Result<()> {
    validate_remote_gc_clearance_request_input(&RemoteGcClearanceRequestInput {
        requester_ref: &request.requester_ref,
        peer_ref: &request.peer_ref,
        object_ref: &request.object_ref,
        object_kind: &request.object_kind,
        retention_class: &request.retention_class,
        action: &request.action,
        remote_ref: &request.remote_ref,
        policy_ref: &request.policy_ref,
        authority_ref: &request.authority_ref,
        evidence_refs: &request.evidence_refs,
    })
}

fn validate_remote_gc_clearance_live_loopback_input(input: &RemoteGcClearanceLiveLoopbackInput<'_>) -> Result<()> {
    validate_remote_gc_clearance_request_input(&RemoteGcClearanceRequestInput {
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
    validate_refs(input.response_evidence_refs, "retention live response evidence ref")?;
    validate_refs(input.retained_refs, "retention live retained ref")?;
    validate_refs(input.revoked_refs, "retention live revoked ref")?;
    ensure_count_at_most(
        input.response_diagnostics.len(),
        MAX_RETENTION_DIAGNOSTICS,
        "retention live response diagnostics",
    )?;
    validate_name(input.requester_node_id, "retention live requester node id")?;
    validate_name(input.peer_node_id, "retention live peer node id")?;
    validate_name(input.topic, "retention live topic")?;
    validate_refs(input.request_peer_bootstrap_refs, "retention live request peer bootstrap ref")?;
    validate_refs(input.request_authority_refs, "retention live request authority ref")?;
    validate_refs(input.request_policy_refs, "retention live request policy ref")?;
    validate_refs(input.request_resource_refs, "retention live request resource ref")?;
    validate_refs(input.request_transport_evidence_refs, "retention live request evidence ref")?;
    validate_refs(input.response_peer_bootstrap_refs, "retention live response peer bootstrap ref")?;
    validate_refs(input.response_authority_refs, "retention live response authority ref")?;
    validate_refs(input.response_policy_refs, "retention live response policy ref")?;
    validate_refs(input.response_resource_refs, "retention live response resource ref")?;
    validate_refs(input.response_transport_evidence_refs, "retention live response evidence ref")?;
    Ok(())
}

fn validate_remote_gc_clearance_live_request_send_input(
    input: &RemoteGcClearanceLiveRequestSendInput<'_>,
) -> Result<()> {
    validate_remote_gc_clearance_request_input(&RemoteGcClearanceRequestInput {
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
    validate_name(input.requester_node_id, "retention live requester node id")?;
    validate_name(input.peer_node_id, "retention live peer node id")?;
    validate_name(input.topic, "retention live topic")?;
    validate_refs(input.peer_bootstrap_refs, "retention live peer bootstrap ref")?;
    validate_refs(input.authority_refs, "retention live authority ref")?;
    validate_refs(input.policy_refs, "retention live policy ref")?;
    validate_refs(input.resource_refs, "retention live resource ref")?;
    validate_refs(input.transport_evidence_refs, "retention live transport evidence ref")
}

fn validate_remote_gc_clearance_live_response_send_input(
    input: &RemoteGcClearanceLiveResponseSendInput<'_>,
) -> Result<()> {
    parse_remote_gc_clearance_request(input.request_value)?;
    validate_refs(input.response_evidence_refs, "retention live response evidence ref")?;
    validate_refs(input.retained_refs, "retention live retained ref")?;
    validate_refs(input.revoked_refs, "retention live revoked ref")?;
    ensure_count_at_most(
        input.response_diagnostics.len(),
        MAX_RETENTION_DIAGNOSTICS,
        "retention live response diagnostics",
    )?;
    validate_name(input.peer_node_id, "retention live peer node id")?;
    validate_name(input.requester_node_id, "retention live requester node id")?;
    validate_name(input.topic, "retention live topic")?;
    validate_refs(input.peer_bootstrap_refs, "retention live response peer bootstrap ref")?;
    validate_refs(input.authority_refs, "retention live response authority ref")?;
    validate_refs(input.policy_refs, "retention live response policy ref")?;
    validate_refs(input.resource_refs, "retention live response resource ref")?;
    validate_refs(input.transport_evidence_refs, "retention live response transport evidence ref")
}

fn validate_remote_gc_clearance_live_import_workflow_input(
    input: &RemoteGcClearanceLiveImportWorkflowInput<'_>,
) -> Result<()> {
    require_ref(input.request_ingress_ref, "retention live request ingress ref")?;
    require_ref(input.response_ingress_ref, "retention live response ingress ref")?;
    if let Some(peer_ref) = input.expected_peer_ref {
        require_ref(peer_ref, "retention live expected peer ref")?;
    }
    if let Some(remote_ref) = input.expected_remote_ref {
        require_ref(remote_ref, "retention live expected remote ref")?;
    }
    Ok(())
}

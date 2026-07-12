
pub fn store_remote_gc_clearance_response(
    input: RemoteGcClearanceResponseInput<'_>,
) -> Result<RemoteGcClearanceResponse> {
    let root = open_capability_retention_root(input.root)?;
    store_remote_gc_clearance_response_with_root(RemoteGcClearanceResponseInput {
        root: &root,
        request_value: input.request_value,
        evidence_refs: input.evidence_refs,
        retained_refs: input.retained_refs,
        is_current: input.is_current,
        revoked_refs: input.revoked_refs,
        diagnostics: input.diagnostics,
    })
}

pub fn store_remote_gc_clearance_response_with_root(
    input: RemoteGcClearanceResponseInput<'_, CapabilityRetentionRoot>,
) -> Result<RemoteGcClearanceResponse> {
    ensure_store_with_root(input.root)?;
    let request = parse_remote_gc_clearance_request(input.request_value)?;
    let diagnostics = remote_clearance_response_diagnostics(&input)?;
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let mut clearance_evidence_refs = request.evidence_refs.clone();
    for reference in input.evidence_refs {
        push_bounded(
            &mut clearance_evidence_refs,
            reference.clone(),
            MAX_RETENTION_REFS,
            "retention remote clearance response evidence refs",
        )?;
    }
    let clearance_value = remote_gc_clearance_value(&RemoteGcClearanceInput {
        decision,
        requester_ref: &request.requester_ref,
        peer_ref: &request.peer_ref,
        object_ref: &request.object_ref,
        object_kind: &request.object_kind,
        retention_class: &request.retention_class,
        action: &request.action,
        remote_ref: &request.remote_ref,
        policy_ref: &request.policy_ref,
        authority_ref: &request.authority_ref,
        evidence_refs: &clearance_evidence_refs,
        retained_refs: input.retained_refs,
        is_current: input.is_current,
        revoked_refs: input.revoked_refs,
        diagnostics: &diagnostics,
    })?;
    let clearance = parse_remote_gc_clearance(&clearance_value)?;
    let value = remote_gc_clearance_response_value(&request, &clearance, decision, &diagnostics)?;
    let response = parse_remote_gc_clearance_response(&value)?;
    write_store_value_with_root(
        input.root,
        &capability_ref_path(REMOTE_CLEARANCE_RESPONSE_DIR, &response.response_ref)?,
        &response.value,
    )?;
    Ok(response)
}

pub fn remote_gc_clearance_response_value(
    request: &RemoteGcClearanceRequest,
    clearance: &RemoteGcClearance,
    decision: &str,
    diagnostics: &[String],
) -> Result<IoValue> {
    validate_decision(decision)?;
    ensure_count_at_most(
        diagnostics.len(),
        MAX_RETENTION_DIAGNOSTICS,
        "retention remote clearance response diagnostics",
    )?;
    validate_remote_gc_clearance_workflow_scope(request, clearance)?;
    Ok(crate::preserves_rail::record("retention-remote-gc-clearance-response-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::RETENTION_REMOTE_GC_CLEARANCE_RESPONSE_SCHEMA),
        crate::preserves_rail::record("request", vec![
            crate::preserves_rail::string(&request.request_ref),
            request.value.clone(),
        ]),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(decision)]),
        crate::preserves_rail::record("clearance", vec![
            crate::preserves_rail::string(&clearance.clearance_ref),
            clearance.value.clone(),
        ]),
        crate::preserves_rail::record("diagnostics", vec![crate::preserves_rail::sequence(
            diagnostics.iter().map(crate::preserves_rail::string).collect(),
        )]),
        checks_value(&[
            ("request-ref-verified", "pass"),
            ("clearance-ref-verified", "pass"),
            ("clearance-scope-bound", pass_or_deny(decision == clearance.decision)),
        ]),
    ]))
}

pub fn parse_remote_gc_clearance_response(value: &IoValue) -> Result<RemoteGcClearanceResponse> {
    let fields = value
        .collect_simple_record("retention-remote-gc-clearance-response-v1", Some(6))
        .ok_or_else(|| MoltenError::invalid_harness("expected <retention-remote-gc-clearance-response-v1 ...>"))?;
    require_schema(
        &fields[0],
        crate::preserves_rail::RETENTION_REMOTE_GC_CLEARANCE_RESPONSE_SCHEMA,
        "retention remote clearance response schema",
    )?;
    let request = parse_embedded_remote_clearance_request(&fields[1])?;
    let decision = record_string(&fields[2], "decision")?;
    validate_decision(&decision)?;
    let clearance = parse_embedded_remote_clearance(&fields[3])?;
    let diagnostics = record_string_sequence(&fields[4], "diagnostics")?;
    let checks = parse_checks(&fields[5])?;
    require_check(&checks, "request-ref-verified", "retention remote clearance response")?;
    require_check(&checks, "clearance-ref-verified", "retention remote clearance response")?;
    if decision != clearance.decision {
        return Err(MoltenError::invalid_harness("remote clearance response decision does not match clearance"));
    }
    validate_remote_gc_clearance_workflow_scope(&request, &clearance)?;
    Ok(RemoteGcClearanceResponse {
        response_ref: crate::preserves_rail::canonical_hash(value)?,
        decision,
        request_ref: request.request_ref.clone(),
        request,
        clearance_ref: clearance.clearance_ref.clone(),
        clearance,
        diagnostics,
        value: value.clone(),
    })
}

pub fn import_remote_gc_clearance_response(input: RemoteGcClearanceImportInput<'_>) -> Result<RemoteGcClearanceImport> {
    let root = open_capability_retention_root(input.root)?;
    import_remote_gc_clearance_response_with_root(RemoteGcClearanceImportInput {
        root: &root,
        request_value: input.request_value,
        response_value: input.response_value,
        expected_peer_ref: input.expected_peer_ref,
        expected_remote_ref: input.expected_remote_ref,
    })
}

pub fn import_remote_gc_clearance_response_with_root(
    input: RemoteGcClearanceImportInput<'_, CapabilityRetentionRoot>,
) -> Result<RemoteGcClearanceImport> {
    ensure_store_with_root(input.root)?;
    if let Some(peer_ref) = input.expected_peer_ref {
        require_ref(peer_ref, "retention remote clearance import expected peer ref")?;
    }
    if let Some(remote_ref) = input.expected_remote_ref {
        require_ref(remote_ref, "retention remote clearance import expected remote ref")?;
    }
    let request = parse_remote_gc_clearance_request(input.request_value)?;
    let response = match parse_remote_gc_clearance_response(input.response_value) {
        Ok(response) => response,
        Err(error) => {
            let diagnostics = vec![format!("remote-clearance-tampered-response:{error}")];
            let response_ref = crate::preserves_rail::canonical_hash(input.response_value)?;
            let value = remote_gc_clearance_import_value(&RemoteGcClearanceImportValueInput {
                decision: "deny",
                request_ref: &request.request_ref,
                response_ref: &response_ref,
                clearance_ref: None,
                peer_ref: &request.peer_ref,
                remote_ref: &request.remote_ref,
                diagnostics: &diagnostics,
            })?;
            let import = parse_remote_gc_clearance_import(&value)?;
            write_store_value_with_root(
                input.root,
                &capability_ref_path(REMOTE_CLEARANCE_IMPORT_DIR, &import.import_ref)?,
                &import.value,
            )?;
            return Ok(import);
        }
    };
    let mut diagnostics = Vec::new();
    push_remote_clearance_import_diagnostics(&mut diagnostics, &request, &response, &input)?;
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let clearance_ref = if decision == "pass" {
        write_store_value_with_root(
            input.root,
            &capability_ref_path(REMOTE_CLEARANCE_DIR, &response.clearance.clearance_ref)?,
            &response.clearance.value,
        )?;
        Some(response.clearance.clearance_ref.clone())
    } else {
        None
    };
    let value = remote_gc_clearance_import_value(&RemoteGcClearanceImportValueInput {
        decision,
        request_ref: &request.request_ref,
        response_ref: &response.response_ref,
        clearance_ref: clearance_ref.as_deref(),
        peer_ref: &request.peer_ref,
        remote_ref: &request.remote_ref,
        diagnostics: &diagnostics,
    })?;
    let import = parse_remote_gc_clearance_import(&value)?;
    write_store_value_with_root(
        input.root,
        &capability_ref_path(REMOTE_CLEARANCE_IMPORT_DIR, &import.import_ref)?,
        &import.value,
    )?;
    Ok(import)
}

pub fn remote_gc_clearance_import_value(input: &RemoteGcClearanceImportValueInput<'_>) -> Result<IoValue> {
    validate_decision(input.decision)?;
    require_ref(input.request_ref, "retention remote clearance import request ref")?;
    require_ref(input.response_ref, "retention remote clearance import response ref")?;
    if let Some(reference) = input.clearance_ref {
        require_ref(reference, "retention remote clearance import clearance ref")?;
    }
    require_ref(input.peer_ref, "retention remote clearance import peer ref")?;
    require_ref(input.remote_ref, "retention remote clearance import remote ref")?;
    ensure_count_at_most(
        input.diagnostics.len(),
        MAX_RETENTION_DIAGNOSTICS,
        "retention remote clearance import diagnostics",
    )?;
    Ok(crate::preserves_rail::record("retention-remote-gc-clearance-import-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::RETENTION_REMOTE_GC_CLEARANCE_IMPORT_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(input.decision)]),
        crate::preserves_rail::record("request", vec![crate::preserves_rail::string(input.request_ref)]),
        crate::preserves_rail::record("response", vec![crate::preserves_rail::string(input.response_ref)]),
        crate::preserves_rail::record("clearance", vec![optional_ref_value(input.clearance_ref)]),
        crate::preserves_rail::record("peer", vec![crate::preserves_rail::string(input.peer_ref)]),
        crate::preserves_rail::record("remote", vec![crate::preserves_rail::string(input.remote_ref)]),
        crate::preserves_rail::record("diagnostics", vec![crate::preserves_rail::sequence(
            input.diagnostics.iter().map(crate::preserves_rail::string).collect(),
        )]),
        checks_value(&[
            ("evidence-only", "pass"),
            ("local-clearance-stored", pass_or_deny(input.clearance_ref.is_some())),
        ]),
    ]))
}

pub fn parse_remote_gc_clearance_import(value: &IoValue) -> Result<RemoteGcClearanceImport> {
    let fields = value
        .collect_simple_record("retention-remote-gc-clearance-import-v1", Some(9))
        .ok_or_else(|| MoltenError::invalid_harness("expected <retention-remote-gc-clearance-import-v1 ...>"))?;
    require_schema(
        &fields[0],
        crate::preserves_rail::RETENTION_REMOTE_GC_CLEARANCE_IMPORT_SCHEMA,
        "retention remote clearance import schema",
    )?;
    require_check(&parse_checks(&fields[8])?, "evidence-only", "retention remote clearance import")?;
    let decision = record_string(&fields[1], "decision")?;
    validate_decision(&decision)?;
    let request_ref = record_ref(&fields[2], "request")?;
    let response_ref = record_ref(&fields[3], "response")?;
    let clearance_ref = record_optional_ref(&fields[4], "clearance")?;
    let peer_ref = record_ref(&fields[5], "peer")?;
    let remote_ref = record_ref(&fields[6], "remote")?;
    let diagnostics = record_string_sequence(&fields[7], "diagnostics")?;
    Ok(RemoteGcClearanceImport {
        import_ref: crate::preserves_rail::canonical_hash(value)?,
        decision,
        request_ref,
        response_ref,
        clearance_ref,
        peer_ref,
        remote_ref,
        diagnostics,
        value: value.clone(),
    })
}

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

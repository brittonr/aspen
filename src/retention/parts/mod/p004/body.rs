
pub fn evaluate(input: EvaluationInput<'_>) -> Result<Evaluation> {
    ensure_store(input.root)?;
    validate_class(input.retention_class)?;
    validate_action(input.action)?;
    require_ref(input.object_ref, "retention object ref")?;
    validate_name(input.object_kind, "retention object kind")?;
    require_ref(input.requester_ref, "retention requester ref")?;
    validate_refs(input.policy_refs, "retention policy ref")?;
    validate_refs(input.evidence_refs, "retention evidence ref")?;
    validate_refs(input.retained_refs, "retention retained ref")?;
    validate_refs(input.remote_refs, "retention remote ref")?;
    let index = reference_index_for_object(ReferenceIndexForObjectInput {
        root: input.root,
        object_ref: input.object_ref,
        object_kind: input.object_kind,
        retained_refs: input.retained_refs,
        remote_refs: input.remote_refs,
        is_complete: input.is_reference_index_complete,
    })?;
    let diagnostics = evaluation_diagnostics(&input, &index)?;
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let receipt = build_receipt(ReceiptBuildInput {
        decision,
        action: input.action,
        object_ref: input.object_ref,
        object_kind: input.object_kind,
        retention_class: input.retention_class,
        requester_ref: input.requester_ref,
        index_ref: &index.index_ref,
        pin_refs: &index.pin_refs,
        retained_refs: input.retained_refs,
        remote_refs: input.remote_refs,
        policy_refs: input.policy_refs,
        evidence_refs: input.evidence_refs,
        tombstone_ref: None,
        diagnostics: &diagnostics,
    })?;
    write_store_value(&receipt_path(input.root, &receipt.receipt_ref)?, &receipt.value)?;
    if decision == "pass" && is_destructive_action(input.action) {
        let tombstone = build_tombstone(TombstoneBuildInput {
            object_ref: input.object_ref,
            object_kind: input.object_kind,
            retention_class: input.retention_class,
            action: input.action,
            receipt_ref: &receipt.receipt_ref,
            policy_refs: input.policy_refs,
            evidence_refs: input.evidence_refs,
        })?;
        write_store_value(&tombstone_path(input.root, &tombstone.tombstone_ref)?, &tombstone.value)?;
        return Ok(Evaluation {
            receipt,
            index,
            tombstone: Some(tombstone),
        });
    }
    Ok(Evaluation {
        receipt,
        index,
        tombstone: None,
    })
}

pub fn evidence_admission_value(input: &EvidenceAdmissionInput<'_>) -> Result<IoValue> {
    validate_evidence_admission_input(input)?;
    Ok(crate::preserves_rail::record("retention-evidence-admission-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::RETENTION_EVIDENCE_ADMISSION_SCHEMA),
        crate::preserves_rail::record("kind", vec![crate::preserves_rail::string(input.kind)]),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(input.decision)]),
        crate::preserves_rail::record("requester", vec![crate::preserves_rail::string(input.requester_ref)]),
        object_value(input.object_ref, input.object_kind),
        crate::preserves_rail::record("class", vec![crate::preserves_rail::string(input.retention_class)]),
        crate::preserves_rail::record("action", vec![crate::preserves_rail::string(input.action)]),
        crate::preserves_rail::record("bound", vec![strings_sequence(input.bound_refs)]),
        crate::preserves_rail::record("retained", vec![strings_sequence(input.retained_refs)]),
        crate::preserves_rail::record("remote", vec![strings_sequence(input.remote_refs)]),
        crate::preserves_rail::record("reference-index-complete", vec![crate::preserves_rail::string(pass_or_deny(
            input.is_reference_index_complete,
        ))]),
        crate::preserves_rail::record("current", vec![crate::preserves_rail::string(pass_or_deny(input.is_current))]),
        crate::preserves_rail::record("revoked", vec![strings_sequence(input.revoked_refs)]),
        crate::preserves_rail::record("diagnostics", vec![strings_sequence(input.diagnostics)]),
        checks_value(&[
            ("canonical-ref-binding", "pass"),
            ("scope-bound", "pass"),
            ("typed-admission", "pass"),
            ("non-authority-evidence-separated", "pass"),
        ]),
    ]))
}

pub fn parse_evidence_admission(value: &IoValue) -> Result<EvidenceAdmission> {
    let fields = value
        .collect_simple_record("retention-evidence-admission-v1", Some(15))
        .ok_or_else(|| MoltenError::invalid_harness("expected <retention-evidence-admission-v1 ...>"))?;
    require_schema(
        &fields[0],
        crate::preserves_rail::RETENTION_EVIDENCE_ADMISSION_SCHEMA,
        "retention evidence admission schema",
    )?;
    let kind = record_string(&fields[1], "kind")?;
    validate_admission_kind(&kind)?;
    let decision = record_string(&fields[2], "decision")?;
    validate_decision(&decision)?;
    let requester_ref = record_ref(&fields[3], "requester")?;
    let (object_ref, object_kind) = parse_object_value(&fields[4])?;
    let retention_class = record_string(&fields[5], "class")?;
    validate_class(&retention_class)?;
    let action = record_string(&fields[6], "action")?;
    validate_action(&action)?;
    let bound_refs = record_ref_sequence(&fields[7], "bound")?;
    let retained_refs = record_ref_sequence(&fields[8], "retained")?;
    let remote_refs = record_ref_sequence(&fields[9], "remote")?;
    let is_reference_index_complete = record_pass_bool(&fields[10], "reference-index-complete")?;
    let is_current = record_pass_bool(&fields[11], "current")?;
    let revoked_refs = record_ref_sequence(&fields[12], "revoked")?;
    let diagnostics = record_string_sequence(&fields[13], "diagnostics")?;
    require_check(&parse_checks(&fields[14])?, "typed-admission", "retention evidence admission")?;
    Ok(EvidenceAdmission {
        admission_ref: crate::preserves_rail::canonical_hash(value)?,
        kind,
        decision,
        requester_ref,
        object_ref,
        object_kind,
        retention_class,
        action,
        bound_refs,
        retained_refs,
        remote_refs,
        is_reference_index_complete,
        is_current,
        revoked_refs,
        diagnostics,
        value: value.clone(),
    })
}

pub fn store_evidence_admission(root: &Path, input: &EvidenceAdmissionInput<'_>) -> Result<EvidenceAdmission> {
    ensure_store(root)?;
    let value = evidence_admission_value(input)?;
    let admission = parse_evidence_admission(&value)?;
    write_store_value(&admission_path(root, &admission.admission_ref)?, &admission.value)?;
    Ok(admission)
}

pub fn remote_gc_clearance_value(input: &RemoteGcClearanceInput<'_>) -> Result<IoValue> {
    validate_remote_gc_clearance_input(input)?;
    Ok(crate::preserves_rail::record("retention-remote-gc-clearance-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::RETENTION_REMOTE_GC_CLEARANCE_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(input.decision)]),
        crate::preserves_rail::record("requester", vec![crate::preserves_rail::string(input.requester_ref)]),
        crate::preserves_rail::record("peer", vec![crate::preserves_rail::string(input.peer_ref)]),
        object_value(input.object_ref, input.object_kind),
        crate::preserves_rail::record("class", vec![crate::preserves_rail::string(input.retention_class)]),
        crate::preserves_rail::record("action", vec![crate::preserves_rail::string(input.action)]),
        crate::preserves_rail::record("remote", vec![crate::preserves_rail::string(input.remote_ref)]),
        crate::preserves_rail::record("policy", vec![crate::preserves_rail::string(input.policy_ref)]),
        crate::preserves_rail::record("authority", vec![crate::preserves_rail::string(input.authority_ref)]),
        crate::preserves_rail::record("evidence", vec![strings_sequence(input.evidence_refs)]),
        crate::preserves_rail::record("retained", vec![strings_sequence(input.retained_refs)]),
        crate::preserves_rail::record("current", vec![crate::preserves_rail::string(pass_or_deny(input.is_current))]),
        crate::preserves_rail::record("revoked", vec![strings_sequence(input.revoked_refs)]),
        crate::preserves_rail::record("diagnostics", vec![strings_sequence(input.diagnostics)]),
        checks_value(&[
            ("canonical-ref-binding", "pass"),
            ("peer-bound", "pass"),
            ("scope-bound", "pass"),
            ("remote-ref-bound", "pass"),
            ("non-authority-evidence-separated", "pass"),
        ]),
    ]))
}

pub fn parse_remote_gc_clearance(value: &IoValue) -> Result<RemoteGcClearance> {
    let fields = value
        .collect_simple_record("retention-remote-gc-clearance-v1", Some(16))
        .ok_or_else(|| MoltenError::invalid_harness("expected <retention-remote-gc-clearance-v1 ...>"))?;
    require_schema(
        &fields[0],
        crate::preserves_rail::RETENTION_REMOTE_GC_CLEARANCE_SCHEMA,
        "retention remote GC clearance schema",
    )?;
    let decision = record_string(&fields[1], "decision")?;
    validate_decision(&decision)?;
    let requester_ref = record_ref(&fields[2], "requester")?;
    let peer_ref = record_ref(&fields[3], "peer")?;
    let (object_ref, object_kind) = parse_object_value(&fields[4])?;
    let retention_class = record_string(&fields[5], "class")?;
    validate_class(&retention_class)?;
    let action = record_string(&fields[6], "action")?;
    validate_action(&action)?;
    let remote_ref = record_ref(&fields[7], "remote")?;
    let policy_ref = record_ref(&fields[8], "policy")?;
    let authority_ref = record_ref(&fields[9], "authority")?;
    let evidence_refs = record_ref_sequence(&fields[10], "evidence")?;
    let retained_refs = record_ref_sequence(&fields[11], "retained")?;
    let is_current = record_pass_bool(&fields[12], "current")?;
    let revoked_refs = record_ref_sequence(&fields[13], "revoked")?;
    let diagnostics = record_string_sequence(&fields[14], "diagnostics")?;
    require_check(&parse_checks(&fields[15])?, "peer-bound", "retention remote GC clearance")?;
    Ok(RemoteGcClearance {
        clearance_ref: crate::preserves_rail::canonical_hash(value)?,
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
        is_current,
        revoked_refs,
        diagnostics,
        value: value.clone(),
    })
}

pub fn store_remote_gc_clearance(root: &Path, input: &RemoteGcClearanceInput<'_>) -> Result<RemoteGcClearance> {
    ensure_store(root)?;
    let value = remote_gc_clearance_value(input)?;
    let clearance = parse_remote_gc_clearance(&value)?;
    write_store_value(&remote_clearance_path(root, &clearance.clearance_ref)?, &clearance.value)?;
    Ok(clearance)
}

pub fn remote_gc_clearance_request_value(input: &RemoteGcClearanceRequestInput<'_>) -> Result<IoValue> {
    validate_remote_gc_clearance_request_input(input)?;
    Ok(crate::preserves_rail::record("retention-remote-gc-clearance-request-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::RETENTION_REMOTE_GC_CLEARANCE_REQUEST_SCHEMA),
        crate::preserves_rail::record("requester", vec![crate::preserves_rail::string(input.requester_ref)]),
        crate::preserves_rail::record("peer", vec![crate::preserves_rail::string(input.peer_ref)]),
        object_value(input.object_ref, input.object_kind),
        crate::preserves_rail::record("class", vec![crate::preserves_rail::string(input.retention_class)]),
        crate::preserves_rail::record("action", vec![crate::preserves_rail::string(input.action)]),
        crate::preserves_rail::record("remote", vec![crate::preserves_rail::string(input.remote_ref)]),
        crate::preserves_rail::record("policy", vec![crate::preserves_rail::string(input.policy_ref)]),
        crate::preserves_rail::record("authority", vec![crate::preserves_rail::string(input.authority_ref)]),
        crate::preserves_rail::record("evidence", vec![strings_sequence(input.evidence_refs)]),
        checks_value(&[("request-scope-bound", "pass"), ("peer-bound", "pass")]),
    ]))
}

pub fn parse_remote_gc_clearance_request(value: &IoValue) -> Result<RemoteGcClearanceRequest> {
    let fields = value
        .collect_simple_record("retention-remote-gc-clearance-request-v1", Some(11))
        .ok_or_else(|| MoltenError::invalid_harness("expected <retention-remote-gc-clearance-request-v1 ...>"))?;
    require_schema(
        &fields[0],
        crate::preserves_rail::RETENTION_REMOTE_GC_CLEARANCE_REQUEST_SCHEMA,
        "retention remote clearance request schema",
    )?;
    require_check(&parse_checks(&fields[10])?, "request-scope-bound", "retention remote clearance request")?;
    let (object_ref, object_kind) = parse_object_value(&fields[3])?;
    let request = RemoteGcClearanceRequest {
        request_ref: crate::preserves_rail::canonical_hash(value)?,
        requester_ref: record_ref(&fields[1], "requester")?,
        peer_ref: record_ref(&fields[2], "peer")?,
        object_ref,
        object_kind,
        retention_class: record_string(&fields[4], "class")?,
        action: record_string(&fields[5], "action")?,
        remote_ref: record_ref(&fields[6], "remote")?,
        policy_ref: record_ref(&fields[7], "policy")?,
        authority_ref: record_ref(&fields[8], "authority")?,
        evidence_refs: record_ref_sequence(&fields[9], "evidence")?,
        value: value.clone(),
    };
    validate_remote_gc_clearance_request(&request)?;
    Ok(request)
}

pub fn store_remote_gc_clearance_request(
    root: &Path,
    input: &RemoteGcClearanceRequestInput<'_>,
) -> Result<RemoteGcClearanceRequest> {
    ensure_store(root)?;
    let value = remote_gc_clearance_request_value(input)?;
    let request = parse_remote_gc_clearance_request(&value)?;
    write_store_value(&remote_clearance_request_path(root, &request.request_ref)?, &request.value)?;
    Ok(request)
}

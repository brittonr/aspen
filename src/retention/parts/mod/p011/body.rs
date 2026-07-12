
fn parse_plan_gate(value: &IoValue) -> Result<PlanGate> {
    let fields = value
        .collect_simple_record("gate", Some(5))
        .ok_or_else(|| MoltenError::invalid_harness("expected retention GC plan gate"))?;
    let name = record_string(&fields[0], "name")?;
    validate_name(&name, "retention GC plan gate name")?;
    let decision = record_string(&fields[1], "decision")?;
    validate_decision(&decision)?;
    let required_refs = record_ref_sequence(&fields[2], "required")?;
    let admitted_refs = record_ref_sequence(&fields[3], "admitted")?;
    let diagnostics = record_string_sequence(&fields[4], "diagnostics")?;
    Ok(PlanGate {
        name,
        decision,
        required_refs,
        admitted_refs,
        diagnostics,
    })
}

fn parse_embedded_reference_index(value: &Value<IoValue>) -> Result<(String, ReferenceIndex)> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let fields = value
        .collect_simple_record("index", Some(2))
        .ok_or_else(|| MoltenError::invalid_harness("expected embedded retention index"))?;
    let index_ref = required_string(&fields[0], "embedded retention index ref")?;
    require_ref(&index_ref, "embedded retention index ref")?;
    let index_value = crate::preserves_rail::value_to_iovalue(&fields[1]);
    let index = parse_reference_index(&index_value)?;
    if index.index_ref != index_ref {
        return Err(MoltenError::invalid_harness("embedded retention index ref mismatch"));
    }
    Ok((index_ref, index))
}

fn parse_embedded_destructive_evidence_summary(value: &Value<IoValue>) -> Result<IoValue> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let fields = value
        .collect_simple_record("retention-evidence", Some(1))
        .ok_or_else(|| MoltenError::invalid_harness("expected embedded retention evidence summary"))?;
    parse_destructive_evidence_summary(&crate::preserves_rail::value_to_iovalue(&fields[0]))
}

fn parse_destructive_evidence_summary(value: &IoValue) -> Result<IoValue> {
    parse_destructive_evidence_summary_to_evidence(value)?;
    Ok(value.clone())
}

fn parse_destructive_evidence_summary_to_evidence(value: &IoValue) -> Result<DestructiveEvidence> {
    let fields = value
        .collect_simple_record("retention-evidence-summary-v1", Some(12))
        .ok_or_else(|| MoltenError::invalid_harness("expected <retention-evidence-summary-v1 ...>"))?;
    let requester_fields = fields[0]
        .collect_simple_record("requester", Some(1))
        .ok_or_else(|| MoltenError::invalid_harness("expected retention evidence requester"))?;
    let requester_value = crate::preserves_rail::value_to_iovalue(&requester_fields[0]);
    let requester_ref = if requester_value.collect_simple_record("none", Some(0)).is_some() {
        None
    } else {
        let requester_ref = required_string(&requester_fields[0], "retention evidence requester")?;
        require_ref(&requester_ref, "retention evidence requester")?;
        Some(requester_ref)
    };
    let evidence = DestructiveEvidence {
        requester_ref,
        policy_refs: record_ref_sequence(&fields[1], "policy")?,
        authority_refs: record_ref_sequence(&fields[2], "authority")?,
        evidence_refs: record_ref_sequence(&fields[3], "evidence")?,
        retained_refs: record_ref_sequence(&fields[4], "retained")?,
        remote_peer_refs: record_ref_sequence(&fields[5], "remote-peer")?,
        remote_refs: record_ref_sequence(&fields[6], "remote")?,
        reference_index_refs: record_ref_sequence(&fields[7], "reference-index")?,
        remote_gc_refs: record_ref_sequence(&fields[8], "remote-gc")?,
        remote_clearance_refs: record_ref_sequence(&fields[9], "remote-clearance")?,
        is_reference_index_complete: record_pass_bool(&fields[10], "reference-index-complete")?,
    };
    parse_checks(&fields[11])?;
    validate_destructive_evidence(&evidence)?;
    Ok(evidence)
}

struct AdmissionCheck {
    is_admitted: bool,
    scope_mismatches: usize,
}

fn push_admission_diagnostic<S>(diagnostics: &mut S, diagnostic: String) -> Result<()>
where S: VecSink<String> {
    push_bounded(diagnostics, diagnostic, MAX_RETENTION_DIAGNOSTICS, "retention admission diagnostics")
}

fn check_admission_basics<Root: ?Sized, S>(
    input: &AdmissionRefsInput<'_, Root>,
    reference: &str,
    admission: &EvidenceAdmission,
    diagnostics: &mut S,
) -> Result<bool>
where
    S: VecSink<String>,
{
    let mut is_admitted = true;
    if admission.admission_ref != reference {
        is_admitted = false;
        push_admission_diagnostic(
            diagnostics,
            format!("{}-admission-ref-mismatch:{}", input.expected_kind, reference),
        )?;
    }
    if admission.kind != input.expected_kind {
        is_admitted = false;
        push_admission_diagnostic(
            diagnostics,
            format!("{}-admission-kind-mismatch:{}", input.expected_kind, reference),
        )?;
    }
    if admission.decision != "pass" {
        is_admitted = false;
        push_admission_diagnostic(diagnostics, format!("{}-admission-not-pass:{}", input.expected_kind, reference))?;
    }
    if !admission.is_current {
        is_admitted = false;
        push_admission_diagnostic(diagnostics, format!("{}-admission-stale:{}", input.expected_kind, reference))?;
    }
    if !admission.revoked_refs.is_empty() {
        is_admitted = false;
        push_admission_diagnostic(diagnostics, format!("{}-admission-revoked:{}", input.expected_kind, reference))?;
    }
    if admission.bound_refs.is_empty() {
        is_admitted = false;
        push_admission_diagnostic(
            diagnostics,
            format!("{}-admission-empty-bound-refs:{}", input.expected_kind, reference),
        )?;
    }
    Ok(is_admitted)
}

fn admission_scope_mismatch_count(scope: &AdmissionScope<'_>, admission: &EvidenceAdmission) -> usize {
    let mut count = 0usize;
    if scope.requester_ref != Some(admission.requester_ref.as_str()) {
        count += 1;
    }
    if admission.object_ref != scope.object_ref || admission.object_kind != scope.object_kind {
        count += 1;
    }
    if admission.retention_class != scope.retention_class {
        count += 1;
    }
    if admission.action != scope.action {
        count += 1;
    }
    count
}

fn check_admission_required_refs<Root: ?Sized, S>(
    input: &AdmissionRefsInput<'_, Root>,
    reference: &str,
    admission: &EvidenceAdmission,
    diagnostics: &mut S,
) -> Result<bool>
where
    S: VecSink<String>,
{
    let mut is_admitted = true;
    if input.expected_kind == ADMISSION_KIND_REFERENCE_INDEX && !admission.is_reference_index_complete {
        is_admitted = false;
        push_admission_diagnostic(diagnostics, format!("reference-index-admission-incomplete:{}", reference))?;
    }
    if input.expected_kind == ADMISSION_KIND_REMOTE_GC {
        for required in input.required_remote_refs {
            if !admission.remote_refs.iter().any(|remote| remote == required) {
                is_admitted = false;
                push_admission_diagnostic(
                    diagnostics,
                    format!("remote-gc-admission-missing-remote:{}:{}", reference, required),
                )?;
            }
        }
    }
    Ok(is_admitted)
}

fn check_admission_ref<Root: ?Sized, S>(
    input: &AdmissionRefsInput<'_, Root>,
    reference: &str,
    admission: &EvidenceAdmission,
    diagnostics: &mut S,
) -> Result<AdmissionCheck>
where
    S: VecSink<String>,
{
    let mut is_admitted = check_admission_basics(input, reference, admission, diagnostics)?;
    let scope_mismatches = admission_scope_mismatch_count(input.scope, admission);
    if scope_mismatches > 0 {
        is_admitted = false;
    }
    if !check_admission_required_refs(input, reference, admission, diagnostics)? {
        is_admitted = false;
    }
    Ok(AdmissionCheck {
        is_admitted,
        scope_mismatches,
    })
}

fn admit_evidence_refs_with_root(
    input: AdmissionRefsInput<'_, CapabilityRetentionRoot>,
) -> Result<AdmissionRefsResult> {
    admit_evidence_refs_core(input, read_evidence_admission_with_root)
}

fn admit_evidence_refs_core<Root: ?Sized, ReadAdmission>(
    input: AdmissionRefsInput<'_, Root>,
    mut read_admission: ReadAdmission,
) -> Result<AdmissionRefsResult>
where
    ReadAdmission: FnMut(&Root, &str) -> Result<EvidenceAdmission>,
{
    let mut diagnostics = Vec::new();
    let mut admitted_refs = Vec::new();
    let mut remote_refs = Vec::new();
    let mut scope_mismatches = 0usize;
    for reference in input.refs {
        let admission = match read_admission(input.root, reference) {
            Ok(admission) => admission,
            Err(error) => {
                push_admission_diagnostic(
                    &mut diagnostics,
                    format!("{}-admission-unreadable:{}:{}", input.expected_kind, reference, error),
                )?;
                continue;
            }
        };
        let check = check_admission_ref(&input, reference, &admission, &mut diagnostics)?;
        scope_mismatches += check.scope_mismatches;
        if check.is_admitted {
            push_bounded(&mut admitted_refs, admission.admission_ref, MAX_RETENTION_REFS, "retention admitted refs")?;
            for remote_ref in admission.remote_refs {
                push_bounded(&mut remote_refs, remote_ref, MAX_RETENTION_REFS, "retention admitted remote refs")?;
            }
        }
    }
    if !input.refs.is_empty() && admitted_refs.is_empty() && scope_mismatches > 0 {
        push_admission_diagnostic(&mut diagnostics, format!("{}-admission-scope-mismatch", input.expected_kind))?;
    }
    Ok(AdmissionRefsResult {
        diagnostics,
        admitted_refs,
        remote_refs,
    })
}

struct Check {
    is_admitted: bool,
    scope_mismatches: usize,
}

fn push_clear_note<S>(diagnostics: &mut S, message: String) -> Result<()>
where S: VecSink<String> {
    push_bounded(diagnostics, message, MAX_RETENTION_DIAGNOSTICS, "retention remote clearance diagnostics")
}

fn check_state<S>(reference: &str, clearance: &RemoteGcClearance, diagnostics: &mut S) -> Result<bool>
where S: VecSink<String> {
    let mut is_admitted = true;
    if clearance.clearance_ref != *reference {
        is_admitted = false;
        push_clear_note(diagnostics, format!("remote-clearance-ref-mismatch:{}", reference))?;
    }
    if clearance.decision != "pass" {
        is_admitted = false;
        push_clear_note(diagnostics, format!("remote-clearance-not-pass:{}", reference))?;
    }
    if !clearance.is_current {
        is_admitted = false;
        push_clear_note(diagnostics, format!("remote-clearance-stale:{}", reference))?;
    }
    if !clearance.revoked_refs.is_empty() {
        is_admitted = false;
        push_clear_note(diagnostics, format!("remote-clearance-revoked:{}", reference))?;
    }
    if !clearance.retained_refs.is_empty() {
        is_admitted = false;
        push_clear_note(diagnostics, format!("remote-clearance-retained:{}", clearance.remote_ref))?;
    }
    Ok(is_admitted)
}

fn check_scope<Root: ?Sized>(input: &RemoteClearanceRefsInput<'_, Root>, clearance: &RemoteGcClearance) -> Check {
    let mut scope_mismatches = 0usize;
    if input.scope.requester_ref != Some(clearance.requester_ref.as_str()) {
        scope_mismatches += 1;
    }
    if clearance.object_ref != input.scope.object_ref || clearance.object_kind != input.scope.object_kind {
        scope_mismatches += 1;
    }
    if clearance.retention_class != input.scope.retention_class {
        scope_mismatches += 1;
    }
    if clearance.action != input.scope.action {
        scope_mismatches += 1;
    }
    Check {
        is_admitted: scope_mismatches == 0,
        scope_mismatches,
    }
}

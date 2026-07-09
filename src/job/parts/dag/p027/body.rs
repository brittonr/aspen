
pub fn admit_remote_execution(input: RemoteExecutionAdmissionInput) -> Result<RemoteExecutionAdmissionReceipt> {
    let request = parse_remote_execution_request(&input.request)?;
    let plan = parse_remote_execution_closure_plan(&input.closure_plan)?;
    validate_refs(&input.fetched_refs, "remote execution fetched ref")?;
    validate_refs(&input.verified_artifact_refs, "remote execution verified artifact ref")?;
    validate_refs(&input.admitted_capability_refs, "remote execution admitted capability ref")?;
    validate_ref(&input.handler_profile_admission_ref, "remote execution handler profile admission ref")?;
    validate_refs(&input.local_policy_refs, "remote execution local policy ref")?;
    validate_refs(&input.provenance_receipt_refs, "remote execution provenance receipt ref")?;
    validate_refs(&input.source_gate_receipt_refs, "remote execution source gate receipt ref")?;
    validate_refs(&input.resource_receipt_refs, "remote execution resource receipt ref")?;
    validate_refs(&input.evidence_refs, "remote execution admission evidence ref")?;
    let diagnostics = remote_execution_admission_diagnostics(&request, &plan, &input)?;
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let value = remote_execution_admission_receipt_value(&RemoteExecutionAdmissionValueInput {
        decision,
        request_ref: &request.request_ref,
        execution_id: &request.execution_id,
        root_artifact_ref: &request.root_artifact_ref,
        closure_plan_ref: &plan.plan_ref,
        fetched_refs: &input.fetched_refs,
        verified_artifact_refs: &input.verified_artifact_refs,
        handler_profile_admission_ref: &input.handler_profile_admission_ref,
        diagnostics: &diagnostics,
        evidence_refs: &input.evidence_refs,
    })?;
    Ok(RemoteExecutionAdmissionReceipt {
        receipt_ref: crate::preserves_rail::canonical_hash(&value)?,
        decision: decision.to_string(),
        request_ref: request.request_ref,
        execution_id: request.execution_id,
        root_artifact_ref: request.root_artifact_ref,
        closure_plan_ref: plan.plan_ref,
        fetched_refs: input.fetched_refs,
        verified_artifact_refs: input.verified_artifact_refs,
        handler_profile_admission_ref: input.handler_profile_admission_ref,
        diagnostics,
        evidence_refs: input.evidence_refs,
        value,
    })
}

pub fn parse_remote_execution_admission_receipt(value: &IoValue) -> Result<RemoteExecutionAdmissionReceipt> {
    let fields = simple_record(
        value,
        "remote-execution-admission-receipt-v1",
        REMOTE_EXECUTION_ADMISSION_RECEIPT_FIELDS + 1,
    )?;
    require_schema(
        &fields[0],
        REMOTE_EXECUTION_ADMISSION_RECEIPT_SCHEMA,
        "remote execution admission receipt schema",
    )?;
    let checks = parse_checks(&fields[12])?;
    require_check(&checks, "receiver-closure-complete", "remote execution admission receipt")?;
    Ok(RemoteExecutionAdmissionReceipt {
        receipt_ref: crate::preserves_rail::canonical_hash(value)?,
        decision: record_string(&fields[1], "decision")?,
        request_ref: record_ref(&fields[2], "request")?,
        execution_id: record_string(&fields[3], "execution")?,
        root_artifact_ref: record_ref(&fields[4], "root")?,
        closure_plan_ref: record_ref(&fields[5], "closure-plan")?,
        fetched_refs: record_ref_sequence(&fields[6], "fetched")?,
        verified_artifact_refs: record_ref_sequence(&fields[7], "verified")?,
        handler_profile_admission_ref: record_ref(&fields[8], "handler-profile-admission")?,
        diagnostics: record_string_sequence(&fields[9], "diagnostics")?,
        evidence_refs: record_ref_sequence(&fields[10], "evidence")?,
        value: value.clone(),
    })
}

fn remote_execution_admission_receipt_value(input: &RemoteExecutionAdmissionValueInput<'_>) -> Result<IoValue> {
    validate_worker_decision(input.decision)?;
    validate_ref(input.request_ref, "remote execution admission request ref")?;
    validate_non_empty(input.execution_id, "remote execution admission id")?;
    validate_ref(input.root_artifact_ref, "remote execution admission root ref")?;
    validate_ref(input.closure_plan_ref, "remote execution admission closure plan ref")?;
    validate_refs(input.fetched_refs, "remote execution admission fetched ref")?;
    validate_refs(input.verified_artifact_refs, "remote execution admission verified ref")?;
    validate_ref(
        input.handler_profile_admission_ref,
        "remote execution admission handler profile admission ref",
    )?;
    validate_refs(input.evidence_refs, "remote execution admission evidence ref")?;
    Ok(crate::preserves_rail::record("remote-execution-admission-receipt-v1", vec![
        crate::preserves_rail::string(REMOTE_EXECUTION_ADMISSION_RECEIPT_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(input.decision)]),
        crate::preserves_rail::record("request", vec![crate::preserves_rail::string(input.request_ref)]),
        crate::preserves_rail::record("execution", vec![crate::preserves_rail::string(input.execution_id)]),
        crate::preserves_rail::record("root", vec![crate::preserves_rail::string(input.root_artifact_ref)]),
        crate::preserves_rail::record("closure-plan", vec![crate::preserves_rail::string(input.closure_plan_ref)]),
        crate::preserves_rail::record("fetched", vec![refs_sequence(&sorted_unique(input.fetched_refs))]),
        crate::preserves_rail::record("verified", vec![refs_sequence(&sorted_unique(input.verified_artifact_refs))]),
        crate::preserves_rail::record("handler-profile-admission", vec![crate::preserves_rail::string(
            input.handler_profile_admission_ref,
        )]),
        crate::preserves_rail::record("diagnostics", vec![crate::preserves_rail::sequence(
            input.diagnostics.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("evidence", vec![refs_sequence(&sorted_unique(input.evidence_refs))]),
        crate::preserves_rail::record("refs", vec![refs_sequence(&remote_execution_admission_refs(input)?) ]),
        checks_value(&[
            "receiver-closure-complete",
            "fetch-hash-verified",
            "handler-profile-admission-bound",
            "capability-policy-provenance-resource-bound",
            "transport-is-not-authority",
        ]),
    ]))
}

fn remote_execution_admission_refs(input: &RemoteExecutionAdmissionValueInput<'_>) -> Result<Vec<String>> {
    let mut refs = vec![
        input.request_ref.to_string(),
        input.root_artifact_ref.to_string(),
        input.closure_plan_ref.to_string(),
        input.handler_profile_admission_ref.to_string(),
    ];
    extend_cloned_bounded(&mut refs, input.fetched_refs, MAX_JOB_REFS, "remote execution admission refs")?;
    extend_cloned_bounded(
        &mut refs,
        input.verified_artifact_refs,
        MAX_JOB_REFS,
        "remote execution admission refs",
    )?;
    extend_cloned_bounded(&mut refs, input.evidence_refs, MAX_JOB_REFS, "remote execution admission refs")?;
    Ok(sorted_unique(&refs))
}

fn remote_execution_admission_diagnostics(
    request: &RemoteExecutionRequest,
    plan: &RemoteExecutionClosurePlan,
    input: &RemoteExecutionAdmissionInput,
) -> Result<Vec<String>> {
    let mut diagnostics = Vec::new();
    push_remote_execution_root_diagnostic(&mut diagnostics, request, plan)?;
    extend_cloned_bounded(
        &mut diagnostics,
        &plan.diagnostics,
        MAX_REMOTE_EXECUTION_DIAGNOSTICS,
        "remote execution admission diagnostics",
    )?;
    push_remote_execution_fetch_diagnostics(&mut diagnostics, plan, input)?;
    push_remote_execution_binding_diagnostics(&mut diagnostics, request, input)?;
    push_remote_execution_evidence_diagnostics(&mut diagnostics, input)?;
    Ok(diagnostics)
}

fn push_remote_execution_root_diagnostic(
    diagnostics: &mut impl crate::bounded::VecSink<String>,
    request: &RemoteExecutionRequest,
    plan: &RemoteExecutionClosurePlan,
) -> Result<()> {
    if plan.root_artifact_ref != request.root_artifact_ref {
        push_bounded(
            diagnostics,
            "remote execution closure plan root does not match request".to_string(),
            MAX_REMOTE_EXECUTION_DIAGNOSTICS,
            "remote execution admission diagnostics",
        )?;
    }
    Ok(())
}

fn push_remote_execution_fetch_diagnostics(
    diagnostics: &mut impl crate::bounded::VecSink<String>,
    plan: &RemoteExecutionClosurePlan,
    input: &RemoteExecutionAdmissionInput,
) -> Result<()> {
    for missing_ref in &plan.missing_refs {
        if !input.fetched_refs.iter().any(|reference| reference == missing_ref) {
            push_remote_execution_diagnostic(diagnostics, "missing dependency was not fetched", missing_ref)?;
        }
    }
    for fetched_ref in &input.fetched_refs {
        if !plan.selected_fetch_refs.iter().any(|reference| reference == fetched_ref) {
            push_remote_execution_diagnostic(diagnostics, "fetched ref was not receiver-selected", fetched_ref)?;
        }
        if !input.verified_artifact_refs.iter().any(|reference| reference == fetched_ref) {
            push_remote_execution_diagnostic(diagnostics, "fetched ref lacks hash verification", fetched_ref)?;
        }
    }
    Ok(())
}

fn push_remote_execution_binding_diagnostics(
    diagnostics: &mut impl crate::bounded::VecSink<String>,
    request: &RemoteExecutionRequest,
    input: &RemoteExecutionAdmissionInput,
) -> Result<()> {
    for capability_ref in &request.capability_refs {
        if !input.admitted_capability_refs.iter().any(|reference| reference == capability_ref) {
            push_remote_execution_diagnostic(diagnostics, "missing admitted capability", capability_ref)?;
        }
    }
    for policy_ref in &request.policy_refs {
        if !input.local_policy_refs.iter().any(|reference| reference == policy_ref) {
            push_remote_execution_diagnostic(diagnostics, "local policy did not admit policy ref", policy_ref)?;
        }
    }
    Ok(())
}

fn push_remote_execution_evidence_diagnostics(
    diagnostics: &mut impl crate::bounded::VecSink<String>,
    input: &RemoteExecutionAdmissionInput,
) -> Result<()> {
    if input.handler_profile_admission_ref.is_empty() {
        push_remote_execution_static_diagnostic(diagnostics, "missing handler profile admission ref")?;
    }
    if input.provenance_receipt_refs.is_empty() {
        push_remote_execution_static_diagnostic(diagnostics, "missing provenance admission evidence")?;
    }
    if input.source_gate_receipt_refs.is_empty() {
        push_remote_execution_static_diagnostic(diagnostics, "missing source-gate evidence")?;
    }
    if input.resource_receipt_refs.is_empty() {
        push_remote_execution_static_diagnostic(diagnostics, "missing resource admission evidence")?;
    }
    Ok(())
}

fn push_remote_execution_diagnostic(
    diagnostics: &mut impl crate::bounded::VecSink<String>,
    reason: &str,
    reference: &str,
) -> Result<()> {
    push_bounded(
        diagnostics,
        format!("remote execution {reason}: {reference}"),
        MAX_REMOTE_EXECUTION_DIAGNOSTICS,
        "remote execution admission diagnostics",
    )
}

fn push_remote_execution_static_diagnostic(
    diagnostics: &mut impl crate::bounded::VecSink<String>,
    reason: &str,
) -> Result<()> {
    push_bounded(
        diagnostics,
        format!("remote execution {reason}"),
        MAX_REMOTE_EXECUTION_DIAGNOSTICS,
        "remote execution admission diagnostics",
    )
}

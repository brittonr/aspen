
fn explicit_admission_authority(
    request: &JobAdmissionRequest,
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) -> bool {
    let mut has_explicit_authority = true;
    if request.policy_refs.is_empty() {
        has_explicit_authority = false;
        diagnostics.push_item("job admission missing explicit policy refs".to_string());
    }
    if request.capability_refs.is_empty() {
        has_explicit_authority = false;
        diagnostics.push_item("job admission missing explicit capability refs".to_string());
    }
    if request.evidence_refs.is_empty() {
        has_explicit_authority = false;
        diagnostics.push_item("job admission missing explicit evidence refs".to_string());
    }
    if request.resource_refs.is_empty() {
        has_explicit_authority = false;
        diagnostics.push_item("job admission missing explicit resource refs".to_string());
    }
    has_explicit_authority
}

fn capability_contexts_admit(
    target_registry: &FilePath,
    request: &JobAdmissionRequest,
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) -> Result<(bool, Vec<String>)> {
    if request.capability_refs.is_empty() {
        return Ok((false, Vec::new()));
    }
    let mut has_passing_authority = false;
    let mut receipt_refs = Vec::new();
    for capability_ref in &request.capability_refs {
        match authority_context_value_for_ref(target_registry, capability_ref)? {
            Some(context_value) => {
                let admission =
                    crate::authority::admit_authority(&context_value, "job:execute", &request.job_ref, 0, &[])?;
                push_bounded(
                    &mut receipt_refs,
                    admission.receipt.receipt_ref.clone(),
                    MAX_JOB_REFS,
                    "job admission authority receipt refs",
                )?;
                if admission.decision == "pass" {
                    has_passing_authority = true;
                } else {
                    diagnostics.push_item(format!(
                        "authority context {capability_ref} denied job:execute for {}",
                        request.job_ref
                    ));
                }
            }
            None => diagnostics.push_item(format!(
                "capability ref {capability_ref} is not an authority-context artifact in target registry"
            )),
        }
    }
    if !has_passing_authority {
        diagnostics.push_item(format!("no authority context admits job:execute for {}", request.job_ref));
    }
    Ok((has_passing_authority, receipt_refs))
}

fn authority_context_value_for_ref(target_registry: &FilePath, context_ref: &str) -> Result<Option<IoValue>> {
    validate_ref(context_ref, "job admission authority context ref")?;
    for artifact in crate::artifacts::list_artifacts(target_registry, None)? {
        let payload = crate::artifacts::read_payload(target_registry, &artifact.artifact_ref)?;
        if let Ok(context) = crate::authority::parse_context(&payload)
            && context.context_ref == context_ref
        {
            return Ok(Some(payload));
        }
    }
    Ok(None)
}

fn sync_evidence_bound(request: &JobAdmissionRequest, diagnostics: &mut impl crate::bounded::VecSink<String>) -> bool {
    if request.evidence_refs.iter().any(|reference| reference == &request.sync_ref) {
        true
    } else {
        diagnostics.push_item(format!("job admission evidence refs do not bind sync evidence {}", request.sync_ref));
        false
    }
}

fn source_gate_evidence_bound(
    target_registry: &FilePath,
    request: &JobAdmissionRequest,
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) -> Result<(bool, Vec<String>)> {
    let candidates = request
        .evidence_refs
        .iter()
        .filter(|reference| *reference != &request.sync_ref)
        .cloned()
        .collect::<Vec<_>>();
    if candidates.is_empty() {
        diagnostics.push_item("job admission missing strict Octet source gate evidence ref".to_string());
        return Ok((false, Vec::new()));
    }
    let mut validation_refs = Vec::new();
    let mut has_passing_source_gate = false;
    for candidate in candidates {
        match source_gate_value_for_ref(target_registry, &candidate)? {
            Some(value) => {
                let validation = crate::octet_gate::validate_octet_source_gate(
                    &crate::octet_gate::OctetSourceGateValidationInput {
                        consumer: "job-remote-admission".to_string(),
                        subject_ref: request.job_ref.clone(),
                        receipt_value: Some(value),
                        source_scope: Vec::new(),
                    },
                )?;
                push_bounded(
                    &mut validation_refs,
                    validation.validation_ref.clone(),
                    MAX_JOB_REFS,
                    "job admission source gate validation refs",
                )?;
                if validation.decision == "pass" {
                    has_passing_source_gate = true;
                } else {
                    diagnostics.push_item(format!(
                        "strict Octet source gate {candidate} denied validation {}",
                        validation.validation_ref
                    ));
                }
            }
            None => diagnostics.push_item(format!(
                "strict Octet source gate evidence {candidate} is not available as a target artifact payload"
            )),
        }
    }
    if !has_passing_source_gate {
        diagnostics.push_item("job admission found no passing strict Octet source gate validation".to_string());
    }
    Ok((has_passing_source_gate, validation_refs))
}

fn source_gate_value_for_ref(target_registry: &FilePath, gate_ref: &str) -> Result<Option<IoValue>> {
    validate_ref(gate_ref, "job admission source gate ref")?;
    if let Ok(value) = crate::artifacts::read_payload(target_registry, gate_ref) {
        return Ok(Some(value));
    }
    for artifact in crate::artifacts::list_artifacts(target_registry, None)? {
        let payload = crate::artifacts::read_payload(target_registry, &artifact.artifact_ref)?;
        if crate::preserves_rail::canonical_hash(&payload)? == gate_ref {
            return Ok(Some(payload));
        }
    }
    Ok(None)
}

fn resource_profile_admits(
    request: &JobAdmissionRequest,
    stage_order: &[String],
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) -> Result<bool> {
    if request.resource_refs.is_empty() {
        return Ok(false);
    }
    let stages = stage_order.iter().map(|stage| (stage.as_str(), 1_u64)).collect::<Vec<_>>();
    let planned =
        crate::resources::plan_job_stages(&stages, usize_to_u64(request.resource_refs.len(), "job resource refs")?)?;
    if planned.len() == stage_order.len() {
        Ok(true)
    } else {
        diagnostics.push_item(format!(
            "job admission resource refs admit {} of {} selected stages",
            planned.len(),
            stage_order.len()
        ));
        Ok(false)
    }
}

struct JobAdmissionReceiptValueInput<'a> {
    operation: &'a str,
    decision: &'a str,
    request: &'a JobAdmissionRequest,
    plan_ref: &'a str,
    closure_refs: &'a [String],
    stage_order: &'a [String],
    authority_receipt_refs: &'a [String],
    source_gate_validation_refs: &'a [String],
    resource_verdict: &'a str,
    diagnostics: &'a [String],
    checks: &'a [(&'a str, &'a str)],
}

fn job_admission_receipt_value(input: JobAdmissionReceiptValueInput<'_>) -> Result<IoValue> {
    validate_non_empty(input.operation, "job admission receipt operation")?;
    validate_decision(input.decision)?;
    validate_ref(input.plan_ref, "job admission receipt plan ref")?;
    validate_refs(input.closure_refs, "job admission receipt closure ref")?;
    validate_refs(input.authority_receipt_refs, "job admission authority receipt ref")?;
    validate_refs(input.source_gate_validation_refs, "job admission source gate validation ref")?;
    for stage_id in input.stage_order {
        validate_node_id(stage_id)?;
    }
    validate_decision(input.resource_verdict)?;
    let mut refs = vec![
        input.request.job_ref.clone(),
        input.request.sync_ref.clone(),
        input.request.request_ref.clone(),
        input.plan_ref.to_string(),
    ];
    refs.extend(input.closure_refs.iter().cloned());
    refs.extend(input.authority_receipt_refs.iter().cloned());
    refs.extend(input.source_gate_validation_refs.iter().cloned());
    refs.extend(input.request.policy_refs.iter().cloned());
    refs.extend(input.request.capability_refs.iter().cloned());
    refs.extend(input.request.evidence_refs.iter().cloned());
    refs.extend(input.request.resource_refs.iter().cloned());
    let mut checks = input.checks.to_vec();
    checks.push(("canonical-receipt", "pass"));
    Ok(crate::preserves_rail::record("job-admission-receipt-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::JOB_ADMISSION_RECEIPT_SCHEMA),
        crate::preserves_rail::record("operation", vec![crate::preserves_rail::string(input.operation)]),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(input.decision)]),
        crate::preserves_rail::record("job", vec![crate::preserves_rail::string(&input.request.job_ref)]),
        crate::preserves_rail::record("request", vec![crate::preserves_rail::string(&input.request.request_ref)]),
        crate::preserves_rail::record("artifact", vec![crate::preserves_rail::string(input.plan_ref)]),
        crate::preserves_rail::record("sync", vec![crate::preserves_rail::string(&input.request.sync_ref)]),
        crate::preserves_rail::record("target-peer", vec![crate::preserves_rail::string(&input.request.target_peer)]),
        crate::preserves_rail::record("closure", vec![refs_sequence(input.closure_refs)]),
        crate::preserves_rail::record("stages", vec![crate::preserves_rail::sequence(
            input.stage_order.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("authority", vec![refs_sequence(input.authority_receipt_refs)]),
        crate::preserves_rail::record("resource-verdict", vec![crate::preserves_rail::string(input.resource_verdict)]),
        crate::preserves_rail::record("diagnostics", vec![crate::preserves_rail::sequence(
            input.diagnostics.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("refs", vec![refs_sequence(&sorted_unique(&refs))]),
        checks_value_from_pairs(&checks),
    ]))
}

fn job_execution_receipt_value(input: ExecutionReceiptValueInput<'_>) -> Result<IoValue> {
    validate_decision(input.decision)?;
    validate_refs(input.stage_receipt_refs, "job execution stage receipt ref")?;
    validate_refs(input.output_refs, "job execution output ref")?;
    validate_refs(input.run_receipt_refs, "job execution run receipt ref")?;
    let stage_values = input
        .admission
        .stage_order
        .iter()
        .enumerate()
        .map(|(index, stage_id)| {
            crate::preserves_rail::record("stage", vec![
                crate::preserves_rail::string(stage_id),
                optional_ref_value(input.stage_receipt_refs.get(index).map(String::as_str)),
            ])
        })
        .collect::<Vec<_>>();
    let mut refs = vec![
        input.request.job_ref.clone(),
        input.request.request_ref.clone(),
        input.request.admission_ref.clone(),
        input.admission.sync_ref.clone(),
        input.admission.plan_ref.clone(),
    ];
    refs.extend(input.admission.closure_refs.iter().cloned());
    refs.extend(input.admission.authority_receipt_refs.iter().cloned());
    refs.extend(input.stage_receipt_refs.iter().cloned());
    refs.extend(input.output_refs.iter().cloned());
    refs.extend(input.run_receipt_refs.iter().cloned());
    refs.extend(input.request.policy_refs.iter().cloned());
    refs.extend(input.request.capability_refs.iter().cloned());
    refs.extend(input.request.resource_refs.iter().cloned());
    let mut checks = input.checks.to_vec();
    checks.push(("canonical-receipt", "pass"));
    Ok(crate::preserves_rail::record("job-execution-receipt-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::JOB_EXECUTION_RECEIPT_SCHEMA),
        crate::preserves_rail::record("operation", vec![crate::preserves_rail::string("execute-loopback")]),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(input.decision)]),
        crate::preserves_rail::record("job", vec![crate::preserves_rail::string(&input.request.job_ref)]),
        crate::preserves_rail::record("request", vec![crate::preserves_rail::string(&input.request.request_ref)]),
        crate::preserves_rail::record("admission", vec![crate::preserves_rail::string(&input.request.admission_ref)]),
        crate::preserves_rail::record("sync", vec![crate::preserves_rail::string(&input.admission.sync_ref)]),
        crate::preserves_rail::record("target-peer", vec![crate::preserves_rail::string(&input.request.target_peer)]),
        crate::preserves_rail::record("closure", vec![refs_sequence(&input.admission.closure_refs)]),
        crate::preserves_rail::record("authority", vec![refs_sequence(&input.admission.authority_receipt_refs)]),
        crate::preserves_rail::record("stages", vec![crate::preserves_rail::sequence(stage_values)]),
        crate::preserves_rail::record("outputs", vec![refs_sequence(input.output_refs)]),
        crate::preserves_rail::record("run", vec![refs_sequence(input.run_receipt_refs)]),
        crate::preserves_rail::record("diagnostics", vec![crate::preserves_rail::sequence(
            input.diagnostics.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("refs", vec![refs_sequence(&sorted_unique(&refs))]),
        checks_value_from_pairs(&checks),
    ]))
}


fn plan_record(input: RecordInput<'_>) -> IoValue {
    crate::preserves_rail::record("job-admission-plan-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::JOB_ADMISSION_PLAN_SCHEMA),
        crate::preserves_rail::record("request", vec![crate::preserves_rail::string(&input.request.request_ref)]),
        crate::preserves_rail::record("job", vec![crate::preserves_rail::string(&input.request.job_ref)]),
        crate::preserves_rail::record("sync", vec![crate::preserves_rail::string(&input.request.sync_ref)]),
        crate::preserves_rail::record("target-peer", vec![crate::preserves_rail::string(&input.request.target_peer)]),
        crate::preserves_rail::record("stages", vec![crate::preserves_rail::sequence(
            input.request.stage_ids.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("closure", vec![refs_sequence(&input.readiness.closure_refs)]),
        crate::preserves_rail::record("topology", vec![crate::preserves_rail::sequence(
            input.readiness.stage_order.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("stage-verdicts", vec![crate::preserves_rail::sequence(verdict_values(
            &input.readiness.stage_verdicts,
        ))]),
        crate::preserves_rail::record("authority", vec![refs_sequence(input.authority_receipt_refs)]),
        crate::preserves_rail::record("resource-verdict", vec![crate::preserves_rail::string(input.resource_verdict)]),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(input.decision)]),
        crate::preserves_rail::record("diagnostics", vec![crate::preserves_rail::sequence(
            input.diagnostics.iter().map(crate::preserves_rail::string).collect(),
        )]),
        checks_value_from_pairs(input.checks),
    ])
}

fn verdict_values(verdicts: &[JobAdmissionStageVerdict]) -> Vec<IoValue> {
    verdicts
        .iter()
        .map(|verdict| {
            crate::preserves_rail::record("stage", vec![
                crate::preserves_rail::string(&verdict.stage_id),
                crate::preserves_rail::string(&verdict.decision),
                crate::preserves_rail::record("diagnostics", vec![crate::preserves_rail::sequence(
                    verdict.diagnostics.iter().map(crate::preserves_rail::string).collect(),
                )]),
            ])
        })
        .collect()
}

pub fn admission_loopback(target_registry: &FilePath, request_value: &IoValue) -> Result<JobAdmissionLoopback> {
    let plan = admission_plan_value(target_registry, request_value)?;
    let checks = [
        ("target-closure-present", status(plan.decision == "pass" || !plan.closure_refs.is_empty())),
        ("trellis-topology", status(plan.stage_verdicts.iter().all(|verdict| verdict.decision == "pass"))),
        (
            "executable-artifact-gate",
            status(plan.stage_verdicts.iter().all(|verdict| verdict.decision == "pass")),
        ),
        (
            "explicit-authority",
            status(!plan.request.policy_refs.is_empty() && !plan.request.capability_refs.is_empty()),
        ),
        (
            "capability-authority-context",
            status(!plan.authority_receipt_refs.is_empty() && plan.decision == "pass"),
        ),
        ("resource-profile", status(plan.resource_verdict == "pass")),
        (
            "sync-evidence-bound",
            status(plan.request.evidence_refs.iter().any(|reference| reference == &plan.request.sync_ref)),
        ),
        (
            "strict-octet-source-gate-bound",
            status(plan.request.evidence_refs.iter().any(|reference| reference != &plan.request.sync_ref)),
        ),
        ("loopback-admission", "pass"),
        ("no-execution", "pass"),
    ];
    let receipt_value = job_admission_receipt_value(JobAdmissionReceiptValueInput {
        operation: "admit-loopback",
        decision: &plan.decision,
        request: &plan.request,
        plan_ref: &plan.plan_ref,
        closure_refs: &plan.closure_refs,
        stage_order: &plan.stage_order,
        authority_receipt_refs: &plan.authority_receipt_refs,
        source_gate_validation_refs: &plan.source_gate_validation_refs,
        resource_verdict: &plan.resource_verdict,
        diagnostics: &plan.diagnostics,
        checks: &checks,
    })?;
    let receipt_ref = crate::preserves_rail::canonical_hash(&receipt_value)?;
    Ok(JobAdmissionLoopback {
        receipt_ref,
        plan,
        receipt_value,
    })
}

pub fn missing_admission_execution_receipt_value(request_value: &IoValue, diagnostic: &str) -> Result<IoValue> {
    let request = parse_job_execution_request_value(request_value)?;
    let sync_ref = local_ref("missing-execution-sync", &request.admission_ref)?;
    let mut refs = vec![
        request.job_ref.clone(),
        request.request_ref.clone(),
        request.admission_ref.clone(),
        sync_ref.clone(),
    ];
    refs.extend(request.policy_refs.iter().cloned());
    refs.extend(request.capability_refs.iter().cloned());
    refs.extend(request.resource_refs.iter().cloned());
    Ok(crate::preserves_rail::record("job-execution-receipt-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::JOB_EXECUTION_RECEIPT_SCHEMA),
        crate::preserves_rail::record("operation", vec![crate::preserves_rail::string("execute-loopback")]),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string("deny")]),
        crate::preserves_rail::record("job", vec![crate::preserves_rail::string(&request.job_ref)]),
        crate::preserves_rail::record("request", vec![crate::preserves_rail::string(&request.request_ref)]),
        crate::preserves_rail::record("admission", vec![crate::preserves_rail::string(&request.admission_ref)]),
        crate::preserves_rail::record("sync", vec![crate::preserves_rail::string(&sync_ref)]),
        crate::preserves_rail::record("target-peer", vec![crate::preserves_rail::string(&request.target_peer)]),
        crate::preserves_rail::record("closure", vec![refs_sequence(&[])]),
        crate::preserves_rail::record("authority", vec![refs_sequence(&[])]),
        crate::preserves_rail::record("stages", vec![crate::preserves_rail::sequence(Vec::new())]),
        crate::preserves_rail::record("outputs", vec![refs_sequence(&[])]),
        crate::preserves_rail::record("run", vec![refs_sequence(&[])]),
        crate::preserves_rail::record("diagnostics", vec![crate::preserves_rail::sequence(vec![
            crate::preserves_rail::string(diagnostic),
        ])]),
        crate::preserves_rail::record("refs", vec![refs_sequence(&sorted_unique(&refs))]),
        checks_value_from_pairs(&[
            ("admission-required", "fail"),
            ("admission-readable", "fail"),
            ("no-stage-execution-on-deny", "pass"),
            ("canonical-receipt", "pass"),
        ]),
    ]))
}

pub fn execution_loopback(input: ExecutionLoopbackInput<'_>) -> Result<JobExecutionLoopback> {
    let request = parse_job_execution_request_value(input.request_value)?;
    let admission = parse_job_admission_receipt_value(input.admission_receipt_value)?;
    let mut diagnostics = Vec::new();
    let mut checks = Vec::new();

    check_receipt_bindings(&request, &admission, &mut diagnostics, &mut checks);
    check_admission_readiness(&admission, &mut diagnostics, &mut checks);

    let dag = match read_job_dag(input.target_registry, &request.job_ref) {
        Ok(dag) => dag,
        Err(error) => {
            diagnostics.push(format!("target job unavailable before execution: {error}"));
            return deny_result(DenyInput {
                request,
                admission,
                diagnostics,
                checks,
                extra_checks: &[("target-job-present", "fail"), ("no-stage-execution-on-deny", "pass")],
            });
        }
    };
    push_check(&mut checks, "target-job-present", true);

    check_target_selection(
        SelectionInput {
            target_registry: input.target_registry,
            request: &request,
            admission: &admission,
            dag: &dag,
        },
        &mut diagnostics,
        &mut checks,
    )?;

    if checks.iter().any(|(_, status)| *status != "pass") {
        return deny_result(DenyInput {
            request,
            admission,
            diagnostics,
            checks,
            extra_checks: &[("no-stage-execution-on-deny", "pass")],
        });
    }

    let run = run_job_dag(&dag, &JobRunOptions {
        registry_root: input.target_registry,
        storage_root: input.storage_root,
        cache_root: input.cache_root,
        chunk_root: input.chunk_root,
        ledger_root: None,
        output_request: None,
    })?;
    pass_result(PassInput {
        request,
        admission,
        run,
        diagnostics,
        checks,
    })
}

fn check_receipt_bindings(
    request: &JobExecutionRequest,
    admission: &JobAdmissionReceipt,
    diagnostics: &mut impl crate::bounded::VecSink<String>,
    checks: &mut impl crate::bounded::VecSink<(&'static str, &'static str)>,
) {
    let has_matching_admission_ref = admission.receipt_ref == request.admission_ref;
    push_check(checks, "admission-ref-binding", has_matching_admission_ref);
    if !has_matching_admission_ref {
        diagnostics.push_item(format!(
            "job execution request admission ref {} does not match receipt {}",
            request.admission_ref, admission.receipt_ref
        ));
    }

    let is_admission_pass = admission.decision == "pass";
    push_check(checks, "admission-pass", is_admission_pass);
    if !is_admission_pass {
        diagnostics.push_item(format!("job execution admission decision is {}", admission.decision));
    }

    let has_matching_job_ref = admission.job_ref == request.job_ref;
    push_check(checks, "job-ref-binding", has_matching_job_ref);
    if !has_matching_job_ref {
        diagnostics.push_item(format!(
            "job execution request job {} does not match admission job {}",
            request.job_ref, admission.job_ref
        ));
    }

    let has_matching_target_peer = admission.target_peer == request.target_peer;
    push_check(checks, "target-peer-binding", has_matching_target_peer);
    if !has_matching_target_peer {
        diagnostics.push_item(format!(
            "job execution target peer {} does not match admission target peer {}",
            request.target_peer, admission.target_peer
        ));
    }
}

fn check_admission_readiness(
    admission: &JobAdmissionReceipt,
    diagnostics: &mut impl crate::bounded::VecSink<String>,
    checks: &mut impl crate::bounded::VecSink<(&'static str, &'static str)>,
) {
    let required_admission_checks = [
        "target-closure-present",
        "trellis-topology",
        "executable-artifact-gate",
        "capability-authority-context",
        "resource-profile",
        "sync-evidence-bound",
        "strict-octet-source-gate-bound",
        "no-execution",
    ];
    let has_required_admission_checks = required_admission_checks
        .iter()
        .all(|required| admission.checks.iter().any(|check| check == *required));
    push_check(checks, "admission-checkset", has_required_admission_checks);
    if !has_required_admission_checks {
        diagnostics.push_item("job execution admission receipt is missing required target-side checks".to_string());
    }

    let has_authority_receipts = !admission.authority_receipt_refs.is_empty();
    push_check(checks, "authority-receipt-binding", has_authority_receipts);
    if !has_authority_receipts {
        diagnostics.push_item("job execution admission has no authority receipt refs".to_string());
    }

    let has_resource_profile = admission.resource_verdict == "pass";
    push_check(checks, "resource-profile-binding", has_resource_profile);
    if !has_resource_profile {
        diagnostics.push_item(format!("job execution resource verdict is {}", admission.resource_verdict));
    }
}

struct SelectionInput<'a> {
    target_registry: &'a FilePath,
    request: &'a JobExecutionRequest,
    admission: &'a JobAdmissionReceipt,
    dag: &'a JobDag,
}

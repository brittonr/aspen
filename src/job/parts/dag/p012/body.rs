
fn job_worker_assignment_value(
    request: &JobWorkerRequest,
    delivery: &crate::remote_dataspace::Delivery,
) -> Result<IoValue> {
    Ok(crate::preserves_rail::record("job-worker-assignment-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::JOB_WORKER_ASSIGNMENT_SCHEMA),
        crate::preserves_rail::record("request", vec![crate::preserves_rail::string(&request.request_ref)]),
        crate::preserves_rail::record("job", vec![crate::preserves_rail::string(&request.job_ref)]),
        crate::preserves_rail::record("target-peer", vec![crate::preserves_rail::string(&request.target_peer)]),
        crate::preserves_rail::record("stages", vec![crate::preserves_rail::sequence(
            request.stage_ids.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("from-peer", vec![crate::preserves_rail::string(&delivery.envelope.from_peer)]),
        crate::preserves_rail::record("delivery-envelope", vec![crate::preserves_rail::string(
            &delivery.envelope.envelope_ref,
        )]),
        crate::preserves_rail::record("operation-ref", vec![crate::preserves_rail::string(
            &delivery.envelope.operation_ref,
        )]),
        crate::preserves_rail::record("execution-request", vec![crate::preserves_rail::string(
            &request.execution_request_ref,
        )]),
        checks_value(&[
            "request-assigned-to-target",
            "remote-dataspace-envelope-bound",
            "delivery-operation-ref-bound",
            "transport-is-not-authority",
        ]),
    ]))
}

fn job_worker_status_value(input: WorkerStatusValueInput<'_>) -> Result<IoValue> {
    validate_worker_state(input.state)?;
    let mut refs = vec![
        input.request.request_ref.clone(),
        input.delivery.envelope.envelope_ref.clone(),
        input.delivery.envelope.operation_ref.clone(),
    ];
    if let Some(execution_receipt_ref) = input.execution_receipt_ref {
        push_bounded(&mut refs, execution_receipt_ref.to_string(), MAX_JOB_REFS, "job worker status refs")?;
    }
    let mut status_checks = input.checks.to_vec();
    status_checks.push(("canonical-status", "pass"));
    status_checks.push(("delivery-operation-ref-bound", "pass"));
    Ok(crate::preserves_rail::record("job-worker-status-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::JOB_WORKER_STATUS_SCHEMA),
        crate::preserves_rail::record("request", vec![crate::preserves_rail::string(&input.request.request_ref)]),
        crate::preserves_rail::record("job", vec![crate::preserves_rail::string(&input.request.job_ref)]),
        crate::preserves_rail::record("target-peer", vec![crate::preserves_rail::string(&input.request.target_peer)]),
        crate::preserves_rail::record("state", vec![crate::preserves_rail::string(input.state)]),
        crate::preserves_rail::record("delivery-envelope", vec![crate::preserves_rail::string(
            &input.delivery.envelope.envelope_ref,
        )]),
        crate::preserves_rail::record("operation-ref", vec![crate::preserves_rail::string(
            &input.delivery.envelope.operation_ref,
        )]),
        crate::preserves_rail::record("execution-receipt", vec![optional_ref_value(input.execution_receipt_ref)]),
        crate::preserves_rail::record("diagnostics", vec![crate::preserves_rail::sequence(
            input.diagnostics.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("refs", vec![refs_sequence(&sorted_unique(&refs))]),
        checks_value_from_pairs(&status_checks),
    ]))
}

fn job_worker_result_value(input: WorkerResultValueInput<'_>) -> Result<IoValue> {
    validate_worker_decision(input.decision)?;
    validate_refs(input.output_refs, "job worker output ref")?;
    validate_stage_receipt_refs(input.stage_receipt_refs)?;
    validate_refs(input.resource_receipt_refs, "job worker resource receipt ref")?;
    if let Some(delivery_log_ref) = input.delivery_log_ref {
        validate_ref(delivery_log_ref, "job worker delivery log ref")?;
    }
    let mut refs = vec![
        input.request.request_ref.clone(),
        input.request.job_ref.clone(),
        input.request.sync_ref.clone(),
        input.request.admission_ref.clone(),
        input.request.execution_request_ref.clone(),
    ];
    if let Some(execution_receipt_ref) = input.execution_receipt_ref {
        push_bounded(&mut refs, execution_receipt_ref.to_string(), MAX_JOB_REFS, "job worker result refs")?;
    }
    extend_cloned_bounded(&mut refs, input.output_refs, MAX_JOB_REFS, "job worker result refs")?;
    for (_, receipt_ref) in input.stage_receipt_refs {
        push_bounded(&mut refs, receipt_ref.clone(), MAX_JOB_REFS, "job worker result refs")?;
    }
    extend_cloned_bounded(&mut refs, input.resource_receipt_refs, MAX_JOB_REFS, "job worker result refs")?;
    if let Some(delivery_log_ref) = input.delivery_log_ref {
        push_bounded(&mut refs, delivery_log_ref.to_string(), MAX_JOB_REFS, "job worker result refs")?;
    }
    let stage_values = input
        .stage_receipt_refs
        .iter()
        .map(|(stage_id, receipt_ref)| {
            crate::preserves_rail::record("stage", vec![
                crate::preserves_rail::string(stage_id),
                crate::preserves_rail::string(receipt_ref),
            ])
        })
        .collect::<Vec<_>>();
    let mut checks = input.checks.to_vec();
    checks.push(("canonical-result", "pass"));
    Ok(crate::preserves_rail::record("job-worker-result-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::JOB_WORKER_RESULT_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(input.decision)]),
        crate::preserves_rail::record("job", vec![crate::preserves_rail::string(&input.request.job_ref)]),
        crate::preserves_rail::record("target-peer", vec![crate::preserves_rail::string(&input.request.target_peer)]),
        crate::preserves_rail::record("execution-receipt", vec![optional_ref_value(input.execution_receipt_ref)]),
        crate::preserves_rail::record("outputs", vec![refs_sequence(input.output_refs)]),
        crate::preserves_rail::record("stage-receipts", vec![crate::preserves_rail::sequence(stage_values)]),
        crate::preserves_rail::record("resource", vec![refs_sequence(input.resource_receipt_refs)]),
        crate::preserves_rail::record("delivery-log", vec![optional_ref_value(input.delivery_log_ref)]),
        crate::preserves_rail::record("diagnostics", vec![crate::preserves_rail::sequence(
            input.diagnostics.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("refs", vec![refs_sequence(&sorted_unique(&refs))]),
        checks_value_from_pairs(&checks),
    ]))
}

fn job_worker_receipt_value(input: WorkerReceiptValueInput<'_>) -> Result<IoValue> {
    validate_worker_decision(input.decision)?;
    validate_ref(input.assignment_ref, "job worker assignment ref")?;
    validate_refs(input.status_refs, "job worker status ref")?;
    validate_ref(input.result_ref, "job worker result ref")?;
    if let Some(execution_receipt_ref) = input.execution_receipt_ref {
        validate_ref(execution_receipt_ref, "job worker execution receipt ref")?;
    }
    if let Some(delivery_log_ref) = input.delivery_log_ref {
        validate_ref(delivery_log_ref, "job worker delivery log ref")?;
    }
    let request_ref = input.request.map(|request| request.request_ref.as_str());
    let job_ref = input.request.map(|request| request.job_ref.as_str());
    let mut refs = vec![input.assignment_ref.to_string(), input.result_ref.to_string()];
    extend_cloned_bounded(&mut refs, input.status_refs, MAX_JOB_REFS, "job worker receipt refs")?;
    if let Some(request) = input.request {
        push_bounded(&mut refs, request.request_ref.clone(), MAX_JOB_REFS, "job worker receipt refs")?;
        push_bounded(&mut refs, request.job_ref.clone(), MAX_JOB_REFS, "job worker receipt refs")?;
    }
    if let Some(execution_receipt_ref) = input.execution_receipt_ref {
        push_bounded(&mut refs, execution_receipt_ref.to_string(), MAX_JOB_REFS, "job worker receipt refs")?;
    }
    if let Some(delivery_log_ref) = input.delivery_log_ref {
        push_bounded(&mut refs, delivery_log_ref.to_string(), MAX_JOB_REFS, "job worker receipt refs")?;
    }
    let mut checks = input.checks.to_vec();
    checks.push(("canonical-receipt", "pass"));
    Ok(crate::preserves_rail::record("job-worker-receipt-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::JOB_WORKER_RECEIPT_SCHEMA),
        crate::preserves_rail::record("operation", vec![crate::preserves_rail::string("worker-execute")]),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(input.decision)]),
        crate::preserves_rail::record("job", vec![optional_ref_value(job_ref)]),
        crate::preserves_rail::record("request", vec![optional_ref_value(request_ref)]),
        crate::preserves_rail::record("assignment", vec![crate::preserves_rail::string(input.assignment_ref)]),
        crate::preserves_rail::record("status", vec![refs_sequence(input.status_refs)]),
        crate::preserves_rail::record("result", vec![crate::preserves_rail::string(input.result_ref)]),
        crate::preserves_rail::record("execution-receipt", vec![optional_ref_value(input.execution_receipt_ref)]),
        crate::preserves_rail::record("delivery-log", vec![optional_ref_value(input.delivery_log_ref)]),
        crate::preserves_rail::record("diagnostics", vec![crate::preserves_rail::sequence(
            input.diagnostics.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("refs", vec![refs_sequence(&sorted_unique(&refs))]),
        checks_value_from_pairs(&checks),
    ]))
}

pub fn job_worker_schedule_receipt_value(input: JobWorkerScheduleReceiptValueInput<'_>) -> Result<IoValue> {
    validate_non_empty(input.operation, "job worker schedule operation")?;
    validate_decision(input.decision)?;
    validate_ref(input.job_ref, "job worker schedule job ref")?;
    validate_ref(input.request_ref, "job worker schedule request ref")?;
    validate_non_empty(input.queue_key, "job worker schedule queue key")?;
    validate_non_empty(input.lease_key, "job worker schedule lease key")?;
    validate_non_empty(input.worker_session, "job worker schedule worker session")?;
    validate_ref(input.coordination_report_ref, "job worker schedule coordination report ref")?;
    validate_present_refs(&optional_refs(&input))?;
    validate_refs(input.refs, "job worker schedule refs")?;
    ensure_count_at_most(input.diagnostics.len(), MAX_JOB_REFS, "job worker schedule diagnostics")?;
    let refs = collected_refs(&input)?;
    let checks = checked_pairs(&input);
    Ok(crate::preserves_rail::record("job-worker-schedule-receipt-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::JOB_WORKER_SCHEDULE_RECEIPT_SCHEMA),
        crate::preserves_rail::record("operation", vec![crate::preserves_rail::string(input.operation)]),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(input.decision)]),
        crate::preserves_rail::record("job", vec![crate::preserves_rail::string(input.job_ref)]),
        crate::preserves_rail::record("request", vec![crate::preserves_rail::string(input.request_ref)]),
        crate::preserves_rail::record("queue-key", vec![crate::preserves_rail::string(input.queue_key)]),
        crate::preserves_rail::record("lease-key", vec![crate::preserves_rail::string(input.lease_key)]),
        crate::preserves_rail::record("worker-session", vec![crate::preserves_rail::string(input.worker_session)]),
        crate::preserves_rail::record("coordination-report", vec![crate::preserves_rail::string(
            input.coordination_report_ref,
        )]),
        crate::preserves_rail::record("enqueue", vec![optional_ref_value(input.enqueue_receipt_ref)]),
        crate::preserves_rail::record("enqueue-duplicate", vec![optional_ref_value(
            input.enqueue_duplicate_receipt_ref,
        )]),
        crate::preserves_rail::record("dequeue", vec![optional_ref_value(input.dequeue_receipt_ref)]),
        crate::preserves_rail::record("lease", vec![optional_ref_value(input.lease_receipt_ref)]),
        crate::preserves_rail::record("release", vec![optional_ref_value(input.release_receipt_ref)]),
        crate::preserves_rail::record("token", vec![optional_ref_value(input.token_ref)]),
        crate::preserves_rail::record("worker-receipt", vec![optional_ref_value(input.worker_receipt_ref)]),
        crate::preserves_rail::record("result", vec![optional_ref_value(input.result_ref)]),
        crate::preserves_rail::record("diagnostics", vec![crate::preserves_rail::sequence(
            input.diagnostics.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("refs", vec![refs_sequence(&sorted_unique(&refs))]),
        checks_value_from_pairs(&checks),
    ]))
}

fn optional_refs<'a>(input: &JobWorkerScheduleReceiptValueInput<'a>) -> [(&'static str, Option<&'a str>); 8] {
    [
        ("enqueue receipt", input.enqueue_receipt_ref),
        ("enqueue duplicate receipt", input.enqueue_duplicate_receipt_ref),
        ("dequeue receipt", input.dequeue_receipt_ref),
        ("lease receipt", input.lease_receipt_ref),
        ("release receipt", input.release_receipt_ref),
        ("token", input.token_ref),
        ("worker receipt", input.worker_receipt_ref),
        ("result", input.result_ref),
    ]
}

fn validate_present_refs(pairs: &[(&str, Option<&str>)]) -> Result<()> {
    for (label, reference) in pairs {
        if let Some(reference) = reference {
            validate_ref(reference, &format!("job worker schedule {label} ref"))?;
        }
    }
    Ok(())
}

fn collected_refs(input: &JobWorkerScheduleReceiptValueInput<'_>) -> Result<Vec<String>> {
    let mut refs = vec![
        input.job_ref.to_string(),
        input.request_ref.to_string(),
        input.coordination_report_ref.to_string(),
    ];
    for (_, reference) in optional_refs(input) {
        if let Some(reference) = reference {
            push_bounded(&mut refs, reference.to_string(), MAX_JOB_REFS, "job worker schedule refs")?;
        }
    }
    extend_cloned_bounded(&mut refs, input.refs, MAX_JOB_REFS, "job worker schedule refs")?;
    Ok(refs)
}

fn checked_pairs<'a>(input: &JobWorkerScheduleReceiptValueInput<'a>) -> Vec<(&'a str, &'a str)> {
    let mut checks = input.checks.to_vec();
    checks.push(("coordination-queue-bound", "pass"));
    checks.push(("coordination-lease-bound", status(input.token_ref.is_some())));
    checks.push(("transport-is-not-authority", "pass"));
    checks.push(("canonical-receipt", "pass"));
    checks
}

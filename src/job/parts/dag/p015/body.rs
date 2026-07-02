
fn finish_worker_delivery(mut input: FinishDeliveryInput<'_>) -> Result<JobWorkerExecution> {
    let mut diagnostics = input.delivery.diagnostics.clone();
    let outputs = worker_outputs(&input.request, input.run.execution.as_ref(), &mut diagnostics)?;
    let final_decision = final_worker_decision(outputs.is_execution_pass, input.delivery.has_recorded_delivery);
    let final_status = final_worker_status(FinalStatusInput {
        delivery: input.input.delivery,
        request: &input.request,
        diagnostics: &diagnostics,
        outputs: &outputs,
        is_preliminary_pass: input.delivery.is_preliminary_pass,
        final_decision,
    })?;
    push_bounded(&mut input.run.status_values, final_status, MAX_JOB_REFS, "job worker statuses")?;

    let result_checks = checks_with_extra(&input.delivery.checks, &[
        ("loopback-execution-verifier", status(input.run.execution.is_some())),
        ("executed-on-target-state", status(outputs.is_execution_pass)),
        ("result-output-binding", status(final_decision != "pass" || !outputs.output_refs.is_empty())),
        ("resource-accounting", status(!input.request.resource_refs.is_empty())),
        ("live-unrecorded-diagnostic", status(final_decision != "non-replayable")),
    ]);
    let result = worker_result(&input, &outputs, &diagnostics, &result_checks)?;
    let receipt = worker_receipt(WorkerReceiptInput {
        input: &input,
        status_values: &input.run.status_values,
        outputs: &outputs,
        diagnostics: &diagnostics,
        result: &result,
        result_checks: &result_checks,
    })?;
    if let Some(ledger_root) = input.input.ledger_root {
        import_worker_artifacts(
            ledger_root,
            &input.assignment_value,
            &input.run.status_values,
            &result.value,
            &receipt.receipt_value,
        )?;
    }
    Ok(JobWorkerExecution {
        request: Some(input.request),
        assignment_value: input.assignment_value,
        status_values: input.run.status_values,
        result,
        receipt_ref: receipt.receipt_ref,
        receipt_value: receipt.receipt_value,
        execution: input.run.execution,
    })
}

fn worker_outputs(
    request: &JobWorkerRequest,
    execution: Option<&JobExecutionLoopback>,
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) -> Result<WorkerOutputs> {
    let execution_receipt_ref = execution.map(|execution| execution.receipt_ref.clone());
    let mut output_refs = Vec::new();
    let mut stage_receipt_refs = Vec::new();
    let mut resource_receipt_refs = request.resource_refs.clone();
    push_bounded(
        &mut resource_receipt_refs,
        local_ref("job-worker-resource-accounting", &request.request_ref)?,
        MAX_JOB_REFS,
        "job worker resource receipt refs",
    )?;
    if let Some(execution) = execution {
        if let Some(run) = execution.run.as_ref() {
            output_refs = run.output_refs.clone();
            stage_receipt_refs = worker_stage_receipts(&execution.admission.stage_order, &run.stage_receipt_refs)?;
        }
        diagnostics.extend_cloned_items(&execution.diagnostics);
    }
    Ok(WorkerOutputs {
        execution_receipt_ref,
        output_refs,
        stage_receipt_refs,
        resource_receipt_refs,
        is_execution_pass: execution.is_some_and(|execution| execution.decision == "pass"),
    })
}

fn final_worker_decision(is_execution_pass: bool, has_recorded_delivery: bool) -> &'static str {
    if is_execution_pass && has_recorded_delivery {
        "pass"
    } else if is_execution_pass {
        "non-replayable"
    } else {
        "deny"
    }
}

fn final_worker_status(input: FinalStatusInput<'_>) -> Result<IoValue> {
    let final_state = match input.final_decision {
        "pass" => "completed",
        "non-replayable" => "non-replayable",
        _ => "denied",
    };
    job_worker_status_value(WorkerStatusValueInput {
        request: input.request,
        delivery: input.delivery,
        state: final_state,
        execution_receipt_ref: input.outputs.execution_receipt_ref.as_deref(),
        diagnostics: input.diagnostics,
        checks: &[(
            "no-stage-execution-on-deny",
            status(!input.is_preliminary_pass || input.outputs.is_execution_pass),
        )],
    })
}

struct WorkerReceipt {
    receipt_ref: String,
    receipt_value: IoValue,
}

struct FinalStatusInput<'a> {
    delivery: &'a crate::remote_dataspace::Delivery,
    request: &'a JobWorkerRequest,
    diagnostics: &'a [String],
    outputs: &'a WorkerOutputs,
    is_preliminary_pass: bool,
    final_decision: &'a str,
}

struct WorkerReceiptInput<'a> {
    input: &'a FinishDeliveryInput<'a>,
    status_values: &'a [IoValue],
    outputs: &'a WorkerOutputs,
    diagnostics: &'a [String],
    result: &'a JobWorkerResult,
    result_checks: &'a [(&'static str, &'static str)],
}

fn worker_result(
    input: &FinishDeliveryInput<'_>,
    outputs: &WorkerOutputs,
    diagnostics: &[String],
    result_checks: &[(&'static str, &'static str)],
) -> Result<JobWorkerResult> {
    let final_decision = final_worker_decision(outputs.is_execution_pass, input.delivery.has_recorded_delivery);
    let result_value = job_worker_result_value(WorkerResultValueInput {
        decision: final_decision,
        request: &input.request,
        execution_receipt_ref: outputs.execution_receipt_ref.as_deref(),
        output_refs: &outputs.output_refs,
        stage_receipt_refs: &outputs.stage_receipt_refs,
        resource_receipt_refs: &outputs.resource_receipt_refs,
        delivery_log_ref: input.delivery.delivery_log_ref.as_deref(),
        diagnostics,
        checks: result_checks,
    })?;
    parse_job_worker_result_value(&result_value)
}

fn worker_receipt(input: WorkerReceiptInput<'_>) -> Result<WorkerReceipt> {
    let final_decision =
        final_worker_decision(input.outputs.is_execution_pass, input.input.delivery.has_recorded_delivery);
    let status_refs =
        input.status_values.iter().map(crate::preserves_rail::canonical_hash).collect::<Result<Vec<_>>>()?;
    let receipt_checks = checks_with_extra(input.result_checks, &[
        ("assignment-bound", "pass"),
        ("status-log-bound", "pass"),
        ("worker-result-bound", "pass"),
        ("transport-is-not-authority", "pass"),
    ]);
    let receipt_value = job_worker_receipt_value(WorkerReceiptValueInput {
        decision: final_decision,
        request: Some(&input.input.request),
        assignment_ref: &input.input.assignment_ref,
        status_refs: &status_refs,
        result_ref: &input.result.result_ref,
        execution_receipt_ref: input.outputs.execution_receipt_ref.as_deref(),
        delivery_log_ref: input.input.delivery.delivery_log_ref.as_deref(),
        diagnostics: input.diagnostics,
        checks: &receipt_checks,
    })?;
    Ok(WorkerReceipt {
        receipt_ref: crate::preserves_rail::canonical_hash(&receipt_value)?,
        receipt_value,
    })
}

fn push_delivery_checks(
    input: &JobWorkerExecuteInput<'_>,
    request: &JobWorkerRequest,
    buffers: &mut DeliveryCheckBuffers,
) -> (Option<String>, bool) {
    let has_message_operation = input.delivery.envelope.operation == crate::remote_dataspace::Operation::Message;
    buffers.push("remote-dataspace-message", has_message_operation);
    if !has_message_operation {
        buffers.note("job worker request was not delivered as a remote dataspace message");
    }

    let has_target_binding = input.delivery.envelope.to_peer == request.target_peer;
    buffers.push("target-peer-binding", has_target_binding);
    if !has_target_binding {
        buffers.note(format!(
            "job worker envelope target {} does not match request target {}",
            input.delivery.envelope.to_peer, request.target_peer
        ));
    }

    let delivery_log_ref = input.delivery_log.map(|log| log.log_ref.clone());
    let has_recorded_delivery = input.delivery_log.is_some_and(|log| {
        log.replayable
            && log.entries.iter().any(|entry| entry.envelope.envelope_ref == input.delivery.envelope.envelope_ref)
    });
    buffers.push("recorded-delivery-log", has_recorded_delivery);
    if !has_recorded_delivery {
        buffers.note("job worker delivery log is missing, non-replayable, or does not bind request envelope");
    }
    (delivery_log_ref, has_recorded_delivery)
}

fn push_input_checks(
    input: &JobWorkerExecuteInput<'_>,
    request: &JobWorkerRequest,
    buffers: &mut DeliveryCheckBuffers,
) -> Result<(JobExecutionRequest, JobAdmissionReceipt)> {
    let execution_request_ref = crate::preserves_rail::canonical_hash(input.execution_request_value)?;
    let has_execution_request_ref = execution_request_ref == request.execution_request_ref;
    buffers.push("execution-request-ref-binding", has_execution_request_ref);
    if !has_execution_request_ref {
        buffers.note(format!(
            "job worker execution request hashes to {execution_request_ref}, expected {}",
            request.execution_request_ref
        ));
    }
    let execution_request = parse_job_execution_request_value(input.execution_request_value)?;

    let admission_ref = crate::preserves_rail::canonical_hash(input.admission_receipt_value)?;
    let has_admission_ref = admission_ref == request.admission_ref;
    buffers.push("admission-ref-binding", has_admission_ref);
    if !has_admission_ref {
        buffers.note(format!(
            "job worker admission receipt hashes to {admission_ref}, expected {}",
            request.admission_ref
        ));
    }
    let admission = parse_job_admission_receipt_value(input.admission_receipt_value)?;
    Ok((execution_request, admission))
}

fn push_binding_checks(
    request: &JobWorkerRequest,
    execution_request: &JobExecutionRequest,
    admission: &JobAdmissionReceipt,
    buffers: &mut DeliveryCheckBuffers,
) {
    let has_job_binding = request.job_ref == execution_request.job_ref && request.job_ref == admission.job_ref;
    buffers.push("job-ref-binding", has_job_binding);
    if !has_job_binding {
        buffers.note("job worker request, execution request, and admission job refs diverge");
    }

    let has_sync_binding = request.sync_ref == admission.sync_ref;
    buffers.push("sync-ref-binding", has_sync_binding);
    if !has_sync_binding {
        buffers.note(format!(
            "job worker sync ref {} does not match admission sync {}",
            request.sync_ref, admission.sync_ref
        ));
    }

    let has_execution_admission_binding = execution_request.admission_ref == request.admission_ref;
    buffers.push("execution-admission-ref-binding", has_execution_admission_binding);
    if !has_execution_admission_binding {
        buffers.note("job worker execution request does not bind worker admission ref");
    }

    let is_stage_binding = if request.stage_ids.is_empty() {
        execution_request.stage_ids == admission.stage_order
    } else {
        request.stage_ids == execution_request.stage_ids && request.stage_ids == admission.stage_order
    };
    buffers.push("selected-stage-binding", is_stage_binding);
    if !is_stage_binding {
        buffers.note("job worker selected stages do not match execution/admission stage order");
    }

    let has_evidence_refs = request.evidence_refs.iter().any(|reference| reference == &request.sync_ref)
        && request.evidence_refs.iter().any(|reference| reference == &request.admission_ref)
        && request.evidence_refs.iter().any(|reference| reference == &request.execution_request_ref);
    buffers.push("sync-admission-execution-evidence", has_evidence_refs);
    if !has_evidence_refs {
        buffers.note("job worker evidence refs must bind sync, admission, and execution request refs");
    }

    let has_transport_not_authority = !request.peer_bootstrap_refs.is_empty() && !request.node_identity_refs.is_empty();
    buffers.push("peer-bootstrap-node-identity-binding", has_transport_not_authority);
    if !has_transport_not_authority {
        buffers.note("job worker requires peer bootstrap and node identity evidence separate from transport");
    }
}

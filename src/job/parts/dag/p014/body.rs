
fn blob_ref_job_receipt_value(input: BlobRefReceiptValueInput<'_>) -> Result<IoValue> {
    validate_worker_decision(input.decision)?;
    validate_refs(input.status_refs, "job ref receipt status ref")?;
    validate_refs(input.verify_refs, "job ref receipt verify ref")?;
    validate_refs(input.fetch_refs, "job ref receipt fetch ref")?;
    validate_refs(input.pin_refs, "job ref receipt pin ref")?;
    validate_refs(input.cleanup_refs, "job ref receipt cleanup ref")?;
    if let Some(output_manifest_ref) = input.output_manifest_ref {
        validate_ref(output_manifest_ref, "job ref receipt output manifest ref")?;
    }
    if let Some(output_put_ref) = input.output_put_ref {
        validate_ref(output_put_ref, "job ref receipt output put ref")?;
    }
    let mut refs = vec![
        input.submission.submission_ref.clone(),
        input.submission.operation_id.clone(),
        input.submission.executable.content_ref.clone(),
        input.submission.context_ref.clone(),
    ];
    for content in &input.submission.inputs {
        push_bounded(&mut refs, content.content_ref.clone(), MAX_JOB_REFS, "job ref receipt refs")?;
    }
    extend_cloned_bounded(&mut refs, input.status_refs, MAX_JOB_REFS, "job ref receipt refs")?;
    extend_cloned_bounded(&mut refs, input.verify_refs, MAX_JOB_REFS, "job ref receipt refs")?;
    extend_cloned_bounded(&mut refs, input.fetch_refs, MAX_JOB_REFS, "job ref receipt refs")?;
    extend_cloned_bounded(&mut refs, input.pin_refs, MAX_JOB_REFS, "job ref receipt refs")?;
    extend_cloned_bounded(&mut refs, input.cleanup_refs, MAX_JOB_REFS, "job ref receipt refs")?;
    extend_cloned_bounded(&mut refs, &input.submission.policy_refs, MAX_JOB_REFS, "job ref receipt refs")?;
    extend_cloned_bounded(&mut refs, &input.submission.provenance_refs, MAX_JOB_REFS, "job ref receipt refs")?;
    extend_cloned_bounded(&mut refs, &input.submission.evidence_refs, MAX_JOB_REFS, "job ref receipt refs")?;
    if let Some(output_manifest_ref) = input.output_manifest_ref {
        push_bounded(&mut refs, output_manifest_ref.to_string(), MAX_JOB_REFS, "job ref receipt refs")?;
    }
    if let Some(output_put_ref) = input.output_put_ref {
        push_bounded(&mut refs, output_put_ref.to_string(), MAX_JOB_REFS, "job ref receipt refs")?;
    }
    let mut checks = input.checks.to_vec();
    checks.push(("canonical-receipt", "pass"));
    Ok(crate::preserves_rail::record("job-ref-receipt-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::JOB_REF_RECEIPT_SCHEMA),
        crate::preserves_rail::record("operation", vec![crate::preserves_rail::string("blob-ref-worker-execute")]),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(input.decision)]),
        crate::preserves_rail::record("submission", vec![crate::preserves_rail::string(
            &input.submission.submission_ref,
        )]),
        crate::preserves_rail::record("job-id", vec![crate::preserves_rail::string(&input.submission.job_id)]),
        crate::preserves_rail::record("operation-id", vec![crate::preserves_rail::string(
            &input.submission.operation_id,
        )]),
        crate::preserves_rail::record("executable", vec![crate::preserves_rail::string(
            &input.submission.executable.content_ref,
        )]),
        crate::preserves_rail::record("inputs", vec![refs_sequence(
            &input.submission.inputs.iter().map(|content| content.content_ref.clone()).collect::<Vec<_>>(),
        )]),
        crate::preserves_rail::record("status", vec![refs_sequence(input.status_refs)]),
        crate::preserves_rail::record("verify", vec![refs_sequence(input.verify_refs)]),
        crate::preserves_rail::record("fetch", vec![refs_sequence(input.fetch_refs)]),
        crate::preserves_rail::record("pins", vec![refs_sequence(input.pin_refs)]),
        crate::preserves_rail::record("cleanup", vec![refs_sequence(input.cleanup_refs)]),
        crate::preserves_rail::record("output", vec![optional_ref_value(input.output_manifest_ref)]),
        crate::preserves_rail::record("output-put", vec![optional_ref_value(input.output_put_ref)]),
        crate::preserves_rail::record("diagnostics", vec![crate::preserves_rail::sequence(
            input.diagnostics.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("refs", vec![refs_sequence(&sorted_unique(&refs))]),
        checks_value_from_pairs(&checks),
    ]))
}

fn fetch_blob_ref_job_content(
    chunk_root: &FilePath,
    content: &JobContentRef,
    verify_refs: &mut impl crate::bounded::VecSink<String>,
    fetch_refs: &mut impl crate::bounded::VecSink<String>,
    pin_refs: &mut impl crate::bounded::VecSink<String>,
) -> Result<Vec<u8>> {
    let manifest = crate::chunk_store::read_manifest(chunk_root, &content.content_ref)?;
    if manifest.total_len != content.size {
        return Err(MoltenError::invalid_harness(format!(
            "job content {} size hint {} does not match manifest length {}",
            content.content_ref, content.size, manifest.total_len
        )));
    }
    let verify = crate::chunk_store::verify_manifest(chunk_root, &content.content_ref)?;
    push_bounded(
        verify_refs,
        crate::preserves_rail::canonical_hash(&verify.receipt_value)?,
        MAX_JOB_REFS,
        "job ref verify refs",
    )?;
    let read = crate::chunk_store::read_object(chunk_root, &content.content_ref)?;
    push_bounded(
        fetch_refs,
        crate::preserves_rail::canonical_hash(&read.receipt_value)?,
        MAX_JOB_REFS,
        "job ref fetch refs",
    )?;
    let pin = crate::chunk_store::pin_manifest(chunk_root, &content.content_ref)?;
    push_bounded(
        pin_refs,
        crate::preserves_rail::canonical_hash(&pin.receipt_value)?,
        MAX_JOB_REFS,
        "job ref pin refs",
    )?;
    Ok(read.bytes)
}

fn run_blob_ref_job_handler(submission: &BlobRefJobSubmission, input_bytes: &[Vec<u8>]) -> Result<Vec<u8>> {
    match submission.handler_profile.as_str() {
        "local-echo-v1" => {
            let total_len = input_bytes.iter().try_fold(0usize, |sum, bytes| {
                checked_count_sum(sum, bytes.len(), MAX_JOB_STAGE_VALUES * MAX_JOB_STAGE_VALUES, "job ref output bytes")
            })?;
            let mut output = Vec::with_capacity(total_len);
            for bytes in input_bytes {
                output.extend_from_slice(bytes);
            }
            Ok(output)
        }
        other => Err(MoltenError::invalid_harness(format!("unsupported job ref handler profile {other}"))),
    }
}

fn import_blob_ref_job_artifacts(ledger_root: &FilePath, statuses: &[IoValue], receipt_value: &IoValue) -> Result<()> {
    for status_value in statuses {
        crate::ledger::import_artifact(ledger_root, status_value)?;
    }
    crate::ledger::import_artifact(ledger_root, receipt_value)?;
    Ok(())
}

fn push_check(checks: &mut impl crate::bounded::VecSink<(&'static str, &'static str)>, name: &'static str, ok: bool) {
    checks.push_item((name, status(ok)));
}

fn checks_with_extra(
    checks: &[(&'static str, &'static str)],
    extra: &[(&'static str, &'static str)],
) -> Vec<(&'static str, &'static str)> {
    let mut merged = checks.to_vec();
    merged.extend_from_slice(extra);
    merged
}

struct DeliveryChecks {
    diagnostics: Vec<String>,
    checks: Vec<(&'static str, &'static str)>,
    delivery_log_ref: Option<String>,
    has_recorded_delivery: bool,
    is_preliminary_pass: bool,
}

struct WorkerRun {
    status_values: Vec<IoValue>,
    execution: Option<JobExecutionLoopback>,
}

struct WorkerOutputs {
    execution_receipt_ref: Option<String>,
    output_refs: Vec<String>,
    stage_receipt_refs: Vec<(String, String)>,
    resource_receipt_refs: Vec<String>,
    is_execution_pass: bool,
}

struct FinishDeliveryInput<'a> {
    input: JobWorkerExecuteInput<'a>,
    request: JobWorkerRequest,
    assignment_value: IoValue,
    assignment_ref: String,
    delivery: DeliveryChecks,
    run: WorkerRun,
}

#[derive(Default)]
struct DeliveryCheckBuffers {
    diagnostics: Vec<String>,
    checks: Vec<(&'static str, &'static str)>,
}

impl DeliveryCheckBuffers {
    fn push(&mut self, name: &'static str, ok: bool) {
        push_check(&mut self.checks, name, ok);
    }

    fn note(&mut self, value: impl Into<String>) {
        self.diagnostics.push(value.into());
    }

    fn finish(self, delivery_log_ref: Option<String>, has_recorded_delivery: bool) -> DeliveryChecks {
        let is_preliminary_pass = self
            .checks
            .iter()
            .filter(|(name, _)| *name != "recorded-delivery-log")
            .all(|(_, value)| *value == "pass");
        DeliveryChecks {
            diagnostics: self.diagnostics,
            checks: self.checks,
            delivery_log_ref,
            has_recorded_delivery,
            is_preliminary_pass,
        }
    }
}

fn collect_delivery_checks(input: &JobWorkerExecuteInput<'_>, request: &JobWorkerRequest) -> Result<DeliveryChecks> {
    let mut buffers = DeliveryCheckBuffers::default();
    let (delivery_log_ref, has_recorded_delivery) = push_delivery_checks(input, request, &mut buffers);
    let (execution_request, admission) = push_input_checks(input, request, &mut buffers)?;
    push_binding_checks(request, &execution_request, &admission, &mut buffers);
    push_authority_checks(request, &admission, &mut buffers);
    push_target_state_check(request, &execution_request, &mut buffers)?;
    Ok(buffers.finish(delivery_log_ref, has_recorded_delivery))
}

fn run_worker_delivery(
    input: &JobWorkerExecuteInput<'_>,
    request: &JobWorkerRequest,
    delivery: &DeliveryChecks,
) -> Result<WorkerRun> {
    let mut status_values = vec![job_worker_status_value(WorkerStatusValueInput {
        request,
        delivery: input.delivery,
        state: "received",
        execution_receipt_ref: None,
        diagnostics: &delivery.diagnostics,
        checks: &[("request-envelope-bound", status(delivery.is_preliminary_pass))],
    })?];

    let execution = if delivery.is_preliminary_pass {
        let running_status = job_worker_status_value(WorkerStatusValueInput {
            request,
            delivery: input.delivery,
            state: "running",
            execution_receipt_ref: None,
            diagnostics: &[],
            checks: &[("target-side-admission-verified", "pass")],
        })?;
        push_bounded(&mut status_values, running_status, MAX_JOB_REFS, "job worker statuses")?;
        Some(execution_loopback(ExecutionLoopbackInput {
            target_registry: input.target_registry,
            storage_root: input.storage_root,
            cache_root: input.cache_root,
            chunk_root: input.chunk_root,
            admission_receipt_value: input.admission_receipt_value,
            request_value: input.execution_request_value,
        })?)
    } else {
        None
    };
    Ok(WorkerRun {
        status_values,
        execution,
    })
}

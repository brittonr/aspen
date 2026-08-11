
fn parse_job_content_ref_value(value: &IoValue) -> Result<JobContentRef> {
    let fields = simple_record(value, "job-content-ref", 4)?;
    let size_value = record_iovalue(&fields[1], "size")?;
    let content = JobContentRef {
        content_ref: record_ref(&fields[0], "content-ref")?,
        size: required_u64_value(&size_value, "job content size")?,
        format: record_string(&fields[2], "format")?,
        schema_ref: record_optional_ref(&fields[3], "schema")?,
    };
    validate_job_content_ref(&content, "job content ref")?;
    Ok(content)
}

fn validate_blob_ref_submission_input(input: &BlobRefJobSubmissionValueInput<'_>) -> Result<()> {
    validate_non_empty(input.job_id, "job ref submission job id")?;
    validate_ref(input.operation_id, "job ref submission operation id")?;
    validate_job_content_ref(&input.executable, "job ref executable")?;
    ensure_count_at_most(input.inputs.len(), MAX_JOB_REFS, "job ref submission inputs")?;
    for content in &input.inputs {
        validate_job_content_ref(content, "job ref input")?;
    }
    validate_output_mode(input.output_mode)?;
    validate_blob_ref_handler_profile(input.handler_profile)?;
    validate_ref(input.context_ref, "job ref authority context ref")?;
    validate_refs(input.input_schema_refs, "job ref input schema ref")?;
    validate_refs(input.output_schema_refs, "job ref output schema ref")?;
    validate_refs(input.effect_manifest_refs, "job ref effect manifest ref")?;
    validate_refs(input.policy_refs, "job ref policy ref")?;
    validate_refs(input.provenance_refs, "job ref provenance ref")?;
    validate_refs(input.evidence_refs, "job ref evidence ref")?;
    Ok(())
}

fn validate_blob_ref_submission(submission: &BlobRefJobSubmission) -> Result<()> {
    validate_non_empty(&submission.job_id, "job ref submission job id")?;
    validate_ref(&submission.operation_id, "job ref submission operation id")?;
    validate_job_content_ref(&submission.executable, "job ref executable")?;
    ensure_count_at_most(submission.inputs.len(), MAX_JOB_REFS, "job ref submission inputs")?;
    for content in &submission.inputs {
        validate_job_content_ref(content, "job ref input")?;
    }
    validate_output_mode(&submission.output_mode)?;
    validate_blob_ref_handler_profile(&submission.handler_profile)?;
    validate_ref(&submission.context_ref, "job ref authority context ref")?;
    validate_refs(&submission.input_schema_refs, "job ref input schema ref")?;
    validate_refs(&submission.output_schema_refs, "job ref output schema ref")?;
    validate_refs(&submission.effect_manifest_refs, "job ref effect manifest ref")?;
    validate_refs(&submission.policy_refs, "job ref policy ref")?;
    validate_refs(&submission.provenance_refs, "job ref provenance ref")?;
    validate_refs(&submission.evidence_refs, "job ref evidence ref")?;
    Ok(())
}

fn validate_job_content_ref(content: &JobContentRef, field: &str) -> Result<()> {
    if content.size > MAX_JOB_INLINE_BYTES && content.content_ref.is_empty() {
        return Err(MoltenError::invalid_harness("large job content must use a content ref"));
    }
    validate_ref(&content.content_ref, field)?;
    validate_non_empty(&content.format, "job content format")?;
    if let Some(schema_ref) = content.schema_ref.as_ref() {
        validate_ref(schema_ref, "job content schema ref")?;
    }
    Ok(())
}

fn validate_output_mode(output_mode: &str) -> Result<()> {
    if output_mode == "chunk-manifest" {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("unsupported job ref output mode {output_mode}")))
    }
}

fn validate_blob_ref_handler_profile(handler_profile: &str) -> Result<()> {
    if handler_profile == "local-echo-v1" {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("unsupported job ref handler profile {handler_profile}")))
    }
}

fn validate_blob_ref_state(state: &str) -> Result<()> {
    if matches!(state, "queued" | "fetching" | "running" | "result-ready" | "complete" | "failed" | "cancelled") {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("unsupported job ref status state {state}")))
    }
}

// r[impl molten.blob_ref_jobs.no_inline_large_bytes]
fn reject_blob_ref_job_inline_tokens(value: &IoValue) -> Result<()> {
    let text = crate::preserves_rail::to_text(value)?;
    for token in ["inline-bytes", "inline-executable", "inline-dataset"] {
        if text.contains(token) {
            return Err(MoltenError::invalid_harness(format!(
                "job ref submission must use content refs, found inline token {token}"
            )));
        }
    }
    Ok(())
}

fn validate_stage_kind(kind: &str) -> Result<()> {
    if matches!(kind, "source" | "map" | "filter" | "reduce" | "materialize") {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("unsupported job stage kind {kind}")))
    }
}

fn validate_stage_operation(operation: &str) -> Result<()> {
    if matches!(
        operation,
        "source"
            | "materialize"
            | "identity"
            | "wrap"
            | "tag-record"
            | "project-field"
            | "keep-all"
            | "drop-all"
            | "equals"
            | "match-record"
            | "count"
            | "sum-u64"
            | "sum-integers"
            | "concat-lists"
    ) {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("unsupported job stage operation {operation}")))
    }
}

fn validate_partitioning(partitioning: &str) -> Result<()> {
    if matches!(partitioning, "single" | "partitioned") {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("unsupported job edge partitioning {partitioning}")))
    }
}

fn validate_materialization(materialization: &str) -> Result<()> {
    if matches!(materialization, "stream" | "typed-ref" | "content-ref") {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("unsupported job edge materialization {materialization}")))
    }
}

fn worker_stage_receipts(stage_order: &[String], receipt_refs: &[String]) -> Result<Vec<(String, String)>> {
    ensure_count_at_most(stage_order.len(), MAX_JOB_NODES, "job worker stage receipts")?;
    ensure_count_at_most(receipt_refs.len(), MAX_JOB_NODES, "job worker stage receipts")?;
    let mut stage_receipts = Vec::with_capacity(stage_order.len().min(receipt_refs.len()));
    for (stage_id, receipt_ref) in stage_order.iter().zip(receipt_refs.iter()) {
        validate_node_id(stage_id)?;
        validate_ref(receipt_ref, "job worker stage receipt ref")?;
        push_bounded(
            &mut stage_receipts,
            (stage_id.clone(), receipt_ref.clone()),
            MAX_JOB_NODES,
            "job worker stage receipts",
        )?;
    }
    Ok(stage_receipts)
}

fn validate_stage_receipt_refs(stage_receipts: &[(String, String)]) -> Result<()> {
    ensure_count_at_most(stage_receipts.len(), MAX_JOB_NODES, "job worker stage receipts")?;
    for (stage_id, receipt_ref) in stage_receipts {
        validate_node_id(stage_id)?;
        validate_ref(receipt_ref, "job worker stage receipt ref")?;
    }
    Ok(())
}

fn record_stage_receipt_sequence(value: &Value<IoValue>, label: &str) -> Result<Vec<(String, String)>> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    let items = required_sequence(&record[0], label)?;
    ensure_count_at_most(items.len(), MAX_JOB_NODES, label)?;
    let mut stage_receipts = Vec::with_capacity(items.len());
    for item in items.iter() {
        let item = crate::preserves_rail::value_to_iovalue(item);
        let stage = simple_record(&item, "stage", 2)?;
        let stage_id = required_string(&stage[0], "job worker stage id")?;
        validate_node_id(&stage_id)?;
        let receipt_ref = required_ref(&stage[1], "job worker stage receipt ref")?;
        push_bounded(&mut stage_receipts, (stage_id, receipt_ref), MAX_JOB_NODES, label)?;
    }
    Ok(stage_receipts)
}

fn import_worker_artifacts(
    ledger_root: &FilePath,
    assignment_value: &IoValue,
    status_values: &[IoValue],
    result_value: &IoValue,
    receipt_value: &IoValue,
) -> Result<()> {
    crate::ledger::import_artifact(ledger_root, assignment_value)?;
    for status_value in status_values {
        crate::ledger::import_artifact(ledger_root, status_value)?;
    }
    crate::ledger::import_artifact(ledger_root, result_value)?;
    crate::ledger::import_artifact(ledger_root, receipt_value)?;
    Ok(())
}

fn reject_worker_ambient_tokens(value: &IoValue) -> Result<()> {
    if let Some(marker) = crate::preserves_rail::find_ambient_job_token(value)? {
        Err(MoltenError::invalid_harness(format!(
            "job worker request contains mobile/ambient token {}",
            marker.token
        )))
    } else {
        Ok(())
    }
}

fn validate_request_materialization(materialization: &str) -> Result<()> {
    if matches!(materialization, "inline" | "typed-storage" | "chunk-manifest") {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("unsupported job output materialization {materialization}")))
    }
}

fn validate_receipt_operation(operation: &str) -> Result<()> {
    if matches!(operation, "install" | "run" | "stage" | "memo-hit" | "memo-miss" | "materialize" | "deny") {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("unsupported job receipt operation {operation}")))
    }
}

fn validate_decision(decision: &str) -> Result<()> {
    if matches!(decision, "pass" | "deny") {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("unsupported job receipt decision {decision}")))
    }
}

fn validate_worker_decision(decision: &str) -> Result<()> {
    if matches!(decision, "pass" | "deny" | "non-replayable") {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("unsupported job worker decision {decision}")))
    }
}

fn validate_worker_state(state: &str) -> Result<()> {
    if matches!(state, "received" | "running" | "completed" | "denied" | "non-replayable") {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("unsupported job worker state {state}")))
    }
}

fn validate_node_id(id: &str) -> Result<()> {
    validate_non_empty(id, "job node id")?;
    if id.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == '.') {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!(
            "job node id {id} must use ascii alphanumeric, '-', '_' or '.'"
        )))
    }
}

fn validate_ref(value_ref: &str, field: &str) -> Result<()> {
    validate_non_empty(value_ref, field)?;
    crate::preserves_rail::validate_content_ref(value_ref).map_err(|error| {
        MoltenError::invalid_harness(format!("{field} must be a canonical blake3 content ref: {error}"))
    })
}

fn validate_refs(refs: &[String], field: &str) -> Result<()> {
    for value_ref in refs {
        validate_ref(value_ref, field)?;
    }
    Ok(())
}

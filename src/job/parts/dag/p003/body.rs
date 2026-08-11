
pub fn parse_job_sync_request_value(value: &IoValue) -> Result<JobSyncRequest> {
    let fields = value
        .collect_simple_record("job-sync-request-v1", Some(8))
        .ok_or_else(|| MoltenError::invalid_harness("expected <job-sync-request-v1 ...>"))?;
    require_schema(&fields[0], crate::preserves_rail::JOB_SYNC_REQUEST_SCHEMA, "job sync request")?;
    let checks = parse_checks(&fields[7])?;
    require_check(&checks, "no-execution", "job sync request")?;
    Ok(JobSyncRequest {
        request_ref: crate::preserves_rail::canonical_hash(value)?,
        job_ref: record_ref(&fields[1], "job")?,
        stage_ids: record_node_id_sequence(&fields[2], "stages")?,
        target_peer: record_string(&fields[3], "target-peer")?,
        policy_refs: record_ref_sequence(&fields[4], "policy")?,
        capability_refs: record_ref_sequence(&fields[5], "capability")?,
        evidence_refs: record_ref_sequence(&fields[6], "evidence")?,
        value: value.clone(),
    })
}

pub fn job_admission_request_value(input: AdmissionRequestValueInput<'_>) -> Result<IoValue> {
    validate_ref(input.job_ref, "job admission request job ref")?;
    validate_ref(input.sync_ref, "job admission request sync ref")?;
    for stage_id in input.stage_ids {
        validate_node_id(stage_id)?;
    }
    validate_non_empty(input.target_peer, "job admission target peer")?;
    validate_refs(input.policy_refs, "job admission policy ref")?;
    validate_refs(input.capability_refs, "job admission capability ref")?;
    validate_refs(input.evidence_refs, "job admission evidence ref")?;
    validate_refs(input.resource_refs, "job admission resource ref")?;
    Ok(crate::preserves_rail::record("job-admission-request-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::JOB_ADMISSION_REQUEST_SCHEMA),
        crate::preserves_rail::record("job", vec![crate::preserves_rail::string(input.job_ref)]),
        crate::preserves_rail::record("sync", vec![crate::preserves_rail::string(input.sync_ref)]),
        crate::preserves_rail::record("stages", vec![crate::preserves_rail::sequence(
            input.stage_ids.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("target-peer", vec![crate::preserves_rail::string(input.target_peer)]),
        crate::preserves_rail::record("policy", vec![refs_sequence(&sorted_unique(input.policy_refs))]),
        crate::preserves_rail::record("capability", vec![refs_sequence(&sorted_unique(input.capability_refs))]),
        crate::preserves_rail::record("evidence", vec![refs_sequence(&sorted_unique(input.evidence_refs))]),
        crate::preserves_rail::record("resource", vec![refs_sequence(&sorted_unique(input.resource_refs))]),
        checks_value(&["target-side-admission", "no-execution", "full-ref-identity"]),
    ]))
}

pub fn parse_job_admission_request_value(value: &IoValue) -> Result<JobAdmissionRequest> {
    let fields = value
        .collect_simple_record("job-admission-request-v1", Some(10))
        .ok_or_else(|| MoltenError::invalid_harness("expected <job-admission-request-v1 ...>"))?;
    require_schema(&fields[0], crate::preserves_rail::JOB_ADMISSION_REQUEST_SCHEMA, "job admission request")?;
    let checks = parse_checks(&fields[9])?;
    require_check(&checks, "no-execution", "job admission request")?;
    Ok(JobAdmissionRequest {
        request_ref: crate::preserves_rail::canonical_hash(value)?,
        job_ref: record_ref(&fields[1], "job")?,
        sync_ref: record_ref(&fields[2], "sync")?,
        stage_ids: record_node_id_sequence(&fields[3], "stages")?,
        target_peer: record_string(&fields[4], "target-peer")?,
        policy_refs: record_ref_sequence(&fields[5], "policy")?,
        capability_refs: record_ref_sequence(&fields[6], "capability")?,
        evidence_refs: record_ref_sequence(&fields[7], "evidence")?,
        resource_refs: record_ref_sequence(&fields[8], "resource")?,
        value: value.clone(),
    })
}

pub fn job_execution_request_value(input: ExecutionRequestValueInput<'_>) -> Result<IoValue> {
    validate_ref(input.job_ref, "job execution request job ref")?;
    validate_ref(input.admission_ref, "job execution admission receipt ref")?;
    for stage_id in input.stage_ids {
        validate_node_id(stage_id)?;
    }
    validate_non_empty(input.target_peer, "job execution target peer")?;
    validate_ref(input.storage_profile_ref, "job execution storage profile ref")?;
    validate_ref(input.cache_profile_ref, "job execution cache profile ref")?;
    validate_ref(input.chunk_profile_ref, "job execution chunk profile ref")?;
    validate_refs(input.policy_refs, "job execution policy ref")?;
    validate_refs(input.capability_refs, "job execution capability ref")?;
    validate_refs(input.resource_refs, "job execution resource ref")?;
    Ok(crate::preserves_rail::record("job-execution-request-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::JOB_EXECUTION_REQUEST_SCHEMA),
        crate::preserves_rail::record("job", vec![crate::preserves_rail::string(input.job_ref)]),
        crate::preserves_rail::record("admission", vec![crate::preserves_rail::string(input.admission_ref)]),
        crate::preserves_rail::record("target-peer", vec![crate::preserves_rail::string(input.target_peer)]),
        crate::preserves_rail::record("stages", vec![crate::preserves_rail::sequence(
            input.stage_ids.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("storage", vec![crate::preserves_rail::string(input.storage_profile_ref)]),
        crate::preserves_rail::record("cache", vec![crate::preserves_rail::string(input.cache_profile_ref)]),
        crate::preserves_rail::record("chunks", vec![crate::preserves_rail::string(input.chunk_profile_ref)]),
        crate::preserves_rail::record("policy", vec![refs_sequence(&sorted_unique(input.policy_refs))]),
        crate::preserves_rail::record("capability", vec![refs_sequence(&sorted_unique(input.capability_refs))]),
        crate::preserves_rail::record("resource", vec![refs_sequence(&sorted_unique(input.resource_refs))]),
        checks_value(&[
            "admission-required",
            "target-state-only",
            "no-source-registry",
            "full-ref-identity",
        ]),
    ]))
}

pub fn parse_job_execution_request_value(value: &IoValue) -> Result<JobExecutionRequest> {
    let fields = value
        .collect_simple_record("job-execution-request-v1", Some(12))
        .ok_or_else(|| MoltenError::invalid_harness("expected <job-execution-request-v1 ...>"))?;
    require_schema(&fields[0], crate::preserves_rail::JOB_EXECUTION_REQUEST_SCHEMA, "job execution request")?;
    let checks = parse_checks(&fields[11])?;
    require_check(&checks, "admission-required", "job execution request")?;
    require_check(&checks, "target-state-only", "job execution request")?;
    require_check(&checks, "no-source-registry", "job execution request")?;
    Ok(JobExecutionRequest {
        request_ref: crate::preserves_rail::canonical_hash(value)?,
        job_ref: record_ref(&fields[1], "job")?,
        admission_ref: record_ref(&fields[2], "admission")?,
        target_peer: record_string(&fields[3], "target-peer")?,
        stage_ids: record_node_id_sequence(&fields[4], "stages")?,
        storage_profile_ref: record_ref(&fields[5], "storage")?,
        cache_profile_ref: record_ref(&fields[6], "cache")?,
        chunk_profile_ref: record_ref(&fields[7], "chunks")?,
        policy_refs: record_ref_sequence(&fields[8], "policy")?,
        capability_refs: record_ref_sequence(&fields[9], "capability")?,
        resource_refs: record_ref_sequence(&fields[10], "resource")?,
        value: value.clone(),
    })
}

pub fn job_content_ref_value(content: &JobContentRef) -> Result<IoValue> {
    validate_job_content_ref(content, "job content ref")?;
    Ok(crate::preserves_rail::record("job-content-ref", vec![
        crate::preserves_rail::record("content-ref", vec![crate::preserves_rail::string(&content.content_ref)]),
        crate::preserves_rail::record("size", vec![crate::preserves_rail::u64_value(content.size)]),
        crate::preserves_rail::record("format", vec![crate::preserves_rail::string(&content.format)]),
        crate::preserves_rail::record("schema", vec![optional_ref_value(content.schema_ref.as_deref())]),
    ]))
}

// r[impl molten.blob_ref_jobs.payload_model]
pub fn job_ref_submission_value(input: BlobRefJobSubmissionValueInput<'_>) -> Result<IoValue> {
    validate_blob_ref_submission_input(&input)?;
    let input_values = input.inputs.iter().map(job_content_ref_value).collect::<Result<Vec<_>>>()?;
    Ok(crate::preserves_rail::record("job-ref-submission-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::JOB_REF_SUBMISSION_SCHEMA),
        crate::preserves_rail::record("job-id", vec![crate::preserves_rail::string(input.job_id)]),
        crate::preserves_rail::record("operation-id", vec![crate::preserves_rail::string(input.operation_id)]),
        crate::preserves_rail::record("executable", vec![job_content_ref_value(&input.executable)?]),
        crate::preserves_rail::record("inputs", vec![crate::preserves_rail::sequence(input_values)]),
        crate::preserves_rail::record("output-mode", vec![crate::preserves_rail::string(input.output_mode)]),
        crate::preserves_rail::record("input-schemas", vec![refs_sequence(&sorted_unique(input.input_schema_refs))]),
        crate::preserves_rail::record("output-schemas", vec![refs_sequence(&sorted_unique(input.output_schema_refs))]),
        crate::preserves_rail::record("effects", vec![refs_sequence(&sorted_unique(input.effect_manifest_refs))]),
        crate::preserves_rail::record("handler-profile", vec![crate::preserves_rail::string(input.handler_profile)]),
        crate::preserves_rail::record("authority", vec![crate::preserves_rail::string(input.context_ref)]),
        crate::preserves_rail::record("policy", vec![refs_sequence(&sorted_unique(input.policy_refs))]),
        crate::preserves_rail::record("provenance", vec![refs_sequence(&sorted_unique(input.provenance_refs))]),
        crate::preserves_rail::record("evidence", vec![refs_sequence(&sorted_unique(input.evidence_refs))]),
        checks_value(&[
            "content-refs-only",
            "no-inline-large-bytes",
            "handler-profile-declared",
            "authority-context-declared",
            "full-ref-identity",
        ]),
    ]))
}

pub fn parse_job_ref_submission_value(value: &IoValue) -> Result<BlobRefJobSubmission> {
    reject_blob_ref_job_inline_tokens(value)?;
    let fields = value
        .collect_simple_record("job-ref-submission-v1", Some(15))
        .ok_or_else(|| MoltenError::invalid_harness("expected <job-ref-submission-v1 ...>"))?;
    require_schema(&fields[0], crate::preserves_rail::JOB_REF_SUBMISSION_SCHEMA, "job ref submission")?;
    let checks = parse_checks(&fields[14])?;
    require_check(&checks, "content-refs-only", "job ref submission")?;
    require_check(&checks, "no-inline-large-bytes", "job ref submission")?;
    let executable = parse_job_content_ref_record(&fields[3], "executable")?;
    let input_values = record_sequence_values(&fields[4], "inputs")?;
    let mut inputs = Vec::with_capacity(input_values.len());
    for input_value in &input_values {
        push_bounded(
            &mut inputs,
            parse_job_content_ref_value(input_value)?,
            MAX_JOB_REFS,
            "job ref submission inputs",
        )?;
    }
    let submission = BlobRefJobSubmission {
        submission_ref: crate::preserves_rail::canonical_hash(value)?,
        job_id: record_string(&fields[1], "job-id")?,
        operation_id: record_ref(&fields[2], "operation-id")?,
        executable,
        inputs,
        output_mode: record_string(&fields[5], "output-mode")?,
        input_schema_refs: record_ref_sequence(&fields[6], "input-schemas")?,
        output_schema_refs: record_ref_sequence(&fields[7], "output-schemas")?,
        effect_manifest_refs: record_ref_sequence(&fields[8], "effects")?,
        handler_profile: record_string(&fields[9], "handler-profile")?,
        context_ref: record_ref(&fields[10], "authority")?,
        policy_refs: record_ref_sequence(&fields[11], "policy")?,
        provenance_refs: record_ref_sequence(&fields[12], "provenance")?,
        evidence_refs: record_ref_sequence(&fields[13], "evidence")?,
        value: value.clone(),
    };
    validate_blob_ref_submission(&submission)?;
    Ok(submission)
}

// r[impl molten.blob_ref_jobs.local_worker]
// r[impl molten.blob_ref_jobs.retention_pins]
// r[impl molten.blob_ref_jobs.local_tests]
// r[impl molten.blob_ref_jobs.property_tests]
pub fn execute_blob_ref_job(input: BlobRefJobExecuteInput<'_>) -> Result<BlobRefJobExecution> {
    let submission = parse_job_ref_submission_value(input.submission_value)?;
    let (preflight, mut diagnostics) = preflight(&submission)?;
    let mut status_values = vec![blob_ref_job_status_value(&submission, "queued", &[], &[(
        "submission-valid",
        "pass",
    )])?];
    push_bounded(
        &mut status_values,
        blob_ref_job_status_value(&submission, "fetching", &[], &[("content-fetch-started", "pass")])?,
        MAX_JOB_REFS,
        "job ref status values",
    )?;
    let FetchOutcome {
        content_refs,
        input_bytes,
        mut verify_refs,
        fetch_refs,
        mut pin_refs,
        diagnostics: fetch_diagnostics,
        is_content_verified,
    } = fetch_content(input.chunk_root, &submission)?;
    for diagnostic in fetch_diagnostics {
        push_bounded(&mut diagnostics, diagnostic, MAX_JOB_REFS, "job ref diagnostics")?;
    }
    let has_preliminary_pass = preflight.has_policy
        && preflight.has_provenance
        && preflight.has_effect_manifest
        && preflight.has_supported_output_mode
        && preflight.has_supported_handler
        && is_content_verified;
    let (output_manifest_ref, output_put_ref) = if has_preliminary_pass {
        let output = run_output(input.chunk_root, &submission, &input_bytes)?;
        push_bounded(&mut verify_refs, output.verify_ref, MAX_JOB_REFS, "job ref verify refs")?;
        push_bounded(&mut pin_refs, output.pin_ref, MAX_JOB_REFS, "job ref pin refs")?;
        for status_value in output.status_values {
            push_bounded(&mut status_values, status_value, MAX_JOB_REFS, "job ref status values")?;
        }
        (Some(output.output_manifest_ref), Some(output.output_put_ref))
    } else {
        (None, None)
    };
    let cleanup_refs = cleanup_content(input.chunk_root, &content_refs)?;
    finish_run(FinishInput {
        ledger_root: input.ledger_root,
        submission,
        status_values,
        verify_refs,
        fetch_refs,
        pin_refs,
        cleanup_refs,
        output_manifest_ref,
        output_put_ref,
        diagnostics,
        preflight,
        is_content_verified,
        has_preliminary_pass,
    })
}

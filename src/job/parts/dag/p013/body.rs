
// r[impl molten.blob_ref_jobs.provenance_policy]
fn preflight(submission: &BlobRefJobSubmission) -> Result<(Preflight, Vec<String>)> {
    let preflight = Preflight {
        has_policy: !submission.policy_refs.is_empty(),
        has_provenance: !submission.provenance_refs.is_empty(),
        has_effect_manifest: !submission.effect_manifest_refs.is_empty(),
        has_supported_output_mode: submission.output_mode == "chunk-manifest",
        has_supported_handler: submission.handler_profile == "local-echo-v1",
    };
    let mut diagnostics = Vec::new();
    if !preflight.has_policy {
        push_bounded(
            &mut diagnostics,
            "job ref submission missing policy refs".to_string(),
            MAX_JOB_REFS,
            "job ref diagnostics",
        )?;
    }
    if !preflight.has_provenance {
        push_bounded(
            &mut diagnostics,
            "job ref submission missing executable provenance refs".to_string(),
            MAX_JOB_REFS,
            "job ref diagnostics",
        )?;
    }
    if !preflight.has_effect_manifest {
        push_bounded(
            &mut diagnostics,
            "job ref submission missing effect manifest refs".to_string(),
            MAX_JOB_REFS,
            "job ref diagnostics",
        )?;
    }
    if !preflight.has_supported_output_mode {
        push_bounded(
            &mut diagnostics,
            format!("unsupported job ref output mode {}", submission.output_mode),
            MAX_JOB_REFS,
            "job ref diagnostics",
        )?;
    }
    if !preflight.has_supported_handler {
        push_bounded(
            &mut diagnostics,
            format!("unsupported job ref handler profile {}", submission.handler_profile),
            MAX_JOB_REFS,
            "job ref diagnostics",
        )?;
    }
    Ok((preflight, diagnostics))
}

fn fetch_content(chunk_root: &FilePath, submission: &BlobRefJobSubmission) -> Result<FetchOutcome> {
    let mut content_refs = vec![submission.executable.clone()];
    extend_cloned_bounded(&mut content_refs, &submission.inputs, MAX_JOB_REFS, "job ref content refs")?;
    let mut input_bytes = Vec::new();
    let mut verify_refs = Vec::new();
    let mut fetch_refs = Vec::new();
    let mut pin_refs = Vec::new();
    let mut diagnostics = Vec::new();
    let mut is_content_verified = true;
    for (content_index, content) in content_refs.iter().enumerate() {
        let fetched = fetch_blob_ref_job_content(chunk_root, content, &mut verify_refs, &mut fetch_refs, &mut pin_refs);
        match fetched {
            Ok(bytes) => {
                if content_index > 0 {
                    push_bounded(&mut input_bytes, bytes, MAX_JOB_REFS, "job ref input byte sets")?;
                }
            }
            Err(error) => {
                is_content_verified = false;
                push_bounded(&mut diagnostics, error.to_string(), MAX_JOB_REFS, "job ref diagnostics")?;
            }
        }
    }
    Ok(FetchOutcome {
        content_refs,
        input_bytes,
        verify_refs,
        fetch_refs,
        pin_refs,
        diagnostics,
        is_content_verified,
    })
}

fn run_output(
    chunk_root: &FilePath,
    submission: &BlobRefJobSubmission,
    input_bytes: &[Vec<u8>],
) -> Result<OutputOutcome> {
    let mut status_values = Vec::new();
    push_bounded(
        &mut status_values,
        blob_ref_job_status_value(submission, "running", &[], &[("content-verified-before-run", "pass")])?,
        MAX_JOB_REFS,
        "job ref status values",
    )?;
    let output_bytes = run_blob_ref_job_handler(submission, input_bytes)?;
    let put = crate::chunk_store::put_bytes(chunk_root, "job-ref-result", &output_bytes, DEFAULT_FIXED_V1_CHUNK_SIZE)?;
    let output_put_ref = crate::preserves_rail::canonical_hash(&put.receipt_value)?;
    let output_verify = crate::chunk_store::verify_manifest(chunk_root, &put.manifest_ref)?;
    let verify_ref = crate::preserves_rail::canonical_hash(&output_verify.receipt_value)?;
    let output_pin = crate::chunk_store::pin_manifest(chunk_root, &put.manifest_ref)?;
    let pin_ref = crate::preserves_rail::canonical_hash(&output_pin.receipt_value)?;
    let output_manifest_ref = put.manifest_ref.clone();
    push_bounded(
        &mut status_values,
        blob_ref_job_status_value(submission, "result-ready", std::slice::from_ref(&output_manifest_ref), &[(
            "output-content-ref",
            "pass",
        )])?,
        MAX_JOB_REFS,
        "job ref status values",
    )?;
    Ok(OutputOutcome {
        output_manifest_ref,
        output_put_ref,
        verify_ref,
        pin_ref,
        status_values,
    })
}

fn cleanup_content(chunk_root: &FilePath, content_refs: &[JobContentRef]) -> Result<Vec<String>> {
    let mut cleanup_refs = Vec::new();
    for content in content_refs {
        if let Ok(unpin) = crate::chunk_store::unpin_manifest(chunk_root, &content.content_ref) {
            push_bounded(
                &mut cleanup_refs,
                crate::preserves_rail::canonical_hash(&unpin.receipt_value)?,
                MAX_JOB_REFS,
                "job ref cleanup refs",
            )?;
        }
    }
    Ok(cleanup_refs)
}

fn finish_run(input: FinishInput<'_>) -> Result<BlobRefJobExecution> {
    let FinishInput {
        ledger_root,
        submission,
        mut status_values,
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
    } = input;
    let final_decision = if has_preliminary_pass { "pass" } else { "deny" };
    let final_state = if has_preliminary_pass { "complete" } else { "failed" };
    push_bounded(
        &mut status_values,
        blob_ref_job_status_value(&submission, final_state, output_manifest_ref.as_slice(), &[(
            "terminal-status",
            "pass",
        )])?,
        MAX_JOB_REFS,
        "job ref status values",
    )?;
    let status_refs = status_values.iter().map(crate::preserves_rail::canonical_hash).collect::<Result<Vec<_>>>()?;
    let receipt_checks = final_checks(
        preflight,
        is_content_verified,
        output_manifest_ref.is_some(),
        !pin_refs.is_empty(),
        !cleanup_refs.is_empty(),
    );
    let receipt_value = blob_ref_job_receipt_value(BlobRefReceiptValueInput {
        decision: final_decision,
        submission: &submission,
        status_refs: &status_refs,
        verify_refs: &verify_refs,
        fetch_refs: &fetch_refs,
        pin_refs: &pin_refs,
        cleanup_refs: &cleanup_refs,
        output_manifest_ref: output_manifest_ref.as_deref(),
        output_put_ref: output_put_ref.as_deref(),
        diagnostics: &diagnostics,
        checks: &receipt_checks,
    })?;
    let receipt_ref = crate::preserves_rail::canonical_hash(&receipt_value)?;
    if let Some(ledger_root) = ledger_root {
        import_blob_ref_job_artifacts(ledger_root, &status_values, &receipt_value)?;
    }
    Ok(BlobRefJobExecution {
        submission,
        decision: final_decision.to_string(),
        status_values,
        output_manifest_ref,
        receipt_ref,
        receipt_value,
        diagnostics,
    })
}

fn final_checks(
    preflight: Preflight,
    is_content_verified: bool,
    has_output: bool,
    has_pins: bool,
    has_cleanup: bool,
) -> [(&'static str, &'static str); 10] {
    [
        ("content-refs-only", "pass"),
        ("no-inline-large-bytes", "pass"),
        ("content-verification-before-run", status(is_content_verified)),
        ("provenance-policy", status(preflight.has_policy && preflight.has_provenance)),
        ("effect-admission-policy", status(preflight.has_effect_manifest)),
        ("local-worker-handler", status(preflight.has_supported_handler)),
        ("output-content-ref", status(has_output)),
        ("retention-pins", status(has_pins)),
        ("cleanup-receipts", status(has_cleanup)),
        ("job-dag-ref-integration", "pass"),
    ]
}

fn blob_ref_job_status_value(
    submission: &BlobRefJobSubmission,
    state: &str,
    output_refs: &[String],
    checks: &[(&str, &str)],
) -> Result<IoValue> {
    validate_blob_ref_state(state)?;
    validate_refs(output_refs, "job ref status output ref")?;
    let mut refs = vec![submission.submission_ref.clone(), submission.operation_id.clone()];
    push_bounded(&mut refs, submission.executable.content_ref.clone(), MAX_JOB_REFS, "job ref status refs")?;
    extend_cloned_bounded(&mut refs, output_refs, MAX_JOB_REFS, "job ref status refs")?;
    let mut status_checks = checks.to_vec();
    status_checks.push(("canonical-status", "pass"));
    Ok(crate::preserves_rail::record("job-ref-status-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::JOB_REF_STATUS_SCHEMA),
        crate::preserves_rail::record("submission", vec![crate::preserves_rail::string(&submission.submission_ref)]),
        crate::preserves_rail::record("job-id", vec![crate::preserves_rail::string(&submission.job_id)]),
        crate::preserves_rail::record("operation-id", vec![crate::preserves_rail::string(&submission.operation_id)]),
        crate::preserves_rail::record("state", vec![crate::preserves_rail::string(state)]),
        crate::preserves_rail::record("outputs", vec![refs_sequence(output_refs)]),
        crate::preserves_rail::record("refs", vec![refs_sequence(&sorted_unique(&refs))]),
        checks_value_from_pairs(&status_checks),
    ]))
}

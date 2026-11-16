pub fn build_octet_warning_baseline(input: &OctetWarningBaselineInput) -> Result<OctetWarningBaselineArtifact> {
    let mut checks = Vec::new();
    let mut diagnostics = Vec::new();
    let run = load_current_octet_run(&input.artifacts_dir, &capture_workspace(), &mut checks, &mut diagnostics)?;
    if checks.iter().any(|check| check.status != "pass") {
        return Err(MoltenError::invalid_harness(format!(
            "cannot create octet warning baseline from invalid artifacts: {}",
            diagnostics.join("; ")
        )));
    }
    let target_next = input.target_next.unwrap_or(run.status.total_findings);
    let critical_keys = critical_keys(&run.findings);
    let source_snapshot_ref = source_snapshot_ref(&run)?;
    let baseline_value = octet_warning_baseline_value(&OctetWarningBaselineValueInput {
        run: &run,
        created_at: &input.created_at,
        expires_at: &input.expires_at,
        target_next,
        source_snapshot_ref: &source_snapshot_ref,
        checks: &checks,
    });
    let baseline_ref = canonical_hash(&baseline_value)?;
    Ok(OctetWarningBaselineArtifact {
        baseline_ref,
        baseline_value,
        finding_count: run.status.total_findings,
        critical_count: critical_keys.len() as u64,
    })
}

pub fn import_octet_artifacts_to_ledger(input: &OctetArtifactLedgerInput) -> Result<OctetArtifactLedgerImport> {
    let mut checks = Vec::new();
    let mut diagnostics = Vec::new();
    let files = read_import_files(&input.artifacts_dir, &mut checks, &mut diagnostics);
    let mut values = raw_values(&files);
    add_structured_value(&mut values, &files, &mut checks, &mut diagnostics);
    add_fingerprint_value(&mut values, &files, &mut checks, &mut diagnostics)?;
    ensure_count_at_most(values.len(), MAX_OCTET_IMPORTED_REFS, "octet imported artifacts")?;
    let mut imported_refs = Vec::with_capacity(values.len());
    for value in &values {
        imported_refs.push(crate::ledger::import_artifact(&input.ledger_root, value)?.artifact_ref);
    }
    push_check(&mut checks, "octet-ledger-imports", !imported_refs.is_empty());
    let decision = if checks.iter().all(|check| check.status == "pass") {
        "pass"
    } else {
        "deny"
    };
    let receipt_value = octet_artifact_ledger_receipt_value(
        decision,
        &input.artifacts_dir.to_string_lossy(),
        &imported_refs,
        &diagnostics,
        &checks,
    );
    let receipt_ref = canonical_hash(&receipt_value)?;
    Ok(OctetArtifactLedgerImport {
        decision: decision.to_string(),
        imported_refs,
        receipt_ref,
        receipt_value,
        diagnostics,
    })
}
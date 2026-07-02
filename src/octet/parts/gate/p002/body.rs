
pub fn evaluate_octet_gate(input: &OctetGateInput) -> Result<OctetGateEvaluation> {
    let mut checks = Vec::new();
    let mut diagnostics = Vec::new();

    push_initial_checks(input, &mut checks, &mut diagnostics);
    let files = read_required_inputs(&input.artifacts_dir, &mut checks, &mut diagnostics);
    let status = parse_status(files.status_file.as_ref(), &mut checks, &mut diagnostics);
    let lint_counts = parse_summary_lints(files.summary.as_ref(), &mut checks, &mut diagnostics);
    let object_corpus_receipt = validate_object_corpus(files.object_corpus.as_ref(), &mut checks, &mut diagnostics);
    let has_valid_object_corpus = object_corpus_receipt.is_some();
    let has_valid_command_shape = validate_command(files.command.as_ref(), &mut checks, &mut diagnostics);
    let has_current_metadata_binding =
        validate_metadata_binding(files.command.as_ref(), status.as_ref(), &mut checks, &mut diagnostics);
    let counts = finding_counts(status.as_ref(), &lint_counts);
    let evidence = derive_evidence(EvidenceInput {
        status: status.as_ref(),
        status_file: files.status_file.as_ref(),
        summary: files.summary.as_ref(),
        object_corpus_receipt: object_corpus_receipt.as_ref(),
        object_corpus: files.object_corpus.as_ref(),
    })?;
    let has_artifact_bindings = files.command.is_some()
        && files.status_file.is_some()
        && files.summary.is_some()
        && files.object_corpus.is_some();

    push_outcome_checks(
        OutcomeFacts {
            status: status.as_ref(),
            counts: &counts,
            has_artifact_bindings,
            has_structured_findings_ref: evidence.structured_findings_ref.is_some(),
            structured_unkeyed: evidence.structured_unkeyed,
            has_fingerprint_evidence_ref: evidence.fingerprint_evidence_ref.is_some(),
        },
        &mut checks,
        &mut diagnostics,
    );

    let has_passing_gate_checks = checks.iter().all(|check| check.status == "pass")
        && has_valid_object_corpus
        && has_valid_command_shape
        && has_current_metadata_binding;
    let decision = if has_passing_gate_checks { "pass" } else { "deny" }.to_string();
    let policy_value = octet_gate_policy_value(input);
    let policy_ref = canonical_hash(&policy_value)?;
    let receipt_value = octet_gate_receipt_value(OctetGateReceiptInput {
        decision: &decision,
        policy_ref: &policy_ref,
        command_ref: files.command.as_ref().map(|file| file.artifact_ref.as_str()),
        status_ref: files.status_file.as_ref().map(|file| file.artifact_ref.as_str()),
        summary_ref: files.summary.as_ref().map(|file| file.artifact_ref.as_str()),
        structured_findings_ref: evidence.structured_findings_ref.as_deref(),
        object_corpus_ref: files.object_corpus.as_ref().map(|file| file.artifact_ref.as_str()),
        fingerprint_evidence_ref: evidence.fingerprint_evidence_ref.as_deref(),
        config_hash: status.as_ref().map(|status| status.metadata.config_hash.as_str()),
        profile_hash: status.as_ref().map(|status| status.metadata.profile_hash.as_str()),
        toolchain: status.as_ref().map(|status| status.metadata.toolchain.as_str()),
        counts: &counts,
        diagnostics: &diagnostics,
        checks: &checks,
    });
    let receipt_ref = canonical_hash(&receipt_value)?;
    Ok(OctetGateEvaluation {
        decision,
        receipt_ref,
        receipt_value,
        diagnostics,
    })
}

fn push_initial_checks(
    input: &OctetGateInput,
    checks: &mut impl crate::bounded::VecSink<Check>,
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) {
    let has_artifacts_dir = input.artifacts_dir.is_dir();
    push_check(checks, "artifacts-dir-present", has_artifacts_dir);
    if !has_artifacts_dir {
        push_diagnostic(diagnostics, format!("artifacts directory missing: {}", input.artifacts_dir.display()));
    }

    let is_profile_supported = input.profile == STRICT_PROFILE;
    push_check(checks, "profile-supported", is_profile_supported);
    if !is_profile_supported {
        push_diagnostic(diagnostics, format!("unsupported octet gate profile: {}", input.profile));
    }
}

fn read_required_inputs(
    artifacts_dir: &Path,
    checks: &mut impl crate::bounded::VecSink<Check>,
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) -> InputFiles {
    InputFiles {
        command: read_required_file(artifacts_dir, COMMAND_NAME, "command-artifact-present", checks, diagnostics),
        status_file: read_required_file(artifacts_dir, STATUS_NAME, "status-artifact-present", checks, diagnostics),
        summary: read_required_file(artifacts_dir, SUMMARY_NAME, "summary-artifact-present", checks, diagnostics),
        object_corpus: read_required_file(
            artifacts_dir,
            OBJECT_CORPUS_RECEIPT_NAME,
            "object-corpus-artifact-present",
            checks,
            diagnostics,
        ),
    }
}

fn derive_evidence(input: EvidenceInput<'_>) -> Result<DerivedEvidence> {
    let structured_findings = input
        .status
        .zip(input.status_file)
        .zip(input.summary)
        .map(|((status, status_file), summary)| octet_structured_findings_value(status_file, summary, status));
    let structured_findings_ref =
        structured_findings.as_ref().map(|(value, _unkeyed)| canonical_hash(value)).transpose()?;
    let structured_unkeyed = structured_findings
        .as_ref()
        .map(|(_value, unkeyed)| *unkeyed)
        .unwrap_or_else(|| input.status.map_or(0, |status| status.total_findings));
    let fingerprint_evidence = input
        .object_corpus_receipt
        .zip(input.object_corpus)
        .map(|(receipt, file)| octet_fingerprint_evidence_value(file, receipt))
        .transpose()?;
    let fingerprint_evidence_ref = fingerprint_evidence.as_ref().map(canonical_hash).transpose()?;
    Ok(DerivedEvidence {
        structured_findings_ref,
        structured_unkeyed,
        fingerprint_evidence_ref,
    })
}

fn push_outcome_checks(
    facts: OutcomeFacts<'_>,
    checks: &mut impl crate::bounded::VecSink<Check>,
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) {
    let is_strict_status_clean = facts.status.is_some_and(|status| status.status == "clean");
    push_check(checks, "strict-status-clean", is_strict_status_clean);
    let denied_status = match facts.status {
        Some(status) if status.status == "clean" => None,
        Some(status) => Some(status),
        None => None,
    };
    if let Some(status) = denied_status {
        push_diagnostic(
            diagnostics,
            format!("strict profile denies octet status `{}` with {} findings", status.status, status.total_findings),
        );
    }

    let has_zero_critical_findings = facts.counts.critical == 0;
    push_check(checks, "no-critical-findings", has_zero_critical_findings);
    if facts.counts.critical > 0 {
        push_diagnostic(diagnostics, format!("unreviewed critical octet findings: {}", facts.counts.critical));
    }
    push_check(checks, "artifact-ref-binding", facts.has_artifact_bindings);
    push_check(checks, "structured-findings-bound", facts.has_structured_findings_ref);
    push_check(
        checks,
        "structured-findings-keyed",
        facts.has_structured_findings_ref && facts.structured_unkeyed == 0,
    );
    push_check(checks, "fingerprint-evidence-bound", facts.has_fingerprint_evidence_ref);

    if !facts.has_structured_findings_ref {
        push_diagnostic(diagnostics, "missing structured octet findings artifact".to_string());
    }
    if facts.has_structured_findings_ref && facts.structured_unkeyed > 0 {
        push_diagnostic(
            diagnostics,
            format!("structured octet findings omitted stable keys for {} findings", facts.structured_unkeyed),
        );
    }
    if !facts.has_fingerprint_evidence_ref {
        push_diagnostic(diagnostics, "missing octet fingerprint evidence artifact".to_string());
    }
}

#[doc(hidden)]
pub fn synthetic_clean_octet_gate_receipt_for_tests() -> Result<IoValue> {
    let metadata = expected_metadata_for_command(DEFAULT_GATE_COMMAND)
        .map_err(|message| MoltenError::invalid_harness(format!("current octet metadata fixture: {message}")))?;
    let policy = octet_gate_policy_value(&OctetGateInput {
        artifacts_dir: PathBuf::from("target/octet"),
        profile: STRICT_PROFILE.to_string(),
    });
    let counts = FindingCounts::default();
    Ok(octet_gate_receipt_value(OctetGateReceiptInput {
        decision: "pass",
        policy_ref: &canonical_hash(&policy)?,
        command_ref: Some("blake3:test-command"),
        status_ref: Some("blake3:test-status"),
        summary_ref: Some("blake3:test-summary"),
        structured_findings_ref: Some("blake3:test-findings"),
        object_corpus_ref: Some("blake3:test-object-corpus"),
        fingerprint_evidence_ref: Some("blake3:test-fingerprint"),
        config_hash: Some(&metadata.config_hash),
        profile_hash: Some(&metadata.profile_hash),
        toolchain: Some("nightly-test-toolchain"),
        counts: &counts,
        diagnostics: &[],
        checks: PASS_CHECKS,
    }))
}

pub fn build_octet_warning_baseline(input: &OctetWarningBaselineInput) -> Result<OctetWarningBaselineArtifact> {
    let mut checks = Vec::new();
    let mut diagnostics = Vec::new();
    let run = load_current_octet_run(&input.artifacts_dir, &mut checks, &mut diagnostics)?;
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

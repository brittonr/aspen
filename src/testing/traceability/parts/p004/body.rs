fn summarize_entries(entries: &[TraceabilityEntry]) -> Result<TraceabilitySummary> {
    if entries.len() > MAX_SUMMARY_LINES {
        return Err(MoltenError::invalid_harness(format!(
            "traceability summary entry count {} exceeds bound {MAX_SUMMARY_LINES}",
            entries.len()
        )));
    }
    let mut summary = TraceabilitySummary::default();
    for entry in entries {
        match entry.status.as_str() {
            "covered" => summary.covered.push(entry.requirement_id.clone()),
            "exempt" => summary.exempt.push(entry.requirement_id.clone()),
            "missing-positive" => summary.missing_positive.push(entry.requirement_id.clone()),
            "missing-negative" => summary.missing_negative.push(entry.requirement_id.clone()),
            "stale-reference" => summary.stale_reference.push(entry.requirement_id.clone()),
            "unsupported" => summary.unsupported.push(entry.requirement_id.clone()),
            other => return Err(MoltenError::invalid_harness(format!("unsupported traceability status {other}"))),
        }
        if entry
            .positive
            .iter()
            .chain(entry.negative.iter())
            .any(|evidence| evidence.source == "compatibility")
        {
            summary.compatibility_only.push(entry.requirement_id.clone());
        }
    }
    Ok(summary)
}

fn traceability_decision(summary: &TraceabilitySummary) -> &'static str {
    if summary.missing_positive.is_empty() && summary.missing_negative.is_empty() && summary.stale_reference.is_empty()
    {
        "pass"
    } else {
        "deny"
    }
}

fn manifest_value(
    decision: &str,
    entries: &[TraceabilityEntry],
    summary: &TraceabilitySummary,
    require_receipt_backed: bool,
) -> Result<IoValue> {
    validate_decision(decision)?;
    Ok(record("requirement-traceability-manifest-v1", vec![
        string(TRACEABILITY_MANIFEST_SCHEMA),
        record("decision", vec![string(decision)]),
        record("entries", vec![sequence(entry_values(entries)?)]),
        record("summary", vec![summary_value(summary)?]),
        record("policy", vec![record("receipt-backed-required", vec![
            crate::preserves_rail::bool_value(require_receipt_backed),
        ])]),
        record("checks", vec![sequence(vec![
            check_value("requirements-enumerated", status(!entries.is_empty())),
            check_value("positive-and-negative-required", "pass"),
            check_value("stale-references-fail-closed", "pass"),
            check_value("raw-coverage-claims-labeled", "pass"),
            check_value("receipt-backed-policy-explicit", "pass"),
        ])]),
    ]))
}

fn entry_values(entries: &[TraceabilityEntry]) -> Result<Vec<IoValue>> {
    let mut values = Vec::with_capacity(entries.len());
    for entry in entries {
        values.push(entry_value(entry)?);
    }
    Ok(values)
}

fn entry_value(entry: &TraceabilityEntry) -> Result<IoValue> {
    validate_text("entry requirement", &entry.requirement_id)?;
    validate_kind(&entry.kind)?;
    Ok(record("entry", vec![
        record("requirement", vec![string(&entry.requirement_id)]),
        record("source", vec![string(&entry.source)]),
        record("kind", vec![string(&entry.kind)]),
        record("changed", vec![crate::preserves_rail::bool_value(entry.changed)]),
        record("status", vec![string(&entry.status)]),
        record("positive", vec![sequence(evidence_values(&entry.positive)?)]),
        record("negative", vec![sequence(evidence_values(&entry.negative)?)]),
        record("exemption", vec![exemption_value(entry.exemption.as_ref())]),
        record("diagnostics", vec![sequence(entry.diagnostics.iter().map(string).collect())]),
    ]))
}

fn evidence_values(evidence: &[VerificationEvidence]) -> Result<Vec<IoValue>> {
    let mut values = Vec::with_capacity(evidence.len());
    for item in evidence {
        validate_text("evidence target", &item.target)?;
        values.push(record("evidence", vec![
            record("target", vec![string(&item.target)]),
            record("command", vec![string(&item.command)]),
            record("artifact", vec![string(&item.artifact_ref)]),
            record("artifacts", vec![sequence(string_values(&item.artifact_refs)?)]),
            record("target-exists", vec![crate::preserves_rail::bool_value(item.target_exists)]),
            record("artifact-present", vec![crate::preserves_rail::bool_value(item.artifact_present)]),
            record("source", vec![string(&item.source)]),
            record("receipt", vec![optional_ref_value(item.receipt_ref.as_deref())]),
            record("expected-decision", vec![string(&item.expected_decision)]),
        ]));
    }
    Ok(values)
}

fn exemption_value(exemption: Option<&CoverageExemption>) -> IoValue {
    match exemption {
        Some(value) => record("some", vec![
            record("class", vec![string(&value.class)]),
            record("evidence", vec![string(&value.evidence)]),
        ]),
        None => record("none", Vec::new()),
    }
}

fn summary_value(summary: &TraceabilitySummary) -> Result<IoValue> {
    Ok(record("summary", vec![
        group_value("covered", &summary.covered)?,
        group_value("exempt", &summary.exempt)?,
        group_value("missing-positive", &summary.missing_positive)?,
        group_value("missing-negative", &summary.missing_negative)?,
        group_value("stale-reference", &summary.stale_reference)?,
        group_value("unsupported", &summary.unsupported)?,
        group_value("compatibility-only", &summary.compatibility_only)?,
    ]))
}

fn group_value(label: &'static str, ids: &[String]) -> Result<IoValue> {
    Ok(record(label, vec![sequence(string_values(ids)?)]))
}

fn string_values(values: &[String]) -> Result<Vec<IoValue>> {
    let mut output = Vec::with_capacity(values.len());
    for value in values {
        validate_text("summary item", value)?;
        output.push(string(value));
    }
    Ok(output)
}

fn validate_verification_run_input(input: &VerificationRunInput) -> Result<()> {
    validate_requirement_id(&input.requirement_id)?;
    validate_coverage_kind(&input.coverage_kind)?;
    validate_text("verification target", &input.target)?;
    validate_string_list("verification argv", &input.argv, MAX_RECEIPT_ARGS)?;
    validate_ref(&input.profile_ref, "verification profile")?;
    validate_ref_list("verification toolchain refs", &input.toolchain_refs, MAX_RECEIPT_REFS)?;
    validate_ref(&input.stdout_ref, "verification stdout")?;
    validate_ref(&input.stderr_ref, "verification stderr")?;
    validate_ref_list("verification artifact refs", &input.artifact_refs, MAX_RECEIPT_REFS)?;
    Ok(())
}

fn verification_run_diagnostics(input: &VerificationRunInput) -> Result<Vec<String>> {
    let mut diagnostics = Vec::new();
    let is_success = input.exit_status == 0;
    match input.coverage_kind.as_str() {
        "positive" if !is_success => diagnostics.push("positive-run-exited-nonzero".to_string()),
        "negative" if is_success => diagnostics.push("negative-run-did-not-deny".to_string()),
        "positive" | "negative" => {}
        other => return Err(MoltenError::invalid_harness(format!("unsupported coverage kind {other}"))),
    }
    if input.artifact_refs.is_empty() {
        diagnostics.push("missing-produced-artifact-ref".to_string());
    }
    Ok(diagnostics)
}

fn verification_run_receipt_value(
    input: &VerificationRunInput,
    decision: &str,
    diagnostics: &[String],
) -> Result<IoValue> {
    Ok(record("verification-run-receipt-v1", vec![
        string(VERIFICATION_RUN_RECEIPT_SCHEMA),
        record("decision", vec![string(decision)]),
        record("requirement", vec![string(&input.requirement_id)]),
        record("coverage-kind", vec![string(&input.coverage_kind)]),
        record("target", vec![string(&input.target)]),
        record("argv", vec![sequence(string_values(&input.argv)?)]),
        record("profile", vec![string(&input.profile_ref)]),
        record("toolchains", vec![sequence(string_values(&input.toolchain_refs)?)]),
        record("exit-status", vec![IoValue::new(input.exit_status)]),
        record("stdout", vec![string(&input.stdout_ref)]),
        record("stderr", vec![string(&input.stderr_ref)]),
        record("artifacts", vec![sequence(string_values(&input.artifact_refs)?)]),
        record("diagnostics", vec![sequence(string_values(diagnostics)?)]),
    ]))
}

fn validate_verification_receipt_decision(
    decision: &str,
    coverage_kind: &str,
    exit_status: i64,
    diagnostics: &[String],
) -> Result<()> {
    let expected = if diagnostics.is_empty() {
        match coverage_kind {
            "positive" if exit_status == 0 => "pass",
            "negative" if exit_status != 0 => "deny",
            "positive" | "negative" => "deny",
            other => return Err(MoltenError::invalid_harness(format!("unsupported coverage kind {other}"))),
        }
    } else {
        "deny"
    };
    if decision == expected {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!(
            "verification receipt decision {decision} does not match expected {expected}"
        )))
    }
}

fn evidence_from_verification_receipt(
    receipt: &VerificationRunReceipt,
    target_exists: bool,
) -> Result<VerificationEvidence> {
    let artifact_ref = receipt.artifact_refs.first().cloned().unwrap_or_else(|| receipt.receipt_ref.clone());
    Ok(VerificationEvidence {
        target: receipt.target.clone(),
        command: receipt.argv.join(" "),
        artifact_ref,
        artifact_refs: receipt.artifact_refs.clone(),
        target_exists,
        artifact_present: receipt
            .artifact_refs
            .iter()
            .all(|reference| crate::preserves_rail::validate_content_ref(reference).is_ok()),
        source: "verification-run-receipt".to_string(),
        receipt_ref: Some(receipt.receipt_ref.clone()),
        expected_decision: receipt.decision.clone(),
    })
}

fn aggregate_proof_diagnostics(input: &AggregateProofInput) -> Result<Vec<String>> {
    ensure_count_at_most(input.obligations.len(), MAX_PROOF_OBLIGATIONS, "proof obligations")?;
    ensure_count_at_most(input.required_obligation_ids.len(), MAX_PROOF_OBLIGATIONS, "required proof obligations")?;
    let max_diagnostics = aggregate_proof_diagnostic_bound(input)?;
    let mut diagnostics = Vec::with_capacity(input.obligations.len().saturating_add(1));
    if input.obligations.is_empty() {
        diagnostics.push_limited("missing-obligations".to_string(), max_diagnostics, "aggregate proof diagnostics")?;
    }
    let mut ids = OrderedSet::new();
    for obligation in &input.obligations {
        validate_obligation(obligation)?;
        if !ids.insert(obligation.id.clone()) {
            diagnostics.push_limited(
                format!("duplicate-obligation:{}", obligation.id),
                max_diagnostics,
                "aggregate proof diagnostics",
            )?;
        }
        if obligation.subject_ref != input.subject_ref {
            diagnostics.push_limited(
                format!("wrong-subject:{}", obligation.id),
                max_diagnostics,
                "aggregate proof diagnostics",
            )?;
        }
        if obligation.decision != obligation_expected_decision(&obligation.class)? {
            diagnostics.push_limited(
                format!("wrong-expected-decision:{}", obligation.id),
                max_diagnostics,
                "aggregate proof diagnostics",
            )?;
        }
    }
    let obligation_map = input
        .obligations
        .iter()
        .map(|obligation| (obligation.id.clone(), obligation))
        .collect::<OrderedMap<_, _>>();
    for required in &input.required_obligation_ids {
        validate_text("required obligation id", required)?;
        if !obligation_map.contains_key(required) {
            diagnostics.push_limited(
                format!("missing-child:{required}"),
                max_diagnostics,
                "aggregate proof diagnostics",
            )?;
        }
    }
    Ok(diagnostics)
}


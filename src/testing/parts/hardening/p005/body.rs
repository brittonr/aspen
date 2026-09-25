
fn replay_smoke_value(input: &ReplaySmokeInput, decision: &str, diagnostics: &[String]) -> Result<IoValue> {
    Ok(record("replay-smoke-gate-v1", vec![
        string(REPLAY_SMOKE_SCHEMA),
        field_string("decision", decision),
        field_string("suite-id", &input.suite_id),
        field_string("eligibility", &input.eligibility),
        field_sequence("runs", replay_run_values(&input.runs)?),
        field_sequence("variance", string_values(&input.variance)?),
        field_sequence("diagnostic-caveats", string_values(&input.diagnostic_caveats)?),
        field_sequence("diagnostics", string_values(diagnostics)?),
        field_sequence(
            "caveats",
            string_values(&[EVIDENCE_ONLY_CAVEAT.to_string(), DIAGNOSTIC_VIEW_CAVEAT.to_string()])?,
        ),
    ]))
}

fn nextest_profile_matrix_value(
    input: &NextestProfileMatrixInput,
    decision: &str,
    diagnostics: &[String],
) -> Result<IoValue> {
    Ok(record("nextest-profile-matrix-v1", vec![
        string(NEXTEST_PROFILE_MATRIX_SCHEMA),
        field_string("decision", decision),
        field_sequence("profiles", profile_values(&input.profiles)?),
        field_sequence("diagnostics", string_values(diagnostics)?),
        field_sequence(
            "caveats",
            string_values(&[EVIDENCE_ONLY_CAVEAT.to_string(), DIAGNOSTIC_VIEW_CAVEAT.to_string()])?,
        ),
    ]))
}

fn cli_receipt_first_value(input: &CliReceiptFirstInput, decision: &str, diagnostics: &[String]) -> Result<IoValue> {
    Ok(record("cli-receipt-first-gate-v1", vec![
        string(CLI_RECEIPT_FIRST_SCHEMA),
        field_string("decision", decision),
        field_string("command", &input.command),
        record("evidence-bearing", vec![bool_value(input.evidence_bearing)]),
        field_sequence("canonical-artifacts", string_values(&input.canonical_artifact_refs)?),
        field_sequence("rendered-output-kinds", string_values(&input.rendered_output_kinds)?),
        record("negative-case", vec![bool_value(input.negative_case)]),
        field_string("failure-artifact-ref", input.failure_artifact_ref.as_deref().unwrap_or("none")),
        field_sequence("diagnostics", string_values(diagnostics)?),
        field_sequence(
            "caveats",
            string_values(&[DIAGNOSTIC_VIEW_CAVEAT.to_string(), EVIDENCE_ONLY_CAVEAT.to_string()])?,
        ),
    ]))
}

fn boundary_requirement_values(values: &[BoundaryRequirementInput]) -> Result<Vec<IoValue>> {
    values
        .iter()
        .map(|item| {
            Ok(record("boundary-requirement", vec![
                field_string("class", &item.class),
                field_string("polarity", &item.polarity),
                field_string("requirement", &item.requirement_id),
            ]))
        })
        .collect()
}

fn boundary_observation_values(values: &[BoundaryObservationInput]) -> Result<Vec<IoValue>> {
    values
        .iter()
        .map(|item| {
            Ok(record("boundary-observation", vec![
                field_string("class", &item.class),
                field_string("polarity", &item.polarity),
                field_string("requirement", &item.requirement_id),
                field_string("evidence-ref", &item.evidence_ref),
            ]))
        })
        .collect()
}

fn boundary_exemption_values(values: &[BoundaryCoverageExemptionInput]) -> Result<Vec<IoValue>> {
    values
        .iter()
        .map(|item| {
            Ok(record("boundary-exemption", vec![
                field_string("class", &item.class),
                field_string("reason", &item.reason),
                field_string("evidence-ref", &item.evidence_ref),
                field_string("scope", &item.scope),
                field_string("caveat", &item.caveat),
            ]))
        })
        .collect()
}

fn matrix_entry_values(values: &[EvidenceMatrixEntryInput]) -> Result<Vec<IoValue>> {
    values
        .iter()
        .map(|item| {
            Ok(record("entry", vec![
                field_string("requirement", &item.requirement_id),
                field_string("coverage-kind", &item.coverage_kind),
                field_string("evidence-scope", &item.evidence_scope),
                field_string("target", &item.target),
                field_string("command", &item.command),
                field_sequence("artifact-refs", string_values(&item.artifact_refs)?),
                field_string("receipt-ref", item.receipt_ref.as_deref().unwrap_or("none")),
                field_sequence("caveats", string_values(&item.caveats)?),
            ]))
        })
        .collect()
}

fn matrix_exemption_values(values: &[EvidenceMatrixExemptionInput]) -> Result<Vec<IoValue>> {
    values
        .iter()
        .map(|item| {
            Ok(record("exemption", vec![
                field_string("requirement", &item.requirement_id),
                field_string("reason", &item.reason),
                field_string("evidence-ref", &item.evidence_ref),
                field_string("scope", &item.scope),
                field_string("review-note", &item.review_note),
            ]))
        })
        .collect()
}

fn tamper_family_values(values: &[TamperFamilyInput]) -> Result<Vec<IoValue>> {
    values
        .iter()
        .map(|item| {
            Ok(record("family", vec![
                field_string("family", &item.family),
                field_string("control-ref", &item.control_ref),
                field_string("parser", &item.parser),
                field_string("gate", &item.gate),
            ]))
        })
        .collect()
}

fn tamper_case_values(values: &[TamperCaseInput]) -> Result<Vec<IoValue>> {
    values
        .iter()
        .map(|item| {
            Ok(record("case", vec![
                field_string("family", &item.family),
                field_string("mutation", &item.mutation),
                field_string("fixture-ref", &item.fixture_ref),
                field_string("expected-diagnostic", &item.expected_diagnostic),
                field_string("decision", &item.decision),
                field_string("pass-evidence-ref", item.pass_evidence_ref.as_deref().unwrap_or("none")),
            ]))
        })
        .collect()
}

fn replay_run_values(values: &[ReplaySmokeRunInput]) -> Result<Vec<IoValue>> {
    values
        .iter()
        .map(|item| {
            Ok(record("run", vec![
                field_string("role", &item.role),
                field_string("report-ref", &item.report_ref),
                field_string("final-state-ref", &item.final_state_ref),
                field_string("effect-log-ref", &item.effect_log_ref),
                field_string("trace-ref", &item.trace_ref),
                field_sequence("diagnostics", string_values(&item.diagnostics)?),
            ]))
        })
        .collect()
}

fn profile_values(values: &[SemanticProfileInput]) -> Result<Vec<IoValue>> {
    values
        .iter()
        .map(|item| {
            Ok(record("profile", vec![
                field_string("profile-id", &item.profile_id),
                field_string("evidence-scope", &item.evidence_scope),
                field_string("command-surface", &item.command_surface),
                field_string("filter-expression", &item.filter_expression),
                field_string("retry-policy", &item.retry_policy),
                field_sequence("expected-artifacts", string_values(&item.expected_artifacts)?),
                field_string("expected-junit-path", &item.expected_junit_path),
                field_string("cost-class", &item.cost_class),
                field_sequence("caveats", string_values(&item.caveats)?),
                field_sequence("excluded-partitions", string_values(&item.excluded_partitions)?),
                record("platform-required", vec![bool_value(item.platform_required)]),
                record("platform-available", vec![bool_value(item.platform_available)]),
            ]))
        })
        .collect()
}

fn validate_requirement_ids(ids: &[String]) -> Result<()> {
    ensure_bound(ids.len(), "requirement ids")?;
    for id in ids {
        validate_text("requirement id", id)?;
    }
    Ok(())
}

fn validate_ref_list(label: &str, refs: &[String]) -> Result<()> {
    ensure_ref_bound(refs.len(), label)?;
    for reference in refs {
        validate_ref(reference, label)?;
    }
    Ok(())
}

fn validate_ref_list_with_diagnostics(
    label: &str,
    refs: &[String],
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) -> Result<()> {
    ensure_ref_bound(refs.len(), label)?;
    for reference in refs {
        validate_ref_with_diagnostics(reference, label, diagnostics);
    }
    Ok(())
}

fn validate_ref_with_diagnostics(reference: &str, label: &str, diagnostics: &mut impl crate::bounded::VecSink<String>) {
    if let Err(error) = validate_ref(reference, label) {
        diagnostics.push_item(format!("stale-ref:{reference}:{error}"));
    }
}

fn validate_ref(reference: &str, label: &str) -> Result<()> {
    crate::preserves_rail::validate_content_ref(reference)
        .map_err(|error| MoltenError::invalid_harness(format!("invalid {label} ref {reference}: {error}")))
}

fn validate_text(label: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        Err(MoltenError::invalid_harness(format!("{label} must not be empty")))
    } else {
        Ok(())
    }
}

fn validate_decision(decision: &str) -> Result<()> {
    match decision {
        DECISION_PASS | DECISION_DENY => Ok(()),
        other => Err(MoltenError::invalid_harness(format!("unsupported decision {other}"))),
    }
}

fn ensure_bound(count: usize, label: &str) -> Result<()> {
    crate::bounded::ensure_count_at_most(count, MAX_ITEMS, label)
}

fn ensure_ref_bound(count: usize, label: &str) -> Result<()> {
    crate::bounded::ensure_count_at_most(count, MAX_REFS, label)
}

fn decision_for(diagnostics: &[String]) -> &'static str {
    if diagnostics.is_empty() {
        DECISION_PASS
    } else {
        DECISION_DENY
    }
}

fn placeholder_ref() -> Result<String> {
    Ok(crate::preserves_rail::content_ref_from_bytes(b"missing"))
}

fn hash(value: &IoValue) -> Result<String> {
    crate::preserves_rail::canonical_hash(value)
}

fn record(label: &'static str, fields: Vec<IoValue>) -> IoValue {
    crate::preserves_rail::record(label, fields)
}

fn field_string(label: &'static str, value: &str) -> IoValue {
    record(label, vec![string(value)])
}

fn field_u64(label: &'static str, value: u64) -> IoValue {
    record(label, vec![IoValue::new(value)])
}

fn field_sequence(label: &'static str, values: Vec<IoValue>) -> IoValue {
    record(label, vec![sequence(values)])
}

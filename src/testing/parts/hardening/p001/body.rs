
// r[impl molten.testing.boundary_coverage.gate]
// r[impl molten.testing.boundary_coverage.positive_negative]
// r[impl molten.testing.boundary_coverage.exemptions]
// r[impl molten.testing.evidence_matrix.checked_in_manifest]
// r[impl molten.testing.evidence_matrix.changed_requirement_gate]
// r[impl molten.testing.evidence_matrix.receipt_backed_entries]
// r[impl molten.testing.evidence_matrix.exemptions]
// r[impl molten.testing.ci_run_receipt.canonical_receipt]
// r[impl molten.testing.ci_run_receipt.junit_view_only]
// r[impl molten.testing.ci_run_receipt.nix_nextest_binding]
// r[impl molten.testing.ci_run_receipt.deny_on_missing_metadata]
// r[impl molten.testing.tamper_matrix.generated_cases]
// r[impl molten.testing.tamper_matrix.coverage]
// r[impl molten.testing.tamper_matrix.fail_closed]
// r[impl molten.testing.hegel_counterexample.replay_fixture]
// r[impl molten.testing.hegel_counterexample.promotion]
// r[impl molten.testing.hegel_counterexample.redaction]
// r[impl molten.testing.replay_smoke.all_evidence_suites]
// r[impl molten.testing.replay_smoke.fresh_rerun]
// r[impl molten.testing.replay_smoke.non_replayable_excluded]
// r[impl molten.testing.nextest_profiles.semantic_partitions]
// r[impl molten.testing.nextest_profiles.config_readback]
// r[impl molten.testing.nextest_profiles.deterministic_exclusion]
// r[impl molten.testing.nextest_profiles.positive_negative_coverage]
// r[impl molten.testing.cli_receipt_first.normative_artifacts]
// r[impl molten.testing.cli_receipt_first.stdout_diagnostic_only]
// r[impl molten.testing.cli_receipt_first.negative_fail_closed]
pub fn build_boundary_coverage_gate(input: &BoundaryCoverageGateInput) -> Result<BoundaryCoverageGate> {
    validate_ref(&input.report_ref, "boundary report")?;
    validate_ref(&input.suite_ref, "boundary suite")?;
    ensure_bound(input.required.len(), "boundary requirements")?;
    ensure_bound(input.observed.len(), "boundary observations")?;
    ensure_bound(input.exemptions.len(), "boundary exemptions")?;
    let mut diagnostics = Vec::new();
    let mut observed = OrderedSet::new();
    for item in &input.observed {
        validate_boundary_observation(item, &mut diagnostics)?;
        observed.insert(boundary_key(&item.class, &item.polarity));
    }
    let mut exemptions = OrderedSet::new();
    for item in &input.exemptions {
        validate_boundary_exemption(item, &mut diagnostics)?;
        exemptions.insert(item.class.clone());
    }
    let mut missing = Vec::with_capacity(input.required.len());
    diagnostics.reserve(input.required.len());
    for requirement in &input.required {
        validate_boundary_requirement(requirement)?;
        let key = boundary_key(&requirement.class, &requirement.polarity);
        if observed.contains(&key) || exemptions.contains(&requirement.class) {
            continue;
        }
        missing.push(key.clone());
        diagnostics.push(format!("missing-boundary:{key}:{}", requirement.requirement_id));
    }
    diagnostics.sort();
    diagnostics.dedup();
    let observed_classes = observed.into_iter().collect::<Vec<_>>();
    let decision = decision_for(&diagnostics);
    let value = boundary_gate_value(input, decision, &observed_classes, &missing, &diagnostics)?;
    let gate_ref = hash(&value)?;
    Ok(BoundaryCoverageGate {
        decision: decision.to_string(),
        observed_classes,
        missing_classes: missing,
        diagnostics,
        gate_ref,
        value,
    })
}

pub fn build_evidence_matrix_manifest(input: &EvidenceMatrixInput) -> Result<EvidenceMatrixManifest> {
    ensure_bound(input.requirements.len(), "matrix requirements")?;
    ensure_bound(input.entries.len(), "matrix entries")?;
    ensure_bound(input.exemptions.len(), "matrix exemptions")?;
    let requirement_map = requirement_map(&input.requirements)?;
    let mut diagnostics = Vec::new();
    let mut duplicate_keys = OrderedSet::new();
    let mut positive = OrderedSet::new();
    let mut negative = OrderedSet::new();
    diagnostics.reserve(input.entries.len());
    for entry in &input.entries {
        validate_matrix_entry(entry, &requirement_map, &mut diagnostics)?;
        let key = format!("{}|{}|{}|{}", entry.requirement_id, entry.coverage_kind, entry.evidence_scope, entry.target);
        if !duplicate_keys.insert(key.clone()) {
            diagnostics.push(format!("duplicate-entry:{key}"));
        }
        match entry.coverage_kind.as_str() {
            "positive" => {
                positive.insert(entry.requirement_id.clone());
            }
            "negative" => {
                negative.insert(entry.requirement_id.clone());
            }
            _ => {}
        }
    }
    let mut exempt = OrderedSet::new();
    for exemption in &input.exemptions {
        validate_matrix_exemption(exemption, &requirement_map, &mut diagnostics)?;
        exempt.insert(exemption.requirement_id.clone());
    }
    let mut missing_positive = Vec::with_capacity(requirement_map.len());
    let mut missing_negative = Vec::with_capacity(requirement_map.len());
    diagnostics.reserve(requirement_map.len().saturating_mul(MISSING_COVERAGE_DIAGNOSTICS_PER_REQUIREMENT));
    for requirement in requirement_map.values() {
        if !requires_matrix_coverage(requirement) || exempt.contains(&requirement.id) {
            continue;
        }
        if !positive.contains(&requirement.id) {
            missing_positive.push(requirement.id.clone());
            diagnostics.push(format!("missing-positive:{}", requirement.id));
        }
        if !negative.contains(&requirement.id) {
            missing_negative.push(requirement.id.clone());
            diagnostics.push(format!("missing-negative:{}", requirement.id));
        }
    }
    diagnostics.sort();
    diagnostics.dedup();
    let decision = decision_for(&diagnostics);
    let value = evidence_matrix_value(input, decision, &missing_positive, &missing_negative, &diagnostics)?;
    let manifest_ref = hash(&value)?;
    Ok(EvidenceMatrixManifest {
        decision: decision.to_string(),
        diagnostics,
        missing_positive,
        missing_negative,
        manifest_ref,
        value,
    })
}

pub fn build_ci_test_run_receipt(input: &CiTestRunInput) -> Result<CiTestRunReceipt> {
    validate_ci_input(input)?;
    let mut diagnostics = input.diagnostics.clone();
    ci_diagnostics(input, &mut diagnostics)?;
    diagnostics.sort();
    diagnostics.dedup();
    let decision = if diagnostics.is_empty() {
        input.decision.as_str()
    } else {
        DECISION_DENY
    };
    let value = ci_test_run_value(input, decision, &diagnostics)?;
    let receipt_ref = hash(&value)?;
    Ok(CiTestRunReceipt {
        decision: decision.to_string(),
        diagnostics,
        receipt_ref,
        value,
    })
}

pub fn build_tamper_matrix(input: &TamperMatrixInput) -> Result<TamperMatrix> {
    validate_ref(&input.subject_ref, "tamper subject")?;
    ensure_bound(input.families.len(), "tamper families")?;
    ensure_bound(input.cases.len(), "tamper cases")?;
    let mut diagnostics = Vec::new();
    let families = family_map(&input.families, &mut diagnostics)?;
    let mut seen = OrderedSet::new();
    diagnostics.reserve(input.cases.len());
    for case in &input.cases {
        validate_tamper_case(case, &families, &mut diagnostics)?;
        let key = format!("{}|{}", case.family, case.mutation);
        if !seen.insert(key.clone()) {
            diagnostics.push(format!("duplicate-tamper-case:{key}"));
        }
    }
    diagnostics.extend(
        families
            .keys()
            .flat_map(|family| required_tamper_mutations().iter().map(move |mutation| format!("{family}|{mutation}")))
            .filter(|key| !seen.contains(key))
            .map(|key| format!("missing-tamper-case:{key}")),
    );
    diagnostics.sort();
    diagnostics.dedup();
    let decision = decision_for(&diagnostics);
    let value = tamper_matrix_value(input, decision, &diagnostics)?;
    let matrix_ref = hash(&value)?;
    Ok(TamperMatrix {
        decision: decision.to_string(),
        diagnostics,
        generated_cases: input.cases.clone(),
        matrix_ref,
        value,
    })
}

pub fn build_hegel_counterexample_fixture(input: &HegelCounterexampleInput) -> Result<HegelCounterexampleFixture> {
    let mut validation_diagnostics = Vec::new();
    validate_text("hegel property id", &input.property_id)?;
    validate_requirement_ids(&input.requirement_ids)?;
    validate_ref(&input.generator_profile_ref, "hegel generator profile")?;
    validate_ref(&input.shrunk_input_ref, "hegel shrunk input")?;
    validate_ref(&input.replay_identity_ref, "hegel replay identity")?;
    validate_ref_list("hegel trace refs", &input.trace_refs)?;
    validate_ref_list("hegel receipt refs", &input.receipt_refs)?;
    validate_text("hegel confidentiality", &input.confidentiality)?;
    if input.generation_seed.trim().is_empty() {
        validation_diagnostics.push("missing-seed".to_string());
    }
    if input.shrink_path.is_empty() {
        validation_diagnostics.push("missing-shrink-path".to_string());
    }
    if input.diagnostics.is_empty() {
        validation_diagnostics.push("missing-diagnostics".to_string());
    }
    if input.confidentiality == "sensitive" {
        validation_diagnostics.push("sensitive-input-not-redacted".to_string());
    }
    validation_diagnostics.sort();
    validation_diagnostics.dedup();
    let decision = decision_for(&validation_diagnostics);
    let mut output_diagnostics = validation_diagnostics;
    output_diagnostics.extend(input.diagnostics.clone());
    output_diagnostics.sort();
    output_diagnostics.dedup();
    let value = hegel_fixture_value(input, decision, &output_diagnostics)?;
    let fixture_ref = hash(&value)?;
    Ok(HegelCounterexampleFixture {
        decision: decision.to_string(),
        diagnostics: output_diagnostics,
        fixture_ref,
        value,
    })
}

pub fn build_hegel_promotion_record(input: &HegelPromotionInput) -> Result<CiTestRunReceipt> {
    validate_ref(&input.source_fixture_ref, "hegel source fixture")?;
    validate_ref(&input.new_suite_entry_ref, "hegel new suite entry")?;
    validate_ref(&input.review_ref, "hegel review")?;
    validate_text("hegel property id", &input.property_id)?;
    validate_text("hegel reason", &input.reason)?;
    let mut diagnostics = Vec::new();
    if !matches!(input.status.as_str(), "regression-pass" | "known-deny") {
        diagnostics.push(format!("unsupported-promotion-status:{}", input.status));
    }
    let decision = decision_for(&diagnostics);
    let value = hegel_promotion_value(input, decision, &diagnostics)?;
    let receipt_ref = hash(&value)?;
    Ok(CiTestRunReceipt {
        decision: decision.to_string(),
        diagnostics,
        receipt_ref,
        value,
    })
}

pub fn build_replay_smoke_gate(input: &ReplaySmokeInput) -> Result<ReplaySmokeGate> {
    validate_text("replay smoke suite id", &input.suite_id)?;
    validate_replay_eligibility(&input.eligibility)?;
    ensure_bound(input.runs.len(), "replay smoke runs")?;
    let mut diagnostics = Vec::new();
    for variance in &input.variance {
        validate_variance(variance)?;
    }
    for caveat in &input.diagnostic_caveats {
        validate_text("replay smoke caveat", caveat)?;
    }
    if input.eligibility == "deterministic" {
        replay_deterministic_diagnostics(input, &mut diagnostics)?;
    } else if input.diagnostic_caveats.is_empty() {
        diagnostics.push("non-replayable-without-diagnostic".to_string());
    }
    diagnostics.sort();
    diagnostics.dedup();
    let decision = decision_for(&diagnostics);
    let value = replay_smoke_value(input, decision, &diagnostics)?;
    let gate_ref = hash(&value)?;
    Ok(ReplaySmokeGate {
        decision: decision.to_string(),
        diagnostics,
        gate_ref,
        value,
    })
}

pub fn reviewed_nextest_profile_rows() -> Vec<SemanticProfileInput> {
    let mut rows = subsystem_profile_rows();
    rows.extend(platform_and_aggregate_profile_rows());
    rows
}

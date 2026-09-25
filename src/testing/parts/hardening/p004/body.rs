
fn artifact_present(artifacts: &[String], expected: &str) -> bool {
    artifacts.iter().any(|artifact| artifact == expected)
}

fn deterministic_profile_requires_exclusions(profile_id: &str) -> bool {
    matches!(profile_id, "fast-core" | "harness" | "cli" | "distributed-simulation" | "deterministic")
}

fn allowed_excluded_partitions() -> &'static [&'static str] {
    &["live-only", "vm-only", "exploratory", "retry-only", "diagnostic-only"]
}

fn validate_semantic_profile_id(profile_id: &str) -> Result<()> {
    match profile_id {
        "fast-core"
        | "harness"
        | "cli"
        | "distributed-simulation"
        | "vm-platform"
        | "dogfood-soak"
        | "ci"
        | "deterministic"
        | "exploratory" => Ok(()),
        other => Err(MoltenError::invalid_harness(format!("unsupported semantic profile {other}"))),
    }
}

fn validate_retry_policy(policy: &str, diagnostics: &mut impl crate::bounded::VecSink<String>) -> Result<()> {
    match policy {
        "zero-retry" | "retry-diagnostic" | "retry-pass" => Ok(()),
        other => {
            diagnostics.push_item(format!("unsupported-retry-policy:{other}"));
            Ok(())
        }
    }
}

fn validate_cost_class(class: &str, diagnostics: &mut impl crate::bounded::VecSink<String>) -> Result<()> {
    match class {
        "fast" | "moderate" | "expensive" | "platform" | "soak" => Ok(()),
        other => {
            diagnostics.push_item(format!("unsupported-cost-class:{other}"));
            Ok(())
        }
    }
}

fn validate_rendered_output_kind(kind: &str) -> Result<()> {
    match kind {
        "stdout" | "stderr" | "markdown" | "json" | "junit" | "terminal-summary" => Ok(()),
        other => Err(MoltenError::invalid_harness(format!("unsupported rendered output kind {other}"))),
    }
}

fn requirement_map(
    requirements: &[crate::requirement_traceability::RequirementInput],
) -> Result<OrderedMap<String, crate::requirement_traceability::RequirementInput>> {
    let mut output = OrderedMap::new();
    for requirement in requirements {
        validate_text("requirement id", &requirement.id)?;
        if output.insert(requirement.id.clone(), requirement.clone()).is_some() {
            return Err(MoltenError::invalid_harness(format!("duplicate requirement {}", requirement.id)));
        }
    }
    Ok(output)
}

fn allowed_boundary_classes() -> &'static [&'static str] {
    &[
        "envelope-send",
        "envelope-receive",
        "dataspace-assert",
        "dataspace-retract",
        "dataspace-observe",
        "policy-pass",
        "policy-denial",
        "capability-pass",
        "capability-denial",
        "effect-request",
        "effect-response",
        "hostcall-request",
        "hostcall-denial",
        "resource-pass",
        "resource-exhaustion",
        "replay-pass",
        "replay-divergence",
        "redaction-pass",
        "redaction-denial",
        "adapter-pass",
        "adapter-denial",
        "adapter-failure",
        "pass-evidence-gate",
        "diagnostic-only-rejection",
    ]
}

fn required_tamper_mutations() -> &'static [&'static str] {
    &[
        "missing-required-field",
        "stale-content-ref",
        "wrong-artifact-kind",
        "malformed-content-ref",
        "duplicate-member",
        "tampered-embedded-receipt",
        "noncanonical-value",
        "diagnostic-only-as-pass",
        "missing-child-receipt",
        "unsupported-schema-version",
    ]
}

fn required_semantic_profiles() -> &'static [&'static str] {
    &[
        "fast-core",
        "harness",
        "cli",
        "distributed-simulation",
        "vm-platform",
        "dogfood-soak",
    ]
}

fn required_deterministic_exclusions() -> &'static [&'static str] {
    &["live-only", "vm-only", "exploratory", "retry-only", "diagnostic-only"]
}

struct ProfileRowInput<'a> {
    profile_id: &'a str,
    evidence_scope: &'a str,
    filter_expression: &'a str,
    retry_policy: &'a str,
    expected_junit_path: &'a str,
    cost_class: &'a str,
    caveats: &'a [&'a str],
    excluded_partitions: &'a [&'a str],
    platform_required: bool,
    platform_available: bool,
}

fn semantic_profile_row(input: ProfileRowInput<'_>) -> SemanticProfileInput {
    SemanticProfileInput {
        profile_id: input.profile_id.to_string(),
        evidence_scope: input.evidence_scope.to_string(),
        command_surface: format!("{NEXTEST_COMMAND_PREFIX}{}", input.profile_id),
        filter_expression: input.filter_expression.to_string(),
        retry_policy: input.retry_policy.to_string(),
        expected_artifacts: default_profile_artifacts(),
        expected_junit_path: input.expected_junit_path.to_string(),
        cost_class: input.cost_class.to_string(),
        caveats: input.caveats.iter().map(|caveat| (*caveat).to_string()).collect(),
        excluded_partitions: input.excluded_partitions.iter().map(|partition| (*partition).to_string()).collect(),
        platform_required: input.platform_required,
        platform_available: input.platform_available,
    }
}

fn default_profile_artifacts() -> Vec<String> {
    [
        PROFILE_METADATA_ARTIFACT,
        FILTER_READBACK_ARTIFACT,
        JUNIT_ARTIFACT,
        CANONICAL_TEST_RUN_ARTIFACT,
    ]
    .iter()
    .map(|artifact| (*artifact).to_string())
    .collect()
}

fn boundary_gate_value(
    input: &BoundaryCoverageGateInput,
    decision: &str,
    observed: &[String],
    missing: &[String],
    diagnostics: &[String],
) -> Result<IoValue> {
    Ok(record("boundary-coverage-gate-v1", vec![
        string(BOUNDARY_COVERAGE_GATE_SCHEMA),
        field_string("decision", decision),
        field_string("report-ref", &input.report_ref),
        field_string("suite-ref", &input.suite_ref),
        field_sequence("required", boundary_requirement_values(&input.required)?),
        field_sequence("observed", boundary_observation_values(&input.observed)?),
        field_sequence("observed-classes", string_values(observed)?),
        field_sequence("missing", string_values(missing)?),
        field_sequence("exemptions", boundary_exemption_values(&input.exemptions)?),
        field_sequence("diagnostics", string_values(diagnostics)?),
        field_sequence(
            "caveats",
            string_values(&[EVIDENCE_ONLY_CAVEAT.to_string(), DIAGNOSTIC_VIEW_CAVEAT.to_string()])?,
        ),
    ]))
}

fn evidence_matrix_value(
    input: &EvidenceMatrixInput,
    decision: &str,
    missing_positive: &[String],
    missing_negative: &[String],
    diagnostics: &[String],
) -> Result<IoValue> {
    Ok(record("evidence-matrix-v1", vec![
        string(EVIDENCE_MATRIX_SCHEMA),
        field_string("decision", decision),
        field_sequence("entries", matrix_entry_values(&input.entries)?),
        field_sequence("exemptions", matrix_exemption_values(&input.exemptions)?),
        field_sequence("missing-positive", string_values(missing_positive)?),
        field_sequence("missing-negative", string_values(missing_negative)?),
        field_sequence("diagnostics", string_values(diagnostics)?),
        field_sequence(
            "caveats",
            string_values(&[EVIDENCE_ONLY_CAVEAT.to_string(), DIAGNOSTIC_VIEW_CAVEAT.to_string()])?,
        ),
    ]))
}

fn ci_test_run_value(input: &CiTestRunInput, decision: &str, diagnostics: &[String]) -> Result<IoValue> {
    Ok(record("ci-test-run-receipt-v1", vec![
        string(CI_TEST_RUN_RECEIPT_SCHEMA),
        field_string("decision", decision),
        field_string("source-ref", &input.source_ref),
        field_string("profile-id", &input.profile_id),
        field_string("command-surface", &input.command_surface),
        field_string("nextest-config-ref", &input.nextest_config_ref),
        field_string("cargo-metadata-ref", &input.cargo_metadata_ref),
        field_string("binaries-metadata-ref", &input.binaries_metadata_ref),
        field_string("junit-ref", &input.junit_ref),
        record("counts", vec![
            field_u64("total", input.counts.total),
            field_u64("passed", input.counts.passed),
            field_u64("failed", input.counts.failed),
            field_u64("skipped", input.counts.skipped),
        ]),
        field_sequence("diagnostics", string_values(diagnostics)?),
        field_sequence("caveats", string_values(&input.caveats)?),
    ]))
}

fn tamper_matrix_value(input: &TamperMatrixInput, decision: &str, diagnostics: &[String]) -> Result<IoValue> {
    Ok(record("tamper-negative-matrix-v1", vec![
        string(TAMPER_MATRIX_SCHEMA),
        field_string("decision", decision),
        field_string("subject", &input.subject_ref),
        field_sequence("families", tamper_family_values(&input.families)?),
        field_sequence("cases", tamper_case_values(&input.cases)?),
        field_sequence("diagnostics", string_values(diagnostics)?),
        field_sequence("caveats", string_values(&[EVIDENCE_ONLY_CAVEAT.to_string()])?),
    ]))
}

fn hegel_fixture_value(input: &HegelCounterexampleInput, decision: &str, diagnostics: &[String]) -> Result<IoValue> {
    Ok(record("hegel-counterexample-fixture-v1", vec![
        string(HEGEL_COUNTEREXAMPLE_SCHEMA),
        field_string("decision", decision),
        field_string("property-id", &input.property_id),
        field_sequence("requirements", string_values(&input.requirement_ids)?),
        field_string("generator-profile-ref", &input.generator_profile_ref),
        field_string("generation-seed", &input.generation_seed),
        field_sequence("shrink-path", string_values(&input.shrink_path)?),
        field_string("shrunk-input-ref", &input.shrunk_input_ref),
        field_string("replay-identity-ref", &input.replay_identity_ref),
        field_sequence("trace-refs", string_values(&input.trace_refs)?),
        field_sequence("receipt-refs", string_values(&input.receipt_refs)?),
        field_string("confidentiality", &input.confidentiality),
        field_sequence("diagnostics", string_values(diagnostics)?),
        field_sequence("caveats", string_values(&[EVIDENCE_ONLY_CAVEAT.to_string()])?),
    ]))
}

fn hegel_promotion_value(input: &HegelPromotionInput, decision: &str, diagnostics: &[String]) -> Result<IoValue> {
    Ok(record("hegel-counterexample-promotion-v1", vec![
        string(HEGEL_PROMOTION_SCHEMA),
        field_string("decision", decision),
        field_string("source-fixture-ref", &input.source_fixture_ref),
        field_string("new-suite-entry-ref", &input.new_suite_entry_ref),
        field_string("review-ref", &input.review_ref),
        field_string("property-id", &input.property_id),
        field_string("reason", &input.reason),
        field_string("status", &input.status),
        field_sequence("diagnostics", string_values(diagnostics)?),
    ]))
}

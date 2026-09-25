
fn validate_matrix_coverage_kind(kind: &str, diagnostics: &mut impl crate::bounded::VecSink<String>) -> Result<()> {
    match kind {
        "positive" | "negative" => Ok(()),
        other => {
            diagnostics.push_item(format!("unsupported-coverage-kind:{other}"));
            Ok(())
        }
    }
}

fn validate_evidence_scope(scope: &str, diagnostics: &mut impl crate::bounded::VecSink<String>) -> Result<()> {
    match scope {
        "unit" | "property" | "cli" | "integration" | "vm" | "dogfood" | "exemption" => Ok(()),
        other => {
            diagnostics.push_item(format!("unsupported-evidence-scope:{other}"));
            Ok(())
        }
    }
}

fn requires_matrix_coverage(requirement: &crate::requirement_traceability::RequirementInput) -> bool {
    requirement.changed || requirement.kind == "evidence"
}

fn validate_ci_input(input: &CiTestRunInput) -> Result<()> {
    validate_ref(&input.source_ref, "ci source")?;
    validate_semantic_profile_id(&input.profile_id)?;
    validate_text("ci command surface", &input.command_surface)?;
    validate_ref(&input.nextest_config_ref, "ci nextest config")?;
    validate_ref(&input.cargo_metadata_ref, "ci cargo metadata")?;
    validate_ref(&input.binaries_metadata_ref, "ci binaries metadata")?;
    validate_ref(&input.junit_ref, "ci junit")?;
    validate_decision(&input.decision)?;
    for diagnostic in &input.diagnostics {
        validate_text("ci diagnostic", diagnostic)?;
    }
    for caveat in &input.caveats {
        validate_text("ci caveat", caveat)?;
    }
    Ok(())
}

fn ci_diagnostics(input: &CiTestRunInput, diagnostics: &mut impl crate::bounded::VecSink<String>) -> Result<()> {
    let observed = input
        .counts
        .passed
        .checked_add(input.counts.failed)
        .and_then(|count| count.checked_add(input.counts.skipped))
        .ok_or_else(|| MoltenError::invalid_harness("ci counts overflow"))?;
    if observed > input.counts.total {
        diagnostics.push_item("mismatched-counts".to_string());
    }
    if input.decision == DECISION_PASS && input.counts.total < MINIMUM_CI_TOTAL_FOR_PASS {
        diagnostics.push_item("missing-test-counts".to_string());
    }
    if input.decision == DECISION_PASS && input.counts.failed > ZERO_COUNT {
        diagnostics.push_item("failed-tests-with-pass-decision".to_string());
    }
    if input.profile_id == "exploratory" && input.decision == DECISION_PASS {
        diagnostics.push_item("exploratory-pass-is-diagnostic-only".to_string());
    }
    Ok(())
}

fn family_map(
    families: &[TamperFamilyInput],
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) -> Result<OrderedMap<String, TamperFamilyInput>> {
    let mut output = OrderedMap::new();
    for family in families {
        validate_text("tamper family", &family.family)?;
        validate_ref(&family.control_ref, "tamper control")?;
        validate_text("tamper parser", &family.parser)?;
        validate_text("tamper gate", &family.gate)?;
        if output.insert(family.family.clone(), family.clone()).is_some() {
            diagnostics.push_item(format!("duplicate-family:{}", family.family));
        }
    }
    Ok(output)
}

fn validate_tamper_case(
    case: &TamperCaseInput,
    families: &OrderedMap<String, TamperFamilyInput>,
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) -> Result<()> {
    validate_text("tamper case family", &case.family)?;
    if !families.contains_key(&case.family) {
        diagnostics.push_item(format!("unknown-family:{}", case.family));
    }
    validate_tamper_mutation(&case.mutation, diagnostics);
    validate_ref(&case.fixture_ref, "tamper fixture")?;
    validate_text("tamper expected diagnostic", &case.expected_diagnostic)?;
    validate_decision(&case.decision)?;
    if case.decision != DECISION_DENY {
        diagnostics.push_item(format!("tamper-case-not-deny:{}:{}", case.family, case.mutation));
    }
    if case.pass_evidence_ref.is_some() {
        diagnostics.push_item(format!("tamper-case-emits-pass-evidence:{}:{}", case.family, case.mutation));
    }
    Ok(())
}

fn validate_tamper_mutation(mutation: &str, diagnostics: &mut impl crate::bounded::VecSink<String>) {
    if !required_tamper_mutations().contains(&mutation) {
        diagnostics.push_item(format!("unsupported-mutation:{mutation}"));
    }
}

fn replay_deterministic_diagnostics(
    input: &ReplaySmokeInput,
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) -> Result<()> {
    let mut roles = OrderedMap::new();
    for run in &input.runs {
        validate_replay_run(run)?;
        if roles.insert(run.role.clone(), run).is_some() {
            diagnostics.push_item(format!("duplicate-replay-role:{}", run.role));
        }
    }
    for role in ["fresh", "replay", "fresh-rerun"] {
        if !roles.contains_key(role) {
            diagnostics.push_item(format!("missing-replay-role:{role}"));
        }
    }
    let Some(fresh) = roles.get("fresh") else {
        return Ok(());
    };
    for role in ["replay", "fresh-rerun"] {
        if let Some(run) = roles.get(role) {
            if run.report_ref != fresh.report_ref {
                diagnostics.push_item(format!("report-ref-mismatch:{role}"));
            }
            if run.final_state_ref != fresh.final_state_ref {
                diagnostics.push_item(format!("final-state-ref-mismatch:{role}"));
            }
            if run.effect_log_ref != fresh.effect_log_ref {
                diagnostics.push_item(format!("effect-log-ref-mismatch:{role}"));
            }
            if run.trace_ref != fresh.trace_ref && !input.variance.iter().any(|item| item == "trace-ref") {
                diagnostics.push_item(format!("trace-ref-mismatch:{role}"));
            }
            for diagnostic in &run.diagnostics {
                diagnostics.push_item(format!("run-diagnostic:{role}:{diagnostic}"));
            }
        }
    }
    if fresh.effect_log_ref == placeholder_ref()? {
        diagnostics.push_item("missing-effect-log".to_string());
    }
    Ok(())
}

fn validate_replay_run(run: &ReplaySmokeRunInput) -> Result<()> {
    match run.role.as_str() {
        "fresh" | "replay" | "fresh-rerun" => {}
        other => return Err(MoltenError::invalid_harness(format!("unsupported replay smoke role {other}"))),
    }
    validate_ref(&run.report_ref, "replay report")?;
    validate_ref(&run.final_state_ref, "replay final state")?;
    validate_ref(&run.effect_log_ref, "replay effect log")?;
    validate_ref(&run.trace_ref, "replay trace")?;
    for diagnostic in &run.diagnostics {
        validate_text("replay run diagnostic", diagnostic)?;
    }
    Ok(())
}

fn validate_replay_eligibility(value: &str) -> Result<()> {
    match value {
        "deterministic" | "exploratory" | "live-only" | "vm-unavailable" | "diagnostic-only" => Ok(()),
        other => Err(MoltenError::invalid_harness(format!("unsupported replay eligibility {other}"))),
    }
}

fn validate_variance(value: &str) -> Result<()> {
    match value {
        "temporary-root" | "runtime-path" | "store-path" | "diagnostic-log" | "rendered-output" | "trace-ref" => Ok(()),
        other => Err(MoltenError::invalid_harness(format!("unsupported replay variance {other}"))),
    }
}

fn validate_profile(
    profile: &SemanticProfileInput,
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) -> Result<()> {
    validate_semantic_profile_id(&profile.profile_id)?;
    validate_evidence_scope(&profile.evidence_scope, diagnostics)?;
    validate_text("profile command surface", &profile.command_surface)?;
    validate_profile_filter(profile, diagnostics);
    validate_retry_policy(&profile.retry_policy, diagnostics)?;
    validate_expected_profile_artifacts(profile, diagnostics)?;
    validate_profile_junit_path(profile, diagnostics);
    validate_cost_class(&profile.cost_class, diagnostics)?;
    for caveat in &profile.caveats {
        validate_text("profile caveat", caveat)?;
    }
    validate_excluded_partitions(profile, diagnostics)?;
    validate_deterministic_profile_exclusions(profile, diagnostics);
    if profile.platform_required && !profile.platform_available {
        diagnostics.push_item(format!("required-platform-unavailable:{}", profile.profile_id));
    }
    if profile.profile_id == "exploratory" && profile.retry_policy == "retry-pass" {
        return Ok(());
    }
    if profile.retry_policy == "retry-pass" {
        diagnostics.push_item(format!("retry-pass-not-deterministic:{}", profile.profile_id));
    }
    Ok(())
}

fn validate_profile_filter(profile: &SemanticProfileInput, diagnostics: &mut impl crate::bounded::VecSink<String>) {
    let filter = profile.filter_expression.trim();
    if filter.is_empty() {
        diagnostics.push_item(format!("missing-filter:{}", profile.profile_id));
        return;
    }
    if required_semantic_profiles().contains(&profile.profile_id.as_str()) && filter == NEXTEST_ALL_FILTER {
        diagnostics.push_item(format!("unpartitioned-filter:{}", profile.profile_id));
    }
    if required_semantic_profiles().contains(&profile.profile_id.as_str())
        && !filter.contains(NEXTEST_PARTITION_SELECTOR)
    {
        diagnostics.push_item(format!("missing-metadata-selector:{}", profile.profile_id));
    }
}

fn validate_expected_profile_artifacts(
    profile: &SemanticProfileInput,
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) -> Result<()> {
    if profile.expected_artifacts.is_empty() {
        diagnostics.push_item(format!("missing-expected-artifacts:{}", profile.profile_id));
    }
    for artifact in &profile.expected_artifacts {
        validate_text("profile expected artifact", artifact)?;
    }
    if artifact_present(&profile.expected_artifacts, JUNIT_ARTIFACT)
        && !artifact_present(&profile.expected_artifacts, CANONICAL_TEST_RUN_ARTIFACT)
    {
        diagnostics.push_item(format!("junit-without-canonical-test-run:{}", profile.profile_id));
    }
    if !artifact_present(&profile.expected_artifacts, PROFILE_METADATA_ARTIFACT) {
        diagnostics.push_item(format!("missing-profile-metadata-artifact:{}", profile.profile_id));
    }
    if !artifact_present(&profile.expected_artifacts, FILTER_READBACK_ARTIFACT) {
        diagnostics.push_item(format!("missing-filter-readback-artifact:{}", profile.profile_id));
    }
    Ok(())
}

fn validate_profile_junit_path(profile: &SemanticProfileInput, diagnostics: &mut impl crate::bounded::VecSink<String>) {
    let path = profile.expected_junit_path.trim();
    if path.is_empty() {
        diagnostics.push_item(format!("missing-junit-path:{}", profile.profile_id));
        return;
    }
    if !path.ends_with(NEXTEST_JUNIT_RELATIVE_PATH) {
        diagnostics.push_item(format!("unsupported-junit-path:{}", profile.profile_id));
    }
}

fn validate_excluded_partitions(
    profile: &SemanticProfileInput,
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) -> Result<()> {
    for partition in &profile.excluded_partitions {
        validate_text("profile excluded partition", partition)?;
        if !allowed_excluded_partitions().contains(&partition.as_str()) {
            diagnostics.push_item(format!("unsupported-excluded-partition:{}:{partition}", profile.profile_id));
        }
    }
    Ok(())
}

fn validate_deterministic_profile_exclusions(
    profile: &SemanticProfileInput,
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) {
    if !deterministic_profile_requires_exclusions(&profile.profile_id) {
        return;
    }
    for required in required_deterministic_exclusions() {
        if !profile.excluded_partitions.iter().any(|partition| partition == required) {
            diagnostics.push_item(format!("missing-deterministic-exclusion:{}:{required}", profile.profile_id));
        }
    }
}

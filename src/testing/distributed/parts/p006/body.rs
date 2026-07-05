fn composite_fault_suite_value(
    decision: &str,
    case_refs: &[String],
    run_refs: &[String],
    diagnostics: &[String],
) -> Result<IoValue> {
    validate_decision(decision)?;
    validate_ref_slice("composite case", case_refs)?;
    validate_ref_slice("composite run", run_refs)?;
    validate_strings("composite diagnostic", diagnostics, MAX_DISTRIBUTED_TEXT)?;
    Ok(record("composite-fault-suite-v1", vec![
        string(COMPOSITE_FAULT_SUITE_SCHEMA),
        record("decision", vec![string(decision)]),
        record("cases", vec![refs_sequence(case_refs)]),
        record("runs", vec![refs_sequence(run_refs)]),
        record("diagnostics", vec![sequence(diagnostics.iter().map(string).collect())]),
        checks_value(&[
            ("named-cases-bound", status(!case_refs.is_empty())),
            ("expected-decisions-honored", status(decision == PASS_DECISION)),
            ("simulation-evidence-not-vm-evidence", PASS_DECISION),
        ]),
    ]))
}

fn generated_case_promotion_diagnostics(input: &GeneratedCasePromotionInput) -> Result<Vec<String>> {
    validate_text("promotion case id", &input.case_id)?;
    validate_text("promotion invariant", &input.invariant_name)?;
    validate_ref(&input.seed_ref, "promotion seed")?;
    validate_ref(&input.topology_ref, "promotion topology")?;
    validate_ref(&input.scheduler_ref, "promotion scheduler")?;
    validate_ref(&input.fault_plan_ref, "promotion fault plan")?;
    validate_ref_slice("promotion command", &input.command_refs)?;
    validate_ref(&input.replay_ref, "promotion replay")?;
    validate_ref_slice("promotion diagnostic", &input.diagnostic_refs)?;
    validate_strings("promotion profile", &input.profile_eligibility, MAX_DISTRIBUTED_TEXT)?;
    validate_ref_slice("promotion traceability", &input.traceability_refs)?;
    validate_ref_slice("promotion variance", &input.variance_refs)?;
    validate_cost_class(&input.cost_class)?;
    validate_release_status(&input.release_review_status)?;
    validate_strings("promotion caveat", &input.caveats, MAX_DISTRIBUTED_TEXT)?;
    let mut diagnostics = Vec::new();
    if input.command_refs.is_empty() {
        diagnostics.push_bounded(format!("promotion-missing-command:{}", input.case_id))?;
    }
    if input.diagnostic_refs.is_empty() {
        diagnostics.push_bounded(format!("promotion-missing-diagnostic:{}", input.case_id))?;
    }
    if input.profile_eligibility.is_empty() {
        diagnostics.push_bounded(format!("promotion-missing-profile:{}", input.case_id))?;
    }
    if input.traceability_refs.is_empty() {
        diagnostics.push_bounded(format!("promotion-missing-traceability:{}", input.case_id))?;
    }
    if input.retry_attempts > 0 {
        diagnostics.push_bounded(format!("promotion-retry-only-success:{}", input.case_id))?;
    }
    if input.variance_refs.is_empty() {
        diagnostics.push_bounded(format!("promotion-undeclared-variance:{}", input.case_id))?;
    }
    if input.caveats.is_empty() {
        diagnostics.push_bounded(format!("promotion-missing-caveat:{}", input.case_id))?;
    }
    if input.diagnostic_only && input.release_review_status == RELEASE_REQUIRED {
        diagnostics.push_bounded(format!("promotion-diagnostic-only-release-claim:{}", input.case_id))?;
    }
    Ok(diagnostics)
}

fn generated_case_promotion_value(
    input: &GeneratedCasePromotionInput,
    decision: &str,
    diagnostics: &[String],
) -> Result<IoValue> {
    validate_decision(decision)?;
    validate_strings("promotion diagnostic", diagnostics, MAX_DISTRIBUTED_TEXT)?;
    Ok(record("generated-case-promotion-v1", vec![
        string(GENERATED_CASE_PROMOTION_SCHEMA),
        record("decision", vec![string(decision)]),
        record("case", vec![string(&input.case_id)]),
        record("invariant", vec![string(&input.invariant_name)]),
        record("seed", vec![string(&input.seed_ref)]),
        record("topology", vec![string(&input.topology_ref)]),
        record("scheduler", vec![string(&input.scheduler_ref)]),
        record("fault-plan", vec![string(&input.fault_plan_ref)]),
        record("commands", vec![refs_sequence(&input.command_refs)]),
        record("replay", vec![string(&input.replay_ref)]),
        record("diagnostics", vec![refs_sequence(&input.diagnostic_refs)]),
        record("profiles", vec![sequence(input.profile_eligibility.iter().map(string).collect())]),
        record("traceability", vec![refs_sequence(&input.traceability_refs)]),
        record("retry-attempts", vec![u64_value(input.retry_attempts)]),
        record("variance", vec![refs_sequence(&input.variance_refs)]),
        record("cost-class", vec![string(&input.cost_class)]),
        record("release-review-status", vec![string(&input.release_review_status)]),
        record("diagnostic-only", vec![crate::preserves_rail::bool_value(input.diagnostic_only)]),
        record("diagnostics-text", vec![sequence(diagnostics.iter().map(string).collect())]),
        record("caveats", vec![sequence(input.caveats.iter().map(string).collect())]),
        checks_value(&[
            ("stable-refs-bound", status(decision == PASS_DECISION)),
            ("traceability-required", status(!input.traceability_refs.is_empty())),
            ("retry-only-success-denied", status(input.retry_attempts == 0)),
        ]),
    ]))
}

fn status(condition: bool) -> &'static str {
    if condition { PASS_DECISION } else { "fail" }
}


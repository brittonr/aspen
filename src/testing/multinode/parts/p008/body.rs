
fn scenario_fixture_diagnostics(
    fixture: &ScenarioFixture,
    execution_profiles: &[crate::distributed_core::CiProfile],
    topology_profiles: &[TopologyProfile],
) -> Result<Vec<String>> {
    let mut diagnostics = Vec::new();
    collect_required_text_diagnostic("scenario-id", &fixture.scenario_id, &mut diagnostics)?;
    collect_required_text_diagnostic("purpose", &fixture.purpose, &mut diagnostics)?;
    collect_required_text_diagnostic("evidence-scope", &fixture.evidence_scope, &mut diagnostics)?;
    collect_required_text_diagnostic("topology-profile", &fixture.topology_profile_id, &mut diagnostics)?;
    collect_required_text_diagnostic("execution-profile", &fixture.execution_profile_id, &mut diagnostics)?;
    collect_required_text_diagnostic("command-surface", &fixture.command_surface, &mut diagnostics)?;
    collect_required_text_diagnostic("unavailable-policy", &fixture.unavailable_policy, &mut diagnostics)?;
    push_if(&mut diagnostics, fixture.expected_artifact_kinds.is_empty(), "fixture-missing-artifact-kind")?;
    push_if(&mut diagnostics, fixture.receipt_refs.is_empty(), "fixture-missing-receipt-ref")?;
    push_if(&mut diagnostics, fixture.variance_refs.is_empty(), "fixture-missing-variance-ref")?;
    push_if(&mut diagnostics, fixture.diagnostic_log_refs.is_empty(), "fixture-missing-diagnostic-log-ref")?;
    push_if(&mut diagnostics, fixture.caveats.is_empty(), "fixture-missing-evidence-caveat")?;
    if fixture.unsupported_claims_pass {
        push_diagnostic(&mut diagnostics, "fixture-unsupported-pass-claim".to_string())?;
    }
    collect_invalid_ref_diagnostics(
        "scenario fixture",
        &[
            fixture.topology_ref.clone(),
            fixture.seed_ref.clone(),
            fixture.fault_plan_ref.clone(),
        ],
        &mut diagnostics,
    )?;
    collect_invalid_ref_diagnostics("scenario receipt", &fixture.receipt_refs, &mut diagnostics)?;
    collect_invalid_ref_diagnostics("scenario variance", &fixture.variance_refs, &mut diagnostics)?;
    collect_invalid_ref_diagnostics("scenario diagnostic log", &fixture.diagnostic_log_refs, &mut diagnostics)?;
    match execution_profiles.iter().find(|profile| profile.id == fixture.execution_profile_id) {
        Some(profile) => {
            if fixture.command_surface != profile.command {
                push_diagnostic(&mut diagnostics, "fixture-command-profile-mismatch".to_string())?;
            }
            if fixture.expected_artifact_kinds != profile.expected_artifact_kinds {
                push_diagnostic(&mut diagnostics, "fixture-artifact-kind-mismatch".to_string())?;
            }
        }
        None => push_diagnostic(&mut diagnostics, "fixture-execution-profile-missing".to_string())?,
    }
    if !topology_profiles.iter().any(|profile| profile.id == fixture.topology_profile_id) {
        push_diagnostic(&mut diagnostics, "fixture-topology-profile-missing".to_string())?;
    }
    Ok(diagnostics)
}


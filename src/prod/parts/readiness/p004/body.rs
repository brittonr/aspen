
pub fn deployment_profile_value(input: &DeploymentProfileInput<'_>) -> Result<IoValue> {
    validate_decision(input.decision)?;
    validate_text_field("profile name", input.profile_name)?;
    validate_profile_metadata(input)?;
    validate_diagnostics(input.diagnostics)?;
    require_pass_refs("state layout", input.state_layout_refs, input.decision)?;
    require_pass_refs("required adapter", input.required_adapter_refs, input.decision)?;
    require_pass_refs("source gate", input.source_gate_refs, input.decision)?;
    require_pass_refs("resource limit", input.resource_limit_refs, input.decision)?;
    require_pass_refs("redaction setting", input.redaction_setting_refs, input.decision)?;
    require_pass_refs("live transport", input.live_transport_refs, input.decision)?;
    require_pass_refs("startup expectation", input.startup_expectation_refs, input.decision)?;
    require_pass_refs("shutdown expectation", input.shutdown_expectation_refs, input.decision)?;
    Ok(record("prod-ops-deployment-profile-v1", vec![
        string(PROD_OPS_DEPLOYMENT_PROFILE_SCHEMA),
        decision_field(input.decision),
        record("profile", vec![string(input.profile_name)]),
        record("schema-id", vec![string(input.schema_id)]),
        record("schema-version", vec![u64_value(input.schema_version)]),
        record("source-language", vec![string(input.source_language)]),
        record("profile-identity", vec![string(input.profile_identity)]),
        record("profile-ref", vec![string(input.profile_ref)]),
        refs_field("state-layout", input.state_layout_refs)?,
        refs_field("required-adapters", input.required_adapter_refs)?,
        refs_field("source-gates", input.source_gate_refs)?,
        refs_field("resource-limits", input.resource_limit_refs)?,
        refs_field("redaction-settings", input.redaction_setting_refs)?,
        refs_field("live-transport", input.live_transport_refs)?,
        refs_field("startup-expectations", input.startup_expectation_refs)?,
        refs_field("shutdown-expectations", input.shutdown_expectation_refs)?,
        diagnostics_field(input.diagnostics)?,
        checks_field(vec![
            check_value("explicit-state-layout", pass_check(input.state_layout_refs.is_empty())),
            check_value("required-adapters-bound", pass_check(input.required_adapter_refs.is_empty())),
            check_value("source-gate-inputs-bound", pass_check(input.source_gate_refs.is_empty())),
            check_value(
                "resource-redaction-live-settings-bound",
                pass_check(
                    input.resource_limit_refs.is_empty()
                        || input.redaction_setting_refs.is_empty()
                        || input.live_transport_refs.is_empty(),
                ),
            ),
            check_value("profile-metadata-bound", "pass"),
            check_value("profile-receipt-does-not-grant-authority", "pass"),
            check_value("profile-metadata-does-not-grant-subsystem-trust", "pass"),
        ]),
    ]))
}

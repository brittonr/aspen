
fn collect_override_diagnostic(
    profile: &CheckedNodeProfile,
    field: &str,
    value: &str,
    diagnostics: &mut impl crate::bounded::VecSink<String>,
    accepted: &mut impl crate::bounded::VecSink<String>,
) {
    if profile.overrideable_fields.iter().any(|allowed| allowed == field) && profile.tier != TIER_RELEASE {
        accepted.push_item(format!("accepted-override:{field}={value}"));
    } else {
        diagnostics.push_item(format!("denied-profile-override:{field}"));
    }
}

fn effective_profile(profile: &CheckedNodeProfile, overrides: &NodeProfileOverrides) -> CheckedNodeProfile {
    let mut effective = profile.clone();
    if profile.tier != TIER_RELEASE {
        if let Some(state_root_ref) = overrides.state_root_ref.as_ref()
            && profile.overrideable_fields.iter().any(|field| field == OVERRIDE_STATE_ROOT_REF)
        {
            effective.state_root_ref.clone_from(state_root_ref);
        }
        if let Some(adapters) = overrides.adapters.as_ref()
            && profile.overrideable_fields.iter().any(|field| field == OVERRIDE_ADAPTER_REFS)
        {
            effective.adapters.clone_from(adapters);
        }
        if let Some(policy_refs) = overrides.policy_refs.as_ref()
            && profile.overrideable_fields.iter().any(|field| field == OVERRIDE_POLICY_REFS)
        {
            effective.policy_refs.clone_from(policy_refs);
        }
    }
    effective
}

struct FinishResolutionInput<'a> {
    identity_ref: &'a str,
    profile_ref: &'a str,
    tier: &'a str,
    schema_id: &'a str,
    schema_version: &'a str,
    source_language: &'a str,
    profile_identity: &'a str,
    accepted_overrides: Vec<String>,
    diagnostics: Vec<String>,
    config_value: IoValue,
    caveats: Vec<String>,
}

fn finish_resolution(input: FinishResolutionInput<'_>) -> Result<ResolvedNodeConfig> {
    let FinishResolutionInput {
        identity_ref,
        profile_ref,
        tier,
        schema_id,
        schema_version,
        source_language,
        profile_identity,
        accepted_overrides,
        mut diagnostics,
        config_value,
        caveats,
    } = input;
    diagnostics.sort();
    diagnostics.dedup();
    ensure_diagnostic_bound(diagnostics.len())?;
    let config_ref = crate::preserves_rail::canonical_hash(&config_value)?;
    let decision = if diagnostics.iter().any(|diagnostic| {
        diagnostic.starts_with("denied")
            || diagnostic.contains("mismatch")
            || diagnostic.contains("unsupported")
            || diagnostic.contains("missing-required")
            || diagnostic.contains("runtime-nickel")
    }) {
        DECISION_DENY
    } else {
        DECISION_PASS
    };
    let resolution_value = resolution_value(ResolutionValueInput {
        decision,
        identity_ref,
        profile_ref,
        config_ref: &config_ref,
        tier,
        schema_id,
        schema_version,
        source_language,
        profile_identity,
        accepted_overrides: &accepted_overrides,
        diagnostics: &diagnostics,
        caveats: &caveats,
    })?;
    let resolution_ref = crate::preserves_rail::canonical_hash(&resolution_value)?;
    let profile_metadata_refs = vec![profile_ref.to_string(), resolution_ref.clone()];
    Ok(ResolvedNodeConfig {
        decision: decision.to_string(),
        diagnostics,
        accepted_overrides,
        profile_metadata_refs,
        config_ref,
        config_value,
        resolution_ref,
        resolution_value,
    })
}

struct ResolutionValueInput<'a> {
    decision: &'a str,
    identity_ref: &'a str,
    profile_ref: &'a str,
    config_ref: &'a str,
    tier: &'a str,
    schema_id: &'a str,
    schema_version: &'a str,
    source_language: &'a str,
    profile_identity: &'a str,
    accepted_overrides: &'a [String],
    diagnostics: &'a [String],
    caveats: &'a [String],
}

fn resolution_value(input: ResolutionValueInput<'_>) -> Result<IoValue> {
    let mut caveats = input.caveats.to_vec();
    caveats.push(EVIDENCE_ONLY_CAVEAT.to_string());
    Ok(record("node-profile-config-resolution-v1", vec![
        string(PROFILE_RESOLUTION_SCHEMA),
        field_string("decision", input.decision),
        field_string("identity", input.identity_ref),
        field_string("profile", input.profile_ref),
        field_string("node-config", input.config_ref),
        record("metadata", vec![
            field_string("tier", input.tier),
            field_string("schema-id", input.schema_id),
            field_string("schema-version", input.schema_version),
            field_string("source-language", input.source_language),
            field_string("profile-identity", input.profile_identity),
        ]),
        field_sequence("accepted-overrides", string_values(input.accepted_overrides)?),
        field_sequence("diagnostics", string_values(input.diagnostics)?),
        field_sequence("caveats", string_values(&caveats)?),
    ]))
}

fn validate_override_field(field: &str) -> Result<()> {
    match field {
        OVERRIDE_STATE_ROOT_REF | OVERRIDE_POLICY_REFS | OVERRIDE_ADAPTER_REFS => Ok(()),
        other => Err(MoltenError::invalid_harness(format!("unsupported node profile override field {other}"))),
    }
}

fn is_required_runtime_adapter(name: &str) -> bool {
    crate::node_runtime::REQUIRED_RUNTIME_ADAPTERS.iter().any(|required| required == &name)
}

fn validate_refs(refs: &[String], label: &str) -> Result<()> {
    crate::bounded::ensure_count_at_most(refs.len(), MAX_REFS, label)?;
    for reference in refs {
        validate_ref(reference, label)?;
    }
    Ok(())
}

fn validate_ref(reference: &str, label: &str) -> Result<()> {
    crate::preserves_rail::validate_content_ref(reference)
        .map_err(|error| MoltenError::invalid_harness(format!("invalid {label} {reference}: {error}")))
}

fn validate_text(label: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        Err(MoltenError::invalid_harness(format!("{label} must not be empty")))
    } else {
        Ok(())
    }
}

fn record(label: &'static str, fields: Vec<IoValue>) -> IoValue {
    crate::preserves_rail::record(label, fields)
}

fn field_string(label: &'static str, value: &str) -> IoValue {
    record(label, vec![string(value)])
}

fn field_sequence(label: &'static str, values: Vec<IoValue>) -> IoValue {
    record(label, vec![crate::preserves_rail::sequence(values)])
}

fn string(value: &str) -> IoValue {
    crate::preserves_rail::string(value)
}

fn string_values(values: &[String]) -> Result<Vec<IoValue>> {
    ensure_diagnostic_bound(values.len())?;
    Ok(values.iter().map(|value| string(value)).collect())
}

fn ensure_diagnostic_bound(count: usize) -> Result<()> {
    crate::bounded::ensure_count_at_most(count, MAX_DIAGNOSTICS, "node profile diagnostics")
}

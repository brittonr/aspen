
fn validate_required_refs(
    requirements: &OperationRequirements,
    refs: &ContextRefSet,
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) {
    if requirements.require_policy && refs.policy_refs.is_empty() {
        diagnostics.push_item(format!("missing-required-policy:{}", requirements.operation));
    }
    if requirements.require_authority && refs.authority_refs.is_empty() {
        diagnostics.push_item(format!("missing-required-authority:{}", requirements.operation));
    }
    if requirements.require_resource && refs.resource_refs.is_empty() {
        diagnostics.push_item(format!("missing-required-resource:{}", requirements.operation));
    }
    if requirements.require_evidence && refs.evidence_refs.is_empty() {
        diagnostics.push_item(format!("missing-required-evidence:{}", requirements.operation));
    }
    if requirements.require_retention && refs.retention_refs.is_empty() {
        diagnostics.push_item(format!("missing-required-retention:{}", requirements.operation));
    }
}

fn context_profile_value(input: &ContextProfileInput, decision: &str, diagnostics: &[String]) -> Result<IoValue> {
    Ok(record("context-profile-v1", vec![
        string(CONTEXT_PROFILE_SCHEMA),
        field_string("decision", decision),
        field_string("profile-id", &input.profile_id),
        field_string("profile-tier", &input.profile_tier),
        ref_set_value("refs", &input.refs)?,
        field_sequence("allowed-operations", string_values(&input.allowed_operations)?),
        field_sequence("diagnostics", string_values(diagnostics)?),
        field_sequence("caveats", string_values(&context_caveats(&input.caveats))?),
    ]))
}

struct ExpansionValueInput<'a> {
    profile_ref: &'a str,
    requirements: &'a OperationRequirements,
    overrides: &'a ContextOverrideInput,
    expanded_refs: &'a ContextRefSet,
    decision: &'a str,
    diagnostics: &'a [String],
}

fn context_expansion_value(input: ExpansionValueInput<'_>) -> Result<IoValue> {
    let ExpansionValueInput {
        profile_ref,
        requirements,
        overrides,
        expanded_refs,
        decision,
        diagnostics,
    } = input;
    Ok(record("context-profile-expansion-v1", vec![
        string(CONTEXT_EXPANSION_SCHEMA),
        field_string("decision", decision),
        field_string("profile-ref", profile_ref),
        field_string("operation", &requirements.operation),
        requirement_value(requirements),
        ref_set_value("overrides", &override_ref_set(overrides))?,
        ref_set_value("expanded-refs", expanded_refs)?,
        field_sequence("diagnostics", string_values(diagnostics)?),
        field_sequence("caveats", string_values(&[EVIDENCE_ONLY_CAVEAT.to_string()])?),
    ]))
}

fn requirement_value(requirements: &OperationRequirements) -> IoValue {
    record("requirements", vec![
        record("policy", vec![bool_value(requirements.require_policy)]),
        record("authority", vec![bool_value(requirements.require_authority)]),
        record("resource", vec![bool_value(requirements.require_resource)]),
        record("evidence", vec![bool_value(requirements.require_evidence)]),
        record("retention", vec![bool_value(requirements.require_retention)]),
    ])
}

fn override_ref_set(overrides: &ContextOverrideInput) -> ContextRefSet {
    ContextRefSet {
        policy_refs: overrides.policy_refs.clone(),
        capability_refs: Vec::new(),
        authority_refs: overrides.authority_refs.clone(),
        resource_refs: overrides.resource_refs.clone(),
        evidence_refs: overrides.evidence_refs.clone(),
        redaction_refs: Vec::new(),
        retention_refs: overrides.retention_refs.clone(),
    }
}

fn context_caveats(caveats: &[String]) -> Vec<String> {
    let mut output = caveats.to_vec();
    output.push(EVIDENCE_ONLY_CAVEAT.to_string());
    output
}

fn ref_set_value(label: &'static str, refs: &ContextRefSet) -> Result<IoValue> {
    Ok(record(label, vec![
        field_sequence("policy", string_values(&refs.policy_refs)?),
        field_sequence("capability", string_values(&refs.capability_refs)?),
        field_sequence("authority", string_values(&refs.authority_refs)?),
        field_sequence("resource", string_values(&refs.resource_refs)?),
        field_sequence("evidence", string_values(&refs.evidence_refs)?),
        field_sequence("redaction", string_values(&refs.redaction_refs)?),
        field_sequence("retention", string_values(&refs.retention_refs)?),
    ]))
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

fn bool_value(value: bool) -> IoValue {
    crate::preserves_rail::bool_value(value)
}

fn validate_ref(reference: &str, label: &str) -> Result<()> {
    crate::preserves_rail::validate_content_ref(reference)
        .map_err(|error| MoltenError::invalid_harness(format!("invalid {label} ref {reference}: {error}")))
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
        if let Err(error) = validate_ref(reference, label) {
            diagnostics.push_item(format!("stale-ref:{label}:{reference}:{error}"));
        }
    }
    Ok(())
}

fn validate_text(label: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        Err(MoltenError::invalid_harness(format!("{label} must not be empty")))
    } else {
        Ok(())
    }
}

fn ensure_ref_bound(count: usize, label: &str) -> Result<()> {
    crate::bounded::ensure_count_at_most(count, MAX_REFS, label)
}

fn ensure_scope_bound(count: usize, label: &str) -> Result<()> {
    crate::bounded::ensure_count_at_most(count, MAX_SCOPES, label)
}

fn ensure_caveat_bound(count: usize, label: &str) -> Result<()> {
    crate::bounded::ensure_count_at_most(count, MAX_CAVEATS, label)
}

fn ensure_diagnostic_bound(count: usize) -> Result<()> {
    crate::bounded::ensure_count_at_most(count, MAX_DIAGNOSTICS, "context diagnostics")
}

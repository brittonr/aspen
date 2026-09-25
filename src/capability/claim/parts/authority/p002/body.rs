
fn validate_selector(selector: &ClaimSubjectSelector) -> Result<()> {
    match selector.selector_kind.as_str() {
        SELECTOR_EXACT_REF => validate_ref(&selector.selector_value, "exact claim subject ref")?,
        SELECTOR_REF_PREFIX
        | SELECTOR_ARTIFACT_CLASS
        | SELECTOR_NAMESPACE
        | SELECTOR_SCHEMA_ID
        | SELECTOR_RELEASE_CHANNEL
        | SELECTOR_CLUSTER_ID
        | SELECTOR_POLICY_DEFINED => validate_text("claim selector value", &selector.selector_value)?,
        other => return Err(MoltenError::invalid_harness(format!("unsupported claim selector kind {other}"))),
    }
    validate_text("claim selector subject kind", &selector.subject_kind)?;
    validate_refs(&selector.policy_refs, "claim selector policy ref")?;
    validate_refs(&selector.resource_refs, "claim selector resource ref")?;
    validate_caveats(&selector.caveats)
}

fn validate_claim(claim: &AuthorityClaim) -> Result<()> {
    validate_ref(&claim.issuer_ref, "claim issuer ref")?;
    validate_ref(&claim.holder_ref, "claim holder ref")?;
    validate_ref(&claim.session_ref, "claim session ref")?;
    validate_ref(&claim.context_ref, "claim context ref")?;
    validate_ref(&claim.subject_selector_ref, "claim selector ref")?;
    validate_refs(&claim.exact_subject_refs, "claim exact subject ref")?;
    validate_text("claim kind", &claim.claim_kind)?;
    validate_ref(&claim.claim_value_ref, "claim value ref")?;
    validate_refs(&claim.evidence_refs, "claim evidence ref")?;
    validate_refs(&claim.policy_refs, "claim policy ref")?;
    validate_refs(&claim.resource_refs, "claim resource ref")?;
    validate_ref(&claim.freshness_ref, "claim freshness ref")?;
    validate_refs(&claim.revocation_refs, "claim revocation ref")?;
    validate_caveats(&claim.caveats)
}

fn validate_caveats(caveats: &[String]) -> Result<()> {
    crate::bounded::ensure_count_at_most(caveats.len(), MAX_CAVEATS, "claim caveats")?;
    for caveat in caveats {
        validate_text("claim caveat", caveat)?;
    }
    Ok(())
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

fn admission_text_contains(value: &IoValue, needle: &str) -> Result<bool> {
    Ok(crate::preserves_rail::to_text(value)?.contains(needle))
}

fn canonical_hash(value: &IoValue) -> Result<String> {
    crate::preserves_rail::canonical_hash(value)
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

fn ref_values(refs: &[String]) -> Result<Vec<IoValue>> {
    validate_refs(refs, "claim ref")?;
    Ok(refs.iter().map(|reference| string(reference)).collect())
}

fn string_values(values: &[String]) -> Result<Vec<IoValue>> {
    ensure_diagnostic_bound(values.len())?;
    Ok(values.iter().map(|value| string(value)).collect())
}

fn checks_value(checks: &[(&'static str, &'static str)]) -> IoValue {
    record("checks", vec![crate::preserves_rail::sequence(
        checks.iter().map(|(name, state)| record("check", vec![string(name), string(state)])).collect(),
    )])
}

fn status(value: bool) -> &'static str {
    if value { "pass" } else { "fail" }
}

fn ensure_diagnostic_bound(count: usize) -> Result<()> {
    crate::bounded::ensure_count_at_most(count, MAX_DIAGNOSTICS, "claim authority diagnostics")
}

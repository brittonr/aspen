
fn validate_optional_ref(value: Option<&str>, field: &str) -> Result<()> {
    if let Some(value) = value {
        validate_ref(value, field)
    } else {
        Ok(())
    }
}

fn validate_refs(values: &[String], field: &str) -> Result<()> {
    ensure_count_at_most(values.len(), MAX_PLUGIN_REFS, field)?;
    for value in values {
        validate_ref(value, field)?;
    }
    Ok(())
}

fn require_non_empty_refs(values: &[String], field: &str) -> Result<()> {
    if values.is_empty() {
        return Err(MoltenError::invalid_harness(format!("{field} must not be empty")));
    }
    validate_refs(values, field)
}

fn validate_diagnostics(values: &[String]) -> Result<()> {
    ensure_count_at_most(values.len(), MAX_PLUGIN_DIAGNOSTICS, "plugin diagnostics")
}

fn ensure_count_at_most(count: usize, maximum: usize, label: &str) -> Result<()> {
    crate::bounded::ensure_count_at_most(count, maximum, label)
}

fn status(value: bool) -> &'static str {
    if value { PLUGIN_DECISION_PASS } else { PLUGIN_CHECK_FAIL }
}

fn refs_sequence(refs: &[String]) -> IoValue {
    sequence(refs.iter().map(string).collect())
}

fn strings_sequence(values: &[String]) -> IoValue {
    sequence(values.iter().map(string).collect())
}

fn optional_ref_value(value: Option<&str>) -> IoValue {
    value.map_or_else(|| record("none", Vec::new()), |value| record("some", vec![string(value)]))
}

fn optional_text_value(value: Option<&str>) -> IoValue {
    value.map_or_else(|| record("none", Vec::new()), |value| record("some", vec![string(value)]))
}

fn checks_value(checks: &[(&str, &str)]) -> IoValue {
    record("checks", vec![sequence(
        checks.iter().map(|(name, status)| record("check", vec![string(name), string(status)])).collect(),
    )])
}

fn simple_record<'a>(
    value: &'a IoValue,
    label: &str,
    arity: usize,
) -> Result<std::borrow::Cow<'a, preserves::Record<Value<IoValue>>>> {
    value
        .collect_simple_record(label, Some(arity))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...> with arity {arity}")))
}

fn simple_record_any<'a>(
    value: &'a IoValue,
    label: &str,
) -> Result<std::borrow::Cow<'a, preserves::Record<Value<IoValue>>>> {
    value
        .collect_simple_record(label, None)
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...>")))
}

fn record_arity(record: &preserves::Record<Value<IoValue>>) -> usize {
    record._vec().len().saturating_sub(1)
}

fn record_decision(value: &Value<IoValue>, label: &str) -> Result<String> {
    let decision = record_string(value, label)?;
    validate_decision(&decision)?;
    Ok(decision)
}

fn validate_decision(value: &str) -> Result<()> {
    match value {
        PLUGIN_DECISION_PASS | PLUGIN_DECISION_DENY => Ok(()),
        _ => Err(MoltenError::invalid_harness(format!("plugin receipt decision {value} must be pass or deny"))),
    }
}

fn require_check_status(checks: &[(String, String)], expected: &str, status: &str, context: &str) -> Result<()> {
    match checks.iter().find(|(name, _)| name == expected) {
        Some((_, actual)) if actual == status => Ok(()),
        Some((_, actual)) => Err(MoltenError::invalid_harness(format!(
            "{context} {expected} check has status {actual}, expected {status}"
        ))),
        None => Err(MoltenError::invalid_harness(format!("{context} missing {expected} check"))),
    }
}

fn validate_receipt_coherence(
    decision: &str,
    checks: &[(String, String)],
    diagnostics: &[String],
    context: &str,
) -> Result<()> {
    let has_failed_check = checks.iter().any(|(_, status)| status == PLUGIN_CHECK_FAIL);
    if decision == PLUGIN_DECISION_PASS && has_failed_check {
        return Err(MoltenError::invalid_harness(format!(
            "{context} pass decision carries failed required checks"
        )));
    }
    if decision == PLUGIN_DECISION_DENY && !has_failed_check && diagnostics.is_empty() {
        return Err(MoltenError::invalid_harness(format!(
            "{context} deny decision requires failed checks or diagnostics"
        )));
    }
    Ok(())
}

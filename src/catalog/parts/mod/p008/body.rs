
fn classify_short_id_prefix(prefix: &str) -> ShortIdPrefix<'_> {
    if validate_content_ref(prefix).is_ok() {
        return ShortIdPrefix::FullRef;
    }
    if crate::preserves_rail::content_ref_has_prefix(prefix) {
        let error = validate_content_ref(prefix).expect_err("invalid content ref after failed validation");
        return ShortIdPrefix::Deny(format!("malformed full content ref: {error}"));
    }
    if !prefix.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)) {
        return ShortIdPrefix::Deny("short id prefix must use lowercase hex characters".to_string());
    }
    ShortIdPrefix::HexPrefix(prefix)
}

fn canonical_ref_matches_prefix(candidate: &str, normalized_prefix: &str) -> bool {
    crate::preserves_rail::content_ref_hex(candidate).is_ok_and(|hex| hex.starts_with(normalized_prefix))
}

fn refs_sequence(refs: &[String]) -> IoValue {
    sequence(refs.iter().map(string).collect())
}

fn optional_ref_value(value: Option<&str>) -> IoValue {
    value.map_or_else(|| record("none", Vec::new()), |value| record("some", vec![string(value)]))
}

fn parse_optional_ref_value(value: &PreservesValue<IoValue>) -> Result<Option<String>> {
    if value.collect_simple_record("none", Some(0)).is_some() {
        return Ok(None);
    }
    if let Some(some) = value.collect_simple_record("some", Some(1)) {
        return required_ref(&some[0], "optional ref").map(Some);
    }
    required_ref(value, "optional ref").map(Some)
}

fn checks_value(names: &[&str]) -> IoValue {
    checks_value_from_pairs(&names.iter().map(|name| (*name, "pass")).collect::<Vec<_>>())
}

fn checks_value_from_pairs(checks: &[(&str, &str)]) -> IoValue {
    record("checks", vec![sequence(
        checks.iter().map(|(name, status)| record("check", vec![string(name), string(status)])).collect(),
    )])
}

fn parse_checks(value: &PreservesValue<IoValue>) -> Result<Vec<String>> {
    let value = value_to_iovalue(value);
    let checks = simple_record(&value, "checks", 1)?;
    let items = required_sequence(&checks[0], "catalog checks")?;
    ensure_count_at_most(items.len(), MAX_CATALOG_CHECKS, "catalog checks")?;
    let mut parsed = Vec::new();
    for item in items.iter() {
        let item = value_to_iovalue(item);
        let check = simple_record(&item, "check", 2)?;
        let name = required_string(&check[0], "catalog check name")?;
        let status = required_string(&check[1], "catalog check status")?;
        if status != "pass" && status != "fail" {
            return Err(MoltenError::invalid_harness(format!("catalog check {name} has status {status}")));
        }
        push_bounded(&mut parsed, name, MAX_CATALOG_CHECKS, "catalog checks")?;
    }
    Ok(parsed)
}

fn require_check(checks: &[String], expected: &str, context: &str) -> Result<()> {
    if checks.iter().any(|check| check == expected) {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("{context} missing {expected} check")))
    }
}

fn require_schema(value: &PreservesValue<IoValue>, expected: &str, context: &str) -> Result<()> {
    let actual = required_string(value, context)?;
    if actual == expected {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("unsupported {context} schema {actual}; expected {expected}")))
    }
}

fn simple_record<'a>(
    value: &'a IoValue,
    label: &str,
    arity: usize,
) -> Result<std::borrow::Cow<'a, PreservesRecord<PreservesValue<IoValue>>>> {
    value
        .collect_simple_record(label, Some(arity))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...> with arity {arity}")))
}

#[allow(clippy::owned_cow)]
fn required_sequence<'a>(
    value: &'a PreservesValue<IoValue>,
    field: &str,
) -> Result<std::borrow::Cow<'a, Vec<PreservesValue<IoValue>>>> {
    value
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected sequence for {field}")))
}

fn record_string(value: &PreservesValue<IoValue>, label: &str) -> Result<String> {
    let value = value_to_iovalue(value);
    let fields = simple_record(&value, label, 1)?;
    required_string(&fields[0], label)
}

fn record_ref(value: &PreservesValue<IoValue>, label: &str) -> Result<String> {
    let value = value_to_iovalue(value);
    let fields = simple_record(&value, label, 1)?;
    required_ref(&fields[0], label)
}

fn record_optional_ref(value: &PreservesValue<IoValue>, label: &str) -> Result<Option<String>> {
    let value = value_to_iovalue(value);
    let fields = simple_record(&value, label, 1)?;
    parse_optional_ref_value(&fields[0])
}

fn record_ref_sequence(value: &PreservesValue<IoValue>, label: &str) -> Result<Vec<String>> {
    let value = value_to_iovalue(value);
    let fields = simple_record(&value, label, 1)?;
    parse_ref_sequence_value(&fields[0], label)
}

fn record_string_sequence(value: &PreservesValue<IoValue>, label: &str) -> Result<Vec<String>> {
    let value = value_to_iovalue(value);
    let fields = simple_record(&value, label, 1)?;
    let items = required_sequence(&fields[0], label)?;
    items.iter().map(|item| required_string(item, label)).collect()
}

fn record_sequence_len(value: &PreservesValue<IoValue>, label: &str) -> Result<usize> {
    let value = value_to_iovalue(value);
    let fields = simple_record(&value, label, 1)?;
    Ok(required_sequence(&fields[0], label)?.len())
}

fn record_u64(value: &PreservesValue<IoValue>, label: &str) -> Result<u64> {
    let value = value_to_iovalue(value);
    let fields = simple_record(&value, label, 1)?;
    required_u64(&fields[0], label)
}

fn parse_ref_sequence_value(value: &PreservesValue<IoValue>, label: &str) -> Result<Vec<String>> {
    let items = required_sequence(value, label)?;
    items.iter().map(|item| required_ref(item, label)).collect()
}

fn required_string(value: &PreservesValue<IoValue>, field: &str) -> Result<String> {
    value
        .as_string()
        .map(|value| value.into_owned())
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected string for {field}")))
}

fn required_ref(value: &PreservesValue<IoValue>, field: &str) -> Result<String> {
    let value = required_string(value, field)?;
    validate_ref(&value, field)?;
    Ok(value)
}

fn required_u64(value: &PreservesValue<IoValue>, field: &str) -> Result<u64> {
    value
        .as_u64()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected u64 for {field}")))?
        .map_err(|error| MoltenError::invalid_harness(format!("u64 out of range for {field}: {error}")))
}

fn validate_filters(filters: &[Filter]) -> Result<()> {
    for filter in filters {
        match filter {
            Filter::Ref(value)
            | Filter::SchemaRef(value)
            | Filter::StructuralFingerprint(value)
            | Filter::EffectRef(value)
            | Filter::PolicyRef(value)
            | Filter::CapabilityRef(value)
            | Filter::EvidenceRef(value)
            | Filter::DependencyRef(value)
            | Filter::DependentRef(value) => validate_ref(value, "catalog filter ref")?,
            Filter::ArtifactKind(value)
            | Filter::LedgerKind(value)
            | Filter::ReceiptOperation(value)
            | Filter::ReceiptDecision(value)
            | Filter::TranscriptStatus(value)
            | Filter::UpgradeStatus(value)
            | Filter::Text(value) => validate_non_empty(value, "catalog filter value")?,
        }
    }
    Ok(())
}

fn validate_visibility(visibility: &VisibilityInput) -> Result<()> {
    validate_refs(&visibility.policy_refs, "catalog visibility policy ref")?;
    validate_refs(&visibility.capability_refs, "catalog visibility capability ref")?;
    validate_refs(&visibility.hidden_refs, "catalog visibility hidden ref")?;
    if let Some(redaction_profile_ref) = visibility.redaction_profile_ref.as_ref() {
        validate_ref(redaction_profile_ref, "catalog redaction profile ref")?;
    }
    Ok(())
}

fn validate_decision(decision: &str) -> Result<()> {
    if matches!(decision, "pass" | "deny") {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("unsupported catalog decision {decision}")))
    }
}

fn validate_ref(value_ref: &str, field: &str) -> Result<()> {
    validate_non_empty(value_ref, field)?;
    validate_content_ref(value_ref).map_err(|error| {
        MoltenError::invalid_harness(format!("{field} must be a canonical content ref, got {value_ref}: {error}"))
    })
}

fn validate_refs(refs: &[String], field: &str) -> Result<()> {
    ensure_count_at_most(refs.len(), MAX_CATALOG_REFS, field)?;
    for value_ref in refs {
        validate_ref(value_ref, field)?;
    }
    Ok(())
}

fn ensure_count_at_most(count: usize, maximum: usize, label: &str) -> Result<()> {
    if count > maximum {
        Err(MoltenError::invalid_harness(format!("{label} count {count} exceeds maximum {maximum}")))
    } else {
        Ok(())
    }
}

fn checked_count_sum(left: usize, right: usize, maximum: usize, label: &str) -> Result<usize> {
    let total = left
        .checked_add(right)
        .ok_or_else(|| MoltenError::invalid_harness(format!("{label} count overflow")))?;
    ensure_count_at_most(total, maximum, label)?;
    Ok(total)
}

fn push_bounded<T>(values: &mut impl crate::bounded::VecSink<T>, value: T, maximum: usize, label: &str) -> Result<()> {
    checked_count_sum(values.item_count(), 1, maximum, label)?;
    values.push_item(value);
    Ok(())
}

fn insert_bounded<T: Ord>(values: &mut Set<T>, value: T, maximum: usize, label: &str) -> Result<bool> {
    if values.contains(&value) {
        return Ok(false);
    }
    checked_count_sum(values.len(), 1, maximum, label)?;
    Ok(values.insert(value))
}

fn validate_non_empty(value: &str, field: &str) -> Result<()> {
    if value.is_empty() {
        Err(MoltenError::invalid_harness(format!("{field} must not be empty")))
    } else {
        Ok(())
    }
}

fn sorted_unique(refs: &[String]) -> Vec<String> {
    refs.iter().cloned().collect::<Set<_>>().into_iter().collect()
}

fn push_optional_classification(
    values: &mut impl crate::bounded::VecSink<String>,
    prefix: &str,
    value: Option<&str>,
) -> Result<()> {
    if let Some(value) = value {
        push_bounded(values, format!("{prefix}:{value}"), MAX_CATALOG_REFS, "catalog classifications")?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/catalog/parts/mod/tests/m000/p000/body.rs"));
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/catalog/parts/mod/tests/m000/p001/body.rs"));
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/catalog/parts/mod/tests/m000/p002/body.rs"));
}

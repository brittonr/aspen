
fn refs_sequence(refs: &[String]) -> IoValue {
    crate::preserves_rail::refs_sequence(refs)
}

fn optional_ref_value(value: Option<&str>) -> IoValue {
    crate::preserves_rail::optional_ref_value(value)
}

fn optional_string_value(value: Option<&str>) -> IoValue {
    value.map_or_else(|| record("none", Vec::new()), |value| record("some", vec![string(value)]))
}

fn parse_optional_ref_value(value: &RailValue) -> Result<Option<String>> {
    crate::preserves_rail::optional_content_ref_string(value, "optional ref")
}

fn parse_optional_string_value(value: &RailValue) -> Result<Option<String>> {
    if value.collect_simple_record("none", Some(0)).is_some() {
        return Ok(None);
    }
    if let Some(some) = value.collect_simple_record("some", Some(1)) {
        return required_string(&some[0], "optional string").map(Some);
    }
    required_string(value, "optional string").map(Some)
}

fn record_string(value: &RailValue, label: &str) -> Result<String> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    required_string(&record[0], label)
}

fn record_ref(value: &RailValue, label: &str) -> Result<String> {
    crate::preserves_rail::record_content_ref_string(value, label, label)
}

fn record_optional_ref(value: &RailValue, label: &str) -> Result<Option<String>> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    parse_optional_ref_value(&record[0])
}

fn record_optional_string(value: &RailValue, label: &str) -> Result<Option<String>> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    parse_optional_string_value(&record[0])
}

fn record_ref_sequence(value: &RailValue, label: &str) -> Result<Vec<String>> {
    crate::preserves_rail::record_content_ref_strings(value, label, label, MAX_ARTIFACT_REF_LIST)
}

fn parse_ref_sequence_value(value: &RailValue, label: &str) -> Result<Vec<String>> {
    let items = required_sequence(value, label)?;
    ensure_count_at_most(items.len(), MAX_ARTIFACT_REF_LIST, label)?;
    let mut refs = Vec::with_capacity(items.len());
    for item in items.iter() {
        push_bounded(&mut refs, required_ref(item, label)?, MAX_ARTIFACT_REF_LIST, label)?;
    }
    Ok(refs)
}

fn checks_value(names: &[&str]) -> IoValue {
    checks_value_from_pairs(&names.iter().map(|name| (*name, "pass")).collect::<Vec<_>>())
}

fn checks_value_from_pairs(checks: &[(&str, &str)]) -> IoValue {
    crate::preserves_rail::checks_value(checks)
}

fn parse_checks(value: &RailValue) -> Result<Vec<String>> {
    let parsed = crate::preserves_rail::parse_checks_record(value, MAX_ARTIFACT_CHECKS, "artifact registry")?;
    let mut names = Vec::with_capacity(parsed.len());
    for check in parsed {
        if check.status != "pass" && check.status != "fail" {
            return Err(MoltenError::invalid_harness(format!(
                "artifact registry check {} has status {}",
                check.name, check.status
            )));
        }
        push_bounded(&mut names, check.name, MAX_ARTIFACT_CHECKS, "artifact checks")?;
    }
    Ok(names)
}

fn require_check(checks: &[String], expected: &str, context: &str) -> Result<()> {
    if checks.iter().any(|check| check == expected) {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("{context} missing {expected} check")))
    }
}

fn require_schema(value: &RailValue, expected: &str, context: &str) -> Result<()> {
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
) -> Result<std::borrow::Cow<'a, preserves::Record<RailValue>>> {
    crate::preserves_rail::simple_record_fields(value, label, arity)
}

#[allow(clippy::owned_cow)]
fn required_sequence<'a>(value: &'a RailValue, field: &str) -> Result<std::borrow::Cow<'a, Vec<RailValue>>> {
    crate::preserves_rail::required_sequence_field(value, field)
}

fn required_string(value: &RailValue, field: &str) -> Result<String> {
    crate::preserves_rail::required_string_field(value, field)
}

fn required_ref(value: &RailValue, field: &str) -> Result<String> {
    crate::preserves_rail::required_content_ref_string(value, field)
}

fn required_u64(value: &RailValue, field: &str) -> Result<u64> {
    value
        .as_u64()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected u64 for {field}")))?
        .map_err(|error| MoltenError::invalid_harness(format!("u64 out of range for {field}: {error}")))
}

fn validate_install_input(input: &ArtifactInstallInput) -> Result<()> {
    validate_kind(&input.kind)?;
    validate_refs(&input.schema_refs, "artifact schema ref")?;
    validate_refs(&input.dependency_refs, "artifact dependency ref")?;
    if let Some(effect_manifest_ref) = input.effect_manifest_ref.as_ref() {
        validate_ref(effect_manifest_ref, "artifact effect manifest ref")?;
    }
    validate_refs(&input.policy_refs, "artifact policy ref")?;
    validate_refs(&input.evidence_refs, "artifact evidence ref")?;
    validate_ref(&input.installer_ref, "artifact installer ref")?;
    if input.capability_refs.is_empty() {
        return Err(MoltenError::invalid_harness("artifact install requires at least one capability ref"));
    }
    validate_refs(&input.capability_refs, "artifact capability ref")
}

fn validate_kind(kind: &str) -> Result<()> {
    validate_non_empty(kind, "artifact kind")?;
    if kind.chars().all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-' || ch == '_') {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!(
            "artifact kind {kind} must use lowercase ascii, digits, '-' or '_'"
        )))
    }
}

fn validate_pointer_kind(kind: &str) -> Result<()> {
    if matches!(kind, "name" | "alias" | "tag" | "channel") {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!(
            "unsupported artifact pointer kind {kind}; expected name, alias, tag, or channel"
        )))
    }
}

fn validate_non_empty(value: &str, field: &str) -> Result<()> {
    if value.is_empty() {
        Err(MoltenError::invalid_harness(format!("{field} cannot be empty")))
    } else {
        Ok(())
    }
}

fn validate_ref(value_ref: &str, field: &str) -> Result<()> {
    validate_non_empty(value_ref, field)?;
    crate::preserves_rail::validate_content_ref(value_ref).map_err(|error| {
        MoltenError::invalid_harness(format!("{field} must be a canonical blake3 content ref: {error}"))
    })
}

fn validate_refs(refs: &[String], field: &str) -> Result<()> {
    ensure_count_at_most(refs.len(), MAX_ARTIFACT_REF_LIST, field)?;
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

fn extend_cloned_bounded<T: Clone>(
    values: &mut impl crate::bounded::VecSink<T>,
    incoming: &[T],
    maximum: usize,
    label: &str,
) -> Result<()> {
    let final_count = checked_count_sum(values.item_count(), incoming.len(), maximum, label)?;
    values.reserve_items(final_count.saturating_sub(values.item_count()));
    values.extend_cloned_items(incoming);
    Ok(())
}

fn index_error(error: impl std::fmt::Display) -> MoltenError {
    MoltenError::invalid_harness(format!("artifact registry redb index error: {error}"))
}

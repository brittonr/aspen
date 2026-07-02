
fn validate_bootstrap_input(input: &BootstrapInput) -> Result<()> {
    validate_non_empty(&input.kind, "bootstrap input kind")?;
    validate_non_empty(&input.peer, "bootstrap input peer")?;
    validate_endpoint(&input.endpoint_id)?;
    require_ref(&input.provenance_ref, "bootstrap input provenance ref")?;
    validate_refs(&input.policy_refs, "bootstrap input policy ref")
}

fn validate_features(features: &FeatureVector) -> Result<()> {
    if features.runtime_versions.is_empty()
        || features.registry_protocols.is_empty()
        || features.schema_identities.is_empty()
        || features.preserves_boundaries.is_empty()
        || features.transports.is_empty()
    {
        return Err(MoltenError::invalid_harness("peer feature vectors must include required feature sets"));
    }
    Ok(())
}

fn validate_offer(offer: &CapabilityOffer) -> Result<()> {
    validate_non_empty(&offer.capability, "capability offer capability")?;
    validate_non_empty(&offer.scope, "capability offer scope")?;
    validate_non_empty(&offer.attenuation, "capability offer attenuation")?;
    validate_refs(&offer.policy_refs, "capability offer policy ref")
}

fn validate_join(join: &JoinRequest) -> Result<()> {
    validate_non_empty(&join.kind, "join kind")?;
    validate_non_empty(&join.target, "join target")?;
    validate_non_empty(&join.required_capability, "join required capability")
}

fn validate_endpoint(endpoint_id: &str) -> Result<()> {
    if endpoint_id.starts_with("iroh:") {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!(
            "expected iroh endpoint id for peer bootstrap, got {endpoint_id}"
        )))
    }
}

fn ensure_count_at_most(actual: usize, maximum: usize, label: &str) -> Result<()> {
    if actual <= maximum {
        return Ok(());
    }
    Err(MoltenError::invalid_harness(format!("{label} count {actual} exceeds bound {maximum}")))
}

fn push_bounded<T>(values: &mut impl crate::bounded::VecSink<T>, value: T, maximum: usize, label: &str) -> Result<()> {
    let total = values
        .item_count()
        .checked_add(1)
        .ok_or_else(|| MoltenError::invalid_harness(format!("{label} count overflow")))?;
    ensure_count_at_most(total, maximum, label)?;
    values.push_item(value);
    Ok(())
}

fn validate_non_empty(value: &str, field: &str) -> Result<()> {
    if value.trim().is_empty() {
        Err(MoltenError::invalid_harness(format!("{field} must not be empty")))
    } else {
        Ok(())
    }
}

fn validate_refs(refs: &[String], field: &str) -> Result<()> {
    for reference in refs {
        require_ref(reference, field)?;
    }
    Ok(())
}

fn require_ref(reference: &str, field: &str) -> Result<()> {
    validate_content_ref(reference).map_err(|error| {
        MoltenError::invalid_harness(format!("expected canonical content ref for {field}, got {reference}: {error}"))
    })
}

fn optional_ref_value(value: Option<&str>) -> IoValue {
    value.map_or_else(|| record("none", Vec::new()), |value| record("some", vec![string(value)]))
}

fn optional_u64_value(value: Option<u64>) -> IoValue {
    value.map_or_else(|| record("none", Vec::new()), |value| record("some", vec![u64_value(value)]))
}

fn parse_optional_u64(value: &Value<IoValue>, label: &str) -> Result<Option<u64>> {
    let value = value_to_iovalue(value);
    let fields = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...>")))?;
    let optional = value_to_iovalue(&fields[0]);
    if optional.collect_simple_record("none", Some(0)).is_some() {
        Ok(None)
    } else if let Some(some) = optional.collect_simple_record("some", Some(1)) {
        required_u64(&some[0], label).map(Some)
    } else {
        Err(MoltenError::invalid_harness(format!("expected optional u64 for {label}")))
    }
}

fn parse_ref_sequence(value: &Value<IoValue>, label: &str) -> Result<Vec<String>> {
    let values = field_sequence(value, label)?;
    values
        .iter()
        .map(|value| {
            let reference = required_string(value, label)?;
            require_ref(&reference, label)?;
            Ok(reference)
        })
        .collect()
}

fn parse_string_sequence(value: &Value<IoValue>, label: &str) -> Result<Vec<String>> {
    let values = field_sequence(value, label)?;
    values.iter().map(|value| required_string(value, label)).collect()
}

fn field_sequence(value: &Value<IoValue>, label: &str) -> Result<Vec<Value<IoValue>>> {
    let value = value_to_iovalue(value);
    let fields = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...>")))?;
    let values = fields[0]
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected sequence for {label}")))?;
    Ok(values.iter().cloned().collect())
}

fn parse_checks(value: &Value<IoValue>) -> Result<Vec<(String, String)>> {
    let values = field_sequence(value, "checks")?;
    values
        .iter()
        .map(|check| {
            let check = value_to_iovalue(check);
            let fields = check
                .collect_simple_record("check", Some(2))
                .ok_or_else(|| MoltenError::invalid_harness("expected peer bootstrap check"))?;
            Ok((required_string(&fields[0], "check name")?, required_string(&fields[1], "check status")?))
        })
        .collect()
}

fn require_check(checks: &[(String, String)], name: &str) -> Result<()> {
    if checks.iter().any(|(check, status)| check == name && status == "pass") {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("peer bootstrap evidence missing passing {name} check")))
    }
}

fn record_string(value: &Value<IoValue>, label: &str) -> Result<String> {
    let value = value_to_iovalue(value);
    let fields = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...>")))?;
    required_string(&fields[0], label)
}

fn record_u64(value: &Value<IoValue>, label: &str) -> Result<u64> {
    let value = value_to_iovalue(value);
    let fields = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...>")))?;
    required_u64(&fields[0], label)
}

fn require_schema(value: &Value<IoValue>, expected: &str, field: &str) -> Result<()> {
    let actual = required_string(value, field)?;
    if actual != expected {
        return Err(MoltenError::invalid_harness(format!("expected {field} {expected}, got {actual}")));
    }
    Ok(())
}

fn required_string(value: &Value<IoValue>, field: &str) -> Result<String> {
    value
        .as_string()
        .map(|value| value.into_owned())
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected string for {field}")))
}

fn required_u64(value: &Value<IoValue>, field: &str) -> Result<u64> {
    value
        .as_u64()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected u64 for {field}")))?
        .map_err(|error| MoltenError::invalid_harness(format!("u64 out of range for {field}: {error}")))
}

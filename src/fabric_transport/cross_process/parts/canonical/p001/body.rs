
fn parse_locators(value: &preserves::Value<preserves::IOValue>) -> crate::error::Result<Vec<EndpointLocator>> {
    let values = value.collect_sequence().ok_or_else(|| {
        crate::error::MoltenError::invalid_harness("cross-process endpoint locators must be a sequence")
    })?;
    if values.len() > MAX_CANONICAL_LOCATORS {
        return Err(crate::error::MoltenError::invalid_harness("cross-process endpoint locator count exceeds bound"));
    }
    values
        .iter()
        .map(|value| {
            let value = crate::preserves_rail::value_to_iovalue(&value);
            let fields = simple_record(&value, LOCATOR_RECORD, LOCATOR_FIELD_COUNT)?;
            Ok(EndpointLocator {
                class: parse_locator_class(&required_string(&fields[0], "locator class")?)?,
                value: required_string(&fields[1], "locator value")?,
            })
        })
        .collect()
}

fn parse_locator_classes(
    value: &preserves::Value<preserves::IOValue>,
) -> crate::error::Result<Vec<EndpointLocatorClass>> {
    let values = value
        .collect_sequence()
        .ok_or_else(|| crate::error::MoltenError::invalid_harness("endpoint disclosure classes must be a sequence"))?;
    if values.len() > MAX_CANONICAL_LOCATORS {
        return Err(crate::error::MoltenError::invalid_harness("endpoint disclosure class count exceeds bound"));
    }
    values
        .iter()
        .map(|value| parse_locator_class(&required_string(&value, "endpoint disclosure class")?))
        .collect()
}

fn parse_locator_class(value: &str) -> crate::error::Result<EndpointLocatorClass> {
    match value {
        "ip" => Ok(EndpointLocatorClass::Ip),
        "relay" => Ok(EndpointLocatorClass::Relay),
        "custom" => Ok(EndpointLocatorClass::Custom),
        "private" => Ok(EndpointLocatorClass::Private),
        other => Err(crate::error::MoltenError::invalid_harness(format!("unsupported endpoint locator class {other}"))),
    }
}

fn parse_resources(value: &preserves::Value<preserves::IOValue>) -> crate::error::Result<EndpointResourceBounds> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let fields = simple_record(&value, RESOURCES_RECORD, RESOURCE_FIELD_COUNT)?;
    Ok(EndpointResourceBounds {
        max_sessions: required_u64(&fields[0], "max sessions")?,
        max_frame_bytes: required_u64(&fields[1], "max frame bytes")?,
        max_queued_bytes: required_u64(&fields[RESOURCE_QUEUED_INDEX], "max queued bytes")?,
        max_inflight_bytes: required_u64(&fields[RESOURCE_INFLIGHT_INDEX], "max inflight bytes")?,
    })
}

fn parse_validity(value: &preserves::Value<preserves::IOValue>) -> crate::error::Result<EndpointValidityCohort> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let fields = simple_record(&value, VALIDITY_RECORD, VALIDITY_FIELD_COUNT)?;
    Ok(EndpointValidityCohort {
        cohort_ref: required_ref(&fields[0], "validity cohort ref")?,
        not_before_tick: required_u64(&fields[1], "valid from tick")?,
        expires_at_tick: required_u64(&fields[VALIDITY_EXPIRY_INDEX], "valid until tick")?,
    })
}

fn parse_non_claims(value: &preserves::Value<preserves::IOValue>) -> crate::error::Result<Vec<TransportNonClaim>> {
    let values = value
        .collect_sequence()
        .ok_or_else(|| crate::error::MoltenError::invalid_harness("endpoint non-claims must be a sequence"))?;
    values
        .iter()
        .map(|value| parse_non_claim(&required_string(&value, "endpoint non-claim")?))
        .collect()
}

fn parse_non_claim(value: &str) -> crate::error::Result<TransportNonClaim> {
    REQUIRED_TRANSPORT_NON_CLAIMS
        .into_iter()
        .find(|claim| claim.as_str() == value)
        .ok_or_else(|| crate::error::MoltenError::invalid_harness(format!("unsupported endpoint non-claim {value}")))
}

fn field(name: &str, value: preserves::IOValue) -> preserves::IOValue {
    crate::preserves_rail::record("field", vec![crate::preserves_rail::string(name), value])
}

fn strings_value<'a>(values: impl Iterator<Item = &'a str>) -> preserves::IOValue {
    crate::preserves_rail::sequence(values.map(crate::preserves_rail::string).collect())
}

fn checks(values: &[&str]) -> preserves::IOValue {
    crate::preserves_rail::record(CHECKS_RECORD, vec![strings_value(values.iter().copied())])
}

fn simple_record(
    value: &preserves::IOValue,
    label: &str,
    field_count: usize,
) -> crate::error::Result<Vec<preserves::Value<preserves::IOValue>>> {
    let fields = value
        .collect_simple_record(label, Some(field_count))
        .ok_or_else(|| crate::error::MoltenError::invalid_harness(format!("expected <{label} ...>")))?;
    Ok(fields.iter().collect())
}

fn next_field<'a, 'b>(
    fields: &mut impl Iterator<Item = &'a preserves::Value<preserves::IOValue>>,
    label: &'b str,
) -> crate::error::Result<&'a preserves::Value<preserves::IOValue>> {
    fields
        .next()
        .ok_or_else(|| crate::error::MoltenError::invalid_harness(format!("cross-process endpoint missing {label}")))
}

fn required_string(value: &preserves::Value<preserves::IOValue>, label: &str) -> crate::error::Result<String> {
    value
        .as_string()
        .map(|value| value.into_owned())
        .ok_or_else(|| crate::error::MoltenError::invalid_harness(format!("expected string for {label}")))
}

fn required_ref(value: &preserves::Value<preserves::IOValue>, label: &str) -> crate::error::Result<String> {
    let value = required_string(value, label)?;
    crate::preserves_rail::validate_content_ref(&value)?;
    Ok(value)
}

fn required_u64(value: &preserves::Value<preserves::IOValue>, label: &str) -> crate::error::Result<u64> {
    value
        .as_u64()
        .ok_or_else(|| crate::error::MoltenError::invalid_harness(format!("expected u64 for {label}")))?
        .map_err(|error| crate::error::MoltenError::invalid_harness(format!("u64 out of range for {label}: {error}")))
}

fn required_bool(value: &preserves::Value<preserves::IOValue>, label: &str) -> crate::error::Result<bool> {
    value
        .as_boolean()
        .ok_or_else(|| crate::error::MoltenError::invalid_harness(format!("expected bool for {label}")))
}

fn validation_error(label: &str, issues: &impl std::fmt::Debug) -> crate::error::MoltenError {
    crate::error::MoltenError::invalid_harness(format!("{label} denied: {issues:?}"))
}

use preserves::ValueImpl;

pub(super) fn sequence_strings(
    value: &preserves::Value<preserves::IOValue>,
    field: &str,
) -> crate::error::Result<Vec<String>> {
    let values = required_sequence(value, field)?;
    values.iter().map(|value| required_string(&value, field)).collect()
}

pub(super) fn required_record_iovalue(
    value: &preserves::Value<preserves::IOValue>,
    label: &str,
    _field: &str,
) -> crate::error::Result<preserves::IOValue> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    Ok(crate::preserves_rail::value_to_iovalue(&record[0]))
}

pub(super) fn required_record_string(
    value: &preserves::Value<preserves::IOValue>,
    label: &str,
    field: &str,
) -> crate::error::Result<String> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    required_string(&record[0], field)
}

pub(super) fn required_record_hash(
    value: &preserves::Value<preserves::IOValue>,
    label: &str,
    field: &str,
) -> crate::error::Result<String> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    required_hash(&record[0], field)
}

pub(super) fn required_record_bool(
    value: &preserves::Value<preserves::IOValue>,
    label: &str,
    field: &str,
) -> crate::error::Result<bool> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    required_bool(&record[0], field)
}

pub(super) fn required_record_u64(
    value: &preserves::Value<preserves::IOValue>,
    label: &str,
    field: &str,
) -> crate::error::Result<u64> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    required_u64(&record[0], field)
}

pub(super) fn required_record_sequence(
    value: &preserves::Value<preserves::IOValue>,
    label: &str,
    field: &str,
) -> crate::error::Result<Vec<preserves::Value<preserves::IOValue>>> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    let owned = crate::preserves_rail::value_to_iovalue(&record[0]);
    Ok(owned
        .collect_sequence()
        .ok_or_else(|| crate::error::Failure::invalid_harness(format!("expected sequence for {field}")))?
        .into_owned())
}

pub(super) fn required_record_hash_sequence(
    value: &preserves::Value<preserves::IOValue>,
    label: &str,
    field: &str,
) -> crate::error::Result<Vec<String>> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    let values = required_sequence(&record[0], field)?;
    values.iter().map(|value| required_hash(&value, field)).collect()
}

pub(super) fn required_record_string_sequence(
    value: &preserves::Value<preserves::IOValue>,
    label: &str,
    field: &str,
) -> crate::error::Result<Vec<String>> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    let values = required_sequence(&record[0], field)?;
    values.iter().map(|value| required_string(&value, field)).collect()
}

pub(super) fn required_record_iovalue_sequence(
    value: &preserves::Value<preserves::IOValue>,
    label: &str,
    field: &str,
) -> crate::error::Result<Vec<preserves::IOValue>> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    let values = required_sequence(&record[0], field)?;
    Ok(values.iter().map(|value| crate::preserves_rail::value_to_iovalue(&value)).collect())
}

pub(super) fn simple_record<'a>(
    value: &'a preserves::IOValue,
    label: &str,
    arity: usize,
) -> crate::error::Result<std::borrow::Cow<'a, preserves::Record<preserves::Value<preserves::IOValue>>>> {
    value
        .collect_simple_record(label, Some(arity))
        .ok_or_else(|| crate::error::Failure::invalid_harness(format!("expected <{label} ...> with arity {arity}")))
}

pub(super) fn value_has_record_label(value: &preserves::Value<preserves::IOValue>, label: &str) -> bool {
    value.collect_simple_record(label, None).is_some()
}

#[allow(clippy::owned_cow)]
pub(super) fn required_sequence<'a>(
    value: &'a preserves::Value<preserves::IOValue>,
    field: &str,
) -> crate::error::Result<std::borrow::Cow<'a, Vec<preserves::Value<preserves::IOValue>>>> {
    value
        .collect_sequence()
        .ok_or_else(|| crate::error::Failure::invalid_harness(format!("expected sequence for {field}")))
}

pub(super) fn required_string(
    value: &preserves::Value<preserves::IOValue>,
    field: &str,
) -> crate::error::Result<String> {
    value
        .as_string()
        .map(|value| value.into_owned())
        .ok_or_else(|| crate::error::Failure::invalid_harness(format!("expected string for {field}")))
}

fn required_bool(value: &preserves::Value<preserves::IOValue>, field: &str) -> crate::error::Result<bool> {
    value
        .as_boolean()
        .ok_or_else(|| crate::error::Failure::invalid_harness(format!("expected boolean for {field}")))
}

pub(super) fn optional_string(
    value: &preserves::Value<preserves::IOValue>,
    field: &str,
) -> crate::error::Result<Option<String>> {
    if value.as_boolean() == Some(false) {
        Ok(None)
    } else {
        required_string(value, field).map(Some)
    }
}

pub(super) fn optional_request_string(
    value: &preserves::Value<preserves::IOValue>,
    field: &str,
) -> crate::error::Result<Option<String>> {
    if value.as_boolean() == Some(false) || value.collect_simple_record("none", Some(0)).is_some() {
        return Ok(None);
    }
    if let Some(some) = value.collect_simple_record("some", Some(1)) {
        return required_string(&some[0], field).map(Some);
    }
    // Compatibility with early reports that encoded present optional strings directly.
    required_string(value, field).map(Some)
}

pub(super) fn optional_request_runtime_value(
    value: &preserves::Value<preserves::IOValue>,
    _field: &str,
) -> crate::error::Result<Option<crate::harness::nucleus::RuntimeValue>> {
    if value.as_boolean() == Some(false) || value.collect_simple_record("none", Some(0)).is_some() {
        return Ok(None);
    }
    if let Some(some) = value.collect_simple_record("some", Some(1)) {
        return crate::harness::nucleus::RuntimeValue::new(crate::preserves_rail::value_to_iovalue(&some[0])).map(Some);
    }
    // Compatibility with early reports that encoded present optional values directly.
    crate::harness::nucleus::RuntimeValue::new(crate::preserves_rail::value_to_iovalue(value)).map(Some)
}

pub(super) fn optional_request_u64(
    value: &preserves::Value<preserves::IOValue>,
    field: &str,
) -> crate::error::Result<Option<u64>> {
    if value.as_boolean() == Some(false) || value.collect_simple_record("none", Some(0)).is_some() {
        return Ok(None);
    }
    if let Some(some) = value.collect_simple_record("some", Some(1)) {
        return required_u64(&some[0], field).map(Some);
    }
    // Compatibility with early reports that encoded present optional integers directly.
    required_u64(value, field).map(Some)
}

pub(super) fn optional_action(
    value: &preserves::Value<preserves::IOValue>,
    field: &str,
) -> crate::error::Result<Option<crate::runtime::AdmissionAction>> {
    if value.as_boolean() == Some(false) {
        Ok(None)
    } else {
        super::roster::parse_admission_action(&required_string(value, field)?).map(Some)
    }
}

pub(super) fn optional_runtime_match_value(
    value: &preserves::Value<preserves::IOValue>,
) -> crate::error::Result<Option<crate::harness::nucleus::RuntimeValue>> {
    if value.as_boolean() == Some(false) {
        Ok(None)
    } else {
        crate::harness::nucleus::RuntimeValue::new(crate::preserves_rail::value_to_iovalue(value)).map(Some)
    }
}

pub(super) fn required_hash(value: &preserves::Value<preserves::IOValue>, field: &str) -> crate::error::Result<String> {
    let hash = required_string(value, field)?;
    crate::preserves_rail::validate_content_ref(&hash).map_err(|error| {
        crate::error::Failure::invalid_harness(format!(
            "expected canonical content ref for {field}, got {hash}: {error}"
        ))
    })?;
    Ok(hash)
}

pub(super) fn required_runtime_value(
    value: &preserves::Value<preserves::IOValue>,
    _field: &str,
) -> crate::error::Result<crate::harness::nucleus::RuntimeValue> {
    crate::harness::nucleus::RuntimeValue::new(crate::preserves_rail::value_to_iovalue(value))
}

pub(super) fn required_u64(value: &preserves::Value<preserves::IOValue>, field: &str) -> crate::error::Result<u64> {
    value
        .as_u64()
        .ok_or_else(|| crate::error::Failure::invalid_harness(format!("expected u64 for {field}")))?
        .map_err(|error| crate::error::Failure::invalid_harness(format!("u64 out of range for {field}: {error}")))
}

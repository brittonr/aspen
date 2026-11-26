//! Typed field readers over Preserves records.

pub(in crate::blocks) fn record_optional_ref(
    value: &preserves::Value<preserves::IOValue>,
    label: &str,
) -> crate::error::Result<Option<String>> {
    let record = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| crate::error::Failure::invalid_harness(format!("expected <{label} ...> field")))?;
    let optional = crate::preserves_rail::value_to_iovalue(&record[0]);
    if optional.collect_simple_record("none", Some(0)).is_some() {
        Ok(None)
    } else if let Some(some) = optional.collect_simple_record("some", Some(1)) {
        required_string(&some[0], label).map(Some)
    } else {
        Err(crate::error::Failure::invalid_harness(format!("expected <none> or <some ref> for {label}")))
    }
}

pub(in crate::blocks) fn simple_record<'a>(
    value: &'a preserves::IOValue,
    label: &str,
    arity: usize,
) -> crate::error::Result<std::borrow::Cow<'a, preserves::Record<preserves::Value<preserves::IOValue>>>> {
    value
        .collect_simple_record(label, Some(arity))
        .ok_or_else(|| crate::error::Failure::invalid_harness(format!("expected <{label} ...> with arity {arity}")))
}

pub(super) fn simple_record_any<'a>(
    value: &'a preserves::IOValue,
    label: &str,
) -> crate::error::Result<std::borrow::Cow<'a, preserves::Record<preserves::Value<preserves::IOValue>>>> {
    value
        .collect_simple_record(label, None)
        .ok_or_else(|| crate::error::Failure::invalid_harness(format!("expected <{label} ...> record")))
}

pub(super) fn record_arity(record: &preserves::Record<preserves::Value<preserves::IOValue>>) -> usize {
    record._vec().len().saturating_sub(1)
}

pub(in crate::blocks) fn record_string(
    value: &preserves::Value<preserves::IOValue>,
    label: &str,
) -> crate::error::Result<String> {
    let record = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| crate::error::Failure::invalid_harness(format!("expected <{label} ...> field")))?;
    required_string(&record[0], label)
}

pub(super) fn record_u64(value: &preserves::Value<preserves::IOValue>, label: &str) -> crate::error::Result<u64> {
    let record = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| crate::error::Failure::invalid_harness(format!("expected <{label} ...> field")))?;
    required_u64(&record[0], label)
}

pub(in crate::blocks) fn record_sequence(
    value: &preserves::Value<preserves::IOValue>,
    label: &str,
) -> crate::error::Result<Vec<preserves::IOValue>> {
    let record = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| crate::error::Failure::invalid_harness(format!("expected <{label} ...> field")))?;
    let sequence = record[0]
        .collect_sequence()
        .ok_or_else(|| crate::error::Failure::invalid_harness(format!("expected sequence for {label}")))?;
    Ok(sequence.iter().map(crate::preserves_rail::value_to_iovalue).collect())
}

pub(in crate::blocks) fn record_string_sequence(
    value: &preserves::Value<preserves::IOValue>,
    label: &str,
) -> crate::error::Result<Vec<String>> {
    record_sequence(value, label)?.iter().map(|value| required_string(value, label)).collect()
}

pub(super) fn record_optional_string(
    value: &preserves::Value<preserves::IOValue>,
    label: &str,
) -> crate::error::Result<Option<String>> {
    let record = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| crate::error::Failure::invalid_harness(format!("expected <{label} ...> field")))?;
    if let Some(value) = record[0].as_string() {
        Ok(Some(value.into_owned()))
    } else if record[0].collect_simple_record("none", Some(0)).is_some() {
        Ok(None)
    } else {
        Err(crate::error::Failure::invalid_harness(format!("expected string or <none> for {label}")))
    }
}

pub(in crate::blocks) fn require_schema(
    value: &preserves::Value<preserves::IOValue>,
    expected: &str,
    field: &str,
) -> crate::error::Result<()> {
    let actual = required_string(value, field)?;
    if actual != expected {
        return Err(crate::error::Failure::invalid_harness(format!(
            "expected {field} schema {expected}, got {actual}"
        )));
    }
    Ok(())
}

pub(in crate::blocks) fn required_string(
    value: &preserves::Value<preserves::IOValue>,
    field: &str,
) -> crate::error::Result<String> {
    value
        .as_string()
        .map(|value| value.into_owned())
        .ok_or_else(|| crate::error::Failure::invalid_harness(format!("expected string for {field}")))
}

pub(in crate::blocks) fn required_u64(
    value: &preserves::Value<preserves::IOValue>,
    field: &str,
) -> crate::error::Result<u64> {
    value
        .as_u64()
        .ok_or_else(|| crate::error::Failure::invalid_harness(format!("expected u64 for {field}")))?
        .map_err(|error| crate::error::Failure::invalid_harness(format!("u64 out of range for {field}: {error}")))
}

//! Preserves record field readers.

pub(super) fn optional_string(value: Option<&str>) -> preserves::IOValue {
    match value {
        Some(value) => crate::preserves_rail::record("some", vec![crate::preserves_rail::string(value)]),
        None => crate::preserves_rail::record("none", Vec::new()),
    }
}

pub(super) fn optional_value(value: Option<&preserves::IOValue>) -> preserves::IOValue {
    match value {
        Some(value) => crate::preserves_rail::record("some", vec![value.clone()]),
        None => crate::preserves_rail::record("none", Vec::new()),
    }
}

pub(super) fn diagnostics_include(diagnostics: &[String], needle: &str) -> bool {
    diagnostics.iter().any(|diagnostic| diagnostic.contains(needle))
}

pub(super) fn record_strings(
    value: &preserves::Value<preserves::IOValue>,
    tag: &str,
) -> crate::error::Result<Vec<String>> {
    let record_value = crate::preserves_rail::value_to_iovalue(value);
    let fields = record_value
        .collect_simple_record(tag, Some(1))
        .ok_or_else(|| crate::error::Failure::invalid_harness(format!("expected <{tag} [...]>")))?;
    let items = fields[0]
        .collect_sequence()
        .ok_or_else(|| crate::error::Failure::invalid_harness(format!("{tag} must contain a sequence")))?
        .into_owned();
    let mut values = Vec::with_capacity(items.len());
    for item in items {
        let item = item
            .as_string()
            .map(|value| value.into_owned())
            .ok_or_else(|| crate::error::Failure::invalid_harness(format!("{tag} sequence contains non-string")))?;
        values.push(item);
    }
    Ok(values)
}

pub(super) fn record_optional_string(
    value: &preserves::Value<preserves::IOValue>,
    tag: &str,
) -> crate::error::Result<Option<String>> {
    let record_value = crate::preserves_rail::value_to_iovalue(value);
    let fields = record_value
        .collect_simple_record(tag, Some(1))
        .ok_or_else(|| crate::error::Failure::invalid_harness(format!("expected <{tag} optional>")))?;
    let inner = crate::preserves_rail::value_to_iovalue(&fields[0]);
    if inner.collect_simple_record("none", Some(0)).is_some() {
        return Ok(None);
    }
    let some = inner
        .collect_simple_record("some", Some(1))
        .ok_or_else(|| crate::error::Failure::invalid_harness(format!("{tag} must contain <some string> or <none>")))?;
    let value = some[0]
        .as_string()
        .map(|value| value.into_owned())
        .ok_or_else(|| crate::error::Failure::invalid_harness(format!("{tag} <some> must contain a string")))?;
    Ok(Some(value))
}

pub(super) fn record_optional_value(
    value: &preserves::Value<preserves::IOValue>,
    tag: &str,
) -> crate::error::Result<Option<preserves::IOValue>> {
    let record_value = crate::preserves_rail::value_to_iovalue(value);
    let fields = record_value
        .collect_simple_record(tag, Some(1))
        .ok_or_else(|| crate::error::Failure::invalid_harness(format!("expected <{tag} optional>")))?;
    let inner = crate::preserves_rail::value_to_iovalue(&fields[0]);
    if inner.collect_simple_record("none", Some(0)).is_some() {
        return Ok(None);
    }
    let some = inner
        .collect_simple_record("some", Some(1))
        .ok_or_else(|| crate::error::Failure::invalid_harness(format!("{tag} must contain <some value> or <none>")))?;
    Ok(Some(crate::preserves_rail::value_to_iovalue(&some[0])))
}

pub(super) fn record_optional_ref_string(
    value: &preserves::Value<preserves::IOValue>,
    tag: &str,
) -> crate::error::Result<Option<String>> {
    let reference = record_optional_string(value, tag)?;
    if let Some(reference) = reference.as_ref() {
        super::guards::validate_ingress_ref(reference, tag)?;
    }
    Ok(reference)
}

pub(super) fn record_optional_u64_string(
    value: &preserves::Value<preserves::IOValue>,
    tag: &str,
) -> crate::error::Result<Option<u64>> {
    match record_optional_string(value, tag)? {
        Some(value) => value.parse::<u64>().map(Some).map_err(|_| {
            crate::error::Failure::invalid_harness(format!(
                "{tag} optional value must contain an unsigned integer string"
            ))
        }),
        None => Ok(None),
    }
}

pub(super) fn record_string(value: &preserves::Value<preserves::IOValue>, tag: &str) -> crate::error::Result<String> {
    let record_value = crate::preserves_rail::value_to_iovalue(value);
    let fields = record_value
        .collect_simple_record(tag, Some(1))
        .ok_or_else(|| crate::error::Failure::invalid_harness(format!("expected <{tag} string>")))?;
    fields[0]
        .as_string()
        .map(|value| value.into_owned())
        .ok_or_else(|| crate::error::Failure::invalid_harness(format!("{tag} must contain a string")))
}

pub(super) fn record_sequence_len(
    value: &preserves::Value<preserves::IOValue>,
    tag: &str,
) -> crate::error::Result<usize> {
    let record_value = crate::preserves_rail::value_to_iovalue(value);
    let fields = record_value
        .collect_simple_record(tag, Some(1))
        .ok_or_else(|| crate::error::Failure::invalid_harness(format!("expected <{tag} sequence>")))?;
    fields[0]
        .collect_sequence()
        .map(|items| items.len())
        .ok_or_else(|| crate::error::Failure::invalid_harness(format!("{tag} must contain a sequence")))
}

pub(super) fn record_value(
    value: &preserves::Value<preserves::IOValue>,
    tag: &str,
) -> crate::error::Result<preserves::IOValue> {
    let record_value = crate::preserves_rail::value_to_iovalue(value);
    let fields = record_value
        .collect_simple_record(tag, Some(1))
        .ok_or_else(|| crate::error::Failure::invalid_harness(format!("expected <{tag} value>")))?;
    Ok(crate::preserves_rail::value_to_iovalue(&fields[0]))
}

pub(super) fn record_values(
    value: &preserves::Value<preserves::IOValue>,
    tag: &str,
) -> crate::error::Result<Vec<preserves::IOValue>> {
    let record_value = crate::preserves_rail::value_to_iovalue(value);
    let fields = record_value
        .collect_simple_record(tag, Some(1))
        .ok_or_else(|| crate::error::Failure::invalid_harness(format!("expected <{tag} values>")))?;
    let items = fields[0]
        .collect_sequence()
        .ok_or_else(|| crate::error::Failure::invalid_harness(format!("{tag} must contain a sequence")))?;
    Ok(items.iter().map(crate::preserves_rail::value_to_iovalue).collect())
}

pub(super) fn record_ref_string(
    value: &preserves::Value<preserves::IOValue>,
    tag: &str,
) -> crate::error::Result<String> {
    let reference = record_string(value, tag)?;
    super::guards::validate_ingress_ref(&reference, tag)?;
    Ok(reference)
}

pub(super) fn record_ref_strings(
    value: &preserves::Value<preserves::IOValue>,
    tag: &str,
) -> crate::error::Result<Vec<String>> {
    let record_value = crate::preserves_rail::value_to_iovalue(value);
    let fields = record_value
        .collect_simple_record(tag, Some(1))
        .ok_or_else(|| crate::error::Failure::invalid_harness(format!("expected <{tag} sequence>")))?;
    let items = fields[0]
        .collect_sequence()
        .ok_or_else(|| crate::error::Failure::invalid_harness(format!("{tag} must contain a sequence")))?;
    let mut refs = Vec::with_capacity(items.len());
    for item in items.iter() {
        let reference = item
            .as_string()
            .map(|value| value.into_owned())
            .ok_or_else(|| crate::error::Failure::invalid_harness(format!("{tag} entries must be strings")))?;
        super::guards::validate_ingress_ref(&reference, tag)?;
        refs.push(reference);
    }
    Ok(refs)
}

pub(super) fn record_u64_string(value: &preserves::Value<preserves::IOValue>, tag: &str) -> crate::error::Result<u64> {
    record_string(value, tag)?.parse::<u64>().map_err(|error| {
        crate::error::Failure::invalid_harness(format!("{tag} must contain an unsigned integer string: {error}"))
    })
}

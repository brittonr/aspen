pub(super) fn invalid(message: impl Into<String>) -> crate::error::Failure {
    crate::error::Failure::invalid_harness(message)
}

pub(super) fn fields(
    value: &preserves::Value<preserves::IOValue>,
    label: &str,
    arity: usize,
) -> crate::error::Result<Vec<preserves::Value<preserves::IOValue>>> {
    preserves::ValueImpl::collect_simple_record(value, label, Some(arity))
        .map(|fields| fields.fields_iter().cloned().collect())
        .ok_or_else(|| invalid(format!("expected {label} record")))
}

pub(super) fn text(value: &preserves::Value<preserves::IOValue>) -> crate::error::Result<String> {
    preserves::ValueImpl::as_string(value)
        .map(|text| text.into_owned())
        .ok_or_else(|| invalid("expected witness string"))
}

pub(super) fn number(value: &preserves::Value<preserves::IOValue>) -> crate::error::Result<u64> {
    preserves::ValueImpl::as_u64(value)
        .ok_or_else(|| invalid("expected witness integer"))?
        .map_err(|_| invalid("witness integer outside u64"))
}

pub(super) fn boolean(value: &preserves::Value<preserves::IOValue>) -> crate::error::Result<bool> {
    preserves::ValueImpl::as_boolean(value).ok_or_else(|| invalid("expected witness boolean"))
}

pub(super) fn texts(value: &preserves::Value<preserves::IOValue>) -> crate::error::Result<Vec<String>> {
    preserves::ValueImpl::as_sequence(value)
        .ok_or_else(|| invalid("expected witness sequence"))?
        .map(|item| text(&item))
        .collect()
}

pub(super) fn optional_text(value: Option<&str>) -> preserves::IOValue {
    match value {
        Some(value) => crate::preserves_rail::record("some", vec![crate::preserves_rail::string(value)]),
        None => crate::preserves_rail::record("none", vec![]),
    }
}

pub(super) fn parse_optional_text(
    value: &preserves::Value<preserves::IOValue>,
) -> crate::error::Result<Option<String>> {
    if preserves::ValueImpl::collect_simple_record(value, "none", Some(0)).is_some() {
        return Ok(None);
    }
    Ok(Some(text(&fields(value, "some", 1)?[0])?))
}

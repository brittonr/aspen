
fn record_bool_field(value: &preserves::Value<preserves::IOValue>, label: &str) -> crate::error::Result<bool> {
    named_field_value(value, label)?
        .as_boolean()
        .ok_or_else(|| crate::error::MoltenError::invalid_harness(format!("expected bool for {label}")))
}

fn named_field_value(
    value: &preserves::Value<preserves::IOValue>,
    label: &str,
) -> crate::error::Result<preserves::Value<preserves::IOValue>> {
    const NAMED_FIELD_ARITY: usize = 2;
    let fields = value
        .collect_simple_record("field", Some(NAMED_FIELD_ARITY))
        .ok_or_else(|| crate::error::MoltenError::invalid_harness(format!("expected named field {label}")))?;
    let actual = required_string(&fields[0], "field-name")?;
    if actual != label {
        return Err(crate::error::MoltenError::invalid_harness(format!("expected field {label}, found {actual}")));
    }
    Ok(fields[1].clone())
}

fn required_string(value: &preserves::Value<preserves::IOValue>, label: &str) -> crate::error::Result<String> {
    value
        .as_string()
        .map(|value| value.into_owned())
        .ok_or_else(|| crate::error::MoltenError::invalid_harness(format!("expected string for {label}")))
}

fn validation_error<T: std::fmt::Debug>(label: &str, issues: &[T]) -> crate::error::MoltenError {
    crate::error::MoltenError::invalid_harness(format!("{label} validation failed: {issues:?}"))
}


fn required_record_ref(value: &Value<IoValue>, label: &str, context: &str) -> Result<String> {
    let reference = required_record_string(value, label, context)?;
    crate::preserves_rail::validate_content_ref(&reference)?;
    Ok(reference)
}

fn required_record_string(value: &Value<IoValue>, label: &str, context: &str) -> Result<String> {
    let record = simple_field_record(value, label, context)?;
    required_string(&record[0], context)
}

fn required_record_u64(value: &Value<IoValue>, label: &str, context: &str) -> Result<u64> {
    let record = simple_field_record(value, label, context)?;
    record[0]
        .as_u64()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected u64 for {context}")))?
        .map_err(|error| MoltenError::invalid_harness(format!("u64 out of range for {context}: {error}")))
}

fn require_schema(value: &Value<IoValue>, expected: &str, context: &str) -> Result<()> {
    let actual = required_string(value, context)?;
    if actual == expected {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("unexpected {context} schema {actual}; expected {expected}")))
    }
}

fn simple_record<'a>(
    value: &'a IoValue,
    label: &str,
    arity: usize,
) -> Result<std::borrow::Cow<'a, Record<Value<IoValue>>>> {
    value
        .collect_simple_record(label, Some(arity))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...> with arity {arity}")))
}

fn simple_field_record<'a>(
    value: &'a Value<IoValue>,
    label: &str,
    context: &str,
) -> Result<std::borrow::Cow<'a, Record<Value<IoValue>>>> {
    value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...> for {context}")))
}

fn required_string(value: &Value<IoValue>, context: &str) -> Result<String> {
    value
        .as_string()
        .map(|value| value.into_owned())
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected string for {context}")))
}

fn validate_ref_list(label: &str, refs: &[String]) -> Result<()> {
    if refs.len() > MAX_VM_VALIDATION_ITEMS {
        return Err(MoltenError::invalid_harness(format!(
            "VM {label} ref count {} exceeds bound {MAX_VM_VALIDATION_ITEMS}",
            refs.len()
        )));
    }
    for reference in refs {
        crate::preserves_rail::validate_content_ref(reference)?;
    }
    Ok(())
}

fn validate_strings(label: &str, values: &[String]) -> Result<()> {
    if values.len() > MAX_VM_VALIDATION_ITEMS {
        return Err(MoltenError::invalid_harness(format!(
            "VM {label} count {} exceeds bound {MAX_VM_VALIDATION_ITEMS}",
            values.len()
        )));
    }
    for value in values {
        validate_text(label, value)?;
    }
    Ok(())
}

fn validate_text(label: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        Err(MoltenError::invalid_harness(format!("VM {label} must not be empty")))
    } else {
        Ok(())
    }
}

fn validate_decision(decision: &str) -> Result<()> {
    match decision {
        "pass" | "deny" => Ok(()),
        other => Err(MoltenError::invalid_harness(format!("unsupported VM decision {other}; expected pass or deny"))),
    }
}

fn push_if(
    diagnostics: &mut impl crate::bounded::VecSink<String>,
    condition: bool,
    diagnostic: &'static str,
) -> Result<()> {
    if condition {
        push_diagnostic(diagnostics, diagnostic.to_string())?;
    }
    Ok(())
}

fn push_diagnostic(diagnostics: &mut impl crate::bounded::VecSink<String>, diagnostic: String) -> Result<()> {
    validate_text("diagnostic", &diagnostic)?;
    if diagnostics.item_count() >= MAX_VM_VALIDATION_ITEMS {
        return Err(MoltenError::invalid_harness("VM validation diagnostics exceeded bound"));
    }
    diagnostics.push_item(diagnostic);
    Ok(())
}

fn value_to_iovalue(value: &Value<IoValue>) -> IoValue {
    crate::preserves_rail::value_to_iovalue(value)
}

fn record(label: &'static str, fields: Vec<IoValue>) -> IoValue {
    crate::preserves_rail::record(label, fields)
}

fn sequence(values: Vec<IoValue>) -> IoValue {
    crate::preserves_rail::sequence(values)
}

fn string(value: impl AsRef<str>) -> IoValue {
    crate::preserves_rail::string(value)
}

fn check_value(name: &'static str, state: &'static str) -> IoValue {
    record("check", vec![string(name), string(state)])
}

fn status(is_passing: bool) -> &'static str {
    if is_passing { "pass" } else { "deny" }
}

#[cfg(test)]
mod tests {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/nixos/vm/parts/validation/tests/m000/p000/body.rs"));
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/nixos/vm/parts/validation/tests/m000/p001/body.rs"));
}


fn validate_transfer_attenuation(parent: &str, child: &str) -> Result<()> {
    validate_transfer(parent)?;
    validate_transfer(child)?;
    match parent {
        TRANSFER_LOCAL_ONLY if child != TRANSFER_LOCAL_ONLY => Err(MoltenError::invalid_harness(
            "attenuated effect handle cannot make a local-only parent transferable",
        )),
        TRANSFER_ATTENUATED_DELEGATION if child == TRANSFER_REMOTE_PROXY => {
            Err(MoltenError::invalid_harness("attenuated delegation handle cannot become a remote-proxy handle"))
        }
        _ => Ok(()),
    }
}

fn validate_non_empty(value: &str, field: &str) -> Result<()> {
    if value.is_empty() {
        Err(MoltenError::invalid_harness(format!("{field} must not be empty")))
    } else {
        Ok(())
    }
}

fn require_operation(operations: &[String], operation: &str, label: &str) -> Result<()> {
    if operations.iter().any(|candidate| candidate == operation) {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("{label} does not admit effect operation {operation}")))
    }
}

fn require_schema(value: &Value<IoValue>, expected: &str, field: &str) -> Result<()> {
    let actual = required_string(value, field)?;
    if actual == expected {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("unsupported {field} {actual}; expected {expected}")))
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

#[allow(clippy::owned_cow)]
fn required_sequence<'a>(value: &'a Value<IoValue>, field: &str) -> Result<std::borrow::Cow<'a, Vec<Value<IoValue>>>> {
    value
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected sequence for {field}")))
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

fn required_ref(value: &Value<IoValue>, field: &str) -> Result<String> {
    let reference = required_string(value, field)?;
    require_ref(&reference, field)?;
    Ok(reference)
}

fn require_ref(value: &str, field: &str) -> Result<()> {
    validate_content_ref(value).map_err(|error| {
        MoltenError::invalid_harness(format!("expected canonical content ref for {field}, got {value}: {error}"))
    })
}

fn required_record_string(value: &Value<IoValue>, label: &str, field: &str) -> Result<String> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    required_string(&record[0], field)
}

fn required_record_bool(value: &Value<IoValue>, label: &str, field: &str) -> Result<bool> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    record[0]
        .as_boolean()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected boolean for {field}")))
}

fn required_record_ref(value: &Value<IoValue>, label: &str, field: &str) -> Result<String> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    required_ref(&record[0], field)
}

#[cfg(test)]
mod tests {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/effects/parts/mod/tests/m000/p000/body.rs"));
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/effects/parts/mod/tests/m000/p001/body.rs"));
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/effects/parts/mod/tests/m000/p002/body.rs"));
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/effects/parts/mod/tests/m000/p003/body.rs"));
}

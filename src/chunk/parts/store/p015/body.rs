
fn write_immutable_blob(
    root: &CapabilityChunkRoot,
    path: &StorePath,
    bytes: &[u8],
    expected_ref: &str,
) -> Result<()> {
    if root.root().try_exists(path)? {
        let existing = root.root().read(path)?;
        let existing_ref = hash_blob_bytes(&existing);
        if existing_ref != expected_ref {
            return Err(MoltenError::invalid_harness(format!(
                "immutable blob path for {expected_ref} contains corrupted bytes hashing to {existing_ref}"
            )));
        }
    } else {
        root.root().write(path, bytes)?;
    }
    Ok(())
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

fn simple_record_any<'a>(value: &'a IoValue, label: &str) -> Result<std::borrow::Cow<'a, Record<Value<IoValue>>>> {
    value
        .collect_simple_record(label, None)
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...> record")))
}

fn record_arity(record: &Record<Value<IoValue>>) -> usize {
    record._vec().len().saturating_sub(1)
}

fn record_string(value: &Value<IoValue>, label: &str) -> Result<String> {
    let record = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...> field")))?;
    required_string(&record[0], label)
}

fn record_u64(value: &Value<IoValue>, label: &str) -> Result<u64> {
    let record = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...> field")))?;
    required_u64(&record[0], label)
}

fn record_sequence(value: &Value<IoValue>, label: &str) -> Result<Vec<IoValue>> {
    let record = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...> field")))?;
    let sequence = record[0]
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected sequence for {label}")))?;
    Ok(sequence.iter().map(value_to_iovalue).collect())
}

fn record_string_sequence(value: &Value<IoValue>, label: &str) -> Result<Vec<String>> {
    record_sequence(value, label)?.iter().map(|value| required_string(value, label)).collect()
}

fn record_optional_string(value: &Value<IoValue>, label: &str) -> Result<Option<String>> {
    let record = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...> field")))?;
    if let Some(value) = record[0].as_string() {
        Ok(Some(value.into_owned()))
    } else if record[0].collect_simple_record("none", Some(0)).is_some() {
        Ok(None)
    } else {
        Err(MoltenError::invalid_harness(format!("expected string or <none> for {label}")))
    }
}

fn require_schema(value: &Value<IoValue>, expected: &str, field: &str) -> Result<()> {
    let actual = required_string(value, field)?;
    if actual != expected {
        return Err(MoltenError::invalid_harness(format!("expected {field} schema {expected}, got {actual}")));
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

#[cfg(test)]
mod tests {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/chunk/parts/store/tests/m000/p000/body.rs"));
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/chunk/parts/store/tests/m000/p001/body.rs"));
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/chunk/parts/store/tests/m000/p002/body.rs"));
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/chunk/parts/store/tests/m000/p003/body.rs"));
}

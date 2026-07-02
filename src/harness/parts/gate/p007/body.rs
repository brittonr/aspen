
fn validate_tool_record(value: &Value<IoValue>) -> Result<()> {
    let value = value_to_iovalue(value);
    let tool = simple_record(&value, "tool", 2)?;
    let name = required_string(&tool[0], "gate receipt tool name")?;
    if name != "molten" {
        return Err(MoltenError::invalid_harness(format!("unsupported gate receipt tool {name}")));
    }
    let version = required_string(&tool[1], "gate receipt tool version")?;
    if version.is_empty() {
        return Err(MoltenError::invalid_harness("gate receipt tool version must not be empty"));
    }
    Ok(())
}

fn parse_artifact_refs(value: &Value<IoValue>) -> Result<Vec<(String, String)>> {
    let value = value_to_iovalue(value);
    let artifact_refs = simple_record(&value, "artifact-refs", 1)?;
    let ref_values = required_sequence(&artifact_refs[0], "gate receipt artifact refs")?;
    let mut refs = Vec::with_capacity(ref_values.len());
    for ref_value in ref_values.iter() {
        let ref_value = value_to_iovalue(ref_value);
        let artifact_ref = simple_record(&ref_value, "artifact-ref", 2)?;
        refs.push((
            required_string(&artifact_ref[0], "artifact ref kind")?,
            required_hash(&artifact_ref[1], "artifact ref value")?,
        ));
    }
    Ok(refs)
}

fn require_artifact_ref(refs: &[(String, String)], kind: &str, expected: &str) -> Result<()> {
    if refs.iter().any(|(actual_kind, actual_ref)| actual_kind == kind && actual_ref == expected) {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("gate receipt artifact refs missing {kind} ref {expected}")))
    }
}

fn require_kinds(refs: &[(String, String)], expected: &[&str]) -> Result<()> {
    for kind in expected.iter().copied() {
        require_artifact_kind(refs, kind)?;
    }
    Ok(())
}

fn require_artifact_kind(refs: &[(String, String)], kind: &str) -> Result<()> {
    if refs.iter().any(|(actual_kind, _)| actual_kind == kind) {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("gate receipt artifact refs missing {kind} ref")))
    }
}

fn required_record_string(value: &Value<IoValue>, label: &str, field: &str) -> Result<String> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    required_string(&record[0], field)
}

fn required_record_hash(value: &Value<IoValue>, label: &str, field: &str) -> Result<String> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    required_hash(&record[0], field)
}

fn required_record_optional_hash(value: &Value<IoValue>, label: &str, field: &str) -> Result<Option<String>> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    let optional = value_to_iovalue(&record[0]);
    if optional.collect_simple_record("none", Some(0)).is_some() {
        Ok(None)
    } else if let Some(some) = optional.collect_simple_record("some", Some(1)) {
        required_hash(&some[0], field).map(Some)
    } else {
        Err(MoltenError::invalid_harness(format!("expected <none> or <some ref> for {field}")))
    }
}

fn required_record_hash_sequence(value: &Value<IoValue>, label: &str) -> Result<Vec<String>> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    let values = required_sequence(&record[0], label)?;
    values.iter().map(|value| required_hash(value, label)).collect()
}

fn required_record_value(value: &Value<IoValue>, label: &str) -> Result<IoValue> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    Ok(value_to_iovalue(&record[0]))
}

fn required_record_values(value: &Value<IoValue>, label: &str) -> Result<Vec<IoValue>> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    let values = required_sequence(&record[0], label)?;
    Ok(values.iter().map(value_to_iovalue).collect())
}

fn required_record_u64(value: &Value<IoValue>, label: &str, field: &str) -> Result<u64> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    required_u64(&record[0], field)
}

fn required_chain_scope(value: &Value<IoValue>) -> Result<crate::evidence_chain::ChainScope> {
    let value = value_to_iovalue(value);
    let chain = simple_record(&value, "chain", 3)?;
    Ok(crate::evidence_chain::ChainScope::new(
        required_record_string(&chain[0], "scope", "chain scope")?,
        required_record_string(&chain[1], "id", "chain id")?,
        required_record_string(&chain[2], "epoch", "chain epoch")?,
    ))
}

fn simple_record<'a>(value: &'a IoValue, label: &str, arity: usize) -> Result<Cow<'a, Record<Value<IoValue>>>> {
    value
        .collect_simple_record(label, Some(arity))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...> with arity {arity}")))
}

#[allow(clippy::owned_cow)]
fn required_sequence<'a>(value: &'a Value<IoValue>, field: &str) -> Result<Cow<'a, Vec<Value<IoValue>>>> {
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

fn required_hash(value: &Value<IoValue>, field: &str) -> Result<String> {
    let hash = required_string(value, field)?;
    validate_content_ref(&hash).map_err(|error| {
        MoltenError::invalid_harness(format!("expected canonical content ref for {field}, got {hash}: {error}"))
    })?;
    Ok(hash)
}

fn required_u64(value: &Value<IoValue>, field: &str) -> Result<u64> {
    value
        .as_u64()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected u64 for {field}")))?
        .map_err(|error| MoltenError::invalid_harness(format!("u64 out of range for {field}: {error}")))
}

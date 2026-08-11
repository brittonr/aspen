
fn parse_payloads(value: &Value<IoValue>) -> Result<Vec<ProtocolPayload>> {
    let values = field_sequence(value, "payloads")?;
    let mut payloads = Vec::with_capacity(values.len());
    for (index, payload) in values.iter().enumerate() {
        let fields = payload
            .collect_simple_record("payload", Some(2))
            .ok_or_else(|| MoltenError::invalid_harness("expected protocol payload"))?;
        let tag = required_string(&fields[0], "payload tag")?;
        let schema_ref = required_ref(&fields[1], "payload schema ref")?;
        payloads.push(ProtocolPayload {
            tag,
            schema_ref,
            payload_id: u32::try_from(index)
                .map_err(|error| MoltenError::invalid_harness(format!("payload id out of range: {error}")))?,
        });
    }
    Ok(payloads)
}

fn field_sequence(value: &Value<IoValue>, label: &str) -> Result<Vec<Value<IoValue>>> {
    let value = value_to_iovalue(value);
    let fields = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} [...]>")))?;
    let values = fields[0]
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected sequence for {label}")))?;
    ensure_count_at_most(values.len(), MAX_PROTOCOL_ITEMS, label)?;
    Ok(values.iter().cloned().collect())
}

fn parse_string_sequence(value: &Value<IoValue>, label: &str) -> Result<Vec<String>> {
    let values = field_sequence(value, label)?;
    let mut strings = Vec::with_capacity(values.len());
    for value in &values {
        strings.push(required_string(value, label)?);
    }
    Ok(strings)
}

fn parse_ref_sequence(value: &Value<IoValue>, label: &str) -> Result<Vec<String>> {
    let values = field_sequence(value, label)?;
    let mut refs = Vec::with_capacity(values.len());
    for value in &values {
        refs.push(required_ref(value, label)?);
    }
    Ok(refs)
}

fn parse_checks(value: &Value<IoValue>) -> Result<Vec<(String, String)>> {
    let values = field_sequence(value, "checks")?;
    let mut checks = Vec::with_capacity(values.len());
    for value in &values {
        let fields = value
            .collect_simple_record("check", Some(2))
            .ok_or_else(|| MoltenError::invalid_harness("expected protocol check"))?;
        checks.push((required_string(&fields[0], "check name")?, required_string(&fields[1], "check status")?));
    }
    Ok(checks)
}

fn require_check(checks: &[(String, String)], name: &str, label: &str) -> Result<()> {
    if checks.iter().any(|(check, status)| check == name && status == "pass") {
        return Ok(());
    }
    Err(MoltenError::invalid_harness(format!("missing passing check {name} for {label}")))
}

fn strings_sequence(values: &[String]) -> IoValue {
    sequence(values.iter().map(|value| string(value)).collect())
}

fn refs_sequence(values: &[String]) -> IoValue {
    sequence(values.iter().map(|value| string(value)).collect())
}

fn checks_value(names: &[&str]) -> IoValue {
    let mut checks = Vec::with_capacity(names.len());
    for name in names {
        checks.push(record("check", vec![string(name), string("pass")]));
    }
    record("checks", vec![sequence(checks)])
}

fn optional_ref_value(reference: Option<&str>) -> IoValue {
    match reference {
        Some(value) => record("some", vec![string(value)]),
        None => record("none", Vec::new()),
    }
}

fn record_iovalue(value: &Value<IoValue>, label: &str) -> Result<IoValue> {
    let value = value_to_iovalue(value);
    let fields = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} VALUE>")))?;
    Ok(value_to_iovalue(&fields[0]))
}

fn record_string(value: &Value<IoValue>, label: &str) -> Result<String> {
    let value = value_to_iovalue(value);
    let fields = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} STRING>")))?;
    required_string(&fields[0], label)
}

fn record_ref(value: &Value<IoValue>, label: &str) -> Result<String> {
    let reference = record_string(value, label)?;
    require_ref(&reference, label)?;
    Ok(reference)
}

fn record_optional_ref(value: &Value<IoValue>, label: &str) -> Result<Option<String>> {
    let fields = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} OPTION>")))?;
    if fields[0].collect_simple_record("none", Some(0)).is_some() {
        return Ok(None);
    }
    let some = fields[0]
        .collect_simple_record("some", Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected optional ref for {label}")))?;
    Ok(Some(required_ref(&some[0], label)?))
}

fn record_u64(value: &Value<IoValue>, label: &str) -> Result<u64> {
    let value = value_to_iovalue(value);
    let fields = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} U64>")))?;
    required_u64(&fields[0], label)
}

fn required_string(value: &Value<IoValue>, label: &str) -> Result<String> {
    value
        .as_string()
        .map(|value| value.into_owned())
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected string for {label}")))
}

fn required_u64(value: &Value<IoValue>, label: &str) -> Result<u64> {
    value
        .as_u64()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected u64 for {label}")))?
        .map_err(|error| MoltenError::invalid_harness(format!("u64 out of range for {label}: {error}")))
}

fn required_ref(value: &Value<IoValue>, label: &str) -> Result<String> {
    let reference = required_string(value, label)?;
    require_ref(&reference, label)?;
    Ok(reference)
}

fn require_schema(value: &Value<IoValue>, expected: &str, label: &str) -> Result<()> {
    let actual = required_string(value, label)?;
    if actual == expected {
        return Ok(());
    }
    Err(MoltenError::invalid_harness(format!("expected {expected} for {label}, got {actual}")))
}

fn validate_protocol_id(value: &str) -> Result<()> {
    if value.starts_with("proto:") {
        return Ok(());
    }
    Err(MoltenError::invalid_harness(format!("expected proto: protocol id, got {value}")))
}

fn validate_protocol_ref(value: &str, label: &str) -> Result<()> {
    require_ref(value, label)
}

fn validate_session_id(value: &str) -> Result<()> {
    if value.starts_with("session:") {
        return Ok(());
    }
    Err(MoltenError::invalid_harness(format!("expected session: protocol session id, got {value}")))
}

fn validate_direction(value: &str) -> Result<()> {
    if value == "send" || value == "recv" {
        return Ok(());
    }
    Err(MoltenError::invalid_harness(format!("unsupported protocol local action direction {value}")))
}

fn validate_gate_decision(value: &str, label: &str) -> Result<()> {
    if matches!(value, "pass" | "deny") {
        return Ok(());
    }
    Err(MoltenError::invalid_harness(format!("unsupported {label} {value}")))
}

fn validate_name(value: &str, label: &str) -> Result<()> {
    let has_valid_chars = value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == ':' || ch == '-' || ch == '_' || ch == '.');
    if !value.is_empty() && has_valid_chars {
        return Ok(());
    }
    Err(MoltenError::invalid_harness(format!("invalid {label}: {value}")))
}

fn validate_unique_names(values: &[String], label: &str) -> Result<()> {
    ensure_count_at_most(values.len(), MAX_PROTOCOL_ITEMS, label)?;
    for value in values {
        validate_name(value, label)?;
    }
    for (index, left) in values.iter().enumerate() {
        for right in values.iter().skip(index + 1) {
            if left == right {
                return Err(MoltenError::invalid_harness(format!("duplicate {label} entry {left}")));
            }
        }
    }
    Ok(())
}

fn validate_unique_branch_labels(branches: &[ProtocolBranchInput]) -> Result<()> {
    for (index, left) in branches.iter().enumerate() {
        for right in branches.iter().skip(index + 1) {
            if left.label == right.label {
                return Err(MoltenError::invalid_harness(format!("duplicate protocol branch label {}", left.label)));
            }
        }
    }
    Ok(())
}

fn require_member(value: &str, values: &[String], label: &str) -> Result<()> {
    if values.iter().any(|item| item == value) {
        return Ok(());
    }
    Err(MoltenError::invalid_harness(format!("unknown {label}: {value}")))
}

fn require_payload(value: &str, manifest: &ProtocolManifest) -> Result<()> {
    if manifest.payloads.iter().any(|payload| payload.tag == value) {
        return Ok(());
    }
    Err(MoltenError::invalid_harness(format!("unknown protocol payload tag {value}")))
}

fn registry_id(entries: &[RegistryEntry], name: &str, label: &str) -> Result<u32> {
    for entry in entries {
        if entry.name == name {
            return Ok(entry.id);
        }
    }
    Err(MoltenError::invalid_harness(format!("missing {label} registry entry {name}")))
}

fn registry_name(entries: &[RegistryEntry], id: u32, label: &str) -> Result<String> {
    for entry in entries {
        if entry.id == id {
            return Ok(entry.name.clone());
        }
    }
    Err(MoltenError::invalid_harness(format!("missing {label} registry id {id}")))
}

fn validate_refs(refs: &[String], label: &str) -> Result<()> {
    ensure_count_at_most(refs.len(), MAX_PROTOCOL_ITEMS, label)?;
    for reference in refs {
        require_ref(reference, label)?;
    }
    Ok(())
}

// r[impl molten.runtime_spine.canonical_content_refs.migration]
fn require_ref(reference: &str, label: &str) -> Result<()> {
    validate_content_ref(reference).map_err(|error| {
        MoltenError::invalid_harness(format!("expected canonical content ref for {label}, got {reference}: {error}"))
    })
}

fn ensure_count_at_most(actual: usize, maximum: usize, label: &str) -> Result<()> {
    if actual <= maximum {
        return Ok(());
    }
    Err(MoltenError::invalid_harness(format!("{label} count {actual} exceeds bound {maximum}")))
}

fn synthetic_ref(label: &str) -> Result<String> {
    canonical_hash(&record("protocol-fixture-ref", vec![string(label)]))
}

#[cfg(test)]
mod tests {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/protocol/parts/session/tests/m000/p000/body.rs"));
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/protocol/parts/session/tests/m000/p001/body.rs"));
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/protocol/parts/session/tests/m000/p002/body.rs"));
}

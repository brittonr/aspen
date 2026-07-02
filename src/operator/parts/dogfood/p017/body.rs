
fn parse_checks(value: &Value<IoValue>) -> Result<Vec<(String, String)>> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let checks = simple_record(&value, "checks", 1)?;
    let items = required_sequence(&checks[0], "operator checks")?;
    ensure_count_at_most(items.len(), MAX_OPERATOR_REFS, "operator checks")?;
    let mut parsed = Vec::new();
    for item in items.iter() {
        let item = crate::preserves_rail::value_to_iovalue(item);
        let check = simple_record(&item, "check", 2)?;
        let name = required_string(&check[0], "operator check name")?;
        let status = required_string(&check[1], "operator check status")?;
        if status != "pass" && status != "fail" && status != "diagnostic" {
            return Err(MoltenError::invalid_harness(format!("operator check {name} has status {status}")));
        }
        parsed.push_limited_value((name, status), MAX_OPERATOR_REFS, "operator checks")?;
    }
    Ok(parsed)
}

fn workflow_check_pass(checks: &[(String, String)], expected: &str) -> bool {
    checks.iter().any(|(name, status)| name == expected && status == "pass")
}

fn require_check(checks: &[(String, String)], expected: &str, context: &str) -> Result<()> {
    if checks.iter().any(|(name, _)| name == expected) {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("{context} missing {expected} check")))
    }
}

fn require_schema(value: &Value<IoValue>, expected: &str, context: &str) -> Result<()> {
    let actual = required_string(value, context)?;
    if actual == expected {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("unsupported {context} schema {actual}; expected {expected}")))
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

fn record_string(value: &Value<IoValue>, label: &str) -> Result<String> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let fields = simple_record(&value, label, 1)?;
    required_string(&fields[0], label)
}

fn record_bool(value: &Value<IoValue>, label: &str) -> Result<bool> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let fields = simple_record(&value, label, 1)?;
    fields[0]
        .as_boolean()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected bool for {label}")))
}

fn record_u64(value: &Value<IoValue>, label: &str) -> Result<u64> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let fields = simple_record(&value, label, 1)?;
    let number = fields[0]
        .as_u64()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected u64 for {label}")))?;
    number.map_err(|_| MoltenError::invalid_harness(format!("u64 out of range for {label}")))
}

fn record_ref(value: &Value<IoValue>, label: &str) -> Result<String> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let fields = simple_record(&value, label, 1)?;
    required_ref(&fields[0], label)
}

fn record_optional_ref(value: &Value<IoValue>, label: &str) -> Result<Option<String>> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let fields = simple_record(&value, label, 1)?;
    parse_optional_ref_value(&fields[0])
}

fn record_ref_sequence(value: &Value<IoValue>, label: &str) -> Result<Vec<String>> {
    record_iovalue_sequence(value, label)?
        .iter()
        .map(|item| required_ref(item.as_ref(), label))
        .collect()
}

fn record_string_sequence(value: &Value<IoValue>, label: &str) -> Result<Vec<String>> {
    record_iovalue_sequence(value, label)?
        .iter()
        .map(|item| required_string(item.as_ref(), label))
        .collect()
}

fn record_iovalue_sequence(value: &Value<IoValue>, label: &str) -> Result<Vec<IoValue>> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let fields = simple_record(&value, label, 1)?;
    let items = required_sequence(&fields[0], label)?;
    ensure_count_at_most(items.len(), MAX_OPERATOR_REFS, label)?;
    let mut values = Vec::new();
    for item in items.iter() {
        values.push_limited_value(crate::preserves_rail::value_to_iovalue(item), MAX_OPERATOR_REFS, label)?;
    }
    Ok(values)
}

fn record_step_receipts(value: &Value<IoValue>, label: &str) -> Result<Vec<(String, String)>> {
    let items = record_iovalue_sequence(value, label)?;
    let mut receipts = Vec::new();
    for item in &items {
        let fields = simple_record(item, "step", 2)?;
        let name = required_string(&fields[0], "dogfood step receipt name")?;
        let reference = required_ref(&fields[1], "dogfood step receipt ref")?;
        receipts.push_limited_value((name, reference), MAX_OPERATOR_REFS, "dogfood step receipts")?;
    }
    Ok(receipts)
}

fn record_file_refs(value: &Value<IoValue>, label: &str) -> Result<Vec<(String, String)>> {
    let items = record_iovalue_sequence(value, label)?;
    let mut files = Vec::new();
    for item in &items {
        let fields = simple_record(item, "file", 2)?;
        let name = required_string(&fields[0], "Nix dogfood file name")?;
        let reference = required_ref(&fields[1], "Nix dogfood file ref")?;
        files.push_limited_value((name, reference), MAX_OPERATOR_REFS, "Nix dogfood file refs")?;
    }
    Ok(files)
}

fn member_ref(value: &Value<IoValue>, expected_name: &str) -> Result<String> {
    record_file_refs(value, "members")?
        .into_iter()
        .find_map(|(name, reference)| (name == expected_name).then_some(reference))
        .ok_or_else(|| MoltenError::invalid_harness(format!("release evidence bundle missing member {expected_name}")))
}

fn parse_optional_ref_value(value: &Value<IoValue>) -> Result<Option<String>> {
    if value.collect_simple_record("none", Some(0)).is_some() {
        return Ok(None);
    }
    if let Some(some) = value.collect_simple_record("some", Some(1)) {
        return required_ref(&some[0], "optional ref").map(Some);
    }
    required_ref(value, "optional ref").map(Some)
}

fn required_string(value: &Value<IoValue>, field: &str) -> Result<String> {
    value
        .as_string()
        .map(|value| value.into_owned())
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected string for {field}")))
}

fn required_ref(value: &Value<IoValue>, field: &str) -> Result<String> {
    let value = required_string(value, field)?;
    validate_ref(&value, field)?;
    Ok(value)
}

fn usize_to_u64(value: usize, field: &str) -> Result<u64> {
    u64::try_from(value).map_err(|error| MoltenError::invalid_harness(format!("{field} overflows u64: {error}")))
}

#[cfg(test)]
mod tests {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/operator/parts/dogfood/tests/m000/p000/body.rs"));
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/operator/parts/dogfood/tests/m000/p001/body.rs"));
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/operator/parts/dogfood/tests/m000/p002/body.rs"));
}

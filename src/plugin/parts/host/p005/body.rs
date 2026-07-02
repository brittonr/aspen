
fn parse_checks(value: &Value<IoValue>) -> Result<Vec<(String, String)>> {
    let value = value_to_iovalue(value);
    let checks = simple_record(&value, "checks", 1)?;
    let items = required_sequence(&checks[0], "plugin checks")?;
    ensure_count_at_most(items.len(), MAX_PLUGIN_CHECKS, "plugin checks")?;
    let mut parsed = Vec::new();
    for item in items.iter() {
        let item = value_to_iovalue(item);
        let check = simple_record(&item, "check", 2)?;
        let name = required_string(&check[0], "plugin check name")?;
        let status = required_string(&check[1], "plugin check status")?;
        match status.as_str() {
            "pass" | "fail" | "diagnostic" => {
                parsed.push_limited((name, status), MAX_PLUGIN_CHECKS, "plugin checks")?
            }
            _ => return Err(MoltenError::invalid_harness("plugin check status must be pass/fail/diagnostic")),
        }
    }
    Ok(parsed)
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

#[allow(clippy::owned_cow)]
fn required_sequence<'a>(value: &'a Value<IoValue>, field: &str) -> Result<std::borrow::Cow<'a, Vec<Value<IoValue>>>> {
    value
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected sequence for {field}")))
}

fn record_string(value: &Value<IoValue>, label: &str) -> Result<String> {
    let value = value_to_iovalue(value);
    let fields = simple_record(&value, label, 1)?;
    required_string(&fields[0], label)
}

fn record_ref(value: &Value<IoValue>, label: &str) -> Result<String> {
    let reference = record_string(value, label)?;
    validate_ref(&reference, label)?;
    Ok(reference)
}

fn record_string_sequence(value: &Value<IoValue>, label: &str) -> Result<Vec<String>> {
    let value = value_to_iovalue(value);
    let fields = simple_record(&value, label, 1)?;
    let items = required_sequence(&fields[0], label)?;
    ensure_count_at_most(items.len(), MAX_PLUGIN_REFS, label)?;
    let mut values = Vec::new();
    for item in items.iter() {
        values.push_limited(required_string(item, label)?, MAX_PLUGIN_REFS, label)?;
    }
    Ok(values)
}

fn record_ref_sequence(value: &Value<IoValue>, label: &str) -> Result<Vec<String>> {
    let values = record_string_sequence(value, label)?;
    validate_refs(&values, label)?;
    Ok(values)
}

fn required_string(value: &Value<IoValue>, field: &str) -> Result<String> {
    value
        .as_string()
        .map(|value| value.to_string())
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected string for {field}")))
}

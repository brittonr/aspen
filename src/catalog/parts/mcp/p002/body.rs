
fn push_optional_text_filter(
    filters: &mut impl crate::bounded::VecSink<Filter>,
    args: &[IoValue],
    arg_name: &str,
    prefix: &str,
) -> Result<()> {
    if let Some(value) = optional_arg_string(args, arg_name) {
        push_bounded(filters, Filter::Text(format!("{prefix}:{value}")), MAX_FILTERS, "catalog MCP filters")?;
    }
    Ok(())
}

fn append_filter_args(
    filters: &mut impl crate::bounded::VecSink<Filter>,
    values: Vec<String>,
    convert: impl Fn(String) -> Filter,
) -> Result<()> {
    for value in values {
        push_bounded(&mut *filters, convert(value), MAX_FILTERS, "catalog MCP filters")?;
    }
    Ok(())
}

fn visibility_from_args(args: &[IoValue]) -> Result<VisibilityInput> {
    Ok(VisibilityInput {
        policy_refs: arg_strings(args, "policy-ref")?,
        capability_refs: arg_strings(args, "capability-ref")?,
        hidden_refs: arg_strings(args, "hidden-ref")?,
        redaction_profile_ref: optional_arg_string(args, "redaction-profile-ref"),
    })
}

fn required_arg_string(args: &[IoValue], label: &str) -> Result<String> {
    optional_arg_string(args, label)
        .ok_or_else(|| MoltenError::invalid_harness(format!("catalog MCP request missing required arg <{label} ...>")))
}

fn optional_arg_string(args: &[IoValue], label: &str) -> Option<String> {
    args.iter().find_map(|arg| {
        arg.collect_simple_record(label, Some(1))
            .and_then(|fields| fields[0].as_string().map(|value| value.into_owned()))
    })
}

fn arg_strings(args: &[IoValue], label: &str) -> Result<Vec<String>> {
    ensure_count_at_most(args.len(), MAX_ARGS, "catalog MCP args")?;
    let mut values = Vec::new();
    for arg in args {
        if let Some(fields) = arg.collect_simple_record(label, Some(1)) {
            push_bounded(&mut values, required_string(&fields[0], label)?, MAX_ARGS, "catalog MCP arg strings")?;
        }
    }
    Ok(values)
}

fn arg_bool(args: &[IoValue], label: &str, default: bool) -> Result<bool> {
    for arg in args {
        if let Some(fields) = arg.collect_simple_record(label, Some(1)) {
            return fields[0]
                .as_boolean()
                .ok_or_else(|| MoltenError::invalid_harness(format!("catalog MCP arg {label} must be bool")));
        }
    }
    Ok(default)
}

fn arg_u64(args: &[IoValue], label: &str, default: u64) -> Result<u64> {
    for arg in args {
        if let Some(fields) = arg.collect_simple_record(label, Some(1)) {
            return fields[0]
                .as_u64()
                .ok_or_else(|| MoltenError::invalid_harness(format!("catalog MCP arg {label} must be u64")))?
                .map_err(|error| {
                    MoltenError::invalid_harness(format!("catalog MCP arg {label} is out of range: {error}"))
                });
        }
    }
    Ok(default)
}

fn canonical_hash(value: &IoValue) -> Result<String> {
    crate::preserves_rail::canonical_hash(value)
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

fn value_to_iovalue(value: &PreservesValue<IoValue>) -> IoValue {
    crate::preserves_rail::value_to_iovalue(value)
}

fn refs_sequence(refs: &[String]) -> IoValue {
    sequence(refs.iter().map(string).collect())
}

fn optional_ref_value(value: Option<&str>) -> IoValue {
    value.map_or_else(|| record("none", Vec::new()), |value| record("some", vec![string(value)]))
}

fn parse_optional_ref_value(value: &PreservesValue<IoValue>) -> Result<Option<String>> {
    if value.collect_simple_record("none", Some(0)).is_some() {
        return Ok(None);
    }
    if let Some(some) = value.collect_simple_record("some", Some(1)) {
        return required_ref(&some[0], "optional ref").map(Some);
    }
    required_ref(value, "optional ref").map(Some)
}

fn checks_value(names: &[&str]) -> IoValue {
    checks_value_from_pairs(&names.iter().map(|name| (*name, "pass")).collect::<Vec<_>>())
}

fn checks_value_from_pairs(checks: &[(&str, &str)]) -> IoValue {
    record("checks", vec![sequence(
        checks.iter().map(|(name, status)| record("check", vec![string(name), string(status)])).collect(),
    )])
}

fn parse_checks(value: &PreservesValue<IoValue>) -> Result<Vec<String>> {
    let value = value_to_iovalue(value);
    let checks = simple_record(&value, "checks", 1)?;
    let items = required_sequence(&checks[0], "catalog MCP checks")?;
    ensure_count_at_most(items.len(), MAX_CHECKS, "catalog MCP checks")?;
    let mut parsed = Vec::new();
    for item in items.iter() {
        let item = value_to_iovalue(item);
        let check = simple_record(&item, "check", 2)?;
        let name = required_string(&check[0], "catalog MCP check name")?;
        let status = required_string(&check[1], "catalog MCP check status")?;
        if status != "pass" && status != "fail" {
            return Err(MoltenError::invalid_harness(format!("catalog MCP check {name} has status {status}")));
        }
        push_bounded(&mut parsed, name, MAX_CHECKS, "catalog MCP checks")?;
    }
    Ok(parsed)
}

fn require_check(checks: &[String], expected: &str, context: &str) -> Result<()> {
    if checks.iter().any(|check| check == expected) {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("{context} missing {expected} check")))
    }
}

fn require_schema(value: &PreservesValue<IoValue>, expected: &str, context: &str) -> Result<()> {
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
) -> Result<std::borrow::Cow<'a, PreservesRecord<PreservesValue<IoValue>>>> {
    value
        .collect_simple_record(label, Some(arity))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...> with arity {arity}")))
}

#[allow(clippy::owned_cow)]
fn required_sequence<'a>(
    value: &'a PreservesValue<IoValue>,
    field: &str,
) -> Result<std::borrow::Cow<'a, Vec<PreservesValue<IoValue>>>> {
    value
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected sequence for {field}")))
}

fn record_string(value: &PreservesValue<IoValue>, label: &str) -> Result<String> {
    let value = value_to_iovalue(value);
    let fields = simple_record(&value, label, 1)?;
    required_string(&fields[0], label)
}

fn record_ref(value: &PreservesValue<IoValue>, label: &str) -> Result<String> {
    let value = value_to_iovalue(value);
    let fields = simple_record(&value, label, 1)?;
    required_ref(&fields[0], label)
}

fn record_optional_ref(value: &PreservesValue<IoValue>, label: &str) -> Result<Option<String>> {
    let value = value_to_iovalue(value);
    let fields = simple_record(&value, label, 1)?;
    parse_optional_ref_value(&fields[0])
}

fn record_sequence(value: &PreservesValue<IoValue>, label: &str) -> Result<Vec<IoValue>> {
    let value = value_to_iovalue(value);
    let fields = simple_record(&value, label, 1)?;
    let items = required_sequence(&fields[0], label)?;
    ensure_count_at_most(items.len(), MAX_ARGS, label)?;
    let mut values = Vec::new();
    for item in items.iter() {
        push_bounded(&mut values, value_to_iovalue(item), MAX_ARGS, label)?;
    }
    Ok(values)
}

fn required_string(value: &PreservesValue<IoValue>, field: &str) -> Result<String> {
    value
        .as_string()
        .map(|value| value.into_owned())
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected string for {field}")))
}

fn required_ref(value: &PreservesValue<IoValue>, field: &str) -> Result<String> {
    let value = required_string(value, field)?;
    validate_ref(&value, field)?;
    Ok(value)
}

fn validate_decision(decision: &str) -> Result<()> {
    if matches!(decision, "pass" | "deny") {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("unsupported catalog MCP decision {decision}")))
    }
}

fn validate_ref(value_ref: &str, field: &str) -> Result<()> {
    validate_non_empty(value_ref, field)?;
    crate::preserves_rail::validate_content_ref(value_ref).map_err(|error| {
        MoltenError::invalid_harness(format!("{field} must be a canonical content ref, got {value_ref}: {error}"))
    })
}

fn validate_refs(refs: &[String], field: &str) -> Result<()> {
    ensure_count_at_most(refs.len(), MAX_REFS, field)?;
    for value_ref in refs {
        validate_ref(value_ref, field)?;
    }
    Ok(())
}

fn ensure_count_at_most(count: usize, maximum: usize, label: &str) -> Result<()> {
    if count > maximum {
        Err(MoltenError::invalid_harness(format!("{label} count {count} exceeds maximum {maximum}")))
    } else {
        Ok(())
    }
}

fn checked_count_sum(left: usize, right: usize, maximum: usize, label: &str) -> Result<usize> {
    let total = left
        .checked_add(right)
        .ok_or_else(|| MoltenError::invalid_harness(format!("{label} count overflow")))?;
    ensure_count_at_most(total, maximum, label)?;
    Ok(total)
}

fn push_bounded<T>(values: &mut impl crate::bounded::VecSink<T>, value: T, maximum: usize, label: &str) -> Result<()> {
    checked_count_sum(values.item_count(), 1, maximum, label)?;
    values.push_item(value);
    Ok(())
}

fn validate_non_empty(value: &str, field: &str) -> Result<()> {
    if value.is_empty() {
        Err(MoltenError::invalid_harness(format!("{field} must not be empty")))
    } else {
        Ok(())
    }
}

fn sorted_unique(refs: &[String]) -> Vec<String> {
    refs.iter().cloned().collect::<std::collections::BTreeSet<_>>().into_iter().collect()
}

#[cfg(test)]
mod tests {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/catalog/parts/mcp/tests/m000/p000/body.rs"));
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/catalog/parts/mcp/tests/m000/p001/body.rs"));
}

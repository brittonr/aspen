
fn checks_value_from_pairs(checks: &[(&str, &str)]) -> IoValue {
    record("checks", vec![sequence(
        checks.iter().map(|(name, status)| record("check", vec![string(name), string(status)])).collect(),
    )])
}

fn parse_checks(value: &PreservesValue<IoValue>) -> Result<Vec<String>> {
    let value = value_to_iovalue(value);
    let checks = simple_record(&value, "checks", 1)?;
    let items = required_sequence(&checks[0], "checks")?;
    let mut parsed = Vec::with_capacity(items.len());
    for item in items.iter() {
        let item = value_to_iovalue(item);
        let check = simple_record(&item, "check", 2)?;
        let name = required_string(&check[0], "check name")?;
        let status = required_string(&check[1], "check status")?;
        if status != "pass" && status != "fail" {
            return Err(MoltenError::invalid_harness(format!("eval cache check {name} has status {status}")));
        }
        parsed.push(name);
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
) -> Result<std::borrow::Cow<'a, Record<PreservesValue<IoValue>>>> {
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

fn required_u64(value: &PreservesValue<IoValue>, field: &str) -> Result<u64> {
    value
        .as_u64()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected u64 for {field}")))?
        .map_err(|error| MoltenError::invalid_harness(format!("u64 out of range for {field}: {error}")))
}

fn checked_table_len(count: u64, label: &str) -> Result<usize> {
    usize::try_from(count).map_err(|_| MoltenError::invalid_harness(format!("{label} count {count} exceeds usize")))
}

fn push_bounded<T>(values: &mut impl crate::bounded::VecSink<T>, value: T, maximum: usize, label: &str) -> Result<()> {
    let total = values
        .item_count()
        .checked_add(1)
        .ok_or_else(|| MoltenError::invalid_harness(format!("{label} count overflow")))?;
    if total > maximum {
        return Err(MoltenError::invalid_harness(format!("{label} count {total} exceeds bound {maximum}")));
    }
    values.push_item(value);
    Ok(())
}

fn validate_ref(value_ref: &str, field: &str) -> Result<()> {
    validate_non_empty(value_ref, field)?;
    validate_content_ref(value_ref).map_err(|error| {
        MoltenError::invalid_harness(format!("{field} must be a canonical content ref, got {value_ref}: {error}"))
    })
}

struct ApplyRefMatchInput<'a> {
    root: &'a Path,
    apply_refs: &'a [String],
    subsystem: &'a str,
    action: &'a str,
    object_ref: &'a str,
    object_kind: &'a str,
    retention_class: &'a str,
}

fn matching_apply_ref<'a>(input: ApplyRefMatchInput<'a>) -> Option<&'a str> {
    let mut fallback_ref = None;
    for apply_ref in input.apply_refs {
        let Ok(apply) = crate::retention::read_gc_apply(input.root, apply_ref) else {
            if fallback_ref.is_none() {
                fallback_ref = Some(apply_ref.as_str());
            }
            continue;
        };
        if apply.decision == "pass"
            && apply.subsystem == input.subsystem
            && apply.action == input.action
            && apply.object_ref == input.object_ref
            && apply.object_kind == input.object_kind
            && apply.retention_class == input.retention_class
        {
            return Some(apply_ref.as_str());
        }
        if fallback_ref.is_none() {
            fallback_ref = Some(apply_ref.as_str());
        }
    }
    fallback_ref
}

fn validate_refs(refs: &[String], field: &str) -> Result<()> {
    for value_ref in refs {
        validate_ref(value_ref, field)?;
    }
    Ok(())
}

fn validate_non_empty(value: &str, field: &str) -> Result<()> {
    if value.is_empty() {
        Err(MoltenError::invalid_harness(format!("{field} cannot be empty")))
    } else {
        Ok(())
    }
}

fn index_error(error: impl std::fmt::Display) -> MoltenError {
    MoltenError::invalid_harness(format!("eval cache redb index error: {error}"))
}

#[cfg(test)]
mod tests {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/eval/parts/cache/tests/m000/p000/body.rs"));
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/eval/parts/cache/tests/m000/p001/body.rs"));
}

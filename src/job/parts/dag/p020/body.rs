
fn record_optional_string(value: &Value<IoValue>, label: &str) -> Result<Option<String>> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    parse_optional_string_value(&record[0])
}

fn record_iovalue(value: &Value<IoValue>, label: &str) -> Result<IoValue> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    Ok(crate::preserves_rail::value_to_iovalue(&record[0]))
}

fn record_ref_sequence(value: &Value<IoValue>, label: &str) -> Result<Vec<String>> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    parse_ref_sequence_value(&record[0], label)
}

fn record_node_id_sequence(value: &Value<IoValue>, label: &str) -> Result<Vec<String>> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    let items = required_sequence(&record[0], label)?;
    ensure_count_at_most(items.len(), MAX_JOB_NODES, label)?;
    let mut ids = Vec::with_capacity(items.len());
    for item in items.iter() {
        let id = required_string(item, label)?;
        validate_node_id(&id)?;
        push_bounded(&mut ids, id, MAX_JOB_NODES, label)?;
    }
    Ok(ids)
}

fn record_string_sequence(value: &Value<IoValue>, label: &str) -> Result<Vec<String>> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    let items = required_sequence(&record[0], label)?;
    ensure_count_at_most(items.len(), MAX_JOB_REFS, label)?;
    let mut strings = Vec::with_capacity(items.len());
    for item in items.iter() {
        push_bounded(&mut strings, required_string(item, label)?, MAX_JOB_REFS, label)?;
    }
    Ok(strings)
}

fn record_port_sequence(value: &Value<IoValue>, label: &str) -> Result<Vec<String>> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    let items = required_sequence(&record[0], label)?;
    ensure_count_at_most(items.len(), MAX_JOB_PORTS, label)?;
    let mut ports = Vec::with_capacity(items.len());
    for item in items.iter() {
        let port = crate::preserves_rail::value_to_iovalue(item);
        let fields = simple_record(&port, "port", 2)?;
        let name = required_string(&fields[0], "port name")?;
        validate_non_empty(&name, "port name")?;
        push_bounded(&mut ports, name, MAX_JOB_PORTS, label)?;
    }
    Ok(ports)
}

fn parse_ref_sequence_value(value: &Value<IoValue>, label: &str) -> Result<Vec<String>> {
    let items = required_sequence(value, label)?;
    ensure_count_at_most(items.len(), MAX_JOB_REFS, label)?;
    let mut refs = Vec::with_capacity(items.len());
    for item in items.iter() {
        push_bounded(&mut refs, required_ref(item, label)?, MAX_JOB_REFS, label)?;
    }
    Ok(refs)
}

fn sequence_items(value: &Value<IoValue>, label: &str) -> Result<Vec<IoValue>> {
    let items = required_sequence(value, label)?;
    ensure_count_at_most(items.len(), MAX_JOB_STAGE_VALUES, label)?;
    let mut values = Vec::with_capacity(items.len());
    for item in items.iter() {
        push_bounded(&mut values, crate::preserves_rail::value_to_iovalue(item), MAX_JOB_STAGE_VALUES, label)?;
    }
    Ok(values)
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

fn required_ref(value: &Value<IoValue>, field: &str) -> Result<String> {
    let value = required_string(value, field)?;
    validate_ref(&value, field)?;
    Ok(value)
}

fn required_u64_value(value: &IoValue, field: &str) -> Result<u64> {
    value
        .as_u64()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected u64 for {field}")))?
        .map_err(|error| MoltenError::invalid_harness(format!("u64 out of range for {field}: {error}")))
}

fn usize_to_u64(value: usize, field: &str) -> Result<u64> {
    u64::try_from(value).map_err(|error| MoltenError::invalid_harness(format!("{field} out of range: {error}")))
}

fn ensure_count_at_most(actual: usize, maximum: usize, label: &str) -> Result<()> {
    if actual <= maximum {
        return Ok(());
    }
    Err(MoltenError::invalid_harness(format!("{label} count {actual} exceeds bound {maximum}")))
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

fn extend_cloned_bounded<T: Clone>(
    values: &mut impl crate::bounded::VecSink<T>,
    incoming: &[T],
    maximum: usize,
    label: &str,
) -> Result<()> {
    let final_count = checked_count_sum(values.item_count(), incoming.len(), maximum, label)?;
    values.reserve_items(final_count.saturating_sub(values.item_count()));
    values.extend_cloned_items(incoming);
    Ok(())
}

fn insert_bounded<K: Ord, V>(
    values: &mut OrderedMap<K, V>,
    key: K,
    value: V,
    maximum: usize,
    label: &str,
) -> Result<Option<V>> {
    if !values.contains_key(&key) {
        checked_count_sum(values.len(), 1, maximum, label)?;
    }
    Ok(values.insert(key, value))
}

fn optional_ref_value(value: Option<&str>) -> IoValue {
    value.map_or_else(
        || crate::preserves_rail::record("none", Vec::new()),
        |value| crate::preserves_rail::record("some", vec![crate::preserves_rail::string(value)]),
    )
}

fn optional_string_value(value: Option<&str>) -> IoValue {
    value.map_or_else(
        || crate::preserves_rail::record("none", Vec::new()),
        |value| crate::preserves_rail::record("some", vec![crate::preserves_rail::string(value)]),
    )
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

fn parse_optional_string_value(value: &Value<IoValue>) -> Result<Option<String>> {
    if value.collect_simple_record("none", Some(0)).is_some() {
        return Ok(None);
    }
    if let Some(some) = value.collect_simple_record("some", Some(1)) {
        return required_string(&some[0], "optional string").map(Some);
    }
    required_string(value, "optional string").map(Some)
}

fn refs_sequence(refs: &[String]) -> IoValue {
    crate::preserves_rail::sequence(refs.iter().map(crate::preserves_rail::string).collect())
}

fn ports_sequence(ports: &[String]) -> IoValue {
    crate::preserves_rail::sequence(
        ports
            .iter()
            .map(|port| {
                crate::preserves_rail::record("port", vec![
                    crate::preserves_rail::string(port),
                    crate::preserves_rail::record("none", Vec::new()),
                ])
            })
            .collect(),
    )
}

fn checks_value(names: &[&str]) -> IoValue {
    checks_value_from_pairs(&names.iter().map(|name| (*name, "pass")).collect::<Vec<_>>())
}

fn checks_value_from_pairs(checks: &[(&str, &str)]) -> IoValue {
    crate::preserves_rail::record("checks", vec![crate::preserves_rail::sequence(
        checks
            .iter()
            .map(|(name, status)| {
                crate::preserves_rail::record("check", vec![
                    crate::preserves_rail::string(name),
                    crate::preserves_rail::string(status),
                ])
            })
            .collect(),
    )])
}

fn parse_checks(value: &Value<IoValue>) -> Result<Vec<String>> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let checks = simple_record(&value, "checks", 1)?;
    let items = required_sequence(&checks[0], "checks")?;
    ensure_count_at_most(items.len(), MAX_JOB_CHECKS, "checks")?;
    let mut parsed = Vec::with_capacity(items.len());
    for item in items.iter() {
        let item = crate::preserves_rail::value_to_iovalue(item);
        let check = simple_record(&item, "check", 2)?;
        let name = required_string(&check[0], "check name")?;
        let status = required_string(&check[1], "check status")?;
        if status != "pass" && status != "fail" {
            return Err(MoltenError::invalid_harness(format!("job check {name} has status {status}")));
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

fn require_schema(value: &Value<IoValue>, expected: &str, context: &str) -> Result<()> {
    let actual = required_string(value, context)?;
    if actual == expected {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("unsupported {context} schema {actual}; expected {expected}")))
    }
}

fn record_sequence_values(value: &Value<IoValue>, label: &str) -> Result<Vec<IoValue>> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    let items = required_sequence(&record[0], label)?;
    ensure_count_at_most(items.len(), MAX_JOB_REFS, label)?;
    let mut values = Vec::with_capacity(items.len());
    for item in items.iter() {
        push_bounded(&mut values, crate::preserves_rail::value_to_iovalue(item), MAX_JOB_REFS, label)?;
    }
    Ok(values)
}

fn parse_job_content_ref_record(value: &Value<IoValue>, label: &str) -> Result<JobContentRef> {
    parse_job_content_ref_value(&record_iovalue(value, label)?)
}

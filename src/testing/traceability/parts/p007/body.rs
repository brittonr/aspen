fn ensure_count_at_most(count: usize, maximum: usize, label: &str) -> Result<()> {
    if count > maximum {
        Err(MoltenError::invalid_harness(format!("{label} count {count} exceeds maximum {maximum}")))
    } else {
        Ok(())
    }
}

fn checked_add_count(left: usize, right: usize, label: &str) -> Result<usize> {
    left.checked_add(right)
        .ok_or_else(|| MoltenError::invalid_harness(format!("{label} count overflow")))
}

fn checked_mul_count(left: usize, right: usize, label: &str) -> Result<usize> {
    left.checked_mul(right)
        .ok_or_else(|| MoltenError::invalid_harness(format!("{label} count overflow")))
}

fn aggregate_proof_diagnostic_bound(input: &AggregateProofInput) -> Result<usize> {
    let obligation_diagnostics = checked_mul_count(
        input.obligations.len(),
        MAX_AGGREGATE_DIAGNOSTICS_PER_OBLIGATION,
        "aggregate proof diagnostics",
    )?;
    let with_missing = checked_add_count(obligation_diagnostics, 1, "aggregate proof diagnostics")?;
    checked_add_count(with_missing, input.required_obligation_ids.len(), "aggregate proof diagnostics")
}

fn layered_proof_diagnostic_bound(layers: &[ProofLayerInput]) -> Result<usize> {
    let layer_diagnostics =
        checked_mul_count(layers.len(), MAX_LAYER_DIRECT_DIAGNOSTICS_PER_LAYER, "layered proof diagnostics")?;
    let child_count = layers
        .iter()
        .try_fold(0usize, |count, layer| checked_add_count(count, layer.child_ids.len(), "proof layer child ids"))?;
    let child_diagnostics =
        checked_mul_count(child_count, MAX_LAYER_LINK_DIAGNOSTICS_PER_CHILD, "layered proof diagnostics")?;
    let with_missing = checked_add_count(layer_diagnostics, 1, "layered proof diagnostics")?;
    let with_children = checked_add_count(with_missing, child_diagnostics, "layered proof diagnostics")?;
    checked_add_count(with_children, layers.len(), "layered proof diagnostics")
}

fn deny_path_diagnostic_bound(case_count: usize, required_count: usize) -> Result<usize> {
    let case_diagnostics =
        checked_mul_count(case_count, MAX_DENY_DIRECT_DIAGNOSTICS_PER_CASE, "deny path diagnostics")?;
    checked_add_count(case_diagnostics, required_count, "deny path diagnostics")
}

trait PushLimited<T> {
    fn push_limited(&mut self, value: T, maximum: usize, label: &str) -> Result<()>;
}

impl<T, S> PushLimited<T> for S
where S: VecSink<T>
{
    fn push_limited(&mut self, value: T, maximum: usize, label: &str) -> Result<()> {
        let next = checked_add_count(self.item_count(), 1, label)?;
        ensure_count_at_most(next, maximum, label)?;
        self.push_item(value);
        Ok(())
    }
}

fn render_group_line(label: &str, ids: &[String]) -> String {
    if ids.is_empty() {
        format!("{label}: none")
    } else {
        format!("{label}: {}", ids.join(", "))
    }
}

fn display_group(values: &[String]) -> String {
    if values.is_empty() {
        "none".to_string()
    } else {
        values.join(", ")
    }
}

fn optional_ref_value(value: Option<&str>) -> IoValue {
    value.map_or_else(|| record("none", Vec::new()), |value| record("some", vec![string(value)]))
}

fn optional_string_value(value: Option<&str>) -> IoValue {
    value.map_or_else(|| record("none", Vec::new()), |value| record("some", vec![string(value)]))
}

fn require_schema(value: &Value<IoValue>, expected: &str, label: &str) -> Result<()> {
    let actual = value
        .as_string()
        .map(|value| value.into_owned())
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected schema string for {label}")))?;
    if actual == expected {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("{label} schema {actual} did not match {expected}")))
    }
}

fn record_string(value: &Value<IoValue>, label: &str) -> Result<String> {
    let record = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...> field")))?;
    required_string(&record[0], label)
}

fn record_ref(value: &Value<IoValue>, label: &str) -> Result<String> {
    let reference = record_string(value, label)?;
    validate_ref(&reference, label)?;
    Ok(reference)
}

fn record_string_sequence(value: &Value<IoValue>, label: &str) -> Result<Vec<String>> {
    let record = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...> field")))?;
    let values = record[0]
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected sequence for {label}")))?;
    values.iter().map(|value| required_string(value, label)).collect()
}

fn record_ref_sequence(value: &Value<IoValue>, label: &str) -> Result<Vec<String>> {
    let values = record_string_sequence(value, label)?;
    for reference in &values {
        validate_ref(reference, label)?;
    }
    Ok(values)
}

fn record_i64(value: &Value<IoValue>, label: &str) -> Result<i64> {
    let record = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...> field")))?;
    record[0]
        .as_i64()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected signed integer for {label}")))?
        .map_err(|_| MoltenError::invalid_harness(format!("signed integer for {label} is out of i64 range")))
}

fn required_string(value: &Value<IoValue>, field: &str) -> Result<String> {
    value
        .as_string()
        .map(|value| value.into_owned())
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected string for {field}")))
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


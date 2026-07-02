
pub fn parse_node_health_receipt(value: &IoValue) -> Result<NodeHealthReceipt> {
    let fields = value
        .collect_simple_record("node-health-receipt-v1", Some(11))
        .ok_or_else(|| MoltenError::invalid_harness("expected <node-health-receipt-v1 ...>"))?;
    require_schema(&fields[0], crate::preserves_rail::NODE_HEALTH_RECEIPT_SCHEMA, "node health receipt")?;
    let checks = parse_checks(&fields[10])?;
    require_check(&checks, "canonical-receipt", "node health receipt")?;
    Ok(NodeHealthReceipt {
        receipt_ref: canonical_hash(value)?,
        decision: record_string(&fields[1], "decision")?,
        startup_receipt_ref: record_ref(&fields[2], "startup")?,
        shutdown_receipt_ref: record_optional_ref(&fields[3], "shutdown")?,
        adapters: parse_adapter_receipt_refs(&fields[4])?,
        index_receipt_refs: record_ref_sequence(&fields[5], "indexes")?,
        head_refs: record_ref_sequence(&fields[6], "heads")?,
        open_job_refs: record_ref_sequence(&fields[7], "open-jobs")?,
        replay_status: record_string(&fields[8], "replay")?,
        diagnostics: record_string_sequence(&fields[9], "diagnostics")?,
        checks,
        value: value.clone(),
    })
}

pub fn node_restart_health_receipt_value(input: &RestartHealthReceiptValueInput<'_>) -> Result<IoValue> {
    let mut health_diagnostics = input.diagnostics.to_vec();
    if input.startup_receipt.decision != "pass" {
        health_diagnostics.push("previous startup receipt did not pass".to_string());
    }
    if input.shutdown_receipt_ref.is_none() {
        health_diagnostics.push("previous shutdown receipt missing".to_string());
    }
    if input.index_receipt_refs.is_empty() {
        health_diagnostics.push("adapter indexes not verified on restart".to_string());
    }
    if !input.open_job_refs.is_empty() {
        health_diagnostics.push("restart has open jobs; replay not eligible".to_string());
    }
    let is_replay_eligible = health_diagnostics.is_empty();
    let decision = if is_replay_eligible { "pass" } else { "deny" };
    node_health_receipt_value(&HealthReceiptValueInput {
        decision,
        startup_receipt_ref: &input.startup_receipt.receipt_ref,
        shutdown_receipt_ref: input.shutdown_receipt_ref,
        adapter_receipts: &input.startup_receipt.adapters,
        index_receipt_refs: input.index_receipt_refs,
        head_refs: input.head_refs,
        open_job_refs: input.open_job_refs,
        replay_is_eligible: is_replay_eligible,
        diagnostics: &health_diagnostics,
    })
}

pub fn parse_node_startup_receipt(value: &IoValue) -> Result<NodeStartupReceipt> {
    let fields = value
        .collect_simple_record("node-startup-receipt-v1", Some(13))
        .ok_or_else(|| MoltenError::invalid_harness("expected <node-startup-receipt-v1 ...>"))?;
    require_schema(&fields[0], NODE_STARTUP_RECEIPT_SCHEMA, "node startup receipt")?;
    let checks = parse_checks(&fields[12])?;
    require_check(&checks, "canonical-receipt", "node startup receipt")?;
    Ok(NodeStartupReceipt {
        receipt_ref: canonical_hash(value)?,
        decision: record_string(&fields[1], "decision")?,
        config_ref: record_ref(&fields[2], "node-config")?,
        identity_receipt_ref: record_ref(&fields[3], "identity")?,
        adapters: parse_adapter_receipt_refs(&fields[4])?,
        policy_refs: record_ref_sequence(&fields[5], "policy")?,
        source_gate_receipt_refs: record_ref_sequence(&fields[6], "source-gates")?,
        source_gate_validation_refs: record_ref_sequence(&fields[7], "source-gate-validations")?,
        capability_receipt_refs: record_ref_sequence(&fields[8], "capability")?,
        resource_receipt_refs: record_ref_sequence(&fields[9], "resource")?,
        version_refs: record_ref_sequence(&fields[10], "version")?,
        diagnostics: record_string_sequence(&fields[11], "diagnostics")?,
        checks,
        value: value.clone(),
    })
}

fn deterministic_adapter_order(adapters: &[NodeAdapterBinding]) -> Vec<String> {
    deterministic_adapter_bindings(adapters).into_iter().map(|adapter| adapter.name).collect()
}

fn deterministic_adapter_bindings(adapters: &[NodeAdapterBinding]) -> Vec<NodeAdapterBinding> {
    let mut adapters = adapters.to_vec();
    adapters.sort_by(|left, right| adapter_sort_key(&left.name).cmp(&adapter_sort_key(&right.name)));
    adapters
}

fn adapter_sort_key(name: &str) -> (bool, usize, &str) {
    match REQUIRED_RUNTIME_ADAPTERS.iter().position(|required| required == &name) {
        Some(rank) => (false, rank, name),
        None => (true, 0, name),
    }
}

fn missing_required_adapters(adapters: &[NodeAdapterBinding]) -> Vec<String> {
    REQUIRED_RUNTIME_ADAPTERS
        .iter()
        .filter(|required| !adapters.iter().any(|adapter| adapter.name == **required))
        .map(|required| (*required).to_string())
        .collect()
}

fn ensure_unique_adapter_names(adapters: &[NodeAdapterBinding]) -> Result<()> {
    for (index, adapter) in adapters.iter().enumerate() {
        if adapters.iter().skip(index + 1).any(|other| other.name == adapter.name) {
            return Err(MoltenError::invalid_harness(format!("duplicate node adapter name {}", adapter.name)));
        }
    }
    Ok(())
}

fn adapter_binding_value(binding: &NodeAdapterBinding) -> IoValue {
    record("adapter", vec![string(&binding.name), string(&binding.profile_ref)])
}

fn adapter_receipt_ref_value(receipt: &NodeAdapterReceiptRef) -> IoValue {
    record("adapter", vec![string(&receipt.name), string(&receipt.receipt_ref)])
}

fn parse_adapter_bindings(value: &Value<IoValue>) -> Result<Vec<NodeAdapterBinding>> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, "adapters", 1)?;
    let items = required_sequence(&record[0], "node adapters")?;
    ensure_count_at_most(items.len(), MAX_NODE_ADAPTERS, "node adapters")?;
    let mut adapters = Vec::with_capacity(items.len());
    for item in items.iter() {
        let item = value_to_iovalue(item);
        let fields = simple_record(&item, "adapter", 2)?;
        adapters.push(node_adapter_binding(
            &required_string(&fields[0], "adapter name")?,
            &required_ref(&fields[1], "adapter profile")?,
        )?);
    }
    ensure_unique_adapter_names(&adapters)?;
    Ok(adapters)
}

fn parse_adapter_receipt_refs(value: &Value<IoValue>) -> Result<Vec<NodeAdapterReceiptRef>> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, "adapters", 1)?;
    let items = required_sequence(&record[0], "node adapter receipt refs")?;
    ensure_count_at_most(items.len(), MAX_NODE_ADAPTERS, "node adapter receipt refs")?;
    let mut adapters = Vec::with_capacity(items.len());
    for item in items.iter() {
        let item = value_to_iovalue(item);
        let fields = simple_record(&item, "adapter", 2)?;
        let name = required_string(&fields[0], "adapter name")?;
        validate_adapter_name(&name)?;
        adapters.push(NodeAdapterReceiptRef {
            name,
            receipt_ref: required_ref(&fields[1], "adapter receipt ref")?,
        });
    }
    Ok(adapters)
}

fn record_string(value: &Value<IoValue>, label: &str) -> Result<String> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    required_string(&record[0], label)
}

fn record_ref(value: &Value<IoValue>, label: &str) -> Result<String> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    required_ref(&record[0], label)
}

fn record_optional_ref(value: &Value<IoValue>, label: &str) -> Result<Option<String>> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    parse_optional_ref_value(&record[0])
}

fn record_ref_sequence(value: &Value<IoValue>, label: &str) -> Result<Vec<String>> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    let items = required_sequence(&record[0], label)?;
    let mut refs = Vec::with_capacity(items.len());
    for item in items.iter() {
        refs.push(required_ref(item, label)?);
    }
    Ok(refs)
}

fn record_string_sequence(value: &Value<IoValue>, label: &str) -> Result<Vec<String>> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    let items = required_sequence(&record[0], label)?;
    let mut strings = Vec::with_capacity(items.len());
    for item in items.iter() {
        strings.push(required_string(item, label)?);
    }
    Ok(strings)
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

fn refs_sequence(refs: &[String]) -> IoValue {
    sequence(refs.iter().map(string).collect())
}

fn optional_ref_value(value: Option<&str>) -> IoValue {
    value.map_or_else(|| record("none", Vec::new()), |value| record("some", vec![string(value)]))
}

fn parse_optional_ref_value(value: &Value<IoValue>) -> Result<Option<String>> {
    if value.collect_simple_record("none", Some(0)).is_some() {
        return Ok(None);
    }
    if let Some(fields) = value.collect_simple_record("some", Some(1)) {
        return required_ref(&fields[0], "optional ref").map(Some);
    }
    required_ref(value, "optional ref").map(Some)
}

fn checks_value(checks: &[(&str, &str)]) -> IoValue {
    record("checks", vec![sequence(
        checks.iter().map(|(name, status)| record("check", vec![string(name), string(status)])).collect(),
    )])
}

fn parse_checks(value: &Value<IoValue>) -> Result<Vec<String>> {
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
            return Err(MoltenError::invalid_harness(format!("node runtime check {name} has status {status}")));
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

fn validate_adapter_name(name: &str) -> Result<()> {
    validate_non_empty(name, "node adapter name")?;
    if name.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_') {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!(
            "node adapter name {name} must use ascii alphanumeric, '-', or '_'"
        )))
    }
}

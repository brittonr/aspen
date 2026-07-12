
fn control_live_transport_receipt_path(
    envelope_ref: &str,
    operation: &str,
) -> Result<crate::node_state::NodeStatePath> {
    ingress_receipt_leaf(&format!(
        "{}.live-{}.receipt.preserves",
        ref_file_stem(envelope_ref),
        operation
    ))
}

fn control_live_send_receipt_path(send_ref: &str) -> Result<crate::node_state::NodeStatePath> {
    ingress_receipt_leaf(&format!("{}.live-send.receipt.preserves", ref_file_stem(send_ref)))
}

fn control_live_send_retry_receipt_path(retry_ref: &str) -> Result<crate::node_state::NodeStatePath> {
    ingress_receipt_leaf(&format!(
        "{}.live-send-retry.receipt.preserves",
        ref_file_stem(retry_ref)
    ))
}

fn control_live_send_duplicate_receipt_path(duplicate_ref: &str) -> Result<crate::node_state::NodeStatePath> {
    ingress_receipt_leaf(&format!(
        "{}.live-send-duplicate.receipt.preserves",
        ref_file_stem(duplicate_ref)
    ))
}

fn control_live_workflow_receipt_path(workflow_ref: &str) -> Result<crate::node_state::NodeStatePath> {
    ingress_receipt_leaf(&format!(
        "{}.live-workflow.receipt.preserves",
        ref_file_stem(workflow_ref)
    ))
}

fn control_live_listener_receipt_path(listener_ref: &str) -> Result<crate::node_state::NodeStatePath> {
    node_leaf_path(
        CONTROL_SERVICE_DIR,
        &format!("{}.live-listener-receipt.preserves", ref_file_stem(listener_ref)),
    )
}

fn control_authority_receipt_path(envelope_ref: &str) -> Result<crate::node_state::NodeStatePath> {
    ingress_receipt_leaf(&format!(
        "{}.authority-receipt.preserves",
        ref_file_stem(envelope_ref)
    ))
}

fn ingress_receipt_leaf(leaf: &str) -> Result<crate::node_state::NodeStatePath> {
    fixed_node_path(CONTROL_INGRESS_DIR)?.join("receipts")?.join_segment(leaf)
}

fn ref_file_stem(value_ref: &str) -> String {
    value_ref.replace(':', "-")
}

fn optional_string(value: Option<&str>) -> IoValue {
    match value {
        Some(value) => crate::preserves_rail::record("some", vec![crate::preserves_rail::string(value)]),
        None => crate::preserves_rail::record("none", Vec::new()),
    }
}

fn optional_value(value: Option<&IoValue>) -> IoValue {
    match value {
        Some(value) => crate::preserves_rail::record("some", vec![value.clone()]),
        None => crate::preserves_rail::record("none", Vec::new()),
    }
}

fn diagnostics_include(diagnostics: &[String], needle: &str) -> bool {
    diagnostics.iter().any(|diagnostic| diagnostic.contains(needle))
}

fn record_strings(value: &preserves::Value<preserves::IOValue>, tag: &str) -> Result<Vec<String>> {
    let record_value = crate::preserves_rail::value_to_iovalue(value);
    let fields = record_value
        .collect_simple_record(tag, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{tag} [...]>")))?;
    let items = fields[0]
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness(format!("{tag} must contain a sequence")))?
        .into_owned();
    let mut values = Vec::with_capacity(items.len());
    for item in items {
        let item = item
            .as_string()
            .map(|value| value.into_owned())
            .ok_or_else(|| MoltenError::invalid_harness(format!("{tag} sequence contains non-string")))?;
        values.push(item);
    }
    Ok(values)
}

fn record_optional_string(value: &preserves::Value<preserves::IOValue>, tag: &str) -> Result<Option<String>> {
    let record_value = crate::preserves_rail::value_to_iovalue(value);
    let fields = record_value
        .collect_simple_record(tag, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{tag} optional>")))?;
    let inner = crate::preserves_rail::value_to_iovalue(&fields[0]);
    if inner.collect_simple_record("none", Some(0)).is_some() {
        return Ok(None);
    }
    let some = inner
        .collect_simple_record("some", Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("{tag} must contain <some string> or <none>")))?;
    let value = some[0]
        .as_string()
        .map(|value| value.into_owned())
        .ok_or_else(|| MoltenError::invalid_harness(format!("{tag} <some> must contain a string")))?;
    Ok(Some(value))
}

fn record_optional_value(value: &preserves::Value<preserves::IOValue>, tag: &str) -> Result<Option<IoValue>> {
    let record_value = crate::preserves_rail::value_to_iovalue(value);
    let fields = record_value
        .collect_simple_record(tag, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{tag} optional>")))?;
    let inner = crate::preserves_rail::value_to_iovalue(&fields[0]);
    if inner.collect_simple_record("none", Some(0)).is_some() {
        return Ok(None);
    }
    let some = inner
        .collect_simple_record("some", Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("{tag} must contain <some value> or <none>")))?;
    Ok(Some(crate::preserves_rail::value_to_iovalue(&some[0])))
}

fn record_optional_ref_string(value: &preserves::Value<preserves::IOValue>, tag: &str) -> Result<Option<String>> {
    let reference = record_optional_string(value, tag)?;
    if let Some(reference) = reference.as_ref() {
        validate_ingress_ref(reference, tag)?;
    }
    Ok(reference)
}

fn record_optional_u64_string(value: &preserves::Value<preserves::IOValue>, tag: &str) -> Result<Option<u64>> {
    match record_optional_string(value, tag)? {
        Some(value) => value.parse::<u64>().map(Some).map_err(|_| {
            MoltenError::invalid_harness(format!("{tag} optional value must contain an unsigned integer string"))
        }),
        None => Ok(None),
    }
}

fn validate_decision(decision: &str) -> Result<()> {
    if matches!(decision, "pass" | "deny" | "fail") {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("invalid node control decision `{decision}`")))
    }
}

fn validate_live_send_timeout(timeout_ms: u64) -> Result<()> {
    if timeout_ms == 0 {
        return Err(MoltenError::invalid_harness("node control live send timeout must be positive"));
    }
    if timeout_ms > MAX_CONTROL_LIVE_SEND_TIMEOUT_MS {
        return Err(MoltenError::invalid_harness(format!(
            "node control live send timeout exceeds bounded limit {MAX_CONTROL_LIVE_SEND_TIMEOUT_MS}"
        )));
    }
    Ok(())
}

fn validate_live_send_attempts(max_attempts: u64) -> Result<()> {
    if max_attempts == 0 {
        return Err(MoltenError::invalid_harness("node control live send attempts must be positive"));
    }
    if max_attempts > MAX_CONTROL_LIVE_SEND_ATTEMPTS {
        return Err(MoltenError::invalid_harness(format!(
            "node control live send attempts exceed bounded limit {MAX_CONTROL_LIVE_SEND_ATTEMPTS}"
        )));
    }
    Ok(())
}

fn validate_listener_event_limit(max_events: u64) -> Result<()> {
    if max_events > MAX_CONTROL_LIVE_LISTENER_EVENTS {
        return Err(MoltenError::invalid_harness(format!(
            "node control live listener max events exceeds bounded limit {MAX_CONTROL_LIVE_LISTENER_EVENTS}"
        )));
    }
    Ok(())
}

fn validate_supervisor_policy_bounds(value: u64, label: &str) -> Result<()> {
    if value > MAX_CONTROL_SERVICE_TICKS {
        return Err(MoltenError::invalid_harness(format!(
            "node control supervisor policy {label} exceeds bounded limit {MAX_CONTROL_SERVICE_TICKS}"
        )));
    }
    Ok(())
}

fn validate_service_tick_limit(max_ticks: u64) -> Result<usize> {
    if max_ticks == 0 {
        return Err(MoltenError::invalid_harness("node control service max ticks must be positive"));
    }
    if max_ticks > MAX_CONTROL_SERVICE_TICKS {
        return Err(MoltenError::invalid_harness(format!(
            "node control service max ticks exceeds bounded limit {MAX_CONTROL_SERVICE_TICKS}"
        )));
    }
    usize::try_from(max_ticks)
        .map_err(|_| MoltenError::invalid_harness("node control service max ticks does not fit this platform"))
}

fn validate_loop_request_limit(max_requests: u64) -> Result<usize> {
    if max_requests == 0 {
        return Err(MoltenError::invalid_harness("node control loop max requests must be positive"));
    }
    if max_requests > MAX_CONTROL_LOOP_REQUESTS {
        return Err(MoltenError::invalid_harness(format!(
            "node control loop max requests exceeds bounded limit {MAX_CONTROL_LOOP_REQUESTS}"
        )));
    }
    usize::try_from(max_requests)
        .map_err(|_| MoltenError::invalid_harness("node control loop max requests does not fit this platform"))
}

fn record_string(value: &preserves::Value<preserves::IOValue>, tag: &str) -> Result<String> {
    let record_value = crate::preserves_rail::value_to_iovalue(value);
    let fields = record_value
        .collect_simple_record(tag, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{tag} string>")))?;
    fields[0]
        .as_string()
        .map(|value| value.into_owned())
        .ok_or_else(|| MoltenError::invalid_harness(format!("{tag} must contain a string")))
}

fn record_sequence_len(value: &preserves::Value<preserves::IOValue>, tag: &str) -> Result<usize> {
    let record_value = crate::preserves_rail::value_to_iovalue(value);
    let fields = record_value
        .collect_simple_record(tag, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{tag} sequence>")))?;
    fields[0]
        .collect_sequence()
        .map(|items| items.len())
        .ok_or_else(|| MoltenError::invalid_harness(format!("{tag} must contain a sequence")))
}

fn record_value(value: &preserves::Value<preserves::IOValue>, tag: &str) -> Result<IoValue> {
    let record_value = crate::preserves_rail::value_to_iovalue(value);
    let fields = record_value
        .collect_simple_record(tag, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{tag} value>")))?;
    Ok(crate::preserves_rail::value_to_iovalue(&fields[0]))
}

fn record_values(value: &preserves::Value<preserves::IOValue>, tag: &str) -> Result<Vec<IoValue>> {
    let record_value = crate::preserves_rail::value_to_iovalue(value);
    let fields = record_value
        .collect_simple_record(tag, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{tag} values>")))?;
    let items = fields[0]
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness(format!("{tag} must contain a sequence")))?;
    Ok(items.iter().map(crate::preserves_rail::value_to_iovalue).collect())
}

fn record_ref_string(value: &preserves::Value<preserves::IOValue>, tag: &str) -> Result<String> {
    let reference = record_string(value, tag)?;
    validate_ingress_ref(&reference, tag)?;
    Ok(reference)
}

fn record_ref_strings(value: &preserves::Value<preserves::IOValue>, tag: &str) -> Result<Vec<String>> {
    let record_value = crate::preserves_rail::value_to_iovalue(value);
    let fields = record_value
        .collect_simple_record(tag, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{tag} sequence>")))?;
    let items = fields[0]
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness(format!("{tag} must contain a sequence")))?;
    let mut refs = Vec::with_capacity(items.len());
    for item in items.iter() {
        let reference = item
            .as_string()
            .map(|value| value.into_owned())
            .ok_or_else(|| MoltenError::invalid_harness(format!("{tag} entries must be strings")))?;
        validate_ingress_ref(&reference, tag)?;
        refs.push(reference);
    }
    Ok(refs)
}

fn record_u64_string(value: &preserves::Value<preserves::IOValue>, tag: &str) -> Result<u64> {
    record_string(value, tag)?.parse::<u64>().map_err(|error| {
        MoltenError::invalid_harness(format!("{tag} must contain an unsigned integer string: {error}"))
    })
}

fn validate_ingress_refs(refs: &[String], label: &str) -> Result<()> {
    for reference in refs {
        validate_ingress_ref(reference, label)?;
    }
    Ok(())
}

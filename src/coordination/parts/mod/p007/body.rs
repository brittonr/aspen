
fn parse_checks(value: &Value<IoValue>) -> Result<Vec<(String, String)>> {
    let value = value_to_iovalue(value);
    let checks = simple_record(&value, "checks", 1)?;
    let items = required_sequence(&checks[0], "coordination checks")?;
    ensure_count_at_most(items.len(), MAX_COORDINATION_CHECKS, "coordination checks")?;
    let mut parsed = Vec::new();
    for item in items.iter() {
        let item = value_to_iovalue(item);
        let check = simple_record(&item, "check", 2)?;
        let name = required_string(&check[0], "coordination check name")?;
        let status = required_string(&check[1], "coordination check status")?;
        match status.as_str() {
            "pass" | "fail" | "diagnostic" => {
                parsed.push_limited((name, status), MAX_COORDINATION_CHECKS, "coordination checks")?
            }
            _ => return Err(MoltenError::invalid_harness("coordination check status must be pass/fail/diagnostic")),
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

fn record_u64(value: &Value<IoValue>, label: &str) -> Result<u64> {
    let value = value_to_iovalue(value);
    let fields = simple_record(&value, label, 1)?;
    required_u64(&fields[0], label)
}

fn record_ref(value: &Value<IoValue>, label: &str) -> Result<String> {
    let reference = record_string(value, label)?;
    validate_ref(&reference, label)?;
    Ok(reference)
}

fn record_optional_ref(value: &Value<IoValue>, label: &str) -> Result<Option<String>> {
    let value = value_to_iovalue(value);
    let fields = simple_record(&value, label, 1)?;
    let option = value_to_iovalue(&fields[0]);
    if option.collect_simple_record("none", Some(0)).is_some() {
        Ok(None)
    } else {
        let some = simple_record(&option, "some", 1)?;
        let reference = required_string(&some[0], label)?;
        validate_ref(&reference, label)?;
        Ok(Some(reference))
    }
}

fn record_optional_value(value: &Value<IoValue>, label: &str) -> Result<Option<IoValue>> {
    let value = value_to_iovalue(value);
    let fields = simple_record(&value, label, 1)?;
    let option = value_to_iovalue(&fields[0]);
    if option.collect_simple_record("none", Some(0)).is_some() {
        Ok(None)
    } else {
        let some = simple_record(&option, "some", 1)?;
        Ok(Some(value_to_iovalue(&some[0])))
    }
}

fn record_string_sequence(value: &Value<IoValue>, label: &str) -> Result<Vec<String>> {
    let value = value_to_iovalue(value);
    let fields = simple_record(&value, label, 1)?;
    let items = required_sequence(&fields[0], label)?;
    ensure_count_at_most(items.len(), MAX_COORDINATION_REFS, label)?;
    let mut values = Vec::new();
    for item in items.iter() {
        values.push_limited(required_string(item, label)?, MAX_COORDINATION_REFS, label)?;
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

fn required_u64(value: &Value<IoValue>, field: &str) -> Result<u64> {
    let number = value.as_u64().ok_or_else(|| MoltenError::invalid_harness(format!("expected u64 for {field}")))?;
    number.map_err(|_| MoltenError::invalid_harness(format!("u64 out of range for {field}")))
}

fn validate_request_input(input: &CoordinationRequestInput) -> Result<()> {
    validate_service(&input.service)?;
    validate_operation(&input.service, &input.operation)?;
    validate_key(&input.key)?;
    validate_session(&input.client_session)?;
    validate_ref(&input.operation_id_ref, "coordination operation id ref")?;
    validate_read_consistency_mode(&input.read_consistency_mode)?;
    validate_refs(&input.authority_refs, "coordination authority ref")?;
    validate_refs(&input.resource_refs, "coordination resource ref")?;
    validate_refs(&input.policy_refs, "coordination policy ref")?;
    Ok(())
}

fn validate_service_id(value: &str) -> Result<()> {
    validate_non_empty(value, "coordination service id")?;
    if value.starts_with("coordination:") {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness("coordination service id must start with coordination:"))
    }
}

fn validate_services(values: &[String]) -> Result<()> {
    ensure_count_at_most(values.len(), MAX_COORDINATION_SERVICES, "coordination services")?;
    if values.is_empty() {
        return Err(MoltenError::invalid_harness("coordination manifest requires at least one service"));
    }
    for value in values {
        validate_service(value)?;
    }
    Ok(())
}

fn validate_service(value: &str) -> Result<()> {
    match value {
        SERVICE_LOCK | SERVICE_QUEUE | SERVICE_SEMAPHORE | SERVICE_RATE_LIMIT | SERVICE_ELECTION | SERVICE_BARRIER
        | SERVICE_REGISTRY => Ok(()),
        _ => Err(MoltenError::invalid_harness(format!("unsupported coordination service {value}"))),
    }
}

fn validate_operation(service: &str, operation: &str) -> Result<()> {
    let is_valid = matches!(
        (service, operation),
        (SERVICE_LOCK, OP_ACQUIRE)
            | (SERVICE_LOCK, OP_RELEASE)
            | (SERVICE_LOCK, OP_READ)
            | (SERVICE_QUEUE, OP_ENQUEUE)
            | (SERVICE_QUEUE, OP_DEQUEUE)
            | (SERVICE_QUEUE, OP_READ)
            | (SERVICE_SEMAPHORE, OP_ACQUIRE)
            | (SERVICE_SEMAPHORE, OP_RELEASE)
            | (SERVICE_SEMAPHORE, OP_READ)
            | (SERVICE_RATE_LIMIT, OP_ACQUIRE)
            | (SERVICE_RATE_LIMIT, OP_READ)
            | (SERVICE_ELECTION, OP_ELECT)
            | (SERVICE_ELECTION, OP_READ)
            | (SERVICE_BARRIER, OP_ARRIVE)
            | (SERVICE_BARRIER, OP_READ)
            | (SERVICE_REGISTRY, OP_REGISTER)
            | (SERVICE_REGISTRY, OP_UNREGISTER)
            | (SERVICE_REGISTRY, OP_READ)
    );
    if is_valid {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!(
            "unsupported coordination operation {operation} for service {service}"
        )))
    }
}

fn validate_key(value: &str) -> Result<()> {
    validate_non_empty(value, "coordination key")?;
    ensure_count_at_most(value.len(), MAX_COORDINATION_KEY_LEN, "coordination key bytes")
}

fn validate_session(value: &str) -> Result<()> {
    validate_non_empty(value, "coordination client session")
}

fn validate_capacity(value: u64, label: &str) -> Result<()> {
    if value > 0 {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("{label} must be positive")))
    }
}

fn validate_decision(value: &str) -> Result<()> {
    match value {
        "pass" | "deny" => Ok(()),
        _ => Err(MoltenError::invalid_harness("coordination decision must be pass or deny")),
    }
}

fn validate_transition_kind(value: &str) -> Result<()> {
    match value {
        TRANSITION_KIND_ADVANCE
        | TRANSITION_KIND_DENY_PRESERVE
        | TRANSITION_KIND_DUPLICATE_REPLAY
        | TRANSITION_KIND_CONFLICTING_DUPLICATE
        | TRANSITION_KIND_READ_OBSERVE => Ok(()),
        _ => Err(MoltenError::invalid_harness(format!("unsupported coordination transition kind {value}"))),
    }
}

fn validate_receipt_transition(
    decision: &str,
    state_ref: &str,
    transition: ReceiptTransitionInput<'_>,
) -> Result<()> {
    validate_transition_kind(transition.kind)?;
    validate_ref(transition.before_state_ref, "coordination transition before state ref")?;
    if let Some(value) = transition.after_state_ref {
        validate_ref(value, "coordination transition after state ref")?;
    }
    if let Some(value) = transition.preserved_state_ref {
        validate_ref(value, "coordination transition preserved state ref")?;
    }
    validate_refs(transition.output_refs, "coordination transition output ref")?;
    if let Some(value) = transition.control_plane_intent_ref {
        validate_ref(value, "coordination transition control-plane intent ref")?;
    }
    if let Some(value) = transition.prior_receipt_ref {
        validate_ref(value, "coordination transition prior receipt ref")?;
    }
    match transition.kind {
        TRANSITION_KIND_ADVANCE => validate_advance_transition(decision, state_ref, transition),
        TRANSITION_KIND_DENY_PRESERVE | TRANSITION_KIND_CONFLICTING_DUPLICATE => {
            validate_preserved_transition(decision, state_ref, transition)
        }
        TRANSITION_KIND_DUPLICATE_REPLAY | TRANSITION_KIND_READ_OBSERVE => {
            validate_no_advance_transition(state_ref, transition)
        }
        _ => Err(MoltenError::invalid_harness("unsupported coordination transition kind")),
    }
}

fn validate_advance_transition(
    decision: &str,
    state_ref: &str,
    transition: ReceiptTransitionInput<'_>,
) -> Result<()> {
    if decision != "pass" {
        return Err(MoltenError::invalid_harness("advance transition requires pass decision"));
    }
    if transition.after_state_ref != Some(state_ref) || transition.preserved_state_ref.is_some() {
        return Err(MoltenError::invalid_harness("advance transition must bind after-state as receipt state"));
    }
    if transition.control_plane_intent_ref.is_none() {
        return Err(MoltenError::invalid_harness("advance transition must bind control-plane intent"));
    }
    Ok(())
}

fn validate_preserved_transition(
    decision: &str,
    state_ref: &str,
    transition: ReceiptTransitionInput<'_>,
) -> Result<()> {
    if decision != "deny" {
        return Err(MoltenError::invalid_harness("preserved transition requires deny decision"));
    }
    validate_no_advance_transition(state_ref, transition)
}

fn validate_no_advance_transition(state_ref: &str, transition: ReceiptTransitionInput<'_>) -> Result<()> {
    if transition.after_state_ref.is_some() || transition.preserved_state_ref != Some(state_ref) {
        return Err(MoltenError::invalid_harness("no-advance transition must bind preserved-state as receipt state"));
    }
    Ok(())
}

fn validate_read_consistency_mode(value: &str) -> Result<()> {
    match value {
        READ_CONSISTENCY_LINEARIZABLE | READ_CONSISTENCY_LOCAL_STALE => Ok(()),
        _ => Err(MoltenError::invalid_harness(format!("unsupported coordination read consistency mode {value}"))),
    }
}

fn validate_ref(value: &str, label: &str) -> Result<()> {
    validate_non_empty(value, label)?;
    validate_content_ref(value).map_err(|error| {
        MoltenError::invalid_harness(format!("{label} must be a canonical blake3 content ref: {error}"))
    })
}

fn validate_refs(values: &[String], label: &str) -> Result<()> {
    ensure_count_at_most(values.len(), MAX_COORDINATION_REFS, label)?;
    for value in values {
        validate_ref(value, label)?;
    }
    Ok(())
}

fn validate_non_empty(value: &str, label: &str) -> Result<()> {
    if value.is_empty() {
        Err(MoltenError::invalid_harness(format!("{label} must not be empty")))
    } else {
        Ok(())
    }
}

fn ensure_count_at_most(count: usize, maximum: usize, label: &str) -> Result<()> {
    crate::bounded::ensure_count_at_most(count, maximum, label)
}

fn vec_len_u64<T>(values: &[T]) -> Result<u64> {
    u64::try_from(values.len()).map_err(|_| MoltenError::invalid_harness("coordination vector length overflow"))
}

fn set_len_u64<T>(values: &OrderedSet<T>) -> Result<u64> {
    u64::try_from(values.len()).map_err(|_| MoltenError::invalid_harness("coordination set length overflow"))
}

fn fixture_ref(label: &str) -> String {
    content_ref_from_bytes(label.as_bytes())
}

#[cfg(test)]
mod tests {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/coordination/parts/mod/tests/m000/p000/body.rs"));
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/coordination/parts/mod/tests/m000/p001/body.rs"));
}


fn fixture_request(input: FixtureRequestInput<'_>) -> Result<IoValue> {
    let scope = crate::delivery_idempotency::control_command_scope_ref(
        &fixture_ref("coordination-group"),
        input.client_session,
    )?;
    let payload_ref = input.payload.as_ref().map_or_else(
        || fixture_ref("coordination-empty-payload"),
        |value| canonical_hash(value).unwrap_or_else(|_| fixture_ref("coordination-payload-error")),
    );
    let operation_id =
        crate::delivery_idempotency::derive_operation_id(crate::delivery_idempotency::OperationIdInput {
            scope_ref: scope,
            producer: input.client_session.to_string(),
            consumer: format!("coordination:{}:{}", input.service, input.key),
            sequence: input.sequence,
            intent: format!("{}:{}", input.service, input.operation),
            payload_ref,
            policy_refs: input.refs.policy_refs.to_vec(),
        })?;
    coordination_request_value(&CoordinationRequestInput {
        service: input.service.to_string(),
        operation: input.operation.to_string(),
        key: input.key.to_string(),
        client_session: input.client_session.to_string(),
        operation_id_ref: operation_id.operation_ref,
        read_consistency_mode: READ_CONSISTENCY_LINEARIZABLE.to_string(),
        payload: input.payload,
        authority_refs: input.refs.authority_refs.to_vec(),
        resource_refs: input.refs.resource_refs.to_vec(),
        policy_refs: input.refs.policy_refs.to_vec(),
    })
}

fn supported_services() -> Vec<String> {
    vec![
        SERVICE_LOCK.to_string(),
        SERVICE_QUEUE.to_string(),
        SERVICE_SEMAPHORE.to_string(),
        SERVICE_RATE_LIMIT.to_string(),
        SERVICE_ELECTION.to_string(),
        SERVICE_BARRIER.to_string(),
        SERVICE_REGISTRY.to_string(),
    ]
}

fn strings_sequence(values: &[String]) -> IoValue {
    sequence(values.iter().map(string).collect())
}

fn optional_value(value: Option<&IoValue>) -> IoValue {
    value.map_or_else(|| record("none", Vec::new()), |value| record("some", vec![value.clone()]))
}

fn optional_ref_value(value: Option<&str>) -> IoValue {
    value.map_or_else(|| record("none", Vec::new()), |value| record("some", vec![string(value)]))
}

fn checks_value(checks: &[(&str, &str)]) -> IoValue {
    record("checks", vec![sequence(
        checks.iter().map(|(name, status)| record("check", vec![string(name), string(status)])).collect(),
    )])
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

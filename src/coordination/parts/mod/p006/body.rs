
fn status_fact_for(
    state: &CoordinationState,
    manifest: &CoordinationServiceManifest,
    service: &str,
    key: &str,
) -> Result<IoValue> {
    match service {
        SERVICE_LOCK => Ok(state.locks.get(key).map_or_else(
            || record("lock-free", vec![string(key)]),
            |lock| {
                record("lock-held", vec![
                    string(key),
                    string(&lock.owner),
                    u64_value(lock.token),
                    string(&lock.token_ref),
                ])
            },
        )),
        SERVICE_QUEUE => {
            let depth = match state.queues.get(key) {
                Some(values) => vec_len_u64(values)?,
                None => 0,
            };
            Ok(record("queue-depth", vec![string(key), u64_value(depth)]))
        }
        SERVICE_SEMAPHORE => {
            let used = match state.semaphores.get(key) {
                Some(values) => set_len_u64(values)?,
                None => 0,
            };
            Ok(record("semaphore-available", vec![
                string(key),
                u64_value(manifest.semaphore_capacity.saturating_sub(used)),
            ]))
        }
        SERVICE_RATE_LIMIT => {
            let used = state.rates.get(key).copied().unwrap_or(0);
            Ok(record("rate-limit", vec![string(key), u64_value(used), u64_value(manifest.rate_limit)]))
        }
        SERVICE_ELECTION => Ok(state.elections.get(key).map_or_else(
            || record("no-leader", vec![string(key)]),
            |election| {
                record("leader", vec![
                    string(key),
                    string(&election.leader),
                    u64_value(election.token),
                    string(&election.token_ref),
                ])
            },
        )),
        SERVICE_BARRIER => Ok(state.barriers.get(key).map_or_else(
            || record("barrier", vec![string(key), string("waiting"), u64_value(manifest.barrier_parties)]),
            |barrier| {
                record("barrier", vec![
                    string(key),
                    string(if barrier.is_released { "released" } else { "waiting" }),
                    u64_value(barrier.required),
                ])
            },
        )),
        SERVICE_REGISTRY => Ok(state.registry.get(key).map_or_else(
            || record("service-unregistered", vec![string(key)]),
            |entry| {
                record("service-registered", vec![
                    string(key),
                    string(&entry.endpoint_ref),
                    string(&entry.evidence_ref),
                ])
            },
        )),
        _ => Err(MoltenError::invalid_harness("unsupported coordination status service")),
    }
}

fn collect_admission_diagnostics(
    runtime: &CoordinationRuntime,
    request: &CoordinationRequest,
    diagnostics: &mut impl PushLimited<String>,
) -> Result<()> {
    if runtime.manifest.services.iter().all(|service| service != &request.service) {
        diagnostics.push_limited(
            format!("coordination service {} not declared in manifest", request.service),
            MAX_COORDINATION_DIAGNOSTICS,
            "coordination diagnostics",
        )?;
    }
    if request.authority_refs.is_empty() {
        diagnostics.push_limited(
            "coordination request missing authority evidence".to_string(),
            MAX_COORDINATION_DIAGNOSTICS,
            "coordination diagnostics",
        )?;
    }
    if request.policy_refs.is_empty() {
        diagnostics.push_limited(
            "coordination request missing policy evidence".to_string(),
            MAX_COORDINATION_DIAGNOSTICS,
            "coordination diagnostics",
        )?;
    }
    if request.resource_refs.is_empty() {
        diagnostics.push_limited(
            "coordination request missing resource evidence".to_string(),
            MAX_COORDINATION_DIAGNOSTICS,
            "coordination diagnostics",
        )?;
    }
    if request.operation != OP_READ && request.operation_id_ref.is_empty() {
        diagnostics.push_limited(
            "coordination mutating request missing operation id".to_string(),
            MAX_COORDINATION_DIAGNOSTICS,
            "coordination diagnostics",
        )?;
    }
    Ok(())
}

fn snapshot_from_state(state: &CoordinationState) -> Result<CoordinationStateSnapshot> {
    let value = coordination_state_snapshot_value(state)?;
    parse_coordination_state_snapshot(&value)
}

struct EvidenceValuesInput<'a> {
    request: &'a CoordinationRequest,
    receipt: &'a CoordinationReceipt,
    token: Option<&'a FencingToken>,
    snapshot: &'a CoordinationStateSnapshot,
    assertions: &'a [CoordinationStatusAssertion],
    read: Option<&'a RaftReadReceipt>,
}

fn evidence_values_for(input: EvidenceValuesInput<'_>) -> Vec<IoValue> {
    let mut values = vec![
        input.request.value.clone(),
        input.snapshot.value.clone(),
        input.receipt.value.clone(),
    ];
    if let Some(token) = input.token {
        values.push(token.value.clone());
    }
    if let Some(read) = input.read {
        values.push(read.value.clone());
    }
    values.extend(input.assertions.iter().map(|assertion| assertion.value.clone()));
    values
}

fn retention_refs_for_state(state: &CoordinationState) -> Result<Vec<String>> {
    let mut refs = Vec::new();
    for lock in state.locks.values() {
        refs.push_limited(lock.token_ref.clone(), MAX_COORDINATION_REFS, "coordination retention refs")?;
    }
    for election in state.elections.values() {
        refs.push_limited(election.token_ref.clone(), MAX_COORDINATION_REFS, "coordination retention refs")?;
    }
    for entry in state.registry.values() {
        refs.push_limited(entry.endpoint_ref.clone(), MAX_COORDINATION_REFS, "coordination retention refs")?;
        refs.push_limited(entry.evidence_ref.clone(), MAX_COORDINATION_REFS, "coordination retention refs")?;
    }
    Ok(refs)
}

fn payload_token(request: &CoordinationRequest) -> Result<u64> {
    let payload = request
        .payload
        .as_ref()
        .ok_or_else(|| MoltenError::invalid_harness("coordination release requires token payload"))?;
    let fields = simple_record(payload, "token", 1)?;
    required_u64(&fields[0], "coordination token")
}

fn payload_text(request: &CoordinationRequest, label: &str) -> Result<String> {
    let payload = request
        .payload
        .as_ref()
        .ok_or_else(|| MoltenError::invalid_harness(format!("coordination request requires {label} payload")))?;
    simple_payload_text(payload, label)
}

fn simple_payload_text(payload: &IoValue, label: &str) -> Result<String> {
    let fields = simple_record(payload, label, 1)?;
    required_string(&fields[0], label)
}

fn payload_endpoint(request: &CoordinationRequest) -> Result<(String, String)> {
    let payload = request
        .payload
        .as_ref()
        .ok_or_else(|| MoltenError::invalid_harness("coordination registry register requires endpoint payload"))?;
    let fields = simple_record(payload, "endpoint", 2)?;
    let endpoint_ref = required_string(&fields[0], "coordination endpoint ref")?;
    let evidence_ref = required_string(&fields[1], "coordination endpoint evidence ref")?;
    validate_ref(&endpoint_ref, "coordination endpoint ref")?;
    validate_ref(&evidence_ref, "coordination endpoint evidence ref")?;
    Ok((endpoint_ref, evidence_ref))
}

fn coordination_namespace(service: &str) -> String {
    format!("{COORDINATION_NAMESPACE_PREFIX}:{service}")
}

struct CoordinationRefSlices<'a> {
    authority_refs: &'a [String],
    resource_refs: &'a [String],
    policy_refs: &'a [String],
}

struct FixtureRequestInput<'a> {
    service: &'a str,
    operation: &'a str,
    key: &'a str,
    client_session: &'a str,
    sequence: u64,
    payload: Option<IoValue>,
    refs: &'a CoordinationRefSlices<'a>,
}

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

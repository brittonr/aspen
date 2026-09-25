
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

// r[impl molten.coordination.local_stale_boundaries]
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
    if request.operation != OP_READ && request.read_consistency_mode == READ_CONSISTENCY_LOCAL_STALE {
        diagnostics.push_limited(
            "local-stale coordination read cannot authorize protected action".to_string(),
            MAX_COORDINATION_DIAGNOSTICS,
            "coordination diagnostics",
        )?;
    }
    let engine_gate = active_engine_epoch_gate(runtime, &request.operation)?;
    if engine_gate.decision != "pass" {
        for diagnostic in engine_gate.diagnostics {
            diagnostics.push_limited(diagnostic, MAX_COORDINATION_DIAGNOSTICS, "coordination diagnostics")?;
        }
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
        if let Ok(normalized) = crate::raft_control_plane::normalized_raft_read_receipt_value(
            read,
            crate::raft_control_plane::INITIAL_CONSENSUS_ENGINE_EPOCH,
        ) {
            values.push(normalized);
        }
    }
    values.extend(input.assertions.iter().map(|assertion| assertion.value.clone()));
    values
}

fn transition_output_refs(values: &[IoValue]) -> Result<Vec<String>> {
    let mut refs = Vec::new();
    for value in values {
        refs.push_limited(canonical_hash(value)?, MAX_COORDINATION_REFS, "coordination transition output refs")?;
    }
    Ok(refs)
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

fn active_engine_epoch(_runtime: &CoordinationRuntime) -> u64 {
    crate::raft_control_plane::INITIAL_CONSENSUS_ENGINE_EPOCH
}

// r[impl molten.coordination.engine_agnostic_evidence]
fn active_engine_epoch_gate(
    runtime: &CoordinationRuntime,
    operation: &str,
) -> Result<crate::raft_control_plane::ConsensusEngineEpochGateReceipt> {
    crate::raft_control_plane::consensus_engine_epoch_gate(&crate::raft_control_plane::ConsensusEngineEpochGateInput {
        operation: operation.to_string(),
        active_profile: runtime.raft.manifest.algorithm_profile.clone(),
        active_engine_epoch: active_engine_epoch(runtime),
        presented_profile: runtime.raft.manifest.algorithm_profile.clone(),
        presented_engine_epoch: active_engine_epoch(runtime),
        activation_receipt_ref: Some(runtime.raft.manifest.manifest_ref.clone()),
    })
}

// r[impl molten.coordination.engine_switchover_gates]
pub fn coordination_engine_epoch_admission(
    input: &crate::raft_control_plane::ConsensusEngineEpochGateInput,
) -> Result<crate::raft_control_plane::ConsensusEngineEpochGateReceipt> {
    crate::raft_control_plane::consensus_engine_epoch_gate(input)
}

fn engine_status_fact(
    manifest: &crate::raft_control_plane::RaftGroupManifest,
    engine_epoch: u64,
    fact: &IoValue,
) -> IoValue {
    record("engine-currentness", vec![
        record("profile", vec![string(&manifest.algorithm_profile)]),
        record("version", vec![string(&manifest.admitted_profile_version)]),
        record("engine-epoch", vec![u64_value(engine_epoch)]),
        record("currentness", vec![string(&manifest.manifest_ref)]),
        record("fact", vec![fact.clone()]),
    ])
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

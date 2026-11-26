fn prepare_election(runtime: &CoordinationRuntime, request: &CoordinationRequest, now: u64) -> Result<PreparedMutation> {
    let (leader, lease_payload) = election_candidacy(request)?;
    if let Some(existing) = runtime.state.elections.get(&request.key) {
        if existing.leader == leader {
            return Err(MoltenError::invalid_harness("coordination election already has same leader"));
        }
        return Err(MoltenError::invalid_harness(format!(
            "coordination election already led by {} until clock tick {}",
            existing.leader, existing.lease_expires_at
        )));
    }
    let lease = lease_expiry(lease_payload.as_ref(), now)?;
    let (state, claim) = claim_fencing_token(runtime, request, SERVICE_ELECTION, &leader, lease)?;
    Ok(PreparedMutation {
        state,
        status_fact: record("leader", vec![
            string(&request.key),
            string(&leader),
            u64_value(claim.token),
            u64_value(claim.lease_expires_at),
        ]),
        token_claim: Some(claim),
        checks: vec![("election-single-leader", "pass"), ("fencing-token-monotonic", "pass"), ("lease-bounded", "pass")],
    })
}

/// Elect payloads: none (the session stands), `<candidate name>`, `<lease ttl>` (the session stands), or
/// `<candidate name <lease ttl>>`.
fn election_candidacy(request: &CoordinationRequest) -> Result<(String, Option<IoValue>)> {
    let Some(payload) = request.payload.as_ref() else {
        return Ok((request.client_session.clone(), None));
    };
    if payload.collect_simple_record("lease", Some(1)).is_some() {
        return Ok((request.client_session.clone(), Some(payload.clone())));
    }
    if let Some(fields) = payload.collect_simple_record("candidate", Some(2)) {
        let candidate = required_string(&fields[0], "candidate")?;
        return Ok((candidate, Some(value_to_iovalue(&fields[1]))));
    }
    Ok((simple_payload_text(payload, "candidate")?, None))
}

// r[impl molten.coordination.truthful_tokens_assertions_and_expiry]
/// The current leader steps down by presenting its fencing token; any other session is denied.
fn prepare_election_resign(runtime: &CoordinationRuntime, request: &CoordinationRequest) -> Result<PreparedMutation> {
    let token = payload_token(request)?;
    let election = runtime
        .state
        .elections
        .get(&request.key)
        .ok_or_else(|| MoltenError::invalid_harness("coordination election has no leader"))?;
    if request.client_session != election.leader {
        return Err(MoltenError::invalid_harness(format!(
            "coordination election resignation by non-leader {}; leader is {}",
            request.client_session, election.leader
        )));
    }
    if token != election.token {
        return Err(MoltenError::invalid_harness(format!(
            "stale fencing token {token}; current token is {}",
            election.token
        )));
    }
    let mut state = runtime.state.clone();
    state.elections.remove(&request.key);
    Ok(PreparedMutation {
        state,
        token_claim: None,
        status_fact: record("leader-resigned", vec![string(&request.key), string(&election.leader), u64_value(token)]),
        checks: vec![("stale-token-checked", "pass"), ("election-resign", "pass")],
    })
}

fn prepare_barrier(runtime: &CoordinationRuntime, request: &CoordinationRequest) -> Result<PreparedMutation> {
    let participant = request
        .payload
        .as_ref()
        .map_or_else(|| Ok(request.client_session.clone()), |payload| simple_payload_text(payload, "participant"))?;
    let mut state = runtime.state.clone();
    let barrier = state.barriers.entry(request.key.clone()).or_insert_with(|| BarrierState {
        participants: OrderedSet::new(),
        required: runtime.manifest.barrier_parties,
        is_released: false,
    });
    ensure_count_at_most(
        barrier.participants.len().saturating_add(1),
        MAX_COORDINATION_ITEMS,
        "coordination barrier participants",
    )?;
    barrier.participants.insert(participant);
    barrier.is_released = set_len_u64(&barrier.participants)? >= barrier.required;
    let required = barrier.required;
    let status = if barrier.is_released { "released" } else { "waiting" };
    Ok(PreparedMutation {
        state,
        token_claim: None,
        status_fact: record("barrier", vec![string(&request.key), string(status), u64_value(required)]),
        checks: vec![("barrier-deterministic", "pass")],
    })
}

fn prepare_registry_register(runtime: &CoordinationRuntime, request: &CoordinationRequest) -> Result<PreparedMutation> {
    let (endpoint_ref, evidence_ref) = payload_endpoint(request)?;
    let mut state = runtime.state.clone();
    state.registry.insert(request.key.clone(), RegistryEntry {
        endpoint_ref: endpoint_ref.clone(),
        evidence_ref: evidence_ref.clone(),
    });
    Ok(PreparedMutation {
        state,
        token_claim: None,
        status_fact: record("service-registered", vec![
            string(&request.key),
            string(&endpoint_ref),
            string(&evidence_ref),
        ]),
        checks: vec![("service-registry-pointer", "pass"), ("control-plane-pointer", "pass")],
    })
}

fn prepare_registry_unregister(
    runtime: &CoordinationRuntime,
    request: &CoordinationRequest,
) -> Result<PreparedMutation> {
    let mut state = runtime.state.clone();
    if state.registry.remove(&request.key).is_some() {
        Ok(PreparedMutation {
            state,
            token_claim: None,
            status_fact: record("service-unregistered", vec![string(&request.key)]),
            checks: vec![("service-registry-pointer", "pass")],
        })
    } else {
        Err(MoltenError::invalid_harness("coordination registry entry missing"))
    }
}

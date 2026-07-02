
fn prepare_lock_acquire(runtime: &CoordinationRuntime, request: &CoordinationRequest) -> Result<PreparedMutation> {
    let mut diagnostics = Vec::new();
    if let Some(lock) = runtime.state.locks.get(&request.key) {
        diagnostics.push_limited(
            format!("coordination lock {} already held by {}", request.key, lock.owner),
            MAX_COORDINATION_DIAGNOSTICS,
            "coordination diagnostics",
        )?;
    }
    if diagnostics.is_empty() {
        let token_number = runtime.state.next_fencing_token;
        let placeholder = pending_token(&request.key, &request.client_session, token_number)?;
        let mut state = runtime.state.clone();
        let next = token_number
            .checked_add(1)
            .ok_or_else(|| MoltenError::invalid_harness("coordination fencing token overflow"))?;
        state.next_fencing_token = next;
        state.locks.insert(request.key.clone(), LockState {
            owner: request.client_session.clone(),
            token: token_number,
            token_ref: placeholder.token_ref.clone(),
            lease_epoch: token_number,
        });
        return Ok(PreparedMutation {
            state,
            token: Some(placeholder),
            status_fact: record("lock-held", vec![
                string(&request.key),
                string(&request.client_session),
                u64_value(token_number),
            ]),
            checks: vec![("fencing-token-monotonic", "pass"), ("lock-lease-held", "pass")],
        });
    }
    Err(MoltenError::invalid_harness(diagnostics.join("; ")))
}

fn prepare_lock_release(runtime: &CoordinationRuntime, request: &CoordinationRequest) -> Result<PreparedMutation> {
    let token = payload_token(request)?;
    let lock = runtime
        .state
        .locks
        .get(&request.key)
        .ok_or_else(|| MoltenError::invalid_harness("coordination lock is not held"))?;
    if token != lock.token {
        return Err(MoltenError::invalid_harness(format!(
            "stale fencing token {token}; current token is {}",
            lock.token
        )));
    }
    if request.client_session != lock.owner {
        return Err(MoltenError::invalid_harness("coordination lock release owner mismatch"));
    }
    let mut state = runtime.state.clone();
    state.locks.remove(&request.key);
    Ok(PreparedMutation {
        state,
        token: None,
        status_fact: record("lock-free", vec![string(&request.key), u64_value(token)]),
        checks: vec![("stale-token-checked", "pass"), ("lock-release", "pass")],
    })
}

fn prepare_queue_enqueue(runtime: &CoordinationRuntime, request: &CoordinationRequest) -> Result<PreparedMutation> {
    let item = payload_text(request, "item")?;
    let mut state = runtime.state.clone();
    let current_len = match state.queues.get(&request.key) {
        Some(values) => vec_len_u64(values)?,
        None => 0,
    };
    if current_len >= runtime.manifest.queue_capacity {
        return Err(MoltenError::invalid_harness("coordination queue overflow"));
    }
    let queue = state.queues.entry(request.key.clone()).or_default();
    ensure_count_at_most(queue.len().saturating_add(1), MAX_COORDINATION_ITEMS, "coordination queue items")?;
    queue.push(item.clone());
    Ok(PreparedMutation {
        state,
        token: None,
        status_fact: record("queue-depth", vec![string(&request.key), u64_value(current_len + 1), string(&item)]),
        checks: vec![("fifo-queue", "pass"), ("capacity-checked", "pass")],
    })
}

fn prepare_queue_dequeue(runtime: &CoordinationRuntime, request: &CoordinationRequest) -> Result<PreparedMutation> {
    let queue = runtime
        .state
        .queues
        .get(&request.key)
        .ok_or_else(|| MoltenError::invalid_harness("coordination queue empty"))?;
    let item = queue.first().cloned().ok_or_else(|| MoltenError::invalid_harness("coordination queue empty"))?;
    let mut state = runtime.state.clone();
    let remaining = state
        .queues
        .get_mut(&request.key)
        .ok_or_else(|| MoltenError::invalid_harness("coordination queue missing during dequeue"))?;
    remaining.remove(0);
    let depth = vec_len_u64(remaining)?;
    Ok(PreparedMutation {
        state,
        token: None,
        status_fact: record("queue-depth", vec![string(&request.key), u64_value(depth), string(&item)]),
        checks: vec![("fifo-queue", "pass"), ("dequeue-committed", "pass")],
    })
}

fn prepare_semaphore_acquire(runtime: &CoordinationRuntime, request: &CoordinationRequest) -> Result<PreparedMutation> {
    let mut state = runtime.state.clone();
    let holders = state.semaphores.entry(request.key.clone()).or_default();
    if holders.contains(&request.client_session) {
        return Err(MoltenError::invalid_harness("coordination semaphore already held by client"));
    }
    if set_len_u64(holders)? >= runtime.manifest.semaphore_capacity {
        return Err(MoltenError::invalid_harness("coordination semaphore exhausted"));
    }
    ensure_count_at_most(holders.len().saturating_add(1), MAX_COORDINATION_ITEMS, "coordination semaphore holders")?;
    holders.insert(request.client_session.clone());
    let available = runtime.manifest.semaphore_capacity.saturating_sub(set_len_u64(holders)?);
    Ok(PreparedMutation {
        state,
        token: None,
        status_fact: record("semaphore-available", vec![string(&request.key), u64_value(available)]),
        checks: vec![("semaphore-bounds", "pass"), ("capacity-checked", "pass")],
    })
}

fn prepare_semaphore_release(runtime: &CoordinationRuntime, request: &CoordinationRequest) -> Result<PreparedMutation> {
    let mut state = runtime.state.clone();
    let holders = state
        .semaphores
        .get_mut(&request.key)
        .ok_or_else(|| MoltenError::invalid_harness("coordination semaphore not held"))?;
    if holders.remove(&request.client_session) {
        let available = runtime.manifest.semaphore_capacity.saturating_sub(set_len_u64(holders)?);
        Ok(PreparedMutation {
            state,
            token: None,
            status_fact: record("semaphore-available", vec![string(&request.key), u64_value(available)]),
            checks: vec![("semaphore-bounds", "pass"), ("release-committed", "pass")],
        })
    } else {
        Err(MoltenError::invalid_harness("coordination semaphore release holder mismatch"))
    }
}

fn prepare_rate_acquire(runtime: &CoordinationRuntime, request: &CoordinationRequest) -> Result<PreparedMutation> {
    let used = runtime.state.rates.get(&request.key).copied().unwrap_or(0);
    if used >= runtime.manifest.rate_limit {
        return Err(MoltenError::invalid_harness("coordination rate limit exhausted"));
    }
    let mut state = runtime.state.clone();
    state.rates.insert(request.key.clone(), used + 1);
    Ok(PreparedMutation {
        state,
        token: None,
        status_fact: record("rate-limit", vec![
            string(&request.key),
            u64_value(used + 1),
            u64_value(runtime.manifest.rate_limit),
        ]),
        checks: vec![("rate-limit-bounds", "pass")],
    })
}

fn prepare_election(runtime: &CoordinationRuntime, request: &CoordinationRequest) -> Result<PreparedMutation> {
    let leader = request
        .payload
        .as_ref()
        .map_or_else(|| Ok(request.client_session.clone()), |payload| simple_payload_text(payload, "candidate"))?;
    if let Some(existing) = runtime.state.elections.get(&request.key) {
        if existing.leader == leader {
            return Err(MoltenError::invalid_harness("coordination election already has same leader"));
        }
        return Err(MoltenError::invalid_harness(format!("coordination election already led by {}", existing.leader)));
    }
    let token_number = runtime.state.next_fencing_token;
    let token = pending_token(&request.key, &leader, token_number)?;
    let mut state = runtime.state.clone();
    state.next_fencing_token = token_number
        .checked_add(1)
        .ok_or_else(|| MoltenError::invalid_harness("coordination election token overflow"))?;
    state.elections.insert(request.key.clone(), ElectionState {
        leader: leader.clone(),
        token: token_number,
        token_ref: token.token_ref.clone(),
    });
    Ok(PreparedMutation {
        state,
        token: Some(token),
        status_fact: record("leader", vec![string(&request.key), string(&leader), u64_value(token_number)]),
        checks: vec![("election-single-leader", "pass"), ("fencing-token-monotonic", "pass")],
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
        token: None,
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
        token: None,
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
            token: None,
            status_fact: record("service-unregistered", vec![string(&request.key)]),
            checks: vec![("service-registry-pointer", "pass")],
        })
    } else {
        Err(MoltenError::invalid_harness("coordination registry entry missing"))
    }
}

fn materialize_token(pending: Option<FencingToken>, commit_receipt_ref: &str) -> Result<Option<FencingToken>> {
    pending
        .map(|token| {
            let value =
                fencing_token_value(&token.key, &token.owner, token.token, token.lease_epoch, commit_receipt_ref)?;
            parse_fencing_token(&value)
        })
        .transpose()
}

fn pending_token(key: &str, owner: &str, token: u64) -> Result<FencingToken> {
    let pending_commit_ref = fixture_ref("pending-coordination-commit");
    let value = fencing_token_value(key, owner, token, token, &pending_commit_ref)?;
    parse_fencing_token(&value)
}

fn status_fact_for_token(
    state: &CoordinationState,
    manifest: &CoordinationServiceManifest,
    service: &str,
    key: &str,
    token: &FencingToken,
) -> Result<IoValue> {
    match service {
        SERVICE_LOCK => Ok(record("lock-held", vec![
            string(key),
            string(&token.owner),
            u64_value(token.token),
            string(&token.token_ref),
        ])),
        SERVICE_ELECTION => Ok(record("leader", vec![
            string(key),
            string(&token.owner),
            u64_value(token.token),
            string(&token.token_ref),
        ])),
        _ => status_fact_for(state, manifest, service, key),
    }
}

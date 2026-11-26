
// r[impl molten.coordination.truthful_tokens_assertions_and_expiry]
/// Claims the lock with a lease bounded on the coordination clock. The lock entry is inserted at commit,
/// carrying the fencing token bound to the commit receipt.
fn prepare_lock_acquire(
    runtime: &CoordinationRuntime,
    request: &CoordinationRequest,
    now: u64,
) -> Result<PreparedMutation> {
    if let Some(lock) = runtime.state.locks.get(&request.key) {
        return Err(MoltenError::invalid_harness(format!(
            "coordination lock {} already held by {} until clock tick {}",
            request.key, lock.owner, lock.lease_expires_at
        )));
    }
    let lease = lease_expiry(request.payload.as_ref(), now)?;
    let (state, claim) = claim_fencing_token(runtime, request, SERVICE_LOCK, &request.client_session, lease)?;
    Ok(PreparedMutation {
        state,
        status_fact: record("lock-held", vec![
            string(&request.key),
            string(&claim.owner),
            u64_value(claim.token),
            u64_value(claim.lease_expires_at),
        ]),
        token_claim: Some(claim),
        checks: vec![("fencing-token-monotonic", "pass"), ("lock-lease-held", "pass"), ("lease-bounded", "pass")],
    })
}

/// Advances the fencing-token counter for a token-granting transition.
fn claim_fencing_token(
    runtime: &CoordinationRuntime,
    request: &CoordinationRequest,
    service: &str,
    owner: &str,
    lease_expires_at: u64,
) -> Result<(CoordinationState, CoordinationTokenClaim)> {
    let token_number = runtime.state.next_fencing_token;
    let mut state = runtime.state.clone();
    state.next_fencing_token = token_number
        .checked_add(1)
        .ok_or_else(|| MoltenError::invalid_harness("coordination fencing token overflow"))?;
    let claim = CoordinationTokenClaim {
        service: service.to_string(),
        key: request.key.clone(),
        owner: owner.to_string(),
        token: token_number,
        lease_epoch: token_number,
        lease_expires_at,
    };
    Ok((state, claim))
}

// r[impl molten.coordination.truthful_tokens_assertions_and_expiry]
/// Removes a lock or leader whose lease expiry the coordination clock sample `now` has reached, so a crashed
/// holder cannot wedge the key. Requests present no time (an `expire` payload is denied) and other mutations
/// do not age a lease: only the shell-owned clock does. The holder's fencing token is never reissued.
fn prepare_lease_expiry(
    runtime: &CoordinationRuntime,
    request: &CoordinationRequest,
    now: u64,
) -> Result<PreparedMutation> {
    if request.payload.is_some() {
        return Err(MoltenError::invalid_harness(
            "coordination expire takes no payload; time comes only from the coordination clock",
        ));
    }
    let mut state = runtime.state.clone();
    let (holder, token, expires_at, label) = match request.service.as_str() {
        SERVICE_LOCK => {
            let lock = state
                .locks
                .remove(&request.key)
                .ok_or_else(|| MoltenError::invalid_harness("coordination lock is not held"))?;
            (lock.owner, lock.token, lock.lease_expires_at, "lock-expired")
        }
        SERVICE_ELECTION => {
            let election = state
                .elections
                .remove(&request.key)
                .ok_or_else(|| MoltenError::invalid_harness("coordination election has no leader"))?;
            (election.leader, election.token, election.lease_expires_at, "leader-expired")
        }
        other => return Err(MoltenError::invalid_harness(format!("coordination service {other} has no lease"))),
    };
    if now < expires_at {
        return Err(MoltenError::invalid_harness(format!(
            "coordination {} lease for {} held by {holder} is unexpired: expires at clock tick {expires_at}, coordination clock reads {now}",
            request.service, request.key
        )));
    }
    Ok(PreparedMutation {
        state,
        token_claim: None,
        status_fact: record(label, vec![string(&request.key), string(&holder), u64_value(token), u64_value(expires_at)]),
        checks: vec![("lease-expiry-checked", "pass"), ("shell-clock-sampled", "pass")],
    })
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
        token_claim: None,
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
        token_claim: None,
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
        token_claim: None,
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
        token_claim: None,
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
            token_claim: None,
            status_fact: record("semaphore-available", vec![string(&request.key), u64_value(available)]),
            checks: vec![("semaphore-bounds", "pass"), ("release-committed", "pass")],
        })
    } else {
        Err(MoltenError::invalid_harness("coordination semaphore release holder mismatch"))
    }
}

/// The `rate-limit` service is a per-key acquisition quota for the lifetime of the coordination state.
/// The quota does not consult the coordination clock, so nothing resets the count; the wire name
/// stays `rate-limit` for manifest and receipt compatibility.
// r[impl molten.coordination.rate_limit_semaphore]
fn prepare_rate_acquire(runtime: &CoordinationRuntime, request: &CoordinationRequest) -> Result<PreparedMutation> {
    let used = runtime.state.rates.get(&request.key).copied().unwrap_or(0);
    if used >= runtime.manifest.rate_limit {
        return Err(MoltenError::invalid_harness("coordination rate-limit quota exhausted; it does not reset"));
    }
    let mut state = runtime.state.clone();
    state.rates.insert(request.key.clone(), used + 1);
    Ok(PreparedMutation {
        state,
        token_claim: None,
        status_fact: record("rate-limit", vec![
            string(&request.key),
            u64_value(used + 1),
            u64_value(runtime.manifest.rate_limit),
        ]),
        checks: vec![("rate-limit-bounds", "pass")],
    })
}

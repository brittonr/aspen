use super::*;

// r[impl molten.federated_pull_runtime.receiver_pull]
// r[impl molten.federated_pull_runtime.bounds_freshness]
pub fn plan_pull(input: &PullInput<'_>) -> Result<PullPlan, PullIssue> {
    validate_input(input)?;
    let mut plan = PullPlan {
        peer_ref: input.inventory.peer_ref.clone(),
        generation: input.generation,
        signer_ref: input.inventory.signer_ref.clone(),
        inventory_issued_at: input.inventory.issued_at,
        inventory_expires_at: input.inventory.expires_at,
        deadline: input.now.checked_add(input.bounds.session_ticks).ok_or(PullIssue::Clock)?,
        missing: Vec::new(),
        conflicts: Vec::new(),
        bytes: 0,
        dag_nodes: 0,
        chunks: 0,
    };
    for entry in &input.inventory.entries {
        if let Some(local) = input.local.iter().find(|local| local.name == entry.name) {
            if local.head_ref != entry.head_ref {
                plan.conflicts.push(Conflict {
                    name: entry.name.clone(),
                    local_ref: local.head_ref.clone(),
                    remote: entry.clone(),
                });
            }
            continue;
        }
        if input.local.iter().any(|local| local.head_ref == entry.head_ref) {
            continue;
        }
        plan.bytes = plan.bytes.checked_add(entry.bytes).ok_or(PullIssue::Resource)?;
        plan.dag_nodes = plan.dag_nodes.checked_add(entry.dag_nodes).ok_or(PullIssue::Resource)?;
        plan.chunks = plan.chunks.checked_add(entry.chunks).ok_or(PullIssue::Resource)?;
        if plan.bytes > input.bounds.fetch_bytes
            || plan.dag_nodes > input.bounds.dag_nodes
            || plan.chunks > input.bounds.content_chunks
        {
            return Err(PullIssue::Resource);
        }
        plan.missing.push(entry.clone());
    }
    plan.missing.sort_by(|a, b| a.head_ref.cmp(&b.head_ref));
    plan.missing.dedup_by(|a, b| a.head_ref == b.head_ref);
    plan.conflicts.sort_by(|a, b| a.name.cmp(&b.name));
    // One inventory verification plus verification, seven gates and commit evidence per head.
    let evidence = plan
        .missing
        .len()
        .checked_mul(9)
        .and_then(|count| count.checked_add(1))
        .ok_or(PullIssue::Resource)?;
    if evidence > input.bounds.retained_status {
        return Err(PullIssue::Resource);
    }
    check_resources(input.resource_history, &plan, input.bounds, input.now)?;
    Ok(plan)
}

fn validate_input(input: &PullInput<'_>) -> Result<(), PullIssue> {
    let bounds = input.bounds;
    if bounds.inventory_entries == 0
        || bounds.inventory_entries > MAX_FEDERATION_ENTRIES
        || bounds.concurrent_sessions == 0
        || bounds.retained_status == 0
        || bounds.retained_status > MAX_FEDERATION_ENTRIES
        || bounds.freshness == 0
        || bounds.session_ticks == 0
        || bounds.inventory_bytes == 0
        || input.configured.len() > MAX_FEDERATION_PEERS
    {
        return Err(PullIssue::Bounds);
    }
    if !input.receiver_requested {
        return Err(PullIssue::Unsolicited);
    }
    let inventory = input.inventory;
    if inventory.generation != input.generation {
        return Err(PullIssue::Generation);
    }
    let peer = input
        .configured
        .iter()
        .find(|peer| {
            peer.peer_ref == inventory.peer_ref
                && peer.signer_ref == inventory.signer_ref
                && peer.source == LocatorSource::Static
        })
        .ok_or(PullIssue::UnknownPeer)?;
    if !peer.available {
        return Err(PullIssue::Unavailable);
    }
    if peer.observed_at > input.now || inventory.issued_at > input.now {
        return Err(PullIssue::Clock);
    }
    if peer.expires_at < input.now
        || inventory.expires_at < input.now
        || inventory.expires_at < inventory.issued_at
        || input.now - inventory.issued_at > bounds.freshness
    {
        return Err(PullIssue::Stale);
    }
    if let Some(last) = input.last_query {
        let elapsed = input.now.checked_sub(last).ok_or(PullIssue::Clock)?;
        if elapsed < bounds.query_interval {
            return Err(PullIssue::RateLimited);
        }
    }
    if input.active_sessions >= bounds.concurrent_sessions {
        return Err(PullIssue::Sessions);
    }
    if inventory.encoded_bytes > bounds.inventory_bytes
        || inventory.encoded_bytes == 0
        || inventory.entries.len() > bounds.inventory_entries
        || input.local.len() > MAX_FEDERATION_ENTRIES
    {
        return Err(PullIssue::Inventory);
    }
    validate_entries(&inventory.entries)?;
    validate_entries(input.local)
}

fn validate_entries(entries: &[InventoryEntry]) -> Result<(), PullIssue> {
    for (index, entry) in entries.iter().enumerate() {
        if entry.name.is_empty()
            || entry.name.len() > 256
            || entry.bytes == 0
            || !canonical_ref(&entry.head_ref)
            || !canonical_ref(&entry.ancestry_ref)
            || !canonical_ref(&entry.policy_ref)
        {
            return Err(PullIssue::Inventory);
        }
        if entries[..index].iter().any(|previous| previous.name == entry.name) {
            return Err(PullIssue::DuplicateName);
        }
    }
    Ok(())
}

pub fn canonical_ref(value: &str) -> bool {
    value.strip_prefix("blake3:").is_some_and(|digest| {
        digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    })
}

fn check_resources(
    history: &[ResourceHistory],
    plan: &PullPlan,
    bounds: &PullBounds,
    now: u64,
) -> Result<(), PullIssue> {
    if history.len() > MAX_FEDERATION_ENTRIES || history.windows(2).any(|pair| pair[0].head_ref >= pair[1].head_ref) {
        return Err(PullIssue::ResourceHistory);
    }
    for item in history {
        if !canonical_ref(&item.head_ref)
            || item.window_started > item.last_query
            || item.last_query > now
            || item.attempts == 0
        {
            return Err(PullIssue::ResourceHistory);
        }
    }
    let live = history.iter().filter(|item| now - item.window_started < bounds.session_ticks).count();
    let additional = plan
        .missing
        .iter()
        .filter(|entry| {
            !history
                .iter()
                .any(|item| item.head_ref == entry.head_ref && now - item.window_started < bounds.session_ticks)
        })
        .count();
    if live.checked_add(additional).ok_or(PullIssue::ResourceHistory)? > MAX_FEDERATION_ENTRIES {
        return Err(PullIssue::ResourceHistory);
    }
    for entry in &plan.missing {
        resource_counters(history.iter().find(|item| item.head_ref == entry.head_ref), entry, bounds, now)?;
    }
    Ok(())
}

fn resource_counters(
    previous: Option<&ResourceHistory>,
    entry: &InventoryEntry,
    bounds: &PullBounds,
    now: u64,
) -> Result<(u64, u32, u64, u64, u64), PullIssue> {
    let previous = previous.filter(|item| now - item.window_started < bounds.session_ticks);
    let mut counts = (now, 0u32, 0u64, 0u64, 0u64);
    if let Some(item) = previous {
        if now - item.last_query < bounds.query_interval {
            return Err(PullIssue::ResourceRateLimited);
        }
        counts = (item.window_started, item.attempts, item.bytes, item.dag_nodes, item.chunks);
    }
    if counts.1 > bounds.retries {
        return Err(PullIssue::ResourceRetry);
    }
    counts.1 = counts.1.checked_add(1).ok_or(PullIssue::ResourceRetry)?;
    counts.2 = counts.2.checked_add(entry.bytes).ok_or(PullIssue::Resource)?;
    counts.3 = counts.3.checked_add(entry.dag_nodes).ok_or(PullIssue::Resource)?;
    counts.4 = counts.4.checked_add(entry.chunks).ok_or(PullIssue::Resource)?;
    if counts.2 > bounds.fetch_bytes || counts.3 > bounds.dag_nodes || counts.4 > bounds.content_chunks {
        return Err(PullIssue::Resource);
    }
    Ok(counts)
}

/// Reserve before transfers. Expiration depends only on the admitted logical tick, never ambient
/// pruning.
pub fn reserve_resources(
    history: &[ResourceHistory],
    plan: &PullPlan,
    bounds: &PullBounds,
    now: u64,
) -> Result<Vec<ResourceHistory>, PullIssue> {
    check_resources(history, plan, bounds, now)?;
    let mut next: Vec<_> =
        history.iter().filter(|item| now - item.window_started < bounds.session_ticks).cloned().collect();
    for entry in &plan.missing {
        let previous = history.iter().find(|item| item.head_ref == entry.head_ref);
        let (window_started, attempts, bytes, dag_nodes, chunks) = resource_counters(previous, entry, bounds, now)?;
        let reservation = ResourceHistory {
            head_ref: entry.head_ref.clone(),
            window_started,
            last_query: now,
            attempts,
            bytes,
            dag_nodes,
            chunks,
        };
        match next.iter_mut().find(|item| item.head_ref == entry.head_ref) {
            Some(item) => *item = reservation,
            None => next.push(reservation),
        }
    }
    next.sort_by(|left, right| left.head_ref.cmp(&right.head_ref));
    Ok(next)
}

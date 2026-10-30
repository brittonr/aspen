/// Reconstruct only the held suffix of the latest snapshot from older records.
/// Previous snapshot installations need not remain in the record stream.
fn replay_snapshot_prefix(
    state: &mut ReplicaState,
    durable_commit_index: &mut u64,
    bytes: &[u8],
    boundary_term: &mut Option<u64>,
    is_suffix_retained: &mut Option<bool>,
) -> crate::error::Result<()> {
    let boundary = state.snapshot.as_ref().ok_or_else(||
        crate::error::Failure::invalid_harness("snapshot prefix requires a snapshot"))?.last_included_index;
    let decoded = crate::preserves_rail::strict_canonical_decode(bytes)?;
    let value = &decoded.value;
    if let Some((reference, is_retained)) = suffix_disposition(value)? {
        if state.snapshot.as_ref().is_some_and(|snapshot| snapshot.snapshot_ref == reference) {
            *is_suffix_retained = Some(is_retained);
        }
        return Ok(());
    }
    if let Some(fields) = value.collect_simple_record("raft-log-mutation-v1", Some(LOG_MUTATION_ARITY)) {
        let fields = fields.iter().collect::<Vec<_>>();
        let truncate_from = optional_u64(&fields[0])?;
        if truncate_from.is_some_and(|index| index <= boundary) {
            *boundary_term = None;
        }
        let mut entries = parse_entries(&fields[1])?;
        if let Some(entry) = entries.iter().find(|entry| entry.index == boundary) {
            *boundary_term = Some(entry.term);
        }
        entries.retain(|entry| entry.index > boundary);
        let truncate_from = truncate_from.map(|index| {
            boundary.checked_add(1).map(|after| index.max(after))
                .ok_or_else(|| crate::error::Failure::invalid_harness("snapshot prefix index overflow"))
        }).transpose()?;
        apply_log_mutation(state, boundary, truncate_from, entries)?;
        return Ok(());
    }
    for tag in ["raft-log-flush-v1", "raft-commit-boundary-v1"] {
        if let Some(fields) = value.collect_simple_record(tag, Some(SINGLE_INDEX_ARITY))
            && canonical::required_u64(&fields[0], "snapshot-covered index")? <= boundary
        {
            return Ok(());
        }
    }
    replay_value(state, durable_commit_index, value)
}

fn replay_record(state: &mut ReplicaState, durable_commit_index: &mut u64, bytes: &[u8]) -> crate::error::Result<()> {
    let decoded = crate::preserves_rail::strict_canonical_decode(bytes)?;
    replay_value(state, durable_commit_index, &decoded.value)
}

fn replay_value(
    state: &mut ReplicaState,
    durable_commit_index: &mut u64,
    value: &preserves::Value<preserves::IOValue>,
) -> crate::error::Result<()> {
    if suffix_disposition(value)?.is_some() {
        return Ok(());
    }
    if let Some(fields) = value.collect_simple_record("raft-hard-state-v2", Some(HARD_STATE_ARITY + 1))
        .or_else(|| value.collect_simple_record("raft-hard-state-v1", Some(HARD_STATE_ARITY)))
    {
        let fields = fields.iter().collect::<Vec<_>>();
        let term = canonical::required_u64(&fields[0], "recovered hard-state term")?;
        if term < state.current_term {
            return Err(crate::error::Failure::invalid_harness("live Raft recovery hard-state term regressed"));
        }
        state.current_term = term;
        state.voted_for = optional_string(&fields[1])?;
        state.rejoin = if fields.len() > HARD_STATE_ARITY
            && canonical::required_bool(&fields[2], "recovered recovery-required marker")?
        {
            ReplicaRejoin::Recovering(RecoveryRound {
                round_ref: state.active_election_timer_ref.clone(),
                fence_term: None,
                peers: std::collections::BTreeMap::new(),
            })
        } else {
            ReplicaRejoin::Voter
        };
        return Ok(());
    }
    if let Some(fields) = value.collect_simple_record("raft-log-mutation-v1", Some(LOG_MUTATION_ARITY)) {
        let fields = fields.iter().collect::<Vec<_>>();
        let truncate_from = optional_u64(&fields[0])?;
        let entries = parse_entries(&fields[1])?;
        apply_log_mutation(state, *durable_commit_index, truncate_from, entries)?;
        return Ok(());
    }
    if let Some(fields) = value.collect_simple_record("raft-log-flush-v1", Some(SINGLE_INDEX_ARITY)) {
        let fields = fields.iter().collect::<Vec<_>>();
        let through_index = canonical::required_u64(&fields[0], "recovered flush boundary")?;
        if through_index > support_last_log_index(state) {
            return Err(crate::error::Failure::invalid_harness(
                "live Raft recovery flush boundary exceeds durable log",
            ));
        }
        return Ok(());
    }
    if let Some(fields) = value.collect_simple_record("raft-commit-boundary-v1", Some(SINGLE_INDEX_ARITY)) {
        let fields = fields.iter().collect::<Vec<_>>();
        let through_index = canonical::required_u64(&fields[0], "recovered commit boundary")?;
        if through_index <= *durable_commit_index || through_index > support_last_log_index(state) {
            return Err(crate::error::Failure::invalid_harness(
                "live Raft recovery commit boundary is stale or beyond the log",
            ));
        }
        *durable_commit_index = through_index;
        return Ok(());
    }
    Err(crate::error::Failure::invalid_harness("live Raft recovery found an unsupported durable record"))
}

fn suffix_disposition(value: &preserves::Value<preserves::IOValue>) -> crate::error::Result<Option<(String, bool)>> {
    let Some(fields) = value.collect_simple_record("raft-snapshot-suffix-v1", Some(2)) else {
        return Ok(None);
    };
    let reference = canonical::required_string(&fields[0], "snapshot suffix reference")?;
    crate::preserves_rail::validate_content_ref(&reference)?;
    Ok(Some((reference, canonical::required_bool(&fields[1], "snapshot suffix retention")?)))
}

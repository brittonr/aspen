/// Reconstruct only the held suffix of the latest snapshot from older records.
/// Previous snapshot installations need not remain in the record stream.
fn replay_snapshot_prefix(
    state: &mut ReplicaState,
    durable_commit_index: &mut u64,
    value: &preserves::Value<preserves::IOValue>,
    boundary_term: &mut Option<u64>,
    is_suffix_retained: &mut Option<bool>,
) -> crate::error::Result<()> {
    let boundary = state.snapshot.as_ref().ok_or_else(||
        crate::error::Failure::invalid_harness("snapshot prefix requires a snapshot"))?.last_included_index;
    if let Some((reference, is_retained)) = suffix_disposition(value)? {
        if state.snapshot.as_ref().is_some_and(|snapshot| snapshot.snapshot_ref == reference) {
            *is_suffix_retained = Some(is_retained);
        }
        return Ok(());
    }
    if let Some(fields) = value.collect_simple_record("raft-log-mutation-v1", Some(LOG_MUTATION_ARITY)) {
        let fields = fields.iter().collect::<Vec<_>>();
        let truncate_from = optional_u64(&fields[0])?;
        return replay_prefix_mutation(state, boundary, truncate_from, parse_entries(&fields[1])?, boundary_term);
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
        let is_recovery_required = fields.len() > HARD_STATE_ARITY
            && canonical::required_bool(&fields[2], "recovered recovery-required marker")?;
        return apply_hard_state(state, term, optional_string(&fields[1])?, is_recovery_required);
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

/// Apply a record that replaces the log after `truncate_from` while replaying records the newest
/// snapshot covers: only entries above the snapshot boundary are kept, and the boundary entry's term
/// decides whether the held suffix extends the snapshot.
fn replay_prefix_mutation(
    state: &mut ReplicaState,
    boundary: u64,
    truncate_from: Option<u64>,
    mut entries: Vec<ReplicatedEntry>,
    boundary_term: &mut Option<u64>,
) -> crate::error::Result<()> {
    if truncate_from.is_some_and(|index| index <= boundary) {
        *boundary_term = None;
    }
    if let Some(entry) = entries.iter().find(|entry| entry.index == boundary) {
        *boundary_term = Some(entry.term);
    }
    entries.retain(|entry| entry.index > boundary);
    let truncate_from = truncate_from.map(|index| {
        boundary.checked_add(1).map(|after| index.max(after))
            .ok_or_else(|| crate::error::Failure::invalid_harness("snapshot prefix index overflow"))
    }).transpose()?;
    apply_log_mutation(state, boundary, truncate_from, entries)
}

fn apply_hard_state(
    state: &mut ReplicaState,
    term: u64,
    voted_for: Option<String>,
    is_recovery_required: bool,
) -> crate::error::Result<()> {
    if term < state.current_term {
        return Err(crate::error::Failure::invalid_harness("live Raft recovery hard-state term regressed"));
    }
    state.current_term = term;
    state.voted_for = voted_for;
    state.rejoin = if is_recovery_required {
        ReplicaRejoin::Recovering(RecoveryRound {
            round_ref: state.active_election_timer_ref.clone(),
            fence_term: None,
            peers: std::collections::BTreeMap::new(),
        })
    } else {
        ReplicaRejoin::Voter
    };
    Ok(())
}

/// Header metadata is bounded independently of the suffix. The staged entries are capped by
/// the admitted log bound and never replace any recovered state before the matching footer.
struct CompactionBase {
    snapshot_ref: String,
    boundary: u64,
    term: u64,
    voted_for: Option<String>,
    entry_count: usize,
    commit_index: u64,
    value_ref: String,
    entries: Vec<ReplicatedEntry>,
}

fn compaction_base(value: &preserves::Value<preserves::IOValue>) -> crate::error::Result<Option<CompactionBase>> {
    let Some(fields) = value.collect_simple_record(COMPACTION_BASE_RECORD, Some(COMPACTION_BASE_ARITY)) else {
        return Ok(None);
    };
    let snapshot_ref = canonical::required_string(&fields[0], "compaction base snapshot ref")?;
    crate::preserves_rail::validate_content_ref(&snapshot_ref)?;
    let count = canonical::required_u64(&fields[4], "compaction base entry count")?;
    Ok(Some(CompactionBase {
        snapshot_ref,
        boundary: canonical::required_u64(&fields[1], "compaction base boundary")?,
        term: canonical::required_u64(&fields[2], "compaction base term")?,
        voted_for: optional_string(&fields[3])?,
        entry_count: usize::try_from(count).map_err(|_| {
            crate::error::Failure::invalid_harness("compaction base entry count exceeds address space")
        })?,
        commit_index: canonical::required_u64(&fields[5], "compaction base commit index")?,
        value_ref: String::new(),
        entries: Vec::new(),
    }))
}

struct CompactionReplay {
    pending: Option<CompactionBase>,
    is_required_complete: bool,
}

struct CompactionRecordInput<'a> {
    record: &'a crate::fabric_durability::LogRecord,
    value: &'a preserves::Value<preserves::IOValue>,
    is_covered: bool,
}

fn begin_base(
    state: &mut ReplicaState,
    mut base: CompactionBase,
    input: &CompactionRecordInput<'_>,
    progress: &mut CompactionReplay,
) -> crate::error::Result<()> {
    let snapshot = state.snapshot.as_ref().ok_or_else(|| {
        crate::error::Failure::invalid_harness("live Raft compaction base requires its snapshot")
    })?;
    if base.entry_count > state.profile.max_log_entries
        || (!input.is_covered && (base.snapshot_ref != snapshot.snapshot_ref
            || base.boundary != snapshot.last_included_index))
        || base.commit_index < base.boundary
        || (progress.pending.is_some() && !progress.is_required_complete)
    {
        return Err(crate::error::Failure::invalid_harness("invalid live Raft compaction base header"));
    }
    // Even an interrupted base preserves its hard state and the uncertainty marker. Older
    // durable records still reconstruct its suffix until truncation follows a complete base.
    apply_hard_state(state, base.term, base.voted_for.take(), true)?;
    base.value_ref.clone_from(&input.record.value_ref);
    base.entries = Vec::with_capacity(base.entry_count);
    progress.pending = Some(base);
    Ok(())
}

fn replay_base_record(
    state: &mut ReplicaState,
    durable_commit_index: &mut u64,
    input: CompactionRecordInput<'_>,
    progress: &mut CompactionReplay,
    boundary_term: &mut Option<u64>,
) -> crate::error::Result<bool> {
    let value = input.value;
    if let Some(base) = compaction_base(value)? {
        begin_base(state, base, &input, progress)?;
        return Ok(true);
    }
    if let Some(fields) = value.collect_simple_record("raft-compaction-end-v1", Some(1)) {
        let base = progress.pending.take().ok_or_else(|| {
            crate::error::Failure::invalid_harness("compaction footer without a base")
        })?;
        if canonical::required_string(&fields[0], "compaction header ref")? != base.value_ref
            || base.entries.len() != base.entry_count
        {
            return Err(crate::error::Failure::invalid_harness("incomplete or mismatched compaction base"));
        }
        let snapshot_boundary = state.snapshot.as_ref().ok_or_else(|| {
            crate::error::Failure::invalid_harness("compaction footer requires its snapshot")
        })?.last_included_index;
        let mut entries = base.entries;
        if let Some(entry) = entries.iter().find(|entry| entry.index == snapshot_boundary) {
            *boundary_term = Some(entry.term);
        }
        entries.retain(|entry| entry.index > snapshot_boundary);
        // A base may not replace or omit any already reconstructed committed entry.
        if state.log.iter().filter(|entry| entry.index <= *durable_commit_index)
            .any(|entry| entry.index.checked_sub(snapshot_boundary).and_then(|offset| offset.checked_sub(1))
                .and_then(|offset| usize::try_from(offset).ok()).and_then(|offset| entries.get(offset)) != Some(entry))
        {
            return Err(crate::error::Failure::invalid_harness("compaction base changes committed entries"));
        }
        state.log.clear();
        apply_log_mutation(state, *durable_commit_index, None, entries)?;
        *durable_commit_index = (*durable_commit_index).max(base.commit_index);
        if *durable_commit_index > support_last_log_index(state) {
            return Err(crate::error::Failure::invalid_harness("compaction base omits committed suffix"));
        }
        if base.boundary == snapshot_boundary {
            *boundary_term = state.snapshot.as_ref().map(|snapshot| snapshot.last_included_term);
        }
        progress.is_required_complete = true;
        return Ok(true);
    }
    if let Some(base) = progress.pending.as_mut() {
        if let Some(fields) = value.collect_simple_record("raft-log-mutation-v1", Some(LOG_MUTATION_ARITY)) {
            let entries = fields[1].collect_sequence().ok_or_else(|| {
                crate::error::Failure::invalid_harness("compaction entries must be a sequence")
            })?;
            if optional_u64(&fields[0])?.is_some() || entries.as_ref().as_slice().len() != 1
                || base.entries.len() >= base.entry_count
            {
                return Err(crate::error::Failure::invalid_harness("invalid compaction base entry record"));
            }
            let entry = canonical::parse_entry(&entries.as_ref().as_slice()[0])?;
            let expected = base.boundary.checked_add(base.entries.len() as u64)
                .and_then(|index| index.checked_add(1));
            if Some(entry.index) != expected || entry.term == INITIAL_TERM || entry.term > base.term {
                return Err(crate::error::Failure::invalid_harness("noncontiguous compaction base entries"));
            }
            base.entries.push(entry);
            return Ok(true);
        }
        // A restart can append normal records after an interrupted, untruncated base. Its
        // staged suffix is abandoned, not exposed as a complete replacement.
        if !progress.is_required_complete {
            return Err(crate::error::Failure::invalid_harness("interrupted required compaction base"));
        }
        progress.pending = None;
    }
    Ok(false)
}

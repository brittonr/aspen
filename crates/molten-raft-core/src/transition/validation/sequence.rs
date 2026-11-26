//! Log order, duplicate-request windows, and role identity.

use super::*;

/// One `AppendEntries` batch as the follower log it merges into sees it.
pub(in crate::transition) struct IncomingBatch<'a> {
    pub prev_log_index: u64,
    pub leader_term: u64,
    pub entries: &'a [ReplicatedEntry],
    /// The first index whose local entry the batch replaces (`None` when the batch changes
    /// nothing): duplicate request refs are checked only against local entries below it, which
    /// the merge retains, and against earlier entries of the batch itself.
    pub retained_before: Option<u64>,
}

/// Validate one `AppendEntries` batch against the follower log it merges into.
// r[impl molten.consensus.raft_replication_and_recovery_liveness]
pub(in crate::transition) fn validate_incoming_entries(
    state: &ReplicaState,
    batch: &IncomingBatch<'_>,
) -> crate::denial::Result<()> {
    if batch.entries.len() > state.profile.max_message_entries {
        return Err(crate::denial::ReplicaError::Denied("Raft append entries exceed the admitted message bound"));
    }
    let mut expected_index = batch
        .prev_log_index
        .checked_add(NEXT_LOG_INDEX_STEP)
        .ok_or(crate::denial::ReplicaError::Overflow("Raft append index overflow"))?;
    let mut request_refs = alloc::collections::BTreeMap::new();
    for entry in batch.entries {
        validate_incoming_entry(state, batch, entry, expected_index, &mut request_refs)?;
        expected_index = expected_index
            .checked_add(NEXT_LOG_INDEX_STEP)
            .ok_or(crate::denial::ReplicaError::Overflow("Raft append index overflow"))?;
    }
    Ok(())
}

fn validate_incoming_entry<'a>(
    state: &ReplicaState,
    batch: &IncomingBatch<'_>,
    entry: &'a ReplicatedEntry,
    expected_index: u64,
    request_refs: &mut alloc::collections::BTreeMap<&'a str, u64>,
) -> crate::denial::Result<()> {
    if entry.index != expected_index || entry.term == 0 || entry.term > batch.leader_term {
        return Err(crate::denial::ReplicaError::Denied(
            "Raft append entries are not contiguous or use an invalid term",
        ));
    }
    validate_content_ref(&entry.request_ref, "Raft log request ref")?;
    validate_content_ref(&entry.command_ref, "Raft log command ref")?;
    validate_content_ref(&entry.command_schema_ref, "Raft log command schema ref")?;
    let window = state.profile.request_retention_entries;
    if !admit_request_ref(request_refs, entry, window) {
        return Err(crate::denial::ReplicaError::Denied(
            "Raft append entries repeat a request ref within its duplicate window",
        ));
    }
    if state.log.iter().any(|existing| {
        batch.retained_before.is_none_or(|boundary| existing.index < boundary)
            && existing.request_ref == entry.request_ref
            && existing.index != entry.index
            && existing.index.abs_diff(entry.index) <= window
    }) {
        return Err(crate::denial::ReplicaError::Denied(
            "Raft append reuses a request ref within its duplicate window",
        ));
    }
    Ok(())
}

pub(super) fn validate_log(state: &ReplicaState) -> crate::denial::Result<()> {
    let mut expected = match &state.snapshot {
        Some(snapshot) => snapshot
            .last_included_index
            .checked_add(NEXT_LOG_INDEX_STEP)
            .ok_or(crate::denial::ReplicaError::Overflow("Raft snapshot log index overflow"))?,
        None => INITIAL_LOG_INDEX,
    };
    let mut request_refs = alloc::collections::BTreeMap::new();
    let mut previous_term = state.snapshot.as_ref().map_or(INITIAL_TERM, |snapshot| snapshot.last_included_term);
    for entry in &state.log {
        validate_state_entry(
            entry,
            expected,
            previous_term,
            &mut request_refs,
            state.profile.request_retention_entries,
        )?;
        previous_term = entry.term;
        expected = expected
            .checked_add(NEXT_LOG_INDEX_STEP)
            .ok_or(crate::denial::ReplicaError::Overflow("Raft log index overflow"))?;
    }
    Ok(())
}

fn validate_state_entry<'a>(
    entry: &'a ReplicatedEntry,
    expected_index: u64,
    previous_term: u64,
    request_refs: &mut alloc::collections::BTreeMap<&'a str, u64>,
    window: u64,
) -> crate::denial::Result<()> {
    if entry.index != expected_index || entry.term == 0 || entry.term < previous_term {
        return Err(crate::denial::ReplicaError::Denied("Raft log indices or terms are invalid"));
    }
    validate_content_ref(&entry.request_ref, "Raft state request ref")?;
    validate_content_ref(&entry.command_ref, "Raft state command ref")?;
    validate_content_ref(&entry.command_schema_ref, "Raft state command schema ref")?;
    if !admit_request_ref(request_refs, entry, window) {
        return Err(crate::denial::ReplicaError::Denied("Raft log repeats a request ref within its duplicate window"));
    }
    Ok(())
}

/// A leader appends a request ref only when no copy is pending or committed within the last W
/// applied entries, so two copies of one ref in a log are always more than W indices apart.
/// Entries arrive in increasing index order.
// r[impl molten.consensus.bounded_request_dedup]
fn admit_request_ref<'a>(
    request_refs: &mut alloc::collections::BTreeMap<&'a str, u64>,
    entry: &'a ReplicatedEntry,
    window: u64,
) -> bool {
    let previous = request_refs.insert(entry.request_ref.as_str(), entry.index);
    previous.is_none_or(|previous| entry.index.saturating_sub(previous) > window)
}

pub(super) fn validate_role(state: &ReplicaState) -> crate::denial::Result<()> {
    if state.role == ReplicaRole::Leader && state.leader_id.as_deref() != Some(state.node_id.as_str()) {
        return Err(crate::denial::ReplicaError::Denied("Raft leader state lacks self leader identity"));
    }
    if state.role != ReplicaRole::Leader && state.leader_id.as_deref() == Some(state.node_id.as_str()) {
        return Err(crate::denial::ReplicaError::Denied("non-leader Raft state claims self leadership"));
    }
    if state.role != ReplicaRole::Follower && state.current_term == 0 {
        return Err(crate::denial::ReplicaError::Denied("candidate or leader cannot occupy term zero"));
    }
    Ok(())
}

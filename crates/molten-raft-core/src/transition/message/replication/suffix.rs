//! Reconciling a follower log with one `AppendEntries` batch.

use super::*;

/// The follower log after reconciling one `AppendEntries` batch.
pub(super) struct MergedEntries {
    pub truncate_from: Option<u64>,
    pub appended: Vec<ReplicatedEntry>,
    /// The last index known to match the leader's log after the merge.
    pub acknowledged_index: u64,
}

pub(super) fn first_changed_entry(state: &ReplicaState, incoming: &[ReplicatedEntry]) -> Option<usize> {
    incoming.iter().position(|entry| {
        state
            .log
            .iter()
            .find(|existing| existing.index == entry.index)
            .is_none_or(|existing| existing != entry)
    })
}

/// Truncate the conflicting suffix and append the incoming entries that fit the admitted log bound.
///
/// A follower at its bound stores only the prefix that fits and acknowledges exactly that prefix:
/// commit advancement through it lets the hosting service compact with a snapshot, and the leader
/// resends the rest from the acknowledged index.
// r[impl molten.consensus.raft_replication_and_recovery_liveness]
pub(super) fn merge_entries(
    state: &mut ReplicaState,
    prev_log_index: u64,
    incoming: Vec<ReplicatedEntry>,
    first_change: Option<usize>,
) -> crate::denial::Result<MergedEntries> {
    let Some(first_change) = first_change else {
        return Ok(MergedEntries {
            truncate_from: None,
            appended: Vec::new(),
            acknowledged_index: incoming.last().map_or(prev_log_index, |entry| entry.index),
        });
    };
    let change_index = incoming
        .get(first_change)
        .map(|entry| entry.index)
        .ok_or(crate::denial::ReplicaError::Denied("Raft append change lies outside its batch"))?;
    if change_index <= state.commit_index {
        return Err(crate::denial::ReplicaError::Denied("Raft append would overwrite a committed entry"));
    }
    let matched_index = change_index
        .checked_sub(NEXT_LOG_INDEX_STEP)
        .ok_or(crate::denial::ReplicaError::Overflow("Raft append change index underflow"))?;
    let truncate_from = state.log.iter().any(|entry| entry.index >= change_index).then_some(change_index);
    state.log.retain(|entry| entry.index < change_index);
    let capacity = state.profile.max_log_entries.saturating_sub(state.log.len());
    let appended = incoming.into_iter().skip(first_change).take(capacity).collect::<Vec<_>>();
    state.log.extend(appended.iter().cloned());
    Ok(MergedEntries {
        truncate_from,
        acknowledged_index: appended.last().map_or(matched_index, |entry| entry.index),
        appended,
    })
}

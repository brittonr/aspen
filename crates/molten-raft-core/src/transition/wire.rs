//! Shape validation of a peer envelope, independent of any receiving replica's state.
//!
//! The `molten` shell runs it on every envelope it encodes for or decodes from the wire, and the
//! recovery transition runs it on typed simulation ingress, so both paths admit the same messages.

use crate::model::*;

// r[impl molten.fabric_consistency.live_raft]
pub fn validate_envelope(
    envelope: &ReplicaMessageEnvelope,
    identity: &dyn crate::identity::ReplicaIdentity,
) -> crate::denial::Result<()> {
    crate::denial::refs::validate_content_ref(&envelope.group_binding_ref)?;
    crate::denial::refs::validate_identifier(&envelope.from, "Raft wire sender")?;
    crate::denial::refs::validate_identifier(&envelope.to, "Raft wire recipient")?;
    if envelope.from == envelope.to {
        return Err(crate::denial::ReplicaError::Denied("Raft wire sender and recipient must differ"));
    }
    if envelope.service_generation == 0
        || (envelope.message.term() == 0
            && !matches!(envelope.message, Message::RecoveryQuery { .. } | Message::RecoveryReport { .. }))
        || envelope.message.config_epoch() == 0
        || envelope.message.fencing_epoch() == 0
    {
        return Err(crate::denial::ReplicaError::Denied("Raft wire generation, term, and epochs must be positive"));
    }
    validate_embedded_sender(envelope)?;
    validate_recovery(envelope, identity)?;
    validate_entries(&envelope.message, identity)
}

fn validate_embedded_sender(envelope: &ReplicaMessageEnvelope) -> crate::denial::Result<()> {
    let embedded = match &envelope.message {
        Message::RequestVote { candidate_id, .. } => candidate_id,
        Message::VoteResponse { voter_id, .. } => voter_id,
        Message::AppendEntries { leader_id, .. }
        | Message::ReadProbe { leader_id, .. }
        | Message::InstallSnapshot { leader_id, .. } => leader_id,
        Message::AppendResponse { follower_id, .. }
        | Message::ReadAcknowledgement { follower_id, .. }
        | Message::SnapshotResponse { follower_id, .. } => follower_id,
        Message::RecoveryQuery { requester_id, .. } => requester_id,
        Message::RecoveryReport { peer_id, .. } => peer_id,
    };
    if embedded != &envelope.from {
        return Err(crate::denial::ReplicaError::Denied("Raft wire sender does not match the embedded sender"));
    }
    Ok(())
}

fn validate_entries(message: &Message, identity: &dyn crate::identity::ReplicaIdentity) -> crate::denial::Result<()> {
    match message {
        Message::ReadProbe { request_ref, .. } | Message::ReadAcknowledgement { request_ref, .. } => {
            crate::denial::refs::validate_content_ref(request_ref)
        }
        Message::InstallSnapshot { snapshot, .. } => validate_snapshot(snapshot, identity),
        Message::AppendEntries {
            term,
            prev_log_index,
            entries,
            ..
        } => validate_append(*term, *prev_log_index, entries),
        Message::RecoveryQuery { .. }
        | Message::RecoveryReport { .. }
        | Message::RequestVote { .. }
        | Message::VoteResponse { .. }
        | Message::AppendResponse { .. }
        | Message::SnapshotResponse { .. } => Ok(()),
    }
}

fn validate_append(term: u64, prev_log_index: u64, entries: &[ReplicatedEntry]) -> crate::denial::Result<()> {
    if entries.len() > MAX_REPLICA_MESSAGE_ENTRIES {
        return Err(crate::denial::ReplicaError::Denied("Raft wire entries exceed the message bound"));
    }
    let mut expected = prev_log_index
        .checked_add(NEXT_LOG_INDEX_STEP)
        .ok_or(crate::denial::ReplicaError::Overflow("Raft wire entry index overflow"))?;
    for entry in entries {
        if entry.index != expected || entry.term == 0 || entry.term > term {
            return Err(crate::denial::ReplicaError::Denied(
                "Raft wire entries are non-contiguous or use an invalid term",
            ));
        }
        validate_entry_refs(entry)?;
        expected = expected
            .checked_add(NEXT_LOG_INDEX_STEP)
            .ok_or(crate::denial::ReplicaError::Overflow("Raft wire entry index overflow"))?;
    }
    Ok(())
}

fn validate_entry_refs(entry: &ReplicatedEntry) -> crate::denial::Result<()> {
    for reference in [&entry.request_ref, &entry.command_ref, &entry.command_schema_ref] {
        crate::denial::refs::validate_content_ref(reference)?;
    }
    Ok(())
}

fn validate_snapshot(
    snapshot: &ReplicaSnapshot,
    identity: &dyn crate::identity::ReplicaIdentity,
) -> crate::denial::Result<()> {
    for reference in [
        &snapshot.snapshot_ref,
        &snapshot.group_binding_ref,
        &snapshot.membership_ref,
        &snapshot.application_state_ref,
    ] {
        crate::denial::refs::validate_content_ref(reference)?;
    }
    if snapshot.snapshot_ref != identity.snapshot_ref(snapshot)?
        || snapshot.last_included_index == INITIAL_COMMIT_INDEX
        || snapshot.last_included_term == INITIAL_TERM
        || snapshot.completed_requests.len() > MAX_REPLICA_LOG_ENTRIES
    {
        return Err(crate::denial::ReplicaError::Denied("Raft wire snapshot identity or boundary is invalid"));
    }
    for (request_ref, index) in &snapshot.completed_requests {
        crate::denial::refs::validate_content_ref(request_ref)?;
        if *index == INITIAL_COMMIT_INDEX || *index > snapshot.last_included_index {
            return Err(crate::denial::ReplicaError::Denied("Raft wire snapshot request index is invalid"));
        }
    }
    Ok(())
}

fn validate_recovery(
    envelope: &ReplicaMessageEnvelope,
    identity: &dyn crate::identity::ReplicaIdentity,
) -> crate::denial::Result<()> {
    let message = &envelope.message;
    let (round_ref, from_index) = match message {
        Message::RecoveryQuery {
            round_ref, from_index, ..
        }
        | Message::RecoveryReport {
            round_ref, from_index, ..
        } => (round_ref, *from_index),
        Message::RequestVote { .. }
        | Message::VoteResponse { .. }
        | Message::AppendEntries { .. }
        | Message::AppendResponse { .. }
        | Message::ReadProbe { .. }
        | Message::ReadAcknowledgement { .. }
        | Message::InstallSnapshot { .. }
        | Message::SnapshotResponse { .. } => return Ok(()),
    };
    crate::denial::refs::validate_content_ref(round_ref)?;
    if from_index == 0 {
        return Err(crate::denial::ReplicaError::Denied("Raft recovery start index must be positive"));
    }
    if matches!(message, Message::RecoveryQuery {
        term: 0,
        is_barrier: true,
        ..
    }) {
        return Err(crate::denial::ReplicaError::Denied("Raft recovery barrier term must be positive"));
    }
    let Message::RecoveryReport {
        term,
        observation_ref,
        last_log_index,
        commit_index,
        entries,
        snapshot,
        ..
    } = message
    else {
        return Ok(());
    };
    crate::denial::refs::validate_content_ref(observation_ref)?;
    if commit_index > last_log_index || entries.len() > MAX_REPLICA_MESSAGE_ENTRIES {
        return Err(crate::denial::ReplicaError::Denied("Raft recovery commit or page bound is invalid"));
    }
    let boundary = match snapshot {
        Some(snapshot) => validate_recovery_snapshot(envelope, snapshot, identity)?,
        None => 0,
    };
    validate_recovery_page(&RecoveryPage {
        term: *term,
        from_index,
        last_log_index: *last_log_index,
        boundary,
        entries,
    })
}

/// Check a recovery report's snapshot against its envelope; returns the snapshot boundary.
fn validate_recovery_snapshot(
    envelope: &ReplicaMessageEnvelope,
    snapshot: &ReplicaSnapshot,
    identity: &dyn crate::identity::ReplicaIdentity,
) -> crate::denial::Result<u64> {
    validate_snapshot(snapshot, identity)?;
    let Message::RecoveryReport {
        term,
        commit_index,
        config_epoch,
        fencing_epoch,
        ..
    } = &envelope.message
    else {
        return Err(crate::denial::ReplicaError::Denied("expected a recovery report"));
    };
    if snapshot.group_binding_ref != envelope.group_binding_ref
        || snapshot.config_epoch != *config_epoch
        || snapshot.fencing_epoch != *fencing_epoch
        || snapshot.last_included_term > *term
        || snapshot.last_included_index > *commit_index
    {
        return Err(crate::denial::ReplicaError::Denied(
            "Raft recovery snapshot binding, epochs, or boundary is invalid",
        ));
    }
    Ok(snapshot.last_included_index)
}

/// One page of a recovery report: the entries from `from_index` above the snapshot `boundary`.
struct RecoveryPage<'a> {
    term: u64,
    from_index: u64,
    last_log_index: u64,
    boundary: u64,
    entries: &'a [ReplicatedEntry],
}

fn validate_recovery_page(page: &RecoveryPage<'_>) -> crate::denial::Result<()> {
    let bound = u64::try_from(MAX_REPLICA_LOG_ENTRIES)
        .map_err(|_| crate::denial::ReplicaError::Overflow("Raft recovery log bound exceeds u64"))?;
    if page.last_log_index.saturating_sub(page.boundary) > bound {
        return Err(crate::denial::ReplicaError::Denied("Raft recovery log exceeds the bound"));
    }
    let start = page.from_index.max(
        page.boundary
            .checked_add(1)
            .ok_or(crate::denial::ReplicaError::Overflow("Raft recovery snapshot boundary overflow"))?,
    );
    let remaining = if start > page.last_log_index {
        0
    } else {
        page.last_log_index - start + 1
    };
    let expected_len = usize::try_from(remaining)
        .map_err(|_| crate::denial::ReplicaError::Overflow("Raft recovery page span exceeds usize"))?
        .min(MAX_REPLICA_MESSAGE_ENTRIES);
    if page.entries.len() != expected_len {
        return Err(crate::denial::ReplicaError::Denied(
            "Raft recovery page is incomplete or exceeds the reported tail",
        ));
    }
    let mut expected = start;
    for entry in page.entries {
        if entry.index != expected || entry.term == 0 || entry.term > page.term {
            return Err(crate::denial::ReplicaError::Denied(
                "Raft recovery page is non-contiguous or uses an invalid term",
            ));
        }
        validate_entry_refs(entry)?;
        expected = expected.saturating_add(1);
    }
    Ok(())
}

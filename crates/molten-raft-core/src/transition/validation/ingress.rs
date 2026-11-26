//! Admission of a peer envelope against the receiving replica's binding, membership and epochs.

use super::*;

pub(in crate::transition) fn validate_message_envelope(
    state: &ReplicaState,
    envelope: &ReplicaMessageEnvelope,
) -> crate::denial::Result<()> {
    validate_content_ref(&envelope.group_binding_ref, "Raft message group binding ref")?;
    if envelope.group_binding_ref != state.profile.group_binding_ref {
        return Err(crate::denial::ReplicaError::Denied("Raft message uses a substituted group binding"));
    }
    if envelope.service_generation != state.profile.service_generation {
        return Err(crate::denial::ReplicaError::Denied("Raft message uses a stale service generation"));
    }
    if envelope.to != state.node_id || envelope.from == state.node_id {
        return Err(crate::denial::ReplicaError::Denied("Raft message endpoint identity is invalid"));
    }
    if !state.membership.voters.contains(&envelope.from) || !state.membership.voters.contains(&envelope.to) {
        return Err(crate::denial::ReplicaError::Denied("Raft message endpoint is outside admitted membership"));
    }
    if envelope.message.term() == 0
        && !matches!(envelope.message, Message::RecoveryQuery { .. } | Message::RecoveryReport { .. })
    {
        return Err(crate::denial::ReplicaError::Denied("Raft protocol message cannot use term zero"));
    }
    if envelope.message.config_epoch() != state.membership.config_epoch
        || envelope.message.fencing_epoch() != state.profile.fencing_epoch
    {
        return Err(crate::denial::ReplicaError::Denied("Raft message uses a stale configuration or fencing epoch"));
    }
    validate_sender(envelope)?;
    validate_request_ref(&envelope.message)
}

fn validate_sender(envelope: &ReplicaMessageEnvelope) -> crate::denial::Result<()> {
    let embedded_sender = match &envelope.message {
        Message::RecoveryQuery { requester_id, .. } => requester_id,
        Message::RecoveryReport { peer_id, .. } => peer_id,
        Message::RequestVote { candidate_id, .. } => candidate_id,
        Message::VoteResponse { voter_id, .. } => voter_id,
        Message::AppendEntries { leader_id, .. }
        | Message::ReadProbe { leader_id, .. }
        | Message::InstallSnapshot { leader_id, .. } => leader_id,
        Message::AppendResponse { follower_id, .. }
        | Message::ReadAcknowledgement { follower_id, .. }
        | Message::SnapshotResponse { follower_id, .. } => follower_id,
    };
    if embedded_sender != &envelope.from {
        return Err(crate::denial::ReplicaError::Denied("Raft message sender does not match its canonical envelope"));
    }
    Ok(())
}

fn validate_request_ref(message: &Message) -> crate::denial::Result<()> {
    match message {
        Message::ReadProbe { request_ref, .. } | Message::ReadAcknowledgement { request_ref, .. } => {
            validate_content_ref(request_ref, "Raft read request ref")
        }
        Message::InstallSnapshot { snapshot, .. } => {
            validate_content_ref(&snapshot.snapshot_ref, "Raft installed snapshot ref")
        }
        Message::RecoveryQuery { .. }
        | Message::RecoveryReport { .. }
        | Message::RequestVote { .. }
        | Message::VoteResponse { .. }
        | Message::AppendEntries { .. }
        | Message::AppendResponse { .. }
        | Message::SnapshotResponse { .. } => Ok(()),
    }
}

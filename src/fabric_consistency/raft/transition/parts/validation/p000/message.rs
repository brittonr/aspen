pub(super) fn validate_message_envelope(
    state: &ReplicaState,
    envelope: &ReplicaMessageEnvelope,
) -> crate::error::Result<()> {
    validate_content_ref(&envelope.group_binding_ref, "Raft message group binding ref")?;
    if envelope.group_binding_ref != state.profile.group_binding_ref {
        return Err(crate::error::Failure::invalid_harness("Raft message uses a substituted group binding"));
    }
    if envelope.service_generation != state.profile.service_generation {
        return Err(crate::error::Failure::invalid_harness("Raft message uses a stale service generation"));
    }
    if envelope.to != state.node_id || envelope.from == state.node_id {
        return Err(crate::error::Failure::invalid_harness("Raft message endpoint identity is invalid"));
    }
    if !state.membership.voters.contains(&envelope.from) || !state.membership.voters.contains(&envelope.to) {
        return Err(crate::error::Failure::invalid_harness("Raft message endpoint is outside admitted membership"));
    }
    if envelope.message.term() == 0
        && !matches!(envelope.message, Message::RecoveryQuery { .. } | Message::RecoveryReport { .. })
    {
        return Err(crate::error::Failure::invalid_harness("Raft protocol message cannot use term zero"));
    }
    if envelope.message.config_epoch() != state.membership.config_epoch
        || envelope.message.fencing_epoch() != state.profile.fencing_epoch
    {
        return Err(crate::error::Failure::invalid_harness(
            "Raft message uses a stale configuration or fencing epoch",
        ));
    }
    validate_message_sender(envelope)?;
    validate_message_request_ref(&envelope.message)
}

fn validate_message_sender(envelope: &ReplicaMessageEnvelope) -> crate::error::Result<()> {
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
        return Err(crate::error::Failure::invalid_harness(
            "Raft message sender does not match its canonical envelope",
        ));
    }
    Ok(())
}

fn validate_message_request_ref(message: &Message) -> crate::error::Result<()> {
    match message {
        Message::ReadProbe { request_ref, .. } | Message::ReadAcknowledgement { request_ref, .. } => {
            validate_content_ref(request_ref, "Raft read request ref")
        }
        Message::InstallSnapshot { snapshot, .. } => {
            validate_content_ref(&snapshot.snapshot_ref, "Raft installed snapshot ref")
        }
        _ => Ok(()),
    }
}
/// Untrusted canonical messages used only to choose a resource policy, never read evidence.
/// All numeric fields in control traffic use their maximum wire width. The first proposal
/// uses the minimum legal indices/terms, so the separation does not depend on an election race.
pub fn uncommitted_frame_budget() -> super::super::Result<u64> {
    let reference = super::super::reference("untrusted-frame-sizing-only");
    let mut budget = 0;
    for message in election_controls(&reference)
        .into_iter()
        .chain(read_controls(&reference))
        .chain(append_controls(&reference))
    {
        budget = budget.max(encoded_size(&reference, message)?);
    }
    let append = encoded_size(&reference, first_append(&reference))?;
    if append <= budget {
        return Err("canonical control and single-entry append frames have no separating budget".into());
    }
    Ok(budget)
}

fn election_controls(reference: &str) -> [molten::fabric_consistency::raft::Message; 2] {
    let maximum = u64::MAX;
    [
        molten::fabric_consistency::raft::Message::RequestVote {
            term: maximum,
            candidate_id: reference.to_owned(),
            last_log_index: maximum,
            last_log_term: maximum,
            config_epoch: maximum,
            fencing_epoch: maximum,
        },
        molten::fabric_consistency::raft::Message::VoteResponse {
            term: maximum,
            voter_id: reference.to_owned(),
            granted: true,
            config_epoch: maximum,
            fencing_epoch: maximum,
        },
    ]
}

fn read_controls(reference: &str) -> [molten::fabric_consistency::raft::Message; 2] {
    let maximum = u64::MAX;
    [
        molten::fabric_consistency::raft::Message::ReadProbe {
            term: maximum,
            leader_id: reference.to_owned(),
            request_ref: reference.to_owned(),
            required_index: maximum,
            config_epoch: maximum,
            fencing_epoch: maximum,
        },
        molten::fabric_consistency::raft::Message::ReadAcknowledgement {
            term: maximum,
            follower_id: reference.to_owned(),
            request_ref: reference.to_owned(),
            config_epoch: maximum,
            fencing_epoch: maximum,
        },
    ]
}

fn append_controls(reference: &str) -> [molten::fabric_consistency::raft::Message; 2] {
    let maximum = u64::MAX;
    [
        molten::fabric_consistency::raft::Message::AppendEntries {
            term: maximum,
            leader_id: reference.to_owned(),
            prev_log_index: maximum - 1,
            prev_log_term: maximum,
            entries: Vec::new(),
            leader_commit: maximum,
            config_epoch: maximum,
            fencing_epoch: maximum,
        },
        molten::fabric_consistency::raft::Message::AppendResponse {
            term: maximum,
            follower_id: reference.to_owned(),
            success: true,
            request_prev_log_index: maximum,
            match_index: maximum,
            conflict_index: maximum,
            config_epoch: maximum,
            fencing_epoch: maximum,
        },
    ]
}

fn encoded_size(reference: &str, message: molten::fabric_consistency::raft::Message) -> super::super::Result<u64> {
    let wire =
        molten::fabric_consistency::raft::replica_message(&molten::fabric_consistency::raft::ReplicaMessageEnvelope {
            group_binding_ref: reference.to_owned(),
            service_generation: 1,
            from: reference.to_owned(),
            to: super::super::reference("untrusted-frame-sizing-peer"),
            message,
        })?;
    Ok(u64::try_from(wire.bytes.len())?)
}

fn first_append(reference: &str) -> molten::fabric_consistency::raft::Message {
    molten::fabric_consistency::raft::Message::AppendEntries {
        term: 1,
        leader_id: reference.to_owned(),
        prev_log_index: 0,
        prev_log_term: 0,
        entries: vec![molten::fabric_consistency::raft::ReplicatedEntry {
            index: 1,
            term: 1,
            request_ref: reference.to_owned(),
            command_ref: reference.to_owned(),
            command_schema_ref: reference.to_owned(),
        }],
        leader_commit: 0,
        config_epoch: 1,
        fencing_epoch: 1,
    }
}

use super::*;
use crate::error::MoltenError;
use crate::error::Result;

pub(super) struct ReadProbeInput {
    pub from: String,
    pub term: u64,
    pub leader_id: String,
    pub request_ref: String,
    pub required_index: u64,
}

pub(super) struct ReadAcknowledgementInput {
    pub from: String,
    pub term: u64,
    pub follower_id: String,
    pub request_ref: String,
}

pub(super) fn handle_read_probe(transition: &mut MessageTransition, input: ReadProbeInput) -> Result<()> {
    if input.term < transition.next.current_term {
        return Ok(());
    }
    if input.leader_id != input.from {
        return Err(MoltenError::invalid_harness("Raft read probe leader does not match its sender"));
    }
    if input.required_index > support::last_log_index(&transition.next) {
        return Err(MoltenError::invalid_harness("Raft read probe requires an unavailable log boundary"));
    }
    if transition.next.role == ReplicaRole::Leader && input.leader_id != transition.next.node_id {
        return Err(MoltenError::invalid_harness("Raft read probe observed two leaders in one term"));
    }
    transition.next.role = ReplicaRole::Follower;
    transition.next.leader_id = Some(input.leader_id);
    transition.next.pending_reads.clear();
    transition
        .effects
        .push(support::send_effect(&transition.next, input.from, RaftMessage::ReadAcknowledgement {
            term: transition.next.current_term,
            follower_id: transition.next.node_id.clone(),
            request_ref: input.request_ref,
            config_epoch: transition.next.membership.config_epoch,
            fencing_epoch: transition.next.profile.fencing_epoch,
        }));
    let timer_effect = support::arm_election_timer(&mut transition.next)?;
    transition.effects.push(timer_effect);
    Ok(())
}

pub(super) fn handle_read_acknowledgement(
    transition: &mut MessageTransition,
    input: ReadAcknowledgementInput,
) -> Result<()> {
    if input.term < transition.next.current_term || transition.next.role != ReplicaRole::Leader {
        return Ok(());
    }
    if input.follower_id != input.from {
        return Err(MoltenError::invalid_harness("Raft read acknowledgement follower does not match its sender"));
    }
    let Some(pending) = transition.next.pending_reads.get_mut(&input.request_ref) else {
        return Ok(());
    };
    if pending.term != transition.next.current_term {
        return Ok(());
    }
    pending.acknowledgements.insert(input.follower_id);
    if pending.acknowledgements.len() < STATIC_QUORUM_COUNT {
        return Ok(());
    }
    let completed = transition
        .next
        .pending_reads
        .remove(&input.request_ref)
        .ok_or_else(|| MoltenError::invalid_harness("Raft pending read disappeared before completion"))?;
    transition.next.quorum_confirmed_term = Some(transition.next.current_term);
    transition.effects.push(ReplicaEffect::ReadOutcome {
        request_ref: completed.request_ref,
        mode: crate::fabric_consistency::ConsistencyReadMode::Linearizable,
        disposition: ReadDisposition::Current,
        observed_index: completed.required_index,
    });
    Ok(())
}

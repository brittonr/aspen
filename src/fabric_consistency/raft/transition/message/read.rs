use super::*;

pub(super) struct ProbeInput {
    pub from: String,
    pub term: u64,
    pub leader_id: String,
    pub request_ref: String,
}

pub(super) struct AcknowledgementInput {
    pub from: String,
    pub term: u64,
    pub follower_id: String,
    pub request_ref: String,
}

/// Confirm the prober's leadership for its term (Raft ReadIndex).
///
/// The leader alone serves the read at its own commit index, so a follower acknowledges a
/// current-term leader whether or not its log has caught up to that index. The probe's higher term
/// was already adopted by `observe_higher_term` and is persisted with this acknowledgement.
// r[impl molten.consensus.raft_replication_and_recovery_liveness]
pub(super) fn handle_probe(transition: &mut MessagePlan, input: ProbeInput) -> crate::error::Result<()> {
    if input.term < transition.next.current_term {
        return Ok(());
    }
    if input.leader_id != input.from {
        return Err(crate::error::Failure::invalid_harness("Raft read probe leader does not match its sender"));
    }
    if transition.next.role == ReplicaRole::Leader && input.leader_id != transition.next.node_id {
        return Err(crate::error::Failure::invalid_harness("Raft read probe observed two leaders in one term"));
    }
    transition.next.role = ReplicaRole::Follower;
    transition.next.leader_id = Some(input.leader_id);
    transition.next.pending_reads.clear();
    transition
        .effects
        .push(support::send_effect(&transition.next, input.from, Message::ReadAcknowledgement {
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

pub(super) fn handle_acknowledgement(
    transition: &mut MessagePlan,
    input: AcknowledgementInput,
) -> crate::error::Result<()> {
    if input.term < transition.next.current_term || transition.next.role != ReplicaRole::Leader {
        return Ok(());
    }
    if input.follower_id != input.from {
        return Err(crate::error::Failure::invalid_harness(
            "Raft read acknowledgement follower does not match its sender",
        ));
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
        .ok_or_else(|| crate::error::Failure::invalid_harness("Raft pending read disappeared before completion"))?;
    transition.next.quorum_confirmed_term = Some(transition.next.current_term);
    transition.effects.push(ReplicaEffect::ReadOutcome {
        request_ref: completed.request_ref,
        mode: crate::fabric_consistency::ReadMode::Linearizable,
        disposition: ReadDisposition::Current,
        observed_index: completed.required_index,
    });
    Ok(())
}

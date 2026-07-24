use super::*;
use crate::error::MoltenError;
use crate::error::Result;

pub(super) struct InstallSnapshotInput {
    pub from: String,
    pub term: u64,
    pub leader_id: String,
    pub snapshot: ReplicaSnapshot,
}

pub(super) struct SnapshotResponseInput {
    pub from: String,
    pub term: u64,
    pub follower_id: String,
    pub snapshot_index: u64,
    pub is_accepted: bool,
}

pub(super) fn handle_install_snapshot(transition: &mut MessageTransition, input: InstallSnapshotInput) -> Result<()> {
    if input.leader_id != input.from {
        return Err(MoltenError::invalid_harness("Raft snapshot leader does not match its sender"));
    }
    if input.term < transition.next.current_term {
        transition.effects.push(snapshot_response(
            &transition.next,
            input.from,
            input.snapshot.last_included_index,
            false,
        ));
        return Ok(());
    }
    validate_snapshot_binding(&transition.next, &input.snapshot)?;
    if input.snapshot.last_included_term > transition.next.current_term {
        return Err(MoltenError::invalid_harness("Raft installed snapshot term exceeds the leader term"));
    }
    if transition.next.role == ReplicaRole::Leader && input.leader_id != transition.next.node_id {
        return Err(MoltenError::invalid_harness("Raft snapshot observed two leaders in one term"));
    }
    let snapshot_index = input.snapshot.last_included_index;
    transition.next.role = ReplicaRole::Follower;
    transition.next.leader_id = Some(input.leader_id);
    transition.next.pending_reads.clear();
    if snapshot_index > transition.next.commit_index {
        install_snapshot(transition, input.snapshot)?;
    }
    transition.effects.push(snapshot_response(&transition.next, input.from, snapshot_index, true));
    let timer_effect = support::arm_election_timer(&mut transition.next)?;
    transition.effects.push(timer_effect);
    Ok(())
}

fn validate_snapshot_binding(state: &ReplicaState, snapshot: &ReplicaSnapshot) -> Result<()> {
    if snapshot.group_binding_ref != state.profile.group_binding_ref
        || snapshot.membership_ref != state.membership.membership_ref
        || snapshot.config_epoch != state.membership.config_epoch
        || snapshot.fencing_epoch != state.profile.fencing_epoch
        || snapshot.snapshot_ref != snapshot_ref(snapshot)?
    {
        return Err(MoltenError::invalid_harness("Raft installed snapshot binding or identity mismatch"));
    }
    Ok(())
}

fn install_snapshot(transition: &mut MessageTransition, snapshot: ReplicaSnapshot) -> Result<()> {
    if snapshot.last_included_index == INITIAL_COMMIT_INDEX || snapshot.last_included_term == INITIAL_TERM {
        return Err(MoltenError::invalid_harness("Raft installed snapshot boundary must be positive"));
    }
    transition.next.log.clear();
    transition.next.commit_index = snapshot.last_included_index;
    transition.next.last_applied = snapshot.last_included_index;
    transition.next.completed_requests.clone_from(&snapshot.completed_requests);
    transition.next.snapshot = Some(snapshot.clone());
    transition.effects.push(ReplicaEffect::PersistSnapshot {
        snapshot: snapshot.clone(),
    });
    transition.effects.push(ReplicaEffect::RestoreApplicationSnapshot { snapshot });
    Ok(())
}

fn snapshot_response(state: &ReplicaState, to: String, snapshot_index: u64, is_accepted: bool) -> ReplicaEffect {
    support::send_effect(state, to, RaftMessage::SnapshotResponse {
        term: state.current_term,
        follower_id: state.node_id.clone(),
        snapshot_index,
        accepted: is_accepted,
        config_epoch: state.membership.config_epoch,
        fencing_epoch: state.profile.fencing_epoch,
    })
}

pub(super) fn handle_snapshot_response(transition: &mut MessageTransition, input: SnapshotResponseInput) -> Result<()> {
    if input.term < transition.next.current_term || transition.next.role != ReplicaRole::Leader || !input.is_accepted {
        return Ok(());
    }
    if input.follower_id != input.from {
        return Err(MoltenError::invalid_harness("Raft snapshot response follower does not match its sender"));
    }
    let local_snapshot_index = transition
        .next
        .snapshot
        .as_ref()
        .map_or(INITIAL_COMMIT_INDEX, |snapshot| snapshot.last_included_index);
    let matched = transition.next.match_index.get(&input.follower_id).copied().unwrap_or(INITIAL_COMMIT_INDEX);
    if input.snapshot_index != local_snapshot_index || input.snapshot_index < matched {
        return Ok(());
    }
    let next_index = input
        .snapshot_index
        .checked_add(NEXT_LOG_INDEX_STEP)
        .ok_or_else(|| MoltenError::invalid_harness("Raft snapshot response index overflow"))?;
    transition.next.match_index.insert(input.follower_id.clone(), input.snapshot_index);
    transition.next.next_index.insert(input.follower_id.clone(), next_index);
    transition.effects.push(support::append_effect_for(&transition.next, input.follower_id)?);
    Ok(())
}

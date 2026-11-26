mod boundary;

use super::*;

fn entry_fact(
    entry: &ReplicatedEntry,
    identity: &dyn crate::identity::ReplicaIdentity,
) -> crate::denial::Result<molten_core::consensus_recovery::EntryIdentity> {
    Ok(molten_core::consensus_recovery::EntryIdentity {
        term: entry.term,
        entry_ref: identity.entry_ref(entry)?,
    })
}

fn snapshot_fact(snapshot: &ReplicaSnapshot) -> molten_core::consensus_recovery::SnapshotFact {
    molten_core::consensus_recovery::SnapshotFact {
        last_included_index: snapshot.last_included_index,
        last_included_term: snapshot.last_included_term,
        content_ref: snapshot.snapshot_ref.clone(),
    }
}

fn entry_facts(
    entries: &[ReplicatedEntry],
    identity: &dyn crate::identity::ReplicaIdentity,
) -> crate::denial::Result<Vec<(u64, molten_core::consensus_recovery::EntryIdentity)>> {
    entries.iter().map(|entry| Ok((entry.index, entry_fact(entry, identity)?))).collect()
}

fn input(
    state: &ReplicaState,
    identity: &dyn crate::identity::ReplicaIdentity,
) -> crate::denial::Result<molten_core::consensus_recovery::ReconcileInput> {
    let ReplicaRejoin::Recovering(round) = &state.rejoin else {
        return Err(crate::denial::ReplicaError::Denied("reconciliation requires a recovery round"));
    };
    let mut peers = Vec::with_capacity(STATIC_VOTER_COUNT - 1);
    for (peer_id, observation) in round.peers.iter().filter(|(_, observation)| observation.is_complete) {
        peers.push(molten_core::consensus_recovery::PeerRecovery {
            peer_id: peer_id.clone(),
            current_term: observation.current_term,
            commit_index: observation.commit_index,
            entries: entry_facts(&observation.entries, identity)?.into_iter().collect(),
            snapshot: observation.snapshot.as_deref().map(snapshot_fact),
            is_complete: true,
        });
    }
    let local_entries = entry_facts(&state.log, identity)?
        .into_iter()
        .map(|(index, fact)| (index, molten_core::consensus_recovery::LocalEntry::Present(fact)))
        .collect();
    Ok(molten_core::consensus_recovery::ReconcileInput {
        participant_id: state.node_id.clone(),
        voters: state.membership.voters.iter().cloned().collect(),
        current_term: state.current_term,
        local_commit_index: state.commit_index,
        local_entries,
        // A repaired store cannot prove that it retained the latest vote record.
        local_hard_state: molten_core::consensus_recovery::LocalHardState::Lost,
        local_snapshot: state.snapshot.as_ref().map(snapshot_fact),
        peers,
        is_persistent_state_suspect: true,
        is_storage_serving: true,
        is_membership_current: true,
        is_log_or_snapshot_validated: true,
    })
}

// r[impl molten.consensus.storage_fault_recovery]
// r[impl molten.consensus.repair_admission]
pub(super) fn reconcile(transition: &mut MessagePlan<'_>) -> crate::denial::Result<()> {
    let outcome =
        molten_core::consensus_recovery::reconcile_participant(&input(&transition.next, transition.identity)?);
    let ReplicaRejoin::Recovering(round) = &transition.next.rejoin else {
        return Err(crate::denial::ReplicaError::Denied("repair admission requires an active round"));
    };
    let reporters = round.peers.values().filter(|peer| peer.is_complete).count();
    if reporters + STATIC_QUORUM_COUNT <= transition.next.membership.voters.len() {
        return apply_available(transition, &outcome);
    }
    if round.fence_term.is_none() {
        return establish_barrier(transition);
    }
    if round.fence_term != Some(transition.next.current_term)
        || round
            .peers
            .values()
            .any(|peer| peer.is_complete && peer.current_term != transition.next.current_term)
    {
        return establish_barrier(transition);
    }
    for effect in &outcome.effects {
        match effect {
            molten_core::consensus_recovery::RecoveryEffect::InstallSnapshot { from_peer, snapshot } => {
                return boundary::install_snapshot(transition, from_peer, snapshot, &outcome.decisions);
            }
            molten_core::consensus_recovery::RecoveryEffect::FetchEntry {
                from_peer,
                index,
                entry,
            } => {
                return fetch(transition, from_peer, *index, entry);
            }
            molten_core::consensus_recovery::RecoveryEffect::RemoveSuffixFrom { index } => {
                return remove_suffix(transition, *index);
            }
            molten_core::consensus_recovery::RecoveryEffect::QueryPeers { .. } => {}
        }
    }
    if outcome.is_reconciled {
        admit(transition)?;
    }
    Ok(())
}

fn remove_suffix(transition: &mut MessagePlan<'_>, index: u64) -> crate::denial::Result<()> {
    if index <= transition.next.commit_index {
        return Err(crate::denial::ReplicaError::Denied("recovery cannot remove committed history"));
    }
    transition.next.log.retain(|entry| entry.index < index);
    transition.effects.push(ReplicaEffect::PersistEntries {
        truncate_from: Some(index),
        entries: Vec::new(),
    });
    transition.effects.push(ReplicaEffect::FlushLog {
        through_index: support::last_log_index(&transition.next),
    });
    Ok(())
}

fn observation<'a>(state: &'a ReplicaState, peer: &str) -> crate::denial::Result<&'a RecoveryObservation> {
    let ReplicaRejoin::Recovering(round) = &state.rejoin else {
        return Err(crate::denial::ReplicaError::Denied("repair requires a recovery round"));
    };
    round
        .peers
        .get(peer)
        .filter(|report| report.is_complete)
        .ok_or(crate::denial::ReplicaError::Denied("repair source has no complete observation"))
}

fn fetch(
    transition: &mut MessagePlan<'_>,
    peer: &str,
    index: u64,
    expected: &molten_core::consensus_recovery::EntryIdentity,
) -> crate::denial::Result<()> {
    if index <= transition.next.commit_index {
        return Err(crate::denial::ReplicaError::Denied("recovery cannot replace committed history"));
    }
    let source = observation(&transition.next, peer)?;
    let entry = source
        .entries
        .iter()
        .find(|entry| entry.index == index)
        .ok_or(crate::denial::ReplicaError::Denied("recovery source lacks planned entry"))?;
    if entry_fact(entry, transition.identity)? != *expected || index > source.commit_index {
        return Err(crate::denial::ReplicaError::Denied("recovery fetched entry failed committed identity check"));
    }
    let entries = source
        .entries
        .iter()
        .filter(|entry| entry.index >= index && entry.index <= source.commit_index)
        .take(MAX_REPLICA_MESSAGE_ENTRIES)
        .cloned()
        .collect::<Vec<_>>();
    transition.next.log.retain(|entry| entry.index < index);
    transition.next.log.extend(entries.iter().cloned());
    transition.effects.push(ReplicaEffect::PersistEntries {
        truncate_from: Some(index),
        entries,
    });
    transition.effects.push(ReplicaEffect::FlushLog {
        through_index: support::last_log_index(&transition.next),
    });
    Ok(())
}

fn admit(transition: &mut MessagePlan<'_>) -> crate::denial::Result<()> {
    let ReplicaRejoin::Recovering(round) = &transition.next.rejoin else {
        return Err(crate::denial::ReplicaError::Denied("admission requires a recovery round"));
    };
    let commit_index = round
        .peers
        .values()
        .filter(|peer| peer.is_complete)
        .map(|peer| peer.commit_index)
        .max()
        .unwrap_or(transition.next.commit_index)
        .max(transition.next.commit_index);
    if commit_index > support::last_log_index(&transition.next) {
        return Err(crate::denial::ReplicaError::Denied("recovery admission lacks committed entries"));
    }
    if commit_index > transition.next.commit_index {
        let entries = transition
            .next
            .log
            .iter()
            .filter(|entry| entry.index > transition.next.last_applied && entry.index <= commit_index)
            .cloned()
            .collect::<Vec<_>>();
        transition.effects.push(ReplicaEffect::PersistCommit {
            through_index: commit_index,
        });
        transition.next.commit_index = commit_index;
        transition.next.last_applied = commit_index;
        record_completed_requests(&mut transition.next, &entries);
        transition.effects.push(ReplicaEffect::ApplyCommitted { entries });
    }
    // Persist a self vote as a conservative fence: an older vote may have been lost.
    // The next campaign advances the term before granting any voting authority.
    transition.next.voted_for = Some(transition.next.node_id.clone());
    transition.persist_hard_state = true;
    transition.next.rejoin = ReplicaRejoin::VoteFenced {
        through_term: transition.next.current_term,
    };
    let timer = support::arm_election_timer(&mut transition.next, transition.identity)?;
    transition.effects.push(timer);
    Ok(())
}

fn apply_available(
    transition: &mut MessagePlan<'_>,
    outcome: &molten_core::consensus_recovery::Reconciliation,
) -> crate::denial::Result<()> {
    for effect in &outcome.effects {
        match effect {
            molten_core::consensus_recovery::RecoveryEffect::FetchEntry {
                from_peer,
                index,
                entry,
            } => {
                return fetch(transition, from_peer, *index, entry);
            }
            molten_core::consensus_recovery::RecoveryEffect::InstallSnapshot { from_peer, snapshot } => {
                return boundary::install_snapshot(transition, from_peer, snapshot, &outcome.decisions);
            }
            molten_core::consensus_recovery::RecoveryEffect::RemoveSuffixFrom { .. }
            | molten_core::consensus_recovery::RecoveryEffect::QueryPeers { .. } => {}
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;

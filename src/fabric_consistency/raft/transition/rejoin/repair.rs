use super::*;

fn identity(entry: &ReplicatedEntry) -> crate::error::Result<molten_core::consensus_recovery::EntryIdentity> {
    Ok(molten_core::consensus_recovery::EntryIdentity {
        term: entry.term,
        entry_ref: canonical::entry_identity_ref(entry)?,
    })
}

fn snapshot_fact(snapshot: &ReplicaSnapshot) -> molten_core::consensus_recovery::SnapshotFact {
    molten_core::consensus_recovery::SnapshotFact {
        last_included_index: snapshot.last_included_index,
        last_included_term: snapshot.last_included_term,
        content_ref: snapshot.snapshot_ref.clone(),
    }
}

fn input(state: &ReplicaState) -> crate::error::Result<molten_core::consensus_recovery::ReconcileInput> {
    let ReplicaRejoin::Recovering(round) = &state.rejoin else {
        return Err(crate::error::Failure::invalid_harness("reconciliation requires a recovery round"));
    };
    let mut peers = Vec::with_capacity(STATIC_VOTER_COUNT - 1);
    for (peer_id, observation) in &round.peers {
        if !observation.is_complete {
            continue;
        }
        let entries = observation
            .entries
            .iter()
            .map(|entry| Ok((entry.index, identity(entry)?)))
            .collect::<crate::error::Result<_>>()?;
        peers.push(molten_core::consensus_recovery::PeerRecovery {
            peer_id: peer_id.clone(),
            current_term: observation.current_term,
            commit_index: observation.commit_index,
            entries,
            snapshot: observation.snapshot.as_deref().map(snapshot_fact),
            is_complete: true,
        });
    }
    let local_entries = state
        .log
        .iter()
        .map(|entry| Ok((entry.index, molten_core::consensus_recovery::LocalEntry::Present(identity(entry)?))))
        .collect::<crate::error::Result<_>>()?;
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
pub(super) fn reconcile(transition: &mut MessagePlan) -> crate::error::Result<()> {
    let outcome = molten_core::consensus_recovery::reconcile_participant(&input(&transition.next)?);
    let ReplicaRejoin::Recovering(round) = &transition.next.rejoin else {
        return Err(crate::error::Failure::invalid_harness("repair admission requires an active round"));
    };
    let reporters = round.peers.values().filter(|peer| peer.is_complete).count();
    if reporters + STATIC_QUORUM_COUNT <= transition.next.membership.voters.len() {
        return apply_available(transition, &outcome.effects);
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
                install_snapshot(transition, from_peer, snapshot)?;
                return Ok(());
            }
            molten_core::consensus_recovery::RecoveryEffect::FetchEntry {
                from_peer,
                index,
                entry,
            } => {
                fetch(transition, from_peer, *index, entry)?;
                return Ok(());
            }
            molten_core::consensus_recovery::RecoveryEffect::RemoveSuffixFrom { index } => {
                if *index <= transition.next.commit_index {
                    return Err(crate::error::Failure::invalid_harness("recovery cannot remove committed history"));
                }
                transition.next.log.retain(|entry| entry.index < *index);
                transition.effects.push(ReplicaEffect::PersistEntries {
                    truncate_from: Some(*index),
                    entries: Vec::new(),
                });
                transition.effects.push(ReplicaEffect::FlushLog {
                    through_index: support::last_log_index(&transition.next),
                });
                return Ok(());
            }
            molten_core::consensus_recovery::RecoveryEffect::QueryPeers { .. } => {}
        }
    }
    if outcome.is_reconciled {
        admit(transition)?;
    }
    Ok(())
}

fn observation<'a>(state: &'a ReplicaState, peer: &str) -> crate::error::Result<&'a RecoveryObservation> {
    let ReplicaRejoin::Recovering(round) = &state.rejoin else {
        return Err(crate::error::Failure::invalid_harness("repair requires a recovery round"));
    };
    round
        .peers
        .get(peer)
        .filter(|report| report.is_complete)
        .ok_or_else(|| crate::error::Failure::invalid_harness("repair source has no complete observation"))
}

fn fetch(
    transition: &mut MessagePlan,
    peer: &str,
    index: u64,
    expected: &molten_core::consensus_recovery::EntryIdentity,
) -> crate::error::Result<()> {
    if index <= transition.next.commit_index {
        return Err(crate::error::Failure::invalid_harness("recovery cannot replace committed history"));
    }
    let source = observation(&transition.next, peer)?;
    let entry = source
        .entries
        .iter()
        .find(|entry| entry.index == index)
        .ok_or_else(|| crate::error::Failure::invalid_harness("recovery source lacks planned entry"))?;
    if identity(entry)? != *expected || index > source.commit_index {
        return Err(crate::error::Failure::invalid_harness("recovery fetched entry failed committed identity check"));
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

fn install_snapshot(
    transition: &mut MessagePlan,
    peer: &str,
    expected: &molten_core::consensus_recovery::SnapshotFact,
) -> crate::error::Result<()> {
    let snapshot = observation(&transition.next, peer)?
        .snapshot
        .as_deref()
        .ok_or_else(|| crate::error::Failure::invalid_harness("recovery source lacks planned snapshot"))?
        .clone();
    if snapshot_fact(&snapshot) != *expected {
        return Err(crate::error::Failure::invalid_harness("recovery snapshot identity mismatch"));
    }
    if snapshot.last_included_index < transition.next.commit_index {
        // Installing an older snapshot would roll back already applied state.
        return Ok(());
    }
    transition.next.log.retain(|entry| entry.index > snapshot.last_included_index);
    transition.next.commit_index = snapshot.last_included_index;
    transition.next.last_applied = snapshot.last_included_index;
    transition.next.completed_requests.clone_from(&snapshot.completed_requests);
    transition.next.snapshot = Some(snapshot.clone());
    transition.effects.push(ReplicaEffect::PersistSnapshot {
        snapshot: snapshot.clone(),
        is_suffix_retained: true,
    });
    transition.effects.push(ReplicaEffect::RestoreApplicationSnapshot { snapshot });
    Ok(())
}

fn admit(transition: &mut MessagePlan) -> crate::error::Result<()> {
    let ReplicaRejoin::Recovering(round) = &transition.next.rejoin else {
        return Err(crate::error::Failure::invalid_harness("admission requires a recovery round"));
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
        return Err(crate::error::Failure::invalid_harness("recovery admission lacks committed entries"));
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
        for entry in &entries {
            transition.next.completed_requests.insert(entry.request_ref.clone(), entry.index);
        }
        transition.effects.push(ReplicaEffect::ApplyCommitted { entries });
        transition.next.commit_index = commit_index;
        transition.next.last_applied = commit_index;
    }
    // Persist a self vote as a conservative fence: an older vote may have been lost.
    // The next campaign advances the term before granting any voting authority.
    transition.next.voted_for = Some(transition.next.node_id.clone());
    transition.persist_hard_state = true;
    transition.next.rejoin = ReplicaRejoin::VoteFenced {
        through_term: transition.next.current_term,
    };
    let timer = support::arm_election_timer(&mut transition.next)?;
    transition.effects.push(timer);
    Ok(())
}

fn apply_available(
    transition: &mut MessagePlan,
    effects: &[molten_core::consensus_recovery::RecoveryEffect],
) -> crate::error::Result<()> {
    for effect in effects {
        match effect {
            molten_core::consensus_recovery::RecoveryEffect::FetchEntry {
                from_peer,
                index,
                entry,
            } => {
                return fetch(transition, from_peer, *index, entry);
            }
            molten_core::consensus_recovery::RecoveryEffect::InstallSnapshot { from_peer, snapshot } => {
                return install_snapshot(transition, from_peer, snapshot);
            }
            molten_core::consensus_recovery::RecoveryEffect::RemoveSuffixFrom { .. }
            | molten_core::consensus_recovery::RecoveryEffect::QueryPeers { .. } => {}
        }
    }
    Ok(())
}

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
                install_snapshot(transition, from_peer, snapshot, &outcome.decisions)?;
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

/// The local log repaired for a peer snapshot whose boundary term it does not match.
enum MismatchedRepair {
    /// Some entry's commitment is unresolved, a committed replacement is not available yet, or the
    /// repaired log would break term order: keep all local evidence and wait.
    Deferred,
    /// Rewrite the log from `rewrite_from` with `entries`: committed replacements, the confirmed
    /// tail, and no established-uncommitted entry. `None` leaves the local log unchanged.
    Repaired {
        rewrite_from: Option<u64>,
        entries: Vec<ReplicatedEntry>,
    },
}

// Like a leader's InstallSnapshot, the local suffix survives whole when the local log agrees with
// the snapshot at its boundary. Otherwise the log is repaired from peer evidence first: an entry
// with a committed peer copy that differs is replaced by that copy, a confirmed (`KeepLocal`) entry
// is never dropped, and only entries peers established as uncommitted are truncated. The repair is
// one durable log mutation ahead of the snapshot, so a crash leaves either the original log or the
// repaired one, never a committed entry held only by peers. While any suffix entry is unresolved
// or its committed replacement is not yet reported, installation waits and the evidence is kept.
// r[impl molten.consensus.storage_fault_recovery]
// r[impl molten.consensus.raft_replication_and_recovery_liveness]
fn install_snapshot(
    transition: &mut MessagePlan,
    peer: &str,
    expected: &molten_core::consensus_recovery::SnapshotFact,
    decisions: &[(u64, molten_core::consensus_recovery::EntryRepairDecision)],
) -> crate::error::Result<()> {
    let snapshot = observation(&transition.next, peer)?
        .snapshot
        .as_deref()
        .ok_or_else(|| crate::error::Failure::invalid_harness("recovery source lacks planned snapshot"))?
        .clone();
    if snapshot_fact(&snapshot) != *expected {
        return Err(crate::error::Failure::invalid_harness("recovery snapshot identity mismatch"));
    }
    check_snapshot_evidence(&transition.next, &snapshot)?;
    if snapshot.last_included_index < transition.next.commit_index {
        // Installing an older snapshot would roll back already applied state.
        return Ok(());
    }
    // A local uncommitted boundary entry with the snapshot's term but another request is a local
    // storage fault, not committed evidence: it takes the verified repair path below.
    let has_matching_boundary = support::term_at(&transition.next, snapshot.last_included_index)
        == Some(snapshot.last_included_term)
        && !transition.next.log.iter().any(|entry| {
            entry.index == snapshot.last_included_index && contradicts_snapshot(&transition.next, &snapshot, entry)
        });
    let mut rewrite = None;
    if !has_matching_boundary {
        match repair_mismatched_log(&transition.next, &snapshot, decisions)? {
            MismatchedRepair::Deferred => return Ok(()),
            MismatchedRepair::Repaired {
                rewrite_from: Some(rewrite_from),
                entries,
            } => {
                transition.next.log.retain(|entry| entry.index < rewrite_from);
                transition.next.log.extend(entries.iter().cloned());
                rewrite = Some((rewrite_from, entries));
            }
            MismatchedRepair::Repaired { rewrite_from: None, .. } => {}
        }
    }
    transition.next.log.retain(|entry| entry.index > snapshot.last_included_index);
    let is_suffix_retained = has_matching_boundary || !transition.next.log.is_empty();
    if is_suffix_retained && let Some((rewrite_from, entries)) = rewrite {
        transition.effects.push(ReplicaEffect::PersistEntries {
            truncate_from: Some(rewrite_from),
            entries,
        });
        transition.effects.push(ReplicaEffect::FlushLog {
            through_index: support::last_log_index(&transition.next),
        });
    }
    transition.next.commit_index = snapshot.last_included_index;
    transition.next.last_applied = snapshot.last_included_index;
    transition.next.completed_requests.clone_from(&snapshot.completed_requests);
    transition.next.snapshot = Some(snapshot.clone());
    transition.effects.push(ReplicaEffect::PersistSnapshot {
        snapshot: snapshot.clone(),
        is_suffix_retained,
    });
    transition.effects.push(ReplicaEffect::RestoreApplicationSnapshot { snapshot });
    Ok(())
}

/// Fail closed when committed evidence contradicts the snapshot's committed prefix.
///
/// Committed copies reported by peers and locally committed entries must agree with the snapshot:
/// the boundary term, and within the duplicate window the request committed at each index.
/// Agreement on terms alone is not enough; the same index with the same term but another request
/// is divergence, so recovery neither rewrites the log nor installs the snapshot, on its first
/// attempt or after a restart. Uncommitted local entries are suspect, not evidence: a faulty one is
/// repaired instead.
// r[impl molten.consensus.storage_fault_recovery]
fn check_snapshot_evidence(state: &ReplicaState, snapshot: &ReplicaSnapshot) -> crate::error::Result<()> {
    let ReplicaRejoin::Recovering(round) = &state.rejoin else {
        return Err(crate::error::Failure::invalid_harness("snapshot evidence requires a recovery round"));
    };
    let peer_committed = round
        .peers
        .values()
        .filter(|peer| peer.is_complete)
        .flat_map(|peer| peer.entries.iter().filter(|entry| entry.index <= peer.commit_index));
    let local_committed = state.log.iter().filter(|entry| entry.index <= state.commit_index);
    if let Some(entry) =
        peer_committed.chain(local_committed).find(|entry| contradicts_snapshot(state, snapshot, entry))
    {
        return Err(crate::error::Failure::classified(
            crate::error::FailureClass::MalformedObservation,
            format!(
                "Raft recovery evidence diverges from snapshot {} at index {} (term {})",
                snapshot.snapshot_ref, entry.index, entry.term
            ),
        ));
    }
    Ok(())
}

fn contradicts_snapshot(state: &ReplicaState, snapshot: &ReplicaSnapshot, entry: &ReplicatedEntry) -> bool {
    if entry.index > snapshot.last_included_index {
        return false;
    }
    if entry.index == snapshot.last_included_index && entry.term != snapshot.last_included_term {
        return true;
    }
    let floor = request_window_floor(snapshot.last_included_index, state.profile.request_retention_entries);
    if entry.index <= floor {
        return false;
    }
    let recorded_index = snapshot.completed_requests.get(&entry.request_ref);
    let recorded_request = snapshot
        .completed_requests
        .iter()
        .find(|(_, index)| **index == entry.index)
        .map(|(request, _)| request);
    recorded_index.is_some_and(|index| *index != entry.index)
        || recorded_request.is_some_and(|request| *request != entry.request_ref)
}

fn repair_mismatched_log(
    state: &ReplicaState,
    snapshot: &ReplicaSnapshot,
    decisions: &[(u64, molten_core::consensus_recovery::EntryRepairDecision)],
) -> crate::error::Result<MismatchedRepair> {
    let boundary = snapshot.last_included_index;
    let mut repaired = Vec::with_capacity(state.log.len());
    let mut rewrite_from = None;
    let mut is_truncating = false;
    for entry in &state.log {
        let decision = decisions.iter().find(|(index, _)| *index == entry.index).map(|(_, decision)| decision);
        let resolved = match decision {
            Some(molten_core::consensus_recovery::EntryRepairDecision::RemoveUncommitted) if entry.index > boundary => {
                is_truncating = true;
                rewrite_from.get_or_insert(entry.index);
                continue;
            }
            // Nothing that may be committed follows an established-uncommitted entry in a
            // consistent report; keep the evidence rather than drop it.
            _ if is_truncating => return Ok(MismatchedRepair::Deferred),
            // The snapshot covers this index. A faulty local copy is replaced by the verified
            // committed copy (agreeing with the snapshot at its boundary) so the rewritten log stays
            // in term order until the snapshot lands; without one the local entry is kept.
            _ if entry.index <= boundary => verified_committed_copy(state, entry.index)
                .filter(|copy| entry.index != boundary || copy.term == snapshot.last_included_term)
                .unwrap_or_else(|| entry.clone()),
            Some(molten_core::consensus_recovery::EntryRepairDecision::ReplaceWithCommitted {
                from_peer,
                entry: committed,
            }) => match committed_replacement(state, from_peer, entry.index, committed)? {
                Some(replacement) => replacement,
                None => return Ok(MismatchedRepair::Deferred),
            },
            Some(molten_core::consensus_recovery::EntryRepairDecision::KeepLocal) => entry.clone(),
            _ => return Ok(MismatchedRepair::Deferred),
        };
        if resolved != *entry {
            rewrite_from.get_or_insert(entry.index);
        }
        repaired.push(resolved);
    }
    if rewrite_from.is_some_and(|index| index <= state.commit_index) {
        return Ok(MismatchedRepair::Deferred);
    }
    // Retained entries must follow the snapshot's term, and a rewritten log must replay as a valid
    // log if the process stops between the rewrite and the snapshot.
    let first_retained_term = repaired.iter().find(|entry| entry.index > boundary).map(|entry| entry.term);
    let base_term = state.snapshot.as_ref().map_or(INITIAL_TERM, |local| local.last_included_term);
    if first_retained_term.is_some_and(|term| term < snapshot.last_included_term)
        || (rewrite_from.is_some() && !is_term_ordered(base_term, &repaired))
    {
        return Ok(MismatchedRepair::Deferred);
    }
    let entries =
        rewrite_from.map_or_else(Vec::new, |index| repaired.into_iter().filter(|entry| entry.index >= index).collect());
    Ok(MismatchedRepair::Repaired { rewrite_from, entries })
}

/// The copy of `index` that every complete peer report holding it within its commit agrees on.
fn verified_committed_copy(state: &ReplicaState, index: u64) -> Option<ReplicatedEntry> {
    let ReplicaRejoin::Recovering(round) = &state.rejoin else {
        return None;
    };
    let mut copies = round
        .peers
        .values()
        .filter(|peer| peer.is_complete && index <= peer.commit_index)
        .filter_map(|peer| peer.entries.iter().find(|entry| entry.index == index));
    let first = copies.next()?;
    copies.all(|copy| copy == first).then(|| first.clone())
}

/// The committed copy a peer reported at `index`, once its report holds it within its commit.
fn committed_replacement(
    state: &ReplicaState,
    peer: &str,
    index: u64,
    expected: &molten_core::consensus_recovery::EntryIdentity,
) -> crate::error::Result<Option<ReplicatedEntry>> {
    let Ok(source) = observation(state, peer) else {
        return Ok(None);
    };
    let Some(entry) = source.entries.iter().find(|entry| entry.index == index && index <= source.commit_index) else {
        return Ok(None);
    };
    if identity(entry)? != *expected {
        return Err(crate::error::Failure::invalid_harness("recovery replacement failed committed identity check"));
    }
    Ok(Some(entry.clone()))
}

fn is_term_ordered(base_term: u64, entries: &[ReplicatedEntry]) -> bool {
    entries
        .iter()
        .try_fold(base_term, |previous, entry| (entry.term >= previous).then_some(entry.term))
        .is_some()
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
    let timer = support::arm_election_timer(&mut transition.next)?;
    transition.effects.push(timer);
    Ok(())
}

fn apply_available(
    transition: &mut MessagePlan,
    outcome: &molten_core::consensus_recovery::Reconciliation,
) -> crate::error::Result<()> {
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
                return install_snapshot(transition, from_peer, snapshot, &outcome.decisions);
            }
            molten_core::consensus_recovery::RecoveryEffect::RemoveSuffixFrom { .. }
            | molten_core::consensus_recovery::RecoveryEffect::QueryPeers { .. } => {}
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fabric_consistency::raft::tests as raft_tests;

    const RECOVERY_TERM: u64 = 2;
    const OLDER_TERM: u64 = 1;
    const SNAPSHOT_INDEX: u64 = 1;
    const SUFFIX_INDEX: u64 = 2;

    fn entry(index: u64, term: u64, label: &str) -> ReplicatedEntry {
        ReplicatedEntry {
            index,
            term,
            request_ref: raft_tests::test_ref(&format!("{label}-request")),
            command_ref: raft_tests::test_ref(&format!("{label}-command")),
            command_schema_ref: raft_tests::test_ref("recovery-schema"),
        }
    }

    fn peer_snapshot(state: &ReplicaState, last_included_term: u64) -> ReplicaSnapshot {
        let mut snapshot = ReplicaSnapshot {
            snapshot_ref: String::new(),
            group_binding_ref: state.profile.group_binding_ref.clone(),
            membership_ref: state.membership.membership_ref.clone(),
            config_epoch: state.membership.config_epoch,
            fencing_epoch: state.profile.fencing_epoch,
            last_included_index: SNAPSHOT_INDEX,
            last_included_term,
            application_state_ref: raft_tests::test_ref("recovery-peer-application-state"),
            completed_requests: std::collections::BTreeMap::from([(
                raft_tests::test_ref("recovery-committed-request"),
                SNAPSHOT_INDEX,
            )]),
        };
        snapshot.snapshot_ref = snapshot_ref(&snapshot).expect("peer snapshot identity");
        snapshot
    }

    fn lagging_observation() -> RecoveryObservation {
        RecoveryObservation {
            observation_ref: raft_tests::test_ref("recovery-lagging-observation"),
            current_term: RECOVERY_TERM,
            commit_index: INITIAL_COMMIT_INDEX,
            last_log_index: INITIAL_COMMIT_INDEX,
            entries: Vec::new(),
            snapshot: None,
            is_complete: true,
        }
    }

    /// Node B recovering at `RECOVERY_TERM` with `log`, once the given peers have reported.
    fn recovering_plan(
        log: Vec<ReplicatedEntry>,
        peers: std::collections::BTreeMap<String, RecoveryObservation>,
    ) -> MessagePlan {
        let mut state = raft_tests::started_state(&raft_tests::active_group(), raft_tests::NODE_B);
        state.current_term = RECOVERY_TERM;
        state.active_election_timer_ref = election_timer_ref(
            &state.profile.group_binding_ref,
            &state.node_id,
            state.profile.service_generation,
            state.current_term,
            state.election_timer_sequence,
        )
        .expect("recovery timer ref");
        state.log = log;
        state.rejoin = ReplicaRejoin::Recovering(RecoveryRound {
            round_ref: state.active_election_timer_ref.clone(),
            fence_term: Some(RECOVERY_TERM),
            peers,
        });
        validation::validate_replica_state(&state).expect("recovering state");
        MessagePlan {
            next: state,
            effects: Vec::new(),
            persist_hard_state: false,
        }
    }

    fn snapshot_observation(
        snapshot: &ReplicaSnapshot,
        commit_index: u64,
        entries: Vec<ReplicatedEntry>,
    ) -> RecoveryObservation {
        RecoveryObservation {
            observation_ref: raft_tests::test_ref("recovery-peer-observation"),
            current_term: RECOVERY_TERM,
            commit_index,
            last_log_index: commit_index,
            entries,
            snapshot: Some(Box::new(snapshot.clone())),
            is_complete: true,
        }
    }

    /// Node B recovers holding uncommitted term-1 entries at indices 1 and 2; node A reports a
    /// snapshot through index 1 taken at `snapshot_term`. With `is_lagging_peer_reported`, node C
    /// also reports an empty log, so both other voters lack the local suffix entry.
    fn recovering_with_suffix(snapshot_term: u64, is_lagging_peer_reported: bool) -> (MessagePlan, ReplicaSnapshot) {
        let template = raft_tests::started_state(&raft_tests::active_group(), raft_tests::NODE_B);
        let snapshot = peer_snapshot(&template, snapshot_term);
        let mut peers = std::collections::BTreeMap::from([(
            raft_tests::NODE_A.to_string(),
            snapshot_observation(&snapshot, SNAPSHOT_INDEX, Vec::new()),
        )]);
        if is_lagging_peer_reported {
            peers.insert(raft_tests::NODE_C.to_string(), lagging_observation());
        }
        let log = vec![
            // The request the peer snapshot records at index 1, so a matching term is consistent.
            entry(SNAPSHOT_INDEX, OLDER_TERM, "recovery-committed"),
            entry(SUFFIX_INDEX, OLDER_TERM, "local-suffix"),
        ];
        (recovering_plan(log, peers), snapshot)
    }

    const UNCONFIRMED_INDEX: u64 = 3;

    /// B's boundary entry is faulty (term 1), but its index-2 entry E (term 2) is correct; `extra`
    /// follows it locally. A's snapshot and C's committed log establish (1, term 2), and both
    /// confirm E as committed.
    fn confirmed_suffix_plan(extra: Option<ReplicatedEntry>) -> (MessagePlan, ReplicaSnapshot, ReplicatedEntry) {
        let template = raft_tests::started_state(&raft_tests::active_group(), raft_tests::NODE_B);
        let snapshot = peer_snapshot(&template, RECOVERY_TERM);
        let confirmed = entry(SUFFIX_INDEX, RECOVERY_TERM, "confirmed-suffix");
        let committed_boundary = entry(SNAPSHOT_INDEX, RECOVERY_TERM, "recovery-committed");
        let peers = std::collections::BTreeMap::from([
            (
                raft_tests::NODE_A.to_string(),
                snapshot_observation(&snapshot, SUFFIX_INDEX, vec![confirmed.clone()]),
            ),
            (raft_tests::NODE_C.to_string(), RecoveryObservation {
                observation_ref: raft_tests::test_ref("recovery-committed-log-observation"),
                current_term: RECOVERY_TERM,
                commit_index: SUFFIX_INDEX,
                last_log_index: SUFFIX_INDEX,
                entries: vec![committed_boundary, confirmed.clone()],
                snapshot: None,
                is_complete: true,
            }),
        ]);
        let mut log = vec![entry(SNAPSHOT_INDEX, OLDER_TERM, "local-boundary"), confirmed.clone()];
        log.extend(extra);
        let plan = recovering_plan(log, peers);
        assert_eq!(suffix_decision(&plan), molten_core::consensus_recovery::EntryRepairDecision::KeepLocal);
        (plan, snapshot, confirmed)
    }

    // r[verify molten.consensus.storage_fault_recovery]
    // r[verify molten.consensus.raft_replication_and_recovery_liveness]
    #[test]
    fn a_faulty_boundary_with_a_peer_confirmed_suffix_retains_it_and_drops_only_the_uncommitted_tail() {
        let unconfirmed = entry(UNCONFIRMED_INDEX, RECOVERY_TERM, "unconfirmed-tail");
        let (mut plan, snapshot, confirmed) = confirmed_suffix_plan(Some(unconfirmed));
        reconcile(&mut plan).expect("snapshot installation");
        validation::validate_replica_state(&plan.next).expect("installed state");
        assert_eq!(plan.next.snapshot.as_ref(), Some(&snapshot));
        assert_eq!(plan.next.log, vec![confirmed.clone()], "both peers lack index 3, so only it is discarded");
        // One durable mutation, before the snapshot is written, rewrites the faulty boundary from C's
        // verified committed copy, keeps E, and drops only the established-uncommitted index 3.
        let ReplicaEffect::PersistEntries { truncate_from, entries } = &plan.effects[0] else {
            panic!("the log repair precedes the snapshot: {:?}", plan.effects);
        };
        assert_eq!(*truncate_from, Some(SNAPSHOT_INDEX));
        assert_eq!(entries, &vec![entry(SNAPSHOT_INDEX, RECOVERY_TERM, "recovery-committed"), confirmed]);
        assert!(matches!(&plan.effects[1..], [
            ReplicaEffect::FlushLog { .. },
            ReplicaEffect::PersistSnapshot {
                is_suffix_retained: true,
                ..
            },
            ReplicaEffect::RestoreApplicationSnapshot { .. },
        ]));
    }

    // r[verify molten.consensus.storage_fault_recovery]
    // r[verify molten.consensus.raft_replication_and_recovery_liveness]
    #[test]
    fn a_faulty_boundary_with_a_peer_confirmed_suffix_installs_retains_it_and_rejoins() {
        let (mut plan, snapshot, confirmed) = confirmed_suffix_plan(None);

        reconcile(&mut plan).expect("snapshot installation");
        validation::validate_replica_state(&plan.next).expect("installed state");
        assert_eq!(plan.next.snapshot.as_ref(), Some(&snapshot));
        assert_eq!(plan.next.log, vec![confirmed.clone()], "the confirmed entry survives the faulty boundary");
        assert_eq!(suffix_retention(&plan), Some(true));

        reconcile(&mut plan).expect("admission");
        validation::validate_replica_state(&plan.next).expect("admitted state");
        assert!(matches!(plan.next.rejoin, ReplicaRejoin::VoteFenced { .. }), "B rejoins: {:?}", plan.next.rejoin);
        assert_eq!(plan.next.commit_index, SUFFIX_INDEX);
        assert_eq!(plan.next.last_applied, SUFFIX_INDEX);
        assert_eq!(plan.next.log, vec![confirmed]);
    }

    const COMMITTED_TAIL_INDEX: u64 = 3;

    /// The review scenario: B's indices 1–2 are faulty (term 1) but its index-3 entry E (term 2)
    /// is correct. C committed Y, X and E with B; A holds only a snapshot through index 1. With
    /// `is_c_reported` unset, C is unavailable and only A reports.
    struct CommittedTail {
        local: Vec<ReplicatedEntry>,
        committed: Vec<ReplicatedEntry>,
        snapshot: ReplicaSnapshot,
    }

    impl CommittedTail {
        fn new() -> Self {
            let template = raft_tests::started_state(&raft_tests::active_group(), raft_tests::NODE_B);
            let tail = entry(COMMITTED_TAIL_INDEX, RECOVERY_TERM, "committed-tail");
            Self {
                local: vec![
                    entry(SNAPSHOT_INDEX, OLDER_TERM, "faulty-first"),
                    entry(SUFFIX_INDEX, OLDER_TERM, "faulty-second"),
                    tail.clone(),
                ],
                committed: vec![
                    entry(SNAPSHOT_INDEX, RECOVERY_TERM, "recovery-committed"),
                    entry(SUFFIX_INDEX, RECOVERY_TERM, "committed-second"),
                    tail,
                ],
                snapshot: peer_snapshot(&template, RECOVERY_TERM),
            }
        }

        fn tail(&self) -> &ReplicatedEntry {
            &self.committed[2]
        }

        fn plan(&self, log: Vec<ReplicatedEntry>, is_c_reported: bool) -> MessagePlan {
            let mut peers = std::collections::BTreeMap::from([(
                raft_tests::NODE_A.to_string(),
                snapshot_observation(&self.snapshot, SNAPSHOT_INDEX, Vec::new()),
            )]);
            if is_c_reported {
                peers.insert(raft_tests::NODE_C.to_string(), RecoveryObservation {
                    observation_ref: raft_tests::test_ref("committed-tail-observation"),
                    current_term: RECOVERY_TERM,
                    commit_index: COMMITTED_TAIL_INDEX,
                    last_log_index: COMMITTED_TAIL_INDEX,
                    entries: self.committed.clone(),
                    snapshot: None,
                    is_complete: true,
                });
            }
            recovering_plan(log, peers)
        }
    }

    // r[verify molten.consensus.storage_fault_recovery]
    // r[verify molten.consensus.raft_replication_and_recovery_liveness]
    #[test]
    fn a_committed_tail_after_replaced_entries_is_rewritten_with_the_repair_and_never_dropped() {
        let scenario = CommittedTail::new();
        let mut plan = scenario.plan(scenario.local.clone(), true);
        let outcome = molten_core::consensus_recovery::reconcile_participant(&input(&plan.next).expect("input"));
        assert!(matches!(
            outcome.decisions.iter().find(|(index, _)| *index == SUFFIX_INDEX),
            Some((_, molten_core::consensus_recovery::EntryRepairDecision::ReplaceWithCommitted { .. }))
        ));
        assert!(matches!(
            outcome.decisions.iter().find(|(index, _)| *index == COMMITTED_TAIL_INDEX),
            Some((_, molten_core::consensus_recovery::EntryRepairDecision::KeepLocal))
        ));

        reconcile(&mut plan).expect("repair and install");
        validation::validate_replica_state(&plan.next).expect("installed state");
        assert_eq!(plan.next.snapshot.as_ref(), Some(&scenario.snapshot));
        assert_eq!(plan.next.log, scenario.committed[1..].to_vec());
        // Indices 1–2 take C's verified committed copies and E is rewritten in that same durable
        // mutation, so no effect ever leaves E only on C.
        assert!(matches!(plan.effects.as_slice(), [
            ReplicaEffect::PersistEntries { truncate_from: Some(SNAPSHOT_INDEX), entries },
            ReplicaEffect::FlushLog { through_index: COMMITTED_TAIL_INDEX },
            ReplicaEffect::PersistSnapshot { is_suffix_retained: true, .. },
            ReplicaEffect::RestoreApplicationSnapshot { .. },
        ] if *entries == scenario.committed));
    }

    // r[verify molten.consensus.storage_fault_recovery]
    // r[verify molten.consensus.raft_replication_and_recovery_liveness]
    #[test]
    fn a_lost_peer_before_the_repair_defers_and_keeps_the_committed_tail() {
        let scenario = CommittedTail::new();
        let mut plan = scenario.plan(scenario.local.clone(), false);
        let before = plan.next.clone();
        reconcile(&mut plan).expect("available repair");
        assert_eq!(plan.next, before, "without C's committed copies nothing is rewritten or dropped");
        assert!(plan.effects.is_empty());
    }

    // r[verify molten.consensus.storage_fault_recovery]
    // r[verify molten.consensus.raft_replication_and_recovery_liveness]
    #[test]
    fn a_crash_between_the_repair_and_the_snapshot_then_peer_loss_keeps_the_committed_tail() {
        let scenario = CommittedTail::new();
        let mut plan = scenario.plan(scenario.local.clone(), true);
        reconcile(&mut plan).expect("repair and install");
        let root = crate::test_support::process_workspace("raft-recovery-committed-tail-crash").expect("workspace");
        let log_ref = raft_tests::test_ref("committed-tail-crash-log");
        let store_ref = raft_tests::test_ref("committed-tail-crash-store");
        let open = || {
            let adapter = crate::fabric_durability::RedbDurableStateAdapter::open(
                &root,
                crate::fabric_durability::tests::profile(crate::fabric_durability::DurableAdapterKind::LiveRedb),
                crate::fabric_durability::tests::descriptor(),
            )
            .expect("Redb adapter");
            RedbReplicaPort::new(adapter, log_ref.clone(), store_ref.clone()).expect("durability port")
        };
        let mut port = open();
        port.persist_hard_state(RECOVERY_TERM, None, true).expect("recovery marker");
        port.persist_entries(None, &scenario.local).expect("faulty local log");
        port.flush_log(COMMITTED_TAIL_INDEX).expect("durable local log");
        // The process stops after the log repair and before the snapshot is written.
        for effect in plan.effects.iter().take_while(|effect| !matches!(effect, ReplicaEffect::PersistSnapshot { .. }))
        {
            match effect {
                ReplicaEffect::PersistEntries { truncate_from, entries } => {
                    port.persist_entries(*truncate_from, entries).expect("repair mutation");
                }
                ReplicaEffect::FlushLog { through_index } => {
                    port.flush_log(*through_index).expect("repair flush");
                }
                other => panic!("unexpected pre-snapshot effect {other:?}"),
            }
        }
        drop(port);

        let group = raft_tests::active_group();
        let recovered = open()
            .plan_recovery(ReplicaStartPlan {
                state: raft_tests::started_state(&group, raft_tests::NODE_B),
                service_id: group.service_id.clone(),
                application_manifest_ref: group.application_manifest_ref.clone(),
                initial_effects: Vec::new(),
                port_binding_refs: Vec::new(),
                production_admitted: false,
            })
            .expect("restart after the crash");
        assert_eq!(recovered.start_plan.state.log, scenario.committed, "the repaired log replays with E");
        assert!(matches!(recovered.start_plan.state.rejoin, ReplicaRejoin::Recovering(_)));

        // C is lost before the next round. The repaired boundary now matches A's snapshot, so A's
        // report alone installs it and the committed tail survives.
        let mut after_crash = scenario.plan(recovered.start_plan.state.log, false);
        reconcile(&mut after_crash).expect("install from the repaired log");
        validation::validate_replica_state(&after_crash.next).expect("installed state");
        assert_eq!(after_crash.next.snapshot.as_ref(), Some(&scenario.snapshot));
        assert_eq!(after_crash.next.log, scenario.committed[1..].to_vec());
        assert_eq!(after_crash.next.log.last(), Some(scenario.tail()));
        assert_eq!(suffix_retention(&after_crash), Some(true));
    }

    const FAULTY_HIGHER_TERM: u64 = 3;

    // r[verify molten.consensus.storage_fault_recovery]
    // r[verify molten.consensus.raft_replication_and_recovery_liveness]
    #[test]
    fn a_faulty_higher_term_boundary_is_repaired_from_the_verified_committed_copy_and_rejoins() {
        // B's faulty log is (1, term 3), (2, term 3); A has a snapshot through (1, term 2) and C has
        // committed (1, term 2), (2, term 2). Rewriting only index 2 would leave terms 3, 2.
        let template = raft_tests::started_state(&raft_tests::active_group(), raft_tests::NODE_B);
        let snapshot = peer_snapshot(&template, RECOVERY_TERM);
        let committed = vec![
            entry(SNAPSHOT_INDEX, RECOVERY_TERM, "recovery-committed"),
            entry(SUFFIX_INDEX, RECOVERY_TERM, "reversed-committed-second"),
        ];
        let peers = std::collections::BTreeMap::from([
            (raft_tests::NODE_A.to_string(), snapshot_observation(&snapshot, SNAPSHOT_INDEX, Vec::new())),
            (raft_tests::NODE_C.to_string(), RecoveryObservation {
                observation_ref: raft_tests::test_ref("reversed-term-observation"),
                current_term: RECOVERY_TERM,
                commit_index: SUFFIX_INDEX,
                last_log_index: SUFFIX_INDEX,
                entries: committed.clone(),
                snapshot: None,
                is_complete: true,
            }),
        ]);
        let faulty = vec![
            entry(SNAPSHOT_INDEX, FAULTY_HIGHER_TERM, "reversed-faulty-first"),
            entry(SUFFIX_INDEX, FAULTY_HIGHER_TERM, "reversed-faulty-second"),
        ];
        let mut plan = recovering_plan(faulty, peers);
        assert!(matches!(
            suffix_decision(&plan),
            molten_core::consensus_recovery::EntryRepairDecision::ReplaceWithCommitted { .. }
        ));

        reconcile(&mut plan).expect("repair and install");
        validation::validate_replica_state(&plan.next).expect("installed state");
        assert!(
            matches!(plan.effects.as_slice(), [
            ReplicaEffect::PersistEntries { truncate_from: Some(SNAPSHOT_INDEX), entries },
            ReplicaEffect::FlushLog { through_index: SUFFIX_INDEX },
            ReplicaEffect::PersistSnapshot { is_suffix_retained: true, .. },
            ReplicaEffect::RestoreApplicationSnapshot { .. },
        ] if *entries == committed),
            "the boundary and index 2 are rewritten in term order: {:?}",
            plan.effects
        );
        assert_eq!(plan.next.snapshot.as_ref(), Some(&snapshot));
        assert_eq!(plan.next.log, committed[1..].to_vec());

        reconcile(&mut plan).expect("admission");
        validation::validate_replica_state(&plan.next).expect("admitted state");
        assert!(matches!(plan.next.rejoin, ReplicaRejoin::VoteFenced { .. }), "B rejoins: {:?}", plan.next.rejoin);
        assert_eq!(plan.next.commit_index, SUFFIX_INDEX);
        assert_eq!(plan.next.log, committed[1..].to_vec());
    }

    /// A's snapshot records request R1 committed at index 1 (term 2); C reports a different request
    /// R2 committed at index 1 with the same term.
    fn contradictory_boundary_plan(log: Vec<ReplicatedEntry>, is_c_reported: bool) -> (MessagePlan, ReplicatedEntry) {
        let template = raft_tests::started_state(&raft_tests::active_group(), raft_tests::NODE_B);
        let snapshot = peer_snapshot(&template, RECOVERY_TERM);
        let contradictory = entry(SNAPSHOT_INDEX, RECOVERY_TERM, "contradictory-committed");
        assert_eq!(snapshot.completed_requests.get(&raft_tests::test_ref("recovery-committed-request")), Some(&1));
        assert_ne!(contradictory.request_ref, raft_tests::test_ref("recovery-committed-request"));
        let mut peers = std::collections::BTreeMap::from([(
            raft_tests::NODE_A.to_string(),
            snapshot_observation(&snapshot, SNAPSHOT_INDEX, Vec::new()),
        )]);
        if is_c_reported {
            peers.insert(raft_tests::NODE_C.to_string(), RecoveryObservation {
                observation_ref: raft_tests::test_ref("contradictory-observation"),
                current_term: RECOVERY_TERM,
                commit_index: SUFFIX_INDEX,
                last_log_index: SUFFIX_INDEX,
                entries: vec![
                    contradictory.clone(),
                    entry(SUFFIX_INDEX, RECOVERY_TERM, "contradictory-second"),
                ],
                snapshot: None,
                is_complete: true,
            });
        }
        (recovering_plan(log, peers), contradictory)
    }

    fn assert_divergence_denied(mut plan: MessagePlan) {
        let before = plan.next.clone();
        let error = reconcile(&mut plan).expect_err("contradictory committed evidence must fail closed");
        assert_eq!(error.class(), Some(crate::error::FailureClass::MalformedObservation), "{error}");
        assert!(error.to_string().contains("diverges from snapshot"), "{error}");
        assert_eq!(plan.next, before, "nothing is rewritten or installed");
        assert!(plan.effects.is_empty());
    }

    // r[verify molten.consensus.storage_fault_recovery]
    #[test]
    fn a_same_term_committed_copy_with_another_request_fails_closed_before_any_rewrite() {
        let faulty = vec![
            entry(SNAPSHOT_INDEX, OLDER_TERM, "contradiction-faulty-first"),
            entry(SUFFIX_INDEX, OLDER_TERM, "contradiction-faulty-second"),
        ];
        let (plan, _contradictory) = contradictory_boundary_plan(faulty, true);
        assert!(matches!(
            suffix_decision(&plan),
            molten_core::consensus_recovery::EntryRepairDecision::ReplaceWithCommitted { .. }
        ));
        assert_divergence_denied(plan);
    }

    // r[verify molten.consensus.storage_fault_recovery]
    #[test]
    fn a_restart_holding_the_contradictory_boundary_fails_closed_while_committed_evidence_contradicts() {
        // After a restart B's uncommitted log holds R2 at index 1, whose term matches A's snapshot.
        let (_, contradictory) = contradictory_boundary_plan(Vec::new(), false);
        let restarted = vec![
            contradictory,
            entry(SUFFIX_INDEX, RECOVERY_TERM, "contradictory-second"),
        ];
        // While C reports R2 as committed, committed evidence contradicts A's snapshot: fail closed.
        let (plan, _) = contradictory_boundary_plan(restarted.clone(), true);
        assert_divergence_denied(plan);
        // With C lost, B's suspect uncommitted copy is no evidence; nothing verifies index 2, so
        // recovery waits and keeps the local log instead of failing stop or installing.
        let (mut plan, _) = contradictory_boundary_plan(restarted, false);
        let before = plan.next.clone();
        reconcile(&mut plan).expect("a suspect uncommitted copy is not a divergence");
        assert_eq!(plan.next, before);
        assert!(plan.effects.is_empty());
    }

    // r[verify molten.consensus.storage_fault_recovery]
    // r[verify molten.consensus.raft_replication_and_recovery_liveness]
    #[test]
    fn a_faulty_uncommitted_same_term_boundary_is_repaired_when_committed_evidence_agrees() {
        // A's snapshot and C's committed log agree on R1 at (1, term 2); B's uncommitted boundary
        // entry has term 2 but request R2 (a local storage fault), and its index-2 entry is correct.
        let template = raft_tests::started_state(&raft_tests::active_group(), raft_tests::NODE_B);
        let snapshot = peer_snapshot(&template, RECOVERY_TERM);
        let committed = vec![
            entry(SNAPSHOT_INDEX, RECOVERY_TERM, "recovery-committed"),
            entry(SUFFIX_INDEX, RECOVERY_TERM, "agreed-committed-second"),
        ];
        let peers = std::collections::BTreeMap::from([
            (raft_tests::NODE_A.to_string(), snapshot_observation(&snapshot, SNAPSHOT_INDEX, Vec::new())),
            (raft_tests::NODE_C.to_string(), RecoveryObservation {
                observation_ref: raft_tests::test_ref("agreed-committed-observation"),
                current_term: RECOVERY_TERM,
                commit_index: SUFFIX_INDEX,
                last_log_index: SUFFIX_INDEX,
                entries: committed.clone(),
                snapshot: None,
                is_complete: true,
            }),
        ]);
        let faulty_boundary = entry(SNAPSHOT_INDEX, RECOVERY_TERM, "same-term-faulty-boundary");
        let mut plan = recovering_plan(vec![faulty_boundary, committed[1].clone()], peers);

        reconcile(&mut plan).expect("repair and install");
        validation::validate_replica_state(&plan.next).expect("installed state");
        assert!(
            matches!(plan.effects.as_slice(), [
            ReplicaEffect::PersistEntries { truncate_from: Some(SNAPSHOT_INDEX), entries },
            ReplicaEffect::FlushLog { through_index: SUFFIX_INDEX },
            ReplicaEffect::PersistSnapshot { is_suffix_retained: true, .. },
            ReplicaEffect::RestoreApplicationSnapshot { .. },
        ] if *entries == committed),
            "the faulty boundary is rewritten from C's verified copy: {:?}",
            plan.effects
        );
        assert_eq!(plan.next.snapshot.as_ref(), Some(&snapshot));
        assert_eq!(plan.next.log, committed[1..].to_vec());

        reconcile(&mut plan).expect("admission");
        validation::validate_replica_state(&plan.next).expect("admitted state");
        assert!(matches!(plan.next.rejoin, ReplicaRejoin::VoteFenced { .. }), "B rejoins: {:?}", plan.next.rejoin);
        assert_eq!(plan.next.commit_index, SUFFIX_INDEX);
        assert_eq!(plan.next.log, committed[1..].to_vec());
    }

    fn suffix_decision(plan: &MessagePlan) -> molten_core::consensus_recovery::EntryRepairDecision {
        let outcome = molten_core::consensus_recovery::reconcile_participant(&input(&plan.next).expect("input"));
        assert!(
            outcome.effects.iter().any(|effect| matches!(
                effect,
                molten_core::consensus_recovery::RecoveryEffect::InstallSnapshot { .. }
            )),
            "recovery plans the peer snapshot installation: {outcome:?}"
        );
        outcome
            .decisions
            .into_iter()
            .find_map(|(index, decision)| (index == SUFFIX_INDEX).then_some(decision))
            .expect("suffix decision")
    }

    fn suffix_retention(plan: &MessagePlan) -> Option<bool> {
        plan.effects.iter().find_map(|effect| match effect {
            ReplicaEffect::PersistSnapshot { is_suffix_retained, .. } => Some(*is_suffix_retained),
            _ => None,
        })
    }

    // r[verify molten.consensus.storage_fault_recovery]
    // r[verify molten.consensus.raft_replication_and_recovery_liveness]
    #[test]
    fn a_single_snapshot_report_defers_installation_over_an_unresolved_conflicting_suffix() {
        let (mut plan, _snapshot) = recovering_with_suffix(RECOVERY_TERM, false);
        let before = plan.next.clone();
        assert_eq!(suffix_decision(&plan), molten_core::consensus_recovery::EntryRepairDecision::Wait);
        reconcile(&mut plan).expect("available repair");
        assert_eq!(plan.next, before, "unresolved local evidence is kept and the round keeps waiting");
        assert!(plan.effects.is_empty());
    }

    // r[verify molten.consensus.storage_fault_recovery]
    // r[verify molten.consensus.raft_replication_and_recovery_liveness]
    #[test]
    fn a_snapshot_with_a_newer_boundary_term_discards_a_suffix_established_uncommitted() {
        let (mut plan, snapshot) = recovering_with_suffix(RECOVERY_TERM, true);
        assert_eq!(suffix_decision(&plan), molten_core::consensus_recovery::EntryRepairDecision::RemoveUncommitted);
        reconcile(&mut plan).expect("barrier repair");
        validation::validate_replica_state(&plan.next).expect("installed state keeps log-term monotonicity");
        assert!(plan.next.log.is_empty(), "the term-1 suffix cannot follow a term-2 snapshot boundary");
        assert_eq!(plan.next.snapshot.as_ref(), Some(&snapshot));
        assert_eq!(plan.next.commit_index, SNAPSHOT_INDEX);
        assert_eq!(suffix_retention(&plan), Some(false));
    }

    // r[verify molten.consensus.raft_replication_and_recovery_liveness]
    #[test]
    fn a_snapshot_with_a_matching_boundary_term_retains_the_suffix() {
        let (mut plan, snapshot) = recovering_with_suffix(OLDER_TERM, false);
        reconcile(&mut plan).expect("available repair");
        validation::validate_replica_state(&plan.next).expect("installed state");
        assert_eq!(plan.next.snapshot.as_ref(), Some(&snapshot));
        assert_eq!(plan.next.log, vec![entry(SUFFIX_INDEX, OLDER_TERM, "local-suffix")]);
        assert_eq!(suffix_retention(&plan), Some(true));
    }
}

//! Installing a peer snapshot during recovery, repairing the local log at its boundary first.

use super::*;

/// The local log repaired for a peer snapshot whose boundary term it does not match.
enum MismatchedLog {
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
pub(super) fn install_snapshot(
    transition: &mut MessagePlan<'_>,
    peer: &str,
    expected: &molten_core::consensus_recovery::SnapshotFact,
    decisions: &[(u64, molten_core::consensus_recovery::EntryRepairDecision)],
) -> crate::denial::Result<()> {
    let snapshot = observation(&transition.next, peer)?
        .snapshot
        .as_deref()
        .ok_or(crate::denial::ReplicaError::Denied("recovery source lacks planned snapshot"))?
        .clone();
    if snapshot_fact(&snapshot) != *expected {
        return Err(crate::denial::ReplicaError::Denied("recovery snapshot identity mismatch"));
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
        match rewrite_mismatched_log(&transition.next, &snapshot, decisions, transition.identity)? {
            MismatchedLog::Deferred => return Ok(()),
            MismatchedLog::Repaired {
                rewrite_from: Some(rewrite_from),
                entries,
            } => {
                transition.next.log.retain(|entry| entry.index < rewrite_from);
                transition.next.log.extend(entries.iter().cloned());
                rewrite = Some((rewrite_from, entries));
            }
            MismatchedLog::Repaired { rewrite_from: None, .. } => {}
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
fn check_snapshot_evidence(state: &ReplicaState, snapshot: &ReplicaSnapshot) -> crate::denial::Result<()> {
    let ReplicaRejoin::Recovering(round) = &state.rejoin else {
        return Err(crate::denial::ReplicaError::Denied("snapshot evidence requires a recovery round"));
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
        return Err(crate::denial::ReplicaError::RecoveryEvidence {
            snapshot_ref: snapshot.snapshot_ref.clone(),
            index: entry.index,
            term: entry.term,
        });
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

fn rewrite_mismatched_log(
    state: &ReplicaState,
    snapshot: &ReplicaSnapshot,
    decisions: &[(u64, molten_core::consensus_recovery::EntryRepairDecision)],
    identity: &dyn crate::identity::ReplicaIdentity,
) -> crate::denial::Result<MismatchedLog> {
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
            _ if is_truncating => return Ok(MismatchedLog::Deferred),
            // The snapshot covers this index. A faulty local copy is replaced by the verified
            // committed copy (agreeing with the snapshot at its boundary) so the rewritten log stays
            // in term order until the snapshot lands; without one the local entry is kept.
            _ if entry.index <= boundary => verified_committed_copy(state, entry.index)
                .filter(|copy| entry.index != boundary || copy.term == snapshot.last_included_term)
                .unwrap_or_else(|| entry.clone()),
            Some(molten_core::consensus_recovery::EntryRepairDecision::ReplaceWithCommitted {
                from_peer,
                entry: committed,
            }) => match committed_replacement(state, from_peer, entry.index, committed, identity)? {
                Some(replacement) => replacement,
                None => return Ok(MismatchedLog::Deferred),
            },
            Some(molten_core::consensus_recovery::EntryRepairDecision::KeepLocal) => entry.clone(),
            _ => return Ok(MismatchedLog::Deferred),
        };
        if resolved != *entry {
            rewrite_from.get_or_insert(entry.index);
        }
        repaired.push(resolved);
    }
    if rewrite_from.is_some_and(|index| index <= state.commit_index) {
        return Ok(MismatchedLog::Deferred);
    }
    // Retained entries must follow the snapshot's term, and a rewritten log must replay as a valid
    // log if the process stops between the rewrite and the snapshot.
    let first_retained_term = repaired.iter().find(|entry| entry.index > boundary).map(|entry| entry.term);
    let base_term = state.snapshot.as_ref().map_or(INITIAL_TERM, |local| local.last_included_term);
    if first_retained_term.is_some_and(|term| term < snapshot.last_included_term)
        || (rewrite_from.is_some() && !is_term_ordered(base_term, &repaired))
    {
        return Ok(MismatchedLog::Deferred);
    }
    let entries =
        rewrite_from.map_or_else(Vec::new, |index| repaired.into_iter().filter(|entry| entry.index >= index).collect());
    Ok(MismatchedLog::Repaired { rewrite_from, entries })
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
    identity: &dyn crate::identity::ReplicaIdentity,
) -> crate::denial::Result<Option<ReplicatedEntry>> {
    let Ok(source) = observation(state, peer) else {
        return Ok(None);
    };
    let Some(entry) = source.entries.iter().find(|entry| entry.index == index && index <= source.commit_index) else {
        return Ok(None);
    };
    if entry_fact(entry, identity)? != *expected {
        return Err(crate::denial::ReplicaError::Denied("recovery replacement failed committed identity check"));
    }
    Ok(Some(entry.clone()))
}

fn is_term_ordered(base_term: u64, entries: &[ReplicatedEntry]) -> bool {
    entries
        .iter()
        .try_fold(base_term, |previous, entry| (entry.term >= previous).then_some(entry.term))
        .is_some()
}

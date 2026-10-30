mod observations;

use super::*;

/// The most indices one reconciliation pass decides.
pub const MAX_RECONCILED_INDICES: usize = 4_096;

/// What one admitted peer reported in answer to a recovery query.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeerRecovery {
    pub peer_id: String,
    pub current_term: u64,
    pub commit_index: u64,
    /// The complete committed-or-held log, keyed by index. Every index after
    /// the snapshot through the held tail is present; compacted entries are absent.
    pub entries: std::collections::BTreeMap<u64, EntryIdentity>,
    pub snapshot: Option<SnapshotFact>,
    /// True only after every page of one stable peer observation was received.
    pub is_complete: bool,
}

/// The facts of one participant reconciliation pass.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReconcileInput {
    pub participant_id: String,
    /// The admitted voters of the current membership, the participant included.
    pub voters: std::collections::BTreeSet<String>,
    pub current_term: u64,
    pub local_commit_index: u64,
    /// The indices the participant verifies and what its storage holds there.
    /// Absent committed indices above the snapshot are checked as missing too.
    pub local_entries: std::collections::BTreeMap<u64, LocalEntry>,
    pub local_hard_state: LocalHardState,
    pub local_snapshot: Option<SnapshotFact>,
    pub peers: Vec<PeerRecovery>,
    pub is_persistent_state_suspect: bool,
    pub is_storage_serving: bool,
    pub is_membership_current: bool,
    pub is_log_or_snapshot_validated: bool,
}

/// The outcome of one reconciliation pass.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reconciliation {
    pub decisions: Vec<(u64, EntryRepairDecision)>,
    pub hard_state: HardStateDecision,
    pub snapshot: SnapshotRepairDecision,
    pub effects: Vec<RecoveryEffect>,
    /// Every decided index and the snapshot agree with admitted peers, reporters
    /// intersect every commit quorum without relying on the participant, and no
    /// wait or conflict is outstanding.
    pub is_reconciled: bool,
    pub admission: RepairAdmission,
}

fn quorum(voters: &std::collections::BTreeSet<String>) -> usize {
    voters.len() / 2 + 1
}

/// Decide one participant's recovery from its own facts and the peer reports.
///
/// The pass never copies a peer's term or vote, never plans the removal of a
/// committed entry, and reads a peer that compacted an index into its snapshot
/// as unable to attest that index rather than as lacking it. The reporters must
/// intersect every possible commit quorum without relying on the participant's
/// suspect state.
/// Reports must be complete, bounded, unique observations of admitted voters.
/// Snapshot installation is the only effect of its pass; the shell must observe
/// the resulting log before repairing entries. Conflicts suppress every effect.
// r[impl molten.consensus.storage_fault_recovery]
// r[impl molten.consensus.repair_admission]
pub fn reconcile_participant(input: &ReconcileInput) -> Reconciliation {
    let Some(indices) = observations::validated_indices(input) else {
        return observations::unresolved(input);
    };
    let peers = &input.peers;
    let mut entry_input = EntryRepairInput {
        index: input.local_commit_index,
        local: LocalEntry::Missing,
        local_commit_index: input.local_commit_index,
        voters: input.voters.clone(),
        participant_id: input.participant_id.clone(),
        peer_reports: Vec::with_capacity(peers.len()),
    };
    let decisions =
        indices
            .into_iter()
            .map(|index| {
                entry_input.peer_reports.clear();
                entry_input.peer_reports.extend(
                    peers.iter().filter(|peer| !observations::is_compacted(peer, index)).map(|peer| PeerEntryReport {
                        peer_id: peer.peer_id.clone(),
                        entry: peer.entries.get(&index).cloned(),
                        commit_index: peer.commit_index,
                    }),
                );
                let local = input.local_entries.get(&index);
                entry_input.index = index;
                entry_input.local = local.cloned().unwrap_or(LocalEntry::Missing);
                let decision = decide_entry_repair(&entry_input);
                let decision = match (local, decision) {
                    (Some(LocalEntry::Present(_)), EntryRepairDecision::ReplaceWithCommitted { .. })
                        if index <= input.local_commit_index =>
                    {
                        EntryRepairDecision::ConflictingHistory
                    }
                    (None | Some(LocalEntry::Missing | LocalEntry::Faulty), decision)
                        if !matches!(
                            decision,
                            EntryRepairDecision::FetchCommitted { .. } | EntryRepairDecision::ConflictingHistory
                        ) =>
                    {
                        observations::decide_unreadable(input, index, local)
                    }
                    (_, EntryRepairDecision::RemoveUncommitted) if index <= input.local_commit_index => {
                        EntryRepairDecision::Wait
                    }
                    (Some(LocalEntry::Present(local)), EntryRepairDecision::Wait)
                        if peers
                            .iter()
                            .filter(|peer| {
                                !observations::is_compacted(peer, index) && peer.entries.get(&index) == Some(local)
                            })
                            .count()
                            + 1
                            >= quorum(&input.voters) =>
                    {
                        EntryRepairDecision::KeepLocal
                    }
                    (_, decision) => decision,
                };
                (index, decision)
            })
            .collect::<Vec<_>>();
    let peer_snapshots = peers
        .iter()
        .filter_map(|peer| {
            peer.snapshot.clone().map(|snapshot| PeerSnapshotReport {
                peer_id: peer.peer_id.clone(),
                snapshot,
                commit_index: peer.commit_index,
            })
        })
        .collect::<Vec<_>>();
    let snapshot = observations::reconcile_snapshot(input, &peer_snapshots);
    let hard_state = if matches!(&input.local_hard_state, LocalHardState::Lost) && peers.len() + 1 != input.voters.len()
    {
        HardStateDecision::Wait
    } else {
        decide_hard_state(&input.local_hard_state, peers.iter().map(|peer| peer.current_term).max())
    };
    // Snapshot installation changes the log's base. Re-observe it before planning
    // entry mutations; a conflict anywhere must preserve all local evidence.
    let has_conflict = decisions.iter().any(|(_, decision)| *decision == EntryRepairDecision::ConflictingHistory)
        || snapshot == SnapshotRepairDecision::ConflictingContent;
    let effects = if has_conflict {
        Vec::new()
    } else {
        match &snapshot {
            SnapshotRepairDecision::InstallFrom { peer_id, snapshot } => vec![RecoveryEffect::InstallSnapshot {
                from_peer: peer_id.clone(),
                snapshot: snapshot.clone(),
            }],
            SnapshotRepairDecision::KeepLocal | SnapshotRepairDecision::Wait => {
                plan_recovery_effects(&decisions, &peers.iter().map(|peer| peer.peer_id.clone()).collect::<Vec<_>>())
            }
            SnapshotRepairDecision::ConflictingContent => Vec::new(),
        }
    };
    let has_quorum_of_reports = peers.len() + quorum(&input.voters) > input.voters.len();
    let is_reconciled = has_quorum_of_reports
        && snapshot == SnapshotRepairDecision::KeepLocal
        && hard_state != HardStateDecision::Wait
        && decisions.iter().all(|(_, decision)| *decision == EntryRepairDecision::KeepLocal);
    let admission = admit_repaired_participant(&RepairAdmissionInput {
        is_persistent_state_suspect: input.is_persistent_state_suspect,
        is_storage_serving: input.is_storage_serving,
        is_protocol_reconciled: is_reconciled,
        is_log_or_snapshot_validated: input.is_log_or_snapshot_validated,
        is_membership_current: input.is_membership_current,
        hard_state: hard_state.clone(),
        current_term: input.current_term,
    });
    Reconciliation {
        decisions,
        hard_state,
        snapshot,
        effects,
        is_reconciled,
        admission,
    }
}

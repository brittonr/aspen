use std::collections::BTreeSet;

use super::*;

/// One Raft entry identity.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct EntryIdentity {
    pub term: u64,
    pub entry_ref: String,
}

/// What the participant's own storage holds at one index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LocalEntry {
    /// The entry is present and passed its checks.
    Present(EntryIdentity),
    Missing,
    /// The entry failed a decode, checksum, or invariant check.
    Faulty,
}

/// What one admitted peer reported for one index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeerEntryReport {
    pub peer_id: String,
    /// The entry the peer holds at the index, if any.
    pub entry: Option<EntryIdentity>,
    /// The peer's durable commit index.
    pub commit_index: u64,
}

impl PeerEntryReport {
    fn committed_entry(&self, index: u64) -> Option<&EntryIdentity> {
        self.entry.as_ref().filter(|_| self.commit_index >= index)
    }
}

/// The facts of one index's repair decision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntryRepairInput {
    pub index: u64,
    pub local: LocalEntry,
    /// The participant's own commit index, admitted from storage that passed
    /// its checks.
    pub local_commit_index: u64,
    /// The admitted voters of the current membership, the participant included.
    pub voters: BTreeSet<String>,
    pub participant_id: String,
    pub peer_reports: Vec<PeerEntryReport>,
}

/// The decision for one index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EntryRepairDecision {
    /// Admitted local durable state is sufficient; no remote repair.
    KeepLocal,
    /// Fetch the committed entry the local store lacks or cannot read.
    FetchCommitted { from_peer: String, entry: EntryIdentity },
    /// Replace a local entry that differs from the committed one.
    ReplaceWithCommitted { from_peer: String, entry: EntryIdentity },
    /// A quorum established that the local entry was never committed.
    RemoveUncommitted,
    /// Commitment is unknown: keep the local evidence and wait.
    Wait,
    /// Peers reported different committed entries at one index.
    ConflictingHistory,
}

fn quorum(voters: &BTreeSet<String>) -> usize {
    voters.len() / 2 + 1
}

/// Decide one index from local and admitted peer facts.
// r[impl molten.consensus.storage_fault_recovery]
pub fn decide_entry_repair(input: &EntryRepairInput) -> EntryRepairDecision {
    let reports = input
        .peer_reports
        .iter()
        .filter(|report| report.peer_id != input.participant_id && input.voters.contains(&report.peer_id))
        .take(MAX_RECOVERY_PEER_REPORTS)
        .collect::<Vec<_>>();
    let committed = reports
        .iter()
        .filter_map(|report| report.committed_entry(input.index).map(|entry| (report.peer_id.as_str(), entry)))
        .collect::<Vec<_>>();
    let distinct = committed.iter().map(|(_, entry)| *entry).collect::<BTreeSet<_>>();
    if distinct.len() > 1 {
        return EntryRepairDecision::ConflictingHistory;
    }
    if let Some((from_peer, entry)) = committed.first() {
        return match &input.local {
            LocalEntry::Present(local) if local == *entry => EntryRepairDecision::KeepLocal,
            LocalEntry::Present(_) => EntryRepairDecision::ReplaceWithCommitted {
                from_peer: (*from_peer).to_string(),
                entry: (*entry).clone(),
            },
            LocalEntry::Missing | LocalEntry::Faulty => EntryRepairDecision::FetchCommitted {
                from_peer: (*from_peer).to_string(),
                entry: (*entry).clone(),
            },
        };
    }
    if matches!(input.local, LocalEntry::Present(_)) && input.local_commit_index >= input.index {
        return EntryRepairDecision::KeepLocal;
    }
    // An entry can have been committed only if a quorum of voters can hold
    // it. A present local entry is lacked by every voter that reports another
    // entry or none; with no readable local entry, only voters that hold no
    // entry at all lack one, and the participant lacks it too. Voters that
    // have not reported are unknown, never lacking.
    let lacking = match &input.local {
        LocalEntry::Present(local) => reports.iter().filter(|report| report.entry.as_ref() != Some(local)).count(),
        LocalEntry::Missing | LocalEntry::Faulty => {
            reports.iter().filter(|report| report.entry.is_none()).count().saturating_add(1)
        }
    };
    if lacking < quorum(&input.voters) {
        return EntryRepairDecision::Wait;
    }
    match input.local {
        LocalEntry::Missing => EntryRepairDecision::KeepLocal,
        LocalEntry::Present(_) | LocalEntry::Faulty => EntryRepairDecision::RemoveUncommitted,
    }
}

/// The participant's own term and vote record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LocalHardState {
    Present {
        term: u64,
        voted_for: Option<String>,
    },
    /// Lost or failed its checks.
    Lost,
}

/// How the participant may treat its term and vote.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HardStateDecision {
    KeepLocal,
    /// The vote record is gone: the participant must not vote in any term up
    /// to and including this one, because it may already have voted there.
    VoteFenced {
        through_term: u64,
    },
    /// No peer term is known yet, so no fence can be bounded: wait.
    Wait,
}

/// Decide the participant's term and vote. A peer's term bounds the fence; a
/// peer's vote is never copied.
// r[impl molten.consensus.storage_fault_recovery]
pub fn decide_hard_state(local: &LocalHardState, highest_peer_term: Option<u64>) -> HardStateDecision {
    match (local, highest_peer_term) {
        (LocalHardState::Present { .. }, _) => HardStateDecision::KeepLocal,
        (LocalHardState::Lost, Some(term)) => HardStateDecision::VoteFenced { through_term: term },
        (LocalHardState::Lost, None) => HardStateDecision::Wait,
    }
}

/// One snapshot a source holds.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SnapshotFact {
    pub last_included_index: u64,
    pub last_included_term: u64,
    pub content_ref: String,
}

/// One peer's snapshot report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeerSnapshotReport {
    pub peer_id: String,
    pub snapshot: SnapshotFact,
    pub commit_index: u64,
}

/// The snapshot repair decision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SnapshotRepairDecision {
    KeepLocal,
    InstallFrom {
        peer_id: String,
        snapshot: SnapshotFact,
    },
    /// Two committed snapshots at one index name different content.
    ConflictingContent,
    Wait,
}

/// Choose a snapshot repair. A snapshot qualifies only at an index its peer
/// has committed, and every committed report at that index must name the same
/// content identity.
// r[impl molten.consensus.storage_fault_recovery]
pub fn decide_snapshot_repair(local: Option<&SnapshotFact>, peers: &[PeerSnapshotReport]) -> SnapshotRepairDecision {
    let committed = peers
        .iter()
        .take(MAX_RECOVERY_PEER_REPORTS)
        .filter(|report| report.commit_index >= report.snapshot.last_included_index)
        .collect::<Vec<_>>();
    let Some(best_index) = committed.iter().map(|report| report.snapshot.last_included_index).max() else {
        return if local.is_some() {
            SnapshotRepairDecision::KeepLocal
        } else {
            SnapshotRepairDecision::Wait
        };
    };
    let at_best = committed
        .iter()
        .filter(|report| report.snapshot.last_included_index == best_index)
        .collect::<Vec<_>>();
    if at_best.iter().map(|report| &report.snapshot).collect::<BTreeSet<_>>().len() > 1 {
        return SnapshotRepairDecision::ConflictingContent;
    }
    match (local, at_best.first()) {
        (Some(local), _) if local.last_included_index >= best_index => SnapshotRepairDecision::KeepLocal,
        (_, Some(report)) => SnapshotRepairDecision::InstallFrom {
            peer_id: report.peer_id.clone(),
            snapshot: report.snapshot.clone(),
        },
        (_, None) => SnapshotRepairDecision::Wait,
    }
}

/// One effect the shell executes for recovery. Networking, persistence, and
/// time stay in the shell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecoveryEffect {
    /// Ask these admitted peers for their entries at these indices.
    QueryPeers {
        peers: Vec<String>,
        indices: Vec<u64>,
    },
    FetchEntry {
        from_peer: String,
        index: u64,
        entry: EntryIdentity,
    },
    /// Remove the local entry at this index and every later one.
    RemoveSuffixFrom {
        index: u64,
    },
    InstallSnapshot {
        from_peer: String,
        snapshot: SnapshotFact,
    },
}

/// Plan the effects of a set of index decisions, in index order. Waiting
/// indices are queried again; a conflict plans nothing.
// r[impl molten.consensus.storage_fault_recovery]
pub fn plan_recovery_effects(decisions: &[(u64, EntryRepairDecision)], peers: &[String]) -> Vec<RecoveryEffect> {
    let mut ordered = decisions.to_vec();
    ordered.sort_by_key(|(index, _)| *index);
    if ordered.iter().any(|(_, decision)| *decision == EntryRepairDecision::ConflictingHistory) {
        return Vec::new();
    }
    let mut effects = Vec::with_capacity(ordered.len().saturating_add(1));
    let waiting = ordered
        .iter()
        .filter(|(_, decision)| *decision == EntryRepairDecision::Wait)
        .map(|(index, _)| *index)
        .collect::<Vec<_>>();
    if !waiting.is_empty() {
        effects.push(RecoveryEffect::QueryPeers {
            peers: peers.to_vec(),
            indices: waiting,
        });
    }
    if let Some((index, _)) = ordered.iter().find(|(_, decision)| *decision == EntryRepairDecision::RemoveUncommitted) {
        effects.push(RecoveryEffect::RemoveSuffixFrom { index: *index });
    }
    for (index, decision) in &ordered {
        if let EntryRepairDecision::FetchCommitted { from_peer, entry }
        | EntryRepairDecision::ReplaceWithCommitted { from_peer, entry } = decision
        {
            effects.push(RecoveryEffect::FetchEntry {
                from_peer: from_peer.clone(),
                index: *index,
                entry: entry.clone(),
            });
        }
    }
    effects
}

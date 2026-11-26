//! The replica's complete deterministic state between steps.

use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingReplicaRead {
    pub request_ref: String,
    pub term: u64,
    pub required_index: u64,
    pub acknowledgements: alloc::collections::BTreeSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReplicaRejoin {
    Voter,
    Recovering(RecoveryRound),
    VoteFenced { through_term: u64 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveryRound {
    pub round_ref: String,
    pub fence_term: Option<u64>,
    pub peers: alloc::collections::BTreeMap<String, RecoveryObservation>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveryObservation {
    pub observation_ref: String,
    pub current_term: u64,
    pub commit_index: u64,
    pub last_log_index: u64,
    pub entries: Vec<ReplicatedEntry>,
    pub snapshot: Option<Box<ReplicaSnapshot>>,
    pub is_complete: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplicaState {
    pub profile: ReplicaProfile,
    pub node_id: String,
    pub membership: StaticMembership,
    pub role: ReplicaRole,
    pub lifecycle: ReplicaLifecycle,
    pub rejoin: ReplicaRejoin,
    pub current_term: u64,
    pub election_timer_sequence: u64,
    pub active_election_timer_ref: String,
    pub voted_for: Option<String>,
    pub leader_id: Option<String>,
    pub log: Vec<ReplicatedEntry>,
    pub commit_index: u64,
    pub last_applied: u64,
    pub snapshot: Option<ReplicaSnapshot>,
    pub completed_requests: alloc::collections::BTreeMap<String, u64>,
    pub pending_reads: alloc::collections::BTreeMap<String, PendingReplicaRead>,
    pub votes_received: alloc::collections::BTreeSet<String>,
    pub next_index: alloc::collections::BTreeMap<String, u64>,
    pub match_index: alloc::collections::BTreeMap<String, u64>,
    pub quorum_confirmed_term: Option<u64>,
}

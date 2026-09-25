
impl ReadDisposition {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Current => "current",
            Self::Local => "local",
            Self::Retryable => "retryable",
            Self::Denied => "denied",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReplicaEffect {
    PersistHardState {
        term: u64,
        voted_for: Option<String>,
    },
    PersistEntries {
        truncate_from: Option<u64>,
        entries: Vec<ReplicatedEntry>,
    },
    FlushLog {
        through_index: u64,
    },
    PersistCommit {
        through_index: u64,
    },
    PersistSnapshot {
        snapshot: ReplicaSnapshot,
    },
    Send {
        envelope: ReplicaMessageEnvelope,
    },
    ArmElectionTimer {
        timer_ref: String,
    },
    ArmHeartbeatTimer,
    RestoreApplicationSnapshot {
        snapshot: ReplicaSnapshot,
    },
    ApplyCommitted {
        entries: Vec<ReplicatedEntry>,
    },
    ProposalOutcome {
        request_ref: String,
        disposition: ProposalDisposition,
        committed_index: Option<u64>,
    },
    ReadOutcome {
        request_ref: String,
        mode: crate::fabric_consistency::ConsistencyReadMode,
        disposition: ReadDisposition,
        observed_index: u64,
    },
    LifecycleChanged {
        lifecycle: ReplicaLifecycle,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplicaState {
    pub profile: ReplicaProfile,
    pub node_id: String,
    pub membership: StaticMembership,
    pub role: ReplicaRole,
    pub lifecycle: ReplicaLifecycle,
    pub current_term: u64,
    pub election_timer_sequence: u64,
    pub active_election_timer_ref: String,
    pub voted_for: Option<String>,
    pub leader_id: Option<String>,
    pub log: Vec<ReplicatedEntry>,
    pub commit_index: u64,
    pub last_applied: u64,
    pub snapshot: Option<ReplicaSnapshot>,
    pub completed_requests: std::collections::BTreeMap<String, u64>,
    pub pending_reads: std::collections::BTreeMap<String, PendingReplicaRead>,
    pub votes_received: std::collections::BTreeSet<String>,
    pub next_index: std::collections::BTreeMap<String, u64>,
    pub match_index: std::collections::BTreeMap<String, u64>,
    pub quorum_confirmed_term: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplicaTransition {
    pub next: ReplicaState,
    pub effects: Vec<ReplicaEffect>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotPlan {
    pub snapshot: ReplicaSnapshot,
    pub compact_through: u64,
    pub retained_entries: Vec<ReplicatedEntry>,
}

pub(crate) fn snapshot_ref(snapshot: &ReplicaSnapshot) -> crate::error::Result<String> {
    crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("raft-replica-snapshot-identity-v1", vec![
        crate::preserves_rail::string(&snapshot.group_binding_ref),
        crate::preserves_rail::string(&snapshot.membership_ref),
        crate::preserves_rail::u64_value(snapshot.config_epoch),
        crate::preserves_rail::u64_value(snapshot.fencing_epoch),
        crate::preserves_rail::u64_value(snapshot.last_included_index),
        crate::preserves_rail::u64_value(snapshot.last_included_term),
        crate::preserves_rail::string(&snapshot.application_state_ref),
        crate::preserves_rail::sequence(
            snapshot
                .completed_requests
                .iter()
                .map(|(request_ref, index)| {
                    crate::preserves_rail::record("completed-request", vec![
                        crate::preserves_rail::string(request_ref),
                        crate::preserves_rail::u64_value(*index),
                    ])
                })
                .collect(),
        ),
    ]))
}

pub(crate) fn election_timer_ref(
    group_binding_ref: &str,
    node_id: &str,
    service_generation: u64,
    term: u64,
    sequence: u64,
) -> crate::error::Result<String> {
    crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("raft-election-timer-v1", vec![
        crate::preserves_rail::string(group_binding_ref),
        crate::preserves_rail::string(node_id),
        crate::preserves_rail::u64_value(service_generation),
        crate::preserves_rail::u64_value(term),
        crate::preserves_rail::u64_value(sequence),
    ]))
}

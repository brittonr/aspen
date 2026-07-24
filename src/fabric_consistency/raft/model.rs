use std::collections::BTreeMap;
use std::collections::BTreeSet;

use crate::fabric_consistency::ConsistencyReadMode;

pub const STATIC_VOTER_COUNT: usize = 3;
pub const MAX_REPLICA_LOG_ENTRIES: usize = 4_096;
pub const MAX_REPLICA_MESSAGE_ENTRIES: usize = 128;
pub const MAX_REPLICA_EFFECTS: usize = 256;
pub const MAX_PENDING_REPLICA_READS: usize = 128;
pub const INITIAL_LOG_INDEX: u64 = 1;
pub const INITIAL_TERM: u64 = 0;
pub const INITIAL_COMMIT_INDEX: u64 = 0;
pub const NEXT_TERM_STEP: u64 = 1;
pub const NEXT_LOG_INDEX_STEP: u64 = 1;
pub const INITIAL_ELECTION_TIMER_SEQUENCE: u64 = 1;
pub const NEXT_ELECTION_TIMER_SEQUENCE_STEP: u64 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplicaRole {
    Follower,
    Candidate,
    Leader,
}

impl ReplicaRole {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Follower => "follower",
            Self::Candidate => "candidate",
            Self::Leader => "leader",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplicaLifecycle {
    Running,
    Draining,
    Stopped,
}

impl ReplicaLifecycle {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Draining => "draining",
            Self::Stopped => "stopped",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StaticMembership {
    pub membership_ref: String,
    pub config_epoch: u64,
    pub voters: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplicaProfile {
    pub profile_ref: String,
    pub group_binding_ref: String,
    pub service_generation: u64,
    pub protocol_ref: String,
    pub durable_log_ref: String,
    pub snapshot_store_ref: String,
    pub timer_profile_ref: String,
    pub entropy_profile_ref: String,
    pub placement_ref: String,
    pub fencing_ref: String,
    pub fencing_epoch: u64,
    pub supervision_ref: String,
    pub resource_profile_ref: String,
    pub heartbeat_ticks: u64,
    pub election_min_ticks: u64,
    pub election_max_ticks: u64,
    pub max_log_entries: usize,
    pub max_message_entries: usize,
    pub max_effects_per_step: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplicatedEntry {
    pub index: u64,
    pub term: u64,
    pub request_ref: String,
    pub command_ref: String,
    pub command_schema_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplicaSnapshot {
    pub snapshot_ref: String,
    pub group_binding_ref: String,
    pub membership_ref: String,
    pub config_epoch: u64,
    pub fencing_epoch: u64,
    pub last_included_index: u64,
    pub last_included_term: u64,
    pub application_state_ref: String,
    pub completed_requests: BTreeMap<String, u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RaftMessage {
    RequestVote {
        term: u64,
        candidate_id: String,
        last_log_index: u64,
        last_log_term: u64,
        config_epoch: u64,
        fencing_epoch: u64,
    },
    VoteResponse {
        term: u64,
        voter_id: String,
        granted: bool,
        config_epoch: u64,
        fencing_epoch: u64,
    },
    AppendEntries {
        term: u64,
        leader_id: String,
        prev_log_index: u64,
        prev_log_term: u64,
        entries: Vec<ReplicatedEntry>,
        leader_commit: u64,
        config_epoch: u64,
        fencing_epoch: u64,
    },
    AppendResponse {
        term: u64,
        follower_id: String,
        success: bool,
        request_prev_log_index: u64,
        match_index: u64,
        conflict_index: u64,
        config_epoch: u64,
        fencing_epoch: u64,
    },
    ReadProbe {
        term: u64,
        leader_id: String,
        request_ref: String,
        required_index: u64,
        config_epoch: u64,
        fencing_epoch: u64,
    },
    ReadAcknowledgement {
        term: u64,
        follower_id: String,
        request_ref: String,
        config_epoch: u64,
        fencing_epoch: u64,
    },
    InstallSnapshot {
        term: u64,
        leader_id: String,
        snapshot: Box<ReplicaSnapshot>,
        config_epoch: u64,
        fencing_epoch: u64,
    },
    SnapshotResponse {
        term: u64,
        follower_id: String,
        snapshot_index: u64,
        accepted: bool,
        config_epoch: u64,
        fencing_epoch: u64,
    },
}

impl RaftMessage {
    pub const fn term(&self) -> u64 {
        match self {
            Self::RequestVote { term, .. }
            | Self::VoteResponse { term, .. }
            | Self::AppendEntries { term, .. }
            | Self::AppendResponse { term, .. }
            | Self::ReadProbe { term, .. }
            | Self::ReadAcknowledgement { term, .. }
            | Self::InstallSnapshot { term, .. }
            | Self::SnapshotResponse { term, .. } => *term,
        }
    }

    pub const fn config_epoch(&self) -> u64 {
        match self {
            Self::RequestVote { config_epoch, .. }
            | Self::VoteResponse { config_epoch, .. }
            | Self::AppendEntries { config_epoch, .. }
            | Self::AppendResponse { config_epoch, .. }
            | Self::ReadProbe { config_epoch, .. }
            | Self::ReadAcknowledgement { config_epoch, .. }
            | Self::InstallSnapshot { config_epoch, .. }
            | Self::SnapshotResponse { config_epoch, .. } => *config_epoch,
        }
    }

    pub const fn fencing_epoch(&self) -> u64 {
        match self {
            Self::RequestVote { fencing_epoch, .. }
            | Self::VoteResponse { fencing_epoch, .. }
            | Self::AppendEntries { fencing_epoch, .. }
            | Self::AppendResponse { fencing_epoch, .. }
            | Self::ReadProbe { fencing_epoch, .. }
            | Self::ReadAcknowledgement { fencing_epoch, .. }
            | Self::InstallSnapshot { fencing_epoch, .. }
            | Self::SnapshotResponse { fencing_epoch, .. } => *fencing_epoch,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingReplicaRead {
    pub request_ref: String,
    pub term: u64,
    pub required_index: u64,
    pub acknowledgements: BTreeSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplicaMessageEnvelope {
    pub group_binding_ref: String,
    pub service_generation: u64,
    pub from: String,
    pub to: String,
    pub message: RaftMessage,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReplicaEvent {
    ElectionTimeout {
        timer_ref: String,
    },
    HeartbeatTimeout,
    Message {
        envelope: ReplicaMessageEnvelope,
    },
    Propose {
        request_ref: String,
        command_ref: String,
        command_schema_ref: String,
    },
    Read {
        request_ref: String,
        mode: ConsistencyReadMode,
    },
    CreateSnapshot {
        application_state_ref: String,
    },
    BeginDrain,
    Stop,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProposalDisposition {
    Committed,
    Retryable,
    Denied,
    Cancelled,
    Uncertain,
}

impl ProposalDisposition {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Committed => "committed",
            Self::Retryable => "retryable",
            Self::Denied => "denied",
            Self::Cancelled => "cancelled",
            Self::Uncertain => "uncertain",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadDisposition {
    Current,
    Local,
    Retryable,
    Denied,
}

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
        mode: ConsistencyReadMode,
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
    pub completed_requests: BTreeMap<String, u64>,
    pub pending_reads: BTreeMap<String, PendingReplicaRead>,
    pub votes_received: BTreeSet<String>,
    pub next_index: BTreeMap<String, u64>,
    pub match_index: BTreeMap<String, u64>,
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

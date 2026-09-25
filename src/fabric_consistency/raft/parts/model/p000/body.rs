pub const STATIC_VOTER_COUNT: usize = 3;
pub const STATIC_QUORUM_COUNT: usize = 2;
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
    pub completed_requests: std::collections::BTreeMap<String, u64>,
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
    pub acknowledgements: std::collections::BTreeSet<String>,
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
        mode: crate::fabric_consistency::ConsistencyReadMode,
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

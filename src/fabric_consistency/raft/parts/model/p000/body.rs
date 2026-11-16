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
    /// Retained applied entries at which the hosting service creates a snapshot; below
    /// `max_log_entries`.
    pub snapshot_threshold_entries: usize,
    /// Width W of the deterministic duplicate-request window: after applying index i, a replica
    /// retains the request refs committed at indices in (i - W, i].
    pub request_retention_entries: u64,
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

include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/fabric_consistency/raft/parts/model/p000/message.rs"));

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
    pub message: Message,
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
        mode: crate::fabric_consistency::ReadMode,
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


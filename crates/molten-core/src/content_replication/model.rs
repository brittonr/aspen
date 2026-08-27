pub const MAX_CONTENTS: usize = 1_024;
pub const MAX_PEERS: usize = 64;
pub const MAX_ACTIONS: usize = 4_096;
pub const MAX_QUEUE_DEPTH: usize = 4_096;
pub const MAX_DIAGNOSTICS: usize = 128;
pub const MAX_FAULT_DOMAINS: usize = 32;
pub const MAX_REPAIR_ATTEMPTS: u32 = 16;
pub const MAX_REPLICATION_BYTES: u64 = 1_073_741_824;
pub const MAX_ID_BYTES: usize = 128;

pub const REQUIRED_PORTS: &[&str] = &[
    "authority",
    "content-store",
    "durable-state",
    "identity",
    "membership",
    "observability",
    "placement",
    "resources",
    "time",
    "transport",
];

pub const NON_CLAIMS: &[&str] = &[
    "replica count does not prove permanent durability",
    "local repair does not prove global availability",
    "transfer completion does not prove exact-once delivery",
    "replication does not grant install or execution authority",
    "replication does not decrypt or reveal protected content",
    "cleanup planning does not grant deletion authority",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Manifest {
    pub service_id: String,
    pub generation: u64,
    pub membership_epoch: u64,
    pub placement_epoch: u64,
    pub authority_ref: String,
    pub identity_ref: String,
    pub content_profile_ref: String,
    pub transport_profile_ref: String,
    pub retention_policy_ref: String,
    pub evidence_profile_ref: String,
    pub ports: Vec<String>,
    pub policy: ReplicaPolicy,
    pub repair: RepairPolicy,
    pub resources: ResourceLimits,
    pub contents: Vec<ContentRule>,
    pub non_claims: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplicaPolicy {
    pub desired_replicas: usize,
    pub minimum_verified_replicas: usize,
    pub minimum_fault_domains: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepairPolicy {
    pub max_attempts: u32,
    pub allow_handoff: bool,
    pub cleanup_after_handoff: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceLimits {
    pub max_concurrent_transfers: usize,
    pub max_transfer_bytes: u64,
    pub max_queue_depth: usize,
    pub max_timers: usize,
    pub max_diagnostics: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ContentRule {
    pub content_ref: String,
    pub manifest_ref: String,
    pub encoded_bytes: u64,
    pub protected: bool,
    pub transform_ref: Option<String>,
    pub cleanup_authority_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Peer {
    pub peer_id: String,
    pub fault_domain: String,
    pub membership_epoch: u64,
    pub placement_epoch: u64,
    pub available: bool,
    pub capacity_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Replica {
    pub content_ref: String,
    pub peer_id: String,
    pub fault_domain: String,
    pub generation: u64,
    pub membership_epoch: u64,
    pub placement_epoch: u64,
    pub present: bool,
    pub identity_verified: bool,
    pub pinned: bool,
    pub protected: bool,
    pub manifest_ref: String,
    pub cleanup_clearance_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Inventory {
    pub replicas: Vec<Replica>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum OperationOutcome {
    Planned,
    Verified,
    Cancelled,
    Uncertain,
    Corrupt,
    Failed,
}

impl OperationOutcome {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Planned => "planned",
            Self::Verified => "verified",
            Self::Cancelled => "cancelled",
            Self::Uncertain => "uncertain",
            Self::Corrupt => "corrupt",
            Self::Failed => "failed",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct PriorOperation {
    pub operation_id: String,
    pub content_ref: String,
    pub source_peer: Option<String>,
    pub target_peer: String,
    pub generation: u64,
    pub membership_epoch: u64,
    pub placement_epoch: u64,
    pub attempt: u32,
    pub outcome: OperationOutcome,
    pub result_ref: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ActionKind {
    Transfer,
    Repair,
    Handoff,
    Reuse,
    Defer,
    Cleanup,
}

impl ActionKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Transfer => "transfer",
            Self::Repair => "repair",
            Self::Handoff => "handoff",
            Self::Reuse => "reuse",
            Self::Defer => "defer",
            Self::Cleanup => "cleanup",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Action {
    pub action_id: String,
    pub operation_id: String,
    pub kind: ActionKind,
    pub content_ref: String,
    pub source_peer: Option<String>,
    pub target_peer: String,
    pub fault_domain: String,
    pub encoded_bytes: u64,
    pub pin_required: bool,
    pub preserve_protected_form: bool,
    pub cleanup_authority_ref: Option<String>,
    pub prior_result_ref: Option<String>,
    pub diagnostic: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    Ready,
    Partial,
    Denied,
}

impl Decision {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::Partial => "partial",
            Self::Denied => "denied",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    pub plan_ref: String,
    pub decision: Decision,
    pub generation: u64,
    pub membership_epoch: u64,
    pub placement_epoch: u64,
    pub actions: Vec<Action>,
    pub desired_replicas: usize,
    pub verified_replicas: usize,
    pub under_replicated: Vec<String>,
    pub deferred: Vec<String>,
    pub required_pins: Vec<String>,
    pub cleanup_candidates: Vec<String>,
    pub issues: Vec<super::Issue>,
    pub non_claims: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReconcileInput {
    pub manifest: Manifest,
    pub inventory: Inventory,
    pub peers: Vec<Peer>,
    pub history: Vec<PriorOperation>,
    pub observed_tick: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Status {
    pub plan_ref: String,
    pub generation: u64,
    pub placement_epoch: u64,
    pub desired_replicas: usize,
    pub verified_replicas: usize,
    pub under_replicated: Vec<String>,
    pub active_operations: Vec<String>,
    pub failures: Vec<String>,
    pub pins: Vec<String>,
    pub non_claims: Vec<String>,
}

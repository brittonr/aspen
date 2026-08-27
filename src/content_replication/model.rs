use molten_core::content_replication::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifecycleState {
    Declared,
    Active,
    Draining,
    Stopped,
    Failed,
}

impl LifecycleState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Declared => "declared",
            Self::Active => "active",
            Self::Draining => "draining",
            Self::Stopped => "stopped",
            Self::Failed => "failed",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceInstance {
    pub manifest: Manifest,
    pub state: LifecycleState,
    pub restart_count: u32,
    pub last_plan_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorityObservation {
    pub observation_ref: String,
    pub authority_ref: String,
    pub service_id: String,
    pub generation: u64,
    pub admitted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentityObservation {
    pub observation_ref: String,
    pub identity_ref: String,
    pub service_id: String,
    pub generation: u64,
    pub current: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MembershipObservation {
    pub observation_ref: String,
    pub membership_epoch: u64,
    pub peers: Vec<Peer>,
    pub current: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlacementObservation {
    pub observation_ref: String,
    pub membership_epoch: u64,
    pub placement_epoch: u64,
    pub current: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimeObservation {
    pub observation_ref: String,
    pub observed_tick: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceObservation {
    pub reservation_ref: String,
    pub plan_ref: String,
    pub generation: u64,
    pub admitted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PinObservation {
    pub pin_ref: String,
    pub operation_id: String,
    pub content_ref: String,
    pub generation: u64,
    pub admitted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CleanupObservation {
    pub cleanup_ref: String,
    pub operation_id: String,
    pub content_ref: String,
    pub generation: u64,
    pub admitted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransferEnvelope {
    pub transfer_ref: String,
    pub transport_verification_ref: String,
    pub operation_id: String,
    pub content_ref: String,
    pub manifest_ref: String,
    pub source_peer: String,
    pub target_peer: String,
    pub generation: u64,
    pub membership_epoch: u64,
    pub placement_epoch: u64,
    pub encoded_bytes: u64,
    pub protected: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransferOutcome {
    Received(TransferEnvelope),
    Cancelled(String),
    Uncertain(String),
    Unavailable(String),
    TimedOut(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerificationObservation {
    pub verification_ref: String,
    pub operation_id: String,
    pub replica: Replica,
    pub identity_verified: bool,
    pub authorization_admitted: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReceiptDecision {
    Complete,
    Partial,
    Denied,
}

impl ReceiptDecision {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Complete => "complete",
            Self::Partial => "partial",
            Self::Denied => "denied",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionReceipt {
    pub decision: ReceiptDecision,
    pub service_id: String,
    pub generation: u64,
    pub plan_ref: String,
    pub status_ref: String,
    pub operations: Vec<PriorOperation>,
    pub evidence_refs: Vec<String>,
    pub issues: Vec<Issue>,
    pub non_claims: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ReconcileOutcome {
    pub instance: ServiceInstance,
    pub plan: Plan,
    pub status: Status,
    pub receipt: ExecutionReceipt,
    pub resource_refs: Vec<String>,
    pub canonical_plan: super::CanonicalReplicationRecord,
    pub canonical_status: super::CanonicalReplicationRecord,
    pub canonical_receipt: super::CanonicalReplicationRecord,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperatorStatusView {
    pub service_id: String,
    pub generation: u64,
    pub placement_epoch: u64,
    pub desired_replicas: usize,
    pub verified_replicas: usize,
    pub under_replicated: Vec<String>,
    pub active_plan_ref: String,
    pub active_operations: Vec<String>,
    pub resource_refs: Vec<String>,
    pub failures: Vec<String>,
    pub pins: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub non_claims: Vec<String>,
}

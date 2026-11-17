//! Deterministic protected-operation admission and claim-specific receipts.
//!
//! The trusted shell owns this table, chooses a context unique for its lifetime,
//! and serializes checkpoint advancement, admission, cancellation and observation.
//! Guest plans, keys, decisions and imported descriptions are data, never grants.
//! Only the explicitly named trusted-host entry points may receive independently
//! acquired credentials, revocation/quota facts or authenticated observations.
//! Their Rust visibility is an embedding boundary, not credential verification.
//! A checkpoint names the shell's local authorization synchronization point;
//! it does not promise atomicity with a remote authority or exactly-once effects.

mod admission;
mod facts;
mod observation;
mod permit;
mod receipts;
mod report;

pub use permit::{Admission, AdmissionRequest, AttemptKey, AttemptSnapshot, Authorization};
pub use permit::{Decision, Denial, Execution, PreflightFailure, Started, Witness, WitnessClaim};
pub use permit::{WitnessSnapshot, WitnessState};
pub use report::{
    BoundaryError, InvocationOutcome, Observation, ObservationDescription, ObservedOutcome,
};
pub use report::{
    Receipt, ReceiptClaim, ReceiptDescription, ReceiptScope, Rejection, UntrustedReceipt,
};

/// Conventional bootstrap mapping; a host profile binds its exact resource kind.
pub const WITNESS_KIND: crate::types::ResourceKind = crate::types::ResourceKind(2);
pub const EXECUTE_RIGHT: u32 = 1;
pub const MAX_RECORDS: usize = 256;
pub const MAX_ARGUMENT_BYTES: usize = 4096;
pub const MAX_CONTRACT_BYTES: usize = 256;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActorId(pub u64);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PolicyRevision(pub u64);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceId(pub u64);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct QuotaId(pub u64);

/// The shell must not reuse this triple while any old handle can still arrive.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OwnerContext {
    pub instance: u64,
    pub invocation: u64,
    pub generation: u64,
}

/// Sequence and time are host facts, not producer-selected freshness claims.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Checkpoint {
    pub sequence: u64,
    pub now: u64,
}

/// Full resolved versioned operation name plus its declared effect identity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OperationContract {
    pub name: alloc::vec::Vec<u8>,
    pub effect: crate::types::EffId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Constraints {
    pub not_before: u64,
    pub expires_at: u64,
    pub quota: QuotaId,
    pub units: u64,
    pub scope: u64,
}

/// Untrusted description. Arguments are the complete canonical argument bytes,
/// not a digest. The shell must use that same encoding for its independent grant.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanDescription {
    pub operation: OperationContract,
    pub arguments: alloc::vec::Vec<u8>,
    pub actor: ActorId,
    pub owner: OwnerContext,
    pub policy: PolicyRevision,
    pub constraints: Constraints,
}

/// Immutable, bounded operation data. Construction establishes no authority.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Plan {
    description: PlanDescription,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlanError {
    EmptyContract,
    OversizedContract,
    OversizedArguments,
    InvalidValidity,
    EmptyQuotaCharge,
}

impl Plan {
    pub fn new(description: PlanDescription) -> Result<Self, PlanError> {
        if description.operation.name.is_empty() {
            return Err(PlanError::EmptyContract);
        }
        if description.operation.name.len() > MAX_CONTRACT_BYTES {
            return Err(PlanError::OversizedContract);
        }
        if description.arguments.len() > MAX_ARGUMENT_BYTES {
            return Err(PlanError::OversizedArguments);
        }
        if description.constraints.not_before >= description.constraints.expires_at {
            return Err(PlanError::InvalidValidity);
        }
        if description.constraints.units == 0 {
            return Err(PlanError::EmptyQuotaCharge);
        }
        Ok(Self { description })
    }

    pub fn description(&self) -> &PlanDescription {
        &self.description
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    pub witnesses: usize,
    pub attempts: usize,
    pub requests: u32,
}

/// Trusted execution-profile configuration. All operations admitted by this
/// module are protected; the shell classifies any unprotected ports separately.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Profile {
    pub owner: OwnerContext,
    pub witness_kind: crate::types::ResourceKind,
    pub authority_source: SourceId,
    pub observation_source: SourceId,
    pub invocation_source: SourceId,
    pub checkpoint: Checkpoint,
    pub limits: Limits,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SetupError {
    InvalidLimits,
    StaleCheckpoint,
}

/// Independently acquired host grant: exact arguments and context, not an allow
/// bit or digest. Public fields permit transport, not guest use as authority.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Grant {
    pub plan: Plan,
    pub kind: crate::types::ResourceKind,
    pub rights: u32,
}

/// Input to the trusted-host route. Every required current fact is explicit;
/// `None` fails closed. A new checkpoint's quota must include all prior local
/// reservations; within a checkpoint the kernel subtracts its own admissions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CurrentFacts {
    pub source: SourceId,
    pub checkpoint: Checkpoint,
    pub grant: Option<Grant>,
    pub credentials_valid: Option<bool>,
    pub revoked: Option<bool>,
    pub policy: Option<PolicyRevision>,
    pub quota_remaining: Option<u64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Counters {
    pub authorization_requests: u32,
    pub admission_requests: u32,
    pub witnesses_created: u32,
    pub witness_consumptions: u32,
    pub attempts_admitted: u32,
    pub protected_operations: u32,
    pub approved_observations: u32,
    pub successful_deliveries: u32,
}

struct WitnessRecord {
    claim: WitnessClaim,
    plan: Plan,
    state: WitnessState,
}

struct AttemptRecord {
    key: AttemptKey,
    plan: Plan,
    started: bool,
    admitted: Checkpoint,
    unknown: Option<ObservationDescription>,
    terminal: Option<ObservationDescription>,
}

struct QuotaRecord {
    quota: QuotaId,
    checkpoint: Checkpoint,
    available: u64,
    spent: u64,
}

/// The local delivery commit is the approved success observation for an
/// invocation claim; external operation outcomes remain a separate contract.
struct DeliveryRecord {
    attempt: AttemptKey,
    checkpoint: Checkpoint,
}

/// Bounded invocation-local state. Capacity is reserved before any witness can
/// be issued. No external callbacks, ambient state, clock reads or I/O occur.
pub struct Authority {
    profile: Profile,
    witnesses: alloc::vec::Vec<WitnessRecord>,
    attempts: alloc::vec::Vec<AttemptRecord>,
    quotas: alloc::vec::Vec<QuotaRecord>,
    effects: alloc::vec::Vec<crate::types::EffId>,
    counters: Counters,
    invocation: InvocationOutcome,
    delivered: Option<DeliveryRecord>,
}

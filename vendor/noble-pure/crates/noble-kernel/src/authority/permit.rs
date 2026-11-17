#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Denial {
    WrongAuthoritySource,
    StaleFacts,
    MissingFacts,
    InvalidCredentials,
    Revoked,
    ChangedPlan,
    WrongOwnerContext,
    ChangedPolicy,
    NotYetValid,
    Expired,
    QuotaExhausted,
    InconsistentQuota,
    WrongResourceKind,
    MissingRights,
    UnknownWitness,
    StaleGeneration,
    WrongScope,
    ConsumedWitness,
    RetiredWitness,
    UntrustedWitnessClaim,
    InvocationFinished,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PreflightFailure {
    WitnessCapacity,
    AttemptCapacity,
    RequestCapacity,
}

/// Pure policy output. There is deliberately no conversion from this to Witness.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Decision {
    Allow,
    Deny(Denial),
}

/// Untrusted routing claim. It can be inspected or rejected but never imported
/// into an opaque witness. Admission also requires the original host-owned value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WitnessClaim {
    pub owner: super::OwnerContext,
    pub slot: usize,
    pub generation: u64,
    pub kind: crate::types::ResourceKind,
    pub rights: u32,
    pub scope: u64,
}

/// Move-only host obligation; no public fields, Default, Clone or Copy.
///
/// ```compile_fail
/// use noble_kernel::authority::Witness;
/// let _ = Witness::default();
/// ```
///
/// ```compile_fail
/// use noble_kernel::authority::Witness;
/// fn duplicate(witness: Witness) { let _ = witness.clone(); }
/// ```
///
/// ```compile_fail
/// use noble_kernel::authority::Witness;
/// fn requires_copy<T: Copy>() {}
/// requires_copy::<Witness>();
/// ```
///
/// ```compile_fail
/// use noble_kernel::authority::{Witness, WitnessClaim};
/// fn forge(claim: WitnessClaim) -> Witness { Witness { claim } }
/// ```
///
/// ```compile_fail
/// use noble_kernel::authority::{Witness, WitnessClaim};
/// fn import(claim: WitnessClaim) -> Witness { Witness::from(claim) }
/// ```
#[derive(Debug)]
#[must_use]
pub struct Witness {
    pub(super) claim: WitnessClaim,
}

impl Witness {
    /// A description is not a constructor or a serializable live capability.
    pub fn claim(&self) -> WitnessClaim {
        self.claim
    }
}

#[derive(Debug)]
pub enum Authorization {
    Authorized(Witness),
    Denied(super::Rejection),
    Preflight(super::Rejection),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AdmissionRequest {
    pub claim: WitnessClaim,
    pub plan: super::Plan,
}

/// Failed preflight and denial both return the caller's opaque obligation.
/// Its retained liveness is unchanged; a stale/consumed token is not resurrected.
#[derive(Debug)]
pub enum Admission {
    Committed(Execution),
    Denied {
        witness: Option<Witness>,
        rejection: super::Rejection,
    },
    Preflight {
        witness: Option<Witness>,
        rejection: super::Rejection,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AttemptKey {
    pub owner: super::OwnerContext,
    pub slot: usize,
    pub generation: u64,
}

/// One-shot permission to cross the shell execution boundary after commit.
#[derive(Debug)]
#[must_use]
pub struct Execution {
    pub(super) attempt: AttemptKey,
}

impl Execution {
    pub fn attempt(&self) -> AttemptKey {
        self.attempt
    }
}

/// Issued only after `start_execution` records the effect, before external work.
/// The shell consumes this value exactly once when performing that work.
#[derive(Debug)]
#[must_use]
pub struct Started {
    pub(super) attempt: AttemptKey,
    pub(super) plan: super::Plan,
}

impl Started {
    pub fn attempt(&self) -> AttemptKey {
        self.attempt
    }

    pub fn plan(&self) -> &super::Plan {
        &self.plan
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[tigerstyle::state_machine_enum]
pub enum WitnessState {
    Live,
    Consumed,
    Retired,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WitnessSnapshot {
    pub claim: WitnessClaim,
    pub state: WitnessState,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AttemptSnapshot {
    pub key: AttemptKey,
    pub effect: crate::types::EffId,
    pub started: bool,
    pub outcome: Option<super::ObservedOutcome>,
}

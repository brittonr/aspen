#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[tigerstyle::state_machine_enum]
pub enum InvocationOutcome {
    Pending,
    Succeeded,
    Failed,
    Cancelled,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[tigerstyle::state_machine_enum]
pub enum ObservedOutcome {
    Unknown,
    OperationFailure,
    OperationSuccess,
}

/// Schema-valid data alone has no provenance. There is no `trusted` flag.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObservationDescription {
    pub attempt: super::AttemptKey,
    pub plan: super::Plan,
    pub source: super::SourceId,
    pub checkpoint: super::Checkpoint,
    pub outcome: ObservedOutcome,
}

/// An approved observation is not constructible from an earlier boundary role.
///
/// ```compile_fail
/// use noble_kernel::authority::{Authority, AttemptKey, Plan, ReceiptClaim};
/// fn forge(table: &Authority, attempt: AttemptKey, plan: Plan) {
///     let _ = table.receipt(attempt, ReceiptClaim::OperationSuccess, Some(&plan));
/// }
/// ```
///
/// ```compile_fail
/// use noble_kernel::authority::{Authority, AttemptKey, Witness, ReceiptClaim};
/// fn forge(table: &Authority, attempt: AttemptKey, witness: Witness) {
///     let _ = table.receipt(attempt, ReceiptClaim::OperationSuccess, Some(&witness));
/// }
/// ```
///
/// ```compile_fail
/// use noble_kernel::authority::{Authority, AttemptKey, ReceiptClaim};
/// fn forge(table: &Authority, attempt: AttemptKey) {
///     let _ = table.receipt(attempt, ReceiptClaim::OperationSuccess, Some(&attempt));
/// }
/// ```
#[derive(Debug)]
pub struct Observation {
    pub(super) attempt: super::AttemptKey,
    pub(super) outcome: ObservedOutcome,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BoundaryError {
    WrongContext,
    UnknownAttempt,
    StaleGeneration,
    NotStarted,
    AlreadyStarted,
    WrongSource,
    StaleObservation,
    ChangedPlan,
    ConflictingObservation,
    MissingObservation,
    InapplicableObservation,
    InvocationFinished,
    ImportedTrustFlag,
    InapplicableReceipt,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[tigerstyle::state_machine_enum]
pub enum ReceiptClaim {
    Denial,
    PreflightFailure,
    Unknown,
    OperationFailure,
    OperationSuccess,
    InvocationSuccess,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReceiptScope {
    AuthorizationBoundary,
    AdmissionBoundary,
    OperationBoundary,
    InvocationDelivery,
}

/// Exported data, not authority, authenticity, or arbitrary behavioral proof.
/// The context and observation scope are explicit and there are no trust flags.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReceiptDescription {
    pub plan: super::Plan,
    pub boundary_owner: super::OwnerContext,
    pub attempt: Option<super::AttemptKey>,
    pub source: super::SourceId,
    pub checkpoint: super::Checkpoint,
    pub claim: ReceiptClaim,
    pub scope: ReceiptScope,
    pub invocation: InvocationOutcome,
    pub denial: Option<super::Denial>,
    pub preflight: Option<super::PreflightFailure>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Receipt {
    pub(super) description: ReceiptDescription,
}

impl Receipt {
    pub fn description(&self) -> &ReceiptDescription {
        &self.description
    }
}

#[derive(Debug)]
pub struct Rejection {
    pub(super) receipt: Receipt,
}

impl Rejection {
    pub fn receipt(&self) -> &Receipt {
        &self.receipt
    }
}

/// Imported descriptions stay untrusted until independently matched against an
/// approved observation. Even a claimed `false` trust flag is rejected.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UntrustedReceipt {
    pub(super) description: ReceiptDescription,
}

impl UntrustedReceipt {
    pub fn import(
        description: ReceiptDescription,
        trust_flag: Option<bool>,
    ) -> Result<Self, BoundaryError> {
        match trust_flag {
            Some(_) => Err(BoundaryError::ImportedTrustFlag),
            None => Ok(Self { description }),
        }
    }

    pub fn description(&self) -> &ReceiptDescription {
        &self.description
    }
}

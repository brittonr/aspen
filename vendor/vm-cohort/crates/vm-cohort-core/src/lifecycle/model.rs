use alloc::vec::Vec;

use serde::{Deserialize, Serialize};

use crate::{CloneRef, CohortPlan, CohortRef, EffectKind, OperationRef, ReceiptRef, ResourceRef};

/// Observed outcome for one planned host effect.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OperationOutcome {
    /// Exact effect completed.
    Success,
    /// No effect completed and a reviewed retry class applies.
    TransientFailure,
    /// Permanent denial or failure.
    PermanentFailure,
    /// The effect may or may not have completed.
    OutcomeUnknown,
    /// Current cancellation stopped the effect.
    Cancelled,
}

/// Exact observation for one effect request.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EffectObservation {
    /// Cohort identity.
    pub cohort_ref: CohortRef,
    /// Clone identity.
    pub clone_ref: CloneRef,
    /// Planned operation identity.
    pub operation_ref: OperationRef,
    /// Planned effect kind.
    pub kind: EffectKind,
    /// Observed outcome.
    pub outcome: OperationOutcome,
    /// Detached observation identity.
    pub observation_ref: ReceiptRef,
}

/// Exact cleanup obligation for one clone-private resource.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CleanupObligation {
    /// Clone identity.
    pub clone_ref: CloneRef,
    /// Resource that must be released.
    pub resource_ref: ResourceRef,
    /// Cleanup operation identity.
    pub cleanup_operation_ref: OperationRef,
}

/// Cleanup outcome for one obligation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CleanupOutcome {
    /// Resource release was observed.
    Released,
    /// Release failed and remains required.
    Failed,
    /// Release effect outcome is unknown.
    OutcomeUnknown,
}

/// Exact cleanup observation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CleanupObservation {
    /// Cohort identity.
    pub cohort_ref: CohortRef,
    /// Clone identity.
    pub clone_ref: CloneRef,
    /// Cleanup operation identity.
    pub cleanup_operation_ref: OperationRef,
    /// Target resource identity.
    pub resource_ref: ResourceRef,
    /// Observed cleanup outcome.
    pub outcome: CleanupOutcome,
    /// Detached observation identity.
    pub observation_ref: ReceiptRef,
}

/// Public cohort lifecycle state.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CohortPhase {
    /// Plan exists and no effect succeeded.
    Planned,
    /// At least one effect succeeded but activation is incomplete.
    Preparing,
    /// Every clone activated.
    Active,
    /// Cleanup obligations remain.
    CleanupRequired,
    /// At least one cleanup or creation outcome is unknown.
    CleanupUncertain,
    /// Every cleanup obligation was released.
    Cleaned,
    /// Cohort failed before resource allocation.
    Failed,
    /// Operator stopped recovery while obligations remain explicit.
    Abandoned,
}

/// Complete deterministic lifecycle reduction state.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CohortState {
    /// Immutable cohort plan.
    pub plan: CohortPlan,
    /// Current public phase.
    pub phase: CohortPhase,
    /// Successful effect operations.
    pub completed_operations: Vec<OperationRef>,
    /// Activated clones.
    pub activated_clones: Vec<CloneRef>,
    /// Required cleanup resources.
    pub cleanup_obligations: Vec<CleanupObligation>,
    /// Successfully released cleanup operations.
    pub released_cleanup_operations: Vec<OperationRef>,
    /// Accepted observation identities.
    pub observation_refs: Vec<ReceiptRef>,
}

/// Lifecycle reduction issue.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum LifecycleIssue {
    /// Observation belongs to another cohort.
    WrongCohort,
    /// Observation belongs to another clone.
    WrongClone,
    /// Operation was not planned or changed kind.
    UnplannedOperation,
    /// Observation identity was reused for different facts.
    DuplicateObservation,
    /// Observation count exceeds the public bound.
    ObservationLimit,
    /// Cleanup obligation count exceeds the public bound.
    CleanupLimit,
    /// Cleanup observation does not match one exact obligation.
    WrongCleanup,
    /// Terminal state cannot reactivate or accept creation effects.
    Terminal,
    /// Identity construction failed.
    Identity,
}

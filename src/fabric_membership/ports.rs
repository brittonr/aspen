//! Application-owned capability contracts for membership orchestration.

#![allow(
    tigerstyle::non_trait_imports,
    tigerstyle::path_segment_repetition,
    reason = "membership port contracts use explicit domain-owned request and result types"
)]

use super::*;
use crate::fabric::FabricPortResult;

// r[impl molten.modularity.fabric_boundary.ports]

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MembershipProviderSnapshot {
    pub profile: MembershipSourceProfile,
    pub view: MembershipView,
    pub descriptors: Vec<NodeDescriptor>,
    pub detector_profiles: Vec<FailureDetectorProfile>,
    pub failure_observations: Vec<FailureObservation>,
    pub reservations: Vec<CapacityReservation>,
    pub observed_now_ticks: u64,
    pub required_compatibility_ref: String,
}

pub trait MembershipPlacementProvider {
    fn provider_kind(&self) -> MembershipProviderKind;
    fn snapshot(&mut self) -> FabricPortResult<MembershipProviderSnapshot>;
}

/// Canonical current assignment observed from the assignment persistence owner.
/// This is an observation, not permission: callers must independently admit exact operator scope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CurrentAssignment {
    pub assignment_ref: String,
    pub assignment: RoleAssignment,
}

/// An intent recorded by `record_intent` that has not been committed, aborted, or reconciled.
/// It is an observation for recovery, not permission to replay its role effect.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingAssignmentIntent {
    /// Canonical identity of the last committed assignment the intent was recorded against.
    pub committed_ref: String,
    pub committed: RoleAssignment,
    pub transition: AssignmentTransition,
    pub intent_ref: String,
}

/// Resolution of a pending intent whose role effect may already have happened. Each route
/// carries the port whose observation or authorization the owner verifies; the caller supplies
/// no evidence refs.
pub enum PendingIntentReconciliation<'a> {
    /// Commit the exact pending transition only if the role lifecycle port observes, for this
    /// exact intent, that its effect completed; the observed effect ref becomes the commit's
    /// effect evidence. Any other observation leaves the intent pending and uncertain.
    ConfirmObservedEffect {
        lifecycle: &'a mut dyn ExtensionRoleLifecyclePort,
    },
    /// The effect outcome cannot be established: quarantine the last committed assignment under a
    /// transition ref derived from the intent, mark the old owner uncertain, and persist the
    /// quarantine reconciliation record citing the authorization the quarantine authority issued
    /// for this exact assignment and intent. The pending role effect is never replayed; the slot
    /// moves on only through a fenced quarantine successor.
    QuarantineUncertain {
        authority: &'a mut dyn QuarantineAuthorityPort,
    },
}

/// The exact recovery a quarantine authority is asked to authorize.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuarantineAuthorizationRequest {
    pub assignment_id: String,
    /// Canonical ref of the assignment as last committed by the owner.
    pub committed_assignment_ref: String,
    pub basis: QuarantineReconciliationBasis,
}

/// An authority's decision for one exact `QuarantineAuthorizationRequest`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuarantineAuthorization {
    pub request: QuarantineAuthorizationRequest,
    pub authorization_ref: String,
}

/// Operator or policy authority that decides whether an uncertain role slot may be resolved by
/// quarantine. A denial is an error; an authorization must echo the exact request.
pub trait QuarantineAuthorityPort {
    fn authorize_quarantine(
        &mut self,
        request: &QuarantineAuthorizationRequest,
    ) -> FabricPortResult<QuarantineAuthorization>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReconciledAssignment {
    pub reconcile_ref: String,
    pub current: CurrentAssignment,
}

/// The complete assignment persistence owner: the committed assignment log plus the recovery
/// routes for pending intents and quarantined role slots.
pub trait AssignmentPersistence: AssignmentLog + PendingIntentRecovery + QuarantineRecovery {}

impl<T: AssignmentLog + PendingIntentRecovery + QuarantineRecovery + ?Sized> AssignmentPersistence for T {}

/// Committed role ownership and its strict intent/commit protocol.
pub trait AssignmentLog {
    /// Insert pure Proposed state without replacing an ID. A role successor must bind its exact
    /// released predecessor and advance the durable role-slot epoch and fencing token.
    fn initialize(&mut self, proposal: &AssignmentProposal) -> FabricPortResult<CurrentAssignment>;

    /// Read committed current role ownership. An unresolved intent or superseded ID fails closed,
    /// never returning an old apparently-current assignment. Pending role effects must not replay.
    fn current(&self, assignment_id: &str) -> FabricPortResult<Option<CurrentAssignment>>;

    fn record_intent(
        &mut self,
        current: &RoleAssignment,
        transition: &AssignmentTransition,
    ) -> FabricPortResult<String>;

    fn commit(
        &mut self,
        transition: &AssignmentTransition,
        intent_ref: &str,
        role_effect_ref: Option<&str>,
    ) -> FabricPortResult<String>;
}

/// Recovery of an intent recorded but not committed, without replaying its role effect.
pub trait PendingIntentRecovery {
    /// Read the unresolved intent for an assignment, if any. Never replays its role effect.
    // r[impl molten.fabric_membership.recoverable_pending_and_quarantined_slots]
    fn inspect_pending(&self, assignment_id: &str) -> FabricPortResult<Option<PendingAssignmentIntent>>;

    /// Clear the exact pending intent and keep the last committed assignment. An effectless
    /// transition (`reserve`, `assign`, `acknowledge`, per the persisted intent) aborts directly.
    /// Otherwise the owner asks `lifecycle` about this exact intent and aborts only on a
    /// `RoleEffectObservation::NotExecuted` bound to it; anything else fails closed as uncertain,
    /// leaving `reconcile_pending_intent` as the only route.
    // r[impl molten.fabric_membership.recoverable_pending_and_quarantined_slots]
    fn abort_pre_effect_intent(
        &mut self,
        assignment_id: &str,
        intent_ref: &str,
        lifecycle: &mut dyn ExtensionRoleLifecyclePort,
    ) -> FabricPortResult<CurrentAssignment>;

    /// Resolve the exact pending intent whose role effect may have happened, preserving
    /// uncertainty unless the lifecycle port observes the exact intent's effect as executed.
    // r[impl molten.fabric_membership.recoverable_pending_and_quarantined_slots]
    fn reconcile_pending_intent(
        &mut self,
        assignment_id: &str,
        intent_ref: &str,
        reconciliation: PendingIntentReconciliation<'_>,
    ) -> FabricPortResult<ReconciledAssignment>;
}

/// Reconciliation and fenced succession of quarantined role slots.
pub trait QuarantineRecovery {
    /// Read the reconciliation record this owner persisted when the assignment was quarantined.
    // r[impl molten.fabric_membership.recoverable_pending_and_quarantined_slots]
    fn quarantine_reconciliation(&self, assignment_id: &str) -> FabricPortResult<Option<QuarantineReconciliation>>;

    /// Migrate a `Quarantined` record written before reconciliation records existed: with no
    /// pending intent and no record, ask `authority` to authorize the exact quarantined
    /// assignment and its quarantine transition, then persist a `LegacyRecord` reconciliation.
    // r[impl molten.fabric_membership.recoverable_pending_and_quarantined_slots]
    fn reconcile_legacy_quarantine(
        &mut self,
        assignment_id: &str,
        authority: &mut dyn QuarantineAuthorityPort,
    ) -> FabricPortResult<QuarantineReconciliation>;

    /// Insert a fenced successor over a `Quarantined` role-slot owner. Before the role slot
    /// changes owner, the clearance is checked against the persisted reconciliation record for
    /// the exact quarantined predecessor and against a fence observed from `fencing`, which must
    /// already enforce the successor's advanced epoch and token.
    // r[impl molten.fabric_membership.recoverable_pending_and_quarantined_slots]
    fn initialize_quarantine_successor(
        &mut self,
        proposal: &AssignmentProposal,
        clearance: &QuarantineClearance,
        fencing: &mut dyn FencingAuthorityPort,
    ) -> FabricPortResult<CurrentAssignment>;
}

/// Independent observation of the fence a fencing authority currently enforces.
pub trait FencingAuthorityPort {
    /// Observe the fence enforced for `successor`'s role slot and fencing profile.
    fn observe_fence(&mut self, successor: &RoleAssignment) -> FabricPortResult<FenceObservation>;
}

/// The role lifecycle port's evidence about one exact recorded intent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoleIntentEvidence {
    pub assignment_id: String,
    /// The command transition ref of the intent's transition.
    pub transition_ref: String,
    pub kind: AssignmentCommandKind,
    pub intent_ref: String,
    /// The port's evidence identity: a refusal record, or the executed effect's ref.
    pub evidence_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RoleEffectObservation {
    /// The port definitely never executed the intent's role effect.
    NotExecuted(RoleIntentEvidence),
    /// The port executed the intent's role effect; `evidence_ref` is the effect ref.
    Executed(RoleIntentEvidence),
    /// The port cannot establish the outcome.
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoleEffectFailure {
    pub message: String,
    pub effect_may_have_happened: bool,
}

pub trait ExtensionRoleLifecyclePort {
    fn activate(&mut self, assignment: &RoleAssignment) -> std::result::Result<String, RoleEffectFailure>;
    fn begin_drain(&mut self, assignment: &RoleAssignment) -> std::result::Result<String, RoleEffectFailure>;
    fn begin_replacement(&mut self, assignment: &RoleAssignment) -> std::result::Result<String, RoleEffectFailure>;
    fn release(&mut self, assignment: &RoleAssignment) -> std::result::Result<String, RoleEffectFailure>;
    fn fail(&mut self, assignment: &RoleAssignment) -> std::result::Result<String, RoleEffectFailure>;
    fn quarantine(&mut self, assignment: &RoleAssignment) -> std::result::Result<String, RoleEffectFailure>;

    /// Observe the outcome of the role effect for one exact recorded intent of `transition`.
    /// Ports that keep no intent-bound effect record report `Unknown`, so recovery stays
    /// uncertain rather than trusting a caller's claim.
    fn observe_intent(&mut self, _transition: &AssignmentTransition, _intent_ref: &str) -> RoleEffectObservation {
        RoleEffectObservation::Unknown
    }
}

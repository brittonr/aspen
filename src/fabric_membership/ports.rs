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

pub trait AssignmentPersistence {
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
}

use std::fmt;

use artifact_auth_core::ArtifactStatement;
use molten_core::world_head::WorldBranchId;
use molten_core::world_head::WorldHeadAuthorityObservation;
use molten_core::world_head::WorldHeadConflictSet;
use molten_core::world_head::WorldHeadPolicyRef;
use molten_core::world_head::WorldHeadSignerRole;
use molten_core::world_head::WorldHeadState;
use molten_core::world_head::WorldHeadTransitionPlan;

use super::CanonicalWorldHeadConflict;
use super::CanonicalWorldHeadTransitionReceipt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldHeadPortError {
    pub class: &'static str,
    pub message: String,
}

impl WorldHeadPortError {
    pub fn new(class: &'static str, message: impl Into<String>) -> Self {
        Self {
            class,
            message: message.into(),
        }
    }
}

impl fmt::Display for WorldHeadPortError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.class, self.message)
    }
}

impl std::error::Error for WorldHeadPortError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldHeadSignerIdentity {
    pub producer_id: String,
    pub key_id: String,
    pub key_identity: artifact_auth_core::ArtifactRef,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldHeadSignatureCarrier {
    pub producer_id: String,
    pub key_id: String,
    pub public_key_bytes: Vec<u8>,
    pub signature_bytes: Vec<u8>,
    pub key_generation: u64,
    pub role: WorldHeadSignerRole,
    pub authority_admitted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldHeadFreshAdmission {
    pub authentication_passed: bool,
    pub authority: WorldHeadAuthorityObservation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorldHeadMutationOutcome {
    Applied,
    AlreadyApplied,
    Stale,
    Uncertain,
}

impl WorldHeadMutationOutcome {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Applied => "applied",
            Self::AlreadyApplied => "already-applied",
            Self::Stale => "stale",
            Self::Uncertain => "uncertain",
        }
    }
}

pub trait WorldHeadSigningPort {
    fn signer_identity(
        &mut self,
        role: WorldHeadSignerRole,
        policy_ref: &WorldHeadPolicyRef,
    ) -> Result<WorldHeadSignerIdentity, WorldHeadPortError>;

    fn sign_statement(
        &mut self,
        statement: &ArtifactStatement,
        role: WorldHeadSignerRole,
        policy_ref: &WorldHeadPolicyRef,
    ) -> Result<WorldHeadSignatureCarrier, WorldHeadPortError>;
}

pub trait WorldHeadAuthorityPort {
    fn observe_authority(
        &mut self,
        branch_id: &WorldBranchId,
        policy_ref: &WorldHeadPolicyRef,
        expected_generation: u64,
    ) -> Result<WorldHeadAuthorityObservation, WorldHeadPortError>;
}

pub trait WorldHeadStatePort {
    fn read_head(&self, branch_id: &WorldBranchId) -> Result<Option<WorldHeadState>, WorldHeadPortError>;

    fn apply_transition<F>(
        &mut self,
        plan: &WorldHeadTransitionPlan,
        receipt: &CanonicalWorldHeadTransitionReceipt,
        recheck: F,
    ) -> Result<WorldHeadMutationOutcome, WorldHeadPortError>
    where
        F: FnOnce(Option<&WorldHeadState>) -> Result<WorldHeadFreshAdmission, WorldHeadPortError>;
}

pub trait WorldHeadConflictPort {
    fn record_conflict(
        &mut self,
        conflict: &WorldHeadConflictSet,
        canonical: &CanonicalWorldHeadConflict,
    ) -> Result<(), WorldHeadPortError>;

    fn read_conflicts(&self, branch_id: &WorldBranchId) -> Result<Vec<Vec<u8>>, WorldHeadPortError>;
}

pub trait WorldHeadReconciliationPort {
    fn record_uncertain_transition(
        &mut self,
        plan: &WorldHeadTransitionPlan,
        receipt: &CanonicalWorldHeadTransitionReceipt,
    ) -> Result<(), WorldHeadPortError>;
}

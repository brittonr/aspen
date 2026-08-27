use std::collections::BTreeSet;

use super::WorldBranchClass;
use super::WorldBranchId;
use super::WorldHeadAuthenticationDecisionRef;
use super::WorldHeadAuthorityRef;
use super::WorldHeadClaimRef;
use super::WorldHeadCurrentnessClass;
use super::WorldHeadCurrentnessRef;
use super::WorldHeadPolicyRef;
use super::WorldHeadPurpose;
use super::WorldHeadSignerRole;
use super::WorldHeadStatementRef;
use crate::world_commit::WorldCommitRef;

pub const WORLD_HEAD_CLAIM_SCHEMA: &str = "molten.world-head-claim.v1";
pub const WORLD_HEAD_CONFLICT_SCHEMA: &str = "molten.world-head-conflict.v1";
pub const WORLD_HEAD_TRANSITION_SCHEMA: &str = "molten.world-head-transition.v1";
pub const WORLD_HEAD_ARTIFACT_AUTH_DOMAIN: &str = "molten.world-head-claim.v1";
pub const WORLD_HEAD_ARTIFACT_AUTH_PROFILE: &str = "molten.crypto.ed25519-iroh.v1";
pub const WORLD_HEAD_ARTIFACT_AUTH_PURPOSE: &str = "authority";
pub const MAX_WORLD_BRANCH_ID_BYTES: usize = 128;
pub const MAX_WORLD_HEAD_SIGNERS: usize = 16;
pub const MAX_WORLD_HEAD_CONFLICTS: u32 = 16;
pub const MAX_WORLD_HEAD_HISTORY_NODES: usize = 4_096;
pub const MAX_WORLD_HEAD_PARENTS: usize = 64;
pub const MAX_WORLD_HEAD_LABEL_BYTES: usize = 128;
pub const MAX_WORLD_HEAD_METADATA_ENTRIES: usize = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldHeadClaim {
    pub branch_id: WorldBranchId,
    pub branch_class: WorldBranchClass,
    pub expected_head: Option<WorldCommitRef>,
    pub successor_head: WorldCommitRef,
    pub expected_generation: u64,
    pub successor_generation: u64,
    pub purpose: WorldHeadPurpose,
    pub policy_ref: WorldHeadPolicyRef,
    pub source_heads: Vec<WorldCommitRef>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldHeadState {
    pub branch_id: WorldBranchId,
    pub branch_class: WorldBranchClass,
    pub head: WorldCommitRef,
    pub generation: u64,
    pub policy_ref: WorldHeadPolicyRef,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldCommitHistoryNode {
    pub commit: WorldCommitRef,
    pub parents: Vec<WorldCommitRef>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldHeadPolicy {
    pub policy_ref: WorldHeadPolicyRef,
    pub allowed_branch_classes: BTreeSet<WorldBranchClass>,
    pub allowed_purposes: BTreeSet<WorldHeadPurpose>,
    pub allowed_signer_roles: BTreeSet<WorldHeadSignerRole>,
    pub signature_threshold: usize,
    pub max_conflicts: u32,
    pub allow_recovery: bool,
    pub require_independent_recovery_currentness: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldHeadSignerObservation {
    pub key_identity_ref: String,
    pub role: WorldHeadSignerRole,
    pub authenticated: bool,
    pub current: bool,
    pub revoked: bool,
    pub authority_admitted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldHeadAuthenticationObservation {
    pub statement_ref: WorldHeadStatementRef,
    pub decision_ref: WorldHeadAuthenticationDecisionRef,
    pub passed: bool,
    pub purpose_matches: bool,
    pub policy_matches: bool,
    pub signers: Vec<WorldHeadSignerObservation>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldHeadAuthorityObservation {
    pub authority_ref: WorldHeadAuthorityRef,
    pub policy_ref: WorldHeadPolicyRef,
    pub admitted: bool,
    pub observed_generation: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldHeadCurrentnessObservation {
    pub durable_generation_observed: bool,
    pub independent_ref: Option<WorldHeadCurrentnessRef>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldHeadBounds {
    pub max_history_nodes: usize,
    pub max_parents_per_commit: usize,
    pub max_signers: usize,
    pub max_conflicts: u32,
}

impl WorldHeadBounds {
    pub const fn standard() -> Self {
        Self {
            max_history_nodes: MAX_WORLD_HEAD_HISTORY_NODES,
            max_parents_per_commit: MAX_WORLD_HEAD_PARENTS,
            max_signers: MAX_WORLD_HEAD_SIGNERS,
            max_conflicts: MAX_WORLD_HEAD_CONFLICTS,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldHeadPlanRequest {
    pub claim_ref: WorldHeadClaimRef,
    pub claim: WorldHeadClaim,
    pub current: Option<WorldHeadState>,
    pub history: Vec<WorldCommitHistoryNode>,
    pub policy: WorldHeadPolicy,
    pub authentication: WorldHeadAuthenticationObservation,
    pub authority: WorldHeadAuthorityObservation,
    pub currentness: WorldHeadCurrentnessObservation,
    pub bounds: WorldHeadBounds,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldHeadTransitionPlan {
    pub claim_ref: WorldHeadClaimRef,
    pub before: Option<WorldHeadState>,
    pub after: WorldHeadState,
    pub choregraph_before_identity: Option<String>,
    pub choregraph_after_identity: String,
    pub currentness: WorldHeadCurrentnessClass,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct WorldHeadConflictMember {
    pub claim_ref: WorldHeadClaimRef,
    pub successor_head: WorldCommitRef,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldHeadConflictSet {
    pub branch_id: WorldBranchId,
    pub expected_head: WorldCommitRef,
    pub expected_generation: u64,
    pub members: Vec<WorldHeadConflictMember>,
    pub conflict_ref: String,
}

use crate::world_head::WorldCommitHistoryNode;
use crate::world_head::WorldHeadAuthenticationObservation;
use crate::world_head::WorldHeadAuthorityObservation;
use crate::world_head::WorldHeadBounds;
use crate::world_head::WorldHeadClaim;
use crate::world_head::WorldHeadClaimRef;
use crate::world_head::WorldHeadConflictSet;
use crate::world_head::WorldHeadCurrentnessObservation;
use crate::world_head::WorldHeadIssue;
use crate::world_head::WorldHeadPolicy;
use crate::world_head::WorldHeadState;
use crate::world_head::WorldHeadTransitionPlan;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteWorldHeadClaim {
    pub peer_ref: String,
    pub claim_ref: WorldHeadClaimRef,
    pub claim: WorldHeadClaim,
    pub authentication: WorldHeadAuthenticationObservation,
    pub authority: WorldHeadAuthorityObservation,
    pub currentness: WorldHeadCurrentnessObservation,
    pub encoded_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldClaimAdmissionRequest {
    pub claims: Vec<RemoteWorldHeadClaim>,
    pub current: Option<WorldHeadState>,
    pub history: Vec<WorldCommitHistoryNode>,
    pub policy: WorldHeadPolicy,
    pub bounds: WorldHeadBounds,
    pub max_claims: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldClaimDenial {
    pub claim_ref: WorldHeadClaimRef,
    pub issues: Vec<WorldHeadIssue>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldClaimAdmission {
    pub admitted: Vec<WorldHeadTransitionPlan>,
    pub denied: Vec<WorldClaimDenial>,
    pub conflict: Option<WorldHeadConflictSet>,
    pub selected_claim: Option<WorldHeadClaimRef>,
    pub head_mutation_authorized: bool,
    pub non_claims: Vec<String>,
}

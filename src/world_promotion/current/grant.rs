//! Parsing of a signed grant document into the typed grant its signature covers.

use molten_core::world_commit::WorldCommitRef;
use molten_core::world_head::WorldBranchId;
use molten_core::world_head::WorldHeadPolicyRef;
use molten_core::world_promotion::*;

use super::SignedPromotionAuthority;
use crate::error::Failure;
use crate::error::Result;

impl SignedPromotionAuthority {
    /// The typed grant this document names. The action must be known and exactly its target
    /// fields must be present.
    pub fn grant(&self) -> Result<WorldPromotionGrant> {
        if self.generation == 0 {
            return Err(Failure::invalid_harness("promotion authority is incomplete"));
        }
        let action = WorldPromotionGrantAction::parse(&self.action)
            .ok_or_else(|| Failure::invalid_harness("promotion authority action is unsupported"))?;
        Ok(WorldPromotionGrant {
            branch_id: WorldBranchId::new(self.branch_id.clone())
                .map_err(|error| Failure::invalid_harness(format!("invalid branch id: {error}")))?,
            authority_ref: WorldPromotionAuthorityRef::new(self.authority_ref.clone()).map_err(reference_error)?,
            policy_ref: WorldHeadPolicyRef::new(self.policy_ref.clone())
                .map_err(|error| Failure::invalid_harness(format!("invalid policy ref: {error:?}")))?,
            generation: self.generation,
            admitted: self.admitted,
            target: self.target(action)?,
            nonce: WorldPromotionGrantNonce::new(self.nonce.clone()).map_err(reference_error)?,
            expires_at: self.expires_at,
        })
    }

    fn target(&self, action: WorldPromotionGrantAction) -> Result<WorldPromotionGrantTarget> {
        let fields = (self.candidate_head.as_deref(), self.reservation_ref.as_deref(), self.attempt_ref.as_deref());
        let target = match (action, fields) {
            (WorldPromotionGrantAction::Promote, (Some(candidate_head), None, None)) => {
                WorldPromotionGrantTarget::Promote {
                    candidate_head: WorldCommitRef::new(candidate_head.to_string())
                        .map_err(|error| Failure::invalid_harness(format!("invalid candidate head: {error:?}")))?,
                }
            }
            (WorldPromotionGrantAction::DenyReservation, (None, Some(reservation_ref), None)) => {
                WorldPromotionGrantTarget::DenyReservation {
                    reservation_ref: reservation(reservation_ref)?,
                }
            }
            (WorldPromotionGrantAction::PlanRetry, (None, Some(reservation_ref), Some(attempt_ref))) => {
                WorldPromotionGrantTarget::PlanRetry {
                    reservation_ref: reservation(reservation_ref)?,
                    attempt_ref: attempt(attempt_ref)?,
                }
            }
            (WorldPromotionGrantAction::AbandonUncertain, (None, Some(reservation_ref), Some(attempt_ref))) => {
                WorldPromotionGrantTarget::AbandonUncertain {
                    reservation_ref: reservation(reservation_ref)?,
                    attempt_ref: attempt(attempt_ref)?,
                }
            }
            _ => {
                return Err(Failure::invalid_harness("promotion authority target fields do not match its action"));
            }
        };
        Ok(target)
    }
}

fn reservation(value: &str) -> Result<WorldReleaseReservationRef> {
    WorldReleaseReservationRef::new(value.to_string()).map_err(reference_error)
}

fn attempt(value: &str) -> Result<WorldReleaseAttemptRef> {
    WorldReleaseAttemptRef::new(value.to_string()).map_err(reference_error)
}

fn reference_error(error: WorldPromotionReferenceError) -> Failure {
    Failure::invalid_harness(format!("invalid promotion authority reference: {error:?}"))
}

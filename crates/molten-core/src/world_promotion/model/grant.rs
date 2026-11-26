use super::reference::*;
use crate::world_commit::WorldCommitRef;
use crate::world_head::WorldBranchId;
use crate::world_head::WorldHeadPolicyRef;
use crate::world_head::WorldHeadState;

const GRANT_NONCE_HEX_LENGTH: usize = 64;

/// The one purpose a signed promotion grant authorizes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum WorldPromotionGrantAction {
    Promote,
    DenyReservation,
    PlanRetry,
    AbandonUncertain,
}

impl WorldPromotionGrantAction {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Promote => "promote",
            Self::DenyReservation => "deny-reservation",
            Self::PlanRetry => "plan-retry",
            Self::AbandonUncertain => "abandon-uncertain",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "promote" => Some(Self::Promote),
            "deny-reservation" => Some(Self::DenyReservation),
            "plan-retry" => Some(Self::PlanRetry),
            "abandon-uncertain" => Some(Self::AbandonUncertain),
            _ => None,
        }
    }
}

/// The exact object a grant's action applies to. The action is the variant, so a grant cannot
/// name one action with another action's target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorldPromotionGrantTarget {
    Promote {
        candidate_head: WorldCommitRef,
    },
    DenyReservation {
        reservation_ref: WorldReleaseReservationRef,
    },
    PlanRetry {
        reservation_ref: WorldReleaseReservationRef,
        attempt_ref: WorldReleaseAttemptRef,
    },
    AbandonUncertain {
        reservation_ref: WorldReleaseReservationRef,
        attempt_ref: WorldReleaseAttemptRef,
    },
}

impl WorldPromotionGrantTarget {
    pub const fn action(&self) -> WorldPromotionGrantAction {
        match self {
            Self::Promote { .. } => WorldPromotionGrantAction::Promote,
            Self::DenyReservation { .. } => WorldPromotionGrantAction::DenyReservation,
            Self::PlanRetry { .. } => WorldPromotionGrantAction::PlanRetry,
            Self::AbandonUncertain { .. } => WorldPromotionGrantAction::AbandonUncertain,
        }
    }
}

/// A single-use grant nonce: 32 bytes spelled as 64 lowercase hex digits.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WorldPromotionGrantNonce(String);

impl WorldPromotionGrantNonce {
    pub fn new(value: impl Into<String>) -> Result<Self, WorldPromotionReferenceError> {
        let value = value.into();
        if value.len() != GRANT_NONCE_HEX_LENGTH {
            return Err(WorldPromotionReferenceError::WrongDigestLength);
        }
        if !value.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)) {
            return Err(WorldPromotionReferenceError::InvalidDigestSpelling);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// One verified promotion or operator grant. Every field is covered by the signature.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldPromotionGrant {
    pub branch_id: WorldBranchId,
    pub authority_ref: WorldPromotionAuthorityRef,
    pub policy_ref: WorldHeadPolicyRef,
    pub generation: u64,
    pub admitted: bool,
    pub target: WorldPromotionGrantTarget,
    pub nonce: WorldPromotionGrantNonce,
    /// Exclusive expiry in Unix seconds: the grant is denied once the observed time reaches it.
    pub expires_at: u64,
}

/// Facts the shell reads inside the write transaction that applies a grant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldPromotionLiveFacts {
    pub head: Option<WorldHeadState>,
    pub grant_nonce_consumed: bool,
}

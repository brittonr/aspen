use serde::Deserialize;
use serde::Serialize;

use super::policy::DeliveryCurrentness;
use super::state::DeliveryToken;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case", tag = "kind")]
pub enum DeliveryOperation {
    Enqueue {
        item_ref: String,
        content_ref: String,
        metadata_ref: String,
        metadata_bytes: u32,
    },
    Claim,
    Acknowledge {
        token: DeliveryToken,
    },
    NegativeAcknowledge {
        token: DeliveryToken,
        failure_class: String,
    },
    ExtendLease {
        token: DeliveryToken,
    },
    ExpireLease {
        token: DeliveryToken,
    },
    Redrive {
        item_ref: String,
    },
    CleanupDeadLetter {
        through_tick: u64,
    },
}

impl DeliveryOperation {
    pub const fn kind(&self) -> &'static str {
        match self {
            Self::Enqueue { .. } => "enqueue",
            Self::Claim => "claim",
            Self::Acknowledge { .. } => "acknowledge",
            Self::NegativeAcknowledge { .. } => "negative-acknowledge",
            Self::ExtendLease { .. } => "extend-lease",
            Self::ExpireLease { .. } => "expire-lease",
            Self::Redrive { .. } => "redrive",
            Self::CleanupDeadLetter { .. } => "cleanup-dead-letter",
        }
    }

    pub fn item_ref(&self) -> Option<&str> {
        match self {
            Self::Enqueue { item_ref, .. } | Self::Redrive { item_ref } => Some(item_ref),
            Self::Acknowledge { token }
            | Self::ExtendLease { token }
            | Self::ExpireLease { token }
            | Self::NegativeAcknowledge { token, .. } => Some(&token.item_ref),
            Self::Claim | Self::CleanupDeadLetter { .. } => None,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DeliveryRequest {
    pub schema: String,
    pub queue_id: String,
    pub operation_id: String,
    pub actor_id: String,
    pub service_generation: u64,
    pub consistency_epoch: u64,
    pub engine_epoch: u64,
    pub time_profile_ref: String,
    pub logical_tick: u64,
    pub currentness: DeliveryCurrentness,
    pub authority_refs: Vec<String>,
    pub policy_refs: Vec<String>,
    pub resource_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub operation: DeliveryOperation,
}

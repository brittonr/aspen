use std::collections::BTreeMap;

use serde::Deserialize;
use serde::Serialize;

use super::constants::*;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DeliveryItem {
    pub item_ref: String,
    pub content_ref: String,
    pub metadata_ref: String,
    pub metadata_bytes: u32,
    pub enqueue_sequence: u64,
    pub policy_ref: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReadyDelivery {
    pub item: DeliveryItem,
    pub eligible_at_tick: u64,
    pub cycle: u32,
    pub attempts_in_cycle: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DeliveryToken {
    pub token_ref: String,
    pub delivery_id: String,
    pub queue_id: String,
    pub item_ref: String,
    pub consumer_id: String,
    pub attempt: u64,
    pub cycle: u32,
    pub fencing_token: u64,
    pub claimed_at_tick: u64,
    pub visibility_deadline_tick: u64,
    pub consistency_epoch: u64,
    pub service_generation: u64,
    pub policy_ref: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ActiveDelivery {
    pub item: DeliveryItem,
    pub token: DeliveryToken,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DeliveryAttempt {
    pub delivery_id: String,
    pub item_ref: String,
    pub consumer_id: String,
    pub attempt: u64,
    pub cycle: u32,
    pub outcome: String,
    pub operation_id: String,
    pub observed_at_tick: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DeadLetterDelivery {
    pub item: DeliveryItem,
    pub entered_at_tick: u64,
    pub cycle: u32,
    pub attempts_in_cycle: u64,
    pub total_attempts: u64,
    pub reason: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CompletedDelivery {
    pub item: DeliveryItem,
    pub delivery_id: String,
    pub acknowledged_at_tick: u64,
    pub total_attempts: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AppliedDeliveryOperation {
    pub request_ref: String,
    pub operation_ref: String,
    pub operation_kind: String,
    pub item_ref: Option<String>,
    pub token_ref: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DeliveryState {
    pub schema: String,
    pub queue_id: String,
    pub policy_ref: String,
    pub service_generation: u64,
    pub consistency_epoch: u64,
    pub revision: u64,
    pub next_sequence: u64,
    pub next_fencing_token: u64,
    pub ready: BTreeMap<String, ReadyDelivery>,
    pub in_flight: BTreeMap<String, ActiveDelivery>,
    pub dead_letter: BTreeMap<String, DeadLetterDelivery>,
    pub completed: BTreeMap<String, CompletedDelivery>,
    pub attempts: BTreeMap<String, Vec<DeliveryAttempt>>,
    pub operations: BTreeMap<String, AppliedDeliveryOperation>,
}

impl DeliveryState {
    pub fn empty(
        queue_id: impl Into<String>,
        policy_ref: impl Into<String>,
        service_generation: u64,
        consistency_epoch: u64,
    ) -> Self {
        Self {
            schema: DELIVERY_STATE_SCHEMA.to_string(),
            queue_id: queue_id.into(),
            policy_ref: policy_ref.into(),
            service_generation,
            consistency_epoch,
            revision: INITIAL_DELIVERY_REVISION,
            next_sequence: INITIAL_DELIVERY_SEQUENCE,
            next_fencing_token: INITIAL_DELIVERY_FENCING_TOKEN,
            ready: BTreeMap::new(),
            in_flight: BTreeMap::new(),
            dead_letter: BTreeMap::new(),
            completed: BTreeMap::new(),
            attempts: BTreeMap::new(),
            operations: BTreeMap::new(),
        }
    }
}

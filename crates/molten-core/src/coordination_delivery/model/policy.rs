use std::collections::BTreeMap;
use std::collections::BTreeSet;

use serde::Deserialize;
use serde::Serialize;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum DeliveryOrdering {
    StrictFifo,
    RetryInterleaving,
}

impl DeliveryOrdering {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::StrictFifo => "strict-fifo",
            Self::RetryInterleaving => "retry-interleaving",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum DeliveryBackoff {
    Fixed,
    Exponential,
}

impl DeliveryBackoff {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Fixed => "fixed",
            Self::Exponential => "exponential",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum PoisonItemHandling {
    DeadLetter,
    RetainInFlight,
}

impl PoisonItemHandling {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::DeadLetter => "dead-letter",
            Self::RetainInFlight => "retain-in-flight",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DeliveryPolicy {
    pub schema: String,
    pub policy_id: String,
    pub visibility_timeout_ticks: u64,
    pub maximum_attempts: u64,
    pub retry_base_delay_ticks: u64,
    pub retry_maximum_delay_ticks: u64,
    pub retry_backoff: DeliveryBackoff,
    pub ordering: DeliveryOrdering,
    pub dead_letter_queue_id: String,
    pub dead_letter_retention_ticks: u64,
    pub ready_capacity: u32,
    pub in_flight_capacity: u32,
    pub retry_capacity: u32,
    pub dead_letter_capacity: u32,
    pub metadata_byte_limit: u32,
    pub status_item_limit: u32,
    pub completion_authority_ref: String,
    pub expiry_authority_ref: String,
    pub redrive_authority_ref: String,
    pub retention_authority_ref: String,
    pub retryable_failure_classes: BTreeSet<String>,
    pub poison_failure_classes: BTreeSet<String>,
    pub poison_item_handling: PoisonItemHandling,
    pub non_claims: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DeliveryManifest {
    pub schema: String,
    pub extension_id: String,
    pub service_id: String,
    pub service_generation: u64,
    pub implementation_ref: String,
    pub time_profile_ref: String,
    pub policy_ref: String,
    pub port_bindings: BTreeMap<String, String>,
    pub non_claims: Vec<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum DeliveryCurrentness {
    Linearizable,
    EquivalentFenced,
    LocalStale,
    Unknown,
}

impl DeliveryCurrentness {
    pub const fn is_current(self) -> bool {
        matches!(self, Self::Linearizable | Self::EquivalentFenced)
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Linearizable => "linearizable",
            Self::EquivalentFenced => "equivalent-fenced",
            Self::LocalStale => "local-stale",
            Self::Unknown => "unknown",
        }
    }
}

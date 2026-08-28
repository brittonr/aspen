use serde::Deserialize;
use serde::Serialize;

use super::issue::DeliveryIssue;
use super::state::DeliveryState;
use super::state::DeliveryToken;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum DeliveryDecisionKind {
    Applied,
    DuplicateReplay,
    Denied,
}

impl DeliveryDecisionKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Applied => "applied",
            Self::DuplicateReplay => "duplicate-replay",
            Self::Denied => "denied",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum DeliveryTransitionKind {
    Enqueued,
    Claimed,
    Acknowledged,
    RetryScheduled,
    DeadLettered,
    LeaseExtended,
    Redriven,
    DeadLetterCleaned,
    DuplicateReplay,
    DeniedPreserve,
}

impl DeliveryTransitionKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Enqueued => "enqueued",
            Self::Claimed => "claimed",
            Self::Acknowledged => "acknowledged",
            Self::RetryScheduled => "retry-scheduled",
            Self::DeadLettered => "dead-lettered",
            Self::LeaseExtended => "lease-extended",
            Self::Redriven => "redriven",
            Self::DeadLetterCleaned => "dead-letter-cleaned",
            Self::DuplicateReplay => "duplicate-replay",
            Self::DeniedPreserve => "denied-preserve",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum DeliveryTimerIntentKind {
    ScheduleLeaseExpiry,
    CancelLeaseExpiry,
    ScheduleRetryEligibility,
    ScheduleDeadLetterRetention,
    CancelDeadLetterRetention,
}

impl DeliveryTimerIntentKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ScheduleLeaseExpiry => "schedule-lease-expiry",
            Self::CancelLeaseExpiry => "cancel-lease-expiry",
            Self::ScheduleRetryEligibility => "schedule-retry-eligibility",
            Self::ScheduleDeadLetterRetention => "schedule-dead-letter-retention",
            Self::CancelDeadLetterRetention => "cancel-dead-letter-retention",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DeliveryTimerIntent {
    pub kind: DeliveryTimerIntentKind,
    pub timer_id: String,
    pub item_ref: String,
    pub delivery_id: Option<String>,
    pub deadline_tick: u64,
    pub service_generation: u64,
    pub consistency_epoch: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DeliveryTransition {
    pub schema: String,
    pub decision: DeliveryDecisionKind,
    pub kind: DeliveryTransitionKind,
    pub request_ref: String,
    pub operation_ref: String,
    pub before_state_ref: String,
    pub after_state_ref: String,
    pub next_state: DeliveryState,
    pub token: Option<DeliveryToken>,
    pub timer_intents: Vec<DeliveryTimerIntent>,
    pub issue: Option<DeliveryIssue>,
    pub prior_operation_ref: Option<String>,
    pub worker_dispatch_authorized: bool,
    pub external_effect_exactly_once: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DeliveryWorkerAdmission {
    pub content_verified: bool,
    pub provenance_current: bool,
    pub authority_current: bool,
    pub policy_current: bool,
    pub resource_admitted: bool,
    pub execution_admitted: bool,
    pub evidence_refs: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DeliveryWorkerPlan {
    pub schema: String,
    pub admitted: bool,
    pub delivery_id: String,
    pub item_ref: String,
    pub content_ref: String,
    pub issue: Option<DeliveryIssue>,
    pub external_effect_authorized: bool,
    pub exact_once_claimed: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ActiveDeliveryStatus {
    pub item_ref: String,
    pub delivery_id: String,
    pub consumer_id: String,
    pub attempt: u64,
    pub visibility_deadline_tick: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DeliveryStatus {
    pub schema: String,
    pub queue_id: String,
    pub state_ref: String,
    pub revision: u64,
    pub policy_ref: String,
    pub maximum_attempts: u64,
    pub ready_count: u32,
    pub retry_count: u32,
    pub in_flight_count: u32,
    pub dead_letter_count: u32,
    pub completed_count: u32,
    pub failed_attempt_count: u32,
    pub active_claims: Vec<ActiveDeliveryStatus>,
    pub resource_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub truncated: bool,
    pub payloads_rendered: bool,
}

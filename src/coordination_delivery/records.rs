use molten_core::coordination_delivery::*;
use preserves::IOValue;

use crate::error::MoltenError;
use crate::error::Result;

pub const DELIVERY_COMMIT_RECEIPT_SCHEMA: &str = "molten.coordination-delivery-commit-receipt.v1";
pub const DELIVERY_COMMIT_RECEIPT_RECORD: &str = "molten-coordination-delivery-commit-receipt-v1";
pub const DELIVERY_STATUS_RECORD: &str = "molten-coordination-delivery-status-v1";

const DELIVERY_RECEIPT_DOMAIN: &str = "onixresearch.molten.coordination-delivery-commit-receipt.v1";
const DELIVERY_STATUS_DOMAIN: &str = "onixresearch.molten.coordination-delivery-status.v1";
const MAX_DELIVERY_RECEIPT_BYTES: usize = 262_144;
const MAX_DELIVERY_STATUS_BYTES: usize = 65_536;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DeliveryServiceStatus {
    Denied,
    DuplicateReplay,
    Applied,
    AlreadyApplied,
    AppliedAfterReconciliation,
    NotAppliedAfterReconciliation,
    Stale,
    Unknown,
}

impl DeliveryServiceStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Denied => "denied",
            Self::DuplicateReplay => "duplicate-replay",
            Self::Applied => "applied",
            Self::AlreadyApplied => "already-applied",
            Self::AppliedAfterReconciliation => "applied-after-reconciliation",
            Self::NotAppliedAfterReconciliation => "not-applied-after-reconciliation",
            Self::Stale => "stale",
            Self::Unknown => "unknown",
        }
    }

    pub const fn commit_confirmed(self) -> bool {
        matches!(self, Self::Applied | Self::AlreadyApplied | Self::AppliedAfterReconciliation)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeliveryCommitReceipt {
    pub queue_id: String,
    pub request_ref: String,
    pub operation_ref: String,
    pub before_state_ref: String,
    pub after_state_ref: String,
    pub revision: u64,
    pub status: DeliveryServiceStatus,
    pub currentness: DeliveryCurrentness,
    pub durability: super::DeliveryDurabilityOutcome,
    pub engine_epoch: u64,
    pub timer_refs: Vec<String>,
    pub failed_timer_refs: Vec<String>,
    pub status_ref: Option<String>,
    pub issue: Option<DeliveryIssue>,
    pub authorizes_future_mutation: bool,
    pub authorizes_worker_effects: bool,
    pub claims_exactly_once: bool,
    pub non_claims: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct CanonicalDeliveryCommitReceipt {
    pub receipt_ref: String,
    pub status: DeliveryServiceStatus,
    pub value: IOValue,
    pub bytes: Vec<u8>,
}

pub fn canonical_delivery_commit_receipt(receipt: &DeliveryCommitReceipt) -> Result<CanonicalDeliveryCommitReceipt> {
    if receipt.queue_id.is_empty()
        || receipt.request_ref.is_empty()
        || receipt.operation_ref.is_empty()
        || receipt.before_state_ref.is_empty()
        || receipt.after_state_ref.is_empty()
        || receipt.authorizes_future_mutation
        || receipt.authorizes_worker_effects
        || receipt.claims_exactly_once
        || receipt.non_claims != required_delivery_non_claims()
    {
        return Err(MoltenError::invalid_harness("coordination delivery receipt is invalid"));
    }
    let value = record(DELIVERY_COMMIT_RECEIPT_RECORD, vec![
        field("schema", string(DELIVERY_COMMIT_RECEIPT_SCHEMA)),
        field("queue-id", string(&receipt.queue_id)),
        field("request-ref", string(&receipt.request_ref)),
        field("operation-ref", string(&receipt.operation_ref)),
        field("before-state-ref", string(&receipt.before_state_ref)),
        field("after-state-ref", string(&receipt.after_state_ref)),
        field("revision", number(receipt.revision)),
        field("status", string(receipt.status.as_str())),
        field("currentness", string(receipt.currentness.as_str())),
        field("durability", string(receipt.durability.as_str())),
        field("engine-epoch", number(receipt.engine_epoch)),
        field("timer-refs", sequence(receipt.timer_refs.iter().map(string).collect())),
        field("failed-timer-refs", sequence(receipt.failed_timer_refs.iter().map(string).collect())),
        field("status-ref", optional_text(receipt.status_ref.as_deref())),
        field("issue", optional_text(receipt.issue.as_ref().map(DeliveryIssue::code))),
        field("authorizes-future-mutation", boolean(receipt.authorizes_future_mutation)),
        field("authorizes-worker-effects", boolean(receipt.authorizes_worker_effects)),
        field("claims-exactly-once", boolean(receipt.claims_exactly_once)),
        field("non-claims", sequence(receipt.non_claims.iter().map(string).collect())),
    ]);
    let bytes = crate::preserves_rail::canonical_bytes(&value)?;
    if bytes.len() > MAX_DELIVERY_RECEIPT_BYTES {
        return Err(MoltenError::invalid_harness("coordination delivery receipt exceeds its byte bound"));
    }
    Ok(CanonicalDeliveryCommitReceipt {
        receipt_ref: hash_bytes(DELIVERY_RECEIPT_DOMAIN, &bytes),
        status: receipt.status,
        value,
        bytes,
    })
}

pub fn identify_canonical_delivery_status(status: &DeliveryStatus) -> Result<String> {
    let active = status
        .active_claims
        .iter()
        .map(|claim| {
            record("active-claim", vec![
                string(&claim.item_ref),
                string(&claim.delivery_id),
                string(&claim.consumer_id),
                number(claim.attempt),
                number(claim.visibility_deadline_tick),
            ])
        })
        .collect();
    let value = record(DELIVERY_STATUS_RECORD, vec![
        field("schema", string(&status.schema)),
        field("queue-id", string(&status.queue_id)),
        field("state-ref", string(&status.state_ref)),
        field("revision", number(status.revision)),
        field("policy-ref", string(&status.policy_ref)),
        field("maximum-attempts", number(status.maximum_attempts)),
        field("ready-count", number(u64::from(status.ready_count))),
        field("retry-count", number(u64::from(status.retry_count))),
        field("in-flight-count", number(u64::from(status.in_flight_count))),
        field("dead-letter-count", number(u64::from(status.dead_letter_count))),
        field("completed-count", number(u64::from(status.completed_count))),
        field("failed-attempt-count", number(u64::from(status.failed_attempt_count))),
        field("active-claims", sequence(active)),
        field("resource-refs", sequence(status.resource_refs.iter().map(string).collect())),
        field("evidence-refs", sequence(status.evidence_refs.iter().map(string).collect())),
        field("truncated", boolean(status.truncated)),
        field("payloads-rendered", boolean(status.payloads_rendered)),
    ]);
    let bytes = crate::preserves_rail::canonical_bytes(&value)?;
    if bytes.len() > MAX_DELIVERY_STATUS_BYTES || status.payloads_rendered {
        return Err(MoltenError::invalid_harness("coordination delivery status is invalid or over bound"));
    }
    Ok(hash_bytes(DELIVERY_STATUS_DOMAIN, &bytes))
}

fn hash_bytes(domain: &'static str, bytes: &[u8]) -> String {
    let mut hasher = blake3::Hasher::new_derive_key(domain);
    hasher.update(bytes);
    format!("blake3:{}", hasher.finalize().to_hex())
}

fn optional_text(value: Option<&str>) -> IOValue {
    value.map_or_else(|| record("none", Vec::new()), |value| record("some", vec![string(value)]))
}

fn boolean(value: bool) -> IOValue {
    record(if value { "true" } else { "false" }, Vec::new())
}

fn number(value: u64) -> IOValue {
    crate::preserves_rail::u64_value(value)
}

fn string(value: impl AsRef<str>) -> IOValue {
    crate::preserves_rail::string(value.as_ref())
}

fn sequence(values: Vec<IOValue>) -> IOValue {
    crate::preserves_rail::sequence(values)
}

fn field(label: &'static str, value: IOValue) -> IOValue {
    record(label, vec![value])
}

fn record(label: &'static str, fields: Vec<IOValue>) -> IOValue {
    crate::preserves_rail::record(label, fields)
}

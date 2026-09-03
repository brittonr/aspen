use molten_core::addressable_actor::*;
use preserves::IOValue;

use crate::error::MoltenError;
use crate::error::Result;

pub const ACTOR_COMMIT_RECEIPT_SCHEMA: &str = "molten.addressable-actor.commit-receipt.v1";
pub const ACTOR_COMMIT_RECEIPT_RECORD: &str = "molten-addressable-actor-commit-receipt-v1";
pub const ACTOR_STATUS_RECORD: &str = "molten-addressable-actor-status-v1";

const ACTOR_RECEIPT_DOMAIN: &str = "onixresearch.molten.addressable-actor-commit-receipt.v1";
const ACTOR_STATUS_DOMAIN: &str = "onixresearch.molten.addressable-actor-status.v1";
const MAX_ACTOR_RECEIPT_BYTES: usize = 262_144;
const MAX_ACTOR_STATUS_BYTES: usize = 65_536;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActorServiceStatus {
    Denied,
    DuplicateReplay,
    Applied,
    AlreadyApplied,
    AppliedAfterReconciliation,
    NotAppliedAfterReconciliation,
    Stale,
    EffectAdmissionDenied,
    EffectFailed,
    EffectOutcomeUnknown,
    Unknown,
}

impl ActorServiceStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Denied => "denied",
            Self::DuplicateReplay => "duplicate-replay",
            Self::Applied => "applied",
            Self::AlreadyApplied => "already-applied",
            Self::AppliedAfterReconciliation => "applied-after-reconciliation",
            Self::NotAppliedAfterReconciliation => "not-applied-after-reconciliation",
            Self::Stale => "stale",
            Self::EffectAdmissionDenied => "effect-admission-denied",
            Self::EffectFailed => "effect-failed",
            Self::EffectOutcomeUnknown => "effect-outcome-unknown",
            Self::Unknown => "unknown",
        }
    }

    pub const fn commit_confirmed(self) -> bool {
        matches!(self, Self::Applied | Self::AlreadyApplied | Self::AppliedAfterReconciliation)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActorCommitReceipt {
    pub actor_key_ref: String,
    pub request_ref: String,
    pub operation_ref: String,
    pub before_state_ref: String,
    pub planned_state_ref: String,
    pub final_state_ref: String,
    pub revision: u64,
    pub status: ActorServiceStatus,
    pub currentness: super::ActorCommitCurrentness,
    pub durability: super::ActorDurabilityOutcome,
    pub engine_epoch: u64,
    pub effect_observations: Vec<super::ActorEffectObservation>,
    pub status_ref: Option<String>,
    pub issue: Option<ActorIssue>,
    pub authorizes_future_mutation: bool,
    pub authorizes_effects: bool,
    pub authorizes_retry: bool,
    pub claims_exactly_once: bool,
    pub claims_runtime_survival: bool,
    pub non_claims: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct CanonicalActorCommitReceipt {
    pub receipt_ref: String,
    pub status: ActorServiceStatus,
    pub value: IOValue,
    pub bytes: Vec<u8>,
}

pub fn canonical_actor_commit_receipt(receipt: &ActorCommitReceipt) -> Result<CanonicalActorCommitReceipt> {
    if receipt.actor_key_ref.is_empty()
        || receipt.request_ref.is_empty()
        || receipt.operation_ref.is_empty()
        || receipt.before_state_ref.is_empty()
        || receipt.planned_state_ref.is_empty()
        || receipt.final_state_ref.is_empty()
        || receipt.authorizes_future_mutation
        || receipt.authorizes_effects
        || receipt.authorizes_retry
        || receipt.claims_exactly_once
        || receipt.claims_runtime_survival
        || receipt.non_claims != required_addressable_actor_non_claims()
    {
        return Err(MoltenError::invalid_harness("addressable actor receipt is invalid"));
    }
    let effect_observations = receipt
        .effect_observations
        .iter()
        .map(|observation| {
            record("effect-observation", vec![
                string(&observation.effect_ref),
                string(&observation.admission_ref),
                string(observation.disposition.as_str()),
                optional_text(observation.outcome_ref.as_deref()),
            ])
        })
        .collect();
    let value = record(ACTOR_COMMIT_RECEIPT_RECORD, vec![
        field("schema", string(ACTOR_COMMIT_RECEIPT_SCHEMA)),
        field("actor-key-ref", string(&receipt.actor_key_ref)),
        field("request-ref", string(&receipt.request_ref)),
        field("operation-ref", string(&receipt.operation_ref)),
        field("before-state-ref", string(&receipt.before_state_ref)),
        field("planned-state-ref", string(&receipt.planned_state_ref)),
        field("final-state-ref", string(&receipt.final_state_ref)),
        field("revision", number(receipt.revision)),
        field("status", string(receipt.status.as_str())),
        field("currentness", string(receipt.currentness.as_str())),
        field("durability", string(receipt.durability.as_str())),
        field("engine-epoch", number(receipt.engine_epoch)),
        field("effect-observations", sequence(effect_observations)),
        field("status-ref", optional_text(receipt.status_ref.as_deref())),
        field("issue", optional_text(receipt.issue.as_ref().map(ActorIssue::code))),
        field("authorizes-future-mutation", boolean(receipt.authorizes_future_mutation)),
        field("authorizes-effects", boolean(receipt.authorizes_effects)),
        field("authorizes-retry", boolean(receipt.authorizes_retry)),
        field("claims-exactly-once", boolean(receipt.claims_exactly_once)),
        field("claims-runtime-survival", boolean(receipt.claims_runtime_survival)),
        field("non-claims", sequence(receipt.non_claims.iter().map(string).collect())),
    ]);
    let bytes = crate::preserves_rail::canonical_bytes(&value)?;
    if bytes.len() > MAX_ACTOR_RECEIPT_BYTES {
        return Err(MoltenError::invalid_harness("addressable actor receipt exceeds its byte bound"));
    }
    Ok(CanonicalActorCommitReceipt {
        receipt_ref: hash_bytes(ACTOR_RECEIPT_DOMAIN, &bytes),
        status: receipt.status,
        value,
        bytes,
    })
}

pub fn identify_canonical_actor_status(status: &ActorStatus) -> Result<String> {
    if status.payloads_rendered || status.authorizes_mutation {
        return Err(MoltenError::invalid_harness("addressable actor status exceeds its authority boundary"));
    }
    let value = record(ACTOR_STATUS_RECORD, vec![
        field("schema", string(&status.schema)),
        field("actor-key-ref", string(&status.actor_key_ref)),
        field("profile-ref", string(&status.profile_ref)),
        field("system-extension-manifest-ref", string(&status.system_extension_manifest_ref)),
        field("placement-ref", string(&status.placement_ref)),
        field("extension-generation", number(status.extension_generation)),
        field("lifecycle-sequence", number(status.lifecycle_sequence)),
        field("revision", number(status.revision)),
        field("phase", string(status.phase.as_str())),
        field("checkpoint-ref", optional_text(status.checkpoint_ref.as_deref())),
        field("durable-state-ref", optional_text(status.durable_state_ref.as_deref())),
        field("active-wake-ref", optional_text(status.active_wake_ref.as_deref())),
        field("unknown-effect-ref", optional_text(status.unknown_effect_ref.as_deref())),
        field("mailbox-revision", number(status.mailbox_revision)),
        field("last-activity-tick", number(status.last_activity_tick)),
        field("completed-event-refs", sequence(status.completed_event_refs.iter().map(string).collect())),
        field("evidence-refs", sequence(status.evidence_refs.iter().map(string).collect())),
        field("truncated", boolean(status.truncated)),
        field("payloads-rendered", boolean(status.payloads_rendered)),
        field("authorizes-mutation", boolean(status.authorizes_mutation)),
    ]);
    let bytes = crate::preserves_rail::canonical_bytes(&value)?;
    if bytes.len() > MAX_ACTOR_STATUS_BYTES {
        return Err(MoltenError::invalid_harness("addressable actor status exceeds its byte bound"));
    }
    Ok(hash_bytes(ACTOR_STATUS_DOMAIN, &bytes))
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

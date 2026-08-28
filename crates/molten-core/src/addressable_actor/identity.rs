#![allow(
    tigerstyle::non_trait_imports,
    reason = "the actor identity projection uses one private framing owner throughout"
)]

mod framing;

use framing::FramedHasher;

use super::*;

const KEY_IDENTITY_DOMAIN: &str = "onixresearch.molten.addressable-actor-key.v1";
const PROFILE_IDENTITY_DOMAIN: &str = "onixresearch.molten.addressable-actor-profile.v1";
const STATE_IDENTITY_DOMAIN: &str = "onixresearch.molten.addressable-actor-state.v1";
const REQUEST_IDENTITY_DOMAIN: &str = "onixresearch.molten.addressable-actor-request.v1";
const OPERATION_IDENTITY_DOMAIN: &str = "onixresearch.molten.addressable-actor-operation.v1";
const EFFECT_IDENTITY_DOMAIN: &str = "onixresearch.molten.addressable-actor-effect.v1";
const WAKE_IDENTITY_DOMAIN: &str = "onixresearch.molten.addressable-actor-wake.v1";

#[must_use]
pub fn identify_actor_key(key: &ActorKey) -> String {
    let mut framed = FramedHasher::new(KEY_IDENTITY_DOMAIN);
    framed.text("schema", &key.schema);
    framed.text("namespace-ref", &key.namespace_ref);
    framed.text("actor-type", &key.actor_type);
    framed.text("key", &key.key);
    framed.finish()
}

#[must_use]
pub fn identify_addressable_actor_profile(profile: &AddressableActorProfile) -> String {
    let mut framed = FramedHasher::new(PROFILE_IDENTITY_DOMAIN);
    framed.text("schema", &profile.schema);
    framed.text("profile-id", &profile.profile_id);
    framed.number("profile-version", u64::from(profile.profile_version));
    framed.text("reference-repository", &profile.reference_source.repository);
    framed.text("reference-revision", &profile.reference_source.revision);
    framed.text("reference-license", &profile.reference_source.license);
    for concept in &profile.reference_source.selected_concepts {
        framed.text("selected-concept", concept);
    }
    framed.text("system-extension-profile-ref", &profile.system_extension_profile_ref);
    framed.text("placement-profile-ref", &profile.placement_profile_ref);
    framed.text("delivery-profile-ref", &profile.delivery_profile_ref);
    framed.text("durable-state-profile-ref", &profile.durable_state_profile_ref);
    framed.text("time-profile-ref", &profile.time_profile_ref);
    framed.text("resource-profile-ref", &profile.resource_profile_ref);
    framed.text("supervision-profile-ref", &profile.supervision_profile_ref);
    framed.text("authority-profile-ref", &profile.authority_profile_ref);
    framed.text("evidence-profile-ref", &profile.evidence_profile_ref);
    framed.number("idle-after-ticks", profile.idle_after_ticks);
    framed.number("maximum-drain-items", u64::from(profile.maximum_drain_items));
    hash_survival_matrix(&mut framed, &profile.survival);
    for non_claim in &profile.non_claims {
        framed.text("non-claim", non_claim);
    }
    framed.finish()
}

#[must_use]
pub fn identify_actor_state(state: &ActorState) -> String {
    let mut framed = FramedHasher::new(STATE_IDENTITY_DOMAIN);
    framed.text("schema", &state.schema);
    framed.text("actor-key-ref", &state.actor_key_ref);
    framed.text("profile-ref", &state.profile_ref);
    framed.text("system-extension-manifest-ref", &state.system_extension_manifest_ref);
    framed.text("placement-ref", &state.placement_ref);
    framed.number("extension-generation", state.extension_generation);
    framed.number("lifecycle-sequence", state.lifecycle_sequence);
    framed.number("revision", state.revision);
    framed.text("phase", state.phase.as_str());
    framed.optional_text("checkpoint-ref", state.checkpoint_ref.as_deref());
    framed.optional_text("durable-state-ref", state.durable_state_ref.as_deref());
    framed.optional_text("active-wake-ref", state.active_wake_ref.as_deref());
    framed.optional_text("unknown-effect-ref", state.unknown_effect_ref.as_deref());
    framed.number("mailbox-revision", state.mailbox_revision);
    framed.number("last-activity-tick", state.last_activity_tick);
    for reference in &state.completed_event_refs {
        framed.text("completed-event-ref", reference);
    }
    for (operation_id, operation) in &state.applied_operations {
        framed.text("operation-id", operation_id);
        framed.text("operation-request-ref", &operation.request_ref);
        framed.text("operation-ref", &operation.operation_ref);
        framed.text("operation-kind", &operation.operation_kind);
    }
    framed.finish()
}

#[must_use]
pub fn identify_actor_request(request: &ActorRequest) -> String {
    let mut framed = FramedHasher::new(REQUEST_IDENTITY_DOMAIN);
    framed.text("schema", &request.schema);
    framed.text("operation-id", &request.operation_id);
    framed.text("actor-key-ref", &request.actor_key_ref);
    framed.text("placement-ref", &request.placement_ref);
    framed.number("extension-generation", request.extension_generation);
    framed.number("expected-lifecycle-sequence", request.expected_lifecycle_sequence);
    framed.number("logical-tick", request.logical_tick);
    hash_admission(&mut framed, &request.admission);
    hash_operation(&mut framed, &request.operation);
    framed.finish()
}

#[must_use]
pub fn identify_actor_wake(reason: &WakeReason) -> String {
    let mut framed = FramedHasher::new(WAKE_IDENTITY_DOMAIN);
    hash_wake(&mut framed, reason);
    framed.finish()
}

#[must_use]
pub fn identify_actor_operation(request_ref: &str, operation: &ActorOperation) -> String {
    let mut framed = FramedHasher::new(OPERATION_IDENTITY_DOMAIN);
    framed.text("request-ref", request_ref);
    hash_operation(&mut framed, operation);
    framed.finish()
}

#[derive(Clone, Copy, Debug)]
pub struct ActorEffectIdentityInput<'a> {
    pub request_ref: &'a str,
    pub ordinal: u64,
    pub kind: ActorEffectIntentKind,
    pub actor_key_ref: &'a str,
    pub profile_ref: &'a str,
    pub system_extension_manifest_ref: &'a str,
    pub placement_ref: &'a str,
    pub extension_generation: u64,
    pub lifecycle_sequence: u64,
    pub wake_ref: Option<&'a str>,
    pub subject_ref: Option<&'a str>,
}

#[must_use]
pub fn identify_actor_effect(input: ActorEffectIdentityInput<'_>) -> String {
    let mut framed = FramedHasher::new(EFFECT_IDENTITY_DOMAIN);
    framed.text("request-ref", input.request_ref);
    framed.number("ordinal", input.ordinal);
    framed.text("kind", input.kind.as_str());
    framed.text("actor-key-ref", input.actor_key_ref);
    framed.text("profile-ref", input.profile_ref);
    framed.text("system-extension-manifest-ref", input.system_extension_manifest_ref);
    framed.text("placement-ref", input.placement_ref);
    framed.number("extension-generation", input.extension_generation);
    framed.number("lifecycle-sequence", input.lifecycle_sequence);
    framed.optional_text("wake-ref", input.wake_ref);
    framed.optional_text("subject-ref", input.subject_ref);
    framed.finish()
}

fn hash_survival_matrix(framed: &mut FramedHasher, matrix: &ActorSurvivalMatrix) {
    framed.text("survival-schema", &matrix.schema);
    framed.number("survival-version", u64::from(matrix.profile_version));
    for rule in &matrix.rules {
        framed.text("survival-class", rule.class.as_str());
        framed.text("survival-disposition", rule.disposition.as_str());
    }
}

fn hash_admission(framed: &mut FramedHasher, admission: &ActorAdmissionFacts) {
    framed.text("admission-profile-ref", &admission.profile_ref);
    framed.text("admission-system-extension-manifest-ref", &admission.system_extension_manifest_ref);
    framed.text("authority-ref", &admission.authority_ref);
    framed.text("resource-ref", &admission.resource_ref);
    framed.text("adapter-ref", &admission.adapter_ref);
    framed.boolean("policy-current", admission.policy_current);
    framed.boolean("capability-current", admission.capability_current);
    framed.boolean("placement-current", admission.placement_current);
    framed.boolean("generation-current", admission.generation_current);
    framed.boolean("resources-admitted", admission.resources_admitted);
    framed.boolean("adapter-admitted", admission.adapter_admitted);
}

fn hash_operation(framed: &mut FramedHasher, operation: &ActorOperation) {
    framed.text("operation-kind", operation.kind());
    match operation {
        ActorOperation::Wake { reason } => hash_wake(framed, reason),
        ActorOperation::StartSucceeded { wake_ref } => framed.text("wake-ref", wake_ref),
        ActorOperation::IdleSleep {
            checkpoint_ref,
            pending_mailbox_items,
            unresolved_effects,
        } => {
            framed.text("checkpoint-ref", checkpoint_ref);
            framed.number("pending-mailbox-items", u64::from(*pending_mailbox_items));
            framed.number("unresolved-effects", u64::from(*unresolved_effects));
        }
        ActorOperation::BeginDrain | ActorOperation::Stop => {}
        ActorOperation::DrainSucceeded {
            checkpoint_ref,
            remaining_items,
        } => {
            framed.text("checkpoint-ref", checkpoint_ref);
            framed.number("remaining-items", u64::from(*remaining_items));
        }
        ActorOperation::Degrade { failure_ref } | ActorOperation::RecoveryFailed { failure_ref } => {
            framed.text("failure-ref", failure_ref);
        }
        ActorOperation::BeginRecovery { checkpoint_ref } => framed.text("checkpoint-ref", checkpoint_ref),
        ActorOperation::RecoverySucceeded {
            checkpoint_ref,
            restored_classes,
            durable_state_ref,
        } => {
            framed.text("checkpoint-ref", checkpoint_ref);
            for class in restored_classes {
                framed.text("restored-class", class.as_str());
            }
            framed.text("durable-state-ref", durable_state_ref);
        }
        ActorOperation::CompleteDelivery {
            delivery_item_ref,
            delivery_token_ref,
            semantic_event_ref,
            semantic_commit_ref,
        } => {
            framed.text("delivery-item-ref", delivery_item_ref);
            framed.text("delivery-token-ref", delivery_token_ref);
            framed.text("semantic-event-ref", semantic_event_ref);
            framed.text("semantic-commit-ref", semantic_commit_ref);
        }
        ActorOperation::RecordUnknownEffect { effect_ref } => framed.text("effect-ref", effect_ref),
        ActorOperation::ResolveUnknownEffect {
            effect_ref,
            resolution_ref,
            checkpoint_ref,
        } => {
            framed.text("effect-ref", effect_ref);
            framed.text("resolution-ref", resolution_ref);
            framed.text("checkpoint-ref", checkpoint_ref);
        }
    }
}

fn hash_wake(framed: &mut FramedHasher, reason: &WakeReason) {
    framed.text("wake-kind", reason.kind());
    match reason {
        WakeReason::Message {
            delivery_item_ref,
            delivery_token_ref,
        } => {
            framed.text("delivery-item-ref", delivery_item_ref);
            framed.text("delivery-token-ref", delivery_token_ref);
        }
        WakeReason::Timer { timer_ref } => framed.text("timer-ref", timer_ref),
        WakeReason::Connection { connection_ref } => framed.text("connection-ref", connection_ref),
        WakeReason::Operator { operator_request_ref } => framed.text("operator-request-ref", operator_request_ref),
    }
}

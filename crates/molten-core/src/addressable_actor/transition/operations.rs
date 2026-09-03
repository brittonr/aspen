mod common;
mod delivery;
mod lifecycle;
mod recovery;
mod wake;

use crate::addressable_actor::*;

pub(super) fn plan_operation(
    profile: &AddressableActorProfile,
    state: &ActorState,
    request: &ActorRequest,
) -> Result<super::PlannedChange, ActorIssue> {
    match &request.operation {
        ActorOperation::Wake { reason } => wake::wake(state, request, reason),
        ActorOperation::StartSucceeded { wake_ref } => wake::start_succeeded(state, request, wake_ref),
        ActorOperation::IdleSleep {
            checkpoint_ref,
            pending_mailbox_items,
            unresolved_effects,
        } => wake::idle_sleep(profile, state, request, wake::SleepInput {
            checkpoint_ref,
            pending_mailbox_items: *pending_mailbox_items,
            unresolved_effects: *unresolved_effects,
        }),
        ActorOperation::BeginDrain => lifecycle::begin_drain(state),
        ActorOperation::DrainSucceeded {
            checkpoint_ref,
            remaining_items,
        } => lifecycle::drain_succeeded(profile, state, checkpoint_ref, *remaining_items),
        ActorOperation::Stop => lifecycle::stop(state),
        ActorOperation::Degrade { failure_ref } => lifecycle::degrade(state, failure_ref),
        ActorOperation::BeginRecovery { checkpoint_ref } => recovery::begin_recovery(state, checkpoint_ref),
        ActorOperation::RecoverySucceeded {
            checkpoint_ref,
            restored_classes,
            durable_state_ref,
        } => recovery::recovery_succeeded(profile, state, request, recovery::RecoverySuccessInput {
            checkpoint_ref,
            restored_classes,
            durable_state_ref,
        }),
        ActorOperation::RecoveryFailed { failure_ref } => recovery::recovery_failed(state, failure_ref),
        ActorOperation::CompleteDelivery {
            delivery_item_ref,
            delivery_token_ref,
            semantic_event_ref,
            semantic_commit_ref,
        } => delivery::complete_delivery(state, request, delivery::CompletionInput {
            delivery_item_ref,
            delivery_token_ref,
            semantic_event_ref,
            semantic_commit_ref,
        }),
        ActorOperation::RecordUnknownEffect { effect_ref } => delivery::record_unknown_effect(state, effect_ref),
        ActorOperation::ResolveUnknownEffect {
            effect_ref,
            resolution_ref,
            checkpoint_ref,
        } => delivery::resolve_unknown_effect(state, effect_ref, resolution_ref, checkpoint_ref),
    }
}

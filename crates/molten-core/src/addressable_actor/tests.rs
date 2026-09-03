use super::*;
use crate::system_extension::HealthState;
use crate::system_extension::LifecyclePhase;
use crate::system_extension::LifecycleState;

const IDLE_AFTER_TICKS: u64 = 5;
const MAXIMUM_DRAIN_ITEMS: u32 = 16;
const WAKE_TICK: u64 = 10;
const SLEEP_TICK: u64 = 20;
const STATUS_LIMIT: usize = 1;

fn reference(label: &str) -> String {
    format!("blake3:{}", blake3::hash(label.as_bytes()).to_hex())
}

fn profile() -> AddressableActorProfile {
    AddressableActorProfile {
        schema: ACTOR_PROFILE_SCHEMA.to_string(),
        profile_id: "addressable-actor-v1".to_string(),
        profile_version: ADDRESSABLE_ACTOR_PROFILE_VERSION,
        reference_source: ActorReferenceSource {
            repository: RIVET_ACTORS_REPOSITORY.to_string(),
            revision: RIVET_ACTORS_REVISION.to_string(),
            license: RIVET_ACTORS_LICENSE.to_string(),
            selected_concepts: vec![
                "keyed-addressability".to_string(),
                "generation-fenced-runtime".to_string(),
                "sleep-intent-and-rewake-separation".to_string(),
                "persisted-state-and-scheduled-events".to_string(),
                "runtime-and-durable-survival-separation".to_string(),
            ],
        },
        system_extension_profile_ref: reference("system-extension-profile"),
        placement_profile_ref: reference("placement-profile"),
        delivery_profile_ref: reference("delivery-profile"),
        durable_state_profile_ref: reference("durable-state-profile"),
        time_profile_ref: reference("time-profile"),
        resource_profile_ref: reference("resource-profile"),
        supervision_profile_ref: reference("supervision-profile"),
        authority_profile_ref: reference("authority-profile"),
        evidence_profile_ref: reference("evidence-profile"),
        idle_after_ticks: IDLE_AFTER_TICKS,
        maximum_drain_items: MAXIMUM_DRAIN_ITEMS,
        survival: standard_actor_survival_matrix(),
        non_claims: required_addressable_actor_non_claims(),
    }
}

fn state(profile: &AddressableActorProfile) -> ActorState {
    ActorState::dormant(
        reference("actor-key"),
        identify_addressable_actor_profile(profile),
        reference("system-extension-manifest"),
        reference("placement"),
        ADDRESSABLE_ACTOR_INITIAL_GENERATION,
    )
}

fn admission(state: &ActorState) -> ActorAdmissionFacts {
    ActorAdmissionFacts {
        profile_ref: state.profile_ref.clone(),
        system_extension_manifest_ref: state.system_extension_manifest_ref.clone(),
        authority_ref: reference("authority"),
        resource_ref: reference("resource"),
        adapter_ref: reference("adapter"),
        policy_current: true,
        capability_current: true,
        placement_current: true,
        generation_current: true,
        resources_admitted: true,
        adapter_admitted: true,
    }
}

fn request(state: &ActorState, operation_id: &str, logical_tick: u64, operation: ActorOperation) -> ActorRequest {
    ActorRequest {
        schema: ACTOR_REQUEST_SCHEMA.to_string(),
        operation_id: operation_id.to_string(),
        actor_key_ref: state.actor_key_ref.clone(),
        placement_ref: state.placement_ref.clone(),
        extension_generation: state.extension_generation,
        expected_lifecycle_sequence: state.lifecycle_sequence,
        logical_tick,
        admission: admission(state),
        operation,
    }
}

fn message_wake() -> WakeReason {
    WakeReason::Message {
        delivery_item_ref: reference("delivery-item"),
        delivery_token_ref: reference("delivery-token"),
    }
}

#[test]
fn profile_and_actor_key_admission_is_closed() {
    let profile = profile();
    assert!(validate_addressable_actor_profile(&profile).is_empty());
    let key = ActorKey {
        schema: ACTOR_KEY_SCHEMA.to_string(),
        namespace_ref: reference("namespace"),
        actor_type: "workspace-agent".to_string(),
        key: "tenant:alpha/workspace:one".to_string(),
    };
    assert!(validate_actor_key(&key).is_empty());
    assert!(crate::fabric::valid_blake3_ref(&identify_actor_key(&key)));

    let mut invalid = profile.clone();
    invalid.reference_source.revision = "main".to_string();
    invalid.non_claims.pop();
    let issues = validate_addressable_actor_profile(&invalid);
    assert!(issues.contains(&ActorIssue::ReferenceSourceMismatch));
    assert!(issues.contains(&ActorIssue::MissingNonClaim));

    let mut invalid_key = key;
    invalid_key.key = "bad key".to_string();
    assert!(validate_actor_key(&invalid_key).contains(&ActorIssue::MalformedActorKey));
}

#[test]
fn dormant_message_wake_binds_restore_start_and_delivery() {
    let profile = profile();
    let mut state = state(&profile);
    state.checkpoint_ref = Some(reference("checkpoint"));
    let request = request(&state, "wake-message", WAKE_TICK, ActorOperation::Wake { reason: message_wake() });
    let transition = plan_actor_transition(&profile, &state, &request);
    assert_eq!(transition.decision, ActorDecision::Applied);
    assert_eq!(transition.kind, ActorTransitionKind::WakeStart);
    assert_eq!(transition.next_state.phase, ActorPhase::Starting);
    assert_eq!(transition.effects.len(), 3);
    assert_eq!(transition.effects[0].kind, ActorEffectIntentKind::RestoreCheckpoint);
    assert_eq!(transition.effects[1].kind, ActorEffectIntentKind::StartRuntime);
    assert_eq!(transition.effects[2].kind, ActorEffectIntentKind::DeliverMessage);
    assert!(transition.effects.iter().all(|effect| {
        effect.actor_key_ref == state.actor_key_ref
            && effect.placement_ref == state.placement_ref
            && effect.extension_generation == state.extension_generation
            && effect.requires_fresh_admission
    }));

    let mut stale = request;
    stale.actor_key_ref = reference("other-actor");
    let denied = plan_actor_transition(&profile, &state, &stale);
    assert_eq!(denied.decision, ActorDecision::Denied);
    assert_eq!(denied.issue, Some(ActorIssue::StaleActorKey));
    assert!(denied.effects.is_empty());
    assert_eq!(denied.next_state, state);
}

#[test]
fn start_and_duplicate_wake_paths_are_fenced() {
    let profile = profile();
    let initial = state(&profile);
    let wake = request(&initial, "wake-one", WAKE_TICK, ActorOperation::Wake { reason: message_wake() });
    let waking = plan_actor_transition(&profile, &initial, &wake).next_state;
    let duplicate = plan_actor_transition(
        &profile,
        &waking,
        &request(&waking, "wake-two", WAKE_TICK, ActorOperation::Wake { reason: message_wake() }),
    );
    assert_eq!(duplicate.decision, ActorDecision::Denied);
    assert_eq!(duplicate.issue, Some(ActorIssue::ActiveWake));
    assert!(duplicate.effects.is_empty());

    let wake_ref = waking.active_wake_ref.clone().expect("active wake");
    let started = plan_actor_transition(
        &profile,
        &waking,
        &request(&waking, "start-one", WAKE_TICK, ActorOperation::StartSucceeded { wake_ref }),
    );
    assert_eq!(started.next_state.phase, ActorPhase::Running);
    assert!(started.next_state.active_wake_ref.is_none());

    let replay = plan_actor_transition(&profile, &initial, &wake);
    let duplicate_operation = plan_actor_transition(&profile, &replay.next_state, &wake);
    assert_eq!(duplicate_operation.decision, ActorDecision::Denied);
    assert_eq!(duplicate_operation.issue, Some(ActorIssue::StaleLifecycleSequence));
}

#[test]
fn idle_sleep_requires_time_and_empty_runtime_work() {
    let profile = profile();
    let mut running = state(&profile);
    running.phase = ActorPhase::Running;
    running.last_activity_tick = WAKE_TICK;
    let checkpoint_ref = reference("sleep-checkpoint");

    let too_early = plan_actor_transition(
        &profile,
        &running,
        &request(&running, "sleep-early", WAKE_TICK, ActorOperation::IdleSleep {
            checkpoint_ref: checkpoint_ref.clone(),
            pending_mailbox_items: 0,
            unresolved_effects: 0,
        }),
    );
    assert_eq!(too_early.issue, Some(ActorIssue::IdleThresholdNotReached));

    let pending = plan_actor_transition(
        &profile,
        &running,
        &request(&running, "sleep-pending", SLEEP_TICK, ActorOperation::IdleSleep {
            checkpoint_ref: checkpoint_ref.clone(),
            pending_mailbox_items: 1,
            unresolved_effects: 0,
        }),
    );
    assert_eq!(pending.issue, Some(ActorIssue::PendingMailboxItems));

    let slept = plan_actor_transition(
        &profile,
        &running,
        &request(&running, "sleep", SLEEP_TICK, ActorOperation::IdleSleep {
            checkpoint_ref: checkpoint_ref.clone(),
            pending_mailbox_items: 0,
            unresolved_effects: 0,
        }),
    );
    assert_eq!(slept.next_state.phase, ActorPhase::Dormant);
    assert_eq!(slept.next_state.checkpoint_ref.as_deref(), Some(checkpoint_ref.as_str()));
    assert_eq!(slept.effects[0].kind, ActorEffectIntentKind::PersistCheckpoint);
    assert_eq!(slept.effects[1].kind, ActorEffectIntentKind::StopRuntime);
}

#[test]
fn recovery_exposes_only_durable_survival_classes() {
    let profile = profile();
    let mut recovering = state(&profile);
    let checkpoint_ref = reference("recovery-checkpoint");
    recovering.phase = ActorPhase::Recovering;
    recovering.checkpoint_ref = Some(checkpoint_ref.clone());

    let denied = plan_actor_transition(
        &profile,
        &recovering,
        &request(&recovering, "recover-process", SLEEP_TICK, ActorOperation::RecoverySucceeded {
            checkpoint_ref: checkpoint_ref.clone(),
            restored_classes: vec![SurvivalClass::DurableState, SurvivalClass::Processes],
            durable_state_ref: reference("durable-state"),
        }),
    );
    assert_eq!(
        denied.issue,
        Some(ActorIssue::RestoreClassDenied {
            class: SurvivalClass::Processes,
        })
    );

    let durable = vec![
        SurvivalClass::DurableState,
        SurvivalClass::MailboxEntries,
        SurvivalClass::CompletedSemanticEvents,
        SurvivalClass::Checkpoints,
    ];
    let restored = plan_actor_transition(
        &profile,
        &recovering,
        &request(&recovering, "recover-durable", SLEEP_TICK, ActorOperation::RecoverySucceeded {
            checkpoint_ref,
            restored_classes: durable.clone(),
            durable_state_ref: reference("durable-state"),
        }),
    );
    assert_eq!(restored.decision, ActorDecision::Applied);
    assert_eq!(restored.next_state.phase, ActorPhase::Running);
    assert_eq!(restored.restored_classes, durable);
    assert_eq!(restored.effects[0].kind, ActorEffectIntentKind::StartRuntime);
}

#[test]
fn delivery_ack_requires_a_durable_semantic_commit() {
    let profile = profile();
    let mut running = state(&profile);
    running.phase = ActorPhase::Running;
    let semantic_event_ref = reference("semantic-event");
    let complete = ActorOperation::CompleteDelivery {
        delivery_item_ref: reference("delivery-item"),
        delivery_token_ref: reference("delivery-token"),
        semantic_event_ref: semantic_event_ref.clone(),
        semantic_commit_ref: reference("semantic-commit"),
    };
    let transition =
        plan_actor_transition(&profile, &running, &request(&running, "complete", WAKE_TICK, complete.clone()));
    assert_eq!(transition.decision, ActorDecision::Applied);
    assert_eq!(transition.effects[0].kind, ActorEffectIntentKind::AcknowledgeDelivery);
    assert_eq!(transition.next_state.completed_event_refs, vec![semantic_event_ref]);

    let duplicate = plan_actor_transition(
        &profile,
        &transition.next_state,
        &request(&transition.next_state, "complete-again", WAKE_TICK, complete),
    );
    assert_eq!(duplicate.decision, ActorDecision::DuplicateReplay);
    assert!(duplicate.effects.is_empty());
}

#[test]
fn unknown_effect_blocks_automatic_work_until_explicit_resolution() {
    let profile = profile();
    let mut running = state(&profile);
    running.phase = ActorPhase::Running;
    let effect_ref = reference("uncertain-effect");
    let unknown = plan_actor_transition(
        &profile,
        &running,
        &request(&running, "unknown", WAKE_TICK, ActorOperation::RecordUnknownEffect {
            effect_ref: effect_ref.clone(),
        }),
    );
    assert_eq!(unknown.next_state.phase, ActorPhase::Degraded);
    assert_eq!(unknown.next_state.unknown_effect_ref.as_deref(), Some(effect_ref.as_str()));
    assert!(!unknown.external_effect_retry_authorized);

    let blocked = plan_actor_transition(
        &profile,
        &unknown.next_state,
        &request(&unknown.next_state, "blind-wake", SLEEP_TICK, ActorOperation::Wake { reason: message_wake() }),
    );
    assert_eq!(blocked.decision, ActorDecision::Unknown);
    assert_eq!(blocked.issue, Some(ActorIssue::UnknownExternalOutcome));
    assert!(blocked.effects.is_empty());

    let resolved = plan_actor_transition(
        &profile,
        &unknown.next_state,
        &request(&unknown.next_state, "operator-resolution", SLEEP_TICK, ActorOperation::ResolveUnknownEffect {
            effect_ref,
            resolution_ref: reference("operator-resolution"),
            checkpoint_ref: reference("checkpoint"),
        }),
    );
    assert_eq!(resolved.next_state.phase, ActorPhase::Recovering);
    assert!(resolved.next_state.unknown_effect_ref.is_none());
    assert!(!resolved.external_effect_retry_authorized);
}

#[test]
fn generic_system_extension_binding_is_explicit() {
    let profile = profile();
    let mut actor = state(&profile);
    actor.phase = ActorPhase::Dormant;
    actor.checkpoint_ref = Some(reference("checkpoint"));
    let extension = LifecycleState {
        generation: actor.extension_generation,
        phase: LifecyclePhase::Drained,
        restart_attempts: 0,
        health: HealthState::Stopped,
        checkpoint_ref: actor.checkpoint_ref.clone(),
    };
    assert!(validate_system_extension_binding(&actor, &extension).is_empty());

    let mut stale = extension;
    stale.generation = stale.generation.saturating_add(1);
    stale.phase = LifecyclePhase::Running;
    let issues = validate_system_extension_binding(&actor, &stale);
    assert!(issues.contains(&ActorIssue::SystemExtensionGenerationMismatch));
    assert!(issues.contains(&ActorIssue::SystemExtensionPhaseMismatch));
}

#[test]
fn status_is_bounded_read_only_and_payload_free() {
    let profile = profile();
    let mut state = state(&profile);
    state.completed_event_refs = vec![reference("event-a"), reference("event-b")];
    state.completed_event_refs.sort();
    let status = project_actor_status(&state, ActorStatusProjectionInput {
        maximum_events: STATUS_LIMIT,
        evidence_refs: &[reference("evidence")],
    })
    .expect("status projection");
    assert!(status.truncated);
    assert_eq!(status.completed_event_refs.len(), STATUS_LIMIT);
    assert!(!status.payloads_rendered);
    assert!(!status.authorizes_mutation);
}

#[test]
fn timer_wake_and_bounded_drain_use_typed_effects() {
    let profile = profile();
    let initial = state(&profile);
    let timer = plan_actor_transition(
        &profile,
        &initial,
        &request(&initial, "wake-timer", WAKE_TICK, ActorOperation::Wake {
            reason: WakeReason::Timer {
                timer_ref: reference("timer"),
            },
        }),
    );
    assert_eq!(timer.effects.len(), 2);
    assert_eq!(timer.effects[0].kind, ActorEffectIntentKind::StartRuntime);
    assert_eq!(timer.effects[1].kind, ActorEffectIntentKind::InvokeTimer);

    let mut running = state(&profile);
    running.phase = ActorPhase::Running;
    let draining = plan_actor_transition(
        &profile,
        &running,
        &request(&running, "begin-drain", WAKE_TICK, ActorOperation::BeginDrain),
    );
    let denied = plan_actor_transition(
        &profile,
        &draining.next_state,
        &request(&draining.next_state, "drain-incomplete", SLEEP_TICK, ActorOperation::DrainSucceeded {
            checkpoint_ref: reference("drain-checkpoint"),
            remaining_items: 1,
        }),
    );
    assert_eq!(denied.issue, Some(ActorIssue::DrainNotComplete));
    let completed = plan_actor_transition(
        &profile,
        &draining.next_state,
        &request(&draining.next_state, "drain-complete", SLEEP_TICK, ActorOperation::DrainSucceeded {
            checkpoint_ref: reference("drain-checkpoint"),
            remaining_items: 0,
        }),
    );
    assert_eq!(completed.next_state.phase, ActorPhase::Stopped);
    assert_eq!(completed.effects[0].kind, ActorEffectIntentKind::PersistCheckpoint);
    assert_eq!(completed.effects[1].kind, ActorEffectIntentKind::StopRuntime);
}

#[test]
fn missing_authority_resource_denial_and_failed_recovery_preserve_boundaries() {
    let profile = profile();
    let initial = state(&profile);
    let mut missing_authority =
        request(&initial, "missing-authority", WAKE_TICK, ActorOperation::Wake { reason: message_wake() });
    missing_authority.admission.authority_ref.clear();
    let denied_authority = plan_actor_transition(&profile, &initial, &missing_authority);
    assert_eq!(denied_authority.decision, ActorDecision::Denied);
    assert!(denied_authority.effects.is_empty());

    let mut denied_resource =
        request(&initial, "resource-denied", WAKE_TICK, ActorOperation::Wake { reason: message_wake() });
    denied_resource.admission.resources_admitted = false;
    let denied_resource = plan_actor_transition(&profile, &initial, &denied_resource);
    assert_eq!(denied_resource.issue, Some(ActorIssue::AdmissionDenied));
    assert_eq!(denied_resource.next_state, initial);

    let mut recovering = state(&profile);
    recovering.phase = ActorPhase::Recovering;
    recovering.checkpoint_ref = Some(reference("checkpoint"));
    let failed = plan_actor_transition(
        &profile,
        &recovering,
        &request(&recovering, "recovery-failed", SLEEP_TICK, ActorOperation::RecoveryFailed {
            failure_ref: reference("restore-failure"),
        }),
    );
    assert_eq!(failed.next_state.phase, ActorPhase::Degraded);
    assert_eq!(failed.effects[0].kind, ActorEffectIntentKind::NotifyOperator);
}

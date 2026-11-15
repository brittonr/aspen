use molten_core::addressable_actor::*;

use super::super::*;
use super::support::*;

fn wake() -> Outcome {
    let profile = profile();
    let key = key();
    let initial = initial_state(&profile, &key);
    let request = request(&initial, "wake", INITIAL_TICK, ActorOperation::Wake { reason: message_wake() });
    apply_request(
        &mut MemoryCommitPort::new(CommitMode::Apply),
        &mut MemoryEffectPort::succeeding(2),
        &mut MemoryStatusPort::default(),
        &Request {
            profile: &profile,
            actor_key: &key,
            host_binding: &host_binding(&profile, &initial),
            expected: empty_expected(),
            request: &request,
            requested_engine_epoch: ENGINE_EPOCH,
        },
    )
    .expect("wake")
}

fn completion() -> Outcome {
    let profile = profile();
    let key = key();
    let mut running = initial_state(&profile, &key);
    running.phase = ActorPhase::Running;
    let published = PublishedState::from_state(running.clone());
    let request = request(&running, "complete", INITIAL_TICK, ActorOperation::CompleteDelivery {
        delivery_item_ref: reference("item"),
        delivery_token_ref: reference("token"),
        semantic_event_ref: reference("event"),
        semantic_commit_ref: reference("commit"),
    });
    let mut commits = MemoryCommitPort::new(CommitMode::Apply);
    commits.head = Some(published.clone());
    apply_request(&mut commits, &mut MemoryEffectPort::succeeding(1), &mut MemoryStatusPort::default(), &Request {
        profile: &profile,
        actor_key: &key,
        host_binding: &host_binding(&profile, &running),
        expected: expected(&published),
        request: &request,
        requested_engine_epoch: ENGINE_EPOCH,
    })
    .expect("complete actual actor transition")
}

// r[verify molten.addressable_actor.public_result]
#[test]
fn durable_wake_and_callback_success_are_progress_not_semantic_completion() {
    let outcome = wake();
    assert_eq!(outcome.receipt.status, ServiceStatus::Applied);
    assert_eq!(public_outcome(&outcome), PublicOutcome::Progress);
}

#[test]
fn uncertain_and_buffered_completion_cannot_be_reported_applied() {
    let mut outcome = completion();
    assert_eq!(public_outcome(&outcome), PublicOutcome::Applied);
    outcome.commit_observation.as_mut().expect("commit").durability = DurabilityOutcome::Buffered;
    assert_eq!(public_outcome(&outcome), PublicOutcome::Unknown);
    outcome.commit_observation.as_mut().expect("commit").durability = DurabilityOutcome::Durable;
    outcome.effect_observations[0].disposition = EffectDisposition::Unknown;
    assert_eq!(public_outcome(&outcome), PublicOutcome::Unknown);
    outcome.receipt.status = ServiceStatus::Stale;
    assert_eq!(public_outcome(&outcome), PublicOutcome::Stale);
    outcome.receipt.status = ServiceStatus::EffectAdmissionDenied;
    assert_eq!(public_outcome(&outcome), PublicOutcome::Denied);
    outcome.status_observation.outcome_unknown = true;
    assert_eq!(public_outcome(&outcome), PublicOutcome::Unknown);
}

#[test]
fn actual_running_postcondition_completes_start_without_restarting_runtime() {
    let profile = profile();
    let key = key();
    let mut starting = initial_state(&profile, &key);
    starting.phase = ActorPhase::Starting;
    starting.active_wake_ref = Some(identify_actor_wake(&message_wake()));
    let published = PublishedState::from_state(starting.clone());
    let request = request(&starting, "complete-start", INITIAL_TICK, ActorOperation::StartSucceeded {
        wake_ref: starting.active_wake_ref.clone().expect("wake"),
    });
    let mut host = host_binding(&profile, &starting);
    host.system_extension_phase = molten_core::system_extension::LifecyclePhase::Running;
    let mut commits = MemoryCommitPort::new(CommitMode::Apply);
    commits.head = Some(published.clone());
    let mut effects = MemoryEffectPort::succeeding(0);
    let outcome = apply_request(&mut commits, &mut effects, &mut MemoryStatusPort::default(), &Request {
        profile: &profile,
        actor_key: &key,
        host_binding: &host,
        expected: expected(&published),
        request: &request,
        requested_engine_epoch: ENGINE_EPOCH,
    })
    .expect("observed running completion");
    assert_eq!(outcome.final_state.state.phase, ActorPhase::Running);
    assert_eq!(effects.execution_calls, 0);
    assert_eq!(public_outcome(&outcome), PublicOutcome::Progress);
}

#[test]
fn refused_acknowledgement_cannot_become_semantic_success_on_metadata_replay() {
    let profile = profile();
    let key = key();
    let mut running = initial_state(&profile, &key);
    running.phase = ActorPhase::Running;
    let published = PublishedState::from_state(running.clone());
    let operation = ActorOperation::CompleteDelivery {
        delivery_item_ref: reference("item"),
        delivery_token_ref: reference("token"),
        semantic_event_ref: reference("event"),
        semantic_commit_ref: reference("commit"),
    };
    let first = request(&running, "complete-before-ack", INITIAL_TICK, operation.clone());
    let mut commits = MemoryCommitPort::new(CommitMode::Apply);
    commits.head = Some(published.clone());
    let mut effects = MemoryEffectPort::succeeding(1);
    effects.deny_admission_at = Some(0);
    let refused = apply_request(&mut commits, &mut effects, &mut MemoryStatusPort::default(), &Request {
        profile: &profile,
        actor_key: &key,
        host_binding: &host_binding(&profile, &running),
        expected: expected(&published),
        request: &first,
        requested_engine_epoch: ENGINE_EPOCH,
    })
    .expect("refused acknowledgement");
    assert_eq!(refused.receipt.status, ServiceStatus::EffectAdmissionDenied);
    assert_eq!(effects.execution_calls, 0);
    let current = commits.head.clone().expect("completion metadata committed before acknowledgement");
    let replay = request(&current.state, "replay-unacknowledged", INITIAL_TICK + 1, operation);
    let observed = apply_request(&mut commits, &mut effects, &mut MemoryStatusPort::default(), &Request {
        profile: &profile,
        actor_key: &key,
        host_binding: &host_binding(&profile, &current.state),
        expected: expected(&current),
        request: &replay,
        requested_engine_epoch: ENGINE_EPOCH,
    })
    .expect("metadata replay");
    assert_eq!(observed.transition.decision, ActorDecision::DuplicateReplay);
    assert_eq!(effects.execution_calls, 0);
    assert_eq!(public_outcome(&observed), PublicOutcome::Unknown);
}

fn completion_with_acknowledgement(refused: bool) -> (Outcome, crate::addressable_actor::store::completion::Evidence) {
    use molten_core::coordination_delivery::*;

    let profile = profile();
    let key = key();
    let mut running = initial_state(&profile, &key);
    running.phase = ActorPhase::Running;
    let mut token = DeliveryToken {
        token_ref: String::new(),
        delivery_id: reference("delivery"),
        queue_id: "actor-queue".to_string(),
        item_ref: reference("item"),
        consumer_id: "counter".to_string(),
        attempt: 1,
        cycle: 1,
        fencing_token: 1,
        claimed_at_tick: INITIAL_TICK,
        visibility_deadline_tick: INITIAL_TICK + 10,
        consistency_epoch: 1,
        service_generation: 1,
        policy_ref: reference("delivery-policy"),
    };
    token.token_ref = identify_delivery_token(&token);
    let operation = ActorOperation::CompleteDelivery {
        delivery_item_ref: token.item_ref.clone(),
        delivery_token_ref: token.token_ref.clone(),
        semantic_event_ref: reference("event"),
        semantic_commit_ref: reference("commit"),
    };
    let acknowledgement = DeliveryRequest {
        schema: DELIVERY_REQUEST_SCHEMA.to_string(),
        queue_id: token.queue_id.clone(),
        operation_id: "original-acknowledgement".to_string(),
        actor_id: token.consumer_id.clone(),
        service_generation: 1,
        consistency_epoch: 1,
        engine_epoch: ENGINE_EPOCH,
        time_profile_ref: reference("time"),
        logical_tick: INITIAL_TICK,
        currentness: DeliveryCurrentness::Linearizable,
        authority_refs: vec![reference("authority")],
        policy_refs: vec![token.policy_ref.clone()],
        resource_refs: vec![reference("resource")],
        evidence_refs: vec![reference("event"), reference("commit"), reference("semantic-state")],
        operation: DeliveryOperation::Acknowledge { token },
    };
    let published = PublishedState::from_state(running.clone());
    let mut commits = MemoryCommitPort::new(CommitMode::Apply);
    commits.head = Some(published.clone());
    let mut effects = MemoryEffectPort::succeeding(1);
    effects.deny_admission_at = refused.then_some(0);
    let request = request(&running, "original-completion", INITIAL_TICK, operation.clone());
    let outcome = apply_request(&mut commits, &mut effects, &mut MemoryStatusPort::default(), &Request {
        profile: &profile,
        actor_key: &key,
        host_binding: &host_binding(&profile, &running),
        expected: expected(&published),
        request: &request,
        requested_engine_epoch: ENGINE_EPOCH,
    })
    .expect("actor completion with actual shell admission outcome");
    let evidence = crate::addressable_actor::store::completion::Evidence {
        ingress_ref: reference("ingress"),
        actor_key_ref: running.actor_key_ref,
        operation,
        acknowledgement,
        receipt_bytes: outcome.receipt.bytes.clone(),
        receipt_ref: outcome.receipt.receipt_ref.clone(),
    };
    (outcome, evidence)
}

#[test]
fn original_completion_receipt_and_exact_acknowledgement_survive_reopen() {
    let workspace =
        crate::test_support::process_workspace("actor-original-completion-evidence").expect("scoped actor workspace");
    let root = crate::node_state::DirectoryRoot::open_existing(workspace.as_ref()).expect("actor root");
    root.create_layout().expect("node layout");
    let storage = root.namespace(crate::node_state::NamespaceKind::Storage).expect("actor storage");
    let (outcome, evidence) = completion_with_acknowledgement(false);
    assert_eq!(public_outcome(&outcome), PublicOutcome::Applied);
    {
        let mut owner = Local::open(&storage, ENGINE_EPOCH).expect("actor owner");
        assert!(
            owner
                .completion(&evidence.actor_key_ref, &evidence.ingress_ref)
                .expect("pending observation")
                .is_none()
        );
        owner.retain_completion(&evidence).expect("retain original successful canonical receipt");
        owner.retain_completion(&evidence).expect("exact evidence duplicate is immutable");
    }
    let owner = Local::open(&storage, ENGINE_EPOCH).expect("reopened actor owner");
    let original = owner
        .completion(&evidence.actor_key_ref, &evidence.ingress_ref)
        .expect("read original evidence")
        .expect("successful actor completion");
    assert_eq!(original.receipt_bytes, outcome.receipt.bytes);
    assert_eq!(original.receipt_ref, outcome.receipt.receipt_ref);
    assert!(original.matches_original(None));
    assert!(original.matches_original(Some(&evidence.acknowledgement)));
    let mut later = evidence.acknowledgement.clone();
    later.operation_id = "separately-authorized-acknowledgement".to_string();
    assert!(!original.matches_original(Some(&later)));
    assert_eq!(
        owner
            .completion(&evidence.actor_key_ref, &evidence.ingress_ref)
            .expect("readback again")
            .expect("original result remains")
            .receipt_bytes,
        outcome.receipt.bytes
    );
}

#[test]
fn refused_receipt_cannot_supply_successful_original_evidence() {
    let workspace =
        crate::test_support::process_workspace("actor-refused-completion-evidence").expect("scoped actor workspace");
    let root = crate::node_state::DirectoryRoot::open_existing(workspace.as_ref()).expect("actor root");
    root.create_layout().expect("node layout");
    let storage = root.namespace(crate::node_state::NamespaceKind::Storage).expect("actor storage");
    let (refused, evidence) = completion_with_acknowledgement(true);
    assert_eq!(public_outcome(&refused), PublicOutcome::Denied);
    assert!(refused.final_state.state.completed_event_refs.contains(&reference("event")));
    let mut owner = Local::open(&storage, ENGINE_EPOCH).expect("actor owner");
    assert!(owner.retain_completion(&evidence).is_err(), "a canonical refusal is not completion evidence");
    assert!(
        owner
            .completion(&evidence.actor_key_ref, &evidence.ingress_ref)
            .expect("read refused original")
            .is_none()
    );
    let mut fabricated = evidence;
    fabricated.receipt_bytes.push(0);
    assert!(owner.retain_completion(&fabricated).is_err(), "malformed evidence must remain an error");
}

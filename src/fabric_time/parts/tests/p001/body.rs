
// r[verify molten.audit_f09.validation]
#[test]
fn extension_context_resumes_a_blocked_occurrence_at_the_active_limit() {
    let mut limited = simulation_profile();
    limited.profile.max_runnables = 1;
    limited.profile.max_scheduler_concurrency = 1;
    limited.profile.max_scheduler_queue_depth = 1;
    let profile = &limited.profile;
    let context =
        ExtensionTimeContext::from_test_snapshot("test-service", GENERATION, profile, vec![HASH_B.to_string()]);
    let policy = SchedulerPolicy {
        ordering: SchedulerOrdering::Fifo,
        replay: SchedulerReplayPolicy::Deterministic,
        overload: SchedulerOverloadPolicy::Reject,
    };
    let key = RunnableKey {
        service_id: "test-service".to_string(),
        generation: GENERATION,
        runnable_id: "resumed".to_string(),
    };
    let mut state = new_scheduler_state(profile, GENERATION);
    state = context
        .apply_scheduler_command(profile, policy, &state, &SchedulerCommand::Wake {
            key: key.clone(),
            priority: 0,
        })
        .expect("admitted wake")
        .next;
    state = context.choose_runnable(profile, policy, &state, None).expect("select").next;
    state = context
        .apply_scheduler_command(profile, policy, &state, &SchedulerCommand::Block { key: key.clone() })
        .expect("admitted block")
        .next;
    let blocked = state.clone();

    let resumed = context
        .apply_scheduler_command(profile, policy, &state, &SchedulerCommand::Wake {
            key: key.clone(),
            priority: 1,
        })
        .expect("admitted resume at the active limit");

    assert_eq!(resumed.action, SchedulerAction::Woken);
    assert_eq!(resumed.next.runnables.len(), blocked.runnables.len());
    assert!(
        resumed
            .next
            .runnables
            .iter()
            .any(|runnable| runnable.key == key && runnable.phase == RunnablePhase::Ready)
    );
}

// r[verify molten.audit_f10.validation]
#[test]
fn extension_context_reports_overload_for_yield_at_the_ready_bound() {
    let mut limited = simulation_profile();
    limited.profile.max_scheduler_queue_depth = 1;
    let profile = &limited.profile;
    let context =
        ExtensionTimeContext::from_test_snapshot("test-service", GENERATION, profile, vec![HASH_B.to_string()]);
    let policy = SchedulerPolicy {
        ordering: SchedulerOrdering::Fifo,
        replay: SchedulerReplayPolicy::Deterministic,
        overload: SchedulerOverloadPolicy::Reject,
    };
    let running_key = RunnableKey {
        service_id: "test-service".to_string(),
        generation: GENERATION,
        runnable_id: "yielding".to_string(),
    };
    let queued_key = RunnableKey {
        service_id: "test-service".to_string(),
        generation: GENERATION,
        runnable_id: "queued".to_string(),
    };
    let mut state = new_scheduler_state(profile, GENERATION);
    state = context
        .apply_scheduler_command(profile, policy, &state, &SchedulerCommand::Wake {
            key: running_key.clone(),
            priority: 0,
        })
        .expect("admitted wake")
        .next;
    state = context.choose_runnable(profile, policy, &state, None).expect("select").next;
    state = context
        .apply_scheduler_command(profile, policy, &state, &SchedulerCommand::Wake {
            key: queued_key.clone(),
            priority: 0,
        })
        .expect("admitted queued wake")
        .next;
    let before = state.clone();

    let denied = context
        .apply_scheduler_command(profile, policy, &state, &SchedulerCommand::Yield {
            key: running_key.clone(),
        })
        .expect("yield decision at the ready bound");

    assert_eq!(denied.action, SchedulerAction::RejectedOverload);
    assert_eq!(denied.next, before);
    assert!(
        denied
            .next
            .runnables
            .iter()
            .any(|runnable| runnable.key == running_key && runnable.phase == RunnablePhase::Running)
    );
}

// r[verify molten.audit_f10.validation]
#[test]
fn corrected_yield_admission_diverges_from_an_over_capacity_history() {
    let mut limited = simulation_profile();
    limited.profile.max_scheduler_queue_depth = 1;
    let profile = &limited.profile;
    let context =
        ExtensionTimeContext::from_test_snapshot("test-service", GENERATION, profile, vec![HASH_B.to_string()]);
    let policy = profile.scheduler_policy;
    let running_key = RunnableKey {
        service_id: "test-service".to_string(),
        generation: GENERATION,
        runnable_id: "yielding".to_string(),
    };
    let queued_key = RunnableKey {
        service_id: "test-service".to_string(),
        generation: GENERATION,
        runnable_id: "queued".to_string(),
    };
    let mut state = new_scheduler_state(profile, GENERATION);
    state = context
        .apply_scheduler_command(profile, policy, &state, &SchedulerCommand::Wake {
            key: running_key.clone(),
            priority: 0,
        })
        .expect("admitted wake")
        .next;
    state = context.choose_runnable(profile, policy, &state, None).expect("select").next;
    state = context
        .apply_scheduler_command(profile, policy, &state, &SchedulerCommand::Wake {
            key: queued_key.clone(),
            priority: 0,
        })
        .expect("admitted queued wake")
        .next;

    let replayed = context
        .apply_scheduler_command(profile, policy, &state, &SchedulerCommand::Yield {
            key: running_key.clone(),
        })
        .expect("replay decision");

    // A history recorded under the former rule reported `Yielded`; the corrected admission
    // reports the overload result and leaves the recorded state untouched.
    assert_ne!(replayed.action, SchedulerAction::Yielded);
    assert_eq!(replayed.action, SchedulerAction::RejectedOverload);
    assert_eq!(replayed.next, state);
}

#[test]
fn entropy_evidence_omits_output_bytes() {
    let profile = simulation_profile();
    let stream = open_entropy_stream(&profile.profile, GENERATION, &EntropyStreamRequest {
        profile_ref: profile.profile.profile_ref.clone(),
        stream_id: "stream".to_string(),
        purpose: "purpose".to_string(),
        capability_ref: HASH_B.to_string(),
        generation: GENERATION,
        mode: EntropyMode::DeterministicSimulation,
        explicit_simulation_seed: Some(SECRET_TEST_SEED),
        explicit_simulation_seed_ref: Some(HASH_A.to_string()),
    })
    .expect("stream");
    let transition = draw_deterministic_entropy(&profile.profile, GENERATION, &stream, EntropyRequest::Bytes {
        count: ENTROPY_COUNT,
    })
    .expect("draw");
    let secret_hex = match &transition.value {
        EntropyValue::Bytes(bytes) => bytes.iter().map(|byte| format!("{byte:02x}")).collect::<String>(),
        EntropyValue::Choice(_) => panic!("expected bytes"),
    };
    let event = canonical_entropy_event(&entropy_evidence_metadata(&stream, &transition)).expect("canonical event");
    let encoded = crate::preserves_rail::to_text(&event.value).expect("encode event");
    assert!(!encoded.contains(&secret_hex));
    assert!(!encoded.contains(&SECRET_TEST_SEED.to_string()));
    assert!(encoded.contains(HASH_A));
    assert!(encoded.contains("secret-output-omitted"));
}

#[test]
fn reference_matrix_preserves_plugin_extension_and_application_authority_boundaries() {
    let plugin = crate::fabric::ExtensionTierRequest {
        tier: crate::fabric::ExtensionTier::SandboxedPlugin,
        requested_authorities: vec![crate::fabric::FabricAuthority::Time],
        admission_evidence: Vec::new(),
    };
    assert!(crate::fabric::validate_extension_tier(&plugin).is_err());

    let extension = crate::fabric::ExtensionTierRequest {
        tier: crate::fabric::ExtensionTier::SystemExtension,
        requested_authorities: vec![
            crate::fabric::FabricAuthority::Time,
            crate::fabric::FabricAuthority::Scheduling,
        ],
        admission_evidence: crate::fabric::REQUIRED_SYSTEM_EXTENSION_EVIDENCE.to_vec(),
    };
    assert!(crate::fabric::validate_extension_tier(&extension).is_ok());

    let application = crate::fabric::ExtensionTierRequest {
        tier: crate::fabric::ExtensionTier::ApplicationWorkload,
        requested_authorities: vec![crate::fabric::FabricAuthority::ApplicationServiceUse],
        admission_evidence: Vec::new(),
    };
    assert!(crate::fabric::validate_extension_tier(&application).is_ok());
}

// r[verify molten.audit_f12.validation]
#[test]
fn retry_observations_replay_exactly_and_differ_from_wrapped_history() {
    const AUDIT_BASE: u64 = 2;
    const AUDIT_ATTEMPT: u64 = 63;
    let profile = simulation_profile();
    let now = TimeValue::Virtual(VirtualInstant {
        profile_ref: profile.profile.profile_ref.clone(),
        ticks: TIMER_DEADLINE,
    });
    let policy = RetryPolicy {
        maximum_attempts: u64::MAX,
        base_delay_ticks: AUDIT_BASE,
        maximum_delay_ticks: PROFILE_LIMIT,
        backoff: RetryBackoff::Exponential,
        jitter: RetryJitter::None,
    };
    let events = super::fixture::retry_events(&profile, &now, AUDIT_ATTEMPT, policy, None).expect("retry evidence");
    let expected_deadline = canonical_named_event(EventHeader {
        profile_ref: &profile.profile_ref,
        kind: CanonicalTimeEventKind::Deadline,
        generation: GENERATION,
        subject: "fixture-retry",
        action: "retry-planned",
        ticks: TIMER_DEADLINE + PROFILE_LIMIT,
    })
    .expect("deadline");
    let expected_delay = canonical_named_event(EventHeader {
        profile_ref: &profile.profile_ref,
        kind: CanonicalTimeEventKind::Deadline,
        generation: GENERATION,
        subject: "fixture-retry",
        action: "retry-delay",
        ticks: PROFILE_LIMIT,
    })
    .expect("delay");
    assert_eq!(events, [expected_deadline.clone(), expected_delay]);
    let replay = super::fixture::retry_events(&profile, &now, AUDIT_ATTEMPT, policy, None).expect("retry replay");
    assert_eq!(events, replay);
    let legacy = canonical_named_event(EventHeader {
        profile_ref: &profile.profile_ref,
        kind: CanonicalTimeEventKind::Deadline,
        generation: GENERATION,
        subject: "fixture-retry",
        action: "retry-planned",
        ticks: TIMER_DEADLINE,
    })
    .expect("legacy wrapped deadline");
    assert_ne!(expected_deadline.value, legacy.value);
    assert_ne!(expected_deadline.evidence_ref, legacy.evidence_ref);
}

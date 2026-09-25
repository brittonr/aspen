mod replay;

use super::*;

const HASH_A: &str = "blake3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const HASH_B: &str = "blake3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const GENERATION: u64 = 1;
const STALE_GENERATION: u64 = 2;
const PROFILE_LIMIT: u64 = 128;
const ENTROPY_TOTAL_LIMIT: u64 = 1_024;
const CONCURRENCY_LIMIT: u64 = 4;
const QUEUE_LIMIT: u64 = 8;
const WALL_BASE: u64 = 1_000;
const TIMER_DEADLINE: u64 = 10;
const TIMER_DELAY: u64 = 5;
const ENTROPY_COUNT: u64 = 8;
const TIME_PORTS_PER_PROFILE: usize = 4;
const PROFILE_COUNT: usize = 2;
const EXPECTED_PORT_DESCRIPTOR_REFS: usize = TIME_PORTS_PER_PROFILE * PROFILE_COUNT;
const SECRET_TEST_SEED: u64 = 9_876_543_210;

fn descriptor(kind: TimeProfileKind, id: &str, profile_ref: &str) -> TimeProfileDescriptor {
    let replay = match kind {
        TimeProfileKind::Live => SchedulerReplayPolicy::RecordedChoiceRequired,
        TimeProfileKind::DeterministicSimulation => SchedulerReplayPolicy::Deterministic,
    };
    TimeProfileDescriptor {
        schema: FABRIC_TIME_PROFILE_SCHEMA.to_string(),
        profile_id: id.to_string(),
        profile_ref: profile_ref.to_string(),
        kind,
        supported_domains: REQUIRED_TIME_DOMAINS.to_vec(),
        max_duration_ticks: PROFILE_LIMIT,
        max_uncertainty_ticks: PROFILE_LIMIT,
        max_timers: PROFILE_LIMIT,
        max_runnables: PROFILE_LIMIT,
        max_entropy_request_bytes: PROFILE_LIMIT,
        max_entropy_total_bytes: ENTROPY_TOTAL_LIMIT,
        max_scheduler_concurrency: CONCURRENCY_LIMIT,
        max_scheduler_queue_depth: QUEUE_LIMIT,
        fairness_bound_turns: None,
        scheduler_policy: SchedulerPolicy {
            ordering: SchedulerOrdering::Fifo,
            replay,
            overload: SchedulerOverloadPolicy::Reject,
        },
        evidence_mode: TimeEvidenceMode::Aggregate,
        non_claims: REQUIRED_TIME_NON_CLAIMS.to_vec(),
    }
}

fn simulation_profile() -> CanonicalTimeProfile {
    canonical_admit_time_profile(&descriptor(TimeProfileKind::DeterministicSimulation, "test-simulation", HASH_A))
        .expect("simulation profile")
}

pub(crate) fn live_profile() -> CanonicalTimeProfile {
    canonical_admit_time_profile(&descriptor(TimeProfileKind::Live, "test-live", HASH_B)).expect("live profile")
}

fn one_shot(profile: &AdmittedTimeProfile, generation: u64) -> TimerScheduleRequest {
    TimerScheduleRequest {
        profile_ref: profile.profile_ref.clone(),
        key: TimerKey {
            service_id: "test-service".to_string(),
            generation,
            sequence: 0,
        },
        domain: TimeDomain::Virtual,
        deadline_ticks: TIMER_DEADLINE,
        kind: TimerKind::OneShot,
        ordering_key: 0,
        coalescing: TimerCoalescingPolicy::CoalesceLatest,
        lateness: TimerLatenessPolicy::DeliverRegardless,
        overload: TimerOverloadPolicy::RejectAndRetain,
        resource_charge: TimerResourceCharge::single_slot(),
    }
}

// r[verify molten.modularity.fabric_boundary.adapters.clock]
// r[impl molten.fabric_time.final_validation]
#[test]
fn executable_fixture_exercises_both_profiles_and_bounded_evidence() {
    let run = run_executable_fabric_time_fixture(FabricTimeFixtureSelection::Both).expect("fabric time fixture");
    assert_eq!(run.live_conformance.timer_action, TimerAction::Deliver);
    assert_eq!(run.simulation_conformance.timer_action, TimerAction::Deliver);
    assert_eq!(run.live_conformance.domain, TimeDomain::Monotonic);
    assert_eq!(run.simulation_conformance.domain, TimeDomain::Virtual);
    assert!(run.live_conformance.stale_generation_discarded);
    assert!(run.simulation_conformance.cancellation_prevented_delivery);
    assert!(run.live_conformance.scheduler_selected);
    assert!(run.simulation_conformance.scheduler_cancellation_recorded);
    assert!(run.live_conformance.entropy_bound_rejected);
    assert_eq!(run.port_descriptor_refs.len(), EXPECTED_PORT_DESCRIPTOR_REFS);
    assert!(!run.events.is_empty());
    assert!(run.report.report.shared_conformance_passed);
    let readback = parse_fabric_time_run_readback(&run.report.value).expect("report readback");
    assert_eq!(readback.report_ref, run.report.report_ref);
    assert!(parse_fabric_time_run_readback(&run.events[0].value).is_err());
    assert_eq!(run.production_entropy_source, "unix-dev-urandom");
}

#[test]
fn deterministic_fixture_report_is_reproducible_despite_live_adapter_execution() {
    let first = run_executable_fabric_time_fixture(FabricTimeFixtureSelection::DeterministicSimulation)
        .expect("first deterministic fixture");
    let second = run_executable_fabric_time_fixture(FabricTimeFixtureSelection::DeterministicSimulation)
        .expect("second deterministic fixture");
    assert_eq!(first.report.report_ref, second.report.report_ref);
    assert_eq!(first.report.report.evidence_refs, second.report.report.evidence_refs);
}

#[test]
fn canonical_time_values_and_durations_bind_domain_and_profile() {
    let profile = simulation_profile();
    let time = TimeValue::Virtual(VirtualInstant {
        profile_ref: profile.profile.profile_ref.clone(),
        ticks: TIMER_DEADLINE,
    });
    let canonical_time = canonical_time_value(&profile, &time).expect("canonical time");
    let canonical_duration = canonical_duration(&profile, &CheckedDuration {
        profile_ref: profile.profile.profile_ref.clone(),
        domain: TimeDomain::Virtual,
        ticks: TIMER_DELAY,
    })
    .expect("canonical duration");
    assert!(canonical_time.value_ref.starts_with("blake3:"));
    assert!(canonical_duration.value_ref.starts_with("blake3:"));

    let mixed = CheckedDuration {
        domain: TimeDomain::Logical,
        ..canonical_duration.duration
    };
    assert!(checked_add_duration(&profile.profile, &time, &mixed).is_err());
}

#[test]
fn canonical_profile_rejects_missing_non_claim() {
    let mut invalid = descriptor(TimeProfileKind::DeterministicSimulation, "invalid-simulation", HASH_A);
    invalid.non_claims.pop();
    let error = canonical_admit_time_profile(&invalid).expect_err("missing non-claim must fail");
    assert!(error.to_string().contains("MissingNonClaim"));
}

#[test]
fn port_bindings_reject_silent_profile_substitution() {
    let profile = simulation_profile();
    let descriptors = fabric_time_port_descriptors(&profile);
    let registry = crate::fabric::build_fabric_port_registry(&descriptors).expect("time registry");
    let clock = &descriptors[0];
    let mut requirement = crate::fabric::FabricPortRequirement {
        port_id: clock.port_id.clone(),
        version: clock.version.clone(),
        class: crate::fabric::FabricPortClass::Time,
        operation_classes: clock.operation_classes.clone(),
        input_schema_refs: clock.input_schema_refs.clone(),
        output_schema_refs: clock.output_schema_refs.clone(),
        allowed_authorities: vec![crate::fabric::FabricAuthority::Time],
        available_resources: vec![crate::fabric::FabricResource::LogicalTime],
        expected_determinism: crate::fabric::DeterminismClass::DeterministicWithRecordedInputs,
        expected_replay: crate::fabric::ReplayClass::Recompute,
        expected_profile: "unadmitted-fallback".to_string(),
    };
    assert!(crate::fabric::resolve_fabric_port_binding(&registry, &requirement).is_err());
    requirement.expected_profile = profile.profile.profile_id.clone();
    assert!(crate::fabric::resolve_fabric_port_binding(&registry, &requirement).is_ok());
}

#[test]
fn virtual_faults_are_explicit_and_timer_faults_do_not_bypass_core() {
    let profile = simulation_profile();
    let mut clock = VirtualClockAdapter::new(&profile.profile, 0, WALL_BASE).expect("clock");
    let previous = clock.observe_wall().expect("first wall");
    assert!(
        apply_clock_fault(&mut clock, &FabricTimeFault::BackwardWallJump { ticks: TIMER_DELAY }).expect("apply fault")
    );
    clock.advance(1).expect("advance");
    let observed = clock.observe_wall().expect("second wall");
    let anomaly = classify_wall_clock_observation(&previous, &observed, WallClockAnomalyPolicy {
        max_forward_jump_nanos: PROFILE_LIMIT,
        max_uncertainty_nanos: PROFILE_LIMIT,
    })
    .expect("classify fault");
    assert_eq!(anomaly.kind, WallClockAnomalyKind::BackwardJump);

    let timer =
        schedule_timer(&profile.profile, GENERATION, 0, &one_shot(&profile.profile, GENERATION)).expect("timer");
    let delayed = poll_timer_with_fault(
        &timer,
        GENERATION,
        TIMER_DEADLINE,
        1,
        Some(&FabricTimeFault::DelayTimer {
            key: timer.key.clone(),
            ticks: TIMER_DELAY,
        }),
    )
    .expect("faulted poll");
    assert_eq!(delayed.action, TimerAction::NotDue);
}

#[test]
fn extension_context_enforces_service_generation_resources_and_capability() {
    let profile = simulation_profile();
    let context = ExtensionTimeContext::from_test_snapshot("test-service", GENERATION, &profile.profile, vec![
        HASH_B.to_string(),
    ]);
    assert!(context.schedule_timer(&profile.profile, 0, &one_shot(&profile.profile, GENERATION)).is_ok());
    assert!(
        context
            .schedule_timer(&profile.profile, PROFILE_LIMIT, &one_shot(&profile.profile, GENERATION),)
            .is_err()
    );
    assert!(context.schedule_timer(&profile.profile, 0, &one_shot(&profile.profile, STALE_GENERATION),).is_err());
    let mut substituted_profile = profile.profile.clone();
    substituted_profile.profile_id = "silent-fallback".to_string();
    let substituted = ExtensionTimeContext::from_test_snapshot("test-service", GENERATION, &substituted_profile, vec![
        HASH_B.to_string(),
    ]);
    assert!(substituted.schedule_timer(&profile.profile, 0, &one_shot(&profile.profile, GENERATION)).is_err());

    let denied = EntropyStreamRequest {
        profile_ref: profile.profile.profile_ref.clone(),
        stream_id: "stream".to_string(),
        purpose: "purpose".to_string(),
        capability_ref: HASH_A.to_string(),
        generation: GENERATION,
        mode: EntropyMode::DeterministicSimulation,
        explicit_simulation_seed: Some(1),
        explicit_simulation_seed_ref: Some(HASH_A.to_string()),
    };
    assert!(context.open_entropy_stream(&profile.profile, &denied).is_err());
    let admitted = EntropyStreamRequest {
        capability_ref: HASH_B.to_string(),
        ..denied
    };
    assert!(context.open_entropy_stream(&profile.profile, &admitted).is_ok());
    assert!(
        context
            .admit_entropy_request(EntropyRequest::Bytes {
                count: PROFILE_LIMIT + 1,
            })
            .is_err()
    );
}

#[test]
fn live_scheduler_wake_shell_routes_only_admitted_wake_transitions() {
    let profile = simulation_profile();
    let policy = SchedulerPolicy {
        ordering: SchedulerOrdering::Fifo,
        replay: SchedulerReplayPolicy::Deterministic,
        overload: SchedulerOverloadPolicy::Reject,
    };
    let state = new_scheduler_state(&profile.profile, GENERATION);
    let key = RunnableKey {
        service_id: "wake-service".to_string(),
        generation: GENERATION,
        runnable_id: "wake-runnable".to_string(),
    };
    let transition = apply_scheduler_command(&profile.profile, policy, &state, GENERATION, &SchedulerCommand::Wake {
        key: key.clone(),
        priority: 0,
    })
    .expect("wake transition");
    let mut adapter = ThreadSchedulerWakeAdapter::default();
    assert!(adapter.route(&transition).is_err());
    adapter.register(key.clone(), std::thread::current()).expect("register current thread");
    adapter.route(&transition).expect("route admitted wake");
    assert!(adapter.unregister(&key));
}

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

// r[verify molten.audit_f12.compatibility]
#[test]
fn retry_observation_denial_preserves_existing_events() {
    let profile = simulation_profile();
    let policy = RetryPolicy {
        maximum_attempts: 1,
        base_delay_ticks: TIMER_DELAY,
        maximum_delay_ticks: PROFILE_LIMIT,
        backoff: RetryBackoff::Fixed,
        jitter: RetryJitter::None,
    };
    let now = TimeValue::Virtual(VirtualInstant {
        profile_ref: profile.profile.profile_ref.clone(),
        ticks: TIMER_DEADLINE,
    });
    let mut events = Vec::new();
    events.extend(super::fixture::retry_events(&profile, &now, 0, policy, None).expect("fixed retry"));
    let original = events.clone();
    let error = super::fixture::retry_events(&profile, &now, 1, policy, None)
        .map(|batch| events.extend(batch))
        .expect_err("exhausted");
    assert!(error.to_string().contains("RetryExhausted"));
    assert_eq!(events, original);
    let error = super::fixture::retry_events(&profile, &now, 0, policy, Some(0))
        .map(|batch| events.extend(batch))
        .expect_err("jitter");
    assert!(error.to_string().contains("JitterOutOfBounds"));
    assert_eq!(events, original);
    let overflow = TimeValue::Virtual(VirtualInstant {
        profile_ref: profile.profile.profile_ref.clone(),
        ticks: u64::MAX,
    });
    let error = super::fixture::retry_events(&profile, &overflow, 0, policy, None)
        .map(|batch| events.extend(batch))
        .expect_err("overflow");
    assert!(error.to_string().contains("Overflow"));
    assert_eq!(events, original);
}

// r[verify molten.modularity.fabric_boundary.adapters.clock]
// r[verify molten.modularity.fabric_boundary.validation]
#[test]
fn live_adapter_rejects_simulation_profile() {
    let profile = simulation_profile();
    let error = LiveClockAdapter::new(&profile.profile, 0).expect_err("simulation profile cannot use live clock");
    assert!(error.to_string().contains("live time profile"));
}

// r[verify molten.fabric_time.live_sim_parity]
#[test]
fn tick_deadline_expires_exactly_at_its_timeout_on_the_virtual_clock() {
    let profile = simulation_profile();
    let mut clock = VirtualClockAdapter::new(&profile.profile, TIMER_DEADLINE, WALL_BASE).expect("virtual clock");
    let deadline = TickDeadline::after(&mut clock, TIMER_DELAY).expect("deadline inside the clock domain");

    clock
        .await_ticks(TIMER_DEADLINE + TIMER_DELAY - 1)
        .expect("advance to one tick before the deadline");
    assert_eq!(deadline.remaining_ticks(&mut clock).expect("remaining ticks"), 1);
    assert!(!deadline.is_expired(&mut clock).expect("deadline check"));

    clock.await_ticks(TIMER_DEADLINE + TIMER_DELAY).expect("advance to the deadline");
    assert_eq!(deadline.remaining_ticks(&mut clock).expect("remaining ticks"), 0);
    assert!(deadline.is_expired(&mut clock).expect("deadline check"));
}

#[test]
fn tick_deadline_denies_a_timeout_past_the_clock_domain() {
    let profile = simulation_profile();
    let mut clock = VirtualClockAdapter::new(&profile.profile, TIMER_DEADLINE, WALL_BASE).expect("virtual clock");
    assert!(TickDeadline::after(&mut clock, u64::MAX - TIMER_DEADLINE).is_ok());
    assert!(TickDeadline::after(&mut clock, u64::MAX - TIMER_DEADLINE + 1).is_err());
}

#[test]
fn supervision_deadline_admits_the_supervision_bound_and_denies_one_past() {
    const SUPERVISION_BOUND: std::time::Duration = std::time::Duration::from_secs(3_600);
    let mut deadline = SupervisionDeadline::after(SUPERVISION_BOUND).expect("the admitted supervision bound");
    assert!(!deadline.is_expired().expect("fresh deadline check"));
    assert!(deadline.remaining().expect("remaining time") <= SUPERVISION_BOUND);

    let one_past = SUPERVISION_BOUND + std::time::Duration::from_nanos(1);
    let error = SupervisionDeadline::after(one_past).expect_err("one nanosecond past the bound denies");
    assert!(error.to_string().contains("exceeds the admitted maximum"));

    let mut elapsed = SupervisionDeadline::after(std::time::Duration::ZERO).expect("zero timeout");
    assert!(elapsed.is_expired().expect("zero timeout is already expired"));
}

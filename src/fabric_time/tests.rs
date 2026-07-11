use super::*;
use crate::fabric::DeterminismClass;
use crate::fabric::FabricAuthority;
use crate::fabric::FabricPortClass;
use crate::fabric::FabricPortRequirement;
use crate::fabric::FabricResource;
use crate::fabric::ReplayClass;
use crate::fabric::build_fabric_port_registry;
use crate::fabric::resolve_fabric_port_binding;

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
    let registry = build_fabric_port_registry(&descriptors).expect("time registry");
    let clock = &descriptors[0];
    let mut requirement = FabricPortRequirement {
        port_id: clock.port_id.clone(),
        version: clock.version.clone(),
        class: FabricPortClass::Time,
        operation_classes: clock.operation_classes.clone(),
        input_schema_refs: clock.input_schema_refs.clone(),
        output_schema_refs: clock.output_schema_refs.clone(),
        allowed_authorities: vec![FabricAuthority::Time],
        available_resources: vec![FabricResource::LogicalTime],
        expected_determinism: DeterminismClass::DeterministicWithRecordedInputs,
        expected_replay: ReplayClass::Recompute,
        expected_profile: "unadmitted-fallback".to_string(),
    };
    assert!(resolve_fabric_port_binding(&registry, &requirement).is_err());
    requirement.expected_profile = profile.profile.profile_id.clone();
    assert!(resolve_fabric_port_binding(&registry, &requirement).is_ok());
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
    use crate::fabric::ExtensionTier;
    use crate::fabric::ExtensionTierRequest;
    use crate::fabric::FabricAuthority;
    use crate::fabric::REQUIRED_SYSTEM_EXTENSION_EVIDENCE;
    use crate::fabric::validate_extension_tier;

    let plugin = ExtensionTierRequest {
        tier: ExtensionTier::SandboxedPlugin,
        requested_authorities: vec![FabricAuthority::Time],
        admission_evidence: Vec::new(),
    };
    assert!(validate_extension_tier(&plugin).is_err());

    let extension = ExtensionTierRequest {
        tier: ExtensionTier::SystemExtension,
        requested_authorities: vec![FabricAuthority::Time, FabricAuthority::Scheduling],
        admission_evidence: REQUIRED_SYSTEM_EXTENSION_EVIDENCE.to_vec(),
    };
    assert!(validate_extension_tier(&extension).is_ok());

    let application = ExtensionTierRequest {
        tier: ExtensionTier::ApplicationWorkload,
        requested_authorities: vec![FabricAuthority::ApplicationServiceUse],
        admission_evidence: Vec::new(),
    };
    assert!(validate_extension_tier(&application).is_ok());
}

#[test]
fn live_adapter_rejects_simulation_profile() {
    let profile = simulation_profile();
    let error = LiveClockAdapter::new(&profile.profile, 0).expect_err("simulation profile cannot use live clock");
    assert!(error.to_string().contains("live time profile"));
}

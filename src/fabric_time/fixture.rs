use super::*;
use crate::error::MoltenError;
use crate::error::Result;
use crate::fabric::build_fabric_port_registry;
use crate::fabric::canonical_fabric_port_descriptor;

const HASH_A: &str = "blake3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const HASH_B: &str = "blake3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const HASH_C: &str = "blake3:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
const FIXTURE_SERVICE_ID: &str = "molten.fixture.fabric-time.service";
const FIXTURE_GENERATION: u64 = 1;
const PROFILE_MAX_TICKS: u64 = 1_000_000_000;
const PROFILE_MAX_TIMERS: u64 = 64;
const PROFILE_MAX_RUNNABLES: u64 = 64;
const PROFILE_MAX_ENTROPY_REQUEST: u64 = 1_024;
const PROFILE_MAX_ENTROPY_TOTAL: u64 = 65_536;
const PROFILE_MAX_CONCURRENCY: u64 = 8;
const PROFILE_MAX_QUEUE: u64 = 16;
const PROFILE_FAIRNESS_TURNS: u64 = 4;
const WALL_BASE_NANOS: u64 = 1_700_000_000_000_000_000;
const TIMER_DEADLINE: u64 = 10;
const TIMER_PERIOD: u64 = 10;
const TIMER_OBSERVATION: u64 = 35;
const TIMER_DELAY_FAULT: u64 = 5;
const PERIODIC_TIMER_SEQUENCE: u64 = 10;
const DELAYED_TIMER_SEQUENCE: u64 = 11;
const DROPPED_TIMER_SEQUENCE: u64 = 12;
const CANCELLED_TIMER_SEQUENCE: u64 = 13;
const ACTIVE_TIMER_SLOTS_AFTER_PERIODIC: u64 = 1;
const ACTIVE_TIMER_SLOTS_AFTER_DELAYED: u64 = 2;
const ACTIVE_TIMER_SLOTS_BEFORE_CANCELLATION: u64 = 3;
const WALL_JUMP_FAULT: u64 = 25;
const ENTROPY_SEED: u64 = 0xA5A5_5A5A_A5A5_5A5A;
const ENTROPY_BYTE_COUNT: u64 = 16;
const ENTROPY_CHOICE_BOUND: u64 = 11;
const DEADLINE_TARGET: u64 = 50;
const DEADLINE_OBSERVATION: u64 = 40;
const LEASE_EXPIRY: u64 = 60;
const FENCING_TOKEN: u64 = 2;
const PREVIOUS_FENCING_TOKEN: u64 = 1;
const RETRY_ATTEMPTS: u64 = 3;
const RETRY_BASE: u64 = 5;
const RETRY_MAXIMUM: u64 = 20;
const RETRY_JITTER: u64 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FabricTimeFixtureSelection {
    Live,
    DeterministicSimulation,
    Both,
}

impl FabricTimeFixtureSelection {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Live => "live",
            Self::DeterministicSimulation => "deterministic-simulation",
            Self::Both => "both",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutableFabricTimeFixtureRun {
    pub selection: FabricTimeFixtureSelection,
    pub live_profile: CanonicalTimeProfile,
    pub simulation_profile: CanonicalTimeProfile,
    pub port_descriptor_refs: Vec<String>,
    pub live_conformance: AdapterConformanceObservation,
    pub simulation_conformance: AdapterConformanceObservation,
    pub events: Vec<CanonicalTimeEvent>,
    pub report: CanonicalFabricTimeRun,
    pub production_entropy_source: String,
}

// r[impl molten.fabric_time.live_sim_parity]
// r[impl molten.fabric_time.final_validation]
pub fn run_executable_fabric_time_fixture(
    selection: FabricTimeFixtureSelection,
) -> Result<ExecutableFabricTimeFixtureRun> {
    let live_profile =
        canonical_admit_time_profile(&fixture_profile("molten.fabric-time.live", HASH_A, TimeProfileKind::Live, None))?;
    let simulation_profile = canonical_admit_time_profile(&fixture_profile(
        "molten.fabric-time.simulation",
        HASH_B,
        TimeProfileKind::DeterministicSimulation,
        Some(PROFILE_FAIRNESS_TURNS),
    ))?;
    let port_descriptor_refs = validate_fixture_ports(&live_profile, &simulation_profile)?;

    let mut live_clock = LiveClockAdapter::new(&live_profile.profile, PROFILE_MAX_TICKS)?;
    let first_live_wall = live_clock.observe_wall()?;
    let live_conformance =
        run_timer_adapter_conformance(&live_profile.profile, &mut live_clock, FIXTURE_SERVICE_ID, FIXTURE_GENERATION)?;
    let second_live_wall = live_clock.observe_wall()?;
    let live_wall_decision =
        classify_wall_clock_observation(&first_live_wall, &second_live_wall, WallClockAnomalyPolicy {
            max_forward_jump_nanos: u64::MAX,
            max_uncertainty_nanos: PROFILE_MAX_TICKS,
        })
        .map_err(|error| core_error("classify live wall clock", error))?;

    let mut virtual_clock = VirtualClockAdapter::new(&simulation_profile.profile, 0, WALL_BASE_NANOS)?;
    let simulation_conformance = run_timer_adapter_conformance(
        &simulation_profile.profile,
        &mut virtual_clock,
        FIXTURE_SERVICE_ID,
        FIXTURE_GENERATION,
    )?;
    ensure_shared_conformance(&live_conformance, &simulation_conformance)?;

    let live_final_ticks = live_clock.now_ticks()?;
    let simulation_initial_ticks = virtual_clock.now_ticks()?;
    let live_initial = canonical_named_event(
        &live_profile.profile_ref,
        CanonicalTimeEventKind::Conformance,
        FIXTURE_GENERATION,
        "live-run-state",
        "initialized",
        live_final_ticks,
    )?;
    let simulation_initial = canonical_named_event(
        &simulation_profile.profile_ref,
        CanonicalTimeEventKind::Conformance,
        FIXTURE_GENERATION,
        "simulation-run-state",
        "initialized",
        simulation_initial_ticks,
    )?;
    let mut events = vec![live_initial.clone(), simulation_initial.clone()];
    events.push(canonical_named_event(
        &live_profile.profile_ref,
        CanonicalTimeEventKind::Conformance,
        FIXTURE_GENERATION,
        "live-adapter",
        "passed",
        live_final_ticks,
    )?);
    events.push(canonical_named_event(
        &simulation_profile.profile_ref,
        CanonicalTimeEventKind::Conformance,
        FIXTURE_GENERATION,
        "simulation-adapter",
        "passed",
        simulation_initial_ticks,
    )?);
    events.push(canonical_clock_anomaly_event(&live_profile.profile_ref, FIXTURE_GENERATION, &live_wall_decision)?);
    events.push(run_live_scheduler_scenario(&live_profile)?);

    let _counters = run_simulation_scenarios(&simulation_profile, &mut virtual_clock, &mut events)?;
    let production_entropy_source = run_production_entropy_scenario(&live_profile, &mut events)?;
    let live_terminal = canonical_named_event(
        &live_profile.profile_ref,
        CanonicalTimeEventKind::Conformance,
        FIXTURE_GENERATION,
        "live-run-state",
        "completed",
        live_final_ticks,
    )?;
    let simulation_terminal = canonical_named_event(
        &simulation_profile.profile_ref,
        CanonicalTimeEventKind::Conformance,
        FIXTURE_GENERATION,
        "simulation-run-state",
        "completed",
        virtual_clock.now_ticks()?,
    )?;
    events.push(live_terminal.clone());
    events.push(simulation_terminal.clone());

    let selected_events = events
        .iter()
        .filter(|event| match selection {
            FabricTimeFixtureSelection::Live => event.profile_ref == live_profile.profile_ref,
            FabricTimeFixtureSelection::DeterministicSimulation => event.profile_ref == simulation_profile.profile_ref,
            FabricTimeFixtureSelection::Both => true,
        })
        .collect::<Vec<_>>();
    let evidence_refs = selected_events.iter().map(|event| event.evidence_ref.clone()).collect::<Vec<_>>();
    let (profile_ref, final_time_ticks) = match selection {
        FabricTimeFixtureSelection::Live => (live_profile.profile_ref.clone(), live_final_ticks),
        FabricTimeFixtureSelection::DeterministicSimulation | FabricTimeFixtureSelection::Both => {
            (simulation_profile.profile_ref.clone(), virtual_clock.now_ticks()?)
        }
    };
    let initial_state_ref =
        select_boundary_ref(selection, &live_initial.evidence_ref, &simulation_initial.evidence_ref, "initial-state")?;
    let terminal_outcome_ref = select_boundary_ref(
        selection,
        &live_terminal.evidence_ref,
        &simulation_terminal.evidence_ref,
        "terminal-outcome",
    )?;
    let scheduler_trace_ref =
        trace_for_kinds(&selected_events, &[CanonicalTimeEventKind::Scheduler], "scheduler-choice-trace")?;
    let entropy_trace_ref =
        trace_for_kinds(&selected_events, &[CanonicalTimeEventKind::Entropy], "entropy-stream-trace")?;
    let fault_plan_ref = trace_for_kinds(
        &selected_events,
        &[CanonicalTimeEventKind::Fault, CanonicalTimeEventKind::ClockAnomaly],
        "fault-plan",
    )?;
    let report = canonical_fabric_time_run(FabricTimeRunReport {
        profile_ref,
        profile_kind: selection.as_str().to_string(),
        generation: FIXTURE_GENERATION,
        initial_state_ref,
        scheduler_trace_ref,
        entropy_trace_ref,
        fault_plan_ref,
        terminal_outcome_ref,
        final_time_ticks,
        timer_events: count_events(&selected_events, &[CanonicalTimeEventKind::Timer])?,
        scheduler_events: count_events(&selected_events, &[CanonicalTimeEventKind::Scheduler])?,
        entropy_events: count_events(&selected_events, &[CanonicalTimeEventKind::Entropy])?,
        deadline_lease_events: count_events(&selected_events, &[
            CanonicalTimeEventKind::Deadline,
            CanonicalTimeEventKind::Lease,
        ])?,
        fault_events: count_events(&selected_events, &[CanonicalTimeEventKind::Fault])?,
        live_clock_observed: selection != FabricTimeFixtureSelection::DeterministicSimulation,
        shared_conformance_passed: true,
        evidence_refs,
        non_claims: REQUIRED_TIME_NON_CLAIMS.to_vec(),
    })?;

    Ok(ExecutableFabricTimeFixtureRun {
        selection,
        live_profile,
        simulation_profile,
        port_descriptor_refs,
        live_conformance,
        simulation_conformance,
        events,
        report,
        production_entropy_source,
    })
}

fn run_live_scheduler_scenario(profile: &CanonicalTimeProfile) -> Result<CanonicalTimeEvent> {
    let key = RunnableKey {
        service_id: FIXTURE_SERVICE_ID.to_string(),
        generation: FIXTURE_GENERATION,
        runnable_id: "live-wakeup".to_string(),
    };
    let state = new_scheduler_state(&profile.profile, FIXTURE_GENERATION);
    let woken = apply_scheduler_command(
        &profile.profile,
        profile.profile.scheduler_policy,
        &state,
        FIXTURE_GENERATION,
        &SchedulerCommand::Wake {
            key: key.clone(),
            priority: 0,
        },
    )
    .map_err(|error| core_error("wake live fixture runnable", error))?;
    let parked = std::thread::spawn(std::thread::park);
    let mut wake_adapter = ThreadSchedulerWakeAdapter::default();
    wake_adapter.register(key.clone(), parked.thread().clone())?;
    wake_adapter.route(&woken)?;
    parked.join().map_err(|_| MoltenError::invalid_harness("live scheduler wake target panicked"))?;
    if !wake_adapter.unregister(&key) {
        return Err(MoltenError::invalid_harness("live scheduler wake target cleanup failed"));
    }
    let selected = choose_runnable(
        &profile.profile,
        profile.profile.scheduler_policy,
        &woken.next,
        FIXTURE_GENERATION,
        Some(&key),
    )
    .map_err(|error| core_error("select live fixture runnable", error))?;
    canonical_scheduler_selection(&profile.profile_ref, &selected)
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct ScenarioCounters {
    timer_events: u64,
    scheduler_events: u64,
    entropy_events: u64,
    deadline_lease_events: u64,
    fault_events: u64,
}

fn run_simulation_scenarios(
    profile: &CanonicalTimeProfile,
    clock: &mut VirtualClockAdapter,
    events: &mut Vec<CanonicalTimeEvent>,
) -> Result<ScenarioCounters> {
    let mut counters = ScenarioCounters::default();
    let periodic = schedule_timer(
        &profile.profile,
        FIXTURE_GENERATION,
        0,
        &timer_request(
            &profile.profile,
            PERIODIC_TIMER_SEQUENCE,
            TimerKind::Periodic {
                period_ticks: TIMER_PERIOD,
            },
            TimerOverloadPolicy::RejectAndRetain,
        ),
    )
    .map_err(|error| core_error("schedule periodic fixture timer", error))?;
    clock.await_ticks(TIMER_OBSERVATION)?;
    let periodic_transition = poll_timer(&periodic, FIXTURE_GENERATION, clock.now_ticks()?, 1)
        .map_err(|error| core_error("poll periodic fixture timer", error))?;
    events.push(canonical_timer_event(&profile.profile_ref, &periodic_transition)?);
    counters.timer_events = checked_increment(counters.timer_events, "timer event count")?;

    let delayed = schedule_timer(
        &profile.profile,
        FIXTURE_GENERATION,
        ACTIVE_TIMER_SLOTS_AFTER_PERIODIC,
        &timer_request(
            &profile.profile,
            DELAYED_TIMER_SEQUENCE,
            TimerKind::OneShot,
            TimerOverloadPolicy::RejectAndRetain,
        ),
    )
    .map_err(|error| core_error("schedule delayed fixture timer", error))?;
    let delay_fault = FabricTimeFault::DelayTimer {
        key: delayed.key.clone(),
        ticks: TIMER_DELAY_FAULT,
    };
    let delayed_transition =
        poll_timer_with_fault(&delayed, FIXTURE_GENERATION, delayed.next_deadline_ticks, 1, Some(&delay_fault))?;
    events.push(canonical_timer_event(&profile.profile_ref, &delayed_transition)?);
    events.push(canonical_named_event(
        &profile.profile_ref,
        CanonicalTimeEventKind::Fault,
        FIXTURE_GENERATION,
        "timer-delay",
        "injected",
        TIMER_DELAY_FAULT,
    )?);
    counters.timer_events = checked_increment(counters.timer_events, "timer event count")?;
    counters.fault_events = checked_increment(counters.fault_events, "fault event count")?;

    let dropped = schedule_timer(
        &profile.profile,
        FIXTURE_GENERATION,
        ACTIVE_TIMER_SLOTS_AFTER_DELAYED,
        &timer_request(&profile.profile, DROPPED_TIMER_SEQUENCE, TimerKind::OneShot, TimerOverloadPolicy::DropDue),
    )
    .map_err(|error| core_error("schedule dropped fixture timer", error))?;
    let drop_fault = FabricTimeFault::DropTimerDelivery {
        key: dropped.key.clone(),
    };
    let dropped_transition =
        poll_timer_with_fault(&dropped, FIXTURE_GENERATION, dropped.next_deadline_ticks, 1, Some(&drop_fault))?;
    events.push(canonical_timer_event(&profile.profile_ref, &dropped_transition)?);
    events.push(canonical_named_event(
        &profile.profile_ref,
        CanonicalTimeEventKind::Fault,
        FIXTURE_GENERATION,
        "timer-drop",
        "injected-and-recorded",
        dropped.next_deadline_ticks,
    )?);
    counters.timer_events = checked_increment(counters.timer_events, "timer event count")?;
    counters.fault_events = checked_increment(counters.fault_events, "fault event count")?;

    let cancellable = schedule_timer(
        &profile.profile,
        FIXTURE_GENERATION,
        ACTIVE_TIMER_SLOTS_BEFORE_CANCELLATION,
        &timer_request(
            &profile.profile,
            CANCELLED_TIMER_SEQUENCE,
            TimerKind::OneShot,
            TimerOverloadPolicy::RejectAndRetain,
        ),
    )
    .map_err(|error| core_error("schedule cancellation fault timer", error))?;
    let cancellation_fault = FabricTimeFault::CancelTimer {
        key: cancellable.key.clone(),
    };
    let cancelled = poll_timer_with_fault(
        &cancellable,
        FIXTURE_GENERATION,
        cancellable.next_deadline_ticks,
        1,
        Some(&cancellation_fault),
    )?;
    events.push(canonical_timer_event(&profile.profile_ref, &cancelled)?);
    events.push(canonical_named_event(
        &profile.profile_ref,
        CanonicalTimeEventKind::Fault,
        FIXTURE_GENERATION,
        "timer-cancellation-race",
        "cancellation-won",
        cancellable.next_deadline_ticks,
    )?);
    counters.timer_events = checked_increment(counters.timer_events, "timer event count")?;
    counters.fault_events = checked_increment(counters.fault_events, "fault event count")?;

    let cleaned = cleanup_generation(&[periodic_transition.next, delayed_transition.next], FIXTURE_GENERATION);
    if cleaned.iter().any(|timer| timer.phase != TimerPhase::Cancelled) {
        return Err(MoltenError::invalid_harness("fixture generation cleanup leaked an active timer"));
    }
    events.push(canonical_named_event(
        &profile.profile_ref,
        CanonicalTimeEventKind::Timer,
        FIXTURE_GENERATION,
        "timer-generation-cleanup",
        "no-leaks",
        clock.now_ticks()?,
    )?);
    counters.timer_events = checked_increment(counters.timer_events, "timer event count")?;

    run_scheduler_scenario(profile, events, &mut counters)?;
    run_deterministic_entropy_scenario(profile, events, &mut counters)?;
    run_deadline_lease_scenario(profile, events, &mut counters)?;
    run_clock_partition_faults(profile, clock, events, &mut counters)?;
    Ok(counters)
}

fn run_scheduler_scenario(
    profile: &CanonicalTimeProfile,
    events: &mut Vec<CanonicalTimeEvent>,
    counters: &mut ScenarioCounters,
) -> Result<()> {
    let policy = SchedulerPolicy {
        ordering: SchedulerOrdering::PriorityThenFifo,
        replay: SchedulerReplayPolicy::Deterministic,
        overload: SchedulerOverloadPolicy::Reject,
    };
    let mut state = new_scheduler_state(&profile.profile, FIXTURE_GENERATION);
    let low = runnable_key("low");
    let high = runnable_key("high");
    for (key, priority) in [(low.clone(), 0), (high.clone(), 1)] {
        let transition =
            apply_scheduler_command(&profile.profile, policy, &state, FIXTURE_GENERATION, &SchedulerCommand::Wake {
                key,
                priority,
            })
            .map_err(|error| core_error("wake fixture runnable", error))?;
        events.push(canonical_scheduler_transition(&profile.profile_ref, &transition)?);
        state = transition.next;
        counters.scheduler_events = checked_increment(counters.scheduler_events, "scheduler event count")?;
    }
    let selection = choose_runnable(&profile.profile, policy, &state, FIXTURE_GENERATION, Some(&high))
        .map_err(|error| core_error("select fixture runnable", error))?;
    let replay = choose_runnable(&profile.profile, policy, &state, FIXTURE_GENERATION, Some(&selection.selected))
        .map_err(|error| core_error("replay fixture selection", error))?;
    if replay.selected != selection.selected {
        return Err(MoltenError::invalid_harness("deterministic scheduler replay selected a different runnable"));
    }
    if !matches!(
        choose_runnable(&profile.profile, policy, &state, FIXTURE_GENERATION, Some(&low),),
        Err(SchedulerError::UnexpectedReplayChoice { .. })
    ) {
        return Err(MoltenError::invalid_harness("fixture scheduler accepted a divergent replay choice"));
    }
    events.push(canonical_named_event(
        &profile.profile_ref,
        CanonicalTimeEventKind::Scheduler,
        FIXTURE_GENERATION,
        "scheduler-replay",
        "divergence-detected",
        state.choice_sequence,
    )?);
    counters.scheduler_events = checked_increment(counters.scheduler_events, "scheduler event count")?;
    events.push(canonical_scheduler_selection(&profile.profile_ref, &selection)?);
    counters.scheduler_events = checked_increment(counters.scheduler_events, "scheduler event count")?;

    let yielded = apply_scheduler_command(
        &profile.profile,
        policy,
        &selection.next,
        FIXTURE_GENERATION,
        &SchedulerCommand::Yield {
            key: selection.selected,
        },
    )
    .map_err(|error| core_error("yield fixture runnable", error))?;
    events.push(canonical_scheduler_transition(&profile.profile_ref, &yielded)?);
    counters.scheduler_events = checked_increment(counters.scheduler_events, "scheduler event count")?;

    let saturation_fault = FabricTimeFault::SaturateSchedulerQueue;
    let mut saturated = new_scheduler_state(&profile.profile, FIXTURE_GENERATION);
    for index in 0..profile.profile.max_scheduler_queue_depth {
        saturated = apply_scheduler_command(
            &profile.profile,
            policy,
            &saturated,
            FIXTURE_GENERATION,
            &SchedulerCommand::Wake {
                key: runnable_key(&format!("saturated-{index}")),
                priority: 0,
            },
        )
        .map_err(|error| core_error("saturate fixture scheduler", error))?
        .next;
    }
    let overload =
        apply_scheduler_command(&profile.profile, policy, &saturated, FIXTURE_GENERATION, &SchedulerCommand::Wake {
            key: runnable_key("saturated-overflow"),
            priority: 0,
        })
        .map_err(|error| core_error("probe saturated fixture scheduler", error))?;
    validate_scheduler_fault_outcome(&saturation_fault, &overload)?;
    events.push(canonical_scheduler_transition(&profile.profile_ref, &overload)?);
    events.push(canonical_named_event(
        &profile.profile_ref,
        CanonicalTimeEventKind::Fault,
        FIXTURE_GENERATION,
        "scheduler-queue",
        "saturated",
        profile.profile.max_scheduler_queue_depth,
    )?);
    counters.scheduler_events = checked_increment(counters.scheduler_events, "scheduler event count")?;
    counters.fault_events = checked_increment(counters.fault_events, "fault event count")?;
    Ok(())
}

fn run_deterministic_entropy_scenario(
    profile: &CanonicalTimeProfile,
    events: &mut Vec<CanonicalTimeEvent>,
    counters: &mut ScenarioCounters,
) -> Result<()> {
    let mut stream = open_entropy_stream(
        &profile.profile,
        FIXTURE_GENERATION,
        &entropy_stream_request(
            &profile.profile,
            "simulation-stream",
            "scheduler-choice",
            EntropyMode::DeterministicSimulation,
            Some(ENTROPY_SEED),
        ),
    )
    .map_err(|error| core_error("open deterministic entropy stream", error))?;
    for request in [
        EntropyRequest::Bytes {
            count: ENTROPY_BYTE_COUNT,
        },
        EntropyRequest::BoundedChoice {
            upper_exclusive: ENTROPY_CHOICE_BOUND,
        },
    ] {
        let transition = draw_deterministic_entropy(&profile.profile, FIXTURE_GENERATION, &stream, request)
            .map_err(|error| core_error("draw deterministic entropy", error))?;
        let metadata = entropy_evidence_metadata(&stream, &transition);
        events.push(canonical_entropy_event(&metadata)?);
        stream = transition.next;
        counters.entropy_events = checked_increment(counters.entropy_events, "entropy event count")?;
    }
    Ok(())
}

fn run_production_entropy_scenario(
    profile: &CanonicalTimeProfile,
    events: &mut Vec<CanonicalTimeEvent>,
) -> Result<String> {
    let stream = open_entropy_stream(
        &profile.profile,
        FIXTURE_GENERATION,
        &entropy_stream_request(
            &profile.profile,
            "production-stream",
            "session-token",
            EntropyMode::ProductionCryptographic,
            None,
        ),
    )
    .map_err(|error| core_error("open production entropy stream", error))?;
    let mut adapter = ProductionEntropyAdapter::new(OperatingSystemEntropySource);
    let (_, metadata) = adapter.draw(&profile.profile, FIXTURE_GENERATION, &stream, EntropyRequest::Bytes {
        count: ENTROPY_BYTE_COUNT,
    })?;
    events.push(canonical_entropy_event(&metadata)?);
    Ok(adapter.source_id().to_string())
}

fn run_deadline_lease_scenario(
    profile: &CanonicalTimeProfile,
    events: &mut Vec<CanonicalTimeEvent>,
    counters: &mut ScenarioCounters,
) -> Result<()> {
    let target = virtual_value(&profile.profile, DEADLINE_TARGET);
    let deadline = Deadline {
        profile_ref: profile.profile.profile_ref.clone(),
        subject_id: "fixture-deadline".to_string(),
        generation: FIXTURE_GENERATION,
        target,
        uncertainty_ticks: 1,
    };
    let decision = evaluate_deadline(
        &profile.profile,
        FIXTURE_GENERATION,
        &deadline,
        &virtual_value(&profile.profile, DEADLINE_OBSERVATION),
    )
    .map_err(|error| core_error("evaluate fixture deadline", error))?;
    events.push(canonical_deadline_event(&profile.profile_ref, &decision)?);
    counters.deadline_lease_events = checked_increment(counters.deadline_lease_events, "deadline/lease event count")?;

    let retry = plan_retry(
        &profile.profile,
        FIXTURE_GENERATION,
        "fixture-retry",
        FIXTURE_GENERATION,
        &virtual_value(&profile.profile, DEADLINE_OBSERVATION),
        1,
        RetryPolicy {
            maximum_attempts: RETRY_ATTEMPTS,
            base_delay_ticks: RETRY_BASE,
            maximum_delay_ticks: RETRY_MAXIMUM,
            backoff: RetryBackoff::Exponential,
            jitter: RetryJitter::Bounded {
                maximum_ticks: RETRY_JITTER,
            },
        },
        Some(RETRY_JITTER),
    )
    .map_err(|error| core_error("plan fixture retry", error))?;
    events.push(canonical_named_event(
        &profile.profile_ref,
        CanonicalTimeEventKind::Deadline,
        FIXTURE_GENERATION,
        "fixture-retry",
        "retry-planned",
        retry.deadline.target.ticks(),
    )?);
    counters.deadline_lease_events = checked_increment(counters.deadline_lease_events, "deadline/lease event count")?;

    let lease = evaluate_lease(&profile.profile, FIXTURE_GENERATION, &LeaseRequest {
        lease_id: "fixture-lease".to_string(),
        owner_id: "fixture-owner".to_string(),
        generation: FIXTURE_GENERATION,
        now: virtual_value(&profile.profile, DEADLINE_OBSERVATION),
        expires_at: virtual_value(&profile.profile, LEASE_EXPIRY),
        uncertainty_ticks: 0,
        consistency: LeaseConsistency::FencedExclusive,
        action: LeaseAction::AcquireExclusive,
        fencing_token: Some(FENCING_TOKEN),
        previous_fencing_token: Some(PREVIOUS_FENCING_TOKEN),
    })
    .map_err(|error| core_error("evaluate fixture lease", error))?;
    events.push(canonical_lease_event(&profile.profile_ref, &lease)?);
    counters.deadline_lease_events = checked_increment(counters.deadline_lease_events, "deadline/lease event count")?;
    Ok(())
}

fn run_clock_partition_faults(
    profile: &CanonicalTimeProfile,
    clock: &mut VirtualClockAdapter,
    events: &mut Vec<CanonicalTimeEvent>,
    counters: &mut ScenarioCounters,
) -> Result<()> {
    let previous = clock.observe_wall()?;
    let backward = FabricTimeFault::BackwardWallJump { ticks: WALL_JUMP_FAULT };
    if !apply_clock_fault(clock, &backward)? {
        return Err(MoltenError::invalid_harness("backward clock fault was not applied"));
    }
    clock.advance(1)?;
    let observed = clock.observe_wall()?;
    let anomaly = classify_wall_clock_observation(&previous, &observed, WallClockAnomalyPolicy {
        max_forward_jump_nanos: PROFILE_MAX_TICKS,
        max_uncertainty_nanos: PROFILE_MAX_TICKS,
    })
    .map_err(|error| core_error("classify injected wall jump", error))?;
    events.push(canonical_clock_anomaly_event(&profile.profile_ref, FIXTURE_GENERATION, &anomaly)?);
    events.push(canonical_named_event(
        &profile.profile_ref,
        CanonicalTimeEventKind::Fault,
        FIXTURE_GENERATION,
        "wall-clock",
        "backward-jump-injected",
        WALL_JUMP_FAULT,
    )?);
    counters.fault_events = checked_increment(counters.fault_events, "fault event count")?;

    let partition_until = clock
        .now_ticks()?
        .checked_add(TIMER_PERIOD)
        .ok_or_else(|| MoltenError::invalid_harness("partition deadline overflow"))?;
    let partition = FabricTimeFault::PartitionWindow {
        until_ticks: partition_until,
    };
    let partition_deadline_ticks = partition_until
        .checked_add(TIMER_PERIOD)
        .ok_or_else(|| MoltenError::invalid_harness("partition-coupled deadline overflow"))?;
    let partition_decision = evaluate_deadline_with_fault(
        &profile.profile,
        FIXTURE_GENERATION,
        &Deadline {
            profile_ref: profile.profile.profile_ref.clone(),
            subject_id: "partition-coupled-deadline".to_string(),
            generation: FIXTURE_GENERATION,
            target: virtual_value(&profile.profile, partition_deadline_ticks),
            uncertainty_ticks: 0,
        },
        &virtual_value(&profile.profile, clock.now_ticks()?),
        Some(&partition),
    )?;
    if !matches!(partition_decision, FaultedDeadlineDecision::PartitionIndeterminate { .. }) {
        return Err(MoltenError::invalid_harness("partition fault did not make the coupled deadline indeterminate"));
    }
    events.push(canonical_named_event(
        &profile.profile_ref,
        CanonicalTimeEventKind::Fault,
        FIXTURE_GENERATION,
        "partition-window",
        "deadline-indeterminate",
        partition_until,
    )?);
    events.push(canonical_named_event(
        &profile.profile_ref,
        CanonicalTimeEventKind::Deadline,
        FIXTURE_GENERATION,
        "partition-coupled-deadline",
        "indeterminate-during-partition",
        partition_deadline_ticks,
    )?);
    counters.fault_events = checked_increment(counters.fault_events, "fault event count")?;
    counters.deadline_lease_events = checked_increment(counters.deadline_lease_events, "deadline/lease event count")?;
    Ok(())
}

fn fixture_profile(
    profile_id: &str,
    profile_ref: &str,
    kind: TimeProfileKind,
    fairness_bound_turns: Option<u64>,
) -> TimeProfileDescriptor {
    let replay = match kind {
        TimeProfileKind::Live => SchedulerReplayPolicy::RecordedChoiceRequired,
        TimeProfileKind::DeterministicSimulation => SchedulerReplayPolicy::Deterministic,
    };
    TimeProfileDescriptor {
        schema: FABRIC_TIME_PROFILE_SCHEMA.to_string(),
        profile_id: profile_id.to_string(),
        profile_ref: profile_ref.to_string(),
        kind,
        supported_domains: REQUIRED_TIME_DOMAINS.to_vec(),
        max_duration_ticks: PROFILE_MAX_TICKS,
        max_uncertainty_ticks: PROFILE_MAX_TICKS,
        max_timers: PROFILE_MAX_TIMERS,
        max_runnables: PROFILE_MAX_RUNNABLES,
        max_entropy_request_bytes: PROFILE_MAX_ENTROPY_REQUEST,
        max_entropy_total_bytes: PROFILE_MAX_ENTROPY_TOTAL,
        max_scheduler_concurrency: PROFILE_MAX_CONCURRENCY,
        max_scheduler_queue_depth: PROFILE_MAX_QUEUE,
        fairness_bound_turns,
        scheduler_policy: SchedulerPolicy {
            ordering: SchedulerOrdering::PriorityThenFifo,
            replay,
            overload: SchedulerOverloadPolicy::Reject,
        },
        evidence_mode: TimeEvidenceMode::SelectedSemanticBoundaries,
        non_claims: REQUIRED_TIME_NON_CLAIMS.to_vec(),
    }
}

fn validate_fixture_ports(live: &CanonicalTimeProfile, simulation: &CanonicalTimeProfile) -> Result<Vec<String>> {
    let mut refs = Vec::new();
    for profile in [live, simulation] {
        let descriptors = fabric_time_port_descriptors(profile);
        build_fabric_port_registry(&descriptors).map_err(|issues| core_error("validate fixture time ports", issues))?;
        for descriptor in &descriptors {
            let (descriptor_ref, _) = canonical_fabric_port_descriptor(descriptor)?;
            refs.push(descriptor_ref);
        }
    }
    Ok(refs)
}

fn ensure_shared_conformance(
    live: &AdapterConformanceObservation,
    simulation: &AdapterConformanceObservation,
) -> Result<()> {
    if live.timer_action != simulation.timer_action
        || live.delivery_count != simulation.delivery_count
        || live.stale_generation_discarded != simulation.stale_generation_discarded
        || live.cancellation_prevented_delivery != simulation.cancellation_prevented_delivery
        || live.scheduler_selected != simulation.scheduler_selected
        || live.scheduler_cancellation_recorded != simulation.scheduler_cancellation_recorded
        || live.entropy_bound_rejected != simulation.entropy_bound_rejected
    {
        return Err(MoltenError::invalid_harness(format!(
            "live and simulation adapters diverged: live={live:?} simulation={simulation:?}"
        )));
    }
    Ok(())
}

fn timer_request(
    profile: &AdmittedTimeProfile,
    sequence: u64,
    kind: TimerKind,
    overload: TimerOverloadPolicy,
) -> TimerScheduleRequest {
    TimerScheduleRequest {
        profile_ref: profile.profile_ref.clone(),
        key: TimerKey {
            service_id: FIXTURE_SERVICE_ID.to_string(),
            generation: FIXTURE_GENERATION,
            sequence,
        },
        domain: TimeDomain::Virtual,
        deadline_ticks: TIMER_DEADLINE,
        kind,
        ordering_key: sequence,
        coalescing: TimerCoalescingPolicy::CoalesceLatest,
        lateness: TimerLatenessPolicy::DeliverRegardless,
        overload,
        resource_charge: TimerResourceCharge::single_slot(),
    }
}

fn runnable_key(runnable_id: &str) -> RunnableKey {
    RunnableKey {
        service_id: FIXTURE_SERVICE_ID.to_string(),
        generation: FIXTURE_GENERATION,
        runnable_id: runnable_id.to_string(),
    }
}

fn entropy_stream_request(
    profile: &AdmittedTimeProfile,
    stream_id: &str,
    purpose: &str,
    mode: EntropyMode,
    explicit_simulation_seed: Option<u64>,
) -> EntropyStreamRequest {
    EntropyStreamRequest {
        profile_ref: profile.profile_ref.clone(),
        stream_id: stream_id.to_string(),
        purpose: purpose.to_string(),
        capability_ref: HASH_C.to_string(),
        generation: FIXTURE_GENERATION,
        mode,
        explicit_simulation_seed,
        explicit_simulation_seed_ref: explicit_simulation_seed.map(|_| HASH_B.to_string()),
    }
}

fn virtual_value(profile: &AdmittedTimeProfile, ticks: u64) -> TimeValue {
    TimeValue::Virtual(VirtualInstant {
        profile_ref: profile.profile_ref.clone(),
        ticks,
    })
}

fn select_boundary_ref(
    selection: FabricTimeFixtureSelection,
    live_ref: &str,
    simulation_ref: &str,
    trace_kind: &str,
) -> Result<String> {
    match selection {
        FabricTimeFixtureSelection::Live => Ok(live_ref.to_string()),
        FabricTimeFixtureSelection::DeterministicSimulation => Ok(simulation_ref.to_string()),
        FabricTimeFixtureSelection::Both => {
            canonical_time_trace_ref(trace_kind, &[live_ref.to_string(), simulation_ref.to_string()])
        }
    }
}

fn trace_for_kinds(
    events: &[&CanonicalTimeEvent],
    kinds: &[CanonicalTimeEventKind],
    trace_kind: &str,
) -> Result<String> {
    let refs = events
        .iter()
        .filter(|event| kinds.contains(&event.kind))
        .map(|event| event.evidence_ref.clone())
        .collect::<Vec<_>>();
    canonical_time_trace_ref(trace_kind, &refs)
}

fn count_events(events: &[&CanonicalTimeEvent], kinds: &[CanonicalTimeEventKind]) -> Result<u64> {
    u64::try_from(events.iter().filter(|event| kinds.contains(&event.kind)).count())
        .map_err(|_| MoltenError::invalid_harness("fabric-time event count overflow"))
}

fn checked_increment(value: u64, label: &str) -> Result<u64> {
    value.checked_add(1).ok_or_else(|| MoltenError::invalid_harness(format!("{label} overflow")))
}

fn core_error(label: &str, error: impl std::fmt::Debug) -> MoltenError {
    MoltenError::invalid_harness(format!("{label}: {error:?}"))
}

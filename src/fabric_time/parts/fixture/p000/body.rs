use super::*;

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
) -> crate::error::Result<ExecutableFabricTimeFixtureRun> {
    let [live_profile, simulation_profile] = admitted_profiles()?;
    let port_descriptor_refs = validate_fixture_ports(&live_profile, &simulation_profile)?;

    let mut live_clock = LiveClockAdapter::new(&live_profile.profile, PROFILE_MAX_TICKS)?;
    let first_live_wall = live_clock.observe_wall()?;
    let live_conformance =
        run_timer_adapter_conformance(&live_profile.profile, &mut live_clock, FIXTURE_SERVICE_ID, FIXTURE_GENERATION)?;
    let second_live_wall = live_clock.observe_wall()?;
    let live_wall_decision = classify_live_wall(&first_live_wall, &second_live_wall)?;

    let mut virtual_clock = VirtualClockAdapter::new(&simulation_profile.profile, 0, WALL_BASE_NANOS)?;
    let simulation_conformance = run_timer_adapter_conformance(
        &simulation_profile.profile,
        &mut virtual_clock,
        FIXTURE_SERVICE_ID,
        FIXTURE_GENERATION,
    )?;
    ensure_shared_conformance(&live_conformance, &simulation_conformance)?;

    let (live_ref, simulation_ref) = (&live_profile.profile_ref, &simulation_profile.profile_ref);
    let live_final_ticks = live_clock.now_ticks()?;
    let simulation_initial_ticks = virtual_clock.now_ticks()?;
    let live_initial = conformance_event(live_ref, "live-run-state", "initialized", live_final_ticks)?;
    let simulation_initial =
        conformance_event(simulation_ref, "simulation-run-state", "initialized", simulation_initial_ticks)?;
    let mut events = vec![live_initial.clone(), simulation_initial.clone()];
    events.push(conformance_event(live_ref, "live-adapter", "passed", live_final_ticks)?);
    events.push(conformance_event(simulation_ref, "simulation-adapter", "passed", simulation_initial_ticks)?);
    events.push(canonical_clock_anomaly_event(live_ref, FIXTURE_GENERATION, &live_wall_decision)?);
    events.push(run_live_scheduler_scenario(&live_profile)?);

    let _counters = run_simulation_scenarios(&simulation_profile, &mut virtual_clock, &mut events)?;
    let production_entropy_source = run_production_entropy_scenario(&live_profile, &mut events)?;
    let live_terminal = conformance_event(live_ref, "live-run-state", "completed", live_final_ticks)?;
    let simulation_terminal =
        conformance_event(simulation_ref, "simulation-run-state", "completed", virtual_clock.now_ticks()?)?;
    events.push(live_terminal.clone());
    events.push(simulation_terminal.clone());

    let final_time_ticks = match selection {
        FabricTimeFixtureSelection::Live => live_final_ticks,
        FabricTimeFixtureSelection::DeterministicSimulation | FabricTimeFixtureSelection::Both => {
            virtual_clock.now_ticks()?
        }
    };
    let boundaries = [&live_initial, &simulation_initial, &live_terminal, &simulation_terminal];
    let profile_refs = [
        live_profile.profile_ref.as_str(),
        simulation_profile.profile_ref.as_str(),
    ];
    let report = run_report(selection, &events, profile_refs, final_time_ticks, boundaries)?;

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

fn conformance_event(
    profile_ref: &str,
    subject: &str,
    action: &str,
    ticks: u64,
) -> crate::error::Result<CanonicalTimeEvent> {
    canonical_named_event(EventHeader {
        profile_ref,
        kind: CanonicalTimeEventKind::Conformance,
        generation: FIXTURE_GENERATION,
        subject,
        action,
        ticks,
    })
}

/// The admitted live profile and the admitted deterministic simulation profile, in that order.
fn admitted_profiles() -> crate::error::Result<[CanonicalTimeProfile; 2]> {
    let live_profile =
        canonical_admit_time_profile(&fixture_profile("molten.fabric-time.live", HASH_A, TimeProfileKind::Live, None))?;
    let simulation_profile = canonical_admit_time_profile(&fixture_profile(
        "molten.fabric-time.simulation",
        HASH_B,
        TimeProfileKind::DeterministicSimulation,
        Some(PROFILE_FAIRNESS_TURNS),
    ))?;
    Ok([live_profile, simulation_profile])
}

fn classify_live_wall(
    first: &WallClockObservation,
    second: &WallClockObservation,
) -> crate::error::Result<WallClockAnomalyDecision> {
    classify_wall_clock_observation(first, second, WallClockAnomalyPolicy {
        max_forward_jump_nanos: u64::MAX,
        max_uncertainty_nanos: PROFILE_MAX_TICKS,
    })
    .map_err(|error| core_error("classify live wall clock", error))
}

/// The run report over the events the selection covers, bounded by the selected initial and
/// terminal events. `boundaries` holds the live and simulation initial events, then the live and
/// simulation terminal events.
fn run_report(
    selection: FabricTimeFixtureSelection,
    events: &[CanonicalTimeEvent],
    [live_profile_ref, simulation_profile_ref]: [&str; 2],
    final_time_ticks: u64,
    [live_initial, simulation_initial, live_terminal, simulation_terminal]: [&CanonicalTimeEvent; 4],
) -> crate::error::Result<CanonicalFabricTimeRun> {
    let selected_events = events
        .iter()
        .filter(|event| match selection {
            FabricTimeFixtureSelection::Live => event.profile_ref == live_profile_ref,
            FabricTimeFixtureSelection::DeterministicSimulation => event.profile_ref == simulation_profile_ref,
            FabricTimeFixtureSelection::Both => true,
        })
        .collect::<Vec<_>>();
    let evidence_refs = selected_events.iter().map(|event| event.evidence_ref.clone()).collect::<Vec<_>>();
    let profile_ref = match selection {
        FabricTimeFixtureSelection::Live => live_profile_ref.to_string(),
        FabricTimeFixtureSelection::DeterministicSimulation | FabricTimeFixtureSelection::Both => {
            simulation_profile_ref.to_string()
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
    canonical_fabric_time_run(FabricTimeRunReport {
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
    })
}

fn run_live_scheduler_scenario(profile: &CanonicalTimeProfile) -> crate::error::Result<CanonicalTimeEvent> {
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
    parked
        .join()
        .map_err(|_| crate::error::MoltenError::invalid_harness("live scheduler wake target panicked"))?;
    if !wake_adapter.unregister(&key) {
        return Err(crate::error::MoltenError::invalid_harness("live scheduler wake target cleanup failed"));
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

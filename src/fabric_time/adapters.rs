use std::io::Read;
use std::time::Duration;
use std::time::Instant;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use super::AdmittedTimeProfile;
use super::EntropyEvidenceMetadata;
use super::EntropyRequest;
use super::EntropyStreamRequest;
use super::EntropyStreamState;
use super::EntropyTransition;
use super::MonotonicInstant;
use super::RunnableKey;
use super::SchedulerCommand;
use super::SchedulerPolicy;
use super::SchedulerSelection;
use super::SchedulerState;
use super::SchedulerTransition;
use super::TimeDomain;
use super::TimeProfileKind;
use super::TimerAction;
use super::TimerError;
use super::TimerKey;
use super::TimerKind;
use super::TimerScheduleRequest;
use super::TimerState;
use super::TimerTransition;
use super::VirtualInstant;
use super::WallClockObservation;
use super::cancel_timer;
use super::cleanup_generation;
use super::consume_production_entropy;
use super::entropy_evidence_metadata;
use super::open_entropy_stream;
use super::poll_timer;
use super::schedule_timer;
use crate::error::MoltenError;
use crate::error::Result;
use crate::fabric::FabricPortKey;
use crate::system_extension::SystemExtensionExecutor;
use crate::system_extension::SystemExtensionHost;

const NANOS_PER_SECOND: u64 = 1_000_000_000;
const LIVE_CONFORMANCE_DELAY_NANOS: u64 = 2_000_000;
const LIVE_CONFORMANCE_TIMEOUT_MILLIS: u64 = 250;
const LIVE_WAIT_SLICE_MICROS: u64 = 250;
const CANCELLATION_TIMER_SEQUENCE: u64 = 2;
const OS_ENTROPY_PATH: &str = "/dev/urandom";
const CONFORMANCE_CAPABILITY_REF: &str = "blake3:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";

pub trait TimerClockAdapter {
    fn profile_ref(&self) -> &str;
    fn timer_domain(&self) -> TimeDomain;
    fn now_ticks(&mut self) -> Result<u64>;
    fn await_ticks(&mut self, target_ticks: u64) -> Result<u64>;
}

#[derive(Debug)]
pub struct LiveClockAdapter {
    profile_ref: String,
    monotonic_origin: Instant,
    wall_uncertainty_nanos: u64,
    observation_sequence: u64,
    last_monotonic_ticks: u64,
}

impl LiveClockAdapter {
    pub fn new(profile: &AdmittedTimeProfile, wall_uncertainty_nanos: u64) -> Result<Self> {
        if profile.kind != TimeProfileKind::Live {
            return Err(MoltenError::invalid_harness("live clock requires an admitted live time profile"));
        }
        if wall_uncertainty_nanos > profile.max_uncertainty_ticks {
            return Err(MoltenError::invalid_harness(format!(
                "wall uncertainty {wall_uncertainty_nanos} exceeds profile maximum {}",
                profile.max_uncertainty_ticks
            )));
        }
        Ok(Self {
            profile_ref: profile.profile_ref.clone(),
            monotonic_origin: Instant::now(),
            wall_uncertainty_nanos,
            observation_sequence: 0,
            last_monotonic_ticks: 0,
        })
    }

    // r[impl molten.fabric_time.live_sim_parity]
    pub fn observe_wall(&mut self) -> Result<WallClockObservation> {
        let duration = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| MoltenError::invalid_harness(format!("system clock predates Unix epoch: {error}")))?;
        self.observation_sequence = self
            .observation_sequence
            .checked_add(1)
            .ok_or_else(|| MoltenError::invalid_harness("wall observation sequence overflow"))?;
        Ok(WallClockObservation {
            profile_ref: self.profile_ref.clone(),
            unix_nanos: duration_to_u64_nanos(duration)?,
            uncertainty_nanos: self.wall_uncertainty_nanos,
            observation_sequence: self.observation_sequence,
        })
    }

    pub fn observe_monotonic(&mut self) -> Result<MonotonicInstant> {
        let ticks = duration_to_u64_nanos(self.monotonic_origin.elapsed())?;
        if ticks < self.last_monotonic_ticks {
            return Err(MoltenError::invalid_harness("live monotonic clock moved backwards"));
        }
        self.last_monotonic_ticks = ticks;
        Ok(MonotonicInstant {
            profile_ref: self.profile_ref.clone(),
            ticks,
        })
    }
}

impl TimerClockAdapter for LiveClockAdapter {
    fn profile_ref(&self) -> &str {
        &self.profile_ref
    }

    fn timer_domain(&self) -> TimeDomain {
        TimeDomain::Monotonic
    }

    fn now_ticks(&mut self) -> Result<u64> {
        Ok(self.observe_monotonic()?.ticks)
    }

    fn await_ticks(&mut self, target_ticks: u64) -> Result<u64> {
        let wait_started = Instant::now();
        let maximum_wait = Duration::from_millis(LIVE_CONFORMANCE_TIMEOUT_MILLIS);
        let wait_slice = Duration::from_micros(LIVE_WAIT_SLICE_MICROS);
        loop {
            let now = self.now_ticks()?;
            if now >= target_ticks {
                return Ok(now);
            }
            if wait_started.elapsed() >= maximum_wait {
                return Err(MoltenError::invalid_harness(format!(
                    "live timer did not reach {target_ticks} within {} milliseconds",
                    LIVE_CONFORMANCE_TIMEOUT_MILLIS
                )));
            }
            std::thread::sleep(wait_slice);
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VirtualClockAdapter {
    profile_ref: String,
    virtual_ticks: u64,
    logical_position: u64,
    wall_base_nanos: u64,
    wall_offset_nanos: i128,
    observation_sequence: u64,
    wall_uncertainty_nanos: u64,
}

impl VirtualClockAdapter {
    pub fn new(profile: &AdmittedTimeProfile, initial_virtual_ticks: u64, wall_base_nanos: u64) -> Result<Self> {
        if profile.kind != TimeProfileKind::DeterministicSimulation {
            return Err(MoltenError::invalid_harness(
                "virtual clock requires an admitted deterministic simulation profile",
            ));
        }
        Ok(Self {
            profile_ref: profile.profile_ref.clone(),
            virtual_ticks: initial_virtual_ticks,
            logical_position: 0,
            wall_base_nanos,
            wall_offset_nanos: 0,
            observation_sequence: 0,
            wall_uncertainty_nanos: 0,
        })
    }

    // r[impl molten.fabric_time.live_sim_parity]
    pub fn advance(&mut self, delta_ticks: u64) -> Result<VirtualInstant> {
        self.virtual_ticks = self
            .virtual_ticks
            .checked_add(delta_ticks)
            .ok_or_else(|| MoltenError::invalid_harness("virtual time overflow"))?;
        Ok(self.observe_virtual())
    }

    pub fn advance_logical(&mut self) -> Result<u64> {
        self.logical_position = self
            .logical_position
            .checked_add(1)
            .ok_or_else(|| MoltenError::invalid_harness("logical time overflow"))?;
        Ok(self.logical_position)
    }

    pub fn observe_virtual(&self) -> VirtualInstant {
        VirtualInstant {
            profile_ref: self.profile_ref.clone(),
            ticks: self.virtual_ticks,
        }
    }

    pub fn observe_wall(&mut self) -> Result<WallClockObservation> {
        self.observation_sequence = self
            .observation_sequence
            .checked_add(1)
            .ok_or_else(|| MoltenError::invalid_harness("virtual wall observation sequence overflow"))?;
        let base = i128::from(self.wall_base_nanos)
            .checked_add(i128::from(self.virtual_ticks))
            .and_then(|value| value.checked_add(self.wall_offset_nanos))
            .ok_or_else(|| MoltenError::invalid_harness("virtual wall clock overflow"))?;
        let unix_nanos =
            u64::try_from(base).map_err(|_| MoltenError::invalid_harness("virtual wall clock underflow"))?;
        Ok(WallClockObservation {
            profile_ref: self.profile_ref.clone(),
            unix_nanos,
            uncertainty_nanos: self.wall_uncertainty_nanos,
            observation_sequence: self.observation_sequence,
        })
    }

    pub fn inject_wall_jump(&mut self, signed_delta_nanos: i128) -> Result<()> {
        self.wall_offset_nanos = self
            .wall_offset_nanos
            .checked_add(signed_delta_nanos)
            .ok_or_else(|| MoltenError::invalid_harness("virtual wall fault offset overflow"))?;
        Ok(())
    }

    pub fn set_wall_uncertainty(&mut self, uncertainty_nanos: u64) {
        self.wall_uncertainty_nanos = uncertainty_nanos;
    }
}

impl TimerClockAdapter for VirtualClockAdapter {
    fn profile_ref(&self) -> &str {
        &self.profile_ref
    }

    fn timer_domain(&self) -> TimeDomain {
        TimeDomain::Virtual
    }

    fn now_ticks(&mut self) -> Result<u64> {
        Ok(self.virtual_ticks)
    }

    fn await_ticks(&mut self, target_ticks: u64) -> Result<u64> {
        if target_ticks < self.virtual_ticks {
            return Err(MoltenError::invalid_harness(format!(
                "virtual adapter cannot move backwards from {} to {target_ticks}",
                self.virtual_ticks
            )));
        }
        self.virtual_ticks = target_ticks;
        Ok(self.virtual_ticks)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdapterConformanceObservation {
    pub domain: TimeDomain,
    pub timer_action: TimerAction,
    pub delivery_count: u64,
    pub stale_generation_discarded: bool,
    pub cancellation_prevented_delivery: bool,
    pub scheduler_selected: bool,
    pub scheduler_cancellation_recorded: bool,
    pub entropy_bound_rejected: bool,
}

// r[impl molten.fabric_time.live_sim_parity]
pub fn run_timer_adapter_conformance<A: TimerClockAdapter>(
    profile: &AdmittedTimeProfile,
    adapter: &mut A,
    service_id: &str,
    generation: u64,
) -> Result<AdapterConformanceObservation> {
    if adapter.profile_ref() != profile.profile_ref {
        return Err(MoltenError::invalid_harness("timer adapter profile mismatch"));
    }
    let start = adapter.now_ticks()?;
    let delay = match profile.kind {
        TimeProfileKind::Live => LIVE_CONFORMANCE_DELAY_NANOS,
        TimeProfileKind::DeterministicSimulation => 1,
    };
    let deadline = start
        .checked_add(delay)
        .ok_or_else(|| MoltenError::invalid_harness("conformance deadline overflow"))?;
    let request = conformance_timer_request(profile, service_id, generation, adapter.timer_domain(), deadline, 0);
    let timer = schedule_timer(profile, generation, 0, &request)
        .map_err(|error| core_error("schedule conformance timer", error))?;
    let observed = adapter.await_ticks(deadline)?;
    let fired =
        poll_timer(&timer, generation, observed, 1).map_err(|error| core_error("poll conformance timer", error))?;

    let stale_request = conformance_timer_request(profile, service_id, generation, adapter.timer_domain(), deadline, 1);
    let stale_timer = schedule_timer(profile, generation, 0, &stale_request)
        .map_err(|error| core_error("schedule stale probe", error))?;
    let stale = poll_timer(&stale_timer, generation.saturating_add(1), observed, 1)
        .map_err(|error| core_error("poll stale probe", error))?;

    let cancel_request = conformance_timer_request(
        profile,
        service_id,
        generation,
        adapter.timer_domain(),
        deadline,
        CANCELLATION_TIMER_SEQUENCE,
    );
    let cancel_timer_state = schedule_timer(profile, generation, 0, &cancel_request)
        .map_err(|error| core_error("schedule cancellation probe", error))?;
    let cancelled = cancel_timer(&cancel_timer_state, generation).map_err(|error| core_error("cancel probe", error))?;
    let cancellation_prevented_delivery =
        matches!(poll_timer(&cancelled.next, generation, observed, 1), Err(TimerError::TerminalTimer(_)));

    let (scheduler_selected, scheduler_cancellation_recorded) =
        run_scheduler_conformance(profile, service_id, generation)?;
    let entropy_bound_rejected = run_entropy_conformance(profile, generation)?;

    Ok(AdapterConformanceObservation {
        domain: adapter.timer_domain(),
        timer_action: fired.action,
        delivery_count: fired.delivery_count,
        stale_generation_discarded: stale.action == TimerAction::DiscardedStaleGeneration,
        cancellation_prevented_delivery,
        scheduler_selected,
        scheduler_cancellation_recorded,
        entropy_bound_rejected,
    })
}

#[derive(Debug, Default)]
pub struct ThreadSchedulerWakeAdapter {
    targets: std::collections::BTreeMap<RunnableKey, std::thread::Thread>,
}

impl ThreadSchedulerWakeAdapter {
    pub fn register(&mut self, key: RunnableKey, thread: std::thread::Thread) -> Result<()> {
        if self.targets.insert(key.clone(), thread).is_some() {
            return Err(MoltenError::invalid_harness(format!(
                "scheduler wake target {}:{}:{} is already registered",
                key.service_id, key.generation, key.runnable_id
            )));
        }
        Ok(())
    }

    pub fn unregister(&mut self, key: &RunnableKey) -> bool {
        self.targets.remove(key).is_some()
    }

    // The canonical core has already admitted the transition. This shell only
    // translates an admitted wake into the host thread wake primitive.
    pub fn route(&self, transition: &SchedulerTransition) -> Result<()> {
        if !matches!(transition.action, super::SchedulerAction::Woken | super::SchedulerAction::Yielded) {
            return Err(MoltenError::invalid_harness("thread wake adapter received a non-wake scheduler transition"));
        }
        let thread = self.targets.get(&transition.runnable).ok_or_else(|| {
            MoltenError::invalid_harness(format!(
                "no live scheduler wake target for {}:{}:{}",
                transition.runnable.service_id, transition.runnable.generation, transition.runnable.runnable_id
            ))
        })?;
        thread.unpark();
        Ok(())
    }
}

fn run_scheduler_conformance(profile: &AdmittedTimeProfile, service_id: &str, generation: u64) -> Result<(bool, bool)> {
    let runnable = RunnableKey {
        service_id: service_id.to_string(),
        generation,
        runnable_id: "adapter-conformance-runnable".to_string(),
    };
    let state = super::new_scheduler_state(profile, generation);
    let woken = super::apply_scheduler_command(
        profile,
        profile.scheduler_policy,
        &state,
        generation,
        &SchedulerCommand::Wake {
            key: runnable.clone(),
            priority: 0,
        },
    )
    .map_err(|error| core_error("wake conformance runnable", error))?;
    let selected = super::choose_runnable(profile, profile.scheduler_policy, &woken.next, generation, Some(&runnable))
        .map_err(|error| core_error("select conformance runnable", error))?;

    let cancellation_key = RunnableKey {
        runnable_id: "adapter-conformance-cancellation".to_string(),
        ..runnable.clone()
    };
    let cancellation_wake = super::apply_scheduler_command(
        profile,
        profile.scheduler_policy,
        &state,
        generation,
        &SchedulerCommand::Wake {
            key: cancellation_key.clone(),
            priority: 0,
        },
    )
    .map_err(|error| core_error("wake cancellation conformance runnable", error))?;
    let cancelled = super::apply_scheduler_command(
        profile,
        profile.scheduler_policy,
        &cancellation_wake.next,
        generation,
        &SchedulerCommand::Cancel { key: cancellation_key },
    )
    .map_err(|error| core_error("cancel conformance runnable", error))?;
    Ok((selected.selected == runnable, cancelled.action == super::SchedulerAction::Cancelled))
}

fn run_entropy_conformance(profile: &AdmittedTimeProfile, generation: u64) -> Result<bool> {
    let (mode, seed) = match profile.kind {
        TimeProfileKind::Live => (super::EntropyMode::ProductionCryptographic, None),
        TimeProfileKind::DeterministicSimulation => (super::EntropyMode::DeterministicSimulation, Some(1)),
    };
    let stream = open_entropy_stream(profile, generation, &EntropyStreamRequest {
        profile_ref: profile.profile_ref.clone(),
        stream_id: "adapter-conformance-stream".to_string(),
        purpose: "adapter-conformance-bound".to_string(),
        capability_ref: CONFORMANCE_CAPABILITY_REF.to_string(),
        generation,
        mode,
        explicit_simulation_seed: seed,
        explicit_simulation_seed_ref: seed.map(|_| CONFORMANCE_CAPABILITY_REF.to_string()),
    })
    .map_err(|error| core_error("open conformance entropy stream", error))?;
    let over_limit = profile
        .max_entropy_request_bytes
        .checked_add(1)
        .ok_or_else(|| MoltenError::invalid_harness("conformance entropy bound overflow"))?;
    let request = EntropyRequest::Bytes { count: over_limit };
    let rejected = match mode {
        super::EntropyMode::DeterministicSimulation => matches!(
            super::draw_deterministic_entropy(profile, generation, &stream, request),
            Err(super::EntropyError::RequestLimitExceeded { .. })
        ),
        super::EntropyMode::ProductionCryptographic => matches!(
            consume_production_entropy(profile, generation, &stream, request, Vec::new()),
            Err(super::EntropyError::RequestLimitExceeded { .. })
        ),
    };
    Ok(rejected)
}

pub trait CryptographicEntropySource {
    fn source_id(&self) -> &'static str;
    fn fill_secret(&mut self, output: &mut [u8]) -> Result<()>;
}

#[derive(Debug, Default)]
pub struct OperatingSystemEntropySource;

impl CryptographicEntropySource for OperatingSystemEntropySource {
    fn source_id(&self) -> &'static str {
        "unix-dev-urandom"
    }

    fn fill_secret(&mut self, output: &mut [u8]) -> Result<()> {
        #[cfg(unix)]
        {
            let mut source = std::fs::File::open(OS_ENTROPY_PATH)
                .map_err(|error| MoltenError::invalid_harness(format!("open production entropy source: {error}")))?;
            source
                .read_exact(output)
                .map_err(|error| MoltenError::invalid_harness(format!("read production entropy source: {error}")))
        }
        #[cfg(not(unix))]
        {
            let _ = output;
            Err(MoltenError::invalid_harness("production entropy adapter has no admitted source on this platform"))
        }
    }
}

pub struct ProductionEntropyAdapter<S: CryptographicEntropySource> {
    source: S,
}

impl<S: CryptographicEntropySource> ProductionEntropyAdapter<S> {
    pub const fn new(source: S) -> Self {
        Self { source }
    }

    pub fn source_id(&self) -> &'static str {
        self.source.source_id()
    }

    pub fn draw(
        &mut self,
        profile: &AdmittedTimeProfile,
        active_generation: u64,
        state: &EntropyStreamState,
        request: EntropyRequest,
    ) -> Result<(EntropyTransition, EntropyEvidenceMetadata)> {
        let output_len = usize::try_from(request.requested_bytes())
            .map_err(|_| MoltenError::invalid_harness("entropy request length overflow"))?;
        let mut secret = vec![0; output_len];
        self.source.fill_secret(&mut secret)?;
        let transition = consume_production_entropy(profile, active_generation, state, request, secret)
            .map_err(|error| core_error("consume production entropy", error))?;
        let metadata = entropy_evidence_metadata(state, &transition);
        Ok((transition, metadata))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtensionTimeContext {
    service_id: String,
    generation: u64,
    max_timers: u64,
    max_runnables: u64,
    max_concurrency: u64,
    max_entropy_request_bytes: u64,
    capability_refs: Vec<String>,
    timer_profile: Option<String>,
    scheduler_profile: Option<String>,
    entropy_profile: Option<String>,
}

impl ExtensionTimeContext {
    pub fn from_host<E: SystemExtensionExecutor>(host: &SystemExtensionHost<E>) -> Self {
        let manifest = host.manifest().manifest();
        Self {
            service_id: manifest.service_id.clone(),
            generation: host.state().generation,
            max_timers: manifest.resources.max_timers,
            max_runnables: manifest.resources.max_queued_events,
            max_concurrency: manifest.resources.max_concurrent_callbacks,
            max_entropy_request_bytes: manifest.resources.max_inflight_bytes,
            capability_refs: manifest.capability_refs.clone(),
            timer_profile: bound_port_profile(host, super::FABRIC_TIMER_PORT_ID),
            scheduler_profile: bound_port_profile(host, super::FABRIC_SCHEDULER_PORT_ID),
            entropy_profile: bound_port_profile(host, super::FABRIC_ENTROPY_PORT_ID),
        }
    }

    #[cfg(test)]
    pub(crate) fn from_test_snapshot(
        service_id: &str,
        generation: u64,
        profile: &AdmittedTimeProfile,
        capability_refs: Vec<String>,
    ) -> Self {
        Self {
            service_id: service_id.to_string(),
            generation,
            max_timers: profile.max_timers,
            max_runnables: profile.max_runnables,
            max_concurrency: profile.max_scheduler_concurrency,
            max_entropy_request_bytes: profile.max_entropy_request_bytes,
            capability_refs,
            timer_profile: Some(profile.profile_id.clone()),
            scheduler_profile: Some(profile.profile_id.clone()),
            entropy_profile: Some(profile.profile_id.clone()),
        }
    }

    // r[impl molten.fabric_time.live_sim_parity]
    pub fn schedule_timer(
        &self,
        profile: &AdmittedTimeProfile,
        active_timer_count: u64,
        request: &TimerScheduleRequest,
    ) -> Result<TimerState> {
        ensure_bound_profile(&self.timer_profile, profile, "timer")?;
        if request.key.service_id != self.service_id {
            return Err(MoltenError::invalid_harness("timer service identity mismatch"));
        }
        if active_timer_count >= self.max_timers {
            return Err(MoltenError::invalid_harness(format!(
                "system-extension timer limit {} exhausted",
                self.max_timers
            )));
        }
        schedule_timer(profile, self.generation, active_timer_count, request)
            .map_err(|error| core_error("system-extension timer admission", error))
    }

    pub fn apply_scheduler_command(
        &self,
        profile: &AdmittedTimeProfile,
        policy: SchedulerPolicy,
        state: &SchedulerState,
        command: &SchedulerCommand,
    ) -> Result<SchedulerTransition> {
        ensure_bound_profile(&self.scheduler_profile, profile, "scheduler")?;
        let key = scheduler_command_key(command);
        if key.service_id != self.service_id {
            return Err(MoltenError::invalid_harness("runnable service identity mismatch"));
        }
        if matches!(command, SchedulerCommand::Wake { .. }) {
            let active = u64::try_from(
                state
                    .runnables
                    .iter()
                    .filter(|runnable| {
                        !matches!(runnable.phase, super::RunnablePhase::Completed | super::RunnablePhase::Cancelled)
                    })
                    .count(),
            )
            .map_err(|_| MoltenError::invalid_harness("runnable count overflow"))?;
            if active >= self.max_runnables {
                return Err(MoltenError::invalid_harness(format!(
                    "system-extension runnable limit {} exhausted",
                    self.max_runnables
                )));
            }
        }
        super::apply_scheduler_command(profile, policy, state, self.generation, command)
            .map_err(|error| core_error("system-extension scheduler command", error))
    }

    pub fn choose_runnable(
        &self,
        profile: &AdmittedTimeProfile,
        policy: SchedulerPolicy,
        state: &SchedulerState,
        recorded_choice: Option<&RunnableKey>,
    ) -> Result<SchedulerSelection> {
        ensure_bound_profile(&self.scheduler_profile, profile, "scheduler")?;
        let running = u64::try_from(
            state.runnables.iter().filter(|runnable| runnable.phase == super::RunnablePhase::Running).count(),
        )
        .map_err(|_| MoltenError::invalid_harness("running runnable count overflow"))?;
        if running >= self.max_concurrency {
            return Err(MoltenError::invalid_harness(format!(
                "system-extension concurrency limit {} exhausted",
                self.max_concurrency
            )));
        }
        super::choose_runnable(profile, policy, state, self.generation, recorded_choice)
            .map_err(|error| core_error("system-extension scheduler selection", error))
    }

    pub fn open_entropy_stream(
        &self,
        profile: &AdmittedTimeProfile,
        request: &EntropyStreamRequest,
    ) -> Result<EntropyStreamState> {
        ensure_bound_profile(&self.entropy_profile, profile, "entropy")?;
        if !self.capability_refs.contains(&request.capability_ref) {
            return Err(MoltenError::invalid_harness("entropy request lacks an admitted system-extension capability"));
        }
        open_entropy_stream(profile, self.generation, request)
            .map_err(|error| core_error("system-extension entropy stream", error))
    }

    pub fn admit_entropy_request(&self, request: EntropyRequest) -> Result<()> {
        let requested = request.requested_bytes();
        if requested > self.max_entropy_request_bytes {
            return Err(MoltenError::invalid_harness(format!(
                "entropy request {requested} exceeds system-extension byte envelope {}",
                self.max_entropy_request_bytes
            )));
        }
        Ok(())
    }

    pub fn cleanup_retired_timers(&self, states: &[TimerState], retired_generation: u64) -> Vec<TimerState> {
        cleanup_generation(states, retired_generation)
    }
}

fn bound_port_profile<E: SystemExtensionExecutor>(host: &SystemExtensionHost<E>, port_id: &str) -> Option<String> {
    host.manifest()
        .binding_for(&FabricPortKey {
            port_id: port_id.to_string(),
            version: super::FABRIC_TIME_PORT_VERSION.to_string(),
        })
        .map(|binding| binding.binding.implementation_profile.clone())
}

fn ensure_bound_profile(bound_profile: &Option<String>, profile: &AdmittedTimeProfile, port: &str) -> Result<()> {
    let Some(bound_profile) = bound_profile else {
        return Err(MoltenError::invalid_harness(format!(
            "system extension has no admitted {port} fabric port binding"
        )));
    };
    if bound_profile != &profile.profile_id {
        return Err(MoltenError::invalid_harness(format!(
            "system extension {port} profile {bound_profile} does not admit requested profile {}",
            profile.profile_id
        )));
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FabricTimeFault {
    BackwardWallJump { ticks: u64 },
    ForwardWallJump { ticks: u64 },
    DelayTimer { key: TimerKey, ticks: u64 },
    DropTimerDelivery { key: TimerKey },
    SaturateSchedulerQueue,
    CancelTimer { key: TimerKey },
    PartitionWindow { until_ticks: u64 },
}

pub fn apply_clock_fault(clock: &mut VirtualClockAdapter, fault: &FabricTimeFault) -> Result<bool> {
    match fault {
        FabricTimeFault::BackwardWallJump { ticks } => {
            clock.inject_wall_jump(-i128::from(*ticks))?;
            Ok(true)
        }
        FabricTimeFault::ForwardWallJump { ticks } => {
            clock.inject_wall_jump(i128::from(*ticks))?;
            Ok(true)
        }
        _ => Ok(false),
    }
}

pub fn validate_scheduler_fault_outcome(fault: &FabricTimeFault, transition: &SchedulerTransition) -> Result<()> {
    if matches!(fault, FabricTimeFault::SaturateSchedulerQueue)
        && !matches!(transition.action, super::SchedulerAction::RejectedOverload | super::SchedulerAction::Backpressure)
    {
        return Err(MoltenError::invalid_harness(
            "scheduler saturation fault did not produce an explicit overload outcome",
        ));
    }
    Ok(())
}

pub fn poll_timer_with_fault(
    state: &TimerState,
    active_generation: u64,
    now_ticks: u64,
    delivery_capacity: u64,
    fault: Option<&FabricTimeFault>,
) -> Result<TimerTransition> {
    let mut faulted = state.clone();
    let mut capacity = delivery_capacity;
    if let Some(fault) = fault {
        match fault {
            FabricTimeFault::DelayTimer { key, ticks } if key == &state.key => {
                faulted.next_deadline_ticks = faulted
                    .next_deadline_ticks
                    .checked_add(*ticks)
                    .ok_or_else(|| MoltenError::invalid_harness("faulted timer deadline overflow"))?;
            }
            FabricTimeFault::DropTimerDelivery { key } if key == &state.key => {
                faulted.overload = super::TimerOverloadPolicy::DropDue;
                capacity = 0;
            }
            FabricTimeFault::CancelTimer { key } if key == &state.key => {
                return cancel_timer(&faulted, active_generation)
                    .map_err(|error| core_error("cancel faulted timer", error));
            }
            _ => {}
        }
    }
    poll_timer(&faulted, active_generation, now_ticks, capacity)
        .map_err(|error| core_error("poll faulted timer", error))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FaultedDeadlineDecision {
    Evaluated(super::DeadlineDecision),
    PartitionIndeterminate { until_ticks: u64, observed_ticks: u64 },
}

pub fn evaluate_deadline_with_fault(
    profile: &AdmittedTimeProfile,
    active_generation: u64,
    deadline: &super::Deadline,
    observed: &super::TimeValue,
    fault: Option<&FabricTimeFault>,
) -> Result<FaultedDeadlineDecision> {
    if let Some(FabricTimeFault::PartitionWindow { until_ticks }) = fault
        && observed.ticks() <= *until_ticks
    {
        return Ok(FaultedDeadlineDecision::PartitionIndeterminate {
            until_ticks: *until_ticks,
            observed_ticks: observed.ticks(),
        });
    }
    super::evaluate_deadline(profile, active_generation, deadline, observed)
        .map(FaultedDeadlineDecision::Evaluated)
        .map_err(|error| core_error("evaluate faulted deadline", error))
}

fn conformance_timer_request(
    profile: &AdmittedTimeProfile,
    service_id: &str,
    generation: u64,
    domain: TimeDomain,
    deadline_ticks: u64,
    sequence: u64,
) -> TimerScheduleRequest {
    TimerScheduleRequest {
        profile_ref: profile.profile_ref.clone(),
        key: TimerKey {
            service_id: service_id.to_string(),
            generation,
            sequence,
        },
        domain,
        deadline_ticks,
        kind: TimerKind::OneShot,
        ordering_key: sequence,
        coalescing: super::TimerCoalescingPolicy::CoalesceLatest,
        lateness: super::TimerLatenessPolicy::DeliverRegardless,
        overload: super::TimerOverloadPolicy::RejectAndRetain,
        resource_charge: super::TimerResourceCharge::single_slot(),
    }
}

fn scheduler_command_key(command: &SchedulerCommand) -> &RunnableKey {
    match command {
        SchedulerCommand::Wake { key, .. }
        | SchedulerCommand::Yield { key }
        | SchedulerCommand::Block { key }
        | SchedulerCommand::Complete { key }
        | SchedulerCommand::Cancel { key } => key,
    }
}

fn duration_to_u64_nanos(duration: Duration) -> Result<u64> {
    let seconds = duration
        .as_secs()
        .checked_mul(NANOS_PER_SECOND)
        .ok_or_else(|| MoltenError::invalid_harness("duration seconds overflow"))?;
    seconds
        .checked_add(u64::from(duration.subsec_nanos()))
        .ok_or_else(|| MoltenError::invalid_harness("duration nanoseconds overflow"))
}

fn core_error(label: &str, error: impl std::fmt::Debug) -> MoltenError {
    MoltenError::invalid_harness(format!("{label}: {error:?}"))
}

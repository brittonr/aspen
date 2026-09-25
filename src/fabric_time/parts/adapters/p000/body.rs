// r[impl molten.modularity.fabric_boundary.adapters]
use std::io::Read;

use super::CryptographicEntropySource;
use super::TimerClockAdapter;
#[allow(
    tigerstyle::non_trait_imports,
    reason = "time mechanisms implement the application-owned typed port contracts"
)]
use crate::fabric::FabricPortError;
#[allow(
    tigerstyle::non_trait_imports,
    reason = "time mechanisms implement the application-owned typed port contracts"
)]
use crate::fabric::FabricPortResult;

const NANOS_PER_SECOND: u64 = 1_000_000_000;
const LIVE_CONFORMANCE_DELAY_NANOS: u64 = 2_000_000;
const LIVE_CONFORMANCE_TIMEOUT_MILLIS: u64 = 250;
const LIVE_WAIT_SLICE_MICROS: u64 = 250;
const CANCELLATION_TIMER_SEQUENCE: u64 = 2;
const OS_ENTROPY_PATH: &str = "/dev/urandom";
const CONFORMANCE_CAPABILITY_REF: &str = "blake3:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
const NANOS_PER_MILLISECOND: u64 = 1_000_000;
const PROCESS_SUPERVISION_PROFILE_ID: &str = "molten.fabric-time.process-supervision";
/// The longest child-process supervision deadline the live supervision clock admits: one hour.
const PROCESS_SUPERVISION_MAX_TICKS: u64 = 3_600 * NANOS_PER_SECOND;
/// Supervision polls one monotonic deadline at a time and admits no timers, runnables, or entropy
/// of its own.
const PROCESS_SUPERVISION_UNIT_BOUND: u64 = 1;

#[derive(Debug)]
pub struct LiveClockAdapter {
    profile_ref: String,
    monotonic_origin: std::time::Instant,
    wall_uncertainty_nanos: u64,
    observation_sequence: u64,
    last_monotonic_ticks: u64,
}

impl LiveClockAdapter {
    #[allow(
        tigerstyle::ambient_clock,
        reason = "LiveClockAdapter is the documented live monotonic clock capability; it anchors its origin at the host monotonic clock once, and every other reader goes through the TimerClockAdapter port"
    )]
    pub fn new(profile: &super::AdmittedTimeProfile, wall_uncertainty_nanos: u64) -> crate::error::Result<Self> {
        if profile.kind != super::TimeProfileKind::Live {
            return Err(crate::error::MoltenError::invalid_harness(
                "live clock requires an admitted live time profile",
            ));
        }
        if wall_uncertainty_nanos > profile.max_uncertainty_ticks {
            return Err(crate::error::MoltenError::invalid_harness(format!(
                "wall uncertainty {wall_uncertainty_nanos} exceeds profile maximum {}",
                profile.max_uncertainty_ticks
            )));
        }
        Ok(Self {
            profile_ref: profile.profile_ref.clone(),
            monotonic_origin: std::time::Instant::now(),
            wall_uncertainty_nanos,
            observation_sequence: 0,
            last_monotonic_ticks: 0,
        })
    }

    // r[impl molten.fabric_time.live_sim_parity]
    #[allow(
        tigerstyle::ambient_clock,
        reason = "LiveClockAdapter is the documented live wall-clock capability; observe_wall is the single place that reads the host wall clock and returns it as a canonical uncertainty-bounded observation"
    )]
    pub fn observe_wall(&mut self) -> crate::error::Result<super::WallClockObservation> {
        let duration = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_err(|error| {
            crate::error::MoltenError::invalid_harness(format!("system clock predates Unix epoch: {error}"))
        })?;
        self.observation_sequence = self
            .observation_sequence
            .checked_add(1)
            .ok_or_else(|| crate::error::MoltenError::invalid_harness("wall observation sequence overflow"))?;
        Ok(super::WallClockObservation {
            profile_ref: self.profile_ref.clone(),
            unix_nanos: duration_to_u64_nanos(duration)?,
            uncertainty_nanos: self.wall_uncertainty_nanos,
            observation_sequence: self.observation_sequence,
        })
    }

    pub fn observe_monotonic(&mut self) -> crate::error::Result<super::MonotonicInstant> {
        let ticks = duration_to_u64_nanos(self.monotonic_origin.elapsed())?;
        if ticks < self.last_monotonic_ticks {
            return Err(crate::error::MoltenError::invalid_harness("live monotonic clock moved backwards"));
        }
        self.last_monotonic_ticks = ticks;
        Ok(super::MonotonicInstant {
            profile_ref: self.profile_ref.clone(),
            ticks,
        })
    }
}

// r[impl molten.modularity.fabric_boundary.adapters.clock]
impl TimerClockAdapter for LiveClockAdapter {
    fn profile_ref(&self) -> &str {
        &self.profile_ref
    }

    fn timer_domain(&self) -> super::TimeDomain {
        super::TimeDomain::Monotonic
    }

    fn now_ticks(&mut self) -> FabricPortResult<u64> {
        Ok(self.observe_monotonic().map_err(FabricPortError::from)?.ticks)
    }

    fn await_ticks(&mut self, target_ticks: u64) -> FabricPortResult<u64> {
        let wait_deadline = TickDeadline::after(self, LIVE_CONFORMANCE_TIMEOUT_MILLIS * NANOS_PER_MILLISECOND)?;
        let wait_slice = std::time::Duration::from_micros(LIVE_WAIT_SLICE_MICROS);
        loop {
            let now = self.now_ticks()?;
            if now >= target_ticks {
                return Ok(now);
            }
            if wait_deadline.is_expired(self)? {
                return Err(FabricPortError::Timeout {
                    message: format!(
                        "live timer did not reach {target_ticks} within {} milliseconds",
                        LIVE_CONFORMANCE_TIMEOUT_MILLIS
                    ),
                });
            }
            std::thread::sleep(wait_slice);
        }
    }
}

/// A deadline measured in the ticks of an admitted timer clock. Loops poll it through the clock
/// port instead of reading the ambient clock, so a live adapter and a virtual adapter drive the
/// same deadline logic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TickDeadline {
    deadline_ticks: u64,
}

impl TickDeadline {
    pub fn after(clock: &mut impl TimerClockAdapter, timeout_ticks: u64) -> FabricPortResult<Self> {
        let deadline_ticks = clock
            .now_ticks()?
            .checked_add(timeout_ticks)
            .ok_or_else(|| FabricPortError::malformed("tick deadline overflows the clock domain"))?;
        Ok(Self { deadline_ticks })
    }

    pub fn remaining_ticks(&self, clock: &mut impl TimerClockAdapter) -> FabricPortResult<u64> {
        Ok(self.deadline_ticks.saturating_sub(clock.now_ticks()?))
    }

    pub fn is_expired(&self, clock: &mut impl TimerClockAdapter) -> FabricPortResult<bool> {
        Ok(self.remaining_ticks(clock)? == 0)
    }
}

/// A child-process supervision deadline on the admitted live monotonic clock, whose ticks are
/// nanoseconds.
#[derive(Debug)]
pub struct SupervisionDeadline {
    clock: LiveClockAdapter,
    deadline: TickDeadline,
}

impl SupervisionDeadline {
    pub fn after(timeout: std::time::Duration) -> crate::error::Result<Self> {
        let timeout_ticks = duration_to_u64_nanos(timeout)?;
        if timeout_ticks > PROCESS_SUPERVISION_MAX_TICKS {
            return Err(crate::error::MoltenError::invalid_harness(format!(
                "supervision timeout {timeout_ticks}ns exceeds the admitted maximum {PROCESS_SUPERVISION_MAX_TICKS}ns"
            )));
        }
        let profile = super::canonical_admit_time_profile(&process_supervision_profile())?;
        let mut clock = LiveClockAdapter::new(&profile.profile, 0)?;
        let deadline = TickDeadline::after(&mut clock, timeout_ticks)?;
        Ok(Self { clock, deadline })
    }

    pub fn is_expired(&mut self) -> crate::error::Result<bool> {
        Ok(self.deadline.is_expired(&mut self.clock)?)
    }

    pub fn remaining(&mut self) -> crate::error::Result<std::time::Duration> {
        Ok(std::time::Duration::from_nanos(self.deadline.remaining_ticks(&mut self.clock)?))
    }
}

fn process_supervision_profile() -> super::TimeProfileDescriptor {
    super::TimeProfileDescriptor {
        schema: super::FABRIC_TIME_PROFILE_SCHEMA.to_string(),
        profile_id: PROCESS_SUPERVISION_PROFILE_ID.to_string(),
        profile_ref: crate::preserves_rail::content_ref_from_bytes(PROCESS_SUPERVISION_PROFILE_ID.as_bytes()),
        kind: super::TimeProfileKind::Live,
        supported_domains: super::REQUIRED_TIME_DOMAINS.to_vec(),
        max_duration_ticks: PROCESS_SUPERVISION_MAX_TICKS,
        max_uncertainty_ticks: PROCESS_SUPERVISION_UNIT_BOUND,
        max_timers: PROCESS_SUPERVISION_UNIT_BOUND,
        max_runnables: PROCESS_SUPERVISION_UNIT_BOUND,
        max_entropy_request_bytes: PROCESS_SUPERVISION_UNIT_BOUND,
        max_entropy_total_bytes: PROCESS_SUPERVISION_UNIT_BOUND,
        max_scheduler_concurrency: PROCESS_SUPERVISION_UNIT_BOUND,
        max_scheduler_queue_depth: PROCESS_SUPERVISION_UNIT_BOUND,
        fairness_bound_turns: None,
        scheduler_policy: super::SchedulerPolicy {
            ordering: super::SchedulerOrdering::PriorityThenFifo,
            replay: super::SchedulerReplayPolicy::RecordedChoiceRequired,
            overload: super::SchedulerOverloadPolicy::Reject,
        },
        evidence_mode: super::TimeEvidenceMode::SelectedSemanticBoundaries,
        non_claims: super::REQUIRED_TIME_NON_CLAIMS.to_vec(),
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

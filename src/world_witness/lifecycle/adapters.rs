//! Live shell adapters for the witness clock and request-entropy ports, built on the fabric-time
//! capabilities.

const WALL_CLOCK_PROFILE_ID: &str = "molten.world-witness.wall-clock";
/// One hour of nanosecond ticks; the witness wall clock arms no deadlines of its own.
const WALL_CLOCK_MAX_TICKS: u64 = 3_600_000_000_000;
/// The witness wall clock admits no timers, runnables, entropy draws or scheduler work of its own.
const WALL_CLOCK_UNIT_BOUND: u64 = 1;

/// Live wall clock admitted under the witness wall-clock profile. Every host clock read stays
/// inside `fabric_time::LiveClockAdapter::observe_wall`.
pub fn live_clock() -> crate::error::Result<crate::fabric_time::LiveClockAdapter> {
    let profile = crate::fabric_time::admit_profile(&crate::fabric_time::TimeProfileDescriptor {
        schema: crate::fabric_time::FABRIC_TIME_PROFILE_SCHEMA.to_string(),
        profile_id: WALL_CLOCK_PROFILE_ID.to_string(),
        profile_ref: crate::preserves_rail::content_ref_from_bytes(WALL_CLOCK_PROFILE_ID.as_bytes()),
        kind: crate::fabric_time::TimeProfileKind::Live,
        supported_domains: crate::fabric_time::REQUIRED_TIME_DOMAINS.to_vec(),
        max_duration_ticks: WALL_CLOCK_MAX_TICKS,
        max_uncertainty_ticks: WALL_CLOCK_UNIT_BOUND,
        max_timers: WALL_CLOCK_UNIT_BOUND,
        max_runnables: WALL_CLOCK_UNIT_BOUND,
        max_entropy_request_bytes: WALL_CLOCK_UNIT_BOUND,
        max_entropy_total_bytes: WALL_CLOCK_UNIT_BOUND,
        max_scheduler_concurrency: WALL_CLOCK_UNIT_BOUND,
        max_scheduler_queue_depth: WALL_CLOCK_UNIT_BOUND,
        fairness_bound_turns: None,
        scheduler_policy: crate::fabric_time::SchedulerPolicy {
            ordering: crate::fabric_time::SchedulerOrdering::PriorityThenFifo,
            replay: crate::fabric_time::SchedulerReplayPolicy::RecordedChoiceRequired,
            overload: crate::fabric_time::SchedulerOverloadPolicy::Reject,
        },
        evidence_mode: crate::fabric_time::TimeEvidenceMode::SelectedSemanticBoundaries,
        non_claims: crate::fabric_time::REQUIRED_TIME_NON_CLAIMS.to_vec(),
    })?;
    crate::fabric_time::LiveClockAdapter::new(&profile.profile, 0)
}

impl crate::world_witness::WallClock for crate::fabric_time::LiveClockAdapter {
    fn unix_seconds(&mut self) -> crate::error::Result<u64> {
        Ok(std::time::Duration::from_nanos(self.observe_wall()?.unix_nanos).as_secs())
    }
}

/// Live provider ports: the admitted fabric-time wall clock and operating-system request entropy.
pub struct LivePorts {
    clock: crate::fabric_time::LiveClockAdapter,
    entropy: crate::fabric_time::OperatingSystemEntropySource,
}

impl LivePorts {
    pub fn new() -> crate::error::Result<Self> {
        Ok(Self {
            clock: live_clock()?,
            entropy: crate::fabric_time::OperatingSystemEntropySource,
        })
    }
}

impl crate::world_witness::WallClock for LivePorts {
    fn unix_seconds(&mut self) -> crate::error::Result<u64> {
        crate::world_witness::WallClock::unix_seconds(&mut self.clock)
    }
}

impl crate::fabric_time::CryptographicEntropySource for LivePorts {
    fn source_id(&self) -> &'static str {
        crate::fabric_time::CryptographicEntropySource::source_id(&self.entropy)
    }

    fn fill_secret(&mut self, output: &mut [u8]) -> crate::fabric::FabricPortResult<()> {
        crate::fabric_time::CryptographicEntropySource::fill_secret(&mut self.entropy, output)
    }
}

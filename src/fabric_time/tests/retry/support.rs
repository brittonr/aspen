use super::*;

pub(super) const BASE_DELAY: u64 = 2;
pub(super) const ADMITTED_ATTEMPT: u64 = 63;
pub(super) const ATTEMPT_LIMIT: u64 = 65;
pub(super) const PLAN_EVENT_COUNT: usize = 2;
pub(super) const EXECUTION_EVENT_COUNT: usize = PLAN_EVENT_COUNT + 1;
pub(super) const CLOCK_ERROR: &str = "fixture timer unavailable";

pub(super) fn input(profile: &CanonicalTimeProfile) -> fixture_retry::RetryFixtureInput {
    fixture_retry::RetryFixtureInput {
        subject_id: "retry-fixture-test".to_string(),
        generation: GENERATION,
        now: TimeValue::Virtual(VirtualInstant {
            profile_ref: profile.profile.profile_ref.clone(),
            ticks: FIXTURE_RETRY_NOW,
        }),
        attempt: ADMITTED_ATTEMPT,
        policy: RetryPolicy {
            maximum_attempts: ATTEMPT_LIMIT,
            base_delay_ticks: BASE_DELAY,
            maximum_delay_ticks: SATURATED_DELAY,
            backoff: RetryBackoff::Exponential,
            jitter: RetryJitter::None,
        },
        jitter: None,
    }
}

pub(super) fn sentinel(profile: &CanonicalTimeProfile) -> CanonicalTimeEvent {
    canonical_named_event(
        &profile.profile_ref,
        CanonicalTimeEventKind::Conformance,
        GENERATION,
        "existing-consumer-state",
        "preserved",
        FIXTURE_RETRY_NOW,
    )
    .expect("sentinel event")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ClockBehavior {
    Advance,
    Fail,
    ReturnEarly,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct RecordingClock {
    pub profile_ref: String,
    pub domain: TimeDomain,
    pub now: u64,
    pub reads: usize,
    pub waits: Vec<u64>,
    pub behavior: ClockBehavior,
}

impl RecordingClock {
    pub(super) fn new(profile: &CanonicalTimeProfile) -> Self {
        Self {
            profile_ref: profile.profile.profile_ref.clone(),
            domain: TimeDomain::Virtual,
            now: FIXTURE_RETRY_NOW,
            reads: 0,
            waits: Vec::new(),
            behavior: ClockBehavior::Advance,
        }
    }
}

impl TimerClockAdapter for RecordingClock {
    fn profile_ref(&self) -> &str {
        &self.profile_ref
    }

    fn timer_domain(&self) -> TimeDomain {
        self.domain
    }

    fn now_ticks(&mut self) -> crate::fabric::FabricPortResult<u64> {
        self.reads += 1;
        Ok(self.now)
    }

    fn await_ticks(&mut self, target_ticks: u64) -> crate::fabric::FabricPortResult<u64> {
        self.waits.push(target_ticks);
        match self.behavior {
            ClockBehavior::Advance => {
                self.now = target_ticks;
                Ok(self.now)
            }
            ClockBehavior::Fail => Err(crate::fabric::FabricPortError::Timeout {
                message: CLOCK_ERROR.to_string(),
            }),
            ClockBehavior::ReturnEarly => Ok(self.now),
        }
    }
}

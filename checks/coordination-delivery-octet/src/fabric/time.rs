//! Compatibility surface for the already accepted fabric-time core.

#![allow(
    tigerstyle::path_segment_repetition,
    reason = "the stub preserves exact published fabric-time API names"
)]
#![allow(
    tigerstyle::too_many_parameters,
    reason = "the stub preserves the exact accepted plan_retry signature"
)]

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TimeDomain {
    WallClock,
    Monotonic,
    Logical,
    Virtual,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdmittedTimeProfile {
    pub profile_ref: String,
    pub supported_domains: Vec<TimeDomain>,
    pub max_duration_ticks: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LogicalEventTime {
    pub profile_ref: String,
    pub position: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TimeValue {
    Logical(LogicalEventTime),
}

impl TimeValue {
    pub const fn ticks(&self) -> u64 {
        match self {
            Self::Logical(value) => value.position,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CheckedDuration {
    pub profile_ref: String,
    pub domain: TimeDomain,
    pub ticks: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RetryBackoff {
    Fixed,
    Exponential,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RetryJitter {
    None,
    Bounded { maximum_ticks: u64 },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RetryPolicy {
    pub maximum_attempts: u64,
    pub base_delay_ticks: u64,
    pub maximum_delay_ticks: u64,
    pub backoff: RetryBackoff,
    pub jitter: RetryJitter,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Deadline {
    pub target: TimeValue,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RetryPlan {
    pub deadline: Deadline,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StubError {
    ProfileMismatch,
    UnsupportedDomain,
    InvalidPolicy,
    Exhausted,
    Overflow,
}

pub fn checked_add_duration(
    profile: &AdmittedTimeProfile,
    value: &TimeValue,
    duration: &CheckedDuration,
) -> Result<TimeValue, StubError> {
    if duration.profile_ref != profile.profile_ref {
        return Err(StubError::ProfileMismatch);
    }
    if duration.domain != TimeDomain::Logical
        || !profile.supported_domains.contains(&TimeDomain::Logical)
    {
        return Err(StubError::UnsupportedDomain);
    }
    if duration.ticks > profile.max_duration_ticks {
        return Err(StubError::InvalidPolicy);
    }
    let ticks = value
        .ticks()
        .checked_add(duration.ticks)
        .ok_or(StubError::Overflow)?;
    Ok(TimeValue::Logical(LogicalEventTime {
        profile_ref: profile.profile_ref.clone(),
        position: ticks,
    }))
}

pub fn plan_retry(
    profile: &AdmittedTimeProfile,
    _active_generation: u64,
    _subject_id: &str,
    _generation: u64,
    now: &TimeValue,
    attempt: u64,
    policy: RetryPolicy,
    _entropy_jitter_ticks: Option<u64>,
) -> Result<RetryPlan, StubError> {
    if policy.maximum_attempts == 0
        || policy.base_delay_ticks == 0
        || attempt >= policy.maximum_attempts
        || policy.jitter != RetryJitter::None
    {
        return Err(StubError::InvalidPolicy);
    }
    let delay_ticks = match policy.backoff {
        RetryBackoff::Fixed => policy.base_delay_ticks,
        RetryBackoff::Exponential => {
            let shift = u32::try_from(attempt).map_err(|_| StubError::Overflow)?;
            policy
                .base_delay_ticks
                .checked_shl(shift)
                .ok_or(StubError::Overflow)?
        }
    }
    .min(policy.maximum_delay_ticks);
    let target = checked_add_duration(
        profile,
        now,
        &CheckedDuration {
            profile_ref: profile.profile_ref.clone(),
            domain: TimeDomain::Logical,
            ticks: delay_ticks,
        },
    )?;
    Ok(RetryPlan {
        deadline: Deadline { target },
    })
}

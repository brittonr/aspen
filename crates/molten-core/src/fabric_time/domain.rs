use std::cmp::Ordering;

use super::AdmittedTimeProfile;
use super::TimeDomain;
use super::valid_time_ref;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WallClockObservation {
    pub profile_ref: String,
    pub unix_nanos: u64,
    pub uncertainty_nanos: u64,
    pub observation_sequence: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonotonicInstant {
    pub profile_ref: String,
    pub ticks: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogicalEventTime {
    pub profile_ref: String,
    pub position: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VirtualInstant {
    pub profile_ref: String,
    pub ticks: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckedDuration {
    pub profile_ref: String,
    pub domain: TimeDomain,
    pub ticks: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimeValue {
    Wall(WallClockObservation),
    Monotonic(MonotonicInstant),
    Logical(LogicalEventTime),
    Virtual(VirtualInstant),
}

impl TimeValue {
    pub const fn domain(&self) -> TimeDomain {
        match self {
            Self::Wall(_) => TimeDomain::WallClock,
            Self::Monotonic(_) => TimeDomain::Monotonic,
            Self::Logical(_) => TimeDomain::Logical,
            Self::Virtual(_) => TimeDomain::Virtual,
        }
    }

    pub fn profile_ref(&self) -> &str {
        match self {
            Self::Wall(value) => &value.profile_ref,
            Self::Monotonic(value) => &value.profile_ref,
            Self::Logical(value) => &value.profile_ref,
            Self::Virtual(value) => &value.profile_ref,
        }
    }

    pub const fn ticks(&self) -> u64 {
        match self {
            Self::Wall(value) => value.unix_nanos,
            Self::Monotonic(value) => value.ticks,
            Self::Logical(value) => value.position,
            Self::Virtual(value) => value.ticks,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimeArithmeticError {
    ProfileMismatch { expected: String, actual: String },
    DomainMismatch { expected: TimeDomain, actual: TimeDomain },
    UnsupportedDomain(TimeDomain),
    DurationLimitExceeded { actual: u64, maximum: u64 },
    UncertaintyLimitExceeded { actual: u64, maximum: u64 },
    Overflow,
    Underflow,
    NonIncreasingSequence,
    MalformedConversionEvidence(String),
    ConversionSourceMismatch,
    ConversionTargetMismatch,
}

pub fn validate_time_value(profile: &AdmittedTimeProfile, value: &TimeValue) -> Result<(), TimeArithmeticError> {
    validate_profile_and_domain(profile, value.profile_ref(), value.domain())?;
    if let TimeValue::Wall(observation) = value
        && observation.uncertainty_nanos > profile.max_uncertainty_ticks
    {
        return Err(TimeArithmeticError::UncertaintyLimitExceeded {
            actual: observation.uncertainty_nanos,
            maximum: profile.max_uncertainty_ticks,
        });
    }
    Ok(())
}

pub fn validate_duration(profile: &AdmittedTimeProfile, duration: &CheckedDuration) -> Result<(), TimeArithmeticError> {
    validate_profile_and_domain(profile, &duration.profile_ref, duration.domain)?;
    if duration.ticks > profile.max_duration_ticks {
        return Err(TimeArithmeticError::DurationLimitExceeded {
            actual: duration.ticks,
            maximum: profile.max_duration_ticks,
        });
    }
    Ok(())
}

// r[impl molten.fabric_time.time_domains]
pub fn compare_time_values(
    profile: &AdmittedTimeProfile,
    left: &TimeValue,
    right: &TimeValue,
) -> Result<Ordering, TimeArithmeticError> {
    validate_time_value(profile, left)?;
    validate_time_value(profile, right)?;
    if left.domain() != right.domain() {
        return Err(TimeArithmeticError::DomainMismatch {
            expected: left.domain(),
            actual: right.domain(),
        });
    }
    Ok(left.ticks().cmp(&right.ticks()))
}

pub fn checked_add_duration(
    profile: &AdmittedTimeProfile,
    value: &TimeValue,
    duration: &CheckedDuration,
) -> Result<TimeValue, TimeArithmeticError> {
    validate_time_value(profile, value)?;
    validate_duration(profile, duration)?;
    if value.domain() != duration.domain {
        return Err(TimeArithmeticError::DomainMismatch {
            expected: value.domain(),
            actual: duration.domain,
        });
    }
    let ticks = value.ticks().checked_add(duration.ticks).ok_or(TimeArithmeticError::Overflow)?;
    Ok(time_value_with_ticks(value, ticks))
}

pub fn checked_sub_duration(
    profile: &AdmittedTimeProfile,
    value: &TimeValue,
    duration: &CheckedDuration,
) -> Result<TimeValue, TimeArithmeticError> {
    validate_time_value(profile, value)?;
    validate_duration(profile, duration)?;
    if value.domain() != duration.domain {
        return Err(TimeArithmeticError::DomainMismatch {
            expected: value.domain(),
            actual: duration.domain,
        });
    }
    let ticks = value.ticks().checked_sub(duration.ticks).ok_or(TimeArithmeticError::Underflow)?;
    Ok(time_value_with_ticks(value, ticks))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExplicitTimeConversion {
    pub source_profile_ref: String,
    pub target_profile_ref: String,
    pub source_domain: TimeDomain,
    pub target_domain: TimeDomain,
    pub signed_offset_ticks: i128,
    pub uncertainty_ticks: u64,
    pub target_observation_sequence: u64,
    pub conversion_evidence_ref: String,
}

// Conversions are never implicit. The caller supplies the conversion evidence,
// offset, target profile, and uncertainty policy explicitly.
pub fn convert_time_value(
    source_profile: &AdmittedTimeProfile,
    target_profile: &AdmittedTimeProfile,
    value: &TimeValue,
    conversion: &ExplicitTimeConversion,
) -> Result<TimeValue, TimeArithmeticError> {
    validate_time_value(source_profile, value)?;
    if conversion.source_profile_ref != source_profile.profile_ref || conversion.source_domain != value.domain() {
        return Err(TimeArithmeticError::ConversionSourceMismatch);
    }
    if conversion.target_profile_ref != target_profile.profile_ref
        || !target_profile.supported_domains.contains(&conversion.target_domain)
    {
        return Err(TimeArithmeticError::ConversionTargetMismatch);
    }
    if !valid_time_ref(&conversion.conversion_evidence_ref) {
        return Err(TimeArithmeticError::MalformedConversionEvidence(conversion.conversion_evidence_ref.clone()));
    }
    if conversion.uncertainty_ticks > target_profile.max_uncertainty_ticks {
        return Err(TimeArithmeticError::UncertaintyLimitExceeded {
            actual: conversion.uncertainty_ticks,
            maximum: target_profile.max_uncertainty_ticks,
        });
    }
    let source_ticks = i128::from(value.ticks());
    let converted = source_ticks.checked_add(conversion.signed_offset_ticks).ok_or(TimeArithmeticError::Overflow)?;
    let ticks = u64::try_from(converted).map_err(|_| TimeArithmeticError::Underflow)?;
    Ok(match conversion.target_domain {
        TimeDomain::WallClock => TimeValue::Wall(WallClockObservation {
            profile_ref: target_profile.profile_ref.clone(),
            unix_nanos: ticks,
            uncertainty_nanos: conversion.uncertainty_ticks,
            observation_sequence: conversion.target_observation_sequence,
        }),
        TimeDomain::Monotonic => TimeValue::Monotonic(MonotonicInstant {
            profile_ref: target_profile.profile_ref.clone(),
            ticks,
        }),
        TimeDomain::Logical => TimeValue::Logical(LogicalEventTime {
            profile_ref: target_profile.profile_ref.clone(),
            position: ticks,
        }),
        TimeDomain::Virtual => TimeValue::Virtual(VirtualInstant {
            profile_ref: target_profile.profile_ref.clone(),
            ticks,
        }),
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WallClockAnomalyPolicy {
    pub max_forward_jump_nanos: u64,
    pub max_uncertainty_nanos: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WallClockAnomalyKind {
    Stable,
    BackwardJump,
    ForwardJump,
    ExcessiveUncertainty,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WallClockAnomalyDecision {
    pub kind: WallClockAnomalyKind,
    pub previous_unix_nanos: u64,
    pub observed_unix_nanos: u64,
    pub delta_nanos: u64,
}

// r[impl molten.fabric_time.time_domains]
pub fn classify_wall_clock_observation(
    previous: &WallClockObservation,
    observed: &WallClockObservation,
    policy: WallClockAnomalyPolicy,
) -> Result<WallClockAnomalyDecision, TimeArithmeticError> {
    if previous.profile_ref != observed.profile_ref {
        return Err(TimeArithmeticError::ProfileMismatch {
            expected: previous.profile_ref.clone(),
            actual: observed.profile_ref.clone(),
        });
    }
    if observed.observation_sequence <= previous.observation_sequence {
        return Err(TimeArithmeticError::NonIncreasingSequence);
    }
    let (kind, delta_nanos) = if observed.uncertainty_nanos > policy.max_uncertainty_nanos {
        (WallClockAnomalyKind::ExcessiveUncertainty, observed.unix_nanos.abs_diff(previous.unix_nanos))
    } else if observed.unix_nanos < previous.unix_nanos {
        (WallClockAnomalyKind::BackwardJump, previous.unix_nanos - observed.unix_nanos)
    } else {
        let delta = observed.unix_nanos - previous.unix_nanos;
        if delta > policy.max_forward_jump_nanos {
            (WallClockAnomalyKind::ForwardJump, delta)
        } else {
            (WallClockAnomalyKind::Stable, delta)
        }
    };
    Ok(WallClockAnomalyDecision {
        kind,
        previous_unix_nanos: previous.unix_nanos,
        observed_unix_nanos: observed.unix_nanos,
        delta_nanos,
    })
}

fn validate_profile_and_domain(
    profile: &AdmittedTimeProfile,
    profile_ref: &str,
    domain: TimeDomain,
) -> Result<(), TimeArithmeticError> {
    if profile.profile_ref != profile_ref {
        return Err(TimeArithmeticError::ProfileMismatch {
            expected: profile.profile_ref.clone(),
            actual: profile_ref.to_string(),
        });
    }
    if !profile.supported_domains.contains(&domain) {
        return Err(TimeArithmeticError::UnsupportedDomain(domain));
    }
    Ok(())
}

fn time_value_with_ticks(value: &TimeValue, ticks: u64) -> TimeValue {
    match value {
        TimeValue::Wall(value) => TimeValue::Wall(WallClockObservation {
            profile_ref: value.profile_ref.clone(),
            unix_nanos: ticks,
            uncertainty_nanos: value.uncertainty_nanos,
            observation_sequence: value.observation_sequence,
        }),
        TimeValue::Monotonic(value) => TimeValue::Monotonic(MonotonicInstant {
            profile_ref: value.profile_ref.clone(),
            ticks,
        }),
        TimeValue::Logical(value) => TimeValue::Logical(LogicalEventTime {
            profile_ref: value.profile_ref.clone(),
            position: ticks,
        }),
        TimeValue::Virtual(value) => TimeValue::Virtual(VirtualInstant {
            profile_ref: value.profile_ref.clone(),
            ticks,
        }),
    }
}

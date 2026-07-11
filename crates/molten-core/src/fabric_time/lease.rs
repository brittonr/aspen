use super::AdmittedTimeProfile;
use super::CheckedDuration;
use super::TimeArithmeticError;
use super::TimeDomain;
use super::TimeValue;
use super::checked_add_duration;
use super::valid_time_id;
use super::validate_time_value;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Deadline {
    pub profile_ref: String,
    pub subject_id: String,
    pub generation: u64,
    pub target: TimeValue,
    pub uncertainty_ticks: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeadlineStatus {
    Pending,
    Expired,
    IndeterminateWithinUncertainty,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeadlineDecision {
    pub subject_id: String,
    pub generation: u64,
    pub domain: TimeDomain,
    pub status: DeadlineStatus,
    pub observed_ticks: u64,
    pub target_ticks: u64,
    pub uncertainty_ticks: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetryBackoff {
    Fixed,
    Exponential,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetryJitter {
    None,
    Bounded { maximum_ticks: u64 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetryPolicy {
    pub maximum_attempts: u64,
    pub base_delay_ticks: u64,
    pub maximum_delay_ticks: u64,
    pub backoff: RetryBackoff,
    pub jitter: RetryJitter,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetryPlan {
    pub attempt: u64,
    pub delay: CheckedDuration,
    pub deadline: Deadline,
    pub jitter_ticks: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeaseConsistency {
    LocalObservationOnly,
    FencedExclusive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeaseAction {
    Observe,
    Renew,
    AcquireExclusive,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeaseRequest {
    pub lease_id: String,
    pub owner_id: String,
    pub generation: u64,
    pub now: TimeValue,
    pub expires_at: TimeValue,
    pub uncertainty_ticks: u64,
    pub consistency: LeaseConsistency,
    pub action: LeaseAction,
    pub fencing_token: Option<u64>,
    pub previous_fencing_token: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeaseDecisionKind {
    LocallyActive,
    LocallyExpired,
    IndeterminateWithinUncertainty,
    RenewalAllowed,
    ExclusiveActionAllowed,
    DeniedWithoutFencing,
    DeniedStaleFencingToken,
    DeniedExpired,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeaseDecision {
    pub lease_id: String,
    pub owner_id: String,
    pub generation: u64,
    pub kind: LeaseDecisionKind,
    pub fencing_token: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeadlineLeaseError {
    MalformedId { field: &'static str, value: String },
    ZeroGeneration,
    StaleGeneration { expected: u64, actual: u64 },
    ProfileMismatch,
    DomainMismatch,
    UnsupportedDeadlineDomain(TimeDomain),
    UncertaintyLimitExceeded { actual: u64, maximum: u64 },
    InvalidRetryPolicy,
    RetryExhausted { attempt: u64, maximum: u64 },
    JitterRequired,
    JitterOutOfBounds { actual: u64, maximum: u64 },
    Arithmetic(TimeArithmeticError),
    Overflow,
}

// r[impl molten.fabric_time.deadline_lease]
pub fn evaluate_deadline(
    profile: &AdmittedTimeProfile,
    active_generation: u64,
    deadline: &Deadline,
    observed: &TimeValue,
) -> Result<DeadlineDecision, DeadlineLeaseError> {
    validate_deadline(profile, active_generation, deadline)?;
    validate_time_value(profile, observed).map_err(DeadlineLeaseError::Arithmetic)?;
    if deadline.target.domain() != observed.domain() {
        return Err(DeadlineLeaseError::DomainMismatch);
    }
    let status = classify_with_uncertainty(observed.ticks(), deadline.target.ticks(), deadline.uncertainty_ticks);
    Ok(DeadlineDecision {
        subject_id: deadline.subject_id.clone(),
        generation: deadline.generation,
        domain: deadline.target.domain(),
        status,
        observed_ticks: observed.ticks(),
        target_ticks: deadline.target.ticks(),
        uncertainty_ticks: deadline.uncertainty_ticks,
    })
}

pub fn plan_retry(
    profile: &AdmittedTimeProfile,
    active_generation: u64,
    subject_id: &str,
    generation: u64,
    now: &TimeValue,
    attempt: u64,
    policy: RetryPolicy,
    entropy_jitter_ticks: Option<u64>,
) -> Result<RetryPlan, DeadlineLeaseError> {
    validate_retry_policy(policy)?;
    if generation == 0 {
        return Err(DeadlineLeaseError::ZeroGeneration);
    }
    ensure_generation(active_generation, generation)?;
    if !valid_time_id(subject_id) {
        return Err(DeadlineLeaseError::MalformedId {
            field: "subject-id",
            value: subject_id.to_string(),
        });
    }
    validate_time_value(profile, now).map_err(DeadlineLeaseError::Arithmetic)?;
    if attempt >= policy.maximum_attempts {
        return Err(DeadlineLeaseError::RetryExhausted {
            attempt,
            maximum: policy.maximum_attempts,
        });
    }
    let base = match policy.backoff {
        RetryBackoff::Fixed => policy.base_delay_ticks,
        RetryBackoff::Exponential => {
            let shift = u32::try_from(attempt).map_err(|_| DeadlineLeaseError::Overflow)?;
            policy.base_delay_ticks.checked_shl(shift).unwrap_or(u64::MAX).min(policy.maximum_delay_ticks)
        }
    };
    let jitter_ticks = match policy.jitter {
        RetryJitter::None => {
            if entropy_jitter_ticks.is_some() {
                return Err(DeadlineLeaseError::JitterOutOfBounds {
                    actual: entropy_jitter_ticks.unwrap_or_default(),
                    maximum: 0,
                });
            }
            0
        }
        RetryJitter::Bounded { maximum_ticks } => {
            let supplied = entropy_jitter_ticks.ok_or(DeadlineLeaseError::JitterRequired)?;
            if supplied > maximum_ticks {
                return Err(DeadlineLeaseError::JitterOutOfBounds {
                    actual: supplied,
                    maximum: maximum_ticks,
                });
            }
            supplied
        }
    };
    let delay_ticks =
        base.checked_add(jitter_ticks).ok_or(DeadlineLeaseError::Overflow)?.min(policy.maximum_delay_ticks);
    let delay = CheckedDuration {
        profile_ref: profile.profile_ref.clone(),
        domain: now.domain(),
        ticks: delay_ticks,
    };
    let target = checked_add_duration(profile, now, &delay).map_err(DeadlineLeaseError::Arithmetic)?;
    Ok(RetryPlan {
        attempt,
        delay,
        deadline: Deadline {
            profile_ref: profile.profile_ref.clone(),
            subject_id: subject_id.to_string(),
            generation,
            target,
            uncertainty_ticks: 0,
        },
        jitter_ticks,
    })
}

// r[impl molten.fabric_time.deadline_lease]
pub fn evaluate_lease(
    profile: &AdmittedTimeProfile,
    active_generation: u64,
    request: &LeaseRequest,
) -> Result<LeaseDecision, DeadlineLeaseError> {
    validate_lease_request(profile, active_generation, request)?;
    let status = classify_with_uncertainty(request.now.ticks(), request.expires_at.ticks(), request.uncertainty_ticks);
    let kind = match (request.action, status) {
        (LeaseAction::Observe, DeadlineStatus::Pending) => LeaseDecisionKind::LocallyActive,
        (LeaseAction::Observe, DeadlineStatus::Expired) => LeaseDecisionKind::LocallyExpired,
        (_, DeadlineStatus::IndeterminateWithinUncertainty) => LeaseDecisionKind::IndeterminateWithinUncertainty,
        (LeaseAction::Renew, DeadlineStatus::Pending) => LeaseDecisionKind::RenewalAllowed,
        (LeaseAction::Renew, DeadlineStatus::Expired) | (LeaseAction::AcquireExclusive, DeadlineStatus::Expired) => {
            LeaseDecisionKind::DeniedExpired
        }
        (LeaseAction::AcquireExclusive, DeadlineStatus::Pending) => classify_exclusive_lease(request),
    };
    Ok(LeaseDecision {
        lease_id: request.lease_id.clone(),
        owner_id: request.owner_id.clone(),
        generation: request.generation,
        kind,
        fencing_token: request.fencing_token,
    })
}

fn validate_deadline(
    profile: &AdmittedTimeProfile,
    active_generation: u64,
    deadline: &Deadline,
) -> Result<(), DeadlineLeaseError> {
    if deadline.profile_ref != profile.profile_ref {
        return Err(DeadlineLeaseError::ProfileMismatch);
    }
    if !valid_time_id(&deadline.subject_id) {
        return Err(DeadlineLeaseError::MalformedId {
            field: "subject-id",
            value: deadline.subject_id.clone(),
        });
    }
    if deadline.generation == 0 {
        return Err(DeadlineLeaseError::ZeroGeneration);
    }
    ensure_generation(active_generation, deadline.generation)?;
    validate_deadline_domain(deadline.target.domain())?;
    validate_time_value(profile, &deadline.target).map_err(DeadlineLeaseError::Arithmetic)?;
    validate_uncertainty(profile, deadline.uncertainty_ticks)
}

fn validate_lease_request(
    profile: &AdmittedTimeProfile,
    active_generation: u64,
    request: &LeaseRequest,
) -> Result<(), DeadlineLeaseError> {
    for (field, value) in [("lease-id", &request.lease_id), ("owner-id", &request.owner_id)] {
        if !valid_time_id(value) {
            return Err(DeadlineLeaseError::MalformedId {
                field,
                value: value.clone(),
            });
        }
    }
    if request.generation == 0 {
        return Err(DeadlineLeaseError::ZeroGeneration);
    }
    ensure_generation(active_generation, request.generation)?;
    validate_time_value(profile, &request.now).map_err(DeadlineLeaseError::Arithmetic)?;
    validate_time_value(profile, &request.expires_at).map_err(DeadlineLeaseError::Arithmetic)?;
    if request.now.domain() != request.expires_at.domain() {
        return Err(DeadlineLeaseError::DomainMismatch);
    }
    validate_deadline_domain(request.now.domain())?;
    validate_uncertainty(profile, request.uncertainty_ticks)
}

fn validate_deadline_domain(domain: TimeDomain) -> Result<(), DeadlineLeaseError> {
    if domain == TimeDomain::WallClock {
        return Err(DeadlineLeaseError::UnsupportedDeadlineDomain(domain));
    }
    Ok(())
}

fn validate_uncertainty(profile: &AdmittedTimeProfile, uncertainty: u64) -> Result<(), DeadlineLeaseError> {
    if uncertainty > profile.max_uncertainty_ticks {
        return Err(DeadlineLeaseError::UncertaintyLimitExceeded {
            actual: uncertainty,
            maximum: profile.max_uncertainty_ticks,
        });
    }
    Ok(())
}

fn classify_with_uncertainty(now: u64, target: u64, uncertainty: u64) -> DeadlineStatus {
    let earliest_now = now.saturating_sub(uncertainty);
    let latest_now = now.saturating_add(uncertainty);
    if latest_now < target {
        DeadlineStatus::Pending
    } else if earliest_now >= target {
        DeadlineStatus::Expired
    } else {
        DeadlineStatus::IndeterminateWithinUncertainty
    }
}

fn classify_exclusive_lease(request: &LeaseRequest) -> LeaseDecisionKind {
    if request.consistency != LeaseConsistency::FencedExclusive || request.fencing_token.is_none() {
        return LeaseDecisionKind::DeniedWithoutFencing;
    }
    let token = request.fencing_token.unwrap_or_default();
    if request.previous_fencing_token.is_some_and(|previous| token <= previous) {
        return LeaseDecisionKind::DeniedStaleFencingToken;
    }
    LeaseDecisionKind::ExclusiveActionAllowed
}

fn validate_retry_policy(policy: RetryPolicy) -> Result<(), DeadlineLeaseError> {
    if policy.maximum_attempts == 0
        || policy.base_delay_ticks == 0
        || policy.maximum_delay_ticks == 0
        || policy.base_delay_ticks > policy.maximum_delay_ticks
        || matches!(policy.jitter, RetryJitter::Bounded { maximum_ticks: 0 })
    {
        return Err(DeadlineLeaseError::InvalidRetryPolicy);
    }
    Ok(())
}

fn ensure_generation(expected: u64, actual: u64) -> Result<(), DeadlineLeaseError> {
    if expected != actual {
        return Err(DeadlineLeaseError::StaleGeneration { expected, actual });
    }
    Ok(())
}

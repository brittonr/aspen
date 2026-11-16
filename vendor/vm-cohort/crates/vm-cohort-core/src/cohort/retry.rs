use serde::{Deserialize, Serialize};

use crate::{OperationRef, ResourceLimits};

/// Closed failure class used by retry policy.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FailureClass {
    /// KVM reported a reviewed transient resource condition.
    TransientKvm,
    /// Current host resources are insufficient.
    ResourceExhausted,
    /// Checkpoint and host compatibility differ.
    Compatibility,
    /// Effect status is unknown.
    OutcomeUnknown,
    /// Permanent adapter failure.
    Permanent,
}

/// Pure retry policy input.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RetryInput {
    /// Logical operation identity retained across attempts.
    pub operation_ref: OperationRef,
    /// Failure classification.
    pub failure: FailureClass,
    /// Number of completed attempts.
    pub attempts_completed: u32,
    /// Current cancellation state.
    pub cancelled: bool,
    /// Selected reviewed limits.
    pub limits: ResourceLimits,
}

/// Bounded retry plan.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RetryPlan {
    /// Logical operation identity.
    pub operation_ref: OperationRef,
    /// Next one-based attempt number.
    pub next_attempt: u32,
    /// Declared delay before the next attempt.
    pub delay_ticks: u64,
}

/// Plans a retry only for an explicit transient KVM failure.
#[must_use]
// r[impl vm_cohort.retry_policy]
pub fn plan_retry(input: &RetryInput) -> Option<RetryPlan> {
    if input.cancelled || input.failure != FailureClass::TransientKvm {
        return None;
    }
    let next_attempt = input.attempts_completed.checked_add(1)?;
    if next_attempt > input.limits.max_attempts
        || input.limits.retry_delay_ticks == 0
        || input.limits.retry_delay_ticks > input.limits.max_retry_delay_ticks
    {
        return None;
    }
    Some(RetryPlan {
        operation_ref: input.operation_ref.clone(),
        next_attempt,
        delay_ticks: input.limits.retry_delay_ticks,
    })
}

//! Pure scheduler-capacity planning, fencing, and accounting.

mod identity;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    pub profile_ref: String,
    pub generation: u64,
    pub runnable_slots: u64,
    pub queue_slots: u64,
    pub concurrency_slots: u64,
    pub total_slots: u64,
    pub plan_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlanIssue {
    ZeroGeneration,
    ZeroLimit(&'static str),
    QueueExceedsRunnables,
    ConcurrencyExceedsRunnables,
    HardLimitExceeded {
        field: &'static str,
        actual: u64,
        maximum: u64,
    },
    CountUnrepresentable(&'static str),
    AllocationArithmeticOverflow,
}

// r[impl molten.fabric_time.scheduler_capacity.plan]
pub fn derive(profile: &super::super::AdmittedTimeProfile, generation: u64) -> Result<Plan, PlanIssue> {
    if generation == 0 {
        return Err(PlanIssue::ZeroGeneration);
    }
    validate_positive("runnable-slots", profile.max_runnables)?;
    validate_positive("queue-slots", profile.max_scheduler_queue_depth)?;
    validate_positive("concurrency-slots", profile.max_scheduler_concurrency)?;
    validate_maximum("runnable-slots", profile.max_runnables, super::super::profile::MAX_PROFILE_RUNNABLES)?;
    validate_maximum("queue-slots", profile.max_scheduler_queue_depth, super::super::profile::MAX_PROFILE_QUEUE_DEPTH)?;
    validate_maximum(
        "concurrency-slots",
        profile.max_scheduler_concurrency,
        super::super::profile::MAX_PROFILE_CONCURRENCY,
    )?;
    if profile.max_scheduler_queue_depth > profile.max_runnables {
        return Err(PlanIssue::QueueExceedsRunnables);
    }
    if profile.max_scheduler_concurrency > profile.max_runnables {
        return Err(PlanIssue::ConcurrencyExceedsRunnables);
    }
    validate_count("runnable-slots", profile.max_runnables)?;
    validate_count("queue-slots", profile.max_scheduler_queue_depth)?;
    validate_count("concurrency-slots", profile.max_scheduler_concurrency)?;
    let total_slots =
        checked_total(profile.max_runnables, profile.max_scheduler_queue_depth, profile.max_scheduler_concurrency)?;
    let plan_ref = identity::plan(&identity::PlanInput {
        profile_ref: &profile.profile_ref,
        generation,
        runnable_slots: profile.max_runnables,
        queue_slots: profile.max_scheduler_queue_depth,
        concurrency_slots: profile.max_scheduler_concurrency,
        total_slots,
    });
    Ok(Plan {
        profile_ref: profile.profile_ref.clone(),
        generation,
        runnable_slots: profile.max_runnables,
        queue_slots: profile.max_scheduler_queue_depth,
        concurrency_slots: profile.max_scheduler_concurrency,
        total_slots,
        plan_ref,
    })
}

fn validate_positive(field: &'static str, value: u64) -> Result<(), PlanIssue> {
    if value == 0 {
        Err(PlanIssue::ZeroLimit(field))
    } else {
        Ok(())
    }
}

fn validate_maximum(field: &'static str, actual: u64, maximum: u64) -> Result<(), PlanIssue> {
    if actual > maximum {
        Err(PlanIssue::HardLimitExceeded { field, actual, maximum })
    } else {
        Ok(())
    }
}

fn validate_count(field: &'static str, value: u64) -> Result<(), PlanIssue> {
    validate_count_against(field, value, platform_maximum())
}

#[cfg(target_pointer_width = "64")]
const fn platform_maximum() -> u64 {
    u64::MAX
}

#[cfg(target_pointer_width = "32")]
const fn platform_maximum() -> u64 {
    u64::from(u32::MAX)
}

#[cfg(target_pointer_width = "16")]
const fn platform_maximum() -> u64 {
    u64::from(u16::MAX)
}

fn validate_count_against(field: &'static str, value: u64, maximum: u64) -> Result<(), PlanIssue> {
    if value > maximum {
        Err(PlanIssue::CountUnrepresentable(field))
    } else {
        Ok(())
    }
}

fn checked_total(runnable_slots: u64, queue_slots: u64, concurrency_slots: u64) -> Result<u64, PlanIssue> {
    runnable_slots
        .checked_add(queue_slots)
        .and_then(|sum| sum.checked_add(concurrency_slots))
        .ok_or(PlanIssue::AllocationArithmeticOverflow)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UseState {
    pub plan_ref: String,
    pub profile_ref: String,
    pub generation: u64,
    pub runnable_usage: u64,
    pub queue_usage: u64,
    pub runnable_high_water: u64,
    pub queue_high_water: u64,
    pub exhaustion_count: u64,
    pub is_released: bool,
}

impl UseState {
    pub fn activated(plan: &Plan) -> Self {
        Self {
            plan_ref: plan.plan_ref.clone(),
            profile_ref: plan.profile_ref.clone(),
            generation: plan.generation,
            runnable_usage: 0,
            queue_usage: 0,
            runnable_high_water: 0,
            queue_high_water: 0,
            exhaustion_count: 0,
            is_released: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UseDecisionKind {
    Admit,
    Exhausted,
    StaleGeneration,
    ProfileMismatch,
    PlanMismatch,
    Released,
    Underflow,
    Overflow,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UseDecision {
    pub kind: UseDecisionKind,
    pub next: UseState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UseRequest {
    pub runnable_delta: i64,
    pub queue_delta: i64,
}

// r[impl molten.fabric_time.scheduler_capacity.steady_state]
// r[impl molten.fabric_time.scheduler_capacity.boundary]
pub fn apply_use(
    plan: &Plan,
    state: &UseState,
    profile_ref: &str,
    generation: u64,
    request: UseRequest,
) -> UseDecision {
    if state.is_released {
        return unchanged(state, UseDecisionKind::Released);
    }
    if state.plan_ref != plan.plan_ref {
        return unchanged(state, UseDecisionKind::PlanMismatch);
    }
    if state.profile_ref != profile_ref || plan.profile_ref != profile_ref {
        return unchanged(state, UseDecisionKind::ProfileMismatch);
    }
    if state.generation != generation || plan.generation != generation {
        return unchanged(state, UseDecisionKind::StaleGeneration);
    }
    let Some(runnable_usage) = apply_delta(state.runnable_usage, request.runnable_delta) else {
        return unchanged(state, delta_error(state.runnable_usage, request.runnable_delta));
    };
    let Some(queue_usage) = apply_delta(state.queue_usage, request.queue_delta) else {
        return unchanged(state, delta_error(state.queue_usage, request.queue_delta));
    };
    if runnable_usage > plan.runnable_slots || queue_usage > plan.queue_slots {
        let mut next = state.clone();
        next.exhaustion_count = next.exhaustion_count.saturating_add(1);
        return UseDecision {
            kind: UseDecisionKind::Exhausted,
            next,
        };
    }
    let mut next = state.clone();
    next.runnable_usage = runnable_usage;
    next.queue_usage = queue_usage;
    next.runnable_high_water = next.runnable_high_water.max(runnable_usage);
    next.queue_high_water = next.queue_high_water.max(queue_usage);
    UseDecision {
        kind: UseDecisionKind::Admit,
        next,
    }
}

fn apply_delta(current: u64, delta: i64) -> Option<u64> {
    if delta >= 0 {
        current.checked_add(delta.unsigned_abs())
    } else {
        current.checked_sub(delta.unsigned_abs())
    }
}

fn delta_error(current: u64, delta: i64) -> UseDecisionKind {
    if delta < 0 && delta.unsigned_abs() > current {
        UseDecisionKind::Underflow
    } else {
        UseDecisionKind::Overflow
    }
}

fn unchanged(state: &UseState, kind: UseDecisionKind) -> UseDecision {
    UseDecision {
        kind,
        next: state.clone(),
    }
}

// r[impl molten.fabric_time.scheduler_capacity.steady_state]
pub fn release(state: &UseState) -> UseState {
    let mut next = state.clone();
    next.runnable_usage = 0;
    next.queue_usage = 0;
    next.is_released = true;
    next
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Observation {
    pub plan_ref: String,
    pub profile_ref: String,
    pub generation: u64,
    pub runnable_usage: u64,
    pub queue_usage: u64,
    pub runnable_high_water: u64,
    pub queue_high_water: u64,
    pub exhaustion_count: u64,
    pub is_released: bool,
    pub observation_ref: String,
    pub non_claims: Vec<&'static str>,
}

// r[impl molten.fabric_time.scheduler_capacity.observation]
pub fn observe(state: &UseState) -> Observation {
    let observation_ref = identity::observation(state);
    Observation {
        plan_ref: state.plan_ref.clone(),
        profile_ref: state.profile_ref.clone(),
        generation: state.generation,
        runnable_usage: state.runnable_usage,
        queue_usage: state.queue_usage,
        runnable_high_water: state.runnable_high_water,
        queue_high_water: state.queue_high_water,
        exhaustion_count: state.exhaustion_count,
        is_released: state.is_released,
        observation_ref,
        non_claims: vec![
            "does-not-prove-global-latency",
            "does-not-prove-fairness",
            "does-not-prove-liveness",
            "does-not-prove-host-memory-stability",
            "does-not-prove-whole-runtime-zero-allocation",
        ],
    }
}

#[cfg(test)]
mod tests;

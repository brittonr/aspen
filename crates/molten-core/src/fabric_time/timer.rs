use super::AdmittedTimeProfile;
use super::TimeDomain;
use super::valid_time_id;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct TimerKey {
    pub service_id: String,
    pub generation: u64,
    pub sequence: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimerKind {
    OneShot,
    Periodic { period_ticks: u64 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimerCoalescingPolicy {
    DeliverEach { max_catch_up: u64 },
    CoalesceLatest,
    SkipMissed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimerLatenessPolicy {
    DeliverWithin { max_lateness_ticks: u64 },
    DeliverRegardless,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimerOverloadPolicy {
    RejectAndRetain,
    Backpressure,
    DropDue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimerResourceCharge {
    pub timer_slots: u64,
    pub delivery_queue_units: u64,
}

impl TimerResourceCharge {
    pub const fn single_slot() -> Self {
        Self {
            timer_slots: 1,
            delivery_queue_units: 1,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimerScheduleRequest {
    pub profile_ref: String,
    pub key: TimerKey,
    pub domain: TimeDomain,
    pub deadline_ticks: u64,
    pub kind: TimerKind,
    pub ordering_key: u64,
    pub coalescing: TimerCoalescingPolicy,
    pub lateness: TimerLatenessPolicy,
    pub overload: TimerOverloadPolicy,
    pub resource_charge: TimerResourceCharge,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimerPhase {
    Scheduled,
    Cancelled,
    Completed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimerState {
    pub profile_ref: String,
    pub key: TimerKey,
    pub domain: TimeDomain,
    pub next_deadline_ticks: u64,
    pub kind: TimerKind,
    pub ordering_key: u64,
    pub coalescing: TimerCoalescingPolicy,
    pub lateness: TimerLatenessPolicy,
    pub overload: TimerOverloadPolicy,
    pub resource_charge: TimerResourceCharge,
    pub phase: TimerPhase,
    pub fire_count: u64,
    pub skipped_count: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimerAction {
    NotDue,
    Deliver,
    Coalesced,
    DroppedLate,
    DroppedOverload,
    Backpressure,
    RetainedOverload,
    Cancelled,
    DiscardedStaleGeneration,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimerTransition {
    pub next: TimerState,
    pub action: TimerAction,
    pub delivery_count: u64,
    pub skipped_count: u64,
    pub lateness_ticks: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimerError {
    MalformedServiceId(String),
    ZeroGeneration,
    StaleGeneration { expected: u64, actual: u64 },
    ProfileMismatch,
    UnsupportedDomain(TimeDomain),
    TimerLimitExceeded { active: u64, maximum: u64 },
    InvalidPeriod,
    InvalidCatchUpBound,
    InvalidResourceCharge,
    TerminalTimer(TimerPhase),
    Overflow,
}

// r[impl molten.fabric_time.timers]
pub fn schedule_timer(
    profile: &AdmittedTimeProfile,
    active_generation: u64,
    active_timer_slots: u64,
    request: &TimerScheduleRequest,
) -> Result<TimerState, TimerError> {
    if !valid_time_id(&request.key.service_id) {
        return Err(TimerError::MalformedServiceId(request.key.service_id.clone()));
    }
    if request.key.generation == 0 {
        return Err(TimerError::ZeroGeneration);
    }
    ensure_generation(active_generation, request.key.generation)?;
    if request.profile_ref != profile.profile_ref {
        return Err(TimerError::ProfileMismatch);
    }
    if !profile.supported_domains.contains(&request.domain) {
        return Err(TimerError::UnsupportedDomain(request.domain));
    }
    if request.resource_charge.timer_slots == 0 || request.resource_charge.delivery_queue_units == 0 {
        return Err(TimerError::InvalidResourceCharge);
    }
    let charged = active_timer_slots.checked_add(request.resource_charge.timer_slots).ok_or(TimerError::Overflow)?;
    if charged > profile.max_timers {
        return Err(TimerError::TimerLimitExceeded {
            active: charged,
            maximum: profile.max_timers,
        });
    }
    if matches!(request.kind, TimerKind::Periodic { period_ticks: 0 }) {
        return Err(TimerError::InvalidPeriod);
    }
    if matches!(request.coalescing, TimerCoalescingPolicy::DeliverEach { max_catch_up: 0 }) {
        return Err(TimerError::InvalidCatchUpBound);
    }
    Ok(TimerState {
        profile_ref: request.profile_ref.clone(),
        key: request.key.clone(),
        domain: request.domain,
        next_deadline_ticks: request.deadline_ticks,
        kind: request.kind,
        ordering_key: request.ordering_key,
        coalescing: request.coalescing,
        lateness: request.lateness,
        overload: request.overload,
        resource_charge: request.resource_charge,
        phase: TimerPhase::Scheduled,
        fire_count: 0,
        skipped_count: 0,
    })
}

// The transition is pure: callers provide the observed time and queue capacity.
pub fn poll_timer(
    state: &TimerState,
    active_generation: u64,
    now_ticks: u64,
    delivery_capacity: u64,
) -> Result<TimerTransition, TimerError> {
    if state.phase != TimerPhase::Scheduled {
        return Err(TimerError::TerminalTimer(state.phase));
    }
    if state.key.generation != active_generation {
        let mut next = state.clone();
        next.phase = TimerPhase::Cancelled;
        return Ok(TimerTransition {
            next,
            action: TimerAction::DiscardedStaleGeneration,
            delivery_count: 0,
            skipped_count: 0,
            lateness_ticks: 0,
        });
    }
    if now_ticks < state.next_deadline_ticks {
        return Ok(unchanged_transition(state, TimerAction::NotDue));
    }

    let lateness = now_ticks - state.next_deadline_ticks;
    let periods_due = periods_due(state.kind, lateness)?;
    if is_too_late(state.lateness, lateness) {
        return advance_without_delivery(state, periods_due, lateness, TimerAction::DroppedLate);
    }

    let (requested_deliveries, policy_skipped, action) = delivery_plan(state.coalescing, periods_due);
    let required_capacity = requested_deliveries
        .checked_mul(state.resource_charge.delivery_queue_units)
        .ok_or(TimerError::Overflow)?;
    if delivery_capacity < required_capacity {
        return overload_transition(state, periods_due, lateness);
    }
    advance_with_delivery(state, periods_due, requested_deliveries, policy_skipped, lateness, action)
}

pub fn cancel_timer(state: &TimerState, active_generation: u64) -> Result<TimerTransition, TimerError> {
    ensure_generation(active_generation, state.key.generation)?;
    if state.phase != TimerPhase::Scheduled {
        return Err(TimerError::TerminalTimer(state.phase));
    }
    let mut next = state.clone();
    next.phase = TimerPhase::Cancelled;
    Ok(TimerTransition {
        next,
        action: TimerAction::Cancelled,
        delivery_count: 0,
        skipped_count: 0,
        lateness_ticks: 0,
    })
}

pub fn cleanup_generation(states: &[TimerState], generation: u64) -> Vec<TimerState> {
    states
        .iter()
        .map(|state| {
            let mut next = state.clone();
            if state.key.generation == generation && state.phase == TimerPhase::Scheduled {
                next.phase = TimerPhase::Cancelled;
            }
            next
        })
        .collect()
}

pub fn order_due_timers(
    states: &[TimerState],
    active_generation: u64,
    now_ticks: u64,
    maximum: u64,
) -> Result<Vec<TimerKey>, TimerError> {
    let mut due: Vec<&TimerState> = states
        .iter()
        .filter(|state| {
            state.phase == TimerPhase::Scheduled
                && state.key.generation == active_generation
                && state.next_deadline_ticks <= now_ticks
        })
        .collect();
    due.sort_by(|left, right| {
        (left.next_deadline_ticks, left.ordering_key, &left.key).cmp(&(
            right.next_deadline_ticks,
            right.ordering_key,
            &right.key,
        ))
    });
    let maximum = usize::try_from(maximum).map_err(|_| TimerError::Overflow)?;
    due.truncate(maximum);
    Ok(due.into_iter().map(|state| state.key.clone()).collect())
}

fn periods_due(kind: TimerKind, lateness: u64) -> Result<u64, TimerError> {
    match kind {
        TimerKind::OneShot => Ok(1),
        TimerKind::Periodic { period_ticks } => lateness
            .checked_div(period_ticks)
            .and_then(|missed| missed.checked_add(1))
            .ok_or(TimerError::Overflow),
    }
}

fn delivery_plan(policy: TimerCoalescingPolicy, periods_due: u64) -> (u64, u64, TimerAction) {
    match policy {
        TimerCoalescingPolicy::DeliverEach { max_catch_up } => {
            let delivery_count = periods_due.min(max_catch_up);
            let skipped_count = periods_due - delivery_count;
            let action = if skipped_count == 0 {
                TimerAction::Deliver
            } else {
                TimerAction::Coalesced
            };
            (delivery_count, skipped_count, action)
        }
        TimerCoalescingPolicy::CoalesceLatest => {
            let skipped_count = periods_due.saturating_sub(1);
            let action = if skipped_count == 0 {
                TimerAction::Deliver
            } else {
                TimerAction::Coalesced
            };
            (1, skipped_count, action)
        }
        TimerCoalescingPolicy::SkipMissed => {
            let skipped_count = periods_due.saturating_sub(1);
            (1, skipped_count, TimerAction::Deliver)
        }
    }
}

fn is_too_late(policy: TimerLatenessPolicy, lateness: u64) -> bool {
    match policy {
        TimerLatenessPolicy::DeliverWithin { max_lateness_ticks } => lateness > max_lateness_ticks,
        TimerLatenessPolicy::DeliverRegardless => false,
    }
}

fn advance_with_delivery(
    state: &TimerState,
    periods_due: u64,
    deliveries: u64,
    skipped: u64,
    lateness: u64,
    action: TimerAction,
) -> Result<TimerTransition, TimerError> {
    let mut next = advance_deadline(state, periods_due)?;
    next.fire_count = next.fire_count.checked_add(deliveries).ok_or(TimerError::Overflow)?;
    next.skipped_count = next.skipped_count.checked_add(skipped).ok_or(TimerError::Overflow)?;
    Ok(TimerTransition {
        next,
        action,
        delivery_count: deliveries,
        skipped_count: skipped,
        lateness_ticks: lateness,
    })
}

fn advance_without_delivery(
    state: &TimerState,
    periods_due: u64,
    lateness: u64,
    action: TimerAction,
) -> Result<TimerTransition, TimerError> {
    let mut next = advance_deadline(state, periods_due)?;
    next.skipped_count = next.skipped_count.checked_add(periods_due).ok_or(TimerError::Overflow)?;
    Ok(TimerTransition {
        next,
        action,
        delivery_count: 0,
        skipped_count: periods_due,
        lateness_ticks: lateness,
    })
}

fn overload_transition(state: &TimerState, periods_due: u64, lateness: u64) -> Result<TimerTransition, TimerError> {
    match state.overload {
        TimerOverloadPolicy::RejectAndRetain => Ok(TimerTransition {
            next: state.clone(),
            action: TimerAction::RetainedOverload,
            delivery_count: 0,
            skipped_count: 0,
            lateness_ticks: lateness,
        }),
        TimerOverloadPolicy::Backpressure => Ok(TimerTransition {
            next: state.clone(),
            action: TimerAction::Backpressure,
            delivery_count: 0,
            skipped_count: 0,
            lateness_ticks: lateness,
        }),
        TimerOverloadPolicy::DropDue => {
            advance_without_delivery(state, periods_due, lateness, TimerAction::DroppedOverload)
        }
    }
}

fn advance_deadline(state: &TimerState, periods_due: u64) -> Result<TimerState, TimerError> {
    let mut next = state.clone();
    match state.kind {
        TimerKind::OneShot => next.phase = TimerPhase::Completed,
        TimerKind::Periodic { period_ticks } => {
            let advance = period_ticks.checked_mul(periods_due).ok_or(TimerError::Overflow)?;
            next.next_deadline_ticks = next.next_deadline_ticks.checked_add(advance).ok_or(TimerError::Overflow)?;
        }
    }
    Ok(next)
}

fn unchanged_transition(state: &TimerState, action: TimerAction) -> TimerTransition {
    TimerTransition {
        next: state.clone(),
        action,
        delivery_count: 0,
        skipped_count: 0,
        lateness_ticks: 0,
    }
}

fn ensure_generation(expected: u64, actual: u64) -> Result<(), TimerError> {
    if expected != actual {
        return Err(TimerError::StaleGeneration { expected, actual });
    }
    Ok(())
}

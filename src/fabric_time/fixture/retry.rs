use super::*;
use crate::fabric_time::canonical::canonical_retry_events;

const SATURATION_SUBJECT: &str = "fixture-retry-saturation";
const SATURATION_BASE: u64 = 2;
const SATURATION_ATTEMPT: u64 = 63;
const SATURATION_MAXIMUM: u64 = 128;
const SATURATION_ATTEMPT_LIMIT: u64 = 65;
const RETRY_PLAN_EVENT_COUNT: usize = 2;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::fabric_time) struct RetryFixtureInput {
    pub subject_id: String,
    pub generation: u64,
    pub now: TimeValue,
    pub attempt: u64,
    pub policy: RetryPolicy,
    pub jitter: Option<u64>,
}

// r[impl molten.audit_f12.compatibility]
// r[impl molten.audit_f12.validation]
pub(in crate::fabric_time) fn plan_events(
    profile: &CanonicalTimeProfile,
    active_generation: u64,
    input: &RetryFixtureInput,
) -> Result<(RetryPlan, Vec<CanonicalTimeEvent>)> {
    let plan = plan_retry(
        &profile.profile,
        active_generation,
        &input.subject_id,
        input.generation,
        &input.now,
        input.attempt,
        input.policy,
        input.jitter,
    )
    .map_err(|error| core_error("plan fixture retry", error))?;
    assert!(plan.delay.ticks > 0);
    assert!(plan.delay.ticks <= input.policy.maximum_delay_ticks);
    assert!(plan.deadline.target.ticks() >= input.now.ticks());
    let events = canonical_retry_events(&profile.profile_ref, &plan)?;
    Ok((plan, events))
}

// r[impl molten.audit_f12.compatibility]
pub(in crate::fabric_time) fn execute<A: TimerClockAdapter>(
    profile: &CanonicalTimeProfile,
    active_generation: u64,
    input: &RetryFixtureInput,
    clock: &mut A,
    events: &mut Vec<CanonicalTimeEvent>,
) -> Result<RetryPlan> {
    let (plan, mut observations) = plan_events(profile, active_generation, input)?;
    if clock.profile_ref() != profile.profile.profile_ref || clock.timer_domain() != plan.deadline.target.domain() {
        return Err(MoltenError::invalid_harness("retry fixture clock profile or domain mismatch"));
    }
    let timer = retry_timer(profile, active_generation, &plan)?;
    let observed_ticks = clock.await_ticks(plan.deadline.target.ticks())?;
    let transition = poll_timer(&timer, active_generation, observed_ticks, 1)
        .map_err(|error| core_error("poll retry fixture timer", error))?;
    if transition.action != TimerAction::Deliver {
        return Err(MoltenError::invalid_harness("retry fixture clock did not reach the admitted deadline"));
    }
    observations.push(canonical_timer_event(&profile.profile_ref, &transition)?);
    events.extend(observations);
    Ok(plan)
}

fn retry_timer(profile: &CanonicalTimeProfile, active_generation: u64, plan: &RetryPlan) -> Result<TimerState> {
    schedule_timer(&profile.profile, active_generation, 0, &TimerScheduleRequest {
        profile_ref: profile.profile.profile_ref.clone(),
        key: TimerKey {
            service_id: plan.deadline.subject_id.clone(),
            generation: plan.deadline.generation,
            sequence: 0,
        },
        domain: plan.deadline.target.domain(),
        deadline_ticks: plan.deadline.target.ticks(),
        kind: TimerKind::OneShot,
        ordering_key: 0,
        coalescing: TimerCoalescingPolicy::CoalesceLatest,
        lateness: TimerLatenessPolicy::DeliverRegardless,
        overload: TimerOverloadPolicy::RejectAndRetain,
        resource_charge: TimerResourceCharge::single_slot(),
    })
    .map_err(|error| core_error("schedule retry fixture timer", error))
}

// r[impl molten.audit_f12.validation]
pub(in crate::fabric_time) fn replay(
    profile: &CanonicalTimeProfile,
    active_generation: u64,
    input: &RetryFixtureInput,
    recorded: &[CanonicalTimeEvent],
) -> Result<RetryPlan> {
    let (plan, expected) = plan_events(profile, active_generation, input)?;
    if expected.as_slice() != recorded {
        return Err(MoltenError::harness_divergence(crate::error::HarnessDivergence::new(
            "fabric-time-retry",
            Some(input.attempt),
            "canonical delay and deadline from the current retry plan",
            "different recorded retry events",
            "retry replay diverged from recorded delay or deadline",
        )));
    }
    Ok(plan)
}

pub(super) fn run_saturation_scenario(
    profile: &CanonicalTimeProfile,
    clock: &mut VirtualClockAdapter,
) -> Result<Vec<CanonicalTimeEvent>> {
    let input = RetryFixtureInput {
        subject_id: SATURATION_SUBJECT.to_string(),
        generation: FIXTURE_GENERATION,
        now: virtual_value(&profile.profile, clock.now_ticks()?),
        attempt: SATURATION_ATTEMPT,
        policy: RetryPolicy {
            maximum_attempts: SATURATION_ATTEMPT_LIMIT,
            base_delay_ticks: SATURATION_BASE,
            maximum_delay_ticks: SATURATION_MAXIMUM,
            backoff: RetryBackoff::Exponential,
            jitter: RetryJitter::None,
        },
        jitter: None,
    };
    let mut observations = Vec::new();
    execute(profile, FIXTURE_GENERATION, &input, clock, &mut observations)?;
    replay(profile, FIXTURE_GENERATION, &input, &observations[..RETRY_PLAN_EVENT_COUNT])?;
    reject_wrapped_history(profile, &input)?;
    observations.push(canonical_named_event(
        &profile.profile_ref,
        CanonicalTimeEventKind::Conformance,
        FIXTURE_GENERATION,
        SATURATION_SUBJECT,
        "wrapped-replay-rejected",
        input.now.ticks(),
    )?);
    Ok(observations)
}

fn reject_wrapped_history(profile: &CanonicalTimeProfile, input: &RetryFixtureInput) -> Result<()> {
    let (mut wrapped_plan, _) = plan_events(profile, FIXTURE_GENERATION, input)?;
    wrapped_plan.delay.ticks = 0;
    wrapped_plan.deadline.target = input.now.clone();
    let wrapped = canonical_retry_events(&profile.profile_ref, &wrapped_plan)?;
    match replay(profile, FIXTURE_GENERATION, input, &wrapped) {
        Err(MoltenError::HarnessDivergence(_)) => Ok(()),
        Err(error) => Err(error),
        Ok(_) => Err(MoltenError::invalid_harness("retry fixture accepted a wrapped historical delay")),
    }
}

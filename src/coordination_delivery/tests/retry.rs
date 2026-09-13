use molten_core::coordination_delivery::*;

use super::super::*;
use super::support::*;
use super::trace::DeliveryPortCall;

mod harness;

use harness::RetryHarness;

const BASE_WITH_HIGH_BIT_LOSS: u64 = 4;
const CAPPED_DELAY: u64 = 128;
const LAST_RETRYABLE_ATTEMPT: u64 = MAX_DELIVERY_ATTEMPTS - 1;
const APPLIED_CALLS: &[DeliveryPortCall] = &[
    DeliveryPortCall::Load,
    DeliveryPortCall::CompareAndCommit,
    DeliveryPortCall::TimerIntents,
    DeliveryPortCall::PublishStatus,
];

fn negative_acknowledge(token: DeliveryToken) -> DeliveryOperation {
    DeliveryOperation::NegativeAcknowledge {
        token,
        failure_class: "transient".to_string(),
    }
}

fn assert_retry_effect(harness: &RetryHarness, outcome: &DeliveryServiceOutcome, deadline: u64) {
    assert_eq!(outcome.receipt.status, DeliveryServiceStatus::Applied);
    assert_eq!(outcome.transition.kind, DeliveryTransitionKind::RetryScheduled);
    assert_eq!(harness.calls().as_slice(), APPLIED_CALLS);
    let requested = harness.timers.requested.last().expect("timer port invocation");
    assert_eq!(requested, &outcome.transition.timer_intents);
    let retry = requested
        .iter()
        .find(|intent| intent.kind == DeliveryTimerIntentKind::ScheduleRetryEligibility)
        .expect("retry timer intent");
    assert_eq!(retry.deadline_tick, deadline);
    assert_eq!(retry.service_generation, SERVICE_GENERATION);
    assert!(outcome.timer_observation.accepted_timer_refs.contains(&retry.timer_id));
    let published = harness.commit.head.as_ref().expect("committed retry state");
    assert_eq!(published.state_ref, outcome.transition.after_state_ref);
    assert_eq!(published.state.ready[&retry.item_ref].eligible_at_tick, deadline);
    assert!(!published.state.in_flight.contains_key(&retry.item_ref));
}

// r[verify molten.audit_f12.compatibility]
#[test]
fn fixed_retry_commits_before_timer_and_status_effects() {
    let mut harness = RetryHarness::new(policy());
    assert_eq!(harness.policy.retry_backoff, DeliveryBackoff::Fixed);
    harness.enqueue();
    let token = harness.claim(INITIAL_TICK);
    harness.clear_trace();
    let outcome = harness.apply(INITIAL_TICK, negative_acknowledge(token));
    assert_retry_effect(&harness, &outcome, INITIAL_TICK + RETRY_TICKS);
}

// This custom policy does not change the selected fixed delivery profile.
// r[verify molten.audit_f12.saturation]
// r[verify molten.audit_f12.compatibility]
#[test]
fn admitted_exponential_policy_saturates_through_real_claim_and_retry_transitions() {
    let policy = DeliveryPolicy {
        maximum_attempts: MAX_DELIVERY_ATTEMPTS,
        retry_base_delay_ticks: BASE_WITH_HIGH_BIT_LOSS,
        retry_maximum_delay_ticks: CAPPED_DELAY,
        retry_backoff: DeliveryBackoff::Exponential,
        ..policy()
    };
    let mut harness = RetryHarness::new(policy);
    harness.enqueue();
    let mut now = INITIAL_TICK;
    assert!(LAST_RETRYABLE_ATTEMPT < u64::from(u128::BITS));
    for attempt in 1..=LAST_RETRYABLE_ATTEMPT {
        let token = harness.claim(now);
        assert_eq!(token.attempt, attempt);
        let wide_delay = u128::from(BASE_WITH_HIGH_BIT_LOSS) << (attempt - 1);
        let delay = u64::try_from(wide_delay.min(u128::from(CAPPED_DELAY))).expect("bounded reference delay");
        let deadline = now.checked_add(delay).expect("bounded test timeline");
        harness.clear_trace();
        let outcome = harness.apply(now, negative_acknowledge(token));
        assert_retry_effect(&harness, &outcome, deadline);
        now = deadline;
    }
    let final_shift = u32::try_from(LAST_RETRYABLE_ATTEMPT - 1).expect("bounded retry shift");
    assert_eq!(BASE_WITH_HIGH_BIT_LOSS.checked_shl(final_shift), Some(0));
    assert_eq!(u128::from(BASE_WITH_HIGH_BIT_LOSS) << final_shift, u128::from(u64::MAX) + 1);
}

fn assert_denied_retry_preserves_state(
    harness: &mut RetryHarness,
    now: u64,
    token: DeliveryToken,
    issue: DeliveryIssue,
) {
    let head = harness.commit.head.clone();
    let compare_calls = harness.commit.compare_calls;
    let requested = harness.timers.requested.clone();
    let observed = harness.timers.observed.clone();
    let statuses = harness.statuses.status_refs.clone();
    harness.clear_trace();
    let outcome = harness.apply(now, negative_acknowledge(token));
    assert_eq!(outcome.receipt.status, DeliveryServiceStatus::Denied);
    assert_eq!(outcome.transition.issue, Some(issue));
    assert_eq!(outcome.transition.before_state_ref, outcome.transition.after_state_ref);
    assert_eq!(outcome.transition.next_state, head.as_ref().expect("prior state").state);
    assert!(outcome.transition.timer_intents.is_empty());
    assert!(outcome.commit_observation.is_none());
    assert_eq!(outcome.timer_observation, DeliveryTimerObservation::empty());
    assert_eq!(harness.calls(), vec![DeliveryPortCall::Load]);
    assert_eq!(harness.commit.head, head);
    assert_eq!(harness.commit.compare_calls, compare_calls);
    assert_eq!(harness.timers.requested, requested);
    assert_eq!(harness.timers.observed, observed);
    assert_eq!(harness.statuses.status_refs, statuses);
}

// r[verify molten.audit_f12.compatibility]
#[test]
fn retry_duration_denial_preserves_consumer_state_and_translates_the_core_error() {
    let policy = DeliveryPolicy {
        retry_base_delay_ticks: VISIBILITY_TICKS + 1,
        retry_maximum_delay_ticks: VISIBILITY_TICKS + 1,
        ..policy()
    };
    let mut harness = RetryHarness::new(policy);
    harness.time.max_duration_ticks = VISIBILITY_TICKS;
    harness.enqueue();
    let token = harness.claim(INITIAL_TICK);
    assert_denied_retry_preserves_state(&mut harness, INITIAL_TICK, token, DeliveryIssue::ArithmeticOverflow);
}

// The delivery time bound rejects this value before deadline arithmetic.
// r[verify molten.audit_f12.compatibility]
#[test]
fn out_of_range_delivery_time_denies_before_retry_or_timer_effects() {
    let mut harness = RetryHarness::new(policy());
    harness.enqueue();
    let token = harness.claim(INITIAL_TICK);
    const { assert!(u64::MAX > MAX_DELIVERY_TICKS) };
    assert_denied_retry_preserves_state(&mut harness, u64::MAX, token, DeliveryIssue::LogicalTimeRequired);
}

// r[verify molten.audit_f12.compatibility]
#[test]
fn timer_io_failure_remains_distinct_from_retry_plan_denial() {
    let mut harness = RetryHarness::new(policy());
    harness.enqueue();
    let token = harness.claim(INITIAL_TICK);
    let before = harness.commit.head.clone();
    harness.timers.fail = true;
    harness.clear_trace();
    let outcome = harness.apply(INITIAL_TICK, negative_acknowledge(token));
    assert_eq!(outcome.receipt.status, DeliveryServiceStatus::Applied);
    assert_eq!(outcome.transition.kind, DeliveryTransitionKind::RetryScheduled);
    assert_eq!(harness.calls().as_slice(), APPLIED_CALLS);
    assert_ne!(harness.commit.head, before);
    assert_eq!(harness.commit.head.as_ref().expect("committed state").state_ref, outcome.transition.after_state_ref);
    assert!(outcome.timer_observation.accepted_timer_refs.is_empty());
    let requested = harness.timers.requested.last().expect("failed timer invocation");
    let failed_refs = requested.iter().map(|intent| intent.timer_id.clone()).collect::<Vec<_>>();
    assert_eq!(outcome.timer_observation.failed_timer_refs, failed_refs);
    assert!(!outcome.timer_observation.outcome_unknown);
}

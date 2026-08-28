use molten_core::coordination_delivery::*;

use super::super::*;
use super::support::*;

const EXPECTED_COMPARE_CALLS_AFTER_TWO_COMMITS: u32 = 2;

// r[verify molten.coordination_delivery.consistency_durability]
#[test]
fn unknown_before_apply_reconciles_without_blind_retry() {
    let policy = policy();
    let manifest = manifest(&policy);
    let time = time_profile(&manifest);
    let request = enqueue_request(&manifest, '1');
    let mut commit = MemoryCommitPort::new(CommitMode::UnknownBefore);
    let mut timers = timer_port(false);
    let mut statuses = MemoryStatusPort::default();
    let outcome = apply_delivery_request(&mut commit, &mut timers, &mut statuses, &DeliveryServiceRequest {
        manifest: &manifest,
        policy: &policy,
        time_profile: &time,
        host_binding: &host_binding(&manifest),
        expected: empty_expected(),
        request: &request,
    })
    .expect("reconciled outcome");
    assert_eq!(outcome.receipt.status, DeliveryServiceStatus::NotAppliedAfterReconciliation);
    assert_eq!(commit.compare_calls, 1);
    assert!(commit.head.is_none());
    assert!(timers.observed.is_empty());
}

// r[verify molten.coordination_delivery.consistency_durability]
#[test]
fn unknown_after_apply_reconciles_then_schedules_exact_timer_once() {
    let policy = policy();
    let manifest = manifest(&policy);
    let time = time_profile(&manifest);
    let mut commit = MemoryCommitPort::new(CommitMode::Apply);
    let mut timers = timer_port(false);
    let mut statuses = MemoryStatusPort::default();
    let enqueue = enqueue_request(&manifest, '1');
    apply_delivery_request(&mut commit, &mut timers, &mut statuses, &DeliveryServiceRequest {
        manifest: &manifest,
        policy: &policy,
        time_profile: &time,
        host_binding: &host_binding(&manifest),
        expected: empty_expected(),
        request: &enqueue,
    })
    .expect("enqueue");
    let published = commit.head.clone().expect("published enqueue");
    commit.mode = CommitMode::UnknownAfter;
    let claim = request(&manifest, '2', INITIAL_TICK, DeliveryOperation::Claim);
    let outcome = apply_delivery_request(&mut commit, &mut timers, &mut statuses, &DeliveryServiceRequest {
        manifest: &manifest,
        policy: &policy,
        time_profile: &time,
        host_binding: &host_binding(&manifest),
        expected: expected(&published),
        request: &claim,
    })
    .expect("reconciled claim");
    assert_eq!(outcome.receipt.status, DeliveryServiceStatus::AppliedAfterReconciliation);
    assert_eq!(commit.compare_calls, EXPECTED_COMPARE_CALLS_AFTER_TWO_COMMITS);
    assert_eq!(timers.observed.len(), 1);
}

// r[verify molten.coordination_delivery.logical_time]
#[test]
fn timer_failure_does_not_rewrite_a_durable_commit() {
    let policy = policy();
    let manifest = manifest(&policy);
    let time = time_profile(&manifest);
    let mut commit = MemoryCommitPort::new(CommitMode::Apply);
    let mut timers = timer_port(false);
    let mut statuses = MemoryStatusPort::default();
    let enqueue = enqueue_request(&manifest, '1');
    apply_delivery_request(&mut commit, &mut timers, &mut statuses, &DeliveryServiceRequest {
        manifest: &manifest,
        policy: &policy,
        time_profile: &time,
        host_binding: &host_binding(&manifest),
        expected: empty_expected(),
        request: &enqueue,
    })
    .expect("enqueue");
    let published = commit.head.clone().expect("published enqueue");
    timers.fail = true;
    let claim = request(&manifest, '2', INITIAL_TICK, DeliveryOperation::Claim);
    let outcome = apply_delivery_request(&mut commit, &mut timers, &mut statuses, &DeliveryServiceRequest {
        manifest: &manifest,
        policy: &policy,
        time_profile: &time,
        host_binding: &host_binding(&manifest),
        expected: expected(&published),
        request: &claim,
    })
    .expect("claim with timer failure");
    assert_eq!(outcome.receipt.status, DeliveryServiceStatus::Applied);
    assert_eq!(outcome.timer_observation.failed_timer_refs.len(), 1);
    assert_eq!(
        commit.head.as_ref().map(|head| head.state_ref.as_str()),
        Some(outcome.transition.after_state_ref.as_str())
    );
}

#[test]
fn stale_commit_observation_does_not_schedule_follow_up_effects() {
    let policy = policy();
    let manifest = manifest(&policy);
    let time = time_profile(&manifest);
    let mut commit = MemoryCommitPort::new(CommitMode::Stale);
    let mut timers = timer_port(false);
    let mut statuses = MemoryStatusPort::default();
    let request = enqueue_request(&manifest, '1');
    let outcome = apply_delivery_request(&mut commit, &mut timers, &mut statuses, &DeliveryServiceRequest {
        manifest: &manifest,
        policy: &policy,
        time_profile: &time,
        host_binding: &host_binding(&manifest),
        expected: empty_expected(),
        request: &request,
    })
    .expect("stale commit observation");
    assert_eq!(outcome.receipt.status, DeliveryServiceStatus::Stale);
    assert!(timers.observed.is_empty());
    assert!(statuses.status_refs.is_empty());
}

// r[verify molten.coordination_delivery.fenced_completion]
#[test]
fn stale_expected_state_never_calls_compare_and_commit() {
    let policy = policy();
    let manifest = manifest(&policy);
    let time = time_profile(&manifest);
    let mut commit = MemoryCommitPort::new(CommitMode::Apply);
    let mut timers = timer_port(false);
    let mut statuses = MemoryStatusPort::default();
    let request = enqueue_request(&manifest, '1');
    let stale = ExpectedDeliveryState {
        state_ref: Some(reference('f')),
        revision: INITIAL_DELIVERY_REVISION,
    };
    let outcome = apply_delivery_request(&mut commit, &mut timers, &mut statuses, &DeliveryServiceRequest {
        manifest: &manifest,
        policy: &policy,
        time_profile: &time,
        host_binding: &host_binding(&manifest),
        expected: stale,
        request: &request,
    })
    .expect("stale outcome");
    assert_eq!(outcome.receipt.status, DeliveryServiceStatus::Stale);
    assert_eq!(commit.compare_calls, 0);
}

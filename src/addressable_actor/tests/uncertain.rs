use molten_core::addressable_actor::*;

use super::super::*;
use super::support::*;

const WAKE_EFFECT_COUNT: usize = 2;
const EXPECTED_COMMITS_WITH_UNKNOWN_EFFECT: u32 = 2;

// r[verify molten.addressable_actor.authority]
#[test]
fn unknown_before_apply_reconciles_without_blind_retry_or_effects() {
    let profile = profile();
    let actor_key = actor_key();
    let initial = initial_state(&profile, &actor_key);
    let request = request(&initial, "wake-message", INITIAL_TICK, ActorOperation::Wake { reason: message_wake() });
    let mut commit = MemoryCommitPort::new(CommitMode::UnknownBefore);
    let mut effects = MemoryEffectPort::succeeding(WAKE_EFFECT_COUNT);
    let mut statuses = MemoryStatusPort::default();
    let outcome = apply_actor_request(&mut commit, &mut effects, &mut statuses, &ActorServiceRequest {
        profile: &profile,
        actor_key: &actor_key,
        host_binding: &host_binding(&profile, &initial),
        expected: empty_expected(),
        request: &request,
        requested_engine_epoch: ENGINE_EPOCH,
    })
    .expect("reconcile unknown-before commit");

    assert_eq!(outcome.receipt.status, ActorServiceStatus::NotAppliedAfterReconciliation);
    assert_eq!(commit.compare_calls, 1);
    assert!(commit.head.is_none());
    assert_eq!(effects.execution_calls, 0);
}

#[test]
fn unknown_after_apply_reconciles_then_executes_each_effect_once() {
    let profile = profile();
    let actor_key = actor_key();
    let initial = initial_state(&profile, &actor_key);
    let request = request(&initial, "wake-message", INITIAL_TICK, ActorOperation::Wake { reason: message_wake() });
    let mut commit = MemoryCommitPort::new(CommitMode::UnknownAfter);
    let mut effects = MemoryEffectPort::succeeding(WAKE_EFFECT_COUNT);
    let mut statuses = MemoryStatusPort::default();
    let outcome = apply_actor_request(&mut commit, &mut effects, &mut statuses, &ActorServiceRequest {
        profile: &profile,
        actor_key: &actor_key,
        host_binding: &host_binding(&profile, &initial),
        expected: empty_expected(),
        request: &request,
        requested_engine_epoch: ENGINE_EPOCH,
    })
    .expect("reconcile unknown-after commit");

    assert_eq!(outcome.receipt.status, ActorServiceStatus::AppliedAfterReconciliation);
    assert_eq!(commit.compare_calls, 1);
    assert_eq!(effects.execution_calls, WAKE_EFFECT_COUNT);
}

// r[verify molten.addressable_actor.authority]
#[test]
fn changed_generation_denies_the_next_effect_before_execution() {
    let profile = profile();
    let actor_key = actor_key();
    let initial = initial_state(&profile, &actor_key);
    let request = request(&initial, "wake-message", INITIAL_TICK, ActorOperation::Wake { reason: message_wake() });
    let mut commit = MemoryCommitPort::new(CommitMode::Apply);
    let mut effects = MemoryEffectPort::succeeding(WAKE_EFFECT_COUNT);
    effects.deny_admission_at = Some(1);
    let mut statuses = MemoryStatusPort::default();
    let outcome = apply_actor_request(&mut commit, &mut effects, &mut statuses, &ActorServiceRequest {
        profile: &profile,
        actor_key: &actor_key,
        host_binding: &host_binding(&profile, &initial),
        expected: empty_expected(),
        request: &request,
        requested_engine_epoch: ENGINE_EPOCH,
    })
    .expect("generation drift outcome");

    assert_eq!(outcome.receipt.status, ActorServiceStatus::EffectAdmissionDenied);
    assert_eq!(effects.admission_calls, WAKE_EFFECT_COUNT);
    assert_eq!(effects.execution_calls, 1);
    assert_eq!(outcome.effect_observations[1].disposition, ActorEffectDisposition::AdmissionDenied);
}

// r[verify molten.addressable_actor.delivery]
#[test]
fn unknown_effect_quarantines_actor_without_executing_later_effects() {
    let profile = profile();
    let actor_key = actor_key();
    let initial = initial_state(&profile, &actor_key);
    let request = request(&initial, "wake-message", INITIAL_TICK, ActorOperation::Wake { reason: message_wake() });
    let mut commit = MemoryCommitPort::new(CommitMode::Apply);
    let mut effects = MemoryEffectPort::scripted([ActorEffectDisposition::Unknown, ActorEffectDisposition::Succeeded]);
    let mut statuses = MemoryStatusPort::default();
    let outcome = apply_actor_request(&mut commit, &mut effects, &mut statuses, &ActorServiceRequest {
        profile: &profile,
        actor_key: &actor_key,
        host_binding: &host_binding(&profile, &initial),
        expected: empty_expected(),
        request: &request,
        requested_engine_epoch: ENGINE_EPOCH,
    })
    .expect("unknown effect outcome");

    assert_eq!(outcome.receipt.status, ActorServiceStatus::EffectOutcomeUnknown);
    assert_eq!(commit.compare_calls, EXPECTED_COMMITS_WITH_UNKNOWN_EFFECT);
    assert_eq!(effects.execution_calls, 1);
    assert_eq!(outcome.final_state.state.phase, ActorPhase::Degraded);
    assert!(outcome.final_state.state.unknown_effect_ref.is_some());
    assert!(!outcome.receipt.bytes.is_empty());
}

#[test]
fn stale_commit_observation_never_executes_actor_effects() {
    let profile = profile();
    let actor_key = actor_key();
    let initial = initial_state(&profile, &actor_key);
    let request = request(&initial, "wake-message", INITIAL_TICK, ActorOperation::Wake { reason: message_wake() });
    let mut commit = MemoryCommitPort::new(CommitMode::Stale);
    let mut effects = MemoryEffectPort::succeeding(WAKE_EFFECT_COUNT);
    let mut statuses = MemoryStatusPort::default();
    let outcome = apply_actor_request(&mut commit, &mut effects, &mut statuses, &ActorServiceRequest {
        profile: &profile,
        actor_key: &actor_key,
        host_binding: &host_binding(&profile, &initial),
        expected: empty_expected(),
        request: &request,
        requested_engine_epoch: ENGINE_EPOCH,
    })
    .expect("stale commit outcome");

    assert_eq!(outcome.receipt.status, ActorServiceStatus::Stale);
    assert_eq!(effects.execution_calls, 0);
}

#[test]
fn stale_expected_state_never_calls_compare_or_effect_ports() {
    let profile = profile();
    let actor_key = actor_key();
    let initial = initial_state(&profile, &actor_key);
    let request = request(&initial, "wake-message", INITIAL_TICK, ActorOperation::Wake { reason: message_wake() });
    let mut commit = MemoryCommitPort::new(CommitMode::Apply);
    let mut effects = MemoryEffectPort::succeeding(WAKE_EFFECT_COUNT);
    let mut statuses = MemoryStatusPort::default();
    let outcome = apply_actor_request(&mut commit, &mut effects, &mut statuses, &ActorServiceRequest {
        profile: &profile,
        actor_key: &actor_key,
        host_binding: &host_binding(&profile, &initial),
        expected: ExpectedActorState {
            state_ref: Some(reference("stale-state")),
            revision: ADDRESSABLE_ACTOR_INITIAL_REVISION,
        },
        request: &request,
        requested_engine_epoch: ENGINE_EPOCH,
    })
    .expect("stale expected outcome");

    assert_eq!(outcome.receipt.status, ActorServiceStatus::Stale);
    assert_eq!(commit.compare_calls, 0);
    assert_eq!(effects.admission_calls, 0);
}

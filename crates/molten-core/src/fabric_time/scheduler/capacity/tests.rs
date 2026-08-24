use super::*;

const BLAKE3_HEX_LEN: usize = 64;
const GENERATION: u64 = 7;
const STALE_GENERATION: u64 = 8;
const RUNNABLE_LIMIT: u64 = 8;
const RUNNABLE_HARD_CAP: u64 = 65_536;
const QUEUE_LIMIT: u64 = 4;
const CONCURRENCY_LIMIT: u64 = 2;
const FAIRNESS_LIMIT: u64 = 3;
const PROFILE_LIMIT: u64 = 16;
const ENTROPY_LIMIT: u64 = 32;

fn profile() -> super::super::super::AdmittedTimeProfile {
    super::super::super::AdmittedTimeProfile {
        profile_id: "profile.capacity".to_string(),
        profile_ref: format!("blake3:{}", "a".repeat(BLAKE3_HEX_LEN)),
        kind: super::super::super::TimeProfileKind::DeterministicSimulation,
        supported_domains: super::super::super::REQUIRED_TIME_DOMAINS.to_vec(),
        max_duration_ticks: PROFILE_LIMIT,
        max_uncertainty_ticks: PROFILE_LIMIT,
        max_timers: PROFILE_LIMIT,
        max_runnables: RUNNABLE_LIMIT,
        max_entropy_request_bytes: PROFILE_LIMIT,
        max_entropy_total_bytes: ENTROPY_LIMIT,
        max_scheduler_concurrency: CONCURRENCY_LIMIT,
        max_scheduler_queue_depth: QUEUE_LIMIT,
        fairness_bound_turns: Some(FAIRNESS_LIMIT),
        scheduler_policy: super::super::SchedulerPolicy {
            ordering: super::super::SchedulerOrdering::Fifo,
            replay: super::super::SchedulerReplayPolicy::Deterministic,
            overload: super::super::SchedulerOverloadPolicy::Reject,
        },
        evidence_mode: super::super::super::TimeEvidenceMode::SelectedSemanticBoundaries,
        non_claims: super::super::super::REQUIRED_TIME_NON_CLAIMS.to_vec(),
    }
}

// r[verify molten.fabric_time.scheduler_capacity.plan]
// r[verify molten.fabric_time.scheduler_capacity.verification]
#[test]
fn valid_profile_produces_stable_checked_plan() {
    let profile = profile();
    let plan = derive(&profile, GENERATION).expect("capacity plan");
    let repeated = derive(&profile, GENERATION).expect("repeated capacity plan");

    assert_eq!(plan.profile_ref, profile.profile_ref);
    assert_eq!(plan.generation, GENERATION);
    assert_eq!(plan.runnable_slots, RUNNABLE_LIMIT);
    assert_eq!(plan.queue_slots, QUEUE_LIMIT);
    assert_eq!(plan.concurrency_slots, CONCURRENCY_LIMIT);
    assert_eq!(plan.total_slots, RUNNABLE_LIMIT + QUEUE_LIMIT + CONCURRENCY_LIMIT);
    assert_eq!(plan.plan_ref, repeated.plan_ref);
    assert!(plan.plan_ref.starts_with("blake3:"));
}

#[test]
fn zero_generation_relations_conversion_and_overflow_fail_closed() {
    let profile = profile();
    assert_eq!(derive(&profile, 0), Err(PlanIssue::ZeroGeneration));

    let mut bad_queue = profile.clone();
    bad_queue.max_scheduler_queue_depth = RUNNABLE_LIMIT + 1;
    assert_eq!(derive(&bad_queue, GENERATION), Err(PlanIssue::QueueExceedsRunnables));

    let mut bad_concurrency = profile.clone();
    bad_concurrency.max_scheduler_concurrency = RUNNABLE_LIMIT + 1;
    assert_eq!(derive(&bad_concurrency, GENERATION), Err(PlanIssue::ConcurrencyExceedsRunnables));

    let mut over_cap = profile.clone();
    over_cap.max_runnables = RUNNABLE_HARD_CAP + 1;
    assert!(matches!(
        derive(&over_cap, GENERATION),
        Err(PlanIssue::HardLimitExceeded {
            field: "runnable-slots",
            actual: _,
            maximum: RUNNABLE_HARD_CAP,
        })
    ));

    assert_eq!(
        validate_count_against("runnable-slots", RUNNABLE_LIMIT, RUNNABLE_LIMIT - 1),
        Err(PlanIssue::CountUnrepresentable("runnable-slots"))
    );
    assert_eq!(checked_total(u64::MAX, 1, 1), Err(PlanIssue::AllocationArithmeticOverflow));
}

// r[verify molten.fabric_time.scheduler_capacity.steady_state]
// r[verify molten.fabric_time.scheduler_capacity.boundary]
#[test]
fn use_is_bounded_profile_bound_and_generation_fenced() {
    let profile = profile();
    let plan = derive(&profile, GENERATION).expect("capacity plan");
    let state = UseState::activated(&plan);
    let admitted = apply_use(&plan, &state, &profile.profile_ref, GENERATION, UseRequest {
        runnable_delta: 1,
        queue_delta: 1,
    });
    assert_eq!(admitted.kind, UseDecisionKind::Admit);
    assert_eq!(admitted.next.runnable_usage, 1);
    assert_eq!(admitted.next.queue_usage, 1);

    let stale = apply_use(&plan, &admitted.next, &profile.profile_ref, STALE_GENERATION, UseRequest {
        runnable_delta: 1,
        queue_delta: 0,
    });
    assert_eq!(stale.kind, UseDecisionKind::StaleGeneration);
    assert_eq!(stale.next, admitted.next);

    let wrong_profile = apply_use(
        &plan,
        &admitted.next,
        "blake3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        GENERATION,
        UseRequest {
            runnable_delta: 1,
            queue_delta: 0,
        },
    );
    assert_eq!(wrong_profile.kind, UseDecisionKind::ProfileMismatch);
    assert_eq!(wrong_profile.next, admitted.next);
}

#[test]
fn exhaustion_underflow_release_and_observation_are_explicit() {
    let profile = profile();
    let plan = derive(&profile, GENERATION).expect("capacity plan");
    let mut state = UseState::activated(&plan);
    state.runnable_usage = RUNNABLE_LIMIT;
    state.queue_usage = QUEUE_LIMIT;

    let exhausted = apply_use(&plan, &state, &profile.profile_ref, GENERATION, UseRequest {
        runnable_delta: 1,
        queue_delta: 0,
    });
    assert_eq!(exhausted.kind, UseDecisionKind::Exhausted);
    assert_eq!(exhausted.next.exhaustion_count, 1);
    assert_eq!(exhausted.next.runnable_usage, RUNNABLE_LIMIT);

    let underflow = apply_use(&plan, &UseState::activated(&plan), &profile.profile_ref, GENERATION, UseRequest {
        runnable_delta: -1,
        queue_delta: 0,
    });
    assert_eq!(underflow.kind, UseDecisionKind::Underflow);

    let released = release(&exhausted.next);
    let denied = apply_use(&plan, &released, &profile.profile_ref, GENERATION, UseRequest {
        runnable_delta: 1,
        queue_delta: 1,
    });
    assert_eq!(denied.kind, UseDecisionKind::Released);
    let observation = observe(&released);
    assert!(observation.is_released);
    assert_eq!(observation.exhaustion_count, 1);
    assert!(observation.observation_ref.starts_with("blake3:"));
    assert!(observation.non_claims.contains(&"does-not-prove-fairness"));
    assert!(observation.non_claims.contains(&"does-not-prove-liveness"));
}

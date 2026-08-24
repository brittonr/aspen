use super::*;

const BLAKE3_HEX_LEN: usize = 64;
const GENERATION: u64 = 7;
const STALE_GENERATION: u64 = 8;
const RUNNABLE_LIMIT: u64 = 8;
const QUEUE_LIMIT: u64 = 4;
const CONCURRENCY_LIMIT: u64 = 2;
const PROFILE_LIMIT: u64 = 16;
const ENTROPY_LIMIT: u64 = 32;
const FAIRNESS_LIMIT: u64 = 3;

fn profile() -> molten_core::fabric_time::AdmittedTimeProfile {
    molten_core::fabric_time::AdmittedTimeProfile {
        profile_id: "profile.capacity-shell".to_string(),
        profile_ref: format!("blake3:{}", "a".repeat(BLAKE3_HEX_LEN)),
        kind: molten_core::fabric_time::TimeProfileKind::DeterministicSimulation,
        supported_domains: molten_core::fabric_time::REQUIRED_TIME_DOMAINS.to_vec(),
        max_duration_ticks: PROFILE_LIMIT,
        max_uncertainty_ticks: PROFILE_LIMIT,
        max_timers: PROFILE_LIMIT,
        max_runnables: RUNNABLE_LIMIT,
        max_entropy_request_bytes: PROFILE_LIMIT,
        max_entropy_total_bytes: ENTROPY_LIMIT,
        max_scheduler_concurrency: CONCURRENCY_LIMIT,
        max_scheduler_queue_depth: QUEUE_LIMIT,
        fairness_bound_turns: Some(FAIRNESS_LIMIT),
        scheduler_policy: molten_core::fabric_time::SchedulerPolicy {
            ordering: molten_core::fabric_time::SchedulerOrdering::Fifo,
            replay: molten_core::fabric_time::SchedulerReplayPolicy::Deterministic,
            overload: molten_core::fabric_time::SchedulerOverloadPolicy::Reject,
        },
        evidence_mode: molten_core::fabric_time::TimeEvidenceMode::SelectedSemanticBoundaries,
        non_claims: molten_core::fabric_time::REQUIRED_TIME_NON_CLAIMS.to_vec(),
    }
}

// r[verify molten.fabric_time.scheduler_capacity.activation]
// r[verify molten.fabric_time.scheduler_capacity.verification]
#[test]
fn activation_reserves_the_complete_plan_before_use() {
    let profile = profile();
    let runtime = Runtime::activate(&profile, GENERATION).expect("activation");

    assert!(runtime.runnable_capacity() >= usize::try_from(RUNNABLE_LIMIT).expect("runnable count"));
    assert!(runtime.queue_capacity() >= usize::try_from(QUEUE_LIMIT).expect("queue count"));
    assert_eq!(runtime.state().runnable_usage, 0);
    assert_eq!(runtime.state().queue_usage, 0);
    assert_eq!(runtime.plan().profile_ref, profile.profile_ref);
    assert_eq!(runtime.plan().generation, GENERATION);
}

#[test]
fn allocation_failure_denies_without_smaller_fallback() {
    let profile = profile();
    let result = activate_with(&profile, GENERATION, |kind, _slots| {
        if kind == ReservationKind::Queue {
            Err(())
        } else {
            Ok(())
        }
    });

    assert!(matches!(result, Err(ActivationError::ReservationFailed(ReservationKind::Queue))));
}

// r[verify molten.fabric_time.scheduler_capacity.steady_state]
// r[verify molten.fabric_time.scheduler_capacity.boundary]
#[test]
fn steady_state_is_bounded_and_generation_fenced_without_growth() {
    let profile = profile();
    let mut runtime = Runtime::activate(&profile, GENERATION).expect("activation");
    let runnable_capacity = runtime.runnable_capacity();
    let queue_capacity = runtime.queue_capacity();

    let admitted = runtime.apply(&profile.profile_ref, GENERATION, molten_core::fabric_time::capacity::UseRequest {
        runnable_delta: 1,
        queue_delta: 1,
    });
    assert_eq!(admitted, molten_core::fabric_time::capacity::UseDecisionKind::Admit);
    assert_eq!(runtime.runnable_capacity(), runnable_capacity);
    assert_eq!(runtime.queue_capacity(), queue_capacity);

    let stale = runtime.apply(&profile.profile_ref, STALE_GENERATION, molten_core::fabric_time::capacity::UseRequest {
        runnable_delta: 1,
        queue_delta: 0,
    });
    assert_eq!(stale, molten_core::fabric_time::capacity::UseDecisionKind::StaleGeneration);
    assert_eq!(runtime.state().runnable_usage, 1);

    let wrong_profile = runtime.apply(
        "blake3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        GENERATION,
        molten_core::fabric_time::capacity::UseRequest {
            runnable_delta: 1,
            queue_delta: 0,
        },
    );
    assert_eq!(wrong_profile, molten_core::fabric_time::capacity::UseDecisionKind::ProfileMismatch);
}

// r[verify molten.fabric_time.scheduler_capacity.observation]
#[test]
fn exhaustion_release_and_observation_remain_scoped() {
    let profile = profile();
    let mut runtime = Runtime::activate(&profile, GENERATION).expect("activation");
    let exhausted = runtime.apply(&profile.profile_ref, GENERATION, molten_core::fabric_time::capacity::UseRequest {
        runnable_delta: i64::try_from(RUNNABLE_LIMIT + 1).expect("delta"),
        queue_delta: 0,
    });
    assert_eq!(exhausted, molten_core::fabric_time::capacity::UseDecisionKind::Exhausted);
    assert_eq!(runtime.state().exhaustion_count, 1);

    runtime.release();
    let observation = runtime.observation();
    assert!(observation.is_released);
    assert_eq!(runtime.runnable_capacity(), 0);
    assert_eq!(runtime.queue_capacity(), 0);
    assert!(observation.non_claims.contains(&"does-not-prove-global-latency"));
    assert!(observation.non_claims.contains(&"does-not-prove-whole-runtime-zero-allocation"));
}

#[test]
fn accounting_preserves_existing_fifo_selection() {
    let profile = profile();
    let mut runtime = Runtime::activate(&profile, GENERATION).expect("activation");
    let key = molten_core::fabric_time::RunnableKey {
        service_id: "service.scheduler".to_string(),
        generation: GENERATION,
        runnable_id: "runnable-a".to_string(),
    };
    let state = molten_core::fabric_time::new_scheduler_state(&profile, GENERATION);
    let capacity_decision =
        runtime.apply(&profile.profile_ref, GENERATION, molten_core::fabric_time::capacity::UseRequest {
            runnable_delta: 1,
            queue_delta: 1,
        });
    assert_eq!(capacity_decision, molten_core::fabric_time::capacity::UseDecisionKind::Admit);
    let transition = molten_core::fabric_time::apply_scheduler_command(
        &profile,
        profile.scheduler_policy,
        &state,
        GENERATION,
        &molten_core::fabric_time::SchedulerCommand::Wake {
            key: key.clone(),
            priority: 0,
        },
    )
    .expect("wake");
    let selection = molten_core::fabric_time::choose_runnable(
        &profile,
        profile.scheduler_policy,
        &transition.next,
        GENERATION,
        Some(&key),
    )
    .expect("selection");

    assert_eq!(selection.selected, key);
    assert_eq!(selection.choice_sequence, 1);
}

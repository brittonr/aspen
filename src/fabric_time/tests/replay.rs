use super::*;

const BASE_DELAY: u64 = 2;
const SATURATING_ATTEMPT: u64 = 63;
const DEADLINE_EVENT: usize = 0;
const DELAY_EVENT: usize = 1;

fn input<'a>(profile: &Profile, recorded: &'a [Event]) -> FixtureRetryInput<'a> {
    FixtureRetryInput {
        now: TimeValue::Virtual(VirtualInstant {
            profile_ref: profile.profile.profile_ref.clone(),
            ticks: TIMER_DEADLINE,
        }),
        attempt: SATURATING_ATTEMPT,
        policy: RetryPolicy {
            maximum_attempts: u64::MAX,
            base_delay_ticks: BASE_DELAY,
            maximum_delay_ticks: PROFILE_LIMIT,
            backoff: RetryBackoff::Exponential,
            jitter: RetryJitter::None,
        },
        jitter: None,
        recorded,
    }
}

fn events(profile: &Profile) -> Vec<Event> {
    let request = input(profile, &[]);
    crate::fabric_time::fixture::retry_events(profile, &request.now, request.attempt, request.policy, request.jitter)
        .expect("admitted fixture")
        .to_vec()
}

// r[verify molten.audit_f12.validation]
#[test]
fn accepts_matching_saturated_and_fixed_observations() {
    let profile = simulation_profile();
    let recorded = events(&profile);
    replay_fixture_retry(&profile, input(&profile, &recorded)).expect("matching saturated observations");
    let mut request = input(&profile, &[]);
    request.policy.backoff = RetryBackoff::Fixed;
    let fixed = crate::fabric_time::fixture::retry_events(
        &profile,
        &request.now,
        request.attempt,
        request.policy,
        request.jitter,
    )
    .expect("fixed observations");
    request.recorded = &fixed;
    replay_fixture_retry(&profile, request).expect("matching fixed observations");
}

// r[verify molten.audit_f12.validation]
#[test]
fn rejects_wrapped_history_without_rewriting_it() {
    let profile = simulation_profile();
    let mut recorded = events(&profile);
    recorded[DEADLINE_EVENT] = named_event(EventHeader {
        profile_ref: &profile.profile_ref,
        kind: EventKind::Deadline,
        generation: GENERATION,
        subject: "fixture-retry",
        action: "retry-planned",
        ticks: TIMER_DEADLINE,
    })
    .expect("historical deadline");
    recorded[DELAY_EVENT] = named_event(EventHeader {
        profile_ref: &profile.profile_ref,
        kind: EventKind::Deadline,
        generation: GENERATION,
        subject: "fixture-retry",
        action: "retry-delay",
        ticks: 0,
    })
    .expect("historical delay");
    let original = recorded.clone();
    let error = replay_fixture_retry(&profile, input(&profile, &recorded)).expect_err("wrapped history diverges");
    assert!(error.to_string().contains("retry replay diverged"));
    assert_eq!(recorded, original);
}

// r[verify molten.audit_f12.validation]
#[test]
fn rejects_missing_extra_reordered_and_tampered_observations() {
    let profile = simulation_profile();
    let recorded = events(&profile);
    let mut extra = recorded.clone();
    extra.push(recorded[DELAY_EVENT].clone());
    let mut reordered = recorded.clone();
    reordered.reverse();
    let mut tampered = recorded.clone();
    tampered[DELAY_EVENT].evidence_ref = HASH_B.to_owned();
    for invalid in [
        Vec::new(),
        vec![recorded[DEADLINE_EVENT].clone()],
        extra,
        reordered,
        tampered,
    ] {
        let error = replay_fixture_retry(&profile, input(&profile, &invalid)).expect_err("invalid observations");
        assert!(error.to_string().contains("retry replay diverged"));
    }
}

// r[verify molten.audit_f12.validation]
#[test]
fn rejects_each_changed_event_with_valid_canonical_identity() {
    let profile = simulation_profile();
    let recorded = events(&profile);
    for (index, status) in [(DEADLINE_EVENT, "retry-planned"), (DELAY_EVENT, "retry-delay")] {
        let mut changed = recorded.clone();
        changed[index] = named_event(EventHeader {
            profile_ref: &profile.profile_ref,
            kind: EventKind::Deadline,
            generation: GENERATION,
            subject: "fixture-retry",
            action: status,
            ticks: 0,
        })
        .expect("canonical but divergent event");
        let error = replay_fixture_retry(&profile, input(&profile, &changed)).expect_err("changed event");
        assert!(error.to_string().contains("retry replay diverged"));
    }
}

// r[verify molten.audit_f12.validation]
#[test]
fn rejects_invalid_planner_inputs_before_comparison() {
    let profile = simulation_profile();
    let recorded = events(&profile);
    let mut exhausted = input(&profile, &recorded);
    exhausted.attempt = u64::MAX;
    let error = replay_fixture_retry(&profile, exhausted).expect_err("exhausted budget");
    assert!(error.to_string().contains("RetryExhausted"));
    let mut jitter = input(&profile, &recorded);
    jitter.jitter = Some(0);
    let error = replay_fixture_retry(&profile, jitter).expect_err("unexpected jitter");
    assert!(error.to_string().contains("JitterOutOfBounds"));
}

const RETENTION_TEST_LIMIT: u64 = 3;
const FIRST_RETAINED_WAKE: usize = 0;
const FIRST_GENERATION_CLEANUP: usize = 8;

fn retention_profile() -> Profile {
    let mut declaration = descriptor(TimeProfileKind::DeterministicSimulation, "retention-replay", HASH_A);
    declaration.max_runnables = RETENTION_TEST_LIMIT;
    declaration.max_scheduler_concurrency = 1;
    declaration.max_scheduler_queue_depth = 1;
    admit_profile(&declaration).expect("bounded retention replay profile")
}

// r[verify molten.audit_f11.freshness]
// r[verify molten.audit_f11.validation]
#[test]
fn scheduler_retention_distinguishes_reused_logical_names() {
    let profile = retention_profile();
    let recorded =
        crate::fabric_time::fixture::scheduler_retention_events(&profile).expect("actual retention execution");
    crate::fabric_time::fixture::replay_scheduler_retention(&profile, &recorded).expect("matching retention replay");
    let mut substituted = recorded.clone();
    let fresh_wake = substituted.len() - 2;
    substituted[fresh_wake] = recorded[FIRST_RETAINED_WAKE].clone();
    crate::fabric_time::fixture::replay_scheduler_retention(&profile, &substituted)
        .expect_err("old canonical occurrence cannot stand in for fresh same-name work");
}

// r[verify molten.audit_f11.retention]
// r[verify molten.audit_f11.validation]
#[test]
fn scheduler_retention_rejects_missing_reordered_and_tampered_reclamation() {
    let profile = retention_profile();
    let recorded =
        crate::fabric_time::fixture::scheduler_retention_events(&profile).expect("actual retention execution");
    let mut missing = recorded.clone();
    missing.pop();
    let mut extra = recorded.clone();
    extra.push(recorded[FIRST_GENERATION_CLEANUP].clone());
    let mut reordered = recorded.clone();
    reordered.swap(FIRST_RETAINED_WAKE, FIRST_GENERATION_CLEANUP);
    let mut tampered = recorded.clone();
    tampered[FIRST_GENERATION_CLEANUP].evidence_ref = HASH_B.to_owned();
    let mut old_cleanup = recorded.clone();
    let terminal_cleanup = old_cleanup.len() - 1;
    old_cleanup[terminal_cleanup] = recorded[FIRST_GENERATION_CLEANUP].clone();
    for invalid in [missing, extra, reordered, tampered, old_cleanup] {
        crate::fabric_time::fixture::replay_scheduler_retention(&profile, &invalid)
            .expect_err("changed retention history must diverge");
    }
}

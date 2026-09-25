use super::*;

const BASE_DELAY: u64 = 2;
const SATURATING_ATTEMPT: u64 = 63;
const DEADLINE_EVENT: usize = 0;
const DELAY_EVENT: usize = 1;

fn input<'a>(profile: &CanonicalTimeProfile, recorded: &'a [CanonicalTimeEvent]) -> FixtureRetryInput<'a> {
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

fn events(profile: &CanonicalTimeProfile) -> Vec<CanonicalTimeEvent> {
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
    recorded[DEADLINE_EVENT] = canonical_named_event(EventHeader {
        profile_ref: &profile.profile_ref,
        kind: CanonicalTimeEventKind::Deadline,
        generation: GENERATION,
        subject: "fixture-retry",
        action: "retry-planned",
        ticks: TIMER_DEADLINE,
    })
    .expect("historical deadline");
    recorded[DELAY_EVENT] = canonical_named_event(EventHeader {
        profile_ref: &profile.profile_ref,
        kind: CanonicalTimeEventKind::Deadline,
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
        changed[index] = canonical_named_event(EventHeader {
            profile_ref: &profile.profile_ref,
            kind: CanonicalTimeEventKind::Deadline,
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

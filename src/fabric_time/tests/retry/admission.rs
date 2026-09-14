use super::support::*;
use super::*;

// r[verify molten.audit_f12.compatibility]
#[test]
fn admitted_retry_reaches_the_clock_and_records_observed_timer_delivery() {
    let profile = simulation_profile();
    let request = input(&profile);
    let before = request.clone();
    let mut clock = RecordingClock::new(&profile);
    let mut events = Vec::new();
    let plan = fixture_retry::execute(&profile, GENERATION, &request, &mut clock, &mut events)
        .expect("admitted retry execution");
    assert_eq!(plan.delay.ticks, SATURATED_DELAY);
    assert_eq!(clock.waits, vec![SATURATED_DEADLINE]);
    assert_eq!(clock.now, SATURATED_DEADLINE);
    assert_eq!(clock.reads, 0);
    assert_eq!(events.len(), EXECUTION_EVENT_COUNT);
    assert_eq!(
        events.last().expect("timer observation"),
        &expected_timer_event(&profile, &request.subject_id, SATURATED_DEADLINE)
    );
    assert_eq!(request, before);
}

// r[verify molten.audit_f12.compatibility]
#[test]
fn retry_denials_preserve_clock_and_consumer_state_with_exact_core_error_translation() {
    let profile = simulation_profile();
    for (request, expected) in denied_requests(&profile) {
        let original_request = request.clone();
        let mut clock = RecordingClock::new(&profile);
        let original_clock = clock.clone();
        let mut events = vec![sentinel(&profile)];
        let original_events = events.clone();
        let error = fixture_retry::execute(&profile, GENERATION, &request, &mut clock, &mut events)
            .expect_err("core denial must precede clock effects");
        assert_eq!(error, crate::error::MoltenError::invalid_harness(format!("plan fixture retry: {expected:?}")));
        assert_eq!(clock, original_clock);
        assert_eq!(events, original_events);
        assert_eq!(request, original_request);
    }
}

fn denied_requests(profile: &CanonicalTimeProfile) -> Vec<(fixture_retry::RetryFixtureInput, DeadlineLeaseError)> {
    let mut jitter = input(profile);
    jitter.jitter = Some(1);
    let mut missing_jitter = input(profile);
    missing_jitter.policy.jitter = RetryJitter::Bounded { maximum_ticks: 1 };
    let mut exhausted = input(profile);
    exhausted.attempt = ATTEMPT_LIMIT;
    let mut overflow = input(profile);
    overflow.now = TimeValue::Virtual(VirtualInstant {
        profile_ref: profile.profile.profile_ref.clone(),
        ticks: u64::MAX - (SATURATED_DELAY - 1),
    });
    let mut stale = input(profile);
    stale.generation = STALE_GENERATION;
    let mut zero_generation = input(profile);
    zero_generation.generation = 0;
    let mut wrong_profile = input(profile);
    wrong_profile.now = TimeValue::Virtual(VirtualInstant {
        profile_ref: HASH_B.to_string(),
        ticks: FIXTURE_RETRY_NOW,
    });
    vec![
        (jitter, DeadlineLeaseError::JitterOutOfBounds { actual: 1, maximum: 0 }),
        (missing_jitter, DeadlineLeaseError::JitterRequired),
        (exhausted, DeadlineLeaseError::RetryExhausted {
            attempt: ATTEMPT_LIMIT,
            maximum: ATTEMPT_LIMIT,
        }),
        (overflow, DeadlineLeaseError::Arithmetic(TimeArithmeticError::Overflow)),
        (stale, DeadlineLeaseError::StaleGeneration {
            expected: GENERATION,
            actual: STALE_GENERATION,
        }),
        (zero_generation, DeadlineLeaseError::ZeroGeneration),
        (
            wrong_profile,
            DeadlineLeaseError::Arithmetic(TimeArithmeticError::ProfileMismatch {
                expected: profile.profile.profile_ref.clone(),
                actual: HASH_B.to_string(),
            }),
        ),
    ]
}

// r[verify molten.audit_f12.compatibility]
#[test]
fn clock_profile_and_domain_substitution_deny_before_wait_or_publication() {
    let profile = simulation_profile();
    let request = input(&profile);
    let mut wrong_profile = RecordingClock::new(&profile);
    wrong_profile.profile_ref = HASH_B.to_string();
    let mut wrong_domain = RecordingClock::new(&profile);
    wrong_domain.domain = TimeDomain::Monotonic;
    for mut clock in [wrong_profile, wrong_domain] {
        let before_clock = clock.clone();
        let mut events = vec![sentinel(&profile)];
        let before_events = events.clone();
        let error = fixture_retry::execute(&profile, GENERATION, &request, &mut clock, &mut events)
            .expect_err("clock substitution");
        assert_eq!(error, crate::error::MoltenError::invalid_harness("retry fixture clock profile or domain mismatch"));
        assert_eq!(clock, before_clock);
        assert_eq!(events, before_events);
    }
}

// r[verify molten.audit_f12.compatibility]
#[test]
fn timer_port_failure_remains_distinct_from_planner_denial() {
    let profile = simulation_profile();
    let request = input(&profile);
    let mut clock = RecordingClock::new(&profile);
    clock.behavior = ClockBehavior::Fail;
    let mut events = vec![sentinel(&profile)];
    let before_events = events.clone();
    let error = fixture_retry::execute(&profile, GENERATION, &request, &mut clock, &mut events)
        .expect_err("controlled timer failure");
    let expected = crate::fabric::FabricPortError::Timeout {
        message: CLOCK_ERROR.to_string(),
    };
    assert_eq!(error, crate::error::MoltenError::from(expected));
    assert_eq!(clock.waits, vec![SATURATED_DEADLINE]);
    assert_eq!(clock.now, FIXTURE_RETRY_NOW);
    assert_eq!(events, before_events);
}

// r[verify molten.audit_f12.compatibility]
#[test]
fn early_adapter_return_does_not_publish_a_timer_delivery() {
    let profile = simulation_profile();
    let request = input(&profile);
    let mut clock = RecordingClock::new(&profile);
    clock.behavior = ClockBehavior::ReturnEarly;
    let mut events = vec![sentinel(&profile)];
    let before_events = events.clone();
    let error = fixture_retry::execute(&profile, GENERATION, &request, &mut clock, &mut events)
        .expect_err("early timer observation");
    assert_eq!(
        error,
        crate::error::MoltenError::invalid_harness("retry fixture clock did not reach the admitted deadline")
    );
    assert_eq!(clock.waits, vec![SATURATED_DEADLINE]);
    assert_eq!(events, before_events);
}

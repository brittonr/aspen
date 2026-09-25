
// r[verify molten.audit_f12.compatibility]
#[test]
fn retry_observation_denial_preserves_existing_events() {
    let profile = simulation_profile();
    let policy = RetryPolicy {
        maximum_attempts: 1,
        base_delay_ticks: TIMER_DELAY,
        maximum_delay_ticks: PROFILE_LIMIT,
        backoff: RetryBackoff::Fixed,
        jitter: RetryJitter::None,
    };
    let now = TimeValue::Virtual(VirtualInstant {
        profile_ref: profile.profile.profile_ref.clone(),
        ticks: TIMER_DEADLINE,
    });
    let mut events = Vec::new();
    events.extend(super::fixture::retry_events(&profile, &now, 0, policy, None).expect("fixed retry"));
    let original = events.clone();
    let error = super::fixture::retry_events(&profile, &now, 1, policy, None)
        .map(|batch| events.extend(batch))
        .expect_err("exhausted");
    assert!(error.to_string().contains("RetryExhausted"));
    assert_eq!(events, original);
    let error = super::fixture::retry_events(&profile, &now, 0, policy, Some(0))
        .map(|batch| events.extend(batch))
        .expect_err("jitter");
    assert!(error.to_string().contains("JitterOutOfBounds"));
    assert_eq!(events, original);
    let overflow = TimeValue::Virtual(VirtualInstant {
        profile_ref: profile.profile.profile_ref.clone(),
        ticks: u64::MAX,
    });
    let error = super::fixture::retry_events(&profile, &overflow, 0, policy, None)
        .map(|batch| events.extend(batch))
        .expect_err("overflow");
    assert!(error.to_string().contains("Overflow"));
    assert_eq!(events, original);
}

// r[verify molten.modularity.fabric_boundary.adapters.clock]
// r[verify molten.modularity.fabric_boundary.validation]
#[test]
fn live_adapter_rejects_simulation_profile() {
    let profile = simulation_profile();
    let error = LiveClockAdapter::new(&profile.profile, 0).expect_err("simulation profile cannot use live clock");
    assert!(error.to_string().contains("live time profile"));
}

// r[verify molten.fabric_time.live_sim_parity]
#[test]
fn tick_deadline_expires_exactly_at_its_timeout_on_the_virtual_clock() {
    let profile = simulation_profile();
    let mut clock = VirtualClockAdapter::new(&profile.profile, TIMER_DEADLINE, WALL_BASE).expect("virtual clock");
    let deadline = TickDeadline::after(&mut clock, TIMER_DELAY).expect("deadline inside the clock domain");

    clock
        .await_ticks(TIMER_DEADLINE + TIMER_DELAY - 1)
        .expect("advance to one tick before the deadline");
    assert_eq!(deadline.remaining_ticks(&mut clock).expect("remaining ticks"), 1);
    assert!(!deadline.is_expired(&mut clock).expect("deadline check"));

    clock.await_ticks(TIMER_DEADLINE + TIMER_DELAY).expect("advance to the deadline");
    assert_eq!(deadline.remaining_ticks(&mut clock).expect("remaining ticks"), 0);
    assert!(deadline.is_expired(&mut clock).expect("deadline check"));
}

#[test]
fn tick_deadline_denies_a_timeout_past_the_clock_domain() {
    let profile = simulation_profile();
    let mut clock = VirtualClockAdapter::new(&profile.profile, TIMER_DEADLINE, WALL_BASE).expect("virtual clock");
    assert!(TickDeadline::after(&mut clock, u64::MAX - TIMER_DEADLINE).is_ok());
    assert!(TickDeadline::after(&mut clock, u64::MAX - TIMER_DEADLINE + 1).is_err());
}

#[test]
fn supervision_deadline_admits_the_supervision_bound_and_denies_one_past() {
    const SUPERVISION_BOUND: std::time::Duration = std::time::Duration::from_secs(3_600);
    let mut deadline = SupervisionDeadline::after(SUPERVISION_BOUND).expect("the admitted supervision bound");
    assert!(!deadline.is_expired().expect("fresh deadline check"));
    assert!(deadline.remaining().expect("remaining time") <= SUPERVISION_BOUND);

    let one_past = SUPERVISION_BOUND + std::time::Duration::from_nanos(1);
    let error = SupervisionDeadline::after(one_past).expect_err("one nanosecond past the bound denies");
    assert!(error.to_string().contains("exceeds the admitted maximum"));

    let mut elapsed = SupervisionDeadline::after(std::time::Duration::ZERO).expect("zero timeout");
    assert!(elapsed.is_expired().expect("zero timeout is already expired"));
}

use super::*;

const BLAKE3_HEX_LEN: usize = 64;
const ACTIVE_GENERATION: u64 = 1;
const STALE_GENERATION: u64 = 2;
const PROFILE_LIMIT: u64 = 128;
const ENTROPY_TOTAL_LIMIT: u64 = 1_024;
const CONCURRENCY_LIMIT: u64 = 4;
const QUEUE_LIMIT: u64 = 8;
const FAIRNESS_TURNS: u64 = 3;
const DEADLINE_TICKS: u64 = 10;
const LATE_TICKS: u64 = 35;
const PERIOD_TICKS: u64 = 10;
const MAX_LATENESS: u64 = 100;
const THREE_PERIODS: u64 = 3;
const CHOICE_UPPER: u64 = 17;
const ENTROPY_BYTES: u64 = 16;
const SIMULATION_SEED: u64 = 42;
const FORWARD_JUMP_LIMIT: u64 = 20;
const WALL_START: u64 = 100;
const WALL_FORWARD_JUMP: u64 = 150;
const RETRY_BASE: u64 = 5;
const RETRY_MAX: u64 = 40;
const RETRY_ATTEMPTS: u64 = 4;
const RETRY_ATTEMPT: u64 = 2;
const RETRY_JITTER: u64 = 3;
const EXPECTED_RETRY_DELAY: u64 = 23;
const LEASE_EXPIRY: u64 = 20;
const FENCING_TOKEN: u64 = 8;
const PREVIOUS_FENCING_TOKEN: u64 = 7;
const FAIRNESS_PROBE_CHOICES: usize = 3;
const SECOND_TIMER_SEQUENCE: u64 = 2;
const HALF_DIVISOR: u64 = 2;

fn reference(character: char) -> String {
    format!("blake3:{}", character.to_string().repeat(BLAKE3_HEX_LEN))
}

fn descriptor(kind: TimeProfileKind) -> TimeProfileDescriptor {
    let replay = match kind {
        TimeProfileKind::Live => SchedulerReplayPolicy::RecordedChoiceRequired,
        TimeProfileKind::DeterministicSimulation => SchedulerReplayPolicy::Deterministic,
    };
    TimeProfileDescriptor {
        schema: FABRIC_TIME_PROFILE_SCHEMA.to_string(),
        profile_id: "profile.test".to_string(),
        profile_ref: reference('a'),
        kind,
        supported_domains: REQUIRED_TIME_DOMAINS.to_vec(),
        max_duration_ticks: PROFILE_LIMIT,
        max_uncertainty_ticks: PROFILE_LIMIT,
        max_timers: PROFILE_LIMIT,
        max_runnables: PROFILE_LIMIT,
        max_entropy_request_bytes: PROFILE_LIMIT,
        max_entropy_total_bytes: ENTROPY_TOTAL_LIMIT,
        max_scheduler_concurrency: CONCURRENCY_LIMIT,
        max_scheduler_queue_depth: QUEUE_LIMIT,
        fairness_bound_turns: Some(FAIRNESS_TURNS),
        scheduler_policy: scheduler_policy(replay),
        evidence_mode: TimeEvidenceMode::SelectedSemanticBoundaries,
        non_claims: REQUIRED_TIME_NON_CLAIMS.to_vec(),
    }
}

fn profile() -> AdmittedTimeProfile {
    admit_time_profile(&descriptor(TimeProfileKind::DeterministicSimulation)).expect("valid profile")
}

fn virtual_time(ticks: u64) -> TimeValue {
    TimeValue::Virtual(VirtualInstant {
        profile_ref: reference('a'),
        ticks,
    })
}

fn timer_request(generation: u64, kind: TimerKind) -> TimerScheduleRequest {
    TimerScheduleRequest {
        profile_ref: reference('a'),
        key: TimerKey {
            service_id: "service.timer".to_string(),
            generation,
            sequence: 0,
        },
        domain: TimeDomain::Virtual,
        deadline_ticks: DEADLINE_TICKS,
        kind,
        ordering_key: 0,
        coalescing: TimerCoalescingPolicy::CoalesceLatest,
        lateness: TimerLatenessPolicy::DeliverWithin {
            max_lateness_ticks: MAX_LATENESS,
        },
        overload: TimerOverloadPolicy::RejectAndRetain,
        resource_charge: TimerResourceCharge::single_slot(),
    }
}

fn runnable(id: &str, generation: u64) -> RunnableKey {
    RunnableKey {
        service_id: "service.scheduler".to_string(),
        generation,
        runnable_id: id.to_string(),
    }
}

fn scheduler_policy(replay: SchedulerReplayPolicy) -> SchedulerPolicy {
    SchedulerPolicy {
        ordering: SchedulerOrdering::PriorityThenFifo,
        replay,
        overload: SchedulerOverloadPolicy::Reject,
    }
}

fn open_simulation_stream() -> EntropyStreamState {
    open_entropy_stream(&profile(), ACTIVE_GENERATION, &EntropyStreamRequest {
        profile_ref: reference('a'),
        stream_id: "stream.test".to_string(),
        purpose: "scheduler-choice".to_string(),
        capability_ref: reference('b'),
        generation: ACTIVE_GENERATION,
        mode: EntropyMode::DeterministicSimulation,
        explicit_simulation_seed: Some(SIMULATION_SEED),
        explicit_simulation_seed_ref: Some(reference('c')),
    })
    .expect("simulation stream")
}

#[test]
fn admits_complete_profile_and_canonicalizes_sets() {
    let admitted = profile();
    assert_eq!(admitted.supported_domains, REQUIRED_TIME_DOMAINS);
    assert_eq!(admitted.non_claims, REQUIRED_TIME_NON_CLAIMS);
}

#[test]
fn rejects_incomplete_profile_and_hard_limit() {
    let mut invalid = descriptor(TimeProfileKind::Live);
    invalid.supported_domains.pop();
    invalid.non_claims.pop();
    invalid.max_timers = u64::MAX;
    let issues = admit_time_profile(&invalid).expect_err("incomplete profile must fail");
    assert!(issues.iter().any(|issue| matches!(issue, TimeProfileIssue::MissingDomain(_))));
    assert!(issues.iter().any(|issue| matches!(issue, TimeProfileIssue::MissingNonClaim(_))));
    assert!(issues.iter().any(|issue| matches!(issue, TimeProfileIssue::HardLimitExceeded {
        field: "max-timers",
        ..
    })));
}

#[test]
fn time_arithmetic_is_domain_typed_and_checked() {
    let profile = profile();
    let duration = CheckedDuration {
        profile_ref: profile.profile_ref.clone(),
        domain: TimeDomain::Virtual,
        ticks: DEADLINE_TICKS,
    };
    assert_eq!(
        checked_add_duration(&profile, &virtual_time(DEADLINE_TICKS), &duration)
            .expect("checked add")
            .ticks(),
        DEADLINE_TICKS + DEADLINE_TICKS
    );
    let logical = TimeValue::Logical(LogicalEventTime {
        profile_ref: profile.profile_ref.clone(),
        position: DEADLINE_TICKS,
    });
    assert!(matches!(
        compare_time_values(&profile, &virtual_time(DEADLINE_TICKS), &logical),
        Err(TimeArithmeticError::DomainMismatch { .. })
    ));
    let maximum = virtual_time(u64::MAX);
    assert_eq!(checked_add_duration(&profile, &maximum, &duration), Err(TimeArithmeticError::Overflow));
}

#[test]
fn explicit_conversion_requires_evidence_and_target_profile() {
    let source = profile();
    let mut target_descriptor = descriptor(TimeProfileKind::Live);
    target_descriptor.profile_id = "profile.target".to_string();
    target_descriptor.profile_ref = reference('c');
    let target = admit_time_profile(&target_descriptor).expect("target profile");
    let conversion = ExplicitTimeConversion {
        source_profile_ref: source.profile_ref.clone(),
        target_profile_ref: target.profile_ref.clone(),
        source_domain: TimeDomain::Virtual,
        target_domain: TimeDomain::Monotonic,
        signed_offset_ticks: i128::from(DEADLINE_TICKS),
        uncertainty_ticks: 0,
        target_observation_sequence: 0,
        conversion_evidence_ref: reference('d'),
    };
    let converted =
        convert_time_value(&source, &target, &virtual_time(DEADLINE_TICKS), &conversion).expect("explicit conversion");
    assert_eq!(converted.domain(), TimeDomain::Monotonic);
    assert_eq!(converted.ticks(), DEADLINE_TICKS + DEADLINE_TICKS);

    let mut malformed = conversion;
    malformed.conversion_evidence_ref = "transport-name".to_string();
    assert!(matches!(
        convert_time_value(&source, &target, &virtual_time(0), &malformed),
        Err(TimeArithmeticError::MalformedConversionEvidence(_))
    ));
}

#[test]
fn wall_clock_anomalies_are_explicit() {
    let previous = WallClockObservation {
        profile_ref: reference('a'),
        unix_nanos: WALL_START,
        uncertainty_nanos: 0,
        observation_sequence: 1,
    };
    let observed = WallClockObservation {
        observation_sequence: STALE_GENERATION,
        unix_nanos: WALL_FORWARD_JUMP,
        ..previous.clone()
    };
    let decision = classify_wall_clock_observation(&previous, &observed, WallClockAnomalyPolicy {
        max_forward_jump_nanos: FORWARD_JUMP_LIMIT,
        max_uncertainty_nanos: PROFILE_LIMIT,
    })
    .expect("classify jump");
    assert_eq!(decision.kind, WallClockAnomalyKind::ForwardJump);

    let duplicate = WallClockObservation {
        observation_sequence: previous.observation_sequence,
        ..observed
    };
    assert_eq!(
        classify_wall_clock_observation(&previous, &duplicate, WallClockAnomalyPolicy {
            max_forward_jump_nanos: FORWARD_JUMP_LIMIT,
            max_uncertainty_nanos: PROFILE_LIMIT,
        },),
        Err(TimeArithmeticError::NonIncreasingSequence)
    );
}

#[test]
fn one_shot_timer_fires_once_and_rejects_duplicate_poll() {
    let state = schedule_timer(&profile(), ACTIVE_GENERATION, 0, &timer_request(ACTIVE_GENERATION, TimerKind::OneShot))
        .expect("schedule");
    let fired = poll_timer(&state, ACTIVE_GENERATION, DEADLINE_TICKS, 1).expect("fire");
    assert_eq!(fired.action, TimerAction::Deliver);
    assert_eq!(fired.next.phase, TimerPhase::Completed);
    assert!(matches!(
        poll_timer(&fired.next, ACTIVE_GENERATION, DEADLINE_TICKS, 1),
        Err(TimerError::TerminalTimer(TimerPhase::Completed))
    ));
}

#[test]
fn periodic_timer_coalesces_and_advances_past_observation() {
    let state = schedule_timer(
        &profile(),
        ACTIVE_GENERATION,
        0,
        &timer_request(ACTIVE_GENERATION, TimerKind::Periodic {
            period_ticks: PERIOD_TICKS,
        }),
    )
    .expect("schedule periodic");
    let fired = poll_timer(&state, ACTIVE_GENERATION, LATE_TICKS, 1).expect("poll");
    assert_eq!(fired.delivery_count, 1);
    assert_eq!(fired.skipped_count, THREE_PERIODS - 1);
    assert!(fired.next.next_deadline_ticks > LATE_TICKS);
}

#[test]
fn timer_lateness_policy_records_drop_without_delivery() {
    let mut request = timer_request(ACTIVE_GENERATION, TimerKind::OneShot);
    request.lateness = TimerLatenessPolicy::DeliverWithin { max_lateness_ticks: 0 };
    let state = schedule_timer(&profile(), ACTIVE_GENERATION, 0, &request).expect("schedule late timer");
    let late_observation = DEADLINE_TICKS + 1;
    let dropped = poll_timer(&state, ACTIVE_GENERATION, late_observation, 1).expect("drop late timer");
    assert_eq!(dropped.action, TimerAction::DroppedLate);
    assert_eq!(dropped.delivery_count, 0);
    assert_eq!(dropped.next.phase, TimerPhase::Completed);
}

#[test]
fn timers_fence_generations_cancel_and_report_overload() {
    let state = schedule_timer(&profile(), ACTIVE_GENERATION, 0, &timer_request(ACTIVE_GENERATION, TimerKind::OneShot))
        .expect("schedule");
    let stale = poll_timer(&state, STALE_GENERATION, DEADLINE_TICKS, 1).expect("stale is discarded");
    assert_eq!(stale.action, TimerAction::DiscardedStaleGeneration);
    let overloaded = poll_timer(&state, ACTIVE_GENERATION, DEADLINE_TICKS, 0).expect("overload");
    assert_eq!(overloaded.action, TimerAction::RetainedOverload);
    let cancelled = cancel_timer(&state, ACTIVE_GENERATION).expect("cancel");
    assert_eq!(cancelled.next.phase, TimerPhase::Cancelled);
    assert!(cancel_timer(&cancelled.next, ACTIVE_GENERATION).is_err());
}

#[test]
fn simultaneous_timers_are_stably_ordered_and_retired_generations_are_cleaned() {
    let profile = profile();
    let first = schedule_timer(&profile, ACTIVE_GENERATION, 0, &timer_request(ACTIVE_GENERATION, TimerKind::OneShot))
        .expect("first timer");
    let mut second_request = timer_request(ACTIVE_GENERATION, TimerKind::OneShot);
    second_request.key.sequence = SECOND_TIMER_SEQUENCE;
    second_request.ordering_key = 1;
    let second = schedule_timer(&profile, ACTIVE_GENERATION, 1, &second_request).expect("second timer");
    let ordered = order_due_timers(&[second.clone(), first.clone()], ACTIVE_GENERATION, DEADLINE_TICKS, PROFILE_LIMIT)
        .expect("order due timers");
    assert_eq!(ordered, vec![first.key.clone(), second.key.clone()]);
    let cleaned = cleanup_generation(&[first, second], ACTIVE_GENERATION);
    assert!(cleaned.iter().all(|timer| timer.phase == TimerPhase::Cancelled));
}

#[test]
fn timer_admission_is_resource_bounded() {
    assert!(matches!(
        schedule_timer(
            &profile(),
            ACTIVE_GENERATION,
            PROFILE_LIMIT,
            &timer_request(ACTIVE_GENERATION, TimerKind::OneShot),
        ),
        Err(TimerError::TimerLimitExceeded { .. })
    ));
    let mut invalid = timer_request(ACTIVE_GENERATION, TimerKind::Periodic { period_ticks: 0 });
    invalid.coalescing = TimerCoalescingPolicy::DeliverEach { max_catch_up: 0 };
    assert_eq!(schedule_timer(&profile(), ACTIVE_GENERATION, 0, &invalid), Err(TimerError::InvalidPeriod));
    invalid.kind = TimerKind::OneShot;
    invalid.coalescing = TimerCoalescingPolicy::CoalesceLatest;
    invalid.resource_charge.timer_slots = 0;
    assert_eq!(schedule_timer(&profile(), ACTIVE_GENERATION, 0, &invalid), Err(TimerError::InvalidResourceCharge));
}

#[test]
fn scheduler_orders_records_and_replays_choices() {
    let profile = profile();
    let mut state = new_scheduler_state(&profile, ACTIVE_GENERATION);
    let low = runnable("low", ACTIVE_GENERATION);
    let wrong_policy = SchedulerPolicy {
        ordering: SchedulerOrdering::Fifo,
        replay: SchedulerReplayPolicy::Deterministic,
        overload: SchedulerOverloadPolicy::Reject,
    };
    assert!(matches!(
        apply_scheduler_command(&profile, wrong_policy, &state, ACTIVE_GENERATION, &SchedulerCommand::Wake {
            key: low.clone(),
            priority: 0,
        },),
        Err(SchedulerError::PolicyMismatch { .. })
    ));
    let high = runnable("high", ACTIVE_GENERATION);
    for (key, priority) in [(low.clone(), 0), (high.clone(), 1)] {
        state = apply_scheduler_command(
            &profile,
            scheduler_policy(SchedulerReplayPolicy::Deterministic),
            &state,
            ACTIVE_GENERATION,
            &SchedulerCommand::Wake { key, priority },
        )
        .expect("wake")
        .next;
    }
    let selected = choose_runnable(
        &profile,
        scheduler_policy(SchedulerReplayPolicy::Deterministic),
        &state,
        ACTIVE_GENERATION,
        Some(&high),
    )
    .expect("choose high priority");
    assert_eq!(selected.selected, high);

    let recorded_profile = admit_time_profile(&descriptor(TimeProfileKind::Live)).expect("recorded profile");
    let recorded_policy = scheduler_policy(SchedulerReplayPolicy::RecordedChoiceRequired);
    let recorded_state = apply_scheduler_command(
        &recorded_profile,
        recorded_policy,
        &new_scheduler_state(&recorded_profile, ACTIVE_GENERATION),
        ACTIVE_GENERATION,
        &SchedulerCommand::Wake { key: high, priority: 1 },
    )
    .expect("wake recorded runnable")
    .next;
    assert!(matches!(
        choose_runnable(&recorded_profile, recorded_policy, &recorded_state, ACTIVE_GENERATION, None,),
        Err(SchedulerError::ReplayChoiceRequired)
    ));
}

#[test]
fn scheduler_rejects_duplicate_stale_and_illegal_transitions() {
    let profile = profile();
    let state = new_scheduler_state(&profile, ACTIVE_GENERATION);
    let key = runnable("work", ACTIVE_GENERATION);
    let woken = apply_scheduler_command(
        &profile,
        scheduler_policy(SchedulerReplayPolicy::Deterministic),
        &state,
        ACTIVE_GENERATION,
        &SchedulerCommand::Wake {
            key: key.clone(),
            priority: 0,
        },
    )
    .expect("wake");
    assert!(matches!(
        apply_scheduler_command(
            &profile,
            scheduler_policy(SchedulerReplayPolicy::Deterministic),
            &woken.next,
            ACTIVE_GENERATION,
            &SchedulerCommand::Wake {
                key: key.clone(),
                priority: 0
            },
        ),
        Err(SchedulerError::DuplicateRunnable(_))
    ));
    assert!(matches!(
        apply_scheduler_command(
            &profile,
            scheduler_policy(SchedulerReplayPolicy::Deterministic),
            &woken.next,
            ACTIVE_GENERATION,
            &SchedulerCommand::Complete { key },
        ),
        Err(SchedulerError::InvalidPhase { .. })
    ));
    let stale = apply_scheduler_command(
        &profile,
        scheduler_policy(SchedulerReplayPolicy::Deterministic),
        &woken.next,
        ACTIVE_GENERATION,
        &SchedulerCommand::Wake {
            key: runnable("stale", STALE_GENERATION),
            priority: 0,
        },
    )
    .expect("stale command is discarded");
    assert_eq!(stale.action, SchedulerAction::DiscardedStaleGeneration);
}

#[test]
fn fairness_bound_prevents_indefinite_starvation_and_replay_mismatch_fails() {
    let profile = profile();
    let policy = scheduler_policy(SchedulerReplayPolicy::Deterministic);
    let low = runnable("low-fairness", ACTIVE_GENERATION);
    let high = runnable("high-fairness", ACTIVE_GENERATION);
    let mut state = new_scheduler_state(&profile, ACTIVE_GENERATION);
    for (key, priority) in [(low.clone(), 0), (high.clone(), 1)] {
        state = apply_scheduler_command(&profile, policy, &state, ACTIVE_GENERATION, &SchedulerCommand::Wake {
            key,
            priority,
        })
        .expect("wake fairness runnable")
        .next;
    }
    for _ in 0..FAIRNESS_PROBE_CHOICES {
        let selected = choose_runnable(&profile, policy, &state, ACTIVE_GENERATION, Some(&high))
            .expect("high selected before fairness bound");
        state =
            apply_scheduler_command(&profile, policy, &selected.next, ACTIVE_GENERATION, &SchedulerCommand::Yield {
                key: high.clone(),
            })
            .expect("yield high")
            .next;
    }
    let fairness_selection = choose_runnable(&profile, policy, &state, ACTIVE_GENERATION, Some(&low))
        .expect("fairness selects starved runnable");
    assert_eq!(fairness_selection.selected, low);
    assert!(matches!(
        choose_runnable(&profile, policy, &state, ACTIVE_GENERATION, Some(&high),),
        Err(SchedulerError::UnexpectedReplayChoice { .. })
    ));
}

#[test]
fn deterministic_entropy_is_reproducible_and_secret_free_in_metadata() {
    let profile = profile();
    let first =
        draw_deterministic_entropy(&profile, ACTIVE_GENERATION, &open_simulation_stream(), EntropyRequest::Bytes {
            count: ENTROPY_BYTES,
        })
        .expect("first draw");
    let second =
        draw_deterministic_entropy(&profile, ACTIVE_GENERATION, &open_simulation_stream(), EntropyRequest::Bytes {
            count: ENTROPY_BYTES,
        })
        .expect("second draw");
    assert_eq!(first.value, second.value);
    let metadata = entropy_evidence_metadata(&open_simulation_stream(), &first);
    assert_eq!(metadata.request_bytes, ENTROPY_BYTES);
    assert_eq!(metadata.end_position_bytes, ENTROPY_BYTES);
}

#[test]
fn deterministic_entropy_stream_is_chunk_invariant_and_exhaustion_fails() {
    let profile = profile();
    let whole =
        draw_deterministic_entropy(&profile, ACTIVE_GENERATION, &open_simulation_stream(), EntropyRequest::Bytes {
            count: ENTROPY_BYTES,
        })
        .expect("whole draw");
    let half_count = ENTROPY_BYTES / HALF_DIVISOR;
    let first =
        draw_deterministic_entropy(&profile, ACTIVE_GENERATION, &open_simulation_stream(), EntropyRequest::Bytes {
            count: half_count,
        })
        .expect("first half");
    let second = draw_deterministic_entropy(&profile, ACTIVE_GENERATION, &first.next, EntropyRequest::Bytes {
        count: half_count,
    })
    .expect("second half");
    let mut joined = match first.value {
        EntropyValue::Bytes(bytes) => bytes,
        EntropyValue::Choice(_) => panic!("expected bytes"),
    };
    match second.value {
        EntropyValue::Bytes(bytes) => joined.extend(bytes),
        EntropyValue::Choice(_) => panic!("expected bytes"),
    }
    assert_eq!(whole.value, EntropyValue::Bytes(joined));

    let mut bounded_descriptor = descriptor(TimeProfileKind::DeterministicSimulation);
    bounded_descriptor.max_entropy_request_bytes = ENTROPY_BYTES;
    bounded_descriptor.max_entropy_total_bytes = ENTROPY_BYTES;
    let bounded = admit_time_profile(&bounded_descriptor).expect("bounded profile");
    let stream = open_entropy_stream(&bounded, ACTIVE_GENERATION, &EntropyStreamRequest {
        profile_ref: bounded.profile_ref.clone(),
        stream_id: "bounded-stream".to_string(),
        purpose: "bounded-purpose".to_string(),
        capability_ref: reference('b'),
        generation: ACTIVE_GENERATION,
        mode: EntropyMode::DeterministicSimulation,
        explicit_simulation_seed: Some(SIMULATION_SEED),
        explicit_simulation_seed_ref: Some(reference('c')),
    })
    .expect("bounded stream");
    let exhausted = draw_deterministic_entropy(&bounded, ACTIVE_GENERATION, &stream, EntropyRequest::Bytes {
        count: ENTROPY_BYTES,
    })
    .expect("fill entropy budget");
    assert!(matches!(
        draw_deterministic_entropy(&bounded, ACTIVE_GENERATION, &exhausted.next, EntropyRequest::Bytes { count: 1 },),
        Err(EntropyError::TotalLimitExceeded { .. })
    ));
}

#[test]
fn entropy_choices_are_bounded_and_production_bytes_are_exact() {
    let profile = profile();
    let choice = draw_deterministic_entropy(
        &profile,
        ACTIVE_GENERATION,
        &open_simulation_stream(),
        EntropyRequest::BoundedChoice {
            upper_exclusive: CHOICE_UPPER,
        },
    )
    .expect("choice");
    assert!(matches!(choice.value, EntropyValue::Choice(value) if value < CHOICE_UPPER));

    let live_profile = admit_time_profile(&descriptor(TimeProfileKind::Live)).expect("live profile");
    let production = open_entropy_stream(&live_profile, ACTIVE_GENERATION, &EntropyStreamRequest {
        profile_ref: live_profile.profile_ref.clone(),
        stream_id: "production".to_string(),
        purpose: "token".to_string(),
        capability_ref: reference('b'),
        generation: ACTIVE_GENERATION,
        mode: EntropyMode::ProductionCryptographic,
        explicit_simulation_seed: None,
        explicit_simulation_seed_ref: None,
    })
    .expect("production stream");
    assert!(matches!(
        consume_production_entropy(
            &live_profile,
            ACTIVE_GENERATION,
            &production,
            EntropyRequest::Bytes { count: ENTROPY_BYTES },
            vec![0; ENTROPY_BYTES as usize - 1],
        ),
        Err(EntropyError::SuppliedByteCountMismatch { .. })
    ));
}

#[test]
fn entropy_rejects_stale_generation_empty_requests_and_wrong_mode() {
    let profile = profile();
    let stream = open_simulation_stream();
    assert_eq!(
        open_entropy_stream(&profile, ACTIVE_GENERATION, &EntropyStreamRequest {
            profile_ref: profile.profile_ref.clone(),
            stream_id: "missing-seed-ref".to_string(),
            purpose: "missing-seed-ref".to_string(),
            capability_ref: reference('b'),
            generation: ACTIVE_GENERATION,
            mode: EntropyMode::DeterministicSimulation,
            explicit_simulation_seed: Some(SIMULATION_SEED),
            explicit_simulation_seed_ref: None,
        },),
        Err(EntropyError::MissingSimulationSeedRef)
    );
    assert!(matches!(
        open_entropy_stream(&profile, ACTIVE_GENERATION, &EntropyStreamRequest {
            profile_ref: profile.profile_ref.clone(),
            stream_id: "wrong-mode".to_string(),
            purpose: "wrong-mode".to_string(),
            capability_ref: reference('b'),
            generation: ACTIVE_GENERATION,
            mode: EntropyMode::ProductionCryptographic,
            explicit_simulation_seed: None,
            explicit_simulation_seed_ref: None,
        },),
        Err(EntropyError::WrongMode { .. })
    ));
    assert!(matches!(
        draw_deterministic_entropy(&profile, STALE_GENERATION, &stream, EntropyRequest::Bytes { count: 1 },),
        Err(EntropyError::StaleGeneration { .. })
    ));
    assert_eq!(
        draw_deterministic_entropy(&profile, ACTIVE_GENERATION, &stream, EntropyRequest::Bytes { count: 0 },),
        Err(EntropyError::EmptyRequest)
    );
    assert!(matches!(
        consume_production_entropy(&profile, ACTIVE_GENERATION, &stream, EntropyRequest::Bytes { count: 1 }, vec![0],),
        Err(EntropyError::WrongMode { .. })
    ));
}

#[test]
fn deadlines_respect_uncertainty_and_reject_wall_clock_domain() {
    let profile = profile();
    let deadline = Deadline {
        profile_ref: profile.profile_ref.clone(),
        subject_id: "deadline.test".to_string(),
        generation: ACTIVE_GENERATION,
        target: virtual_time(DEADLINE_TICKS),
        uncertainty_ticks: 1,
    };
    let decision = evaluate_deadline(&profile, ACTIVE_GENERATION, &deadline, &virtual_time(DEADLINE_TICKS))
        .expect("uncertain deadline");
    assert_eq!(decision.status, DeadlineStatus::IndeterminateWithinUncertainty);

    let wall_deadline = Deadline {
        target: TimeValue::Wall(WallClockObservation {
            profile_ref: profile.profile_ref.clone(),
            unix_nanos: DEADLINE_TICKS,
            uncertainty_nanos: 0,
            observation_sequence: 1,
        }),
        ..deadline
    };
    assert!(matches!(
        evaluate_deadline(&profile, ACTIVE_GENERATION, &wall_deadline, &wall_deadline.target,),
        Err(DeadlineLeaseError::UnsupportedDeadlineDomain(TimeDomain::WallClock))
    ));
}

#[test]
fn retry_plans_are_bounded_and_jitter_explicit() {
    let profile = profile();
    let policy = RetryPolicy {
        maximum_attempts: RETRY_ATTEMPTS,
        base_delay_ticks: RETRY_BASE,
        maximum_delay_ticks: RETRY_MAX,
        backoff: RetryBackoff::Exponential,
        jitter: RetryJitter::Bounded {
            maximum_ticks: RETRY_JITTER,
        },
    };
    let plan = plan_retry(
        &profile,
        ACTIVE_GENERATION,
        "retry.test",
        ACTIVE_GENERATION,
        &virtual_time(0),
        RETRY_ATTEMPT,
        policy,
        Some(RETRY_JITTER),
    )
    .expect("retry plan");
    assert_eq!(plan.delay.ticks, EXPECTED_RETRY_DELAY);
    assert!(matches!(
        plan_retry(
            &profile,
            ACTIVE_GENERATION,
            "retry.test",
            ACTIVE_GENERATION,
            &virtual_time(0),
            RETRY_ATTEMPTS,
            policy,
            Some(RETRY_JITTER),
        ),
        Err(DeadlineLeaseError::RetryExhausted { .. })
    ));
}

#[test]
fn exclusive_lease_actions_require_fresh_fencing() {
    let profile = profile();
    let mut request = LeaseRequest {
        lease_id: "lease.test".to_string(),
        owner_id: "owner.test".to_string(),
        generation: ACTIVE_GENERATION,
        now: virtual_time(DEADLINE_TICKS),
        expires_at: virtual_time(LEASE_EXPIRY),
        uncertainty_ticks: 0,
        consistency: LeaseConsistency::LocalObservationOnly,
        action: LeaseAction::AcquireExclusive,
        fencing_token: None,
        previous_fencing_token: None,
    };
    let denied = evaluate_lease(&profile, ACTIVE_GENERATION, &request).expect("local deny");
    assert_eq!(denied.kind, LeaseDecisionKind::DeniedWithoutFencing);

    request.consistency = LeaseConsistency::FencedExclusive;
    request.fencing_token = Some(FENCING_TOKEN);
    request.previous_fencing_token = Some(PREVIOUS_FENCING_TOKEN);
    let allowed = evaluate_lease(&profile, ACTIVE_GENERATION, &request).expect("fenced allow");
    assert_eq!(allowed.kind, LeaseDecisionKind::ExclusiveActionAllowed);

    request.previous_fencing_token = Some(FENCING_TOKEN);
    let stale = evaluate_lease(&profile, ACTIVE_GENERATION, &request).expect("stale deny");
    assert_eq!(stale.kind, LeaseDecisionKind::DeniedStaleFencingToken);

    request.now = virtual_time(LEASE_EXPIRY);
    request.fencing_token = Some(FENCING_TOKEN + 1);
    let expired = evaluate_lease(&profile, ACTIVE_GENERATION, &request).expect("expired deny");
    assert_eq!(expired.kind, LeaseDecisionKind::DeniedExpired);
}


#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct ScenarioCounters {
    timer_events: u64,
    scheduler_events: u64,
    entropy_events: u64,
    deadline_lease_events: u64,
    fault_events: u64,
}

fn run_simulation_scenarios(
    profile: &CanonicalTimeProfile,
    clock: &mut VirtualClockAdapter,
    events: &mut impl crate::bounded::VecSink<CanonicalTimeEvent>,
) -> crate::error::Result<ScenarioCounters> {
    let mut counters = ScenarioCounters::default();
    let periodic = schedule_timer(
        &profile.profile,
        FIXTURE_GENERATION,
        0,
        &timer_request(
            &profile.profile,
            PERIODIC_TIMER_SEQUENCE,
            TimerKind::Periodic {
                period_ticks: TIMER_PERIOD,
            },
            TimerOverloadPolicy::RejectAndRetain,
        ),
    )
    .map_err(|error| core_error("schedule periodic fixture timer", error))?;
    clock.await_ticks(TIMER_OBSERVATION)?;
    let periodic_transition = poll_timer(&periodic, FIXTURE_GENERATION, clock.now_ticks()?, 1)
        .map_err(|error| core_error("poll periodic fixture timer", error))?;
    events.push_item(canonical_timer_event(&profile.profile_ref, &periodic_transition)?);
    counters.timer_events = checked_increment(counters.timer_events, "timer event count")?;

    let delayed = schedule_timer(
        &profile.profile,
        FIXTURE_GENERATION,
        ACTIVE_TIMER_SLOTS_AFTER_PERIODIC,
        &timer_request(
            &profile.profile,
            DELAYED_TIMER_SEQUENCE,
            TimerKind::OneShot,
            TimerOverloadPolicy::RejectAndRetain,
        ),
    )
    .map_err(|error| core_error("schedule delayed fixture timer", error))?;
    let delay_fault = FabricTimeFault::DelayTimer {
        key: delayed.key.clone(),
        ticks: TIMER_DELAY_FAULT,
    };
    let delayed_transition =
        poll_timer_with_fault(&delayed, FIXTURE_GENERATION, delayed.next_deadline_ticks, 1, Some(&delay_fault))?;
    events.push_item(canonical_timer_event(&profile.profile_ref, &delayed_transition)?);
    events.push_item(canonical_named_event(EventHeader {
        profile_ref: &profile.profile_ref,
        kind: CanonicalTimeEventKind::Fault,
        generation: FIXTURE_GENERATION,
        subject: "timer-delay",
        action: "injected",
        ticks: TIMER_DELAY_FAULT,
    })?);
    counters.timer_events = checked_increment(counters.timer_events, "timer event count")?;
    counters.fault_events = checked_increment(counters.fault_events, "fault event count")?;

    run_timer_drop_and_cancel_faults(profile, events, &mut counters)?;

    let cleaned = cleanup_generation(&[periodic_transition.next, delayed_transition.next], FIXTURE_GENERATION);
    if cleaned.iter().any(|timer| timer.phase != TimerPhase::Cancelled) {
        return Err(crate::error::MoltenError::invalid_harness("fixture generation cleanup leaked an active timer"));
    }
    events.push_item(canonical_named_event(EventHeader {
        profile_ref: &profile.profile_ref,
        kind: CanonicalTimeEventKind::Timer,
        generation: FIXTURE_GENERATION,
        subject: "timer-generation-cleanup",
        action: "no-leaks",
        ticks: clock.now_ticks()?,
    })?);
    counters.timer_events = checked_increment(counters.timer_events, "timer event count")?;

    run_scheduler_scenario(profile, events, &mut counters)?;
    run_deterministic_entropy_scenario(profile, events, &mut counters)?;
    run_deadline_lease_scenario(profile, events, &mut counters)?;
    run_clock_partition_faults(profile, clock, events, &mut counters)?;
    Ok(counters)
}

/// A dropped delivery is recorded as a fault, and a cancellation racing the deadline wins.
fn run_timer_drop_and_cancel_faults(
    profile: &CanonicalTimeProfile,
    events: &mut impl crate::bounded::VecSink<CanonicalTimeEvent>,
    counters: &mut ScenarioCounters,
) -> crate::error::Result<()> {
    let dropped = schedule_timer(
        &profile.profile,
        FIXTURE_GENERATION,
        ACTIVE_TIMER_SLOTS_AFTER_DELAYED,
        &timer_request(&profile.profile, DROPPED_TIMER_SEQUENCE, TimerKind::OneShot, TimerOverloadPolicy::DropDue),
    )
    .map_err(|error| core_error("schedule dropped fixture timer", error))?;
    let drop_fault = FabricTimeFault::DropTimerDelivery {
        key: dropped.key.clone(),
    };
    let dropped_transition =
        poll_timer_with_fault(&dropped, FIXTURE_GENERATION, dropped.next_deadline_ticks, 1, Some(&drop_fault))?;
    events.push_item(canonical_timer_event(&profile.profile_ref, &dropped_transition)?);
    events.push_item(canonical_named_event(EventHeader {
        profile_ref: &profile.profile_ref,
        kind: CanonicalTimeEventKind::Fault,
        generation: FIXTURE_GENERATION,
        subject: "timer-drop",
        action: "injected-and-recorded",
        ticks: dropped.next_deadline_ticks,
    })?);
    counters.timer_events = checked_increment(counters.timer_events, "timer event count")?;
    counters.fault_events = checked_increment(counters.fault_events, "fault event count")?;

    let cancellable = schedule_timer(
        &profile.profile,
        FIXTURE_GENERATION,
        ACTIVE_TIMER_SLOTS_BEFORE_CANCELLATION,
        &timer_request(
            &profile.profile,
            CANCELLED_TIMER_SEQUENCE,
            TimerKind::OneShot,
            TimerOverloadPolicy::RejectAndRetain,
        ),
    )
    .map_err(|error| core_error("schedule cancellation fault timer", error))?;
    let cancellation_fault = FabricTimeFault::CancelTimer {
        key: cancellable.key.clone(),
    };
    let cancelled = poll_timer_with_fault(
        &cancellable,
        FIXTURE_GENERATION,
        cancellable.next_deadline_ticks,
        1,
        Some(&cancellation_fault),
    )?;
    events.push_item(canonical_timer_event(&profile.profile_ref, &cancelled)?);
    events.push_item(canonical_named_event(EventHeader {
        profile_ref: &profile.profile_ref,
        kind: CanonicalTimeEventKind::Fault,
        generation: FIXTURE_GENERATION,
        subject: "timer-cancellation-race",
        action: "cancellation-won",
        ticks: cancellable.next_deadline_ticks,
    })?);
    counters.timer_events = checked_increment(counters.timer_events, "timer event count")?;
    counters.fault_events = checked_increment(counters.fault_events, "fault event count")?;
    Ok(())
}

fn run_scheduler_scenario(
    profile: &CanonicalTimeProfile,
    events: &mut impl crate::bounded::VecSink<CanonicalTimeEvent>,
    counters: &mut ScenarioCounters,
) -> crate::error::Result<()> {
    let policy = SchedulerPolicy {
        ordering: SchedulerOrdering::PriorityThenFifo,
        replay: SchedulerReplayPolicy::Deterministic,
        overload: SchedulerOverloadPolicy::Reject,
    };
    let mut state = new_scheduler_state(&profile.profile, FIXTURE_GENERATION);
    let low = runnable_key("low");
    let high = runnable_key("high");
    for (key, priority) in [(low.clone(), 0), (high.clone(), 1)] {
        let transition =
            apply_scheduler_command(&profile.profile, policy, &state, FIXTURE_GENERATION, &SchedulerCommand::Wake {
                key,
                priority,
            })
            .map_err(|error| core_error("wake fixture runnable", error))?;
        events.push_item(canonical_scheduler_transition(&profile.profile_ref, &transition)?);
        state = transition.next;
        counters.scheduler_events = checked_increment(counters.scheduler_events, "scheduler event count")?;
    }
    let selection = choose_runnable(&profile.profile, policy, &state, FIXTURE_GENERATION, Some(&high))
        .map_err(|error| core_error("select fixture runnable", error))?;
    let replay = choose_runnable(&profile.profile, policy, &state, FIXTURE_GENERATION, Some(&selection.selected))
        .map_err(|error| core_error("replay fixture selection", error))?;
    if replay.selected != selection.selected {
        return Err(crate::error::MoltenError::invalid_harness(
            "deterministic scheduler replay selected a different runnable",
        ));
    }
    if !matches!(
        choose_runnable(&profile.profile, policy, &state, FIXTURE_GENERATION, Some(&low),),
        Err(SchedulerError::UnexpectedReplayChoice { .. })
    ) {
        return Err(crate::error::MoltenError::invalid_harness("fixture scheduler accepted a divergent replay choice"));
    }
    events.push_item(canonical_named_event(EventHeader {
        profile_ref: &profile.profile_ref,
        kind: CanonicalTimeEventKind::Scheduler,
        generation: FIXTURE_GENERATION,
        subject: "scheduler-replay",
        action: "divergence-detected",
        ticks: state.choice_sequence,
    })?);
    counters.scheduler_events = checked_increment(counters.scheduler_events, "scheduler event count")?;
    events.push_item(canonical_scheduler_selection(&profile.profile_ref, &selection)?);
    counters.scheduler_events = checked_increment(counters.scheduler_events, "scheduler event count")?;

    let yielded = apply_scheduler_command(
        &profile.profile,
        policy,
        &selection.next,
        FIXTURE_GENERATION,
        &SchedulerCommand::Yield {
            key: selection.selected,
        },
    )
    .map_err(|error| core_error("yield fixture runnable", error))?;
    events.push_item(canonical_scheduler_transition(&profile.profile_ref, &yielded)?);
    counters.scheduler_events = checked_increment(counters.scheduler_events, "scheduler event count")?;

    run_scheduler_saturation(profile, policy, events, counters)
}

/// Fills the scheduler queue to its admitted depth and requires one more wake to be rejected as
/// overload.
fn run_scheduler_saturation(
    profile: &CanonicalTimeProfile,
    policy: SchedulerPolicy,
    events: &mut impl crate::bounded::VecSink<CanonicalTimeEvent>,
    counters: &mut ScenarioCounters,
) -> crate::error::Result<()> {
    let saturation_fault = FabricTimeFault::SaturateSchedulerQueue;
    let mut saturated = new_scheduler_state(&profile.profile, FIXTURE_GENERATION);
    for index in 0..profile.profile.max_scheduler_queue_depth {
        saturated = apply_scheduler_command(
            &profile.profile,
            policy,
            &saturated,
            FIXTURE_GENERATION,
            &SchedulerCommand::Wake {
                key: runnable_key(&format!("saturated-{index}")),
                priority: 0,
            },
        )
        .map_err(|error| core_error("saturate fixture scheduler", error))?
        .next;
    }
    let overload =
        apply_scheduler_command(&profile.profile, policy, &saturated, FIXTURE_GENERATION, &SchedulerCommand::Wake {
            key: runnable_key("saturated-overflow"),
            priority: 0,
        })
        .map_err(|error| core_error("probe saturated fixture scheduler", error))?;
    validate_scheduler_fault_outcome(&saturation_fault, &overload)?;
    events.push_item(canonical_scheduler_transition(&profile.profile_ref, &overload)?);
    events.push_item(canonical_named_event(EventHeader {
        profile_ref: &profile.profile_ref,
        kind: CanonicalTimeEventKind::Fault,
        generation: FIXTURE_GENERATION,
        subject: "scheduler-queue",
        action: "saturated",
        ticks: profile.profile.max_scheduler_queue_depth,
    })?);
    counters.scheduler_events = checked_increment(counters.scheduler_events, "scheduler event count")?;
    counters.fault_events = checked_increment(counters.fault_events, "fault event count")?;
    Ok(())
}

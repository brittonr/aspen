
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct ScenarioCounters {
    timer_events: u64,
    scheduler_events: u64,
    entropy_events: u64,
    deadline_lease_events: u64,
    fault_events: u64,
}

fn run_simulation_scenarios(
    profile: &Profile,
    clock: &mut VirtualClockAdapter,
    events: &mut impl crate::bounded::VecSink<Event>,
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
    events.push_item(timer_event(&profile.profile_ref, &periodic_transition)?);
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
    let delay_fault = Fault::DelayTimer {
        key: delayed.key.clone(),
        ticks: TIMER_DELAY_FAULT,
    };
    let delayed_transition =
        poll_timer_with_fault(&delayed, FIXTURE_GENERATION, delayed.next_deadline_ticks, 1, Some(&delay_fault))?;
    events.push_item(timer_event(&profile.profile_ref, &delayed_transition)?);
    events.push_item(named_event(EventHeader {
        profile_ref: &profile.profile_ref,
        kind: EventKind::Fault,
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
        return Err(crate::error::Failure::invalid_harness("fixture generation cleanup leaked an active timer"));
    }
    events.push_item(named_event(EventHeader {
        profile_ref: &profile.profile_ref,
        kind: EventKind::Timer,
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
    profile: &Profile,
    events: &mut impl crate::bounded::VecSink<Event>,
    counters: &mut ScenarioCounters,
) -> crate::error::Result<()> {
    let dropped = schedule_timer(
        &profile.profile,
        FIXTURE_GENERATION,
        ACTIVE_TIMER_SLOTS_AFTER_DELAYED,
        &timer_request(&profile.profile, DROPPED_TIMER_SEQUENCE, TimerKind::OneShot, TimerOverloadPolicy::DropDue),
    )
    .map_err(|error| core_error("schedule dropped fixture timer", error))?;
    let drop_fault = Fault::DropTimerDelivery {
        key: dropped.key.clone(),
    };
    let dropped_transition =
        poll_timer_with_fault(&dropped, FIXTURE_GENERATION, dropped.next_deadline_ticks, 1, Some(&drop_fault))?;
    events.push_item(timer_event(&profile.profile_ref, &dropped_transition)?);
    events.push_item(named_event(EventHeader {
        profile_ref: &profile.profile_ref,
        kind: EventKind::Fault,
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
    let cancellation_fault = Fault::CancelTimer {
        key: cancellable.key.clone(),
    };
    let cancelled = poll_timer_with_fault(
        &cancellable,
        FIXTURE_GENERATION,
        cancellable.next_deadline_ticks,
        1,
        Some(&cancellation_fault),
    )?;
    events.push_item(timer_event(&profile.profile_ref, &cancelled)?);
    events.push_item(named_event(EventHeader {
        profile_ref: &profile.profile_ref,
        kind: EventKind::Fault,
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
    profile: &Profile,
    events: &mut impl crate::bounded::VecSink<Event>,
    counters: &mut ScenarioCounters,
) -> crate::error::Result<()> {
    let mut owner = scheduler_owner(profile)?;
    let low = runnable_key("low", owner.next_occurrence_sequence()?);
    let first = scheduler_command(&mut owner, SchedulerCommand::Wake { key: low.clone(), priority: 0 })?;
    events.push_item(scheduler_transition(&profile.profile_ref, &first)?);
    counters.scheduler_events = checked_increment(counters.scheduler_events, "scheduler event count")?;
    let high = runnable_key("high", owner.next_occurrence_sequence()?);
    let second = scheduler_command(&mut owner, SchedulerCommand::Wake { key: high.clone(), priority: 1 })?;
    events.push_item(scheduler_transition(&profile.profile_ref, &second)?);
    counters.scheduler_events = checked_increment(counters.scheduler_events, "scheduler event count")?;
    let before_choice_sequence = owner.state().choice_sequence;
    if !matches!(owner.choose(Some(&low)), Err(SchedulerRuntimeError::Core(SchedulerError::UnexpectedReplayChoice(_)))) {
        return Err(crate::error::Failure::invalid_harness("fixture scheduler accepted a divergent replay choice"));
    }
    let choice = owner.choose(Some(&high))?;
    let selection = SchedulerSelection { next: owner.state().clone(), selected: high, choice_sequence: choice.choice_sequence, eligible_count: choice.eligible_count };
    events.push_item(named_event(EventHeader {
        profile_ref: &profile.profile_ref,
        kind: EventKind::Scheduler,
        generation: FIXTURE_GENERATION,
        subject: "scheduler-replay",
        action: "divergence-detected",
        ticks: before_choice_sequence,
    })?);
    counters.scheduler_events = checked_increment(counters.scheduler_events, "scheduler event count")?;
    events.push_item(scheduler_selection(&profile.profile_ref, &selection)?);
    counters.scheduler_events = checked_increment(counters.scheduler_events, "scheduler event count")?;

    let yielded = scheduler_command(&mut owner, SchedulerCommand::Yield { key: selection.selected })?;
    events.push_item(scheduler_transition(&profile.profile_ref, &yielded)?);
    counters.scheduler_events = checked_increment(counters.scheduler_events, "scheduler event count")?;

    run_scheduler_saturation(profile, events, counters)
}



// The executable fixture uses the owning scheduler for every transition. This is an
// observation of the shared law, not a fixture-local retention implementation.
// r[impl molten.audit_f11.retention]
// r[impl molten.audit_f11.freshness]
// r[impl molten.audit_f11.validation]
fn scheduler_retention_scenario<P: SchedulerWakePort>(
    profile: &Profile,
    owner: &mut SchedulerRuntime<P>,
    mut wake: impl FnMut(&mut SchedulerRuntime<P>, RunnableKey) -> crate::error::Result<SchedulerTransition>,
) -> crate::error::Result<(RunnableKey, RunnableKey, Vec<Event>)> {
    let repetitions = profile
        .profile
        .max_runnables
        .checked_add(RETENTION_EVICTION_MARGIN)
        .ok_or_else(|| core_error("retention repetition bound", SchedulerError::Overflow))?;
    const EVENTS_PER_OCCURRENCE: u64 = 3;
    const FINAL_EVENTS: u64 = 2;
    let event_capacity = repetitions
        .checked_mul(EVENTS_PER_OCCURRENCE)
        .and_then(|count| count.checked_add(FINAL_EVENTS))
        .and_then(|count| usize::try_from(count).ok())
        .ok_or_else(|| core_error("retention event bound", SchedulerError::Overflow))?;
    let mut events = Vec::with_capacity(event_capacity);
    let stale = runnable_key("retained-logical-name", owner.next_occurrence_sequence()?);
    for repetition in 0..repetitions {
        let key = runnable_key("retained-logical-name", owner.next_occurrence_sequence()?);
        let transition = wake(owner, key.clone())?;
        if transition.action != SchedulerAction::Woken {
            return Err(crate::error::Failure::invalid_harness("fresh retained-name occurrence was not admitted"));
        }
        events.push(scheduler_transition(&profile.profile_ref, &transition)?);
        let choice = owner.choose(Some(&key))?;
        events.push(scheduler_selection(&profile.profile_ref, &SchedulerSelection {
            next: owner.state().clone(),
            selected: key.clone(),
            choice_sequence: choice.choice_sequence,
            eligible_count: choice.eligible_count,
        })?);
        match repetition % RETENTION_TERMINAL_MODES {
            0 => {
                let action = owner.command(SchedulerCommand::Complete { key: key.clone() })?;
                events.push(scheduler_transition(&profile.profile_ref, &scheduler_snapshot(owner, key, action))?);
            }
            1 => {
                let action = owner.command(SchedulerCommand::Cancel { key: key.clone() })?;
                events.push(scheduler_transition(&profile.profile_ref, &scheduler_snapshot(owner, key, action))?);
            }
            _ => {
                owner.cleanup_generation(FIXTURE_GENERATION)?;
                events.push(scheduler_cleanup(&profile.profile_ref, owner.state(), FIXTURE_GENERATION)?);
            }
        }
        let retained = owner.state().runnables.iter().flatten().count();
        if u64::try_from(retained).ok().is_none_or(|count| count > profile.profile.max_runnables)
            || owner.capacity_state().runnable_usage != 0
            || owner.capacity_state().queue_usage != 0
        {
            return Err(crate::error::Failure::invalid_harness("terminal retention exceeded its admitted bound"));
        }
    }
    if owner.state().runnables.iter().flatten().any(|row| row.key == stale) {
        return Err(crate::error::Failure::invalid_harness("oldest terminal occurrence was not reclaimed"));
    }
    let fresh = runnable_key("retained-logical-name", owner.next_occurrence_sequence()?);
    let transition = wake(owner, fresh.clone())?;
    if transition.action != SchedulerAction::Woken {
        return Err(crate::error::Failure::invalid_harness("fresh occurrence after reclamation was denied"));
    }
    events.push(scheduler_transition(&profile.profile_ref, &transition)?);
    reject_evicted_callbacks(owner, &stale)?;
    Ok((stale, fresh, events))
}

// r[impl molten.audit_f11.atomicity]
fn reject_evicted_callbacks<P: SchedulerWakePort>(
    owner: &mut SchedulerRuntime<P>,
    stale: &RunnableKey,
) -> crate::error::Result<()> {
    let before = owner.state().clone();
    let usage = owner.capacity_state().clone();
    let rejected = owner.command(SchedulerCommand::Wake {
        key: stale.clone(),
        priority: 0,
    });
    if !matches!(rejected, Err(SchedulerRuntimeError::Core(SchedulerError::StaleOccurrence { .. }))) {
        return Err(crate::error::Failure::invalid_harness(
            "evicted wake callback did not reject its stale occurrence",
        ));
    }
    for command in [
        SchedulerCommand::Yield { key: stale.clone() },
        SchedulerCommand::Block { key: stale.clone() },
        SchedulerCommand::Complete { key: stale.clone() },
        SchedulerCommand::Cancel { key: stale.clone() },
    ] {
        if !matches!(owner.command(command), Err(SchedulerRuntimeError::Core(SchedulerError::UnknownRunnable(_)))) {
            return Err(crate::error::Failure::invalid_harness("evicted callback targeted fresh work"));
        }
    }
    if owner.state() != &before || owner.capacity_state() != &usage {
        return Err(crate::error::Failure::invalid_harness(
            "stale callbacks changed committed scheduler state or accounting",
        ));
    }
    Ok(())
}

pub(super) fn scheduler_retention_events(profile: &Profile) -> crate::error::Result<Vec<Event>> {
    let mut owner = scheduler_owner(profile)?;
    let mut first_token = None;
    let mut current_token = None;
    let (_, _, mut events) = scheduler_retention_scenario(profile, &mut owner, |owner, key| {
        let action = owner.command(SchedulerCommand::Wake {
            key: key.clone(),
            priority: 0,
        })?;
        let token = owner
            .effects_mut()
            .drain_next()
            .ok_or_else(|| crate::error::Failure::invalid_harness("retention consumer received no wake"))?;
        if owner.resolve_wake(token).is_none_or(|row| row.key != key) {
            return Err(crate::error::Failure::invalid_harness("retention consumer received a stale wake token"));
        }
        first_token.get_or_insert(token);
        current_token = Some(token);
        Ok(scheduler_snapshot(owner, key, action))
    })?;
    let token = first_token.ok_or_else(|| crate::error::Failure::invalid_harness("missing initial retention wake"))?;
    let current =
        current_token.ok_or_else(|| crate::error::Failure::invalid_harness("missing current retention wake"))?;
    if current.occurrence != token.occurrence
        || owner.resolve_wake(current).is_none()
        || owner.resolve_wake(token).is_some()
        || owner.effects_mut().drain_next().is_some()
    {
        return Err(crate::error::Failure::invalid_harness(
            "reclaimed token resolved or stale callback delivered a wake",
        ));
    }
    owner.cleanup_generation(FIXTURE_GENERATION)?;
    events.push(scheduler_cleanup(&profile.profile_ref, owner.state(), FIXTURE_GENERATION)?);
    Ok(events)
}

fn live_scheduler_retention_events(profile: &Profile) -> crate::error::Result<Vec<Event>> {
    let context =
        ExtensionTimeContext::for_application(FIXTURE_SERVICE_ID.to_string(), FIXTURE_GENERATION, &profile.profile);
    let mut owner =
        SchedulerRuntime::activate(context, profile.profile.clone(), ThreadSchedulerWakeAdapter::default())?;
    let (stale, fresh, mut events) = scheduler_retention_scenario(profile, &mut owner, live_scheduler_wake)?;
    const STALE_CALLBACK_CHANNEL_CAPACITY: usize = 1;
    let (sender, receiver) = std::sync::mpsc::sync_channel(STALE_CALLBACK_CHANNEL_CAPACITY);
    let parked = std::thread::spawn(move || {
        std::thread::park();
        sender.send(())
    });
    let registration = owner.effects_mut().register(fresh.clone(), parked.thread().clone());
    let result = registration
        .map_err(SchedulerRuntimeError::Admission)
        .map_err(crate::error::Failure::from)
        .and_then(|()| reject_evicted_callbacks(&mut owner, &stale));
    let observed = receiver.recv_timeout(std::time::Duration::from_millis(STALE_CALLBACK_OBSERVATION_MILLIS));
    parked.thread().unpark();
    parked
        .join()
        .map_err(|_| crate::error::Failure::invalid_harness("stale-callback receiver panicked"))?
        .map_err(|_| crate::error::Failure::invalid_harness("stale-callback receiver disconnected"))?;
    owner.effects_mut().unregister(&fresh);
    result?;
    if observed != Err(std::sync::mpsc::RecvTimeoutError::Timeout) {
        return Err(crate::error::Failure::invalid_harness("stale occurrence unparked the fresh receiver"));
    }
    owner.cleanup_generation(FIXTURE_GENERATION)?;
    events.push(scheduler_cleanup(&profile.profile_ref, owner.state(), FIXTURE_GENERATION)?);
    Ok(events)
}

#[cfg(test)]
pub(super) fn replay_scheduler_retention(profile: &Profile, recorded: &[Event]) -> crate::error::Result<()> {
    if scheduler_retention_events(profile)? != recorded {
        return Err(crate::error::Failure::invalid_harness("scheduler retention replay diverged"));
    }
    Ok(())
}

#[cfg(test)]
// r[verify molten.audit_f11.atomicity]
// r[verify molten.audit_f11.validation]
#[test]
fn live_retention_denials_release_targets_and_preserve_state() {
    const ACTIVE_LIMIT: u64 = 1;
    for overload in [SchedulerOverloadPolicy::Reject, SchedulerOverloadPolicy::Backpressure] {
        let mut requested = profile("retention-denial", HASH_A, TimeProfileKind::Live, None);
        requested.max_runnables = ACTIVE_LIMIT;
        requested.max_scheduler_queue_depth = ACTIVE_LIMIT;
        requested.max_scheduler_concurrency = ACTIVE_LIMIT;
        requested.scheduler_policy.overload = overload;
        let profile = admit_profile(&requested).expect("bounded live profile");
        let context =
            ExtensionTimeContext::for_application(FIXTURE_SERVICE_ID.to_string(), FIXTURE_GENERATION, &profile.profile);
        let mut owner =
            SchedulerRuntime::activate(context, profile.profile.clone(), ThreadSchedulerWakeAdapter::default())
                .expect("activate real live owner");
        let active = runnable_key("active", 1);
        assert_eq!(live_scheduler_wake(&mut owner, active).expect("actual live wake").action, SchedulerAction::Woken);
        let before = (owner.state().clone(), owner.capacity_state().clone());
        let expected = match overload {
            SchedulerOverloadPolicy::Reject => SchedulerAction::RejectedOverload,
            SchedulerOverloadPolicy::Backpressure => SchedulerAction::Backpressure,
        };
        let stale = RunnableKey {
            generation: FIXTURE_GENERATION + 1,
            ..runnable_key("stale", 3)
        };
        for (key, action) in [
            (runnable_key("over-capacity", 2), expected),
            (stale, SchedulerAction::DiscardedStaleGeneration),
        ] {
            assert_eq!(live_scheduler_wake(&mut owner, key.clone()).expect("denial finishes").action, action);
            assert_eq!((owner.state().clone(), owner.capacity_state().clone()), before);
            assert!(owner.effects_mut().prepare(&key, 0).is_err(), "owned target was unregistered");
        }
        let invalid = runnable_key("zero-issuance", 0);
        assert!(live_scheduler_wake(&mut owner, invalid.clone()).is_err());
        assert!(owner.effects_mut().prepare(&invalid, 0).is_err(), "failed command left no target");
        assert_eq!((owner.state().clone(), owner.capacity_state().clone()), before);
    }
}

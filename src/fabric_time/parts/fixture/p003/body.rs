
fn validate_ports(
    live: &Profile,
    simulation: &Profile,
) -> crate::error::Result<Vec<String>> {
    let mut refs = Vec::new();
    for profile in [live, simulation] {
        let descriptors = port_descriptors(profile);
        crate::fabric::build_fabric_port_registry(&descriptors)
            .map_err(|issues| core_error("validate fixture time ports", issues))?;
        refs.reserve(descriptors.len());
        for descriptor in &descriptors {
            let (descriptor_ref, _) = crate::fabric::canonical_port_descriptor(descriptor)?;
            refs.push(descriptor_ref);
        }
    }
    Ok(refs)
}

fn ensure_shared_conformance(
    live: &AdapterConformanceObservation,
    simulation: &AdapterConformanceObservation,
) -> crate::error::Result<()> {
    if live.timer_action != simulation.timer_action
        || live.delivery_count != simulation.delivery_count
        || live.stale_generation_discarded != simulation.stale_generation_discarded
        || live.cancellation_prevented_delivery != simulation.cancellation_prevented_delivery
        || live.scheduler_selected != simulation.scheduler_selected
        || live.scheduler_cancellation_recorded != simulation.scheduler_cancellation_recorded
        || live.entropy_bound_rejected != simulation.entropy_bound_rejected
    {
        return Err(crate::error::Failure::invalid_harness(format!(
            "live and simulation adapters diverged: live={live:?} simulation={simulation:?}"
        )));
    }
    Ok(())
}

fn timer_request(
    profile: &AdmittedTimeProfile,
    sequence: u64,
    kind: TimerKind,
    overload: TimerOverloadPolicy,
) -> TimerScheduleRequest {
    TimerScheduleRequest {
        profile_ref: profile.profile_ref.clone(),
        key: TimerKey {
            service_id: FIXTURE_SERVICE_ID.to_string(),
            generation: FIXTURE_GENERATION,
            sequence,
        },
        domain: TimeDomain::Virtual,
        deadline_ticks: TIMER_DEADLINE,
        kind,
        ordering_key: sequence,
        coalescing: TimerCoalescingPolicy::CoalesceLatest,
        lateness: TimerLatenessPolicy::DeliverRegardless,
        overload,
        resource_charge: TimerResourceCharge::single_slot(),
    }
}

fn runnable_key(runnable_id: &str) -> RunnableKey {
    RunnableKey {
        service_id: FIXTURE_SERVICE_ID.to_string(),
        generation: FIXTURE_GENERATION,
        runnable_id: runnable_id.to_string(),
    }
}

fn entropy_stream_request(
    profile: &AdmittedTimeProfile,
    stream_id: &str,
    purpose: &str,
    mode: EntropyMode,
    explicit_simulation_seed: Option<u64>,
) -> EntropyStreamRequest {
    EntropyStreamRequest {
        profile_ref: profile.profile_ref.clone(),
        stream_id: stream_id.to_string(),
        purpose: purpose.to_string(),
        capability_ref: HASH_C.to_string(),
        generation: FIXTURE_GENERATION,
        mode,
        explicit_simulation_seed,
        explicit_simulation_seed_ref: explicit_simulation_seed.map(|_| HASH_B.to_string()),
    }
}

fn virtual_value(profile: &AdmittedTimeProfile, ticks: u64) -> TimeValue {
    TimeValue::Virtual(VirtualInstant {
        profile_ref: profile.profile_ref.clone(),
        ticks,
    })
}

fn select_boundary_ref(
    selection: Selection,
    live_ref: &str,
    simulation_ref: &str,
    trace_kind: &str,
) -> crate::error::Result<String> {
    match selection {
        Selection::Live => Ok(live_ref.to_string()),
        Selection::DeterministicSimulation => Ok(simulation_ref.to_string()),
        Selection::Both => {
            trace_ref(trace_kind, &[live_ref.to_string(), simulation_ref.to_string()])
        }
    }
}

fn trace_for_kinds(
    events: &[&Event],
    kinds: &[EventKind],
    trace_kind: &str,
) -> crate::error::Result<String> {
    let refs = events
        .iter()
        .filter(|event| kinds.contains(&event.kind))
        .map(|event| event.evidence_ref.clone())
        .collect::<Vec<_>>();
    trace_ref(trace_kind, &refs)
}

fn count_events(events: &[&Event], kinds: &[EventKind]) -> crate::error::Result<u64> {
    u64::try_from(events.iter().filter(|event| kinds.contains(&event.kind)).count())
        .map_err(|_| crate::error::Failure::invalid_harness("fabric-time event count overflow"))
}

fn checked_increment(value: u64, label: &str) -> crate::error::Result<u64> {
    value
        .checked_add(1)
        .ok_or_else(|| crate::error::Failure::invalid_harness(format!("{label} overflow")))
}

fn core_error(label: &str, error: impl std::fmt::Debug) -> crate::error::Failure {
    crate::error::Failure::invalid_harness(format!("{label}: {error:?}"))
}

fn scheduler_owner(profile: &Profile) -> crate::error::Result<SchedulerRuntime<SimulationSchedulerWakeAdapter>> {
    let context = ExtensionTimeContext::for_application(
        FIXTURE_SERVICE_ID.to_string(),
        FIXTURE_GENERATION,
        &profile.profile,
    );
    Ok(SchedulerRuntime::activate(
        context,
        profile.profile.clone(),
        SimulationSchedulerWakeAdapter::activate(&profile.profile)?,
    )?)
}

fn scheduler_command(
    owner: &mut SchedulerRuntime<SimulationSchedulerWakeAdapter>,
    command: SchedulerCommand,
) -> crate::error::Result<SchedulerTransition> {
    let key = match &command {
        SchedulerCommand::Wake { key, .. }
        | SchedulerCommand::Yield { key }
        | SchedulerCommand::Block { key }
        | SchedulerCommand::Complete { key }
        | SchedulerCommand::Cancel { key } => key.clone(),
    };
    let action = owner.command(command)?;
    let delivered = owner.effects_mut().drain_next();
    if matches!(action, SchedulerAction::Woken | SchedulerAction::Yielded) {
        let index = delivered
            .ok_or_else(|| crate::error::Failure::invalid_harness("simulation consumer received no admitted wake"))?;
        let index = usize::try_from(index)
            .map_err(|_| crate::error::Failure::invalid_harness("simulation consumer index overflow"))?;
        if owner.state().runnables[index].key != key
            || owner.state().runnables[index].phase != RunnablePhase::Ready
            || owner.capacity_state().queue_usage == 0
        {
            return Err(crate::error::Failure::invalid_harness(
                "simulation consumer observed an uncommitted scheduler wake",
            ));
        }
    } else if delivered.is_some() {
        return Err(crate::error::Failure::invalid_harness("simulation consumer received a denied wake"));
    }
    Ok(SchedulerTransition {
        next: owner.state().clone(),
        action,
        runnable: key,
    })
}

/// Fills the scheduler queue to its admitted depth and requires one more wake to be rejected as
/// overload.
fn run_scheduler_saturation(
    profile: &Profile,
    events: &mut impl crate::bounded::VecSink<Event>,
    counters: &mut ScenarioCounters,
) -> crate::error::Result<()> {
    let saturation_fault = Fault::SaturateSchedulerQueue;
    let mut owner = scheduler_owner(profile)?;
    for index in 0..profile.profile.max_scheduler_queue_depth {
        let action = owner.command(SchedulerCommand::Wake { key: runnable_key(&format!("saturated-{index}")), priority: 0 })?;
        let delivered = owner.effects_mut().drain_next().ok_or_else(|| crate::error::Failure::invalid_harness("saturation consumer received no admitted wake"))?;
        let delivered = usize::try_from(delivered)
            .map_err(|_| crate::error::Failure::invalid_harness("saturation consumer index overflow"))?;
        if action != SchedulerAction::Woken || owner.state().runnables[delivered].phase != RunnablePhase::Ready || owner.capacity_state().queue_usage != index + 1 {
            return Err(crate::error::Failure::invalid_harness("saturation consumer observed inconsistent committed usage"));
        }
    }
    let overload = scheduler_command(&mut owner, SchedulerCommand::Wake { key: runnable_key("saturated-overflow"), priority: 0 })?;
    validate_scheduler_fault_outcome(&saturation_fault, &overload)?;
    events.push_item(scheduler_transition(&profile.profile_ref, &overload)?);
    events.push_item(named_event(EventHeader {
        profile_ref: &profile.profile_ref,
        kind: EventKind::Fault,
        generation: FIXTURE_GENERATION,
        subject: "scheduler-queue",
        action: "saturated",
        ticks: profile.profile.max_scheduler_queue_depth,
    })?);
    counters.scheduler_events = checked_increment(counters.scheduler_events, "scheduler event count")?;
    counters.fault_events = checked_increment(counters.fault_events, "fault event count")?;
    Ok(())
}

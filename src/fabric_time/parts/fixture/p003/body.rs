
fn validate_fixture_ports(
    live: &CanonicalTimeProfile,
    simulation: &CanonicalTimeProfile,
) -> crate::error::Result<Vec<String>> {
    let mut refs = Vec::new();
    for profile in [live, simulation] {
        let descriptors = fabric_time_port_descriptors(profile);
        crate::fabric::build_fabric_port_registry(&descriptors)
            .map_err(|issues| core_error("validate fixture time ports", issues))?;
        refs.reserve(descriptors.len());
        for descriptor in &descriptors {
            let (descriptor_ref, _) = crate::fabric::canonical_fabric_port_descriptor(descriptor)?;
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
        return Err(crate::error::MoltenError::invalid_harness(format!(
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
    selection: FabricTimeFixtureSelection,
    live_ref: &str,
    simulation_ref: &str,
    trace_kind: &str,
) -> crate::error::Result<String> {
    match selection {
        FabricTimeFixtureSelection::Live => Ok(live_ref.to_string()),
        FabricTimeFixtureSelection::DeterministicSimulation => Ok(simulation_ref.to_string()),
        FabricTimeFixtureSelection::Both => {
            canonical_time_trace_ref(trace_kind, &[live_ref.to_string(), simulation_ref.to_string()])
        }
    }
}

fn trace_for_kinds(
    events: &[&CanonicalTimeEvent],
    kinds: &[CanonicalTimeEventKind],
    trace_kind: &str,
) -> crate::error::Result<String> {
    let refs = events
        .iter()
        .filter(|event| kinds.contains(&event.kind))
        .map(|event| event.evidence_ref.clone())
        .collect::<Vec<_>>();
    canonical_time_trace_ref(trace_kind, &refs)
}

fn count_events(events: &[&CanonicalTimeEvent], kinds: &[CanonicalTimeEventKind]) -> crate::error::Result<u64> {
    u64::try_from(events.iter().filter(|event| kinds.contains(&event.kind)).count())
        .map_err(|_| crate::error::MoltenError::invalid_harness("fabric-time event count overflow"))
}

fn checked_increment(value: u64, label: &str) -> crate::error::Result<u64> {
    value
        .checked_add(1)
        .ok_or_else(|| crate::error::MoltenError::invalid_harness(format!("{label} overflow")))
}

fn core_error(label: &str, error: impl std::fmt::Debug) -> crate::error::MoltenError {
    crate::error::MoltenError::invalid_harness(format!("{label}: {error:?}"))
}

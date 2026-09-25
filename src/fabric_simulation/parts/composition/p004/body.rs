
/// The earliest future tick at which storage, transport, or an unfired crash or restart fault
/// becomes ready.
fn next_readiness_tick(
    prepared: &PreparedReferenceWorld,
    router: &DeterministicSimulationPortRouter,
    scheduler: &SimulationSchedulerState,
    fired_recovery_faults: &std::collections::BTreeSet<String>,
) -> Option<u64> {
    let mut readiness = Vec::new();
    readiness.extend(router.storage().next_readiness(scheduler.virtual_tick));
    readiness.extend(router.transport().next_readiness(scheduler.virtual_tick));
    readiness.extend(
        prepared
            .world
            .admitted
            .manifest
            .faults
            .iter()
            .filter(|fault| {
                matches!(fault.kind, SimulationFaultKind::Crash | SimulationFaultKind::Restart)
                    && !fired_recovery_faults.contains(&fault.fault_id)
            })
            .map(|fault| fault.activate_at_choice),
    );
    readiness.into_iter().filter(|tick| *tick > scheduler.virtual_tick).min()
}

/// Completes the selected storage operation or delivers the selected message and returns its
/// semantic output.
fn execute_port_choice(
    router: &mut DeterministicSimulationPortRouter,
    transition: &SimulationSchedulerTransition,
) -> crate::error::Result<String> {
    let selected = &transition.record.selected;
    match selected.kind {
        SchedulerChoiceKind::StorageCompletion => {
            let operation = router
                .storage()
                .submitted()
                .iter()
                .find(|operation| storage_completion_choice_id(&operation.operation_id) == selected.choice_id)
                .cloned()
                .ok_or_else(|| crate::error::MoltenError::invalid_harness("selected storage completion disappeared"))?;
            router.complete_storage_head(&operation.operation_id).map_err(|error| {
                crate::error::MoltenError::invalid_harness(format!("reference storage completion denied: {error:?}"))
            })
        }
        SchedulerChoiceKind::MessageDelivery => {
            let transmission = router
                .transport()
                .pending()
                .iter()
                .find(|transmission| message_delivery_choice_id(&transmission.transmission_id) == selected.choice_id)
                .cloned()
                .ok_or_else(|| crate::error::MoltenError::invalid_harness("selected transport delivery disappeared"))?;
            router.deliver_transmission(&transmission.transmission_id).map_err(|error| {
                crate::error::MoltenError::invalid_harness(format!("reference transport delivery denied: {error:?}"))
            })
        }
        other => Err(crate::error::MoltenError::invalid_harness(format!(
            "reference runner cannot execute scheduler choice kind {}",
            other.as_str()
        ))),
    }
}

/// The sorted final state refs, sorted evidence refs, and durable service states of the shut-down
/// hosts.
struct FinishedReferenceHosts {
    final_state_refs: Vec<String>,
    host_evidence_refs: Vec<String>,
    service_states: std::collections::BTreeMap<String, ReferenceServiceState>,
}

/// Drains and shuts down every host, returning their sorted final state refs, sorted evidence refs,
/// and durable service states.
fn finish_reference_hosts(
    prepared: &mut PreparedReferenceWorld,
    scheduler: &SimulationSchedulerState,
) -> crate::error::Result<FinishedReferenceHosts> {
    let host_count = prepared.hosts.len();
    let mut final_state_refs = Vec::with_capacity(host_count);
    let mut host_evidence_refs = Vec::new();
    let mut service_states = std::collections::BTreeMap::new();
    for (node_id, host) in prepared.hosts.iter_mut() {
        host.drain(scheduler.virtual_tick)?;
        host.shutdown(scheduler.virtual_tick)?;
        final_state_refs.push(blake3_ref(format!("{:?}", host.executor().state()).as_bytes()));
        if service_states.len() >= host_count {
            return Err(crate::error::MoltenError::invalid_harness(
                "reference world reports at most one durable service state per host",
            ));
        }
        service_states.insert(node_id.clone(), host.executor().state().clone());
        let host_evidence = host.evidence();
        host_evidence_refs.reserve(host_evidence.len());
        host_evidence_refs.extend(host_evidence.iter().map(|item| item.evidence_ref().to_string()));
    }
    final_state_refs.sort();
    host_evidence_refs.sort();
    Ok(FinishedReferenceHosts {
        final_state_refs,
        host_evidence_refs,
        service_states,
    })
}

/// The router's resource units plus a fixed increment per recorded choice, within the admitted
/// envelope.
fn reference_resource_units(
    prepared: &PreparedReferenceWorld,
    router: &DeterministicSimulationPortRouter,
    choice_count: usize,
) -> crate::error::Result<u64> {
    let choice_resource_units = u64::try_from(choice_count)
        .map_err(|_| crate::error::MoltenError::invalid_harness("reference choice resource count overflow"))?
        .checked_mul(RUN_RESOURCE_INCREMENT)
        .ok_or_else(|| {
            crate::error::MoltenError::invalid_harness("reference choice resource multiplication overflow")
        })?;
    let resource_units = router
        .resource_units()
        .checked_add(choice_resource_units)
        .ok_or_else(|| crate::error::MoltenError::invalid_harness("reference run resource count overflow"))?;
    if resource_units > prepared.world.admitted.manifest.bounds.max_resource_units {
        return Err(crate::error::MoltenError::invalid_harness("reference run exceeded its resource envelope"));
    }
    Ok(resource_units)
}

struct WorkloadStepContext<'a> {
    prepared: &'a mut PreparedReferenceWorld,
    router: &'a mut DeterministicSimulationPortRouter,
    observations: &'a mut Vec<CanonicalSimulationObservation>,
    history_material: &'a mut String,
}

fn execute_workload_step(
    context: &mut WorkloadStepContext<'_>,
    step: &SimulationWorkloadStep,
    transition: &SimulationSchedulerTransition,
) -> crate::error::Result<String> {
    let WorkloadStepContext {
        prepared,
        router,
        observations,
        history_material,
    } = context;
    router.set_dispatching(&step.node_id, &step.request_ref);
    let host = prepared.hosts.get_mut(&step.node_id).ok_or_else(|| {
        crate::error::MoltenError::invalid_harness(format!("missing reference host {}", step.node_id))
    })?;
    let (receipt, outcome) =
        match host.dispatch_request(&step.request_ref, REFERENCE_REQUEST_BYTES, transition.next.virtual_tick)? {
            crate::system_extension::HostDispatchResult::Executed { receipt, outcome, .. } => (receipt, outcome),
            other => {
                return Err(crate::error::MoltenError::invalid_harness(format!(
                    "reference request did not execute through the system-extension host: {other:?}"
                )));
            }
        };
    let completions = host.route_approved_effects(&receipt, &mut **router)?;
    if completions.is_empty() {
        return Err(crate::error::MoltenError::invalid_harness(
            "reference request did not cross a deterministic fabric port",
        ));
    }
    let semantic_output_ref = outcome.output_refs.first().cloned().ok_or_else(|| {
        crate::error::MoltenError::invalid_harness("reference callback returned no decision output ref")
    })?;
    let reference_transition = host.executor().last_transition().ok_or_else(|| {
        crate::error::MoltenError::invalid_harness("reference executor did not retain its pure transition")
    })?;
    let state_ref = outcome
        .state_ref
        .clone()
        .ok_or_else(|| crate::error::MoltenError::invalid_harness("reference callback returned no state ref"))?;
    history_material.push_str(&state_ref);
    history_material.push_str(&transition.record.selected.choice_id);
    let history_ref = blake3_ref(history_material.as_bytes());
    let port_event_ref =
        router.events().last().map(|event| event.event_ref.clone()).ok_or_else(|| {
            crate::error::MoltenError::invalid_harness("reference route emitted no canonical port event")
        })?;
    let observation = canonical_simulation_observation(SimulationObservation {
        sequence: step.sequence,
        node_id: step.node_id.clone(),
        service: Some(step.service),
        generation: transition.record.selected.generation,
        state_ref,
        history_ref,
        port_event_ref,
        ambient_effect: false,
        stale_generation_mutation: false,
        resource_bound_bypass: false,
        port_state_machine_violation: false,
        terminal_cleanup_complete: true,
        semantic_invariants_passed: reference_transition
            .semantic_invariants
            .iter()
            .map(|invariant| (*invariant).to_string())
            .collect(),
    })?;
    observations.push(observation);
    Ok(semantic_output_ref)
}

// r[impl molten.fabric_simulation.stateful_storage]
fn rebuild_hosts_from_image(
    prepared: &mut PreparedReferenceWorld,
    image: &SimulatedDurableImage,
) -> crate::error::Result<()> {
    let node_ids = prepared.hosts.keys().cloned().collect::<Vec<_>>();
    for node_id in node_ids {
        let Some(admitted) = prepared.admissions.get(&node_id).cloned() else {
            continue;
        };
        let kind = service_kind_for_service_id(&admitted.manifest().service_id, &node_id)?;
        let mut state = initial_reference_state(kind);
        for entry in image.entries() {
            let Some((_, _, operation)) = prepared
                .operations
                .iter()
                .find(|(operation_kind, request_ref, _)| *operation_kind == kind && request_ref == &entry.request_ref)
            else {
                continue;
            };
            let transition = apply_reference_operation(&state, operation).map_err(|error| {
                crate::error::MoltenError::invalid_harness(format!("durable image replay denied: {error:?}"))
            })?;
            state = transition.next;
        }
        let executor = ReferenceServiceExecutor::recovered(
            kind,
            operations_for_kind(&prepared.operations, kind),
            &prepared.profiles,
            state,
        )?;
        let mut host = crate::system_extension::SystemExtensionHost::new(admitted, executor)?;
        host.activate(FIRST_VIRTUAL_TICK)?;
        prepared.hosts.insert(node_id, host);
    }
    Ok(())
}

fn reference_contract_differential(
    world: &CanonicalSimulatedWorld,
) -> crate::error::Result<CanonicalSimulationDifferential> {
    let simulation_profile_ref = blake3_ref(FABRIC_SIMULATION_PROFILE_ID.as_bytes());
    let live_profile_ref = blake3_ref(b"reviewed-live-port-contract-profile-v1");
    let shared_contract_ref = blake3_ref(b"fabric-port-command-event-contract-set-v1");
    let trace_refs = world
        .admitted
        .manifest
        .port_profiles
        .iter()
        .map(|profile| {
            blake3_ref(
                format!(
                    "{}:{}:{}:{}",
                    profile.class.as_str(),
                    profile.port_id,
                    profile.command_schema_ref,
                    profile.event_schema_ref
                )
                .as_bytes(),
            )
        })
        .collect::<Vec<_>>();
    canonical_simulation_differential(DifferentialInput {
        simulation_profile_ref: &simulation_profile_ref,
        live_profile_ref: &live_profile_ref,
        shared_contract_ref: &shared_contract_ref,
        simulation_trace_refs: &trace_refs,
        live_trace_refs: &trace_refs,
        normalized_difference_refs: Vec::new(),
    })
}

fn workload_choice(step: &SimulationWorkloadStep) -> EligibleChoice {
    EligibleChoice {
        kind: SchedulerChoiceKind::Runnable,
        choice_id: workload_choice_id(step.sequence),
        node_id: step.node_id.clone(),
        generation: INITIAL_EXTENSION_GENERATION,
        ready_at_tick: FIRST_VIRTUAL_TICK,
    }
}

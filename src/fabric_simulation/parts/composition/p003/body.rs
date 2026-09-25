
// r[impl molten.fabric_simulation.replay_shrink]
// r[impl molten.fabric_simulation.causal_exploration]
pub fn run_reference_shrink_fixture() -> crate::error::Result<ReferenceShrinkFixture> {
    let mut manifest = reference_world_manifest()?;
    manifest.invariants.push(SimulationInvariant::ExtensionSemantic {
        service: crate::fabric::ReferenceSystemKind::TransactionalKeyValue,
        invariant_id: FIXTURE_FAILING_INVARIANT_ID.to_string(),
    });
    let original_world = canonical_admit_simulated_world(&manifest)?;
    let result = shrink_simulation_failure(&original_world.admitted.manifest, |candidate| {
        run_reference_world(&candidate.manifest, DEFAULT_REFERENCE_SEED)
            .ok()
            .and_then(|run| failure_fingerprint(&run.run.summary))
    })
    .map_err(|error| {
        crate::error::MoltenError::invalid_harness(format!("reference shrink fixture denied: {error:?}"))
    })?;
    let shrunk_world = canonical_admit_simulated_world(&result.world)?;
    let shrink = canonical_simulation_shrink(&original_world.world_ref, &shrunk_world, result)?;
    Ok(ReferenceShrinkFixture {
        original_world,
        shrunk_world,
        shrink,
    })
}

fn prepare_reference_world() -> crate::error::Result<PreparedReferenceWorld> {
    prepare_world_for_manifest(&reference_world_manifest()?)
}

fn prepare_world_for_manifest(manifest: &SimulatedWorldManifest) -> crate::error::Result<PreparedReferenceWorld> {
    let profiles = reference_port_profiles();
    let descriptors = reference_port_descriptors(&profiles);
    let operations = default_reference_operations();
    let tier = crate::fabric::canonical_extension_tier_admission(&crate::fabric::ExtensionTierRequest {
        tier: crate::fabric::ExtensionTier::SystemExtension,
        requested_authorities: all_reference_authorities(),
        admission_evidence: crate::fabric::REQUIRED_SYSTEM_EXTENSION_EVIDENCE.to_vec(),
    })?;
    let mut hosts = std::collections::BTreeMap::new();
    let mut admissions = std::collections::BTreeMap::new();
    for node in &manifest.nodes {
        let kind = service_kind_for_node(node)?;
        let implementation_ref = blake3_ref(format!("reference-implementation:{}", kind.as_str()).as_bytes());
        let input = reference_manifest_input(kind, implementation_ref.clone(), &profiles)?;
        let admitted =
            crate::system_extension::canonical_admit_system_extension_manifest(&input, &descriptors, &tier, &[
                crate::system_extension::ExecutionProfile::InProcessNative,
            ])?;
        let executor = ReferenceServiceExecutor::new(kind, operations_for_kind(&operations, kind), &profiles)?;
        let mut host = crate::system_extension::SystemExtensionHost::new(admitted.clone(), executor)?;
        host.activate(FIRST_VIRTUAL_TICK)?;
        if hosts.insert(node.node_id.clone(), host).is_some() {
            return Err(crate::error::MoltenError::invalid_harness(format!(
                "duplicate reference node {}",
                node.node_id
            )));
        }
        if admissions.len() >= manifest.nodes.len() {
            return Err(crate::error::MoltenError::invalid_harness(
                "reference world admits at most one extension admission per node",
            ));
        }
        admissions.insert(node.node_id.clone(), admitted);
    }
    Ok(PreparedReferenceWorld {
        world: canonical_admit_simulated_world(manifest)?,
        hosts,
        admissions,
        profiles,
        operations,
    })
}

fn service_kind_for_node(node: &SimulatedNode) -> crate::error::Result<crate::fabric::ReferenceSystemKind> {
    service_kind_for_service_id(&node.service_id, &node.node_id)
}

fn service_kind_for_service_id(
    service_id: &str,
    node_id: &str,
) -> crate::error::Result<crate::fabric::ReferenceSystemKind> {
    let kinds = [
        crate::fabric::ReferenceSystemKind::TransactionalKeyValue,
        crate::fabric::ReferenceSystemKind::ReplicatedLog,
        crate::fabric::ReferenceSystemKind::DistributedScheduler,
    ];
    kinds.into_iter().find(|kind| service_id.contains(kind.as_str())).ok_or_else(|| {
        crate::error::MoltenError::invalid_harness(format!(
            "reference node {node_id} does not name a reference service"
        ))
    })
}

fn run_prepared_reference_world(
    mut prepared: PreparedReferenceWorld,
    seed: u64,
) -> crate::error::Result<ReferenceSimulationFixtureRun> {
    let mut router = DeterministicSimulationPortRouter::new(&prepared.world);
    let ReferenceChoiceTrace {
        scheduler,
        observations,
        choice_records,
        crash_recoveries,
    } = run_reference_choices(&mut prepared, &mut router, seed)?;
    let scheduler = finish_simulation_scheduler(&prepared.world.admitted, &scheduler).map_err(|error| {
        crate::error::MoltenError::invalid_harness(format!("reference scheduler finish denied: {error:?}"))
    })?;
    let FinishedReferenceHosts {
        final_state_refs,
        host_evidence_refs,
        service_states,
    } = finish_reference_hosts(&mut prepared, &scheduler)?;
    let plain_observations = observations.iter().map(|item| item.observation.clone()).collect::<Vec<_>>();
    let invariant_results = evaluate_invariants(&prepared.world.admitted.manifest.invariants, &plain_observations);
    let decision = if invariant_results.iter().all(|result| result.passed) {
        SimulationDecision::Pass
    } else {
        SimulationDecision::InvariantFailed
    };
    let resource_units = reference_resource_units(&prepared, &router, choice_records.len())?;
    let summary = SimulationRunSummary {
        decision,
        choice_records,
        invariant_results,
        final_state_refs,
        first_divergence: None,
        resource_units,
        virtual_ticks: scheduler.virtual_tick,
    };
    let observation_refs = observations.iter().map(|item| item.observation_ref.clone()).collect();
    let port_event_refs = router.events.iter().map(|item| item.event_ref.clone()).collect();
    let run = canonical_simulation_run(
        &prepared.world.world_ref,
        SimulationClaimProfile::DeterministicWholeSystem,
        summary,
        observation_refs,
        port_event_refs,
    )?;
    let bundle = canonical_simulation_repro_bundle(&prepared.world, &run, None)?;
    let differential = reference_contract_differential(&prepared.world)?;
    Ok(ReferenceSimulationFixtureRun {
        world: prepared.world,
        run,
        bundle,
        observations,
        port_events: router.events,
        differential,
        host_evidence_refs,
        crash_recoveries,
        service_states,
    })
}

/// The scheduler state, observations, recorded choices, and crash recoveries of one seeded
/// reference run.
struct ReferenceChoiceTrace {
    scheduler: SimulationSchedulerState,
    observations: Vec<CanonicalSimulationObservation>,
    choice_records: Vec<SchedulerChoiceRecord>,
    crash_recoveries: Vec<ReferenceCrashRecovery>,
}

/// Drives the seeded scheduler until no choice is eligible and no fault or port becomes ready
/// later.
fn run_reference_choices(
    prepared: &mut PreparedReferenceWorld,
    router: &mut DeterministicSimulationPortRouter,
    seed: u64,
) -> crate::error::Result<ReferenceChoiceTrace> {
    let mut pending = prepared.world.admitted.manifest.workload.clone();
    let mut scheduler = SimulationSchedulerState::initial();
    let mut observations = Vec::new();
    let mut choice_records = Vec::new();
    // The admitted `max-choices` bound caps the recorded scheduler choices (the core scheduler enforces
    // it too).
    let max_choice_records = usize::try_from(prepared.world.admitted.manifest.bounds.max_choices).map_err(|_| {
        crate::error::MoltenError::invalid_harness("reference world max-choices bound does not fit usize")
    })?;
    let mut crash_recoveries = Vec::with_capacity(prepared.world.admitted.manifest.faults.len());
    let mut fired_recovery_faults = std::collections::BTreeSet::new();
    let mut history_material = FIRST_HISTORY_MATERIAL.to_string();
    loop {
        router.begin_choice(scheduler.next_choice_position, scheduler.virtual_tick);
        router.step_boundary_faults();
        apply_due_recovery_faults(prepared, router, &scheduler, &mut fired_recovery_faults, &mut crash_recoveries)?;
        let mut eligible = Vec::new();
        if let Some(step) = pending.first() {
            eligible.push(workload_choice(step));
        }
        eligible.extend(router.storage().eligible_completions(scheduler.virtual_tick));
        eligible.extend(router.transport().eligible_deliveries(scheduler.virtual_tick));
        if eligible.is_empty() {
            let Some(next_tick) = next_readiness_tick(prepared, router, &scheduler, &fired_recovery_faults) else {
                break;
            };
            scheduler = advance_simulation_time(&prepared.world.admitted, &scheduler, next_tick).map_err(|error| {
                crate::error::MoltenError::invalid_harness(format!("reference time advance denied: {error:?}"))
            })?;
            continue;
        }
        eligible.sort();
        let seeded_index = seeded_selection_index(seed, scheduler.next_choice_position, eligible.len());
        let recorded_choice_id = eligible[seeded_index].choice_id.clone();
        let mut transition =
            select_simulation_choice(&prepared.world.admitted, &scheduler, &eligible, Some(&recorded_choice_id))
                .map_err(|error| {
                    crate::error::MoltenError::invalid_harness(format!("reference scheduler denied: {error:?}"))
                })?;
        router.begin_choice(transition.record.position, transition.next.virtual_tick);
        router.step_boundary_faults();
        transition.record.semantic_output_ref = if transition.record.selected.kind == SchedulerChoiceKind::Runnable {
            let step = pending.remove(0);
            let mut context = WorkloadStepContext {
                prepared: &mut *prepared,
                router: &mut *router,
                observations: &mut observations,
                history_material: &mut history_material,
            };
            execute_workload_step(&mut context, &step, &transition)?
        } else {
            execute_port_choice(router, &transition)?
        };
        if choice_records.len() >= max_choice_records {
            return Err(crate::error::MoltenError::invalid_harness(
                "reference world records at most max-choices scheduler choices",
            ));
        }
        choice_records.push(transition.record);
        scheduler = transition.next;
    }
    Ok(ReferenceChoiceTrace {
        scheduler,
        observations,
        choice_records,
        crash_recoveries,
    })
}

/// Applies every crash or restart fault whose activation choice or tick has been reached,
/// rebuilding the hosts from the durable image after each recovery.
fn apply_due_recovery_faults(
    prepared: &mut PreparedReferenceWorld,
    router: &mut DeterministicSimulationPortRouter,
    scheduler: &SimulationSchedulerState,
    fired_recovery_faults: &mut std::collections::BTreeSet<String>,
    crash_recoveries: &mut impl crate::bounded::VecSink<ReferenceCrashRecovery>,
) -> crate::error::Result<()> {
    for fault in prepared.world.admitted.manifest.faults.clone() {
        if !matches!(fault.kind, SimulationFaultKind::Crash | SimulationFaultKind::Restart)
            || (scheduler.next_choice_position < fault.activate_at_choice
                && scheduler.virtual_tick < fault.activate_at_choice)
        {
            continue;
        }
        fired_recovery_faults.insert(fault.fault_id.clone());
        let recovery = router.apply_crash(&fault).map_err(|error| {
            crate::error::MoltenError::invalid_harness(format!("reference crash recovery denied: {error:?}"))
        })?;
        rebuild_hosts_from_image(prepared, router.storage().durable_image())?;
        crash_recoveries.push_item(recovery);
    }
    Ok(())
}

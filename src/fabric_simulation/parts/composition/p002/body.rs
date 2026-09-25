
// r[impl molten.fabric_simulation.port_substitution]
// r[impl molten.fabric_simulation.fault_model]
// r[impl molten.fabric_simulation.stateful_transport]
// r[impl molten.fabric_simulation.stateful_storage]
impl FabricEffectPort for DeterministicSimulationPortRouter {
    fn route(
        &mut self,
        binding: &crate::fabric::CanonicalFabricPortBinding,
        effect: &crate::system_extension::TypedEffectRequest,
    ) -> FabricPortResult<crate::system_extension::PortEffectOutput> {
        let profile = self
            .profiles
            .get(&binding.binding.key.port_id)
            .cloned()
            .ok_or_else(|| FabricPortError::malformed("simulation effect used an unknown port profile"))?;
        if binding.binding.key.version != profile.version
            || binding.binding.class != profile.class
            || binding.binding.implementation_profile != profile.implementation_profile
        {
            return Err(FabricPortError::malformed("simulation effect profile substitution denied"));
        }
        if !profile.deterministic {
            return Err(FabricPortError::capability("simulation effect cannot route through a live adapter"));
        }
        let is_target_matches = matches!(
            &effect.target,
            crate::system_extension::EffectTarget::FabricPort(key) if key == &binding.binding.key
        );
        if !is_target_matches {
            return Err(FabricPortError::malformed("simulation effect target does not match its canonical binding"));
        }
        let active_fault = self.active_fault(&profile).cloned();
        let fault_cost = active_fault.as_ref().map_or(0, |fault| fault.resource_cost);
        let increment = effect
            .accounted_bytes
            .checked_add(fault_cost)
            .ok_or_else(|| FabricPortError::malformed("simulation resource increment overflow"))?;
        let next_units = self
            .resource_units
            .checked_add(increment)
            .ok_or_else(|| FabricPortError::malformed("simulation resource counter overflow"))?;
        if next_units > self.max_resource_units {
            return Err(FabricPortError::capability("simulation resource envelope exhausted"));
        }
        let output_ref = match profile.class {
            crate::fabric::FabricPortClass::Transport => {
                self.submit_transmission(&profile, effect, active_fault.as_ref())?
            }
            crate::fabric::FabricPortClass::DurableState => {
                self.submit_storage_operation(&profile, effect, active_fault.as_ref())?
            }
            _ => {
                let output_material = format!(
                    "{}:{}:{}:{}:{}",
                    profile.port_id,
                    effect.request_ref,
                    effect.generation,
                    self.current_choice_position,
                    active_fault.as_ref().map_or("none", |fault| fault.kind.as_str()),
                );
                blake3_ref(output_material.as_bytes())
            }
        };
        self.push_event(SimulationPortEventInput {
            choice_position: self.current_choice_position,
            class: profile.class,
            port_id: &profile.port_id,
            request_ref: &effect.request_ref,
            output_ref: &output_ref,
            fault: active_fault.map(|fault| fault.kind),
        })?;
        self.resource_units = next_units;
        Ok(crate::system_extension::PortEffectOutput {
            output_schema_ref: effect.output_schema_ref.clone(),
            output_ref,
            materialized_output: None,
        })
    }
}

// r[impl molten.fabric_simulation.same_core]
// r[impl molten.fabric_simulation.reference_services]
// r[impl molten.fabric_simulation.fabric_sufficiency]
pub fn build_reference_simulated_world() -> crate::error::Result<CanonicalSimulatedWorld> {
    Ok(prepare_reference_world()?.world)
}

// r[impl molten.fabric_simulation.world_manifest]
pub fn reference_world_manifest() -> crate::error::Result<SimulatedWorldManifest> {
    let profiles = reference_port_profiles();
    let operations = default_reference_operations();
    let nodes = reference_nodes(&profiles)?;
    let workload = operations
        .iter()
        .enumerate()
        .filter_map(|(index, (kind, request_ref, _))| {
            let sequence = u64::try_from(index).ok()?;
            Some(SimulationWorkloadStep {
                sequence,
                node_id: reference_node_id(*kind),
                request_ref: request_ref.clone(),
                service: *kind,
                expected_failure_class: None,
            })
        })
        .collect::<Vec<_>>();
    let transport_port_id = profiles
        .iter()
        .find(|profile| profile.class == crate::fabric::FabricPortClass::Transport)
        .map(|profile| profile.port_id.clone())
        .ok_or_else(|| crate::error::MoltenError::invalid_harness("reference world has no transport port profile"))?;
    Ok(SimulatedWorldManifest {
        schema: FABRIC_SIMULATION_WORLD_SCHEMA.to_string(),
        runtime_ref: blake3_ref(b"molten-runtime-reference-simulation"),
        scheduler_input_ref: blake3_ref(b"reference-scheduler-input"),
        entropy_input_ref: blake3_ref(b"reference-entropy-input"),
        authority_ref: blake3_ref(b"reference-authority"),
        policy_ref: blake3_ref(b"reference-policy"),
        initial_durable_state_ref: blake3_ref(b"reference-initial-durable-state"),
        resource_profile_ref: blake3_ref(b"reference-resource-profile"),
        workload_ref: blake3_ref(b"reference-workload"),
        fault_plan_ref: blake3_ref(b"reference-fault-plan"),
        invariant_set_ref: blake3_ref(b"reference-invariant-set"),
        nodes,
        port_profiles: profiles,
        workload,
        faults: vec![SimulationFaultAction {
            fault_id: "transport-delay-at-choice-one".to_string(),
            kind: SimulationFaultKind::Delay,
            target: transport_port_id,
            boundary: crate::fabric::FabricPortClass::Transport,
            activate_at_choice: REFERENCE_FAULT_ACTIVATION_CHOICE,
            duration_choices: Some(REFERENCE_FAULT_DURATION_CHOICES),
            resource_cost: REFERENCE_FAULT_RESOURCE_COST,
            expected_observation: "delayed-transport-port-event".to_string(),
            direct_extension_state_mutation: false,
        }],
        invariants: reference_invariants(),
        bounds: SimulationBounds {
            max_choices: REFERENCE_WORLD_MAX_CHOICES,
            max_events: REFERENCE_WORLD_MAX_EVENTS,
            max_virtual_ticks: REFERENCE_WORLD_MAX_VIRTUAL_TICKS,
            max_trace_bytes: REFERENCE_WORLD_MAX_TRACE_BYTES,
            max_resource_units: REFERENCE_WORLD_MAX_RESOURCE_UNITS,
            max_shrink_attempts: REFERENCE_WORLD_MAX_SHRINK_ATTEMPTS,
        },
        claim_profile: SimulationClaimProfile::DeterministicWholeSystem,
        non_claims: REQUIRED_SIMULATION_NON_CLAIMS.to_vec(),
        ambient_inputs: Vec::new(),
    })
}

/// One node per reference system, running the same extension core identity in simulation and live.
fn reference_nodes(profiles: &[SimulatedPortProfile]) -> crate::error::Result<Vec<SimulatedNode>> {
    let kinds = [
        crate::fabric::ReferenceSystemKind::TransactionalKeyValue,
        crate::fabric::ReferenceSystemKind::ReplicatedLog,
        crate::fabric::ReferenceSystemKind::DistributedScheduler,
    ];
    let mut nodes = Vec::with_capacity(kinds.len());
    for kind in kinds {
        let implementation_ref = blake3_ref(format!("reference-implementation:{}", kind.as_str()).as_bytes());
        let input = reference_manifest_input(kind, implementation_ref.clone(), profiles)?;
        let node_id = reference_node_id(kind);
        let identity = ExtensionCoreIdentity {
            implementation_ref,
            manifest_ref: blake3_ref(format!("reference-manifest:{}", kind.as_str()).as_bytes()),
            callback_dispatcher_ref: blake3_ref(b"system-extension-callback-dispatcher-v1"),
            protocol_core_ref: blake3_ref(format!("reference-protocol-core:{}", kind.as_str()).as_bytes()),
            state_machine_ref: blake3_ref(format!("reference-state-machine:{}", kind.as_str()).as_bytes()),
            schema_set_ref: blake3_ref(format!("reference-schema-set:{}", kind.as_str()).as_bytes()),
            port_contract_set_ref: blake3_ref(format!("reference-port-set:{}", kind.as_str()).as_bytes()),
        };
        nodes.push(SimulatedNode {
            node_id,
            extension_id: input.extension_id,
            service_id: input.service_id,
            generation: INITIAL_EXTENSION_GENERATION,
            initial_state_ref: blake3_ref(b"reference-initial-state"),
            membership_view_ref: blake3_ref(b"reference-membership-view"),
            placement_ref: blake3_ref(format!("reference-placement:{}", kind.as_str()).as_bytes()),
            consistency_profile_ref: blake3_ref(b"reference-consistency-profile"),
            same_core: SameCoreWitness {
                simulation: identity.clone(),
                live: identity,
            },
            required_port_classes: reference_required_ports(kind),
        });
    }
    Ok(nodes)
}

// r[impl molten.fabric_simulation.stateful_storage]
pub fn causal_acknowledgment_manifest(is_completion_delayed: bool) -> crate::error::Result<SimulatedWorldManifest> {
    let mut manifest = reference_world_manifest()?;
    let kv_node = reference_node_id(crate::fabric::ReferenceSystemKind::TransactionalKeyValue);
    manifest.nodes.retain(|node| node.node_id == kv_node);
    manifest.workload.retain(|step| step.node_id == kv_node);
    let durable_port_id = manifest
        .port_profiles
        .iter()
        .find(|profile| profile.class == crate::fabric::FabricPortClass::DurableState)
        .map(|profile| profile.port_id.clone())
        .ok_or_else(|| {
            crate::error::MoltenError::invalid_harness("reference world has no durable-state port profile")
        })?;
    let mut faults = Vec::new();
    if is_completion_delayed {
        faults.push(SimulationFaultAction {
            fault_id: "storage-completion-delay".to_string(),
            kind: SimulationFaultKind::Delay,
            target: durable_port_id.clone(),
            boundary: crate::fabric::FabricPortClass::DurableState,
            activate_at_choice: FIRST_CHOICE_POSITION,
            duration_choices: Some(DEFAULT_COMPLETION_DELAY_TICKS),
            resource_cost: REFERENCE_FAULT_RESOURCE_COST,
            expected_observation: "delayed-storage-completion-holds-the-acknowledged-write".to_string(),
            direct_extension_state_mutation: false,
        });
    }
    faults.push(SimulationFaultAction {
        fault_id: "storage-crash-after-acknowledgment".to_string(),
        kind: SimulationFaultKind::Crash,
        target: durable_port_id,
        boundary: crate::fabric::FabricPortClass::DurableState,
        activate_at_choice: REFERENCE_CRASH_ACTIVATION,
        duration_choices: None,
        resource_cost: REFERENCE_FAULT_RESOURCE_COST,
        expected_observation: "service-crashes-and-recovers-from-the-durable-image".to_string(),
        direct_extension_state_mutation: false,
    });
    manifest.faults = faults;
    Ok(manifest)
}

// r[impl molten.fabric_simulation.scheduler]
// r[impl molten.fabric_simulation.invariants]
// r[impl molten.fabric_simulation.reference_services]
// r[impl molten.fabric_simulation.evidence]
// r[impl molten.fabric_simulation.final_validation]
pub fn run_reference_simulation_fixture() -> crate::error::Result<ReferenceSimulationFixtureRun> {
    let prepared = prepare_reference_world()?;
    run_prepared_reference_world(prepared, DEFAULT_REFERENCE_SEED)
}

// r[impl molten.fabric_simulation.causal_exploration]
// r[impl molten.fabric_simulation.scheduler]
// r[impl molten.fabric_simulation.reference_services]
// r[impl molten.fabric_simulation.stateful_transport]
// r[impl molten.fabric_simulation.stateful_storage]
pub fn run_reference_world(
    manifest: &SimulatedWorldManifest,
    seed: u64,
) -> crate::error::Result<ReferenceSimulationFixtureRun> {
    let prepared = prepare_world_for_manifest(manifest)?;
    run_prepared_reference_world(prepared, seed)
}

// r[impl molten.fabric_simulation.replay_shrink]
pub fn replay_reference_simulation_fixture(
    expected: &CanonicalSimulationRun,
) -> crate::error::Result<ReferenceReplayResult> {
    let replay = run_reference_simulation_fixture()?;
    if replay.world.world_ref != expected.world_ref {
        return Err(crate::error::MoltenError::invalid_harness(
            "reference simulation replay world identity differs from the expected run",
        ));
    }
    let comparison = compare_replay(&expected.summary.choice_records, &replay.run.summary.choice_records);
    Ok(ReferenceReplayResult { comparison, replay })
}

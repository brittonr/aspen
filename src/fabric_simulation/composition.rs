use super::*;
#[allow(
    tigerstyle::non_trait_imports,
    reason = "the simulation shell implements the application-owned typed effect-port boundary"
)]
use crate::fabric::FabricPortError;
#[allow(
    tigerstyle::non_trait_imports,
    reason = "the simulation shell implements the application-owned typed effect-port boundary"
)]
use crate::fabric::FabricPortResult;
use crate::system_extension::FabricEffectPort;

const REFERENCE_WORLD_MAX_CHOICES: u64 = 256;
const REFERENCE_WORLD_MAX_EVENTS: u64 = 256;
const REFERENCE_WORLD_MAX_VIRTUAL_TICKS: u64 = 4_096;
const REFERENCE_WORLD_MAX_TRACE_BYTES: u64 = 1_048_576;
const REFERENCE_WORLD_MAX_RESOURCE_UNITS: u64 = 4_096;
const REFERENCE_WORLD_MAX_SHRINK_ATTEMPTS: u64 = 256;
const REFERENCE_REQUEST_BYTES: u64 = 1;
const REFERENCE_FAULT_RESOURCE_COST: u64 = 1;
const REFERENCE_FAULT_ACTIVATION_CHOICE: u64 = 1;
const REFERENCE_FAULT_DURATION_CHOICES: u64 = 6;
const REFERENCE_CHOICE_ID_WIDTH: usize = 4;
const RUN_RESOURCE_INCREMENT: u64 = 1;
const FIRST_HISTORY_MATERIAL: &str = "fabric-simulation-history-v1";
pub const DEFAULT_REFERENCE_SEED: u64 = 0;
pub const DEFAULT_COMPLETION_DELAY_TICKS: u64 = 8;
const SUBMISSION_ACK_MATERIAL: &str = "submitted";
const COMPLETION_ACK_MATERIAL: &str = "completed";
pub const REFERENCE_CRASH_ACTIVATION: u64 = 3;
const DELIVERY_ACK_MATERIAL: &str = "delivered";
const CRASH_ACK_MATERIAL: &str = "crash-recovery";
const FIXTURE_FAILING_INVARIANT_ID: &str = "reference-fixture-failing-invariant";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceSimulationFixtureRun {
    pub world: CanonicalSimulatedWorld,
    pub run: CanonicalSimulationRun,
    pub bundle: CanonicalSimulationReproBundle,
    pub observations: Vec<CanonicalSimulationObservation>,
    pub port_events: Vec<CanonicalSimulationPortEvent>,
    pub differential: CanonicalSimulationDifferential,
    pub host_evidence_refs: Vec<String>,
    pub crash_recoveries: Vec<ReferenceCrashRecovery>,
    pub service_states: std::collections::BTreeMap<String, ReferenceServiceState>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceCrashRecovery {
    pub fault_id: String,
    pub lost_operations: Vec<SimulatedStorageOperation>,
    pub durable_entry_request_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceReplayResult {
    pub comparison: ReplayComparison,
    pub replay: ReferenceSimulationFixtureRun,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceShrinkFixture {
    pub original_world: CanonicalSimulatedWorld,
    pub shrunk_world: CanonicalSimulatedWorld,
    pub shrink: CanonicalSimulationShrink,
}

struct PreparedReferenceWorld {
    world: CanonicalSimulatedWorld,
    hosts: std::collections::BTreeMap<String, crate::system_extension::SystemExtensionHost<ReferenceServiceExecutor>>,
    admissions: std::collections::BTreeMap<String, crate::system_extension::CanonicalAdmittedSystemExtensionManifest>,
    profiles: Vec<SimulatedPortProfile>,
    operations: Vec<(crate::fabric::ReferenceSystemKind, String, ReferenceServiceOperation)>,
}

#[derive(Debug)]
struct DeterministicSimulationPortRouter {
    profiles: std::collections::BTreeMap<String, SimulatedPortProfile>,
    faults: Vec<SimulationFaultAction>,
    current_choice_position: u64,
    current_virtual_tick: u64,
    dispatching_node: String,
    dispatching_request_ref: String,
    next_submission_ordinal: u64,
    resource_units: u64,
    max_resource_units: u64,
    events: Vec<CanonicalSimulationPortEvent>,
    transport: SimulatedTransportState,
    storage: SimulatedStorageState,
    opened_partition_faults: std::collections::BTreeSet<String>,
}

impl DeterministicSimulationPortRouter {
    fn new(world: &CanonicalSimulatedWorld) -> Self {
        let profiles = world
            .admitted
            .manifest
            .port_profiles
            .iter()
            .cloned()
            .map(|profile| (profile.port_id.clone(), profile))
            .collect();
        Self {
            profiles,
            faults: world.admitted.manifest.faults.clone(),
            current_choice_position: FIRST_CHOICE_POSITION,
            current_virtual_tick: FIRST_VIRTUAL_TICK,
            dispatching_node: String::new(),
            dispatching_request_ref: String::new(),
            next_submission_ordinal: FIRST_WORKLOAD_SEQUENCE,
            resource_units: 0,
            max_resource_units: world.admitted.manifest.bounds.max_resource_units,
            events: Vec::new(),
            transport: SimulatedTransportState::new(),
            storage: SimulatedStorageState::new(),
            opened_partition_faults: std::collections::BTreeSet::new(),
        }
    }

    fn begin_choice(&mut self, position: u64, virtual_tick: u64) {
        self.current_choice_position = position;
        self.current_virtual_tick = virtual_tick;
    }

    fn set_dispatching(&mut self, node_id: &str, request_ref: &str) {
        self.dispatching_node = node_id.to_string();
        self.dispatching_request_ref = request_ref.to_string();
    }

    fn events(&self) -> &[CanonicalSimulationPortEvent] {
        &self.events
    }

    fn resource_units(&self) -> u64 {
        self.resource_units
    }

    fn transport(&self) -> &SimulatedTransportState {
        &self.transport
    }

    fn storage(&self) -> &SimulatedStorageState {
        &self.storage
    }

    // r[impl molten.fabric_simulation.stateful_transport]
    // r[impl molten.fabric_simulation.stateful_storage]
    fn step_boundary_faults(&mut self) {
        self.transport.heal_ready_partitions(self.current_virtual_tick);
        let position = self.current_choice_position;
        let tick = self.current_virtual_tick;
        let mut openings = Vec::with_capacity(self.faults.len());
        for fault in &self.faults {
            if fault.kind != SimulationFaultKind::Partition
                || self.opened_partition_faults.contains(&fault.fault_id)
                || !self.fault_is_active(fault, position)
            {
                continue;
            }
            let heals_at_tick = fault
                .duration_choices
                .map_or(NEVER_HEALS_TICK, |duration| tick.checked_add(duration).unwrap_or(NEVER_HEALS_TICK));
            openings.push((fault.fault_id.clone(), SimulatedPartition {
                fault_id: fault.fault_id.clone(),
                destination: fault.target.clone(),
                heals_at_tick,
            }));
        }
        for (fault_id, partition) in openings {
            if self.transport.open_partition(partition).is_ok() {
                self.opened_partition_faults.insert(fault_id);
            }
        }
    }

    fn fault_is_active(&self, fault: &SimulationFaultAction, position: u64) -> bool {
        if position < fault.activate_at_choice {
            return false;
        }
        match fault.duration_choices {
            None => true,
            Some(duration) => fault.activate_at_choice.checked_add(duration).is_some_and(|end| position < end),
        }
    }

    fn active_fault(&self, profile: &SimulatedPortProfile) -> Option<&SimulationFaultAction> {
        self.faults.iter().find(|fault| {
            fault.boundary == profile.class
                && fault.target == profile.port_id
                && self.fault_is_active(fault, self.current_choice_position)
        })
    }

    fn next_submission_ordinal(&mut self) -> Result<u64, FabricPortError> {
        let ordinal = self.next_submission_ordinal;
        self.next_submission_ordinal = self
            .next_submission_ordinal
            .checked_add(UNIT_RESOURCE_COST)
            .ok_or_else(|| FabricPortError::malformed("simulation submission ordinal overflow"))?;
        Ok(ordinal)
    }

    // r[impl molten.fabric_simulation.stateful_transport]
    fn submit_transmission(
        &mut self,
        profile: &SimulatedPortProfile,
        effect: &crate::system_extension::TypedEffectRequest,
        fault: Option<&SimulationFaultAction>,
    ) -> Result<String, FabricPortError> {
        let transmission_id = format!("{}:{}:{}", profile.port_id, effect.request_ref, self.current_choice_position);
        let fault_delay = fault
            .filter(|fault| fault.kind == SimulationFaultKind::Delay)
            .and_then(|fault| fault.duration_choices)
            .unwrap_or(UNIT_RESOURCE_COST);
        let eligible_at_tick = self
            .current_virtual_tick
            .checked_add(fault_delay)
            .ok_or_else(|| FabricPortError::malformed("simulation transport delay tick overflow"))?;
        let transmission = SimulatedTransmission {
            transmission_id: transmission_id.clone(),
            destination: self.dispatching_node.clone(),
            port_id: profile.port_id.clone(),
            request_ref: effect.request_ref.clone(),
            generation: effect.generation,
            submitted_at_tick: self.current_virtual_tick,
            eligible_at_tick,
        };
        self.transport
            .submit(transmission)
            .map_err(|error| FabricPortError::malformed(format!("simulation transport submit denied: {error:?}")))?;
        if fault.is_some_and(|fault| fault.kind == SimulationFaultKind::Drop)
            && self.transport.drop_transmission(&transmission_id).is_err()
        {
            return Err(FabricPortError::malformed("simulation dropped transmission disappeared"));
        }
        let fault_kind = fault.map_or("none", |fault| fault.kind.as_str());
        Ok(blake3_ref(format!("{transmission_id}:{SUBMISSION_ACK_MATERIAL}:{fault_kind}").as_bytes()))
    }

    // r[impl molten.fabric_simulation.stateful_storage]
    fn submit_storage_operation(
        &mut self,
        profile: &SimulatedPortProfile,
        effect: &crate::system_extension::TypedEffectRequest,
        fault: Option<&SimulationFaultAction>,
    ) -> Result<String, FabricPortError> {
        let operation_id = format!("{}:{}:{}", profile.port_id, effect.request_ref, self.current_choice_position);
        let ordinal = self.next_submission_ordinal()?;
        let operation = SimulatedStorageOperation {
            operation_id: operation_id.clone(),
            port_id: profile.port_id.clone(),
            owner_node_id: self.dispatching_node.clone(),
            request_ref: self.dispatching_request_ref.clone(),
            phase: crate::core_api::world_faults::FaultPhase::AfterPossibleSubmit,
            submission_ordinal: ordinal,
            submitted_at_tick: self.current_virtual_tick,
            eligible_at_tick: self.current_virtual_tick,
        };
        self.storage
            .submit(operation)
            .map_err(|error| FabricPortError::malformed(format!("simulation storage submit denied: {error:?}")))?;
        if let Some(fault) = fault.filter(|fault| fault.kind == SimulationFaultKind::Delay) {
            let delay = fault.duration_choices.unwrap_or(DEFAULT_COMPLETION_DELAY_TICKS);
            let eligible_at_tick = self
                .current_virtual_tick
                .checked_add(delay)
                .ok_or_else(|| FabricPortError::malformed("simulation storage delay tick overflow"))?;
            self.storage
                .hold_completion(&operation_id, eligible_at_tick)
                .map_err(|error| FabricPortError::malformed(format!("simulation storage hold denied: {error:?}")))?;
        }
        Ok(blake3_ref(format!("{operation_id}:{SUBMISSION_ACK_MATERIAL}").as_bytes()))
    }

    fn push_event(&mut self, input: SimulationPortEventInput<'_>) -> Result<(), FabricPortError> {
        let event = canonical_simulation_port_event(input).map_err(FabricPortError::from)?;
        self.events.push(event);
        Ok(())
    }

    // r[impl molten.fabric_simulation.stateful_storage]
    fn complete_storage_head(&mut self, operation_id: &str) -> Result<String, FabricPortError> {
        let entry = self
            .storage
            .complete(operation_id, self.current_virtual_tick)
            .map_err(|error| FabricPortError::malformed(format!("simulation storage completion denied: {error:?}")))?;
        let output_ref = blake3_ref(format!("{}:{COMPLETION_ACK_MATERIAL}", entry.entry_id).as_bytes());
        let profile = self
            .profiles
            .values()
            .find(|profile| profile.class == crate::fabric::FabricPortClass::DurableState)
            .cloned()
            .ok_or_else(|| FabricPortError::malformed("simulation world lacks its durable-state profile"))?;
        self.push_event(SimulationPortEventInput {
            choice_position: self.current_choice_position,
            class: profile.class,
            port_id: &profile.port_id,
            request_ref: &entry.request_ref,
            output_ref: &output_ref,
            fault: None,
        })?;
        Ok(output_ref)
    }

    // r[impl molten.fabric_simulation.stateful_transport]
    fn deliver_transmission(&mut self, transmission_id: &str) -> Result<String, FabricPortError> {
        let transmission = self
            .transport
            .deliver(transmission_id, self.current_virtual_tick)
            .map_err(|error| FabricPortError::malformed(format!("simulation transport delivery denied: {error:?}")))?;
        let output_ref = blake3_ref(format!("{}:{DELIVERY_ACK_MATERIAL}", transmission.transmission_id).as_bytes());
        let profile = self
            .profiles
            .values()
            .find(|profile| profile.class == crate::fabric::FabricPortClass::Transport)
            .cloned()
            .ok_or_else(|| FabricPortError::malformed("simulation world lacks its transport profile"))?;
        self.push_event(SimulationPortEventInput {
            choice_position: self.current_choice_position,
            class: profile.class,
            port_id: &profile.port_id,
            request_ref: &transmission.request_ref,
            output_ref: &output_ref,
            fault: None,
        })?;
        Ok(output_ref)
    }

    // r[impl molten.fabric_simulation.stateful_storage]
    fn apply_crash(&mut self, fault: &SimulationFaultAction) -> Result<ReferenceCrashRecovery, FabricPortError> {
        let lost_operations = self.storage.crash_and_recover();
        let durable_entry_request_refs = self.storage().durable_image().request_refs();
        let recovery_ref = blake3_ref(format!("{}:{CRASH_ACK_MATERIAL}", fault.fault_id).as_bytes());
        let profile =
            self.profiles.values().find(|profile| profile.class == fault.boundary).cloned().ok_or_else(|| {
                FabricPortError::malformed("simulation world lacks the crashed fault boundary profile")
            })?;
        let request_ref = blake3_ref(fault.fault_id.as_bytes());
        self.push_event(SimulationPortEventInput {
            choice_position: self.current_choice_position,
            class: profile.class,
            port_id: &profile.port_id,
            request_ref: &request_ref,
            output_ref: &recovery_ref,
            fault: Some(fault.kind),
        })?;
        Ok(ReferenceCrashRecovery {
            fault_id: fault.fault_id.clone(),
            lost_operations,
            durable_entry_request_refs,
        })
    }
}

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
    let kinds = [
        crate::fabric::ReferenceSystemKind::TransactionalKeyValue,
        crate::fabric::ReferenceSystemKind::ReplicatedLog,
        crate::fabric::ReferenceSystemKind::DistributedScheduler,
    ];
    let mut nodes = Vec::with_capacity(kinds.len());
    for kind in kinds {
        let implementation_ref = blake3_ref(format!("reference-implementation:{}", kind.as_str()).as_bytes());
        let input = reference_manifest_input(kind, implementation_ref.clone(), &profiles)?;
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
    let mut pending = prepared.world.admitted.manifest.workload.clone();
    let mut scheduler = SimulationSchedulerState::initial();
    let mut router = DeterministicSimulationPortRouter::new(&prepared.world);
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
            rebuild_hosts_from_image(&mut prepared, router.storage().durable_image())?;
            crash_recoveries.push(recovery);
        }
        let mut eligible = Vec::new();
        if let Some(step) = pending.first() {
            eligible.push(workload_choice(step));
        }
        eligible.extend(router.storage().eligible_completions(scheduler.virtual_tick));
        eligible.extend(router.transport().eligible_deliveries(scheduler.virtual_tick));
        if eligible.is_empty() {
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
            let Some(next_tick) = readiness.into_iter().filter(|tick| *tick > scheduler.virtual_tick).min() else {
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
        let selected = transition.record.selected.clone();
        router.begin_choice(transition.record.position, transition.next.virtual_tick);
        router.step_boundary_faults();
        let semantic_output_ref = match selected.kind {
            SchedulerChoiceKind::Runnable => {
                let step = pending.remove(0);
                execute_workload_step(
                    &mut WorkloadStepContext {
                        prepared: &mut prepared,
                        router: &mut router,
                        observations: &mut observations,
                        history_material: &mut history_material,
                    },
                    &step,
                    &transition,
                )?
            }
            SchedulerChoiceKind::StorageCompletion => {
                let operation = router
                    .storage()
                    .submitted()
                    .iter()
                    .find(|operation| storage_completion_choice_id(&operation.operation_id) == selected.choice_id)
                    .cloned()
                    .ok_or_else(|| {
                        crate::error::MoltenError::invalid_harness("selected storage completion disappeared")
                    })?;
                router.complete_storage_head(&operation.operation_id).map_err(|error| {
                    crate::error::MoltenError::invalid_harness(format!(
                        "reference storage completion denied: {error:?}"
                    ))
                })?
            }
            SchedulerChoiceKind::MessageDelivery => {
                let transmission = router
                    .transport()
                    .pending()
                    .iter()
                    .find(|transmission| {
                        message_delivery_choice_id(&transmission.transmission_id) == selected.choice_id
                    })
                    .cloned()
                    .ok_or_else(|| {
                        crate::error::MoltenError::invalid_harness("selected transport delivery disappeared")
                    })?;
                router.deliver_transmission(&transmission.transmission_id).map_err(|error| {
                    crate::error::MoltenError::invalid_harness(format!(
                        "reference transport delivery denied: {error:?}"
                    ))
                })?
            }
            other => {
                return Err(crate::error::MoltenError::invalid_harness(format!(
                    "reference runner cannot execute scheduler choice kind {}",
                    other.as_str()
                )));
            }
        };
        transition.record.semantic_output_ref = semantic_output_ref;
        if choice_records.len() >= max_choice_records {
            return Err(crate::error::MoltenError::invalid_harness(
                "reference world records at most max-choices scheduler choices",
            ));
        }
        choice_records.push(transition.record);
        scheduler = transition.next;
    }
    scheduler = finish_simulation_scheduler(&prepared.world.admitted, &scheduler).map_err(|error| {
        crate::error::MoltenError::invalid_harness(format!("reference scheduler finish denied: {error:?}"))
    })?;
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
    let plain_observations = observations.iter().map(|item| item.observation.clone()).collect::<Vec<_>>();
    let invariant_results = evaluate_invariants(&prepared.world.admitted.manifest.invariants, &plain_observations);
    let decision = if invariant_results.iter().all(|result| result.passed) {
        SimulationDecision::Pass
    } else {
        SimulationDecision::InvariantFailed
    };
    let choice_resource_units = u64::try_from(choice_records.len())
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
    canonical_simulation_differential(
        &simulation_profile_ref,
        &live_profile_ref,
        &shared_contract_ref,
        &trace_refs,
        &trace_refs,
        Vec::new(),
    )
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

fn workload_choice_id(sequence: u64) -> String {
    format!("workload-{sequence:0width$}", width = REFERENCE_CHOICE_ID_WIDTH)
}

fn reference_node_id(kind: crate::fabric::ReferenceSystemKind) -> String {
    format!("node-{}", kind.as_str())
}

fn reference_invariants() -> Vec<SimulationInvariant> {
    let mut invariants =
        REQUIRED_UNIVERSAL_INVARIANTS.into_iter().map(SimulationInvariant::Universal).collect::<Vec<_>>();
    invariants.extend([
        SimulationInvariant::ExtensionSemantic {
            service: crate::fabric::ReferenceSystemKind::TransactionalKeyValue,
            invariant_id: "transaction-version-monotonic".to_string(),
        },
        SimulationInvariant::ExtensionSemantic {
            service: crate::fabric::ReferenceSystemKind::TransactionalKeyValue,
            invariant_id: "conflict-does-not-mutate".to_string(),
        },
        SimulationInvariant::ExtensionSemantic {
            service: crate::fabric::ReferenceSystemKind::ReplicatedLog,
            invariant_id: "log-offsets-contiguous".to_string(),
        },
        SimulationInvariant::ExtensionSemantic {
            service: crate::fabric::ReferenceSystemKind::ReplicatedLog,
            invariant_id: "retention-follows-replication".to_string(),
        },
        SimulationInvariant::ExtensionSemantic {
            service: crate::fabric::ReferenceSystemKind::DistributedScheduler,
            invariant_id: "single-authoritative-completion".to_string(),
        },
        SimulationInvariant::ExtensionSemantic {
            service: crate::fabric::ReferenceSystemKind::DistributedScheduler,
            invariant_id: "completion-requires-current-lease".to_string(),
        },
    ]);
    invariants
}

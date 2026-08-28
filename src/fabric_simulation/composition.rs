use std::collections::BTreeMap;

use super::*;
use crate::error::MoltenError;
use crate::error::Result;
use crate::fabric::ExtensionTier;
use crate::fabric::ExtensionTierRequest;
use crate::fabric::FabricPortClass;
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
use crate::fabric::REQUIRED_SYSTEM_EXTENSION_EVIDENCE;
use crate::fabric::ReferenceSystemKind;
use crate::fabric::canonical_extension_tier_admission;
use crate::system_extension::FabricEffectPort;
use crate::system_extension::HostDispatchResult;
use crate::system_extension::PortEffectOutput;
use crate::system_extension::SystemExtensionHost;
use crate::system_extension::TypedEffectRequest;
use crate::system_extension::canonical_admit_system_extension_manifest;

const REFERENCE_WORLD_MAX_CHOICES: u64 = 256;
const REFERENCE_WORLD_MAX_EVENTS: u64 = 256;
const REFERENCE_WORLD_MAX_VIRTUAL_TICKS: u64 = 4_096;
const REFERENCE_WORLD_MAX_TRACE_BYTES: u64 = 1_048_576;
const REFERENCE_WORLD_MAX_RESOURCE_UNITS: u64 = 4_096;
const REFERENCE_WORLD_MAX_SHRINK_ATTEMPTS: u64 = 256;
const REFERENCE_REQUEST_BYTES: u64 = 1;
const REFERENCE_FAULT_RESOURCE_COST: u64 = 1;
const REFERENCE_FAULT_ACTIVATION_CHOICE: u64 = 1;
const REFERENCE_FAULT_DURATION_CHOICES: u64 = 1;
const REFERENCE_CHOICE_ID_WIDTH: usize = 4;
const RUN_RESOURCE_INCREMENT: u64 = 1;
const FIRST_HISTORY_MATERIAL: &str = "fabric-simulation-history-v1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceSimulationFixtureRun {
    pub world: CanonicalSimulatedWorld,
    pub run: CanonicalSimulationRun,
    pub bundle: CanonicalSimulationReproBundle,
    pub observations: Vec<CanonicalSimulationObservation>,
    pub port_events: Vec<CanonicalSimulationPortEvent>,
    pub differential: CanonicalSimulationDifferential,
    pub host_evidence_refs: Vec<String>,
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
    hosts: BTreeMap<String, SystemExtensionHost<ReferenceServiceExecutor>>,
}

#[derive(Debug)]
struct DeterministicSimulationPortRouter {
    profiles: BTreeMap<String, SimulatedPortProfile>,
    faults: Vec<SimulationFaultAction>,
    current_choice_position: u64,
    resource_units: u64,
    max_resource_units: u64,
    events: Vec<CanonicalSimulationPortEvent>,
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
            resource_units: 0,
            max_resource_units: world.admitted.manifest.bounds.max_resource_units,
            events: Vec::new(),
        }
    }

    fn begin_choice(&mut self, position: u64) {
        self.current_choice_position = position;
    }

    fn events(&self) -> &[CanonicalSimulationPortEvent] {
        &self.events
    }

    fn resource_units(&self) -> u64 {
        self.resource_units
    }

    fn active_fault(&self, profile: &SimulatedPortProfile) -> Option<&SimulationFaultAction> {
        self.faults.iter().find(|fault| {
            if fault.boundary != profile.class || fault.target != profile.port_id {
                return false;
            }
            if self.current_choice_position < fault.activate_at_choice {
                return false;
            }
            match fault.duration_choices {
                None => true,
                Some(duration) => {
                    fault.activate_at_choice.checked_add(duration).is_some_and(|end| self.current_choice_position < end)
                }
            }
        })
    }
}

// r[impl molten.fabric_simulation.port_substitution]
// r[impl molten.fabric_simulation.fault_model]
impl FabricEffectPort for DeterministicSimulationPortRouter {
    fn route(
        &mut self,
        binding: &crate::fabric::CanonicalFabricPortBinding,
        effect: &TypedEffectRequest,
    ) -> FabricPortResult<PortEffectOutput> {
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
        let target_matches = matches!(
            &effect.target,
            crate::system_extension::EffectTarget::FabricPort(key) if key == &binding.binding.key
        );
        if !target_matches {
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
        let output_material = format!(
            "{}:{}:{}:{}:{}",
            profile.port_id,
            effect.request_ref,
            effect.generation,
            self.current_choice_position,
            active_fault.as_ref().map_or("none", |fault| fault.kind.as_str()),
        );
        let output_ref = blake3_ref(output_material.as_bytes());
        let event = canonical_simulation_port_event(SimulationPortEventInput {
            choice_position: self.current_choice_position,
            class: profile.class,
            port_id: &profile.port_id,
            request_ref: &effect.request_ref,
            output_ref: &output_ref,
            fault: active_fault.map(|fault| fault.kind),
        })
        .map_err(FabricPortError::from)?;
        self.resource_units = next_units;
        self.events.push(event);
        Ok(PortEffectOutput {
            output_schema_ref: effect.output_schema_ref.clone(),
            output_ref,
            materialized_output: None,
        })
    }
}

// r[impl molten.fabric_simulation.same_core]
// r[impl molten.fabric_simulation.reference_services]
// r[impl molten.fabric_simulation.fabric_sufficiency]
pub fn build_reference_simulated_world() -> Result<CanonicalSimulatedWorld> {
    Ok(prepare_reference_world()?.world)
}

// r[impl molten.fabric_simulation.scheduler]
// r[impl molten.fabric_simulation.invariants]
// r[impl molten.fabric_simulation.reference_services]
// r[impl molten.fabric_simulation.evidence]
// r[impl molten.fabric_simulation.final_validation]
pub fn run_reference_simulation_fixture() -> Result<ReferenceSimulationFixtureRun> {
    let prepared = prepare_reference_world()?;
    run_prepared_reference_world(prepared)
}

// r[impl molten.fabric_simulation.replay_shrink]
pub fn replay_reference_simulation_fixture(expected: &CanonicalSimulationRun) -> Result<ReferenceReplayResult> {
    let replay = run_reference_simulation_fixture()?;
    if replay.world.world_ref != expected.world_ref {
        return Err(MoltenError::invalid_harness(
            "reference simulation replay world identity differs from the expected run",
        ));
    }
    let comparison = compare_replay(&expected.summary.choice_records, &replay.run.summary.choice_records);
    Ok(ReferenceReplayResult { comparison, replay })
}

// r[impl molten.fabric_simulation.replay_shrink]
pub fn run_reference_shrink_fixture() -> Result<ReferenceShrinkFixture> {
    let mut original_manifest = build_reference_simulated_world()?.admitted.manifest;
    let first = original_manifest
        .workload
        .first_mut()
        .ok_or_else(|| MoltenError::invalid_harness("reference shrink fixture requires a workload"))?;
    first.expected_failure_class = Some("fixture-invariant-failure".to_string());
    let original_world = canonical_admit_simulated_world(&original_manifest)?;
    let result = shrink_simulation_failure(&original_world.admitted.manifest, |candidate| {
        candidate
            .manifest
            .workload
            .iter()
            .any(|step| step.expected_failure_class.as_deref() == Some("fixture-invariant-failure"))
    })
    .map_err(|error| MoltenError::invalid_harness(format!("reference shrink fixture denied: {error:?}")))?;
    let shrunk_world = canonical_admit_simulated_world(&result.world)?;
    let shrink = canonical_simulation_shrink(&original_world.world_ref, &shrunk_world, result)?;
    Ok(ReferenceShrinkFixture {
        original_world,
        shrunk_world,
        shrink,
    })
}

fn prepare_reference_world() -> Result<PreparedReferenceWorld> {
    let profiles = reference_port_profiles();
    let descriptors = reference_port_descriptors(&profiles);
    let operations = default_reference_operations();
    let tier = canonical_extension_tier_admission(&ExtensionTierRequest {
        tier: ExtensionTier::SystemExtension,
        requested_authorities: all_reference_authorities(),
        admission_evidence: REQUIRED_SYSTEM_EXTENSION_EVIDENCE.to_vec(),
    })?;
    let kinds = [
        ReferenceSystemKind::TransactionalKeyValue,
        ReferenceSystemKind::ReplicatedLog,
        ReferenceSystemKind::DistributedScheduler,
    ];
    let mut hosts = BTreeMap::new();
    let mut nodes = Vec::new();
    for kind in kinds {
        let implementation_ref = blake3_ref(format!("reference-implementation:{}", kind.as_str()).as_bytes());
        let input = reference_manifest_input(kind, implementation_ref.clone(), &profiles)?;
        let admitted = canonical_admit_system_extension_manifest(&input, &descriptors, &tier, &[
            crate::system_extension::ExecutionProfile::InProcessNative,
        ])?;
        let manifest_ref = admitted.manifest_ref().to_string();
        let node_id = reference_node_id(kind);
        let identity = ExtensionCoreIdentity {
            implementation_ref,
            manifest_ref,
            callback_dispatcher_ref: blake3_ref(b"system-extension-callback-dispatcher-v1"),
            protocol_core_ref: blake3_ref(format!("reference-protocol-core:{}", kind.as_str()).as_bytes()),
            state_machine_ref: blake3_ref(format!("reference-state-machine:{}", kind.as_str()).as_bytes()),
            schema_set_ref: blake3_ref(format!("reference-schema-set:{}", kind.as_str()).as_bytes()),
            port_contract_set_ref: blake3_ref(format!("reference-port-set:{}", kind.as_str()).as_bytes()),
        };
        let executor = ReferenceServiceExecutor::new(kind, operations_for_kind(&operations, kind), &profiles)?;
        let initial_state_ref = blake3_ref(format!("{:?}", executor.state()).as_bytes());
        let mut host = SystemExtensionHost::new(admitted, executor)?;
        host.activate(FIRST_VIRTUAL_TICK)?;
        nodes.push(SimulatedNode {
            node_id: node_id.clone(),
            extension_id: input.extension_id,
            service_id: input.service_id,
            generation: INITIAL_EXTENSION_GENERATION,
            initial_state_ref,
            membership_view_ref: blake3_ref(b"reference-membership-view"),
            placement_ref: blake3_ref(format!("reference-placement:{}", kind.as_str()).as_bytes()),
            consistency_profile_ref: blake3_ref(b"reference-consistency-profile"),
            same_core: SameCoreWitness {
                simulation: identity.clone(),
                live: identity,
            },
            required_port_classes: reference_required_ports(kind),
        });
        if hosts.insert(node_id.clone(), host).is_some() {
            return Err(MoltenError::invalid_harness(format!("duplicate reference node {node_id}")));
        }
    }
    let workload = operations
        .iter()
        .enumerate()
        .map(|(index, (kind, request_ref, _))| {
            let sequence = u64::try_from(index)
                .map_err(|_| MoltenError::invalid_harness("reference workload sequence overflow"))?;
            Ok(SimulationWorkloadStep {
                sequence,
                node_id: reference_node_id(*kind),
                request_ref: request_ref.clone(),
                service: *kind,
                expected_failure_class: None,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let transport_port_id = profiles
        .iter()
        .find(|profile| profile.class == FabricPortClass::Transport)
        .map(|profile| profile.port_id.clone())
        .ok_or_else(|| MoltenError::invalid_harness("reference world lacks its transport profile"))?;
    let manifest = SimulatedWorldManifest {
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
            boundary: FabricPortClass::Transport,
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
    };
    Ok(PreparedReferenceWorld {
        world: canonical_admit_simulated_world(&manifest)?,
        hosts,
    })
}

fn run_prepared_reference_world(mut prepared: PreparedReferenceWorld) -> Result<ReferenceSimulationFixtureRun> {
    let mut pending = prepared.world.admitted.manifest.workload.clone();
    let mut scheduler = SimulationSchedulerState::default();
    let mut router = DeterministicSimulationPortRouter::new(&prepared.world);
    let mut observations = Vec::new();
    let mut choice_records = Vec::new();
    let mut history_material = FIRST_HISTORY_MATERIAL.to_string();
    while !pending.is_empty() {
        let eligible = pending.iter().map(workload_choice).collect::<Vec<_>>();
        let transition = select_simulation_choice(&prepared.world.admitted, &scheduler, &eligible, None)
            .map_err(|error| MoltenError::invalid_harness(format!("reference scheduler denied: {error:?}")))?;
        let selected_id = &transition.record.selected.choice_id;
        let selected_index = pending
            .iter()
            .position(|step| workload_choice_id(step.sequence) == *selected_id)
            .ok_or_else(|| MoltenError::invalid_harness("selected reference workload choice disappeared"))?;
        let step = pending.remove(selected_index);
        router.begin_choice(transition.record.position);
        let host = prepared
            .hosts
            .get_mut(&step.node_id)
            .ok_or_else(|| MoltenError::invalid_harness(format!("missing reference host {}", step.node_id)))?;
        let (receipt, outcome) =
            match host.dispatch_request(&step.request_ref, REFERENCE_REQUEST_BYTES, transition.next.virtual_tick)? {
                HostDispatchResult::Executed { receipt, outcome, .. } => (receipt, outcome),
                other => {
                    return Err(MoltenError::invalid_harness(format!(
                        "reference request did not execute through the system-extension host: {other:?}"
                    )));
                }
            };
        let completions = host.route_approved_effects(&receipt, &mut router)?;
        if completions.is_empty() {
            return Err(MoltenError::invalid_harness("reference request did not cross a deterministic fabric port"));
        }
        let reference_transition = host
            .executor()
            .last_transition()
            .ok_or_else(|| MoltenError::invalid_harness("reference executor did not retain its pure transition"))?;
        let state_ref = outcome
            .state_ref
            .ok_or_else(|| MoltenError::invalid_harness("reference callback returned no state ref"))?;
        history_material.push_str(&state_ref);
        history_material.push_str(&transition.record.selected.choice_id);
        let history_ref = blake3_ref(history_material.as_bytes());
        let port_event_ref = router
            .events()
            .last()
            .map(|event| event.event_ref.clone())
            .ok_or_else(|| MoltenError::invalid_harness("reference route emitted no canonical port event"))?;
        let observation = canonical_simulation_observation(SimulationObservation {
            sequence: step.sequence,
            node_id: step.node_id,
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
        choice_records.push(transition.record);
        scheduler = transition.next;
    }
    scheduler = finish_simulation_scheduler(&prepared.world.admitted, &scheduler)
        .map_err(|error| MoltenError::invalid_harness(format!("reference scheduler finish denied: {error:?}")))?;
    let mut final_state_refs = Vec::new();
    let mut host_evidence_refs = Vec::new();
    for host in prepared.hosts.values_mut() {
        host.drain(scheduler.virtual_tick)?;
        host.shutdown(scheduler.virtual_tick)?;
        final_state_refs.push(blake3_ref(format!("{:?}", host.executor().state()).as_bytes()));
        host_evidence_refs.extend(host.evidence().iter().map(|item| item.evidence_ref().to_string()));
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
        .map_err(|_| MoltenError::invalid_harness("reference choice resource count overflow"))?
        .checked_mul(RUN_RESOURCE_INCREMENT)
        .ok_or_else(|| MoltenError::invalid_harness("reference choice resource multiplication overflow"))?;
    let resource_units = router
        .resource_units()
        .checked_add(choice_resource_units)
        .ok_or_else(|| MoltenError::invalid_harness("reference run resource count overflow"))?;
    if resource_units > prepared.world.admitted.manifest.bounds.max_resource_units {
        return Err(MoltenError::invalid_harness("reference run exceeded its resource envelope"));
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
    let port_event_refs = router.events().iter().map(|item| item.event_ref.clone()).collect();
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
    })
}

fn reference_contract_differential(world: &CanonicalSimulatedWorld) -> Result<CanonicalSimulationDifferential> {
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
        ready_at_tick: step.sequence,
    }
}

fn workload_choice_id(sequence: u64) -> String {
    format!("workload-{sequence:0width$}", width = REFERENCE_CHOICE_ID_WIDTH)
}

fn reference_node_id(kind: ReferenceSystemKind) -> String {
    format!("node-{}", kind.as_str())
}

fn reference_invariants() -> Vec<SimulationInvariant> {
    let mut invariants =
        REQUIRED_UNIVERSAL_INVARIANTS.into_iter().map(SimulationInvariant::Universal).collect::<Vec<_>>();
    invariants.extend([
        SimulationInvariant::ExtensionSemantic {
            service: ReferenceSystemKind::TransactionalKeyValue,
            invariant_id: "transaction-version-monotonic".to_string(),
        },
        SimulationInvariant::ExtensionSemantic {
            service: ReferenceSystemKind::TransactionalKeyValue,
            invariant_id: "conflict-does-not-mutate".to_string(),
        },
        SimulationInvariant::ExtensionSemantic {
            service: ReferenceSystemKind::ReplicatedLog,
            invariant_id: "log-offsets-contiguous".to_string(),
        },
        SimulationInvariant::ExtensionSemantic {
            service: ReferenceSystemKind::ReplicatedLog,
            invariant_id: "retention-follows-replication".to_string(),
        },
        SimulationInvariant::ExtensionSemantic {
            service: ReferenceSystemKind::DistributedScheduler,
            invariant_id: "single-authoritative-completion".to_string(),
        },
        SimulationInvariant::ExtensionSemantic {
            service: ReferenceSystemKind::DistributedScheduler,
            invariant_id: "completion-requires-current-lease".to_string(),
        },
    ]);
    invariants
}

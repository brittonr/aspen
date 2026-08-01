use preserves::IOValue;

use super::*;
use crate::error::MoltenError;
use crate::error::Result;
use crate::preserves_rail::bool_value;
use crate::preserves_rail::canonical_hash;
use crate::preserves_rail::record;
use crate::preserves_rail::sequence;
use crate::preserves_rail::string;
use crate::preserves_rail::u64_value;

pub const FABRIC_SIMULATION_OBSERVATION_SCHEMA: &str = "molten.fabric-simulation.observation.v1";
pub const FABRIC_SIMULATION_PORT_EVENT_SCHEMA: &str = "molten.fabric-simulation.port-event.v1";
pub const FABRIC_SIMULATION_DIFFERENTIAL_SCHEMA: &str = "molten.fabric-simulation.differential.v1";
pub const FABRIC_SIMULATION_SHRINK_SCHEMA: &str = "molten.fabric-simulation.shrink.v1";
pub const FABRIC_SIMULATION_CLAIM_SCHEMA: &str = "molten.fabric-simulation.claim-profile.v1";
pub const MAX_CANONICAL_SIMULATION_ITEMS: usize = 4_096;
const RUN_READBACK_FIELD_COUNT: usize = 16;
const RUN_DECISION_FIELD_INDEX: usize = 1;
const RUN_PROFILE_FIELD_INDEX: usize = 2;
const RUN_CHOICE_COUNT_FIELD_INDEX: usize = 3;
const RUN_EVENT_COUNT_FIELD_INDEX: usize = 4;
const RUN_INVARIANT_COUNT_FIELD_INDEX: usize = 5;
const RUN_RESOURCE_UNITS_FIELD_INDEX: usize = 6;
const RUN_VIRTUAL_TICKS_FIELD_INDEX: usize = 7;
const RUN_WORLD_REF_FIELD_INDEX: usize = 8;
const RUN_FINAL_STATE_REFS_FIELD_INDEX: usize = 9;
const RUN_FIRST_DIVERGENCE_FIELD_INDEX: usize = 10;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalSimulatedWorld {
    pub world_ref: String,
    pub admitted: AdmittedSimulatedWorld,
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalSimulationObservation {
    pub observation_ref: String,
    pub observation: SimulationObservation,
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalSimulationPortEvent {
    pub event_ref: String,
    pub choice_position: u64,
    pub class: crate::fabric::FabricPortClass,
    pub port_id: String,
    pub request_ref: String,
    pub output_ref: String,
    pub fault: Option<SimulationFaultKind>,
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalSimulationRun {
    pub run_ref: String,
    pub world_ref: String,
    pub profile: SimulationClaimProfile,
    pub summary: SimulationRunSummary,
    pub observation_refs: Vec<String>,
    pub port_event_refs: Vec<String>,
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalSimulationReproBundle {
    pub bundle_ref: String,
    pub world_ref: String,
    pub run_ref: String,
    pub shrink_ref: Option<String>,
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalSimulationDifferential {
    pub report_ref: String,
    pub simulation_profile_ref: String,
    pub live_profile_ref: String,
    pub shared_contract_ref: String,
    pub equivalent: bool,
    pub normalized_difference_refs: Vec<String>,
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalSimulationShrink {
    pub shrink_ref: String,
    pub original_world_ref: String,
    pub shrunk_world_ref: String,
    pub result: ShrinkResult,
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalClaimPromotion {
    pub decision_ref: String,
    pub current: SimulationClaimProfile,
    pub decision: ClaimPromotionDecision,
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimulationRunReadback {
    pub decision: String,
    pub profile: String,
    pub choice_count: u64,
    pub event_count: u64,
    pub invariant_count: u64,
    pub resource_units: u64,
    pub virtual_ticks: u64,
    pub world_ref: String,
    pub final_state_refs: Vec<String>,
    pub first_divergence: Option<String>,
    pub run_ref: String,
}

// r[impl molten.fabric_simulation.world_manifest]
// r[impl molten.fabric_simulation.same_core]
pub fn canonical_admit_simulated_world(manifest: &SimulatedWorldManifest) -> Result<CanonicalSimulatedWorld> {
    let admitted = admit_simulated_world(manifest)
        .map_err(|issues| MoltenError::invalid_harness(format!("fabric simulation world denied: {issues:?}")))?;
    let value = world_value(&admitted.manifest);
    let world_ref = canonical_hash(&value)?;
    Ok(CanonicalSimulatedWorld {
        world_ref,
        admitted,
        value,
    })
}

// r[impl molten.fabric_simulation.invariants]
pub fn canonical_simulation_observation(observation: SimulationObservation) -> Result<CanonicalSimulationObservation> {
    for reference in [
        &observation.state_ref,
        &observation.history_ref,
        &observation.port_event_ref,
    ] {
        crate::preserves_rail::validate_content_ref(reference)?;
    }
    let value = record("fabric-simulation-observation-v1", vec![
        string(FABRIC_SIMULATION_OBSERVATION_SCHEMA),
        field("sequence", u64_value(observation.sequence)),
        field("node-id", string(&observation.node_id)),
        field("service", optional_string(observation.service.map(|service| service.as_str()))),
        field("generation", u64_value(observation.generation)),
        field("state-ref", string(&observation.state_ref)),
        field("history-ref", string(&observation.history_ref)),
        field("port-event-ref", string(&observation.port_event_ref)),
        field("ambient-effect", bool_value(observation.ambient_effect)),
        field("stale-generation-mutation", bool_value(observation.stale_generation_mutation)),
        field("resource-bound-bypass", bool_value(observation.resource_bound_bypass)),
        field("port-state-machine-violation", bool_value(observation.port_state_machine_violation)),
        field("terminal-cleanup-complete", bool_value(observation.terminal_cleanup_complete)),
        field(
            "semantic-invariants-passed",
            strings_value(observation.semantic_invariants_passed.iter().map(String::as_str)),
        ),
        checks(&[
            "redacted-canonical-observation",
            "generation-explicit",
            "state-and-history-ref-only",
            "secret-payload-excluded",
        ]),
    ]);
    let observation_ref = canonical_hash(&value)?;
    Ok(CanonicalSimulationObservation {
        observation_ref,
        observation,
        value,
    })
}

pub struct SimulationPortEventInput<'a> {
    pub choice_position: u64,
    pub class: crate::fabric::FabricPortClass,
    pub port_id: &'a str,
    pub request_ref: &'a str,
    pub output_ref: &'a str,
    pub fault: Option<SimulationFaultKind>,
}

// r[impl molten.fabric_simulation.port_substitution]
// r[impl molten.fabric_simulation.fault_model]
pub fn canonical_simulation_port_event(input: SimulationPortEventInput<'_>) -> Result<CanonicalSimulationPortEvent> {
    crate::preserves_rail::validate_content_ref(input.request_ref)?;
    crate::preserves_rail::validate_content_ref(input.output_ref)?;
    let value = record("fabric-simulation-port-event-v1", vec![
        string(FABRIC_SIMULATION_PORT_EVENT_SCHEMA),
        field("choice-position", u64_value(input.choice_position)),
        field("class", string(input.class.as_str())),
        field("port-id", string(input.port_id)),
        field("request-ref", string(input.request_ref)),
        field("output-ref", string(input.output_ref)),
        field("fault", optional_string(input.fault.map(SimulationFaultKind::as_str))),
        checks(&[
            "named-port-boundary",
            "direct-extension-state-mutation-denied",
            "deterministic-output-ref",
            "adapter-event-is-not-live-evidence",
        ]),
    ]);
    let event_ref = canonical_hash(&value)?;
    Ok(CanonicalSimulationPortEvent {
        event_ref,
        choice_position: input.choice_position,
        class: input.class,
        port_id: input.port_id.to_string(),
        request_ref: input.request_ref.to_string(),
        output_ref: input.output_ref.to_string(),
        fault: input.fault,
        value,
    })
}

// r[impl molten.fabric_simulation.scheduler]
// r[impl molten.fabric_simulation.invariants]
// r[impl molten.fabric_simulation.evidence]
pub fn canonical_simulation_run(
    world_ref: &str,
    profile: SimulationClaimProfile,
    summary: SimulationRunSummary,
    observation_refs: Vec<String>,
    port_event_refs: Vec<String>,
) -> Result<CanonicalSimulationRun> {
    crate::preserves_rail::validate_content_ref(world_ref)?;
    validate_refs("observation", &observation_refs)?;
    validate_refs("port-event", &port_event_refs)?;
    validate_refs("final-state", &summary.final_state_refs)?;
    validate_count("choice records", summary.choice_records.len())?;
    validate_count("invariant results", summary.invariant_results.len())?;
    validate_count("observation refs", observation_refs.len())?;
    validate_count("port event refs", port_event_refs.len())?;
    let choice_values = summary.choice_records.iter().map(choice_record_value).collect();
    let invariant_values = summary.invariant_results.iter().map(invariant_result_value).collect();
    let value = record("fabric-simulation-run-v1", vec![
        string(FABRIC_SIMULATION_RUN_SCHEMA),
        field("decision", string(summary.decision.as_str())),
        field("profile", string(profile.as_str())),
        field("choice-count", u64_len(summary.choice_records.len())?),
        field("event-count", u64_len(observation_refs.len())?),
        field("invariant-count", u64_len(summary.invariant_results.len())?),
        field("resource-units", u64_value(summary.resource_units)),
        field("virtual-ticks", u64_value(summary.virtual_ticks)),
        field("world-ref", string(world_ref)),
        field("final-state-refs", strings_value(summary.final_state_refs.iter().map(String::as_str))),
        field("first-divergence", optional_divergence_value(summary.first_divergence.as_ref())),
        field("choices", sequence(choice_values)),
        field("invariants", sequence(invariant_values)),
        field("observation-refs", strings_value(observation_refs.iter().map(String::as_str))),
        field("port-event-refs", strings_value(port_event_refs.iter().map(String::as_str))),
        checks(&[
            "world-bound",
            "single-choice-stream",
            "invariants-bounded",
            "secret-payload-excluded",
            "profile-non-claims-preserved",
        ]),
    ]);
    let run_ref = canonical_hash(&value)?;
    Ok(CanonicalSimulationRun {
        run_ref,
        world_ref: world_ref.to_string(),
        profile,
        summary,
        observation_refs,
        port_event_refs,
        value,
    })
}

// r[impl molten.fabric_simulation.live_sim_differential]
pub fn canonical_simulation_differential(
    simulation_profile_ref: &str,
    live_profile_ref: &str,
    shared_contract_ref: &str,
    simulation_trace_refs: &[String],
    live_trace_refs: &[String],
    normalized_difference_refs: Vec<String>,
) -> Result<CanonicalSimulationDifferential> {
    for reference in [simulation_profile_ref, live_profile_ref, shared_contract_ref] {
        crate::preserves_rail::validate_content_ref(reference)?;
    }
    validate_refs("simulation differential", simulation_trace_refs)?;
    validate_refs("live differential", live_trace_refs)?;
    validate_refs("normalized difference", &normalized_difference_refs)?;
    let equivalent = simulation_trace_refs == live_trace_refs && normalized_difference_refs.is_empty();
    let value = record("fabric-simulation-differential-v1", vec![
        string(FABRIC_SIMULATION_DIFFERENTIAL_SCHEMA),
        field("simulation-profile-ref", string(simulation_profile_ref)),
        field("live-profile-ref", string(live_profile_ref)),
        field("shared-contract-ref", string(shared_contract_ref)),
        field("simulation-trace-refs", strings_value(simulation_trace_refs.iter().map(String::as_str))),
        field("live-trace-refs", strings_value(live_trace_refs.iter().map(String::as_str))),
        field("normalized-difference-refs", strings_value(normalized_difference_refs.iter().map(String::as_str))),
        field("equivalent", bool_value(equivalent)),
        checks(&[
            "shared-port-contract",
            "declared-capability-differences-visible",
            "no-live-production-equivalence-claim",
        ]),
    ]);
    let report_ref = canonical_hash(&value)?;
    Ok(CanonicalSimulationDifferential {
        report_ref,
        simulation_profile_ref: simulation_profile_ref.to_string(),
        live_profile_ref: live_profile_ref.to_string(),
        shared_contract_ref: shared_contract_ref.to_string(),
        equivalent,
        normalized_difference_refs,
        value,
    })
}

// r[impl molten.fabric_simulation.claim_ladder]
pub fn canonical_claim_promotion(
    current: SimulationClaimProfile,
    target: SimulationClaimProfile,
    evidence: &ClaimEvidence,
) -> Result<CanonicalClaimPromotion> {
    let decision = evaluate_claim_promotion(current, target, evidence);
    let value = record("fabric-simulation-claim-profile-v1", vec![
        string(FABRIC_SIMULATION_CLAIM_SCHEMA),
        field("current-profile", string(current.as_str())),
        field("target-profile", string(target.as_str())),
        field("evidence-profile", string(evidence.profile.as_str())),
        field("admitted", bool_value(decision.admitted)),
        field("missing-evidence", strings_value(decision.missing_evidence.iter().copied())),
        checks(&[
            "profile-specific-evidence-required",
            "stronger-profile-cannot-use-simulation-label",
            "decision-does-not-grant-runtime-authority",
        ]),
    ]);
    let decision_ref = canonical_hash(&value)?;
    Ok(CanonicalClaimPromotion {
        decision_ref,
        current,
        decision,
        value,
    })
}

// r[impl molten.fabric_simulation.replay_shrink]
pub fn canonical_simulation_shrink(
    original_world_ref: &str,
    shrunk_world: &CanonicalSimulatedWorld,
    result: ShrinkResult,
) -> Result<CanonicalSimulationShrink> {
    crate::preserves_rail::validate_content_ref(original_world_ref)?;
    let value = record("fabric-simulation-shrink-v1", vec![
        string(FABRIC_SIMULATION_SHRINK_SCHEMA),
        field("original-world-ref", string(original_world_ref)),
        field("shrunk-world-ref", string(&shrunk_world.world_ref)),
        field("attempts", u64_value(result.attempts)),
        field("removed-workload-steps", u64_value(result.removed_workload_steps)),
        field("failure-preserved", bool_value(result.failure_preserved)),
        checks(&[
            "candidate-replayed-from-initial-world",
            "invalid-candidates-rejected",
            "failure-class-preserved",
        ]),
    ]);
    let shrink_ref = canonical_hash(&value)?;
    Ok(CanonicalSimulationShrink {
        shrink_ref,
        original_world_ref: original_world_ref.to_string(),
        shrunk_world_ref: shrunk_world.world_ref.clone(),
        result,
        value,
    })
}

// r[impl molten.fabric_simulation.evidence]
pub fn canonical_simulation_repro_bundle(
    world: &CanonicalSimulatedWorld,
    run: &CanonicalSimulationRun,
    shrink: Option<&CanonicalSimulationShrink>,
) -> Result<CanonicalSimulationReproBundle> {
    if run.world_ref != world.world_ref {
        return Err(MoltenError::invalid_harness("simulation repro run does not bind the supplied world"));
    }
    let value = record("fabric-simulation-repro-v1", vec![
        string(FABRIC_SIMULATION_REPRO_SCHEMA),
        field("world-ref", string(&world.world_ref)),
        field("run-ref", string(&run.run_ref)),
        field("shrink-ref", optional_string(shrink.map(|item| item.shrink_ref.as_str()))),
        field("profile", string(run.profile.as_str())),
        field("non-claims", strings_value(world.admitted.manifest.non_claims.iter().map(|item| item.as_str()))),
        checks(&[
            "offline-verifiable-input-closure",
            "bounded-evidence-members",
            "secret-payload-excluded",
            "simulation-not-relabeled-live",
        ]),
    ]);
    let bundle_ref = canonical_hash(&value)?;
    Ok(CanonicalSimulationReproBundle {
        bundle_ref,
        world_ref: world.world_ref.clone(),
        run_ref: run.run_ref.clone(),
        shrink_ref: shrink.map(|item| item.shrink_ref.clone()),
        value,
    })
}

// r[impl molten.fabric_simulation.operator_workflow]
pub fn parse_simulation_run_readback(value: &IOValue) -> Result<SimulationRunReadback> {
    let fields = value
        .collect_simple_record("fabric-simulation-run-v1", Some(RUN_READBACK_FIELD_COUNT))
        .ok_or_else(|| MoltenError::invalid_harness("expected canonical fabric-simulation run"))?;
    let schema = required_string(&fields[0], "simulation run schema")?;
    if schema != FABRIC_SIMULATION_RUN_SCHEMA {
        return Err(MoltenError::invalid_harness(format!("fabric-simulation run schema mismatch: {schema}")));
    }
    let decision = record_string_field(&fields[RUN_DECISION_FIELD_INDEX], "decision")?;
    let profile = record_string_field(&fields[RUN_PROFILE_FIELD_INDEX], "profile")?;
    if !matches!(decision.as_str(), "pass" | "invariant-failed" | "diverged" | "bound-exceeded" | "denied") {
        return Err(MoltenError::invalid_harness(format!("unsupported simulation decision: {decision}")));
    }
    if !matches!(
        profile.as_str(),
        "pure-model" | "deterministic-whole-system" | "multi-process-live" | "host-chaos" | "vm-hardware"
    ) {
        return Err(MoltenError::invalid_harness(format!("unsupported simulation profile: {profile}")));
    }
    let world_ref = record_string_field(&fields[RUN_WORLD_REF_FIELD_INDEX], "world-ref")?;
    crate::preserves_rail::validate_content_ref(&world_ref)?;
    let final_state_refs = record_string_sequence_field(&fields[RUN_FINAL_STATE_REFS_FIELD_INDEX], "final-state-refs")?;
    validate_refs("readback final state", &final_state_refs)?;
    Ok(SimulationRunReadback {
        decision,
        profile,
        choice_count: record_u64_field(&fields[RUN_CHOICE_COUNT_FIELD_INDEX], "choice-count")?,
        event_count: record_u64_field(&fields[RUN_EVENT_COUNT_FIELD_INDEX], "event-count")?,
        invariant_count: record_u64_field(&fields[RUN_INVARIANT_COUNT_FIELD_INDEX], "invariant-count")?,
        resource_units: record_u64_field(&fields[RUN_RESOURCE_UNITS_FIELD_INDEX], "resource-units")?,
        virtual_ticks: record_u64_field(&fields[RUN_VIRTUAL_TICKS_FIELD_INDEX], "virtual-ticks")?,
        world_ref,
        final_state_refs,
        first_divergence: record_optional_divergence(&fields[RUN_FIRST_DIVERGENCE_FIELD_INDEX])?,
        run_ref: canonical_hash(value)?,
    })
}

fn world_value(world: &SimulatedWorldManifest) -> IOValue {
    record("fabric-simulation-world-v1", vec![
        string(FABRIC_SIMULATION_WORLD_SCHEMA),
        field("runtime-ref", string(&world.runtime_ref)),
        field("scheduler-input-ref", string(&world.scheduler_input_ref)),
        field("entropy-input-ref", string(&world.entropy_input_ref)),
        field("authority-ref", string(&world.authority_ref)),
        field("policy-ref", string(&world.policy_ref)),
        field("initial-durable-state-ref", string(&world.initial_durable_state_ref)),
        field("resource-profile-ref", string(&world.resource_profile_ref)),
        field("workload-ref", string(&world.workload_ref)),
        field("fault-plan-ref", string(&world.fault_plan_ref)),
        field("invariant-set-ref", string(&world.invariant_set_ref)),
        field("nodes", sequence(world.nodes.iter().map(node_value).collect())),
        field("port-profiles", sequence(world.port_profiles.iter().map(port_profile_value).collect())),
        field("workload", sequence(world.workload.iter().map(workload_step_value).collect())),
        field("faults", sequence(world.faults.iter().map(fault_value).collect())),
        field("invariants", sequence(world.invariants.iter().map(invariant_value).collect())),
        field("bounds", bounds_value(&world.bounds)),
        field("claim-profile", string(world.claim_profile.as_str())),
        field("non-claims", strings_value(world.non_claims.iter().map(|claim| claim.as_str()))),
        field("ambient-inputs", strings_value(world.ambient_inputs.iter().map(String::as_str))),
        checks(&[
            "behavior-inputs-closed",
            "same-extension-core-bound",
            "ports-explicit",
            "exploration-bounded",
            "ambient-inputs-denied",
            "claim-profile-explicit",
        ]),
    ])
}

fn node_value(node: &SimulatedNode) -> IOValue {
    record("fabric-simulation-node-v1", vec![
        field("node-id", string(&node.node_id)),
        field("extension-id", string(&node.extension_id)),
        field("service-id", string(&node.service_id)),
        field("generation", u64_value(node.generation)),
        field("initial-state-ref", string(&node.initial_state_ref)),
        field("membership-view-ref", string(&node.membership_view_ref)),
        field("placement-ref", string(&node.placement_ref)),
        field("consistency-profile-ref", string(&node.consistency_profile_ref)),
        field("simulation-core", core_identity_value(&node.same_core.simulation)),
        field("live-core", core_identity_value(&node.same_core.live)),
        field(
            "required-port-classes",
            strings_value(node.required_port_classes.iter().map(|class| class.as_str())),
        ),
    ])
}

fn core_identity_value(identity: &ExtensionCoreIdentity) -> IOValue {
    record("fabric-simulation-core-identity-v1", vec![
        field("implementation-ref", string(&identity.implementation_ref)),
        field("manifest-ref", string(&identity.manifest_ref)),
        field("callback-dispatcher-ref", string(&identity.callback_dispatcher_ref)),
        field("protocol-core-ref", string(&identity.protocol_core_ref)),
        field("state-machine-ref", string(&identity.state_machine_ref)),
        field("schema-set-ref", string(&identity.schema_set_ref)),
        field("port-contract-set-ref", string(&identity.port_contract_set_ref)),
    ])
}

fn port_profile_value(profile: &SimulatedPortProfile) -> IOValue {
    record("fabric-simulation-port-profile-v1", vec![
        field("class", string(profile.class.as_str())),
        field("port-id", string(&profile.port_id)),
        field("version", string(&profile.version)),
        field("implementation-profile", string(&profile.implementation_profile)),
        field("descriptor-ref", string(&profile.descriptor_ref)),
        field("command-schema-ref", string(&profile.command_schema_ref)),
        field("event-schema-ref", string(&profile.event_schema_ref)),
        field("deterministic", bool_value(profile.deterministic)),
        field("declared-faults", strings_value(profile.declared_faults.iter().map(|fault| fault.as_str()))),
    ])
}

fn workload_step_value(step: &SimulationWorkloadStep) -> IOValue {
    record("fabric-simulation-workload-step-v1", vec![
        field("sequence", u64_value(step.sequence)),
        field("node-id", string(&step.node_id)),
        field("request-ref", string(&step.request_ref)),
        field("service", string(step.service.as_str())),
        field("expected-failure-class", optional_string(step.expected_failure_class.as_deref())),
    ])
}

fn fault_value(fault: &SimulationFaultAction) -> IOValue {
    record("fabric-simulation-fault-v1", vec![
        field("fault-id", string(&fault.fault_id)),
        field("kind", string(fault.kind.as_str())),
        field("target", string(&fault.target)),
        field("boundary", string(fault.boundary.as_str())),
        field("activate-at-choice", u64_value(fault.activate_at_choice)),
        field("duration-choices", optional_u64(fault.duration_choices)),
        field("resource-cost", u64_value(fault.resource_cost)),
        field("expected-observation", string(&fault.expected_observation)),
        field("direct-extension-state-mutation", bool_value(fault.direct_extension_state_mutation)),
    ])
}

fn invariant_value(invariant: &SimulationInvariant) -> IOValue {
    match invariant {
        SimulationInvariant::Universal(kind) => {
            record("universal-invariant", vec![field("kind", string(kind.as_str()))])
        }
        SimulationInvariant::ExtensionSemantic { service, invariant_id } => record("extension-invariant", vec![
            field("service", string(service.as_str())),
            field("invariant-id", string(invariant_id)),
        ]),
    }
}

fn bounds_value(bounds: &SimulationBounds) -> IOValue {
    record("fabric-simulation-bounds-v1", vec![
        field("max-choices", u64_value(bounds.max_choices)),
        field("max-events", u64_value(bounds.max_events)),
        field("max-virtual-ticks", u64_value(bounds.max_virtual_ticks)),
        field("max-trace-bytes", u64_value(bounds.max_trace_bytes)),
        field("max-resource-units", u64_value(bounds.max_resource_units)),
        field("max-shrink-attempts", u64_value(bounds.max_shrink_attempts)),
    ])
}

fn choice_record_value(record_value: &SchedulerChoiceRecord) -> IOValue {
    record("fabric-simulation-choice-v1", vec![
        field("position", u64_value(record_value.position)),
        field("virtual-tick", u64_value(record_value.virtual_tick)),
        field("eligible", sequence(record_value.eligible.iter().map(eligible_choice_value).collect())),
        field("selected", eligible_choice_value(&record_value.selected)),
    ])
}

fn eligible_choice_value(choice: &EligibleChoice) -> IOValue {
    record("eligible-choice-v1", vec![
        field("kind", string(choice.kind.as_str())),
        field("choice-id", string(&choice.choice_id)),
        field("node-id", string(&choice.node_id)),
        field("generation", u64_value(choice.generation)),
        field("ready-at-tick", u64_value(choice.ready_at_tick)),
    ])
}

fn invariant_result_value(result: &InvariantResult) -> IOValue {
    record("fabric-simulation-invariant-result-v1", vec![
        field("invariant", invariant_value(&result.invariant)),
        field("passed", bool_value(result.passed)),
        field("first-failure-sequence", optional_u64(result.first_failure_sequence)),
    ])
}

fn optional_divergence_value(divergence: Option<&ReplayDivergence>) -> IOValue {
    match divergence {
        None => record("none", Vec::new()),
        Some(divergence) => record("some", vec![record("fabric-simulation-divergence-v1", vec![
            field("position", u64_value(divergence.position)),
            field("expected-choice-id", string(&divergence.expected_choice_id)),
            field("eligible-choice-ids", strings_value(divergence.eligible_choice_ids.iter().map(String::as_str))),
            field("diagnostic", string(&divergence.diagnostic)),
        ])]),
    }
}

fn record_optional_divergence(value: &preserves::Value<IOValue>) -> Result<Option<String>> {
    let field_value = named_field_value(value, "first-divergence")?;
    if field_value.collect_simple_record("none", Some(0)).is_some() {
        return Ok(None);
    }
    let some = field_value
        .collect_simple_record("some", Some(1))
        .ok_or_else(|| MoltenError::invalid_harness("expected optional simulation divergence"))?;
    Ok(Some(canonical_hash((&some[0]).into())?))
}

fn validate_count(label: &str, actual: usize) -> Result<()> {
    if actual > MAX_CANONICAL_SIMULATION_ITEMS {
        Err(MoltenError::invalid_harness(format!(
            "fabric simulation {label} count {actual} exceeds {MAX_CANONICAL_SIMULATION_ITEMS}"
        )))
    } else {
        Ok(())
    }
}

fn validate_refs(label: &str, refs: &[String]) -> Result<()> {
    validate_count(label, refs.len())?;
    for reference in refs {
        crate::preserves_rail::validate_content_ref(reference)?;
    }
    Ok(())
}

fn u64_len(value: usize) -> Result<IOValue> {
    let converted = u64::try_from(value)
        .map_err(|_| MoltenError::invalid_harness("fabric simulation collection length overflow"))?;
    Ok(u64_value(converted))
}

fn field(label: &'static str, value: IOValue) -> IOValue {
    record(label, vec![value])
}

fn checks(values: &[&str]) -> IOValue {
    field("checks", strings_value(values.iter().copied()))
}

fn strings_value<'a>(values: impl IntoIterator<Item = &'a str>) -> IOValue {
    sequence(values.into_iter().map(string).collect())
}

fn optional_string(value: Option<&str>) -> IOValue {
    match value {
        Some(value) => record("some", vec![string(value)]),
        None => record("none", Vec::new()),
    }
}

fn optional_u64(value: Option<u64>) -> IOValue {
    match value {
        Some(value) => record("some", vec![u64_value(value)]),
        None => record("none", Vec::new()),
    }
}

fn record_string_field(value: &preserves::Value<IOValue>, label: &str) -> Result<String> {
    required_string(&named_field_value(value, label)?, label)
}

fn record_u64_field(value: &preserves::Value<IOValue>, label: &str) -> Result<u64> {
    named_field_value(value, label)?
        .as_u64()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected u64 for {label}")))?
        .map_err(|error| MoltenError::invalid_harness(format!("u64 out of range for {label}: {error}")))
}

fn record_string_sequence_field(value: &preserves::Value<IOValue>, label: &str) -> Result<Vec<String>> {
    let field_value = named_field_value(value, label)?;
    let sequence = field_value
        .as_sequence()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected sequence for {label}")))?;
    sequence.map(|item| required_string(&item, label)).collect()
}

fn named_field_value(value: &preserves::Value<IOValue>, label: &str) -> Result<preserves::Value<IOValue>> {
    let fields = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected named field {label}")))?;
    Ok(fields[0].clone())
}

fn required_string(value: &preserves::Value<IOValue>, label: &str) -> Result<String> {
    value
        .as_string()
        .map(|value| value.into_owned())
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected string for {label}")))
}

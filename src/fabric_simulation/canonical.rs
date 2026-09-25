use super::*;

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
    pub value: preserves::IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalSimulationObservation {
    pub observation_ref: String,
    pub observation: SimulationObservation,
    pub value: preserves::IOValue,
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
    pub value: preserves::IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalSimulationRun {
    pub run_ref: String,
    pub world_ref: String,
    pub profile: SimulationClaimProfile,
    pub summary: SimulationRunSummary,
    pub observation_refs: Vec<String>,
    pub port_event_refs: Vec<String>,
    pub value: preserves::IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalSimulationReproBundle {
    pub bundle_ref: String,
    pub world_ref: String,
    pub run_ref: String,
    pub shrink_ref: Option<String>,
    pub value: preserves::IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalSimulationDifferential {
    pub report_ref: String,
    pub simulation_profile_ref: String,
    pub live_profile_ref: String,
    pub shared_contract_ref: String,
    pub equivalent: bool,
    pub normalized_difference_refs: Vec<String>,
    pub value: preserves::IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalSimulationShrink {
    pub shrink_ref: String,
    pub original_world_ref: String,
    pub shrunk_world_ref: String,
    pub result: ShrinkResult,
    pub value: preserves::IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalClaimPromotion {
    pub decision_ref: String,
    pub current: SimulationClaimProfile,
    pub decision: ClaimPromotionDecision,
    pub value: preserves::IOValue,
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
pub fn canonical_admit_simulated_world(
    manifest: &SimulatedWorldManifest,
) -> crate::error::Result<CanonicalSimulatedWorld> {
    let admitted = admit_simulated_world(manifest).map_err(|issues| {
        crate::error::MoltenError::invalid_harness(format!("fabric simulation world denied: {issues:?}"))
    })?;
    let value = world_value(&admitted.manifest);
    let world_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(CanonicalSimulatedWorld {
        world_ref,
        admitted,
        value,
    })
}

// r[impl molten.fabric_simulation.invariants]
pub fn canonical_simulation_observation(
    observation: SimulationObservation,
) -> crate::error::Result<CanonicalSimulationObservation> {
    for reference in [
        &observation.state_ref,
        &observation.history_ref,
        &observation.port_event_ref,
    ] {
        crate::preserves_rail::validate_content_ref(reference)?;
    }
    let value = crate::preserves_rail::record("fabric-simulation-observation-v1", vec![
        crate::preserves_rail::string(FABRIC_SIMULATION_OBSERVATION_SCHEMA),
        field("sequence", crate::preserves_rail::u64_value(observation.sequence)),
        field("node-id", crate::preserves_rail::string(&observation.node_id)),
        field("service", optional_string(observation.service.map(|service| service.as_str()))),
        field("generation", crate::preserves_rail::u64_value(observation.generation)),
        field("state-ref", crate::preserves_rail::string(&observation.state_ref)),
        field("history-ref", crate::preserves_rail::string(&observation.history_ref)),
        field("port-event-ref", crate::preserves_rail::string(&observation.port_event_ref)),
        field("ambient-effect", crate::preserves_rail::bool_value(observation.ambient_effect)),
        field(
            "stale-generation-mutation",
            crate::preserves_rail::bool_value(observation.stale_generation_mutation),
        ),
        field("resource-bound-bypass", crate::preserves_rail::bool_value(observation.resource_bound_bypass)),
        field(
            "port-state-machine-violation",
            crate::preserves_rail::bool_value(observation.port_state_machine_violation),
        ),
        field(
            "terminal-cleanup-complete",
            crate::preserves_rail::bool_value(observation.terminal_cleanup_complete),
        ),
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
    let observation_ref = crate::preserves_rail::canonical_hash(&value)?;
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
pub fn canonical_simulation_port_event(
    input: SimulationPortEventInput<'_>,
) -> crate::error::Result<CanonicalSimulationPortEvent> {
    crate::preserves_rail::validate_content_ref(input.request_ref)?;
    crate::preserves_rail::validate_content_ref(input.output_ref)?;
    let value = crate::preserves_rail::record("fabric-simulation-port-event-v1", vec![
        crate::preserves_rail::string(FABRIC_SIMULATION_PORT_EVENT_SCHEMA),
        field("choice-position", crate::preserves_rail::u64_value(input.choice_position)),
        field("class", crate::preserves_rail::string(input.class.as_str())),
        field("port-id", crate::preserves_rail::string(input.port_id)),
        field("request-ref", crate::preserves_rail::string(input.request_ref)),
        field("output-ref", crate::preserves_rail::string(input.output_ref)),
        field("fault", optional_string(input.fault.map(SimulationFaultKind::as_str))),
        checks(&[
            "named-port-boundary",
            "direct-extension-state-mutation-denied",
            "deterministic-output-ref",
            "adapter-event-is-not-live-evidence",
        ]),
    ]);
    let event_ref = crate::preserves_rail::canonical_hash(&value)?;
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
) -> crate::error::Result<CanonicalSimulationRun> {
    crate::preserves_rail::validate_content_ref(world_ref)?;
    validate_refs("observation", &observation_refs)?;
    validate_refs("port-event", &port_event_refs)?;
    validate_refs("final-state", &summary.final_state_refs)?;
    validate_count("choice records", summary.choice_records.len())?;
    validate_count("invariant results", summary.invariant_results.len())?;
    validate_count("observation refs", observation_refs.len())?;
    validate_count("port event refs", port_event_refs.len())?;
    let choice_values =
        summary.choice_records.iter().map(choice_record_value).collect::<crate::error::Result<Vec<_>>>()?;
    let invariant_values = summary.invariant_results.iter().map(invariant_result_value).collect();
    let value = crate::preserves_rail::record("fabric-simulation-run-v1", vec![
        crate::preserves_rail::string(FABRIC_SIMULATION_RUN_SCHEMA),
        field("decision", crate::preserves_rail::string(summary.decision.as_str())),
        field("profile", crate::preserves_rail::string(profile.as_str())),
        field("choice-count", u64_len(summary.choice_records.len())?),
        field("event-count", u64_len(observation_refs.len())?),
        field("invariant-count", u64_len(summary.invariant_results.len())?),
        field("resource-units", crate::preserves_rail::u64_value(summary.resource_units)),
        field("virtual-ticks", crate::preserves_rail::u64_value(summary.virtual_ticks)),
        field("world-ref", crate::preserves_rail::string(world_ref)),
        field("final-state-refs", strings_value(summary.final_state_refs.iter().map(String::as_str))),
        field("first-divergence", optional_divergence_value(summary.first_divergence.as_ref())),
        field("choices", crate::preserves_rail::sequence(choice_values)),
        field("invariants", crate::preserves_rail::sequence(invariant_values)),
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
    let run_ref = crate::preserves_rail::canonical_hash(&value)?;
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

pub struct DifferentialInput<'a> {
    pub simulation_profile_ref: &'a str,
    pub live_profile_ref: &'a str,
    pub shared_contract_ref: &'a str,
    pub simulation_trace_refs: &'a [String],
    pub live_trace_refs: &'a [String],
    pub normalized_difference_refs: Vec<String>,
}

// r[impl molten.fabric_simulation.live_sim_differential]
pub fn canonical_simulation_differential(
    input: DifferentialInput<'_>,
) -> crate::error::Result<CanonicalSimulationDifferential> {
    let DifferentialInput {
        simulation_profile_ref,
        live_profile_ref,
        shared_contract_ref,
        simulation_trace_refs,
        live_trace_refs,
        normalized_difference_refs,
    } = input;
    for reference in [simulation_profile_ref, live_profile_ref, shared_contract_ref] {
        crate::preserves_rail::validate_content_ref(reference)?;
    }
    validate_refs("simulation differential", simulation_trace_refs)?;
    validate_refs("live differential", live_trace_refs)?;
    validate_refs("normalized difference", &normalized_difference_refs)?;
    let is_equivalent = simulation_trace_refs == live_trace_refs && normalized_difference_refs.is_empty();
    let value = crate::preserves_rail::record("fabric-simulation-differential-v1", vec![
        crate::preserves_rail::string(FABRIC_SIMULATION_DIFFERENTIAL_SCHEMA),
        field("simulation-profile-ref", crate::preserves_rail::string(simulation_profile_ref)),
        field("live-profile-ref", crate::preserves_rail::string(live_profile_ref)),
        field("shared-contract-ref", crate::preserves_rail::string(shared_contract_ref)),
        field("simulation-trace-refs", strings_value(simulation_trace_refs.iter().map(String::as_str))),
        field("live-trace-refs", strings_value(live_trace_refs.iter().map(String::as_str))),
        field("normalized-difference-refs", strings_value(normalized_difference_refs.iter().map(String::as_str))),
        field("equivalent", crate::preserves_rail::bool_value(is_equivalent)),
        checks(&[
            "shared-port-contract",
            "declared-capability-differences-visible",
            "no-live-production-equivalence-claim",
        ]),
    ]);
    let report_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(CanonicalSimulationDifferential {
        report_ref,
        simulation_profile_ref: simulation_profile_ref.to_string(),
        live_profile_ref: live_profile_ref.to_string(),
        shared_contract_ref: shared_contract_ref.to_string(),
        equivalent: is_equivalent,
        normalized_difference_refs,
        value,
    })
}

// r[impl molten.fabric_simulation.claim_ladder]
pub fn canonical_claim_promotion(
    current: SimulationClaimProfile,
    target: SimulationClaimProfile,
    evidence: &ClaimEvidence,
) -> crate::error::Result<CanonicalClaimPromotion> {
    let decision = evaluate_claim_promotion(current, target, evidence);
    let value = crate::preserves_rail::record("fabric-simulation-claim-profile-v1", vec![
        crate::preserves_rail::string(FABRIC_SIMULATION_CLAIM_SCHEMA),
        field("current-profile", crate::preserves_rail::string(current.as_str())),
        field("target-profile", crate::preserves_rail::string(target.as_str())),
        field("evidence-profile", crate::preserves_rail::string(evidence.profile.as_str())),
        field("admitted", crate::preserves_rail::bool_value(decision.admitted)),
        field("missing-evidence", strings_value(decision.missing_evidence.iter().copied())),
        checks(&[
            "profile-specific-evidence-required",
            "stronger-profile-cannot-use-simulation-label",
            "decision-does-not-grant-runtime-authority",
        ]),
    ]);
    let decision_ref = crate::preserves_rail::canonical_hash(&value)?;
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
) -> crate::error::Result<CanonicalSimulationShrink> {
    crate::preserves_rail::validate_content_ref(original_world_ref)?;
    let value = crate::preserves_rail::record("fabric-simulation-shrink-v1", vec![
        crate::preserves_rail::string(FABRIC_SIMULATION_SHRINK_SCHEMA),
        field("original-world-ref", crate::preserves_rail::string(original_world_ref)),
        field("shrunk-world-ref", crate::preserves_rail::string(&shrunk_world.world_ref)),
        field("attempts", crate::preserves_rail::u64_value(result.attempts)),
        field("removed-workload-steps", crate::preserves_rail::u64_value(result.removed_workload_steps)),
        field("failure-preserved", crate::preserves_rail::bool_value(result.failure_preserved)),
        checks(&[
            "candidate-replayed-from-initial-world",
            "invalid-candidates-rejected",
            "failure-class-preserved",
        ]),
    ]);
    let shrink_ref = crate::preserves_rail::canonical_hash(&value)?;
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
) -> crate::error::Result<CanonicalSimulationReproBundle> {
    if run.world_ref != world.world_ref {
        return Err(crate::error::MoltenError::invalid_harness(
            "simulation repro run does not bind the supplied world",
        ));
    }
    let value = crate::preserves_rail::record("fabric-simulation-repro-v1", vec![
        crate::preserves_rail::string(FABRIC_SIMULATION_REPRO_SCHEMA),
        field("world-ref", crate::preserves_rail::string(&world.world_ref)),
        field("run-ref", crate::preserves_rail::string(&run.run_ref)),
        field("shrink-ref", optional_string(shrink.map(|item| item.shrink_ref.as_str()))),
        field("profile", crate::preserves_rail::string(run.profile.as_str())),
        field("non-claims", strings_value(world.admitted.manifest.non_claims.iter().map(|item| item.as_str()))),
        checks(&[
            "offline-verifiable-input-closure",
            "bounded-evidence-members",
            "secret-payload-excluded",
            "simulation-not-relabeled-live",
        ]),
    ]);
    let bundle_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(CanonicalSimulationReproBundle {
        bundle_ref,
        world_ref: world.world_ref.clone(),
        run_ref: run.run_ref.clone(),
        shrink_ref: shrink.map(|item| item.shrink_ref.clone()),
        value,
    })
}

// r[impl molten.fabric_simulation.operator_workflow]
pub fn parse_simulation_run_readback(value: &preserves::IOValue) -> crate::error::Result<SimulationRunReadback> {
    let fields = value
        .collect_simple_record("fabric-simulation-run-v1", Some(RUN_READBACK_FIELD_COUNT))
        .ok_or_else(|| crate::error::MoltenError::invalid_harness("expected canonical fabric-simulation run"))?;
    let schema = required_string(&fields[0], "simulation run schema")?;
    if schema != FABRIC_SIMULATION_RUN_SCHEMA {
        return Err(crate::error::MoltenError::invalid_harness(format!(
            "fabric-simulation run schema mismatch: {schema}"
        )));
    }
    let decision = record_string_field(&fields[RUN_DECISION_FIELD_INDEX], "decision")?;
    let profile = record_string_field(&fields[RUN_PROFILE_FIELD_INDEX], "profile")?;
    if !matches!(decision.as_str(), "pass" | "invariant-failed" | "diverged" | "bound-exceeded" | "denied") {
        return Err(crate::error::MoltenError::invalid_harness(format!("unsupported simulation decision: {decision}")));
    }
    if !matches!(
        profile.as_str(),
        "pure-model" | "deterministic-whole-system" | "multi-process-live" | "host-chaos" | "vm-hardware"
    ) {
        return Err(crate::error::MoltenError::invalid_harness(format!("unsupported simulation profile: {profile}")));
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
        run_ref: crate::preserves_rail::canonical_hash(value)?,
    })
}

fn world_value(world: &SimulatedWorldManifest) -> preserves::IOValue {
    crate::preserves_rail::record("fabric-simulation-world-v1", vec![
        crate::preserves_rail::string(FABRIC_SIMULATION_WORLD_SCHEMA),
        field("runtime-ref", crate::preserves_rail::string(&world.runtime_ref)),
        field("scheduler-input-ref", crate::preserves_rail::string(&world.scheduler_input_ref)),
        field("entropy-input-ref", crate::preserves_rail::string(&world.entropy_input_ref)),
        field("authority-ref", crate::preserves_rail::string(&world.authority_ref)),
        field("policy-ref", crate::preserves_rail::string(&world.policy_ref)),
        field("initial-durable-state-ref", crate::preserves_rail::string(&world.initial_durable_state_ref)),
        field("resource-profile-ref", crate::preserves_rail::string(&world.resource_profile_ref)),
        field("workload-ref", crate::preserves_rail::string(&world.workload_ref)),
        field("fault-plan-ref", crate::preserves_rail::string(&world.fault_plan_ref)),
        field("invariant-set-ref", crate::preserves_rail::string(&world.invariant_set_ref)),
        field("nodes", crate::preserves_rail::sequence(world.nodes.iter().map(node_value).collect())),
        field(
            "port-profiles",
            crate::preserves_rail::sequence(world.port_profiles.iter().map(port_profile_value).collect()),
        ),
        field(
            "workload",
            crate::preserves_rail::sequence(world.workload.iter().map(workload_step_value).collect()),
        ),
        field("faults", crate::preserves_rail::sequence(world.faults.iter().map(fault_value).collect())),
        field(
            "invariants",
            crate::preserves_rail::sequence(world.invariants.iter().map(invariant_value).collect()),
        ),
        field("bounds", bounds_value(&world.bounds)),
        field("claim-profile", crate::preserves_rail::string(world.claim_profile.as_str())),
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

fn node_value(node: &SimulatedNode) -> preserves::IOValue {
    crate::preserves_rail::record("fabric-simulation-node-v1", vec![
        field("node-id", crate::preserves_rail::string(&node.node_id)),
        field("extension-id", crate::preserves_rail::string(&node.extension_id)),
        field("service-id", crate::preserves_rail::string(&node.service_id)),
        field("generation", crate::preserves_rail::u64_value(node.generation)),
        field("initial-state-ref", crate::preserves_rail::string(&node.initial_state_ref)),
        field("membership-view-ref", crate::preserves_rail::string(&node.membership_view_ref)),
        field("placement-ref", crate::preserves_rail::string(&node.placement_ref)),
        field("consistency-profile-ref", crate::preserves_rail::string(&node.consistency_profile_ref)),
        field("simulation-core", core_identity_value(&node.same_core.simulation)),
        field("live-core", core_identity_value(&node.same_core.live)),
        field(
            "required-port-classes",
            strings_value(node.required_port_classes.iter().map(|class| class.as_str())),
        ),
    ])
}

fn core_identity_value(identity: &ExtensionCoreIdentity) -> preserves::IOValue {
    crate::preserves_rail::record("fabric-simulation-core-identity-v1", vec![
        field("implementation-ref", crate::preserves_rail::string(&identity.implementation_ref)),
        field("manifest-ref", crate::preserves_rail::string(&identity.manifest_ref)),
        field("callback-dispatcher-ref", crate::preserves_rail::string(&identity.callback_dispatcher_ref)),
        field("protocol-core-ref", crate::preserves_rail::string(&identity.protocol_core_ref)),
        field("state-machine-ref", crate::preserves_rail::string(&identity.state_machine_ref)),
        field("schema-set-ref", crate::preserves_rail::string(&identity.schema_set_ref)),
        field("port-contract-set-ref", crate::preserves_rail::string(&identity.port_contract_set_ref)),
    ])
}

fn port_profile_value(profile: &SimulatedPortProfile) -> preserves::IOValue {
    crate::preserves_rail::record("fabric-simulation-port-profile-v1", vec![
        field("class", crate::preserves_rail::string(profile.class.as_str())),
        field("port-id", crate::preserves_rail::string(&profile.port_id)),
        field("version", crate::preserves_rail::string(&profile.version)),
        field("implementation-profile", crate::preserves_rail::string(&profile.implementation_profile)),
        field("descriptor-ref", crate::preserves_rail::string(&profile.descriptor_ref)),
        field("command-schema-ref", crate::preserves_rail::string(&profile.command_schema_ref)),
        field("event-schema-ref", crate::preserves_rail::string(&profile.event_schema_ref)),
        field("deterministic", crate::preserves_rail::bool_value(profile.deterministic)),
        field("declared-faults", strings_value(profile.declared_faults.iter().map(|fault| fault.as_str()))),
    ])
}

fn workload_step_value(step: &SimulationWorkloadStep) -> preserves::IOValue {
    crate::preserves_rail::record("fabric-simulation-workload-step-v1", vec![
        field("sequence", crate::preserves_rail::u64_value(step.sequence)),
        field("node-id", crate::preserves_rail::string(&step.node_id)),
        field("request-ref", crate::preserves_rail::string(&step.request_ref)),
        field("service", crate::preserves_rail::string(step.service.as_str())),
        field("expected-failure-class", optional_string(step.expected_failure_class.as_deref())),
    ])
}

fn fault_value(fault: &SimulationFaultAction) -> preserves::IOValue {
    crate::preserves_rail::record("fabric-simulation-fault-v1", vec![
        field("fault-id", crate::preserves_rail::string(&fault.fault_id)),
        field("kind", crate::preserves_rail::string(fault.kind.as_str())),
        field("target", crate::preserves_rail::string(&fault.target)),
        field("boundary", crate::preserves_rail::string(fault.boundary.as_str())),
        field("activate-at-choice", crate::preserves_rail::u64_value(fault.activate_at_choice)),
        field("duration-choices", optional_u64(fault.duration_choices)),
        field("resource-cost", crate::preserves_rail::u64_value(fault.resource_cost)),
        field("expected-observation", crate::preserves_rail::string(&fault.expected_observation)),
        field(
            "direct-extension-state-mutation",
            crate::preserves_rail::bool_value(fault.direct_extension_state_mutation),
        ),
    ])
}

fn invariant_value(invariant: &SimulationInvariant) -> preserves::IOValue {
    match invariant {
        SimulationInvariant::Universal(kind) => crate::preserves_rail::record("universal-invariant", vec![field(
            "kind",
            crate::preserves_rail::string(kind.as_str()),
        )]),
        SimulationInvariant::ExtensionSemantic { service, invariant_id } => {
            crate::preserves_rail::record("extension-invariant", vec![
                field("service", crate::preserves_rail::string(service.as_str())),
                field("invariant-id", crate::preserves_rail::string(invariant_id)),
            ])
        }
    }
}

fn bounds_value(bounds: &SimulationBounds) -> preserves::IOValue {
    crate::preserves_rail::record("fabric-simulation-bounds-v1", vec![
        field("max-choices", crate::preserves_rail::u64_value(bounds.max_choices)),
        field("max-events", crate::preserves_rail::u64_value(bounds.max_events)),
        field("max-virtual-ticks", crate::preserves_rail::u64_value(bounds.max_virtual_ticks)),
        field("max-trace-bytes", crate::preserves_rail::u64_value(bounds.max_trace_bytes)),
        field("max-resource-units", crate::preserves_rail::u64_value(bounds.max_resource_units)),
        field("max-shrink-attempts", crate::preserves_rail::u64_value(bounds.max_shrink_attempts)),
    ])
}

fn choice_record_value(record_value: &SchedulerChoiceRecord) -> crate::error::Result<preserves::IOValue> {
    if record_value.semantic_output_ref.is_empty() || record_value.semantic_output_ref == PENDING_SEMANTIC_OUTPUT_REF {
        return Err(crate::error::MoltenError::invalid_harness(
            "fabric simulation choice record lacks its executed semantic output ref",
        ));
    }
    Ok(crate::preserves_rail::record("fabric-simulation-choice-v1", vec![
        field("position", crate::preserves_rail::u64_value(record_value.position)),
        field("virtual-tick", crate::preserves_rail::u64_value(record_value.virtual_tick)),
        field(
            "eligible",
            crate::preserves_rail::sequence(record_value.eligible.iter().map(eligible_choice_value).collect()),
        ),
        field("selected", eligible_choice_value(&record_value.selected)),
        field("semantic-output-ref", crate::preserves_rail::string(&record_value.semantic_output_ref)),
    ]))
}

fn eligible_choice_value(choice: &EligibleChoice) -> preserves::IOValue {
    crate::preserves_rail::record("eligible-choice-v1", vec![
        field("kind", crate::preserves_rail::string(choice.kind.as_str())),
        field("choice-id", crate::preserves_rail::string(&choice.choice_id)),
        field("node-id", crate::preserves_rail::string(&choice.node_id)),
        field("generation", crate::preserves_rail::u64_value(choice.generation)),
        field("ready-at-tick", crate::preserves_rail::u64_value(choice.ready_at_tick)),
    ])
}

fn invariant_result_value(result: &InvariantResult) -> preserves::IOValue {
    crate::preserves_rail::record("fabric-simulation-invariant-result-v1", vec![
        field("invariant", invariant_value(&result.invariant)),
        field("passed", crate::preserves_rail::bool_value(result.passed)),
        field("first-failure-sequence", optional_u64(result.first_failure_sequence)),
    ])
}

fn optional_divergence_value(divergence: Option<&ReplayDivergence>) -> preserves::IOValue {
    match divergence {
        None => crate::preserves_rail::record("none", Vec::new()),
        Some(divergence) => crate::preserves_rail::record("some", vec![crate::preserves_rail::record(
            "fabric-simulation-divergence-v1",
            vec![
                field("position", crate::preserves_rail::u64_value(divergence.position)),
                field("expected-choice-id", crate::preserves_rail::string(&divergence.expected_choice_id)),
                field("eligible-choice-ids", strings_value(divergence.eligible_choice_ids.iter().map(String::as_str))),
                field("diagnostic", crate::preserves_rail::string(&divergence.diagnostic)),
            ],
        )]),
    }
}

fn record_optional_divergence(value: &preserves::Value<preserves::IOValue>) -> crate::error::Result<Option<String>> {
    let field_value = named_field_value(value, "first-divergence")?;
    if field_value.collect_simple_record("none", Some(0)).is_some() {
        return Ok(None);
    }
    let some = field_value
        .collect_simple_record("some", Some(1))
        .ok_or_else(|| crate::error::MoltenError::invalid_harness("expected optional simulation divergence"))?;
    Ok(Some(crate::preserves_rail::canonical_hash((&some[0]).into())?))
}

fn validate_count(label: &str, actual: usize) -> crate::error::Result<()> {
    if actual > MAX_CANONICAL_SIMULATION_ITEMS {
        Err(crate::error::MoltenError::invalid_harness(format!(
            "fabric simulation {label} count {actual} exceeds {MAX_CANONICAL_SIMULATION_ITEMS}"
        )))
    } else {
        Ok(())
    }
}

fn validate_refs(label: &str, refs: &[String]) -> crate::error::Result<()> {
    validate_count(label, refs.len())?;
    for reference in refs {
        crate::preserves_rail::validate_content_ref(reference)?;
    }
    Ok(())
}

fn u64_len(value: usize) -> crate::error::Result<preserves::IOValue> {
    let converted = u64::try_from(value)
        .map_err(|_| crate::error::MoltenError::invalid_harness("fabric simulation collection length overflow"))?;
    Ok(crate::preserves_rail::u64_value(converted))
}

fn field(label: &'static str, value: preserves::IOValue) -> preserves::IOValue {
    crate::preserves_rail::record(label, vec![value])
}

fn checks(values: &[&str]) -> preserves::IOValue {
    field("checks", strings_value(values.iter().copied()))
}

fn strings_value<'a>(values: impl IntoIterator<Item = &'a str>) -> preserves::IOValue {
    crate::preserves_rail::sequence(values.into_iter().map(crate::preserves_rail::string).collect())
}

fn optional_string(value: Option<&str>) -> preserves::IOValue {
    match value {
        Some(value) => crate::preserves_rail::record("some", vec![crate::preserves_rail::string(value)]),
        None => crate::preserves_rail::record("none", Vec::new()),
    }
}

fn optional_u64(value: Option<u64>) -> preserves::IOValue {
    match value {
        Some(value) => crate::preserves_rail::record("some", vec![crate::preserves_rail::u64_value(value)]),
        None => crate::preserves_rail::record("none", Vec::new()),
    }
}

fn record_string_field(value: &preserves::Value<preserves::IOValue>, label: &str) -> crate::error::Result<String> {
    required_string(&named_field_value(value, label)?, label)
}

fn record_u64_field(value: &preserves::Value<preserves::IOValue>, label: &str) -> crate::error::Result<u64> {
    named_field_value(value, label)?
        .as_u64()
        .ok_or_else(|| crate::error::MoltenError::invalid_harness(format!("expected u64 for {label}")))?
        .map_err(|error| crate::error::MoltenError::invalid_harness(format!("u64 out of range for {label}: {error}")))
}

fn record_string_sequence_field(
    value: &preserves::Value<preserves::IOValue>,
    label: &str,
) -> crate::error::Result<Vec<String>> {
    let field_value = named_field_value(value, label)?;
    let sequence = field_value
        .as_sequence()
        .ok_or_else(|| crate::error::MoltenError::invalid_harness(format!("expected sequence for {label}")))?;
    sequence.map(|item| required_string(&item, label)).collect()
}

fn named_field_value(
    value: &preserves::Value<preserves::IOValue>,
    label: &str,
) -> crate::error::Result<preserves::Value<preserves::IOValue>> {
    let fields = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| crate::error::MoltenError::invalid_harness(format!("expected named field {label}")))?;
    Ok(fields[0].clone())
}

fn required_string(value: &preserves::Value<preserves::IOValue>, label: &str) -> crate::error::Result<String> {
    value
        .as_string()
        .map(|value| value.into_owned())
        .ok_or_else(|| crate::error::MoltenError::invalid_harness(format!("expected string for {label}")))
}

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

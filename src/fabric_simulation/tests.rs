use super::*;
use crate::fabric::FabricPortClass;
use crate::fabric::ReferenceSystemKind;

const EXPECTED_REFERENCE_SERVICES: usize = 3;
const EXPECTED_WORKLOAD_STEPS: usize = 6;
const EXPECTED_PORT_EVENTS_PER_STEP: usize = REQUIRED_SIMULATION_PORT_CLASS_COUNT;
const EXPECTED_PORT_EVENTS: usize = EXPECTED_WORKLOAD_STEPS * EXPECTED_PORT_EVENTS_PER_STEP;

// r[verify molten.fabric_simulation.world_manifest]
// r[verify molten.fabric_simulation.same_core]
// r[verify molten.fabric_simulation.port_substitution]
#[test]
fn reference_world_binds_same_core_and_every_deterministic_port_class() {
    let world = build_reference_simulated_world().expect("reference world");

    assert!(world.world_ref.starts_with("blake3:"));
    assert_eq!(world.admitted.manifest.nodes.len(), EXPECTED_REFERENCE_SERVICES);
    assert_eq!(world.admitted.manifest.port_profiles.len(), REQUIRED_SIMULATION_PORT_CLASS_COUNT);
    assert!(world.admitted.manifest.ambient_inputs.is_empty());
    assert!(world.admitted.manifest.port_profiles.iter().all(|profile| profile.deterministic));
    for node in &world.admitted.manifest.nodes {
        assert_eq!(node.same_core.simulation, node.same_core.live);
        assert!(
            node.required_port_classes
                .iter()
                .all(|class| { world.admitted.manifest.port_profiles.iter().any(|profile| profile.class == *class) })
        );
    }
    for class in REQUIRED_SIMULATION_PORT_CLASSES {
        assert!(world.admitted.manifest.port_profiles.iter().any(|profile| profile.class == class));
    }
}

// r[verify molten.fabric_simulation.scheduler]
// r[verify molten.fabric_simulation.fault_model]
// r[verify molten.fabric_simulation.invariants]
// r[verify molten.fabric_simulation.reference_services]
// r[verify molten.fabric_simulation.fabric_sufficiency]
// r[verify molten.fabric_simulation.evidence]
#[test]
fn three_reference_services_run_through_host_callbacks_and_named_ports() {
    let fixture = run_reference_simulation_fixture().expect("reference simulation");

    assert_eq!(fixture.run.summary.decision, SimulationDecision::Pass);
    assert_eq!(fixture.run.summary.choice_records.len(), EXPECTED_WORKLOAD_STEPS);
    assert_eq!(fixture.observations.len(), EXPECTED_WORKLOAD_STEPS);
    assert_eq!(fixture.port_events.len(), EXPECTED_PORT_EVENTS);
    assert_eq!(fixture.run.summary.final_state_refs.len(), EXPECTED_REFERENCE_SERVICES);
    assert!(fixture.run.summary.invariant_results.iter().all(|result| result.passed));
    assert!(
        fixture
            .port_events
            .iter()
            .any(|event| event.class == FabricPortClass::Transport && event.fault == Some(SimulationFaultKind::Delay))
    );
    assert!(fixture.port_events.iter().all(|event| event.event_ref.starts_with("blake3:")));
    assert!(fixture.host_evidence_refs.iter().all(|reference| reference.starts_with("blake3:")));
    assert!(fixture.bundle.bundle_ref.starts_with("blake3:"));
    let services = fixture
        .observations
        .iter()
        .filter_map(|observation| observation.observation.service)
        .collect::<std::collections::BTreeSet<_>>();
    assert!(services.contains(&ReferenceSystemKind::TransactionalKeyValue));
    assert!(services.contains(&ReferenceSystemKind::ReplicatedLog));
    assert!(services.contains(&ReferenceSystemKind::DistributedScheduler));
}

// r[verify molten.fabric_simulation.scheduler]
// r[verify molten.fabric_simulation.replay_shrink]
#[test]
fn reference_run_repeats_with_identical_world_trace_and_evidence_identity() {
    let first = run_reference_simulation_fixture().expect("first run");
    let replay = replay_reference_simulation_fixture(&first.run).expect("replay");

    assert!(replay.comparison.matches);
    assert!(replay.comparison.first_divergence.is_none());
    assert_eq!(first.world.world_ref, replay.replay.world.world_ref);
    assert_eq!(first.run.run_ref, replay.replay.run.run_ref);
    assert_eq!(first.bundle.bundle_ref, replay.replay.bundle.bundle_ref);
    assert_eq!(first.port_events, replay.replay.port_events);
    assert_eq!(first.observations, replay.replay.observations);
}

// r[verify molten.fabric_simulation.live_sim_differential]
// r[verify molten.fabric_simulation.claim_ladder]
#[test]
fn differential_is_contract_scoped_and_simulation_cannot_promote_itself_to_live() {
    let fixture = run_reference_simulation_fixture().expect("reference simulation");
    let evidence = ClaimEvidence {
        profile: SimulationClaimProfile::DeterministicWholeSystem,
        implementation_ref: fixture.world.admitted.manifest.runtime_ref.clone(),
        environment_ref: None,
        adapter_refs: fixture
            .world
            .admitted
            .manifest
            .port_profiles
            .iter()
            .map(|profile| profile.descriptor_ref.clone())
            .collect(),
        lifecycle_ref: None,
        fault_ref: Some(fixture.world.admitted.manifest.fault_plan_ref.clone()),
        operator_ref: None,
    };
    let promotion = canonical_claim_promotion(
        SimulationClaimProfile::DeterministicWholeSystem,
        SimulationClaimProfile::MultiProcessLive,
        &evidence,
    )
    .expect("canonical claim promotion decision");
    let text = crate::preserves_rail::to_text(&fixture.differential.value).expect("differential text");
    let promotion_text = crate::preserves_rail::to_text(&promotion.value).expect("promotion text");

    assert!(fixture.differential.equivalent);
    assert_ne!(fixture.differential.simulation_profile_ref, fixture.differential.live_profile_ref);
    assert!(text.contains("no-live-production-equivalence-claim"));
    assert!(!promotion.decision.admitted);
    assert!(promotion.decision_ref.starts_with("blake3:"));
    assert!(promotion.decision.missing_evidence.contains(&"profile-specific-evidence"));
    assert!(promotion.decision.missing_evidence.contains(&"environment-evidence"));
    assert!(promotion.decision.missing_evidence.contains(&"lifecycle-evidence"));
    assert!(promotion.decision.missing_evidence.contains(&"operator-evidence"));
    assert!(promotion_text.contains("stronger-profile-cannot-use-simulation-label"));
}

// r[verify molten.fabric_simulation.operator_workflow]
// r[verify molten.fabric_simulation.evidence]
#[test]
fn run_readback_is_bounded_and_excludes_secret_payloads() {
    let fixture = run_reference_simulation_fixture().expect("reference simulation");
    let readback = parse_simulation_run_readback(&fixture.run.value).expect("run readback");
    let text = crate::preserves_rail::to_text(&fixture.run.value).expect("run text");

    assert_eq!(readback.decision, "pass");
    assert_eq!(readback.profile, "deterministic-whole-system");
    assert_eq!(readback.choice_count, EXPECTED_WORKLOAD_STEPS as u64);
    assert_eq!(readback.event_count, EXPECTED_WORKLOAD_STEPS as u64);
    assert_eq!(readback.final_state_refs.len(), EXPECTED_REFERENCE_SERVICES);
    assert_eq!(readback.run_ref, fixture.run.run_ref);
    assert!(!text.contains("private-key"));
    assert!(!text.contains("bearer-token"));
    assert!(!text.contains("environment-variable"));
    assert!(!text.contains("secret-bytes"));
}

// r[verify molten.fabric_simulation.operator_workflow]
// r[verify molten.fabric_simulation.final_validation]
#[test]
fn malformed_run_decision_and_missing_adapter_fail_closed() {
    let fixture = run_reference_simulation_fixture().expect("reference simulation");
    let text = crate::preserves_rail::to_text(&fixture.run.value).expect("run text");
    let malformed_text = text.replacen("pass", "production-approved", 1);
    let malformed = crate::preserves_rail::parse_text(&malformed_text).expect("malformed syntax remains valid");
    let readback_error = parse_simulation_run_readback(&malformed).expect_err("unknown decision must deny");
    let mut manifest = fixture.world.admitted.manifest.clone();
    manifest.port_profiles.retain(|profile| profile.class != FabricPortClass::DurableState);
    let world_error = canonical_admit_simulated_world(&manifest).expect_err("missing port must deny");

    assert!(readback_error.to_string().contains("unsupported simulation decision"));
    assert!(world_error.to_string().contains("MissingPortClass(DurableState)"));
}

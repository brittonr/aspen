use super::*;

fn fixture() -> (
    crate::fabric_simulation::InstalledWorldInput,
    Vec<crate::fabric_simulation::InstalledExtensionProfile>,
) {
    let state = fixtures::initial_state().expect("state");
    let initial = validated_state_ref(&state).expect("identity");
    let command = crate::fabric_consistency::raft::Command {
        request_ref: fixtures::reference("request"),
        command_ref: fixtures::reference("command"),
        command_schema_ref: command_schema_ref().expect("schema"),
    };
    inputs(&initial, 0, &[command]).expect("installed profile inputs")
}

// r[verify molten.fabric_simulation.installed_extension_admission]
#[test]
fn loaded_profile_generation_and_canonical_body_are_not_substitutable() {
    let (input, profiles) = fixture();
    let mut wrong_generation = input.clone();
    wrong_generation.mechanisms.nodes[0].generation = 2;
    assert_eq!(
        crate::fabric_simulation::admit_installed_world(&wrong_generation, &profiles),
        Err(crate::fabric_simulation::InstalledWorldIssue::WorkloadGeneration)
    );
    let mut stale_identity = profiles.clone();
    stale_identity[0].max_nodes = 4;
    assert!(crate::fabric_simulation::admit_extension_world(&input, &stale_identity).is_err());
    let mut duplicate = input;
    duplicate.mechanisms.invariants.push(duplicate.mechanisms.invariants[0].clone());
    assert!(crate::fabric_simulation::admit_installed_world(&duplicate, &profiles).is_err());
}

// r[verify molten.fabric_simulation.installed_extension_bounded_execution]
#[test]
fn selected_workload_rejects_foreign_request_profile_node_generation_and_sequence() {
    let (input, profiles) = fixture();
    let world = crate::fabric_simulation::admit_installed_world(&input, &profiles).expect("world");
    let step = &input.workload[0];
    let scheduler = crate::fabric_simulation::SimulationSchedulerState::initial();
    let candidate = crate::fabric_simulation::InstalledEligibleChoice {
        choice: crate::fabric_simulation::EligibleChoice {
            kind: crate::fabric_simulation::SchedulerChoiceKind::MessageDelivery,
            choice_id: crate::fabric_simulation::message_delivery_choice_id("scope-command-0-0"),
            node_id: step.node_id.clone(),
            generation: step.generation,
            ready_at_tick: 0,
        },
        workload_sequence: step.sequence,
        profile_ref: step.profile_ref.clone(),
        request_ref: step.request_ref.clone(),
    };
    let selected = crate::fabric_simulation::select_installed_workload_choice(
        &world,
        &scheduler,
        std::slice::from_ref(&candidate),
        None,
    )
    .expect("selected");
    assert_eq!(selected.workload, *step);
    assert_eq!(selected.transition.next.next_choice_position, 1);
    for variant in 0..5 {
        let mut mutant = candidate.clone();
        match variant {
            0 => mutant.request_ref = artifact("foreign-request"),
            1 => mutant.profile_ref = artifact("foreign-profile"),
            2 => mutant.choice.node_id = "scope-node-1".to_string(),
            3 => mutant.choice.generation = 2,
            _ => mutant.workload_sequence = 3,
        }
        assert!(
            crate::fabric_simulation::select_installed_workload_choice(&world, &scheduler, &[mutant], None).is_err(),
            "workload mutant {variant} admitted"
        );
    }
}

// r[verify molten.fabric_simulation.installed_extension_reference_preservation]
#[test]
fn reference_manifest_still_requires_its_reference_semantic_oracles() {
    let mut reference = crate::fabric_simulation::reference_world_manifest().expect("reference world");
    reference
        .invariants
        .retain(|invariant| matches!(invariant, crate::fabric_simulation::SimulationInvariant::Universal(_)));
    assert!(crate::fabric_simulation::admit_simulated_world(&reference).is_err());
}

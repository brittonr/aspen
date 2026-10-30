use super::*;

fn artifact(label: &str) -> molten_core::nominal::ArtifactRef {
    let value =
        crate::preserves_rail::record("collaboration-simulation-invariant-id-v1", vec![crate::preserves_rail::string(
            label,
        )]);
    molten_core::nominal::ArtifactRef::new(crate::preserves_rail::canonical_hash(&value).expect("canonical artifact"))
        .expect("bounded artifact")
}
fn source_ref(label: &'static str, source: &str) -> crate::error::Result<String> {
    crate::preserves_rail::canonical_hash(&crate::preserves_rail::record(label, vec![crate::preserves_rail::string(
        source,
    )]))
}
pub(super) fn inputs(
    initial: &StateRef,
    seed: u64,
    commands: &[crate::fabric_consistency::raft::Command],
) -> crate::error::Result<(
    crate::fabric_simulation::InstalledWorldInput,
    Vec<crate::fabric_simulation::InstalledExtensionProfile>,
)> {
    let (identity, handler) = core_identity()?;
    let profile = extension_profile(&identity, &handler)?;
    let profile_ref = profile.profile_ref.clone();
    let mut mechanisms = crate::fabric_simulation::reference_world_manifest()?;
    bind_mechanisms(&mut mechanisms, initial, seed, &identity, &handler)?;
    declare_faults(&mut mechanisms)?;
    let nodes = mechanisms
        .nodes
        .iter()
        .map(|node| crate::fabric_simulation::InstalledNodeBinding {
            node_id: node.node_id.clone(),
            profile_ref: profile_ref.clone(),
        })
        .collect();
    let workload = commands
        .iter()
        .enumerate()
        .flat_map(|(sequence, command)| (0..3).map(move |node| (sequence, command, node)))
        .map(|(sequence, command, node)| {
            Ok(crate::fabric_simulation::InstalledWorkloadStep {
                sequence: (sequence * 3 + node) as u64,
                node_id: format!("scope-node-{node}"),
                generation: 1,
                profile_ref: profile_ref.clone(),
                request_ref: molten_core::nominal::ArtifactRef::new(command.request_ref.clone())
                    .map_err(|_| denied(Denial::Domain))?,
            })
        })
        .collect::<crate::error::Result<Vec<_>>>()?;
    let invariants = profile
        .required_invariants
        .iter()
        .cloned()
        .map(|invariant| crate::fabric_simulation::InstalledInvariantBinding {
            profile_ref: profile_ref.clone(),
            invariant,
        })
        .collect();
    Ok((
        crate::fabric_simulation::InstalledWorldInput {
            mechanisms,
            nodes,
            workload,
            invariants,
        },
        vec![profile],
    ))
}
fn core_identity()
-> crate::error::Result<(crate::fabric_simulation::ExtensionCoreIdentity, molten_core::nominal::ArtifactRef)> {
    let implementation = source_ref(
        "collaboration-implementation-source-v1",
        concat!(include_str!("../application/runtime.rs"), include_str!("../application/commit.rs")),
    )?;
    let core = source_ref(
        "collaboration-state-machine-source-v1",
        concat!(
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/crates/molten-core/src/collaborationscope/admission.rs"
            )),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/crates/molten-core/src/collaborationscope/transition.rs"
            ))
        ),
    )?;
    let handler = molten_core::nominal::ArtifactRef::new(source_ref(
        "collaboration-invariant-handler-source-v1",
        concat!(include_str!("../simulation.rs"), include_str!("dispatch.rs")),
    )?)
    .map_err(|_| denied(Denial::Domain))?;
    let identity = crate::fabric_simulation::ExtensionCoreIdentity {
        implementation_ref: implementation.clone(),
        manifest_ref: source_ref("collaboration-schema-profile-v1", SCHEMA)?,
        callback_dispatcher_ref: implementation,
        protocol_core_ref: core.clone(),
        state_machine_ref: core,
        schema_set_ref: source_ref("collaboration-wire-schema-source-v1", include_str!("../codec/canonical.rs"))?,
        port_contract_set_ref: source_ref(
            "collaboration-port-contract-source-v1",
            include_str!("../admission/manifest.rs"),
        )?,
    };
    Ok((identity, handler))
}
fn extension_profile(
    identity: &crate::fabric_simulation::ExtensionCoreIdentity,
    handler: &molten_core::nominal::ArtifactRef,
) -> crate::error::Result<crate::fabric_simulation::InstalledExtensionProfile> {
    let mut profile = crate::fabric_simulation::InstalledExtensionProfile {
        profile_ref: artifact("collaboration-installed-simulation-profile-v1"),
        service_generation: 1,
        extension_id: molten_core::nominal::ServiceRef::new(EXTENSION_ID).map_err(|_| denied(Denial::Domain))?,
        core: identity.clone(),
        required_invariants: ["epoch-monotonic", "one-owner", "denial-preservation", "convergence"]
            .into_iter()
            .map(|name| crate::fabric_simulation::InstalledInvariant {
                invariant_ref: artifact(name),
                handler_ref: handler.clone(),
            })
            .collect(),
        max_nodes: 3,
        max_steps: 6,
    };
    profile.profile_ref = crate::fabric_simulation::extension_profile_ref(&profile)?;
    Ok(profile)
}
fn bind_mechanisms(
    mechanisms: &mut molten_core::fabric_simulation::SimulatedWorldManifest,
    initial: &StateRef,
    seed: u64,
    identity: &crate::fabric_simulation::ExtensionCoreIdentity,
    handler: &molten_core::nominal::ArtifactRef,
) -> crate::error::Result<()> {
    mechanisms.runtime_ref = identity.implementation_ref.clone();
    mechanisms.initial_durable_state_ref = initial.as_str().to_string();
    mechanisms.entropy_input_ref = source_ref("collaboration-simulation-seed-v1", &seed.to_string())?;
    mechanisms.policy_ref = policy_ref(fixtures::POLICY)?.as_str().to_string();
    mechanisms.authority_ref =
        source_ref("collaboration-authority-source-v1", include_str!("../fixtures/authority.rs"))?;
    mechanisms.scheduler_input_ref =
        source_ref("collaboration-scheduler-input-source-v1", include_str!("../simulation.rs"))?;
    mechanisms.workload_ref = source_ref("collaboration-workload-source-v1", include_str!("../simulation.rs"))?;
    mechanisms.fault_plan_ref = source_ref("collaboration-fault-plan-source-v1", include_str!("world.rs"))?;
    mechanisms.invariant_set_ref = handler.as_str().to_string();
    mechanisms.resource_profile_ref =
        source_ref("collaboration-resource-profile-source-v1", include_str!("../simulation.rs"))?;
    mechanisms.workload.clear();
    mechanisms.invariants = crate::fabric_simulation::REQUIRED_UNIVERSAL_INVARIANTS
        .iter()
        .copied()
        .map(crate::fabric_simulation::SimulationInvariant::Universal)
        .collect();
    for (index, node) in mechanisms.nodes.iter_mut().enumerate() {
        node.node_id = format!("scope-node-{index}");
        node.extension_id = EXTENSION_ID.to_string();
        node.service_id = format!("collaboration-service-{index}");
        node.initial_state_ref = initial.as_str().to_string();
        node.same_core = crate::fabric_simulation::SameCoreWitness {
            live: identity.clone(),
            simulation: identity.clone(),
        };
    }
    Ok(())
}
fn declare_faults(mechanisms: &mut molten_core::fabric_simulation::SimulatedWorldManifest) -> crate::error::Result<()> {
    for (class, fault) in [
        (crate::fabric::FabricPortClass::Transport, crate::fabric_simulation::SimulationFaultKind::Partition),
        (crate::fabric::FabricPortClass::DurableState, crate::fabric_simulation::SimulationFaultKind::Crash),
    ] {
        let port = mechanisms
            .port_profiles
            .iter_mut()
            .find(|port| port.class == class)
            .ok_or_else(|| denied(Denial::Missing))?;
        if !port.declared_faults.contains(&fault) {
            port.declared_faults.push(fault);
        }
    }
    mechanisms.faults = vec![
        crate::fabric_simulation::SimulationFaultAction {
            fault_id: "scope-partition".to_string(),
            kind: crate::fabric_simulation::SimulationFaultKind::Partition,
            target: "scope-node-2".to_string(),
            boundary: crate::fabric::FabricPortClass::Transport,
            activate_at_choice: 0,
            duration_choices: Some(5),
            resource_cost: 1,
            expected_observation: "scope-third-node-held-until-healing".to_string(),
            direct_extension_state_mutation: false,
        },
        crate::fabric_simulation::SimulationFaultAction {
            fault_id: "scope-crash".to_string(),
            kind: crate::fabric_simulation::SimulationFaultKind::Crash,
            target: "scope-node-2".to_string(),
            boundary: crate::fabric::FabricPortClass::DurableState,
            activate_at_choice: 4,
            duration_choices: None,
            resource_cost: 1,
            expected_observation: "machine-loss-state-reopened".to_string(),
            direct_extension_state_mutation: false,
        },
    ];
    Ok(())
}
pub(super) fn manifest(
    initial: &StateRef,
    seed: u64,
    commands: &[crate::fabric_consistency::raft::Command],
) -> crate::error::Result<crate::fabric_simulation::AdmittedSimulatedWorld> {
    let (input, profiles) = inputs(initial, seed, commands)?;
    crate::fabric_simulation::admit_extension_world(&input, &profiles).map(|world| world.admitted)
}
// r[verify molten.fabric_simulation.installed_extension_validation]
#[test]
fn installed_admission_denies_missing_wrong_duplicate_foreign_core_and_bound_mutants() {
    let state = fixtures::initial_state().expect("state");
    let initial = validated_state_ref(&state).expect("identity");
    let command = crate::fabric_consistency::raft::Command {
        request_ref: fixtures::reference("request"),
        command_ref: fixtures::reference("command"),
        command_schema_ref: command_schema_ref().expect("schema"),
    };
    let (input, profiles) = inputs(&initial, 0, &[command]).expect("profile inputs");
    assert!(
        crate::fabric_simulation::admit_installed_world(&input, &profiles)
            .expect("admitted installed profile")
            .installed()
            .is_some()
    );
    let mut missing = input.clone();
    missing.invariants.pop();
    assert_eq!(
        crate::fabric_simulation::admit_installed_world(&missing, &profiles),
        Err(crate::fabric_simulation::InstalledWorldIssue::MissingInvariant)
    );
    let mut wrong = input.clone();
    wrong.invariants[0].invariant.handler_ref = artifact("foreign-handler");
    assert_eq!(
        crate::fabric_simulation::admit_installed_world(&wrong, &profiles),
        Err(crate::fabric_simulation::InstalledWorldIssue::WrongInvariant)
    );
    let mut duplicate = input.clone();
    duplicate.invariants.push(duplicate.invariants[0].clone());
    assert_eq!(
        crate::fabric_simulation::admit_installed_world(&duplicate, &profiles),
        Err(crate::fabric_simulation::InstalledWorldIssue::DuplicateInvariant)
    );
    let mut foreign = input.clone();
    foreign.nodes[0].profile_ref = artifact("foreign-profile");
    assert_eq!(
        crate::fabric_simulation::admit_installed_world(&foreign, &profiles),
        Err(crate::fabric_simulation::InstalledWorldIssue::UnknownProfile)
    );
    let mut wrong_core = input.clone();
    wrong_core.mechanisms.nodes[0].same_core.live.state_machine_ref = fixtures::reference("foreign-core");
    assert!(crate::fabric_simulation::admit_installed_world(&wrong_core, &profiles).is_err());
    let mut bounded = profiles.clone();
    bounded[0].max_nodes = 2;
    assert_eq!(
        crate::fabric_simulation::admit_installed_world(&input, &bounded),
        Err(crate::fabric_simulation::InstalledWorldIssue::Bound)
    );
    assert!(crate::fabric_simulation::admit_simulated_world(&input.mechanisms).is_err());
    let reference = crate::fabric_simulation::reference_world_manifest().expect("reference world");
    assert!(
        crate::fabric_simulation::admit_simulated_world(&reference)
            .expect("reference admission unchanged")
            .installed()
            .is_none()
    );
}

#[path = "admission.rs"]
mod admissiontests;

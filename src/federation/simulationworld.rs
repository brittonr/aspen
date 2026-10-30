pub struct CampaignInput<'a, 's> {
    pub manifest: &'a super::admission::Manifest,
    pub replay: &'a super::simulation::Replay<'s>,
    pub inventory: &'a super::envelope::network::VerifiedRecord,
    pub live_session: &'a molten_core::federation::Session,
    pub now: u64,
}

/// Admits the maintained federation core and actually dispatches its bounded evaluator.
/// Source identities identify code, not reviewed production conformance evidence.
// r[impl molten.federated_pull_runtime.final_validation]
pub fn run(input: CampaignInput<'_, '_>) -> crate::error::Result<preserves::IOValue> {
    if input.replay.manifest_ref != input.manifest.manifest_ref()
        || input.replay.generation != input.manifest.generation()
        || input.replay.bounds != input.manifest.bounds()
        || input.replay.peers != input.manifest.peers()
    {
        return Err(invalid("installed federation replay differs from the admitted live manifest"));
    }
    let (world_input, profile) = inputs(&input)?;
    let world_artifact = crate::fabric_simulation::admit_extension_world(&world_input, std::slice::from_ref(&profile))?;
    let world = &world_artifact.admitted;
    let choice = molten_core::fabric_simulation::EligibleChoice {
        kind: molten_core::fabric_simulation::SchedulerChoiceKind::Runnable,
        choice_id: "federation-owned-campaign".to_string(),
        node_id: "federation-receiver".to_string(),
        generation: input.replay.generation,
        ready_at_tick: input.now,
    };
    let step = world
        .installed()
        .and_then(|installed| installed.input().workload.first())
        .ok_or_else(|| invalid("installed federation workload is absent"))?;
    let candidate = molten_core::fabric_simulation::InstalledEligibleChoice {
        choice,
        workload_sequence: step.sequence,
        profile_ref: step.profile_ref.clone(),
        request_ref: step.request_ref.clone(),
    };
    let selected = molten_core::fabric_simulation::select_installed_workload_choice(
        world,
        &molten_core::fabric_simulation::SimulationSchedulerState::initial(),
        &[candidate],
        None,
    )
    .map_err(|issue| invalid(&format!("federation installed scheduler denied: {issue:?}")))?;
    if selected.workload.request_ref.as_str() != input.inventory.verification().outcome_ref {
        return Err(invalid("scheduled request does not identify the actual verified core input"));
    }
    let mut scheduled = selected.transition;
    let campaign = super::simulation::adversarial::planner_campaign(input.replay, input.inventory, input.now)?;
    let parity = super::simulation::compare_live_plan(input.replay, input.inventory, input.now, input.live_session)?;
    let output = crate::preserves_rail::record("federation-installed-evaluator-output-v1", vec![campaign, parity]);
    scheduled.record.semantic_output_ref = crate::preserves_rail::canonical_hash(&output)?;
    let terminal = molten_core::fabric_simulation::finish_simulation_scheduler(world, &scheduled.next)
        .map_err(|issue| invalid(&format!("federation scheduler completion denied: {issue:?}")))?;
    Ok(crate::preserves_rail::record("federation-installed-simulation-proof-v1", vec![
        crate::preserves_rail::string(profile.profile_ref.as_str()),
        crate::preserves_rail::string(&world_artifact.world_ref),
        world_artifact.value,
        crate::preserves_rail::string(&profile.core.manifest_ref),
        crate::preserves_rail::string(&scheduled.record.selected.node_id),
        crate::preserves_rail::string(&scheduled.record.selected.choice_id),
        crate::preserves_rail::u64_value(scheduled.record.position),
        crate::preserves_rail::u64_value(scheduled.record.virtual_tick),
        crate::preserves_rail::string(&scheduled.record.semantic_output_ref),
        output,
        crate::preserves_rail::u64_value(terminal.next_choice_position),
        crate::preserves_rail::bool_value(terminal.terminal),
        crate::preserves_rail::string(
            "actual owned-core evaluator; recorded crypto inputs; not live networking or authority",
        ),
    ]))
}

fn inputs(
    input: &CampaignInput<'_, '_>,
) -> crate::error::Result<(
    molten_core::fabric_simulation::InstalledWorldInput,
    molten_core::fabric_simulation::InstalledExtensionProfile,
)> {
    let core = identity(input.manifest.manifest_ref())?;
    let profile = extension_profile(input, core.clone())?;
    let profile_ref = profile.profile_ref.clone();
    let (mechanisms, node_id) = mechanisms(input, core)?;
    let nodes = vec![molten_core::fabric_simulation::InstalledNodeBinding {
        node_id: node_id.clone(),
        profile_ref: profile_ref.clone(),
    }];
    let workload = vec![molten_core::fabric_simulation::InstalledWorkloadStep {
        sequence: 0,
        node_id,
        generation: input.replay.generation,
        profile_ref: profile_ref.clone(),
        request_ref: artifact(input.inventory.verification().outcome_ref.clone())?,
    }];
    let invariants = profile
        .required_invariants
        .iter()
        .cloned()
        .map(|invariant| molten_core::fabric_simulation::InstalledInvariantBinding {
            profile_ref: profile_ref.clone(),
            invariant,
        })
        .collect();
    Ok((
        molten_core::fabric_simulation::InstalledWorldInput {
            mechanisms,
            nodes,
            workload,
            invariants,
        },
        profile,
    ))
}

fn extension_profile(
    input: &CampaignInput<'_, '_>,
    core: molten_core::fabric_simulation::ExtensionCoreIdentity,
) -> crate::error::Result<molten_core::fabric_simulation::InstalledExtensionProfile> {
    let handler = artifact(source(
        "federation-invariant-evaluator-v1",
        concat!(
            include_str!("simulation.rs"),
            include_str!("simulation/adversarial.rs"),
            include_str!("simulationworld.rs")
        ),
    )?)?;
    let invariants = [
        "receiver-owned-no-push",
        "generation-and-freshness",
        "resource-retry-bound",
        "conflict-preservation",
        "admission-before-commit",
        "restart-preservation",
        "live-plan-equality",
    ]
    .into_iter()
    .map(|name| {
        Ok(molten_core::fabric_simulation::InstalledInvariant {
            invariant_ref: artifact(source("federation-owned-invariant-v1", name)?)?,
            handler_ref: handler.clone(),
        })
    })
    .collect::<crate::error::Result<Vec<_>>>()?;
    let extension_id = molten_core::nominal::ServiceRef::new(&input.manifest.extension().manifest().extension_id)
        .map_err(|_| invalid("federation extension identity is not nominal"))?;
    let mut profile = molten_core::fabric_simulation::InstalledExtensionProfile {
        profile_ref: artifact(core.manifest_ref.clone())?,
        extension_id,
        service_generation: input.replay.generation,
        core,
        required_invariants: invariants,
        max_nodes: 1,
        max_steps: 1,
    };
    profile.profile_ref = crate::fabric_simulation::extension_profile_ref(&profile)?;
    Ok(profile)
}

/// Returns the single-node simulation mechanisms and the receiver node identity.
fn mechanisms(
    input: &CampaignInput<'_, '_>,
    core: molten_core::fabric_simulation::ExtensionCoreIdentity,
) -> crate::error::Result<(molten_core::fabric_simulation::SimulatedWorldManifest, String)> {
    let mut mechanisms = crate::fabric_simulation::reference_world_manifest()?;
    mechanisms.runtime_ref = core.implementation_ref.clone();
    mechanisms.policy_ref = input.manifest.local_policy_ref().to_string();
    mechanisms.initial_durable_state_ref = initial_state_ref(input)?;
    mechanisms.workload.clear();
    mechanisms.faults.clear();
    mechanisms.nodes.truncate(1);
    mechanisms.invariants = molten_core::fabric_simulation::REQUIRED_UNIVERSAL_INVARIANTS
        .iter()
        .copied()
        .map(molten_core::fabric_simulation::SimulationInvariant::Universal)
        .collect();
    mechanisms.bounds.max_choices = 1;
    mechanisms.bounds.max_virtual_ticks = input
        .now
        .checked_add(input.replay.bounds.session_ticks)
        .ok_or_else(|| invalid("federation virtual-time bound overflow"))?;
    let node = mechanisms.nodes.first_mut().ok_or_else(|| invalid("simulation mechanisms lack a node"))?;
    node.node_id = "federation-receiver".to_string();
    node.generation = input.replay.generation;
    node.extension_id = input.manifest.extension().manifest().extension_id.clone();
    node.service_id = input.manifest.extension().manifest().service_id.clone();
    node.initial_state_ref = mechanisms.initial_durable_state_ref.clone();
    node.same_core = molten_core::fabric_simulation::SameCoreWitness {
        live: core.clone(),
        simulation: core,
    };
    let node_id = node.node_id.clone();
    Ok((mechanisms, node_id))
}

fn identity(manifest_ref: &str) -> crate::error::Result<molten_core::fabric_simulation::ExtensionCoreIdentity> {
    Ok(molten_core::fabric_simulation::ExtensionCoreIdentity {
        implementation_ref: source(
            "federation-implementation-v1",
            concat!(include_str!("lifecycle/mod.rs"), include_str!("transfer/execution.rs")),
        )?,
        manifest_ref: manifest_ref.to_string(),
        callback_dispatcher_ref: source("federation-dispatcher-v1", include_str!("lifecycle/extension.rs"))?,
        protocol_core_ref: source(
            "federation-protocol-v1",
            concat!(
                include_str!("envelope/records.rs"),
                include_str!("envelope/signatures.rs"),
                include_str!("envelope/wire.rs")
            ),
        )?,
        state_machine_ref: source(
            "federation-state-machine-v1",
            concat!(
                include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/crates/molten-core/src/federation/planner.rs")),
                include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/crates/molten-core/src/federation/session.rs"))
            ),
        )?,
        schema_set_ref: source(
            "federation-schema-v1",
            include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/crates/molten-core/src/federation/model.rs")),
        )?,
        port_contract_set_ref: source(
            "federation-port-contracts-v1",
            concat!(include_str!("envelope/network.rs"), include_str!("transfer/import.rs")),
        )?,
    })
}
fn source(label: &'static str, bytes: &str) -> crate::error::Result<String> {
    crate::preserves_rail::canonical_hash(&crate::preserves_rail::record(label, vec![crate::preserves_rail::string(
        bytes,
    )]))
}
fn artifact(reference: String) -> crate::error::Result<molten_core::nominal::ArtifactRef> {
    molten_core::nominal::ArtifactRef::new(reference).map_err(|_| invalid("installed artifact identity is malformed"))
}
fn invalid(message: &str) -> crate::error::Failure {
    crate::error::Failure::invalid_harness(message)
}

fn initial_state_ref(input: &CampaignInput<'_, '_>) -> crate::error::Result<String> {
    if input.replay.local.len() > molten_core::federation::MAX_FEDERATION_ENTRIES {
        return Err(invalid("installed initial inventory exceeds bound"));
    }
    let local = crate::preserves_rail::sequence(
        input
            .replay
            .local
            .iter()
            .map(|entry| {
                crate::preserves_rail::record("local-entry", vec![
                    crate::preserves_rail::string(&entry.name),
                    crate::preserves_rail::string(&entry.head_ref),
                    crate::preserves_rail::string(&entry.ancestry_ref),
                    crate::preserves_rail::string(&entry.policy_ref),
                    crate::preserves_rail::u64_value(entry.bytes),
                    crate::preserves_rail::u64_value(entry.dag_nodes),
                    crate::preserves_rail::u64_value(entry.chunks),
                ])
            })
            .collect(),
    );
    let session = input
        .replay
        .session
        .as_ref()
        .map_or_else(|| crate::preserves_rail::record("none", Vec::new()), super::lifecycle::checkpoint::encode);
    let last_query = input
        .replay
        .last_query
        .map_or_else(|| crate::preserves_rail::record("none", Vec::new()), crate::preserves_rail::u64_value);
    crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("federation-initial-state-v1", vec![
        crate::preserves_rail::string(input.replay.manifest_ref),
        crate::preserves_rail::u64_value(input.replay.generation),
        local,
        session,
        last_query,
        super::lifecycle::resources::encode(&input.replay.resource_history)?,
    ]))
}

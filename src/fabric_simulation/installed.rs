//! Canonical artifact for separately admitted installed-consumer worlds.
use super::*;

// r[impl molten.fabric_simulation.installed_extension_admission]
pub fn admit_extension_world(
    input: &InstalledWorldInput,
    registry: &[InstalledExtensionProfile],
) -> crate::error::Result<SimulatedWorld> {
    for profile in registry {
        if extension_profile_ref(profile)? != profile.profile_ref {
            return Err(crate::error::Failure::invalid_harness("installed simulation profile identity mismatch"));
        }
    }
    let admitted = admit_installed_world(input, registry)
        .map_err(|_| crate::error::Failure::invalid_harness("installed simulation world denied"))?;
    let value = extension_world_value(&admitted)?;
    let world_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(SimulatedWorld {
        world_ref,
        admitted,
        value,
    })
}
pub fn extension_world_value(world: &AdmittedSimulatedWorld) -> crate::error::Result<preserves::IOValue> {
    let installed = world
        .installed()
        .ok_or_else(|| crate::error::Failure::invalid_harness("not an installed simulation world"))?;
    let input = installed.input();
    Ok(crate::preserves_rail::record("fabric-installed-extension-world-v1", vec![
        super::canonical::world_value(&world.manifest),
        crate::preserves_rail::sequence(installed.profiles().iter().map(profile_value).collect()),
        crate::preserves_rail::sequence(
            input
                .nodes
                .iter()
                .map(|binding| {
                    crate::preserves_rail::record("installed-node-binding-v1", vec![
                        crate::preserves_rail::string(&binding.node_id),
                        crate::preserves_rail::string(binding.profile_ref.as_str()),
                    ])
                })
                .collect(),
        ),
        crate::preserves_rail::sequence(
            input
                .workload
                .iter()
                .map(|step| {
                    crate::preserves_rail::record("installed-workload-step-v1", vec![
                        crate::preserves_rail::u64_value(step.sequence),
                        crate::preserves_rail::string(&step.node_id),
                        crate::preserves_rail::u64_value(step.generation),
                        crate::preserves_rail::string(step.profile_ref.as_str()),
                        crate::preserves_rail::string(step.request_ref.as_str()),
                    ])
                })
                .collect(),
        ),
        crate::preserves_rail::sequence(
            input
                .invariants
                .iter()
                .map(|binding| {
                    crate::preserves_rail::record("installed-invariant-binding-v1", vec![
                        crate::preserves_rail::string(binding.profile_ref.as_str()),
                        invariant_value(&binding.invariant),
                    ])
                })
                .collect(),
        ),
    ]))
}
fn invariant_value(invariant: &InstalledInvariant) -> preserves::IOValue {
    crate::preserves_rail::record("installed-invariant-v1", vec![
        crate::preserves_rail::string(invariant.invariant_ref.as_str()),
        crate::preserves_rail::string(invariant.handler_ref.as_str()),
    ])
}
fn profile_value(profile: &InstalledExtensionProfile) -> preserves::IOValue {
    crate::preserves_rail::record("installed-extension-profile-v1", vec![
        crate::preserves_rail::string(profile.profile_ref.as_str()),
        profile_body_value(profile),
    ])
}

pub fn extension_profile_ref(
    profile: &InstalledExtensionProfile,
) -> crate::error::Result<molten_core::nominal::ArtifactRef> {
    molten_core::nominal::ArtifactRef::new(crate::preserves_rail::canonical_hash(&profile_body_value(profile))?)
        .map_err(|_| crate::error::Failure::invalid_harness("invalid installed simulation profile identity"))
}
fn profile_body_value(profile: &InstalledExtensionProfile) -> preserves::IOValue {
    let mut invariants: Vec<_> = profile.required_invariants.iter().collect();
    invariants.sort();
    crate::preserves_rail::record("installed-extension-profile-body-v1", vec![
        crate::preserves_rail::string(profile.extension_id.as_str()),
        crate::preserves_rail::u64_value(profile.service_generation),
        core_value(&profile.core),
        crate::preserves_rail::sequence(invariants.into_iter().map(invariant_value).collect()),
        crate::preserves_rail::u64_value(profile.max_nodes),
        crate::preserves_rail::u64_value(profile.max_steps),
    ])
}
fn core_value(core: &ExtensionCoreIdentity) -> preserves::IOValue {
    crate::preserves_rail::record(
        "installed-extension-core-v1",
        [
            &core.implementation_ref,
            &core.manifest_ref,
            &core.callback_dispatcher_ref,
            &core.protocol_core_ref,
            &core.state_machine_ref,
            &core.schema_set_ref,
            &core.port_contract_set_ref,
        ]
        .into_iter()
        .map(crate::preserves_rail::string)
        .collect(),
    )
}

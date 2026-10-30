//! Generic installed-consumer admission; no reference-product exception and no callbacks.
use std::collections;

use super::*;

pub const MAX_INSTALLED_SIMULATION_ITEMS: usize = 128;
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct InstalledInvariant {
    pub invariant_ref: crate::nominal::ArtifactRef,
    pub handler_ref: crate::nominal::ArtifactRef,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledExtensionProfile {
    pub profile_ref: crate::nominal::ArtifactRef,
    pub extension_id: crate::nominal::ServiceRef,
    pub service_generation: u64,
    pub core: ExtensionCoreIdentity,
    pub required_invariants: Vec<InstalledInvariant>,
    pub max_nodes: u64,
    pub max_steps: u64,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledNodeBinding {
    pub node_id: String,
    pub profile_ref: crate::nominal::ArtifactRef,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledWorkloadStep {
    pub sequence: u64,
    pub node_id: String,
    pub generation: u64,
    pub profile_ref: crate::nominal::ArtifactRef,
    pub request_ref: crate::nominal::ArtifactRef,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledInvariantBinding {
    pub profile_ref: crate::nominal::ArtifactRef,
    pub invariant: InstalledInvariant,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledWorldInput {
    pub mechanisms: SimulatedWorldManifest,
    pub nodes: Vec<InstalledNodeBinding>,
    pub workload: Vec<InstalledWorkloadStep>,
    pub invariants: Vec<InstalledInvariantBinding>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmittedInstalledWorld {
    input: InstalledWorldInput,
    profiles: Vec<InstalledExtensionProfile>,
}
impl AdmittedInstalledWorld {
    pub fn input(&self) -> &InstalledWorldInput {
        &self.input
    }
    pub fn profiles(&self) -> &[InstalledExtensionProfile] {
        &self.profiles
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstalledWorldIssue {
    Mechanism(Vec<WorldIssue>),
    Bound,
    ReferenceWorkload,
    ReferenceInvariant,
    UnknownProfile,
    DuplicateProfile,
    InvalidProfile,
    MissingNode,
    DuplicateNode,
    WrongCore,
    WrongExtension,
    MissingInvariant,
    WrongInvariant,
    DuplicateInvariant,
    WorkloadSequence,
    WorkloadGeneration,
}

// r[impl molten.fabric_simulation.installed_extension_admission]
// r[impl molten.fabric_simulation.installed_extension_invariant_bindings]
// r[impl molten.fabric_simulation.installed_extension_same_core]
// r[impl molten.fabric_simulation.installed_extension_bounded_execution]
pub fn admit_installed_world(
    input: &InstalledWorldInput,
    registry: &[InstalledExtensionProfile],
) -> Result<AdmittedSimulatedWorld, InstalledWorldIssue> {
    let issues = super::admission::validate_installed_mechanisms(&input.mechanisms);
    if !issues.is_empty() {
        return Err(InstalledWorldIssue::Mechanism(issues));
    }
    validate_shape(input, registry)?;
    let profiles = profile_registry(registry)?;
    let selected = validate_nodes(input, &profiles)?;
    validate_workload(input, &profiles, &selected)?;
    validate_invariants(input, &profiles, &selected)?;
    let mut normalized = input.clone();
    normalized.mechanisms = super::admission::normalize_mechanisms(&input.mechanisms);
    normalized.nodes.sort_by(|left, right| left.node_id.cmp(&right.node_id));
    normalized.workload.sort_by_key(|step| step.sequence);
    normalized
        .invariants
        .sort_by(|left, right| (&left.profile_ref, &left.invariant).cmp(&(&right.profile_ref, &right.invariant)));
    let selected_profiles = selected.iter().map(|reference| (*profiles[reference]).clone()).collect();
    Ok(AdmittedSimulatedWorld {
        manifest: normalized.mechanisms.clone(),
        installed: Some(AdmittedInstalledWorld {
            input: normalized,
            profiles: selected_profiles,
        }),
    })
}
fn validate_shape(
    input: &InstalledWorldInput,
    registry: &[InstalledExtensionProfile],
) -> Result<(), InstalledWorldIssue> {
    if registry.is_empty()
        || registry.len() > MAX_INSTALLED_SIMULATION_ITEMS
        || input.nodes.is_empty()
        || input.nodes.len() > MAX_INSTALLED_SIMULATION_ITEMS
        || input.workload.is_empty()
        || input.workload.len() as u64 > input.mechanisms.bounds.max_choices
        || input.invariants.len() > MAX_INSTALLED_SIMULATION_ITEMS
    {
        return Err(InstalledWorldIssue::Bound);
    }
    if !input.mechanisms.workload.is_empty() {
        return Err(InstalledWorldIssue::ReferenceWorkload);
    }
    if input
        .mechanisms
        .invariants
        .iter()
        .any(|invariant| matches!(invariant, SimulationInvariant::ExtensionSemantic { .. }))
    {
        return Err(InstalledWorldIssue::ReferenceInvariant);
    }
    Ok(())
}
fn profile_registry(
    registry: &[InstalledExtensionProfile],
) -> Result<collections::BTreeMap<crate::nominal::ArtifactRef, &InstalledExtensionProfile>, InstalledWorldIssue> {
    let mut profiles = collections::BTreeMap::new();
    for profile in registry {
        if profile.service_generation == 0
            || profile.max_nodes == 0
            || profile.max_nodes > MAX_INSTALLED_SIMULATION_ITEMS as u64
            || profile.max_steps == 0
            || profile.max_steps > 4096
            || profile.required_invariants.is_empty()
            || profile.required_invariants.len() > MAX_INSTALLED_SIMULATION_ITEMS
        {
            return Err(InstalledWorldIssue::InvalidProfile);
        }
        if [
            &profile.core.implementation_ref,
            &profile.core.manifest_ref,
            &profile.core.callback_dispatcher_ref,
            &profile.core.protocol_core_ref,
            &profile.core.state_machine_ref,
            &profile.core.schema_set_ref,
            &profile.core.port_contract_set_ref,
        ]
        .into_iter()
        .any(|reference| !crate::fabric::valid_blake3_ref(reference))
        {
            return Err(InstalledWorldIssue::InvalidProfile);
        }
        let unique: collections::BTreeSet<_> =
            profile.required_invariants.iter().map(|row| &row.invariant_ref).collect();
        if unique.len() != profile.required_invariants.len() {
            return Err(InstalledWorldIssue::DuplicateInvariant);
        }
        if profiles.insert(profile.profile_ref.clone(), profile).is_some() {
            return Err(InstalledWorldIssue::DuplicateProfile);
        }
    }
    Ok(profiles)
}
fn validate_nodes(
    input: &InstalledWorldInput,
    registry: &collections::BTreeMap<crate::nominal::ArtifactRef, &InstalledExtensionProfile>,
) -> Result<collections::BTreeSet<crate::nominal::ArtifactRef>, InstalledWorldIssue> {
    if input.nodes.len() != input.mechanisms.nodes.len() {
        return Err(InstalledWorldIssue::MissingNode);
    }
    let mut nodes = collections::BTreeSet::new();
    let mut selected = collections::BTreeSet::new();
    for binding in &input.nodes {
        if !nodes.insert(&binding.node_id) {
            return Err(InstalledWorldIssue::DuplicateNode);
        }
        let node = input
            .mechanisms
            .nodes
            .iter()
            .find(|node| node.node_id == binding.node_id)
            .ok_or(InstalledWorldIssue::MissingNode)?;
        let profile = registry.get(&binding.profile_ref).ok_or(InstalledWorldIssue::UnknownProfile)?;
        if node.extension_id != profile.extension_id.as_str() {
            return Err(InstalledWorldIssue::WrongExtension);
        }
        if node.generation != profile.service_generation {
            return Err(InstalledWorldIssue::WorkloadGeneration);
        }
        if node.same_core.live != profile.core || node.same_core.simulation != profile.core {
            return Err(InstalledWorldIssue::WrongCore);
        }
        selected.insert(binding.profile_ref.clone());
    }
    for reference in &selected {
        let count = input.nodes.iter().filter(|binding| &binding.profile_ref == reference).count();
        if count as u64 > registry[reference].max_nodes {
            return Err(InstalledWorldIssue::Bound);
        }
    }
    Ok(selected)
}
fn validate_workload(
    input: &InstalledWorldInput,
    registry: &collections::BTreeMap<crate::nominal::ArtifactRef, &InstalledExtensionProfile>,
    selected: &collections::BTreeSet<crate::nominal::ArtifactRef>,
) -> Result<(), InstalledWorldIssue> {
    let mut sequences: Vec<_> = input.workload.iter().collect();
    sequences.sort_by_key(|step| step.sequence);
    for (sequence, step) in sequences.into_iter().enumerate() {
        if step.sequence != sequence as u64 {
            return Err(InstalledWorldIssue::WorkloadSequence);
        }
        let binding = input
            .nodes
            .iter()
            .find(|binding| binding.node_id == step.node_id)
            .ok_or(InstalledWorldIssue::MissingNode)?;
        if binding.profile_ref != step.profile_ref {
            return Err(InstalledWorldIssue::UnknownProfile);
        }
        let node = input
            .mechanisms
            .nodes
            .iter()
            .find(|node| node.node_id == step.node_id)
            .ok_or(InstalledWorldIssue::MissingNode)?;
        if node.generation != step.generation {
            return Err(InstalledWorldIssue::WorkloadGeneration);
        }
    }
    for reference in selected {
        if input.workload.iter().filter(|step| &step.profile_ref == reference).count() as u64
            > registry[reference].max_steps
        {
            return Err(InstalledWorldIssue::Bound);
        }
    }
    Ok(())
}
fn validate_invariants(
    input: &InstalledWorldInput,
    registry: &collections::BTreeMap<crate::nominal::ArtifactRef, &InstalledExtensionProfile>,
    selected: &collections::BTreeSet<crate::nominal::ArtifactRef>,
) -> Result<(), InstalledWorldIssue> {
    let mut seen = collections::BTreeSet::new();
    for binding in &input.invariants {
        if !selected.contains(&binding.profile_ref) {
            return Err(InstalledWorldIssue::UnknownProfile);
        }
        if !seen.insert((&binding.profile_ref, &binding.invariant.invariant_ref)) {
            return Err(InstalledWorldIssue::DuplicateInvariant);
        }
        let profile = registry.get(&binding.profile_ref).ok_or(InstalledWorldIssue::UnknownProfile)?;
        if !profile.required_invariants.contains(&binding.invariant) {
            return Err(InstalledWorldIssue::WrongInvariant);
        }
    }
    for reference in selected {
        for required in &registry[reference].required_invariants {
            if !input
                .invariants
                .iter()
                .any(|binding| &binding.profile_ref == reference && &binding.invariant == required)
            {
                return Err(InstalledWorldIssue::MissingInvariant);
            }
        }
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledEligibleChoice {
    pub choice: EligibleChoice,
    pub workload_sequence: u64,
    pub profile_ref: crate::nominal::ArtifactRef,
    pub request_ref: crate::nominal::ArtifactRef,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledChoicePlan {
    pub transition: SimulationSchedulerTransition,
    pub workload: InstalledWorkloadStep,
}
// r[impl molten.fabric_simulation.installed_extension_bounded_execution]
pub fn select_installed_workload_choice(
    world: &AdmittedSimulatedWorld,
    scheduler: &SimulationSchedulerState,
    eligible: &[InstalledEligibleChoice],
    recorded: Option<&str>,
) -> Result<InstalledChoicePlan, InstalledWorldIssue> {
    let installed = world.installed().ok_or(InstalledWorldIssue::UnknownProfile)?;
    if eligible.is_empty() || eligible.len() > MAX_INSTALLED_SIMULATION_ITEMS {
        return Err(InstalledWorldIssue::Bound);
    }
    for candidate in eligible {
        let step = installed
            .input
            .workload
            .iter()
            .find(|step| step.sequence == candidate.workload_sequence)
            .ok_or(InstalledWorldIssue::WorkloadSequence)?;
        if step.node_id != candidate.choice.node_id || step.generation != candidate.choice.generation {
            return Err(InstalledWorldIssue::WorkloadGeneration);
        }
        if step.profile_ref != candidate.profile_ref || step.request_ref != candidate.request_ref {
            return Err(InstalledWorldIssue::UnknownProfile);
        }
    }
    let choices: Vec<_> = eligible.iter().map(|candidate| candidate.choice.clone()).collect();
    let transition = select_simulation_choice(world, scheduler, &choices, recorded)
        .map_err(|_| InstalledWorldIssue::WorkloadSequence)?;
    let candidate = eligible
        .iter()
        .find(|candidate| candidate.choice == transition.record.selected)
        .ok_or(InstalledWorldIssue::WorkloadSequence)?;
    let workload = installed
        .input
        .workload
        .iter()
        .find(|step| step.sequence == candidate.workload_sequence)
        .ok_or(InstalledWorldIssue::WorkloadSequence)?
        .clone();
    Ok(InstalledChoicePlan { transition, workload })
}

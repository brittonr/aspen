#![allow(
    tigerstyle::non_trait_imports,
    tigerstyle::path_segment_repetition,
    tigerstyle::renamed_imports,
    reason = "the narrow external adapter keeps protocol DTO names and a visible ChaosControl namespace at the translation boundary"
)]

use chaoscontrol_snapshot_descriptor as chaos;
use molten_core::world_commit::RootKind;
use molten_core::world_commit::SnapshotCohortRef;
use molten_core::world_commit::SnapshotProfileRef;
use molten_core::world_commit::WorldCommitRef;
use molten_core::world_commit::WorldRootRef;
use molten_core::world_snapshot::CohortFact;
use molten_core::world_snapshot::CohortFactKind;
use molten_core::world_snapshot::ComponentOwner;
use molten_core::world_snapshot::SnapshotClass;
use molten_core::world_snapshot::SnapshotCohort;
use molten_core::world_snapshot::SnapshotComponent;
use molten_core::world_snapshot::SnapshotComponentKind;
use molten_core::world_snapshot::SnapshotDescriptor;
use serde::Serialize;

use crate::error::MoltenError;
use crate::error::Result;

const COHORT_IDENTITY_DOMAIN: &str = "onixresearch.molten.world-snapshot.chaoscontrol-cohort.v1";
const COMPONENT_IDENTITY_DOMAIN: &str = "onixresearch.molten.world-snapshot.chaoscontrol-component.v1";
const FACT_IDENTITY_DOMAIN: &str = "onixresearch.molten.world-snapshot.chaoscontrol-fact.v1";

/// Molten-owned roots and reviewed compatibility bindings for one ChaosControl descriptor.
#[derive(Debug, Clone)]
pub struct ChaosControlSnapshotBinding {
    pub commit_ref: WorldCommitRef,
    pub profile_ref: SnapshotProfileRef,
    pub artifact_root: WorldRootRef,
    pub schema_root: WorldRootRef,
    pub runtime_profile_root: WorldRootRef,
    pub policy_root: WorldRootRef,
    pub runtime_abi_ref: String,
    pub memory_format_ref: String,
    pub disk_format_ref: String,
    pub backend_profile_ref: String,
}

/// Maps one validated portable ChaosControl descriptor into Molten's closed opaque profile.
///
/// # Errors
///
/// Returns a bounded error for descriptor drift, non-BLAKE3 descriptor identity,
/// invalid Molten bindings, or identity projection failure.
// r[impl molten.world_snapshot.opaque]
pub fn map_chaoscontrol_snapshot(
    envelope: &chaos::SnapshotDescriptorEnvelope,
    binding: ChaosControlSnapshotBinding,
) -> Result<SnapshotDescriptor> {
    chaos::validate_descriptor(&envelope.descriptor)
        .map_err(|error| MoltenError::invalid_harness(format!("ChaosControl descriptor denied: {error:?}")))?;
    let observed_id = chaos::descriptor_identity(&envelope.descriptor)
        .map_err(|error| MoltenError::invalid_harness(format!("ChaosControl descriptor identity failed: {error:?}")))?;
    if observed_id != envelope.descriptor_id {
        return Err(MoltenError::invalid_harness("ChaosControl descriptor envelope identity drifted"));
    }
    validate_binding(&binding)?;
    let descriptor_ref = tagged_blake3_ref(&envelope.descriptor_id)?;
    let facts = cohort_facts(&envelope.descriptor, &binding)?;
    let cohort_ref = identify_cohort(&facts)?;
    let components = descriptor_components(&envelope.descriptor, &binding, &descriptor_ref)?;
    Ok(SnapshotDescriptor {
        class: SnapshotClass::Opaque,
        commit_ref: binding.commit_ref,
        profile_ref: binding.profile_ref,
        cohort: SnapshotCohort { cohort_ref, facts },
        components,
        contains_live_handle: false,
        synchronization: None,
    })
}

fn cohort_facts(
    descriptor: &chaos::SnapshotDescriptor,
    binding: &ChaosControlSnapshotBinding,
) -> Result<Vec<CohortFact>> {
    Ok(vec![
        fact(CohortFactKind::Architecture, json_ref("architecture", &descriptor.architecture)?),
        fact(CohortFactKind::RuntimeBuild, json_ref("runtime-build", &descriptor.runtime.runtime_build)?),
        fact(CohortFactKind::RuntimeAbi, binding.runtime_abi_ref.clone()),
        fact(
            CohortFactKind::KvmStateProfile,
            json_ref(
                "kvm-state-profile",
                &(
                    &descriptor.completeness_profile,
                    descriptor.state_schema_version,
                    &descriptor.runtime.kvm_operations,
                ),
            )?,
        ),
        fact(
            CohortFactKind::CpuFeatureInventory,
            json_ref("cpu-feature-inventory", &(&descriptor.topology.msr_indices, &descriptor.runtime.kvm_operations))?,
        ),
        fact(
            CohortFactKind::VcpuTopology,
            json_ref("vcpu-topology", &(descriptor.topology.vcpu_count, descriptor.topology.memory_bytes))?,
        ),
        fact(CohortFactKind::DeviceInventory, json_ref("device-inventory", &descriptor.topology.devices)?),
        fact(CohortFactKind::MemoryFormat, binding.memory_format_ref.clone()),
        fact(CohortFactKind::DiskFormat, binding.disk_format_ref.clone()),
        fact(CohortFactKind::BackendProfile, binding.backend_profile_ref.clone()),
    ])
}

fn descriptor_components(
    descriptor: &chaos::SnapshotDescriptor,
    binding: &ChaosControlSnapshotBinding,
    descriptor_ref: &str,
) -> Result<Vec<SnapshotComponent>> {
    Ok(vec![
        rooted(SnapshotComponentKind::Artifact, &binding.artifact_root, ComponentOwner::Molten),
        rooted(SnapshotComponentKind::Schema, &binding.schema_root, ComponentOwner::Molten),
        rooted(SnapshotComponentKind::RuntimeProfile, &binding.runtime_profile_root, ComponentOwner::Molten),
        rooted(SnapshotComponentKind::Policy, &binding.policy_root, ComponentOwner::Molten),
        opaque_machine_component(descriptor_ref)?,
        opaque_component(
            SnapshotComponentKind::CpuState,
            json_component_ref("cpu-state", &descriptor.topology.msr_indices)?,
        ),
        opaque_component(SnapshotComponentKind::Memory, json_component_ref("memory", &descriptor.payload)?),
        opaque_component(
            SnapshotComponentKind::DeviceState,
            json_component_ref("device-state", &descriptor.topology.devices)?,
        ),
        opaque_component(
            SnapshotComponentKind::DiskState,
            json_component_ref("disk-state", &descriptor.guest_artifacts)?,
        ),
        opaque_component(
            SnapshotComponentKind::BackendState,
            json_component_ref(
                "backend-state",
                &(
                    &descriptor.runtime.scheduler_profile,
                    &descriptor.runtime.time_profile,
                    &descriptor.runtime.entropy_profile,
                    &descriptor.state_owners,
                ),
            )?,
        ),
    ])
}

fn rooted(kind: SnapshotComponentKind, root: &WorldRootRef, owner: ComponentOwner) -> SnapshotComponent {
    SnapshotComponent {
        kind,
        identity: root.as_str().to_string(),
        root: Some(root.clone()),
        owner,
    }
}

fn opaque_machine_component(identity: &str) -> Result<SnapshotComponent> {
    let root = WorldRootRef::parse(RootKind::OpaqueMachineSnapshot, identity.to_string())
        .map_err(|issue| MoltenError::invalid_harness(format!("ChaosControl machine root denied: {issue:?}")))?;
    Ok(SnapshotComponent {
        kind: SnapshotComponentKind::MachineDescriptor,
        identity: identity.to_string(),
        root: Some(root),
        owner: ComponentOwner::ChaosControl,
    })
}

fn opaque_component(kind: SnapshotComponentKind, identity: String) -> SnapshotComponent {
    SnapshotComponent {
        kind,
        identity,
        root: None,
        owner: ComponentOwner::ChaosControl,
    }
}

fn fact(kind: CohortFactKind, identity: String) -> CohortFact {
    CohortFact { kind, identity }
}

fn identify_cohort(facts: &[CohortFact]) -> Result<SnapshotCohortRef> {
    let mut hasher = blake3::Hasher::new_derive_key(COHORT_IDENTITY_DOMAIN);
    for fact in facts {
        update_text(&mut hasher, fact.kind.as_str())?;
        update_text(&mut hasher, &fact.identity)?;
    }
    SnapshotCohortRef::new(format!("blake3:{}", hasher.finalize().to_hex()))
        .map_err(|issue| MoltenError::invalid_harness(format!("ChaosControl cohort identity denied: {issue:?}")))
}

fn json_component_ref<T: Serialize>(label: &str, value: &T) -> Result<String> {
    json_identity(COMPONENT_IDENTITY_DOMAIN, label, value)
}

fn json_ref<T: Serialize>(label: &str, value: &T) -> Result<String> {
    json_identity(FACT_IDENTITY_DOMAIN, label, value)
}

fn json_identity<T: Serialize>(domain: &'static str, label: &str, value: &T) -> Result<String> {
    let bytes = serde_json::to_vec(value).map_err(|error| {
        MoltenError::invalid_harness(format!("ChaosControl identity serialization failed: {error}"))
    })?;
    let mut hasher = blake3::Hasher::new_derive_key(domain);
    update_text(&mut hasher, label)?;
    update_bytes(&mut hasher, &bytes)?;
    Ok(format!("blake3:{}", hasher.finalize().to_hex()))
}

fn tagged_blake3_ref(identity: &chaos::TaggedDigest) -> Result<String> {
    if identity.algorithm != chaos::DigestAlgorithm::Blake3
        || identity.hex.len() != blake3::OUT_LEN * 2
        || !identity.hex.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(MoltenError::invalid_harness("ChaosControl descriptor identity is not canonical BLAKE3"));
    }
    Ok(format!("blake3:{}", identity.hex))
}

fn validate_binding(binding: &ChaosControlSnapshotBinding) -> Result<()> {
    for (field, value) in [
        ("runtime ABI", binding.runtime_abi_ref.as_str()),
        ("memory format", binding.memory_format_ref.as_str()),
        ("disk format", binding.disk_format_ref.as_str()),
        ("backend profile", binding.backend_profile_ref.as_str()),
    ] {
        crate::preserves_rail::validate_content_ref(value)
            .map_err(|_| MoltenError::invalid_harness(format!("ChaosControl {field} binding is invalid")))?;
    }
    Ok(())
}

fn update_text(hasher: &mut blake3::Hasher, value: &str) -> Result<()> {
    update_bytes(hasher, value.as_bytes())
}

fn update_bytes(hasher: &mut blake3::Hasher, value: &[u8]) -> Result<()> {
    let length = u64::try_from(value.len())
        .map_err(|_| MoltenError::invalid_harness("ChaosControl identity input exceeds u64"))?;
    hasher.update(&length.to_le_bytes());
    hasher.update(value);
    Ok(())
}

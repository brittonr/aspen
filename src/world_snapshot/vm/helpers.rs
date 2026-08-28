use chaoscontrol_snapshot_descriptor as chaos;
use molten_core::world_snapshot::CloneChild;
use molten_core::world_snapshot::ClonePlanRequest;
use molten_core::world_snapshot::CohortFactKind;
use molten_core::world_snapshot::SnapshotComponentKind;
use molten_core::world_snapshot::SnapshotDescriptor;
use molten_core::world_snapshot::SnapshotIssue;
use serde::Serialize;
use vm_cohort_core as vm;

use super::cohort::VM_COHORT_ADAPTER_DOMAIN;
use super::cohort::VM_COHORT_CONTEXT_DOMAIN;
use super::cohort::VM_COHORT_PLAN_DOMAIN;
use super::cohort::VM_COHORT_PUBLICATION_REVISION;
use super::cohort::VmCohortCloneBinding;
use super::cohort::VmCohortLimits;
use super::cohort::VmCohortPlanProjection;
use crate::error::MoltenError;
use crate::error::Result;

pub(super) fn project_plan(
    descriptor: &SnapshotDescriptor,
    request: &ClonePlanRequest,
    plan: &vm::CohortPlan,
) -> Result<VmCohortPlanProjection> {
    if request.children.len() != plan.clones.len() {
        return Err(MoltenError::invalid_harness("VM Cohort clone count differs from Molten child count"));
    }
    let mechanism_plan_json = serde_json::to_vec(plan)
        .map_err(|error| MoltenError::invalid_harness(format!("VM Cohort plan serialization failed: {error}")))?;
    let plan_ref = json_ref("mechanism-plan", plan)?;
    let mut clones = Vec::with_capacity(plan.clones.len());
    for (index, (child, clone)) in request.children.iter().zip(&plan.clones).enumerate() {
        clones.push(clone_binding(index, child, clone)?);
    }
    Ok(VmCohortPlanProjection {
        plan_ref,
        descriptor_ref: component_ref(descriptor, SnapshotComponentKind::MachineDescriptor)?.to_string(),
        cohort_ref: plan.cohort_ref.as_str().to_string(),
        checkpoint_ref: plan.checkpoint_ref.as_str().to_string(),
        policy_ref: plan.policy_ref.as_str().to_string(),
        worker_count: plan.reservation.workers,
        mechanism_plan_json,
        clones,
        fault_authority_granted: false,
        replay_authority_granted: false,
        release_authority_granted: false,
    })
}

fn clone_binding(index: usize, child: &CloneChild, clone: &vm::ClonePlan) -> Result<VmCohortCloneBinding> {
    let child_index =
        u32::try_from(index).map_err(|_| MoltenError::invalid_harness("VM Cohort child index exceeds u32"))?;
    Ok(VmCohortCloneBinding {
        child_index,
        vm_clone_ref: clone.clone_ref.as_str().to_string(),
        memory_overlay: child.memory_overlay.0.clone(),
        device_overlay: child.device_overlay.0.clone(),
        disk_overlay: child.disk_overlay.0.clone(),
        endpoint_overlay: child.endpoint_overlay.0.clone(),
        vm_surface_refs: clone.surfaces.iter().map(|surface| surface.resource_ref.as_str().to_string()).collect(),
    })
}

pub(super) fn validate_mapped_descriptor(
    descriptor: &SnapshotDescriptor,
    envelope: &chaos::SnapshotDescriptorEnvelope,
) -> Result<()> {
    chaos::validate_descriptor(&envelope.descriptor)
        .map_err(|error| MoltenError::invalid_harness(format!("ChaosControl descriptor denied: {error:?}")))?;
    let observed = chaos::descriptor_identity(&envelope.descriptor)
        .map_err(|error| MoltenError::invalid_harness(format!("ChaosControl identity failed: {error:?}")))?;
    if observed != envelope.descriptor_id
        || component_ref(descriptor, SnapshotComponentKind::MachineDescriptor)? != tagged_ref(&observed)?
    {
        return Err(MoltenError::invalid_harness("VM Cohort ChaosControl descriptor binding drifted"));
    }
    Ok(())
}

pub(super) fn validate_limits(limits: &VmCohortLimits) -> Result<()> {
    let adapter = adapter_ref();
    for value in [limits.policy_ref.as_str(), adapter.as_str()] {
        crate::preserves_rail::validate_content_ref(value)
            .map_err(|_| MoltenError::invalid_harness("VM Cohort policy reference is invalid"))?;
    }
    Ok(())
}

pub(super) fn mechanism_limits(limits: &VmCohortLimits) -> Result<vm::ResourceLimits> {
    Ok(vm::ResourceLimits {
        policy_ref: vm_profile_ref(&limits.policy_ref)?,
        maximum: vm::ResourceVector {
            workers: limits.maximum_workers,
            vcpus: limits.maximum_vcpus,
            memory_bytes: limits.maximum_memory_bytes,
            storage_bytes: limits.maximum_storage_bytes,
            mappings: limits.maximum_mappings,
            endpoints: limits.maximum_endpoints,
            in_flight: limits.maximum_in_flight,
        },
        max_attempts: limits.max_attempts,
        retry_delay_ticks: limits.retry_delay_ticks,
        max_retry_delay_ticks: limits.max_retry_delay_ticks,
        memory_page_bytes: limits.memory_page_bytes,
        disk_page_bytes: limits.disk_page_bytes,
    })
}

pub(super) fn component_ref(descriptor: &SnapshotDescriptor, kind: SnapshotComponentKind) -> Result<&str> {
    descriptor
        .components
        .iter()
        .find(|component| component.kind == kind)
        .map(|component| component.identity.as_str())
        .ok_or_else(|| MoltenError::invalid_harness("VM Cohort descriptor component is missing"))
}

pub(super) fn fact_ref(descriptor: &SnapshotDescriptor, kind: CohortFactKind) -> Result<&str> {
    descriptor
        .cohort
        .facts
        .iter()
        .find(|fact| fact.kind == kind)
        .map(|fact| fact.identity.as_str())
        .ok_or_else(|| MoltenError::invalid_harness("VM Cohort cohort fact is missing"))
}

pub(super) fn guest_artifact_ref(
    descriptor: &chaos::SnapshotDescriptor,
    role: chaos::GuestArtifactRole,
) -> Result<String> {
    let artifact = descriptor
        .guest_artifacts
        .iter()
        .find(|artifact| artifact.role == role)
        .ok_or_else(|| MoltenError::invalid_harness("ChaosControl guest artifact is missing"))?;
    json_ref("guest-artifact", artifact)
}

pub(super) fn context_ref(descriptor: &SnapshotDescriptor, request: &ClonePlanRequest) -> Result<String> {
    let mut hasher = blake3::Hasher::new_derive_key(VM_COHORT_CONTEXT_DOMAIN);
    update_text(&mut hasher, descriptor.commit_ref.as_str())?;
    update_text(&mut hasher, descriptor.cohort.cohort_ref.as_str())?;
    for child in &request.children {
        update_text(&mut hasher, &child.memory_overlay.0)?;
        update_text(&mut hasher, &child.device_overlay.0)?;
        update_text(&mut hasher, &child.disk_overlay.0)?;
        update_text(&mut hasher, &child.endpoint_overlay.0)?;
    }
    Ok(format!("blake3:{}", hasher.finalize().to_hex()))
}

pub(super) fn adapter_ref() -> String {
    let mut hasher = blake3::Hasher::new_derive_key(VM_COHORT_ADAPTER_DOMAIN);
    hasher.update(VM_COHORT_PUBLICATION_REVISION.as_bytes());
    format!("blake3:{}", hasher.finalize().to_hex())
}

pub(super) fn json_ref<T: Serialize>(label: &str, value: &T) -> Result<String> {
    let bytes = serde_json::to_vec(value)
        .map_err(|error| MoltenError::invalid_harness(format!("VM Cohort identity serialization failed: {error}")))?;
    let mut hasher = blake3::Hasher::new_derive_key(VM_COHORT_PLAN_DOMAIN);
    update_text(&mut hasher, label)?;
    update_bytes(&mut hasher, &bytes)?;
    Ok(format!("blake3:{}", hasher.finalize().to_hex()))
}

fn tagged_ref(identity: &chaos::TaggedDigest) -> Result<String> {
    if identity.algorithm != chaos::DigestAlgorithm::Blake3 {
        return Err(MoltenError::invalid_harness("ChaosControl descriptor identity is not BLAKE3"));
    }
    Ok(format!("blake3:{}", identity.hex))
}

pub(super) fn vm_resource_ref(value: &str) -> Result<vm::ResourceRef> {
    vm::ResourceRef::new(value.to_string())
        .map_err(|_| MoltenError::invalid_harness("VM Cohort resource reference is invalid"))
}

pub(super) fn vm_profile_ref(value: &str) -> Result<vm::ProfileRef> {
    vm::ProfileRef::new(value.to_string())
        .map_err(|_| MoltenError::invalid_harness("VM Cohort profile reference is invalid"))
}

pub(super) fn clone_issues(issues: Vec<SnapshotIssue>) -> MoltenError {
    MoltenError::invalid_harness(format!("Molten clone plan denied: {issues:?}"))
}

fn update_text(hasher: &mut blake3::Hasher, value: &str) -> Result<()> {
    update_bytes(hasher, value.as_bytes())
}

fn update_bytes(hasher: &mut blake3::Hasher, value: &[u8]) -> Result<()> {
    let length =
        u64::try_from(value.len()).map_err(|_| MoltenError::invalid_harness("VM Cohort identity input exceeds u64"))?;
    hasher.update(&length.to_le_bytes());
    hasher.update(value);
    Ok(())
}

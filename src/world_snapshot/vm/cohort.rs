use chaoscontrol_snapshot_descriptor as chaos;
use molten_core::world_snapshot::ClonePlanRequest;
use molten_core::world_snapshot::CohortFactKind;
use molten_core::world_snapshot::SnapshotComponentKind;
use molten_core::world_snapshot::SnapshotDescriptor;
use molten_core::world_snapshot::validate_clone_plan;
use serde::Deserialize;
use serde::Serialize;
use vm_cohort_core as vm;

use super::helpers::*;
use crate::error::MoltenError;
use crate::error::Result;

pub const VM_COHORT_PUBLICATION_REVISION: &str = "31f1696ba9391bfda8577a58af84f72361d5573e";
pub(super) const VM_COHORT_PLAN_DOMAIN: &str = "onixresearch.molten.world-snapshot.vm-cohort-plan.v1";
pub(super) const VM_COHORT_CONTEXT_DOMAIN: &str = "onixresearch.molten.world-snapshot.vm-cohort-context.v1";
pub(super) const VM_COHORT_ADAPTER_DOMAIN: &str = "onixresearch.molten.world-snapshot.vm-cohort-adapter.v1";
const IN_KERNEL_DEVICE_COUNT: u32 = 4;

/// Product-owned VM Cohort resource bounds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VmCohortLimits {
    pub policy_ref: String,
    pub maximum_workers: u32,
    pub maximum_vcpus: u32,
    pub maximum_memory_bytes: u64,
    pub maximum_storage_bytes: u64,
    pub maximum_mappings: u32,
    pub maximum_endpoints: u32,
    pub maximum_in_flight: u32,
    pub max_attempts: u32,
    pub retry_delay_ticks: u64,
    pub max_retry_delay_ticks: u64,
    pub memory_page_bytes: u64,
    pub disk_page_bytes: u64,
}

/// One Molten child bound to one VM Cohort clone and its private surfaces.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VmCohortCloneBinding {
    pub child_index: u32,
    pub vm_clone_ref: String,
    pub memory_overlay: String,
    pub device_overlay: String,
    pub disk_overlay: String,
    pub endpoint_overlay: String,
    pub vm_surface_refs: Vec<String>,
}

/// Product projection of one exact VM Cohort mechanism plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VmCohortPlanProjection {
    pub plan_ref: String,
    pub descriptor_ref: String,
    pub cohort_ref: String,
    pub checkpoint_ref: String,
    pub policy_ref: String,
    pub worker_count: u32,
    pub mechanism_plan_json: Vec<u8>,
    pub clones: Vec<VmCohortCloneBinding>,
    pub fault_authority_granted: bool,
    pub replay_authority_granted: bool,
    pub release_authority_granted: bool,
}

/// Bounded observation from the admitted ChaosControl VM Cohort realization adapter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VmCohortRealizationObservation {
    pub plan_ref: String,
    pub cohort_ref: String,
    pub mechanism_receipt_ref: String,
    pub active_clones: u32,
    pub cleanup_uncertain: bool,
    pub fault_authority_granted: bool,
    pub replay_authority_granted: bool,
    pub release_authority_granted: bool,
}

/// Application-owned capability for exact VM Cohort realization through ChaosControl.
pub trait VmCohortRealizationPort {
    fn realize(&mut self, plan: &VmCohortPlanProjection) -> Result<VmCohortRealizationObservation>;
}

/// Plans parent-bound clones through the pinned VM Cohort functional core.
///
/// # Errors
///
/// Returns a bounded error for profile drift, descriptor mismatch, invalid clone
/// isolation, missing disk facts, resource denial, or projection failure.
// r[impl molten.world_snapshot.cow]
pub fn plan_vm_cohort_clones(
    descriptor: &SnapshotDescriptor,
    chaos_descriptor: &chaos::SnapshotDescriptorEnvelope,
    request: &ClonePlanRequest,
    effective_disk_bytes: u64,
    limits: &VmCohortLimits,
) -> Result<VmCohortPlanProjection> {
    if descriptor.class != molten_core::world_snapshot::SnapshotClass::Opaque
        || descriptor.commit_ref != request.parent_ref
    {
        return Err(MoltenError::invalid_harness("VM Cohort planning requires one matching opaque parent"));
    }
    validate_clone_plan(request).map_err(clone_issues)?;
    validate_mapped_descriptor(descriptor, chaos_descriptor)?;
    validate_limits(limits)?;
    if effective_disk_bytes == 0 {
        return Err(MoltenError::invalid_harness("VM Cohort effective disk observation is empty"));
    }

    let compatibility = vm::CompatibilityProfile {
        profile_ref: vm_profile_ref(descriptor.profile_ref.as_str())?,
        architecture: chaos_descriptor.descriptor.architecture.clone(),
        vcpu_state_ref: vm_resource_ref(component_ref(descriptor, SnapshotComponentKind::CpuState)?)?,
        memory_layout_ref: vm_resource_ref(fact_ref(descriptor, CohortFactKind::MemoryFormat)?)?,
        kernel_ref: vm_resource_ref(&guest_artifact_ref(
            &chaos_descriptor.descriptor,
            chaos::GuestArtifactRole::Kernel,
        )?)?,
        guest_image_ref: vm_resource_ref(&json_ref("guest-image", &chaos_descriptor.descriptor.guest_artifacts)?)?,
        device_model_ref: vm_resource_ref(component_ref(descriptor, SnapshotComponentKind::DeviceState)?)?,
        disk_format_ref: vm_resource_ref(fact_ref(descriptor, CohortFactKind::DiskFormat)?)?,
        runtime_ref: vm_resource_ref(fact_ref(descriptor, CohortFactKind::RuntimeBuild)?)?,
        adapter_ref: vm_resource_ref(&adapter_ref())?,
    };
    let device_count = u32::try_from(chaos_descriptor.descriptor.topology.devices.len())
        .map_err(|_| MoltenError::invalid_harness("VM Cohort device count exceeds u32"))?
        .checked_add(IN_KERNEL_DEVICE_COUNT)
        .ok_or_else(|| MoltenError::invalid_harness("VM Cohort device count overflow"))?;
    let checkpoint = vm::admit_checkpoint(&vm::CheckpointCandidate {
        compatibility: compatibility.clone(),
        effective_memory_base_ref: vm_resource_ref(component_ref(descriptor, SnapshotComponentKind::Memory)?)?,
        effective_disk_base_ref: vm_resource_ref(component_ref(descriptor, SnapshotComponentKind::DiskState)?)?,
        memory_bytes: chaos_descriptor.descriptor.topology.memory_bytes,
        disk_bytes: effective_disk_bytes,
        vcpu_count: chaos_descriptor.descriptor.topology.vcpu_count,
        device_count,
        complete: true,
        host_handles_present: descriptor.contains_live_handle,
        bases_mutable: false,
    })
    .map_err(|issues| MoltenError::invalid_harness(format!("VM Cohort checkpoint denied: {issues:?}")))?;
    let worker_count = u32::try_from(request.children.len())
        .map_err(|_| MoltenError::invalid_harness("VM Cohort child count exceeds u32"))?;
    let plan = vm::plan_cohort(&vm::CohortRequest {
        checkpoint,
        expected_compatibility: compatibility,
        workers: worker_count,
        limits: mechanism_limits(limits)?,
        context_ref: vm_resource_ref(&context_ref(descriptor, request)?)?,
    })
    .map_err(|issues| MoltenError::invalid_harness(format!("VM Cohort plan denied: {issues:?}")))?;
    let isolation_issues = vm::validate_clone_isolation(&plan);
    if !isolation_issues.is_empty() {
        return Err(MoltenError::invalid_harness(format!("VM Cohort isolation denied: {isolation_issues:?}")));
    }
    project_plan(descriptor, request, &plan)
}

/// Executes one projected plan through the admitted consumer-owned realization port.
///
/// # Errors
///
/// Returns a bounded error for crossed plans, partial activation, cleanup
/// uncertainty, malformed receipts, or product-authority overclaim.
// r[impl molten.world_snapshot.cow]
pub fn realize_vm_cohort_clones<P: VmCohortRealizationPort>(
    plan: &VmCohortPlanProjection,
    port: &mut P,
) -> Result<VmCohortRealizationObservation> {
    let observation = port.realize(plan)?;
    crate::preserves_rail::validate_content_ref(&observation.mechanism_receipt_ref)
        .map_err(|_| MoltenError::invalid_harness("VM Cohort receipt reference is invalid"))?;
    if observation.plan_ref != plan.plan_ref
        || observation.cohort_ref != plan.cohort_ref
        || observation.active_clones != plan.worker_count
        || observation.cleanup_uncertain
        || observation.fault_authority_granted
        || observation.replay_authority_granted
        || observation.release_authority_granted
    {
        return Err(MoltenError::invalid_harness(
            "VM Cohort realization is partial, crossed, uncertain, or overclaims authority",
        ));
    }
    Ok(observation)
}

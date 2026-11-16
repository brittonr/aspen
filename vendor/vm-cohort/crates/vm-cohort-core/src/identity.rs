use alloc::format;

use crate::{
    CheckpointCandidate, CheckpointRef, CloneRef, CohortRef, EffectKind, MutableSurfaceKind,
    OperationRef, ReferenceError, ResourceLimits, ResourceRef,
};

const CHECKPOINT_DOMAIN: &str = "onixresearch.vm-cohort.checkpoint.v1";
const COHORT_DOMAIN: &str = "onixresearch.vm-cohort.cohort.v1";
const CLONE_DOMAIN: &str = "onixresearch.vm-cohort.clone.v1";
const SURFACE_DOMAIN: &str = "onixresearch.vm-cohort.surface.v1";
const OPERATION_DOMAIN: &str = "onixresearch.vm-cohort.operation.v1";

pub(crate) fn identify_checkpoint(
    candidate: &CheckpointCandidate,
) -> Result<CheckpointRef, ReferenceError> {
    let mut hasher = blake3::Hasher::new_derive_key(CHECKPOINT_DOMAIN);
    update_compatibility(&mut hasher, candidate)?;
    update_text(&mut hasher, candidate.effective_memory_base_ref.as_str())?;
    update_text(&mut hasher, candidate.effective_disk_base_ref.as_str())?;
    update_u64(&mut hasher, candidate.memory_bytes);
    update_u64(&mut hasher, candidate.disk_bytes);
    update_u64(&mut hasher, u64::from(candidate.vcpu_count));
    update_u64(&mut hasher, u64::from(candidate.device_count));
    CheckpointRef::new(format!("blake3:{}", hasher.finalize().to_hex()))
}

pub(crate) fn identify_cohort(
    checkpoint_ref: &CheckpointRef,
    limits: &ResourceLimits,
    workers: u32,
    context_ref: &ResourceRef,
) -> Result<CohortRef, ReferenceError> {
    let mut hasher = blake3::Hasher::new_derive_key(COHORT_DOMAIN);
    update_text(&mut hasher, checkpoint_ref.as_str())?;
    update_text(&mut hasher, limits.policy_ref.as_str())?;
    update_u64(&mut hasher, u64::from(workers));
    update_text(&mut hasher, context_ref.as_str())?;
    update_limits(&mut hasher, limits);
    CohortRef::new(format!("blake3:{}", hasher.finalize().to_hex()))
}

pub(crate) fn identify_clone(
    cohort_ref: &CohortRef,
    ordinal: u32,
) -> Result<CloneRef, ReferenceError> {
    let mut hasher = blake3::Hasher::new_derive_key(CLONE_DOMAIN);
    update_text(&mut hasher, cohort_ref.as_str())?;
    update_u64(&mut hasher, u64::from(ordinal));
    CloneRef::new(format!("blake3:{}", hasher.finalize().to_hex()))
}

pub(crate) fn identify_surface(
    clone_ref: &CloneRef,
    kind: MutableSurfaceKind,
) -> Result<ResourceRef, ReferenceError> {
    let mut hasher = blake3::Hasher::new_derive_key(SURFACE_DOMAIN);
    update_text(&mut hasher, clone_ref.as_str())?;
    update_text(&mut hasher, surface_name(kind))?;
    ResourceRef::new(format!("blake3:{}", hasher.finalize().to_hex()))
}

pub(crate) fn identify_operation(
    cohort_ref: &CohortRef,
    clone_ref: &CloneRef,
    kind: EffectKind,
) -> Result<OperationRef, ReferenceError> {
    let mut hasher = blake3::Hasher::new_derive_key(OPERATION_DOMAIN);
    update_text(&mut hasher, cohort_ref.as_str())?;
    update_text(&mut hasher, clone_ref.as_str())?;
    update_text(&mut hasher, effect_name(kind))?;
    OperationRef::new(format!("blake3:{}", hasher.finalize().to_hex()))
}

pub(crate) fn update_text(hasher: &mut blake3::Hasher, value: &str) -> Result<(), ReferenceError> {
    let length = u64::try_from(value.len()).map_err(|_| ReferenceError)?;
    update_u64(hasher, length);
    hasher.update(value.as_bytes());
    Ok(())
}

pub(crate) fn update_u64(hasher: &mut blake3::Hasher, value: u64) {
    hasher.update(&value.to_be_bytes());
}

fn update_compatibility(
    hasher: &mut blake3::Hasher,
    candidate: &CheckpointCandidate,
) -> Result<(), ReferenceError> {
    let profile = &candidate.compatibility;
    update_text(hasher, profile.profile_ref.as_str())?;
    update_text(hasher, &profile.architecture)?;
    update_text(hasher, profile.vcpu_state_ref.as_str())?;
    update_text(hasher, profile.memory_layout_ref.as_str())?;
    update_text(hasher, profile.kernel_ref.as_str())?;
    update_text(hasher, profile.guest_image_ref.as_str())?;
    update_text(hasher, profile.device_model_ref.as_str())?;
    update_text(hasher, profile.disk_format_ref.as_str())?;
    update_text(hasher, profile.runtime_ref.as_str())?;
    update_text(hasher, profile.adapter_ref.as_str())?;
    Ok(())
}

fn update_limits(hasher: &mut blake3::Hasher, limits: &ResourceLimits) {
    let maximum = &limits.maximum;
    update_u64(hasher, u64::from(maximum.workers));
    update_u64(hasher, u64::from(maximum.vcpus));
    update_u64(hasher, maximum.memory_bytes);
    update_u64(hasher, maximum.storage_bytes);
    update_u64(hasher, u64::from(maximum.mappings));
    update_u64(hasher, u64::from(maximum.endpoints));
    update_u64(hasher, u64::from(maximum.in_flight));
    update_u64(hasher, u64::from(limits.max_attempts));
    update_u64(hasher, limits.retry_delay_ticks);
    update_u64(hasher, limits.max_retry_delay_ticks);
    update_u64(hasher, limits.memory_page_bytes);
    update_u64(hasher, limits.disk_page_bytes);
}

const fn surface_name(kind: MutableSurfaceKind) -> &'static str {
    match kind {
        MutableSurfaceKind::Memory => "memory",
        MutableSurfaceKind::Disk => "disk",
        MutableSurfaceKind::Device => "device",
        MutableSurfaceKind::Endpoint => "endpoint",
        MutableSurfaceKind::Output => "output",
        MutableSurfaceKind::Lifecycle => "lifecycle",
    }
}

const fn effect_name(kind: EffectKind) -> &'static str {
    match kind {
        EffectKind::PrepareMemory => "prepare-memory",
        EffectKind::PrepareDisk => "prepare-disk",
        EffectKind::RestoreVcpu => "restore-vcpu",
        EffectKind::RestoreDevice => "restore-device",
        EffectKind::BindEndpoint => "bind-endpoint",
        EffectKind::Activate => "activate",
    }
}

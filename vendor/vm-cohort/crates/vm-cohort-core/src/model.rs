mod reference;

pub use reference::*;

use alloc::string::String;
use alloc::vec::Vec;

use serde::{Deserialize, Serialize};

/// Complete behavior-relevant checkpoint compatibility facts.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompatibilityProfile {
    /// Profile identity.
    pub profile_ref: ProfileRef,
    /// Host architecture, for example `x86_64`.
    pub architecture: String,
    /// vCPU state contract identity.
    pub vcpu_state_ref: ResourceRef,
    /// Memory layout identity.
    pub memory_layout_ref: ResourceRef,
    /// Kernel artifact identity.
    pub kernel_ref: ResourceRef,
    /// Guest image identity.
    pub guest_image_ref: ResourceRef,
    /// Device model identity.
    pub device_model_ref: ResourceRef,
    /// Disk format identity.
    pub disk_format_ref: ResourceRef,
    /// Runtime build identity.
    pub runtime_ref: ResourceRef,
    /// KVM adapter identity.
    pub adapter_ref: ResourceRef,
}

/// Supplied initialized checkpoint facts before admission.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CheckpointCandidate {
    /// Compatibility profile.
    pub compatibility: CompatibilityProfile,
    /// Effective initialized memory base identity.
    pub effective_memory_base_ref: ResourceRef,
    /// Effective initialized disk base identity.
    pub effective_disk_base_ref: ResourceRef,
    /// Memory base bytes.
    pub memory_bytes: u64,
    /// Disk base bytes.
    pub disk_bytes: u64,
    /// Number of virtual CPUs.
    pub vcpu_count: u32,
    /// Number of modeled devices.
    pub device_count: u32,
    /// Whether every required component was observed.
    pub complete: bool,
    /// Whether a live host handle was captured in checkpoint bytes.
    pub host_handles_present: bool,
    /// Whether the effective bases can still mutate.
    pub bases_mutable: bool,
}

/// Admitted retained checkpoint.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GoldenCheckpoint {
    /// Deterministic checkpoint identity.
    pub checkpoint_ref: CheckpointRef,
    /// Compatibility profile.
    pub compatibility: CompatibilityProfile,
    /// Immutable initialized memory base.
    pub effective_memory_base_ref: ResourceRef,
    /// Immutable initialized disk base.
    pub effective_disk_base_ref: ResourceRef,
    /// Memory base bytes.
    pub memory_bytes: u64,
    /// Disk base bytes.
    pub disk_bytes: u64,
    /// Number of virtual CPUs.
    pub vcpu_count: u32,
    /// Number of modeled devices.
    pub device_count: u32,
    /// Number of completed or failed cohorts that retained this base.
    pub retained_uses: u64,
}

/// Complete resource usage or reservation.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResourceVector {
    /// Worker count.
    pub workers: u32,
    /// Total vCPUs.
    pub vcpus: u32,
    /// Total logical memory bytes.
    pub memory_bytes: u64,
    /// Total logical storage bytes.
    pub storage_bytes: u64,
    /// Total private mappings.
    pub mappings: u32,
    /// Total endpoint identities.
    pub endpoints: u32,
    /// Total concurrent effect operations.
    pub in_flight: u32,
}

/// Explicit upper bounds for one cohort.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResourceLimits {
    /// Resource-policy identity.
    pub policy_ref: ProfileRef,
    /// Maximum complete reservation.
    pub maximum: ResourceVector,
    /// Maximum retry attempts for one operation.
    pub max_attempts: u32,
    /// Delay ticks for one reviewed retry.
    pub retry_delay_ticks: u64,
    /// Maximum retry delay ticks.
    pub max_retry_delay_ticks: u64,
    /// Private memory-overlay page size.
    pub memory_page_bytes: u64,
    /// Private disk-overlay page size.
    pub disk_page_bytes: u64,
}

/// Request for one complete clone cohort.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CohortRequest {
    /// Retained checkpoint.
    pub checkpoint: GoldenCheckpoint,
    /// Expected current compatibility profile.
    pub expected_compatibility: CompatibilityProfile,
    /// Requested worker count.
    pub workers: u32,
    /// Selected resource and retry limits.
    pub limits: ResourceLimits,
    /// Consumer-owned request context identity.
    pub context_ref: ResourceRef,
}

/// Mutable clone surface that must never be shared between clones.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MutableSurfaceKind {
    /// Memory overlay.
    Memory,
    /// Disk overlay.
    Disk,
    /// Device state.
    Device,
    /// Endpoint state.
    Endpoint,
    /// Output state.
    Output,
    /// Lifecycle state.
    Lifecycle,
}

/// One private mutable surface assignment.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SurfacePlan {
    /// Surface class.
    pub kind: MutableSurfaceKind,
    /// Clone-private identity.
    pub resource_ref: ResourceRef,
}

/// One clone plan bound to immutable bases and private mutable surfaces.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClonePlan {
    /// Clone identity.
    pub clone_ref: CloneRef,
    /// Shared immutable memory base.
    pub memory_base_ref: ResourceRef,
    /// Shared immutable disk base.
    pub disk_base_ref: ResourceRef,
    /// Closed private surface set.
    pub surfaces: Vec<SurfacePlan>,
}

/// Planned host effect kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EffectKind {
    /// Prepare private memory over the immutable base.
    PrepareMemory,
    /// Prepare private disk over the immutable base.
    PrepareDisk,
    /// Restore vCPU state.
    RestoreVcpu,
    /// Restore device state.
    RestoreDevice,
    /// Bind private endpoints and output state.
    BindEndpoint,
    /// Activate the clone.
    Activate,
}

/// One pure host effect request.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EffectRequest {
    /// Stable operation identity.
    pub operation_ref: OperationRef,
    /// Target clone.
    pub clone_ref: CloneRef,
    /// Effect kind.
    pub kind: EffectKind,
}

/// Complete reservation and effect plan for one cohort.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CohortPlan {
    /// Cohort identity.
    pub cohort_ref: CohortRef,
    /// Retained checkpoint identity.
    pub checkpoint_ref: CheckpointRef,
    /// Resource-policy identity.
    pub policy_ref: ProfileRef,
    /// Complete resource reservation.
    pub reservation: ResourceVector,
    /// Every planned clone.
    pub clones: Vec<ClonePlan>,
    /// Ordered host effects.
    pub effects: Vec<EffectRequest>,
}

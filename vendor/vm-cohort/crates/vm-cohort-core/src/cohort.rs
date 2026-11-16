mod retry;

pub use retry::*;

use alloc::collections::BTreeSet;
use alloc::vec;
use alloc::vec::Vec;

use crate::{
    ClonePlan, CohortPlan, CohortRequest, EffectKind, EffectRequest, MAX_EFFECTS, MAX_WORKERS,
    MutableSurfaceKind, ReferenceError, ResourceVector, SurfacePlan,
};

const SURFACES: [MutableSurfaceKind; 6] = [
    MutableSurfaceKind::Memory,
    MutableSurfaceKind::Disk,
    MutableSurfaceKind::Device,
    MutableSurfaceKind::Endpoint,
    MutableSurfaceKind::Output,
    MutableSurfaceKind::Lifecycle,
];
const EFFECTS: [EffectKind; 6] = [
    EffectKind::PrepareMemory,
    EffectKind::PrepareDisk,
    EffectKind::RestoreDevice,
    EffectKind::RestoreVcpu,
    EffectKind::BindEndpoint,
    EffectKind::Activate,
];
const MAPPINGS_PER_CLONE: u32 = 2;
const ENDPOINTS_PER_CLONE: u32 = 1;
const FIXED_ISSUE_CAPACITY: usize = 16;

pub(crate) const fn surface_count() -> usize {
    SURFACES.len()
}

/// Cohort admission or planning issue.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum CohortIssue {
    /// Worker count is zero or above the public bound.
    WorkerCount,
    /// Current compatibility differs from the checkpoint.
    Compatibility,
    /// One page size is zero or incompatible with its base.
    PageGeometry,
    /// Checked resource arithmetic overflowed.
    ArithmeticOverflow,
    /// Complete reservation exceeds selected limits.
    ResourceLimit,
    /// Retry policy has no usable attempt.
    RetryPolicy,
    /// Clone or effect count exceeds the public bound.
    PlanLimit,
    /// A mutable surface identity was shared.
    SharedMutableSurface,
    /// Identity construction failed.
    Identity,
}

/// Plans and reserves a complete cohort before any host effect.
///
/// # Errors
///
/// Returns every bounded compatibility, arithmetic, resource, or identity issue.
// r[impl vm_cohort.resource_admission]
// r[impl vm_cohort.clone_isolation]
// r[impl vm_cohort.functional_core]
pub fn plan_cohort(request: &CohortRequest) -> Result<CohortPlan, Vec<CohortIssue>> {
    let mut issues = validate_request(request);
    let reservation = match reserve(request) {
        Ok(reservation) => reservation,
        Err(issue) => {
            issues.push(issue);
            ResourceVector::default()
        }
    };
    if !within_limits(&reservation, &request.limits.maximum) {
        issues.push(CohortIssue::ResourceLimit);
    }
    if !issues.is_empty() {
        issues.sort();
        issues.dedup();
        return Err(issues);
    }
    let cohort_ref = crate::identity::identify_cohort(
        &request.checkpoint.checkpoint_ref,
        &request.limits,
        request.workers,
        &request.context_ref,
    )
    .map_err(|ReferenceError| vec![CohortIssue::Identity])?;
    build_plan(request, cohort_ref, reservation)
}

fn build_plan(
    request: &CohortRequest,
    cohort_ref: crate::CohortRef,
    reservation: ResourceVector,
) -> Result<CohortPlan, Vec<CohortIssue>> {
    let worker_count =
        usize::try_from(request.workers).map_err(|_| vec![CohortIssue::PlanLimit])?;
    let effect_capacity = worker_count
        .checked_mul(EFFECTS.len())
        .ok_or_else(|| vec![CohortIssue::PlanLimit])?;
    if effect_capacity > MAX_EFFECTS {
        return Err(vec![CohortIssue::PlanLimit]);
    }
    let mut clones = Vec::with_capacity(worker_count);
    let mut effects = Vec::with_capacity(effect_capacity);
    let mut mutable_refs = BTreeSet::new();
    for ordinal in 0..request.workers {
        let clone_ref = crate::identity::identify_clone(&cohort_ref, ordinal)
            .map_err(|ReferenceError| vec![CohortIssue::Identity])?;
        let mut surfaces = Vec::with_capacity(SURFACES.len());
        for kind in SURFACES {
            let resource_ref = crate::identity::identify_surface(&clone_ref, kind)
                .map_err(|ReferenceError| vec![CohortIssue::Identity])?;
            if !mutable_refs.insert(resource_ref.clone()) {
                return Err(vec![CohortIssue::SharedMutableSurface]);
            }
            surfaces.push(SurfacePlan { kind, resource_ref });
        }
        for kind in EFFECTS {
            let operation_ref = crate::identity::identify_operation(&cohort_ref, &clone_ref, kind)
                .map_err(|ReferenceError| vec![CohortIssue::Identity])?;
            effects.push(EffectRequest {
                operation_ref,
                clone_ref: clone_ref.clone(),
                kind,
            });
        }
        clones.push(ClonePlan {
            clone_ref,
            memory_base_ref: request.checkpoint.effective_memory_base_ref.clone(),
            disk_base_ref: request.checkpoint.effective_disk_base_ref.clone(),
            surfaces,
        });
    }
    Ok(CohortPlan {
        cohort_ref,
        checkpoint_ref: request.checkpoint.checkpoint_ref.clone(),
        policy_ref: request.limits.policy_ref.clone(),
        reservation,
        clones,
        effects,
    })
}

/// Validates that every clone shares only the two immutable bases.
#[must_use]
pub fn validate_clone_isolation(plan: &CohortPlan) -> Vec<CohortIssue> {
    let capacity = plan.clones.len().saturating_mul(SURFACES.len());
    let mut mutable_refs = BTreeSet::new();
    let mut issues = Vec::with_capacity(FIXED_ISSUE_CAPACITY);
    for clone in &plan.clones {
        if clone.checkpoint_bases_drifted(plan) {
            issues.push(CohortIssue::Compatibility);
        }
        for surface in &clone.surfaces {
            if !mutable_refs.insert(surface.resource_ref.clone()) {
                issues.push(CohortIssue::SharedMutableSurface);
            }
        }
    }
    if mutable_refs.len() > capacity {
        issues.push(CohortIssue::PlanLimit);
    }
    issues.sort();
    issues.dedup();
    issues
}

impl ClonePlan {
    fn checkpoint_bases_drifted(&self, plan: &CohortPlan) -> bool {
        plan.clones.first().is_some_and(|first| {
            self.memory_base_ref != first.memory_base_ref
                || self.disk_base_ref != first.disk_base_ref
        })
    }
}

fn validate_request(request: &CohortRequest) -> Vec<CohortIssue> {
    let mut issues = Vec::with_capacity(FIXED_ISSUE_CAPACITY);
    if request.workers == 0
        || request.workers > MAX_WORKERS
        || request.workers > request.limits.maximum.workers
    {
        issues.push(CohortIssue::WorkerCount);
    }
    if request.expected_compatibility != request.checkpoint.compatibility {
        issues.push(CohortIssue::Compatibility);
    }
    if request.limits.memory_page_bytes == 0
        || request.limits.disk_page_bytes == 0
        || !request
            .checkpoint
            .memory_bytes
            .is_multiple_of(request.limits.memory_page_bytes)
        || !request
            .checkpoint
            .disk_bytes
            .is_multiple_of(request.limits.disk_page_bytes)
    {
        issues.push(CohortIssue::PageGeometry);
    }
    if request.limits.max_attempts == 0
        || request.limits.retry_delay_ticks == 0
        || request.limits.retry_delay_ticks > request.limits.max_retry_delay_ticks
    {
        issues.push(CohortIssue::RetryPolicy);
    }
    issues
}

fn reserve(request: &CohortRequest) -> Result<ResourceVector, CohortIssue> {
    let workers = request.workers;
    Ok(ResourceVector {
        workers,
        vcpus: workers
            .checked_mul(request.checkpoint.vcpu_count)
            .ok_or(CohortIssue::ArithmeticOverflow)?,
        memory_bytes: u64::from(workers)
            .checked_mul(request.checkpoint.memory_bytes)
            .ok_or(CohortIssue::ArithmeticOverflow)?,
        storage_bytes: u64::from(workers)
            .checked_mul(request.checkpoint.disk_bytes)
            .ok_or(CohortIssue::ArithmeticOverflow)?,
        mappings: workers
            .checked_mul(MAPPINGS_PER_CLONE)
            .ok_or(CohortIssue::ArithmeticOverflow)?,
        endpoints: workers
            .checked_mul(ENDPOINTS_PER_CLONE)
            .ok_or(CohortIssue::ArithmeticOverflow)?,
        in_flight: workers
            .checked_mul(u32::try_from(EFFECTS.len()).map_err(|_| CohortIssue::ArithmeticOverflow)?)
            .ok_or(CohortIssue::ArithmeticOverflow)?,
    })
}

const fn within_limits(actual: &ResourceVector, maximum: &ResourceVector) -> bool {
    actual.workers <= maximum.workers
        && actual.vcpus <= maximum.vcpus
        && actual.memory_bytes <= maximum.memory_bytes
        && actual.storage_bytes <= maximum.storage_bytes
        && actual.mappings <= maximum.mappings
        && actual.endpoints <= maximum.endpoints
        && actual.in_flight <= maximum.in_flight
}

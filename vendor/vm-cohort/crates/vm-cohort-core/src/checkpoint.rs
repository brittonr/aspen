use alloc::vec;
use alloc::vec::Vec;

use crate::{CheckpointCandidate, GoldenCheckpoint, MAX_TEXT_BYTES, ReferenceError};

/// Checkpoint admission or retention issue.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum CheckpointIssue {
    /// Required inventory is incomplete.
    Incomplete,
    /// A live host handle was embedded in portable state.
    HostHandlePresent,
    /// One effective base can still mutate.
    MutableBase,
    /// Memory size is zero.
    EmptyMemory,
    /// Disk size is zero.
    EmptyDisk,
    /// vCPU count is zero.
    EmptyVcpuSet,
    /// Device count is zero.
    EmptyDeviceSet,
    /// Architecture text is empty or overbound.
    InvalidArchitecture,
    /// Identity construction failed.
    Identity,
    /// Retained-use count overflowed.
    RetainedUseOverflow,
}

/// Admits one complete immutable initialized checkpoint.
///
/// # Errors
///
/// Returns every bounded admission issue before it emits a checkpoint.
// r[impl vm_cohort.checkpoint_admission]
// r[impl vm_cohort.effective_bases]
pub fn admit_checkpoint(
    candidate: &CheckpointCandidate,
) -> Result<GoldenCheckpoint, Vec<CheckpointIssue>> {
    let mut issues = Vec::with_capacity(CheckpointIssue::MAX_COUNT);
    if !candidate.complete {
        issues.push(CheckpointIssue::Incomplete);
    }
    if candidate.host_handles_present {
        issues.push(CheckpointIssue::HostHandlePresent);
    }
    if candidate.bases_mutable {
        issues.push(CheckpointIssue::MutableBase);
    }
    if candidate.memory_bytes == 0 {
        issues.push(CheckpointIssue::EmptyMemory);
    }
    if candidate.disk_bytes == 0 {
        issues.push(CheckpointIssue::EmptyDisk);
    }
    if candidate.vcpu_count == 0 {
        issues.push(CheckpointIssue::EmptyVcpuSet);
    }
    if candidate.device_count == 0 {
        issues.push(CheckpointIssue::EmptyDeviceSet);
    }
    if candidate.compatibility.architecture.is_empty()
        || candidate.compatibility.architecture.len() > MAX_TEXT_BYTES
    {
        issues.push(CheckpointIssue::InvalidArchitecture);
    }
    if !issues.is_empty() {
        return Err(issues);
    }
    let checkpoint_ref = crate::identity::identify_checkpoint(candidate)
        .map_err(|ReferenceError| vec![CheckpointIssue::Identity])?;
    Ok(GoldenCheckpoint {
        checkpoint_ref,
        compatibility: candidate.compatibility.clone(),
        effective_memory_base_ref: candidate.effective_memory_base_ref.clone(),
        effective_disk_base_ref: candidate.effective_disk_base_ref.clone(),
        memory_bytes: candidate.memory_bytes,
        disk_bytes: candidate.disk_bytes,
        vcpu_count: candidate.vcpu_count,
        device_count: candidate.device_count,
        retained_uses: 0,
    })
}

/// Records one completed or failed use without consuming either immutable base.
///
/// # Errors
///
/// Returns [`CheckpointIssue::RetainedUseOverflow`] when the counter is full.
// r[impl vm_cohort.checkpoint_retention]
pub fn record_retained_use(
    checkpoint: &GoldenCheckpoint,
) -> Result<GoldenCheckpoint, CheckpointIssue> {
    let retained_uses = checkpoint
        .retained_uses
        .checked_add(1)
        .ok_or(CheckpointIssue::RetainedUseOverflow)?;
    let mut retained = checkpoint.clone();
    retained.retained_uses = retained_uses;
    Ok(retained)
}

impl CheckpointIssue {
    const MAX_COUNT: usize = 9;
}

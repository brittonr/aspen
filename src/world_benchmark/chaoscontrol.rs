use chaoscontrol_snapshot_descriptor::SnapshotDescriptor;
use chaoscontrol_snapshot_descriptor::validate_descriptor;
use molten_core::world_benchmark::*;

use crate::error::MoltenError;
use crate::error::Result;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChaosControlSnapshotSharingObservation {
    pub observation_ref: String,
    pub adapter_ref: String,
    pub page_size_bytes: u64,
    pub copied_pages: u64,
    pub mapped_pages: u64,
    pub physical_bytes_written: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChaosControlSnapshotBenchmarkObservation {
    pub binding: WorldBenchmarkSnapshotBinding,
    pub operation: super::WorldBenchmarkOperationObservation,
    pub observation_ref: String,
}

// r[impl molten.world_bench.snapshot_profiles]
pub fn bind_chaoscontrol_snapshot(
    descriptor_ref: String,
    descriptor: &SnapshotDescriptor,
) -> Result<WorldBenchmarkSnapshotBinding> {
    validate_descriptor(descriptor)
        .map_err(|error| MoltenError::invalid_harness(format!("ChaosControl snapshot descriptor denied: {error:?}")))?;
    if descriptor.completeness_profile != CHAOSCONTROL_SNAPSHOT_PROFILE {
        return Err(MoltenError::invalid_harness(
            "ChaosControl snapshot descriptor profile drifted from the exact benchmark cohort",
        ));
    }
    let closure_members = u64::try_from(descriptor.payload.members.len())
        .map_err(|_| MoltenError::invalid_harness("ChaosControl snapshot closure member count overflow"))?;
    Ok(WorldBenchmarkSnapshotBinding {
        descriptor_ref,
        source_revision: CHAOSCONTROL_SNAPSHOT_REVISION.to_string(),
        completeness_profile: descriptor.completeness_profile.clone(),
        memory_bytes: descriptor.topology.memory_bytes,
        closure_members,
    })
}

pub fn instrument_chaoscontrol_snapshot(
    descriptor_ref: String,
    descriptor: &SnapshotDescriptor,
    observation: &ChaosControlSnapshotSharingObservation,
) -> Result<ChaosControlSnapshotBenchmarkObservation> {
    crate::preserves_rail::validate_content_ref(&observation.observation_ref)
        .map_err(|_| MoltenError::invalid_harness("ChaosControl sharing observation ref is invalid"))?;
    if observation.page_size_bytes == 0 || !descriptor.topology.memory_bytes.is_multiple_of(observation.page_size_bytes)
    {
        return Err(MoltenError::invalid_harness(
            "ChaosControl sharing page geometry does not cover exact snapshot memory",
        ));
    }
    let page_count = descriptor.topology.memory_bytes / observation.page_size_bytes;
    let observed_pages = observation
        .copied_pages
        .checked_add(observation.mapped_pages)
        .ok_or_else(|| MoltenError::invalid_harness("ChaosControl sharing page count overflow"))?;
    let expected_physical_bytes = observation
        .copied_pages
        .checked_mul(observation.page_size_bytes)
        .ok_or_else(|| MoltenError::invalid_harness("ChaosControl sharing byte count overflow"))?;
    if observed_pages != page_count || observation.physical_bytes_written != expected_physical_bytes {
        return Err(MoltenError::invalid_harness(
            "ChaosControl sharing observation is incomplete or internally inconsistent",
        ));
    }
    let binding = bind_chaoscontrol_snapshot(descriptor_ref, descriptor)?;
    let operation = super::instrument_world_benchmark_facts(
        WorldBenchmarkOperation::SnapshotShare,
        &super::WorldBenchmarkOperationFacts {
            adapter_ref: observation.adapter_ref.clone(),
            logical_bytes: descriptor.topology.memory_bytes,
            physical_bytes_written: observation.physical_bytes_written,
            new_objects: 0,
            reused_objects: 0,
            copied_pages: observation.copied_pages,
            mapped_pages: observation.mapped_pages,
            traversed_references: binding.closure_members,
            compared_keys: 0,
            emitted_conflicts: 0,
            transferred_bytes: 0,
            retained_objects: 0,
            planned_deletions: 0,
            protected_deletion_candidates: 0,
            physical_measurement_independent: true,
        },
    )?;
    Ok(ChaosControlSnapshotBenchmarkObservation {
        binding,
        operation,
        observation_ref: observation.observation_ref.clone(),
    })
}

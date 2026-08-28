use molten_core::world_benchmark::*;

use super::WorldBenchmarkOperationObservation;
use crate::error::MoltenError;
use crate::error::Result;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldBenchmarkOperationFacts {
    pub adapter_ref: String,
    pub logical_bytes: u64,
    pub physical_bytes_written: u64,
    pub new_objects: u64,
    pub reused_objects: u64,
    pub copied_pages: u64,
    pub mapped_pages: u64,
    pub traversed_references: u64,
    pub compared_keys: u64,
    pub emitted_conflicts: u64,
    pub transferred_bytes: u64,
    pub retained_objects: u64,
    pub planned_deletions: u64,
    pub protected_deletion_candidates: u64,
    pub physical_measurement_independent: bool,
}

// r[impl molten.world_bench.metrics]
// r[impl molten.world_bench.retention]
pub fn instrument_world_benchmark_facts(
    operation: WorldBenchmarkOperation,
    facts: &WorldBenchmarkOperationFacts,
) -> Result<WorldBenchmarkOperationObservation> {
    if facts.adapter_ref.is_empty() {
        return Err(MoltenError::invalid_harness("world benchmark adapter ref is empty"));
    }
    if !facts.physical_measurement_independent {
        return Err(MoltenError::invalid_harness("world benchmark collapsed logical and physical measurement sources"));
    }
    if operation == WorldBenchmarkOperation::RetentionPlan && facts.protected_deletion_candidates != 0 {
        return Err(MoltenError::invalid_harness(
            "world benchmark retention plan contains a protected deletion candidate",
        ));
    }
    Ok(WorldBenchmarkOperationObservation {
        adapter_ref: facts.adapter_ref.clone(),
        metrics: complete_world_benchmark_metrics(&[
            (WorldBenchmarkMetricKind::LogicalBytes, facts.logical_bytes),
            (WorldBenchmarkMetricKind::PhysicalBytesWritten, facts.physical_bytes_written),
            (WorldBenchmarkMetricKind::NewObjects, facts.new_objects),
            (WorldBenchmarkMetricKind::ReusedObjects, facts.reused_objects),
            (WorldBenchmarkMetricKind::CopiedPages, facts.copied_pages),
            (WorldBenchmarkMetricKind::MappedPages, facts.mapped_pages),
            (WorldBenchmarkMetricKind::TraversedReferences, facts.traversed_references),
            (WorldBenchmarkMetricKind::ComparedKeys, facts.compared_keys),
            (WorldBenchmarkMetricKind::EmittedConflicts, facts.emitted_conflicts),
            (WorldBenchmarkMetricKind::TransferredBytes, facts.transferred_bytes),
            (WorldBenchmarkMetricKind::RetainedObjects, facts.retained_objects),
            (WorldBenchmarkMetricKind::PlannedDeletions, facts.planned_deletions),
        ]),
        physical_measurement_independent: true,
    })
}

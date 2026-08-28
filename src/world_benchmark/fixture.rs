use molten_core::world_benchmark::*;

use super::*;
use crate::error::MoltenError;
use crate::error::Result;

const ROOT_REFERENCE_BYTES: u64 = 64;

#[derive(Debug, Clone)]
pub struct DeterministicWorldBenchmarkFixture {
    pub dataset: WorldBenchmarkDataset,
    pub preparation: WorldBenchmarkPreparationObservation,
    pub snapshot: Option<WorldBenchmarkSnapshotBinding>,
    pub published_records: Vec<String>,
}

impl WorldBenchmarkDatasetPort for DeterministicWorldBenchmarkFixture {
    fn prepare(
        &mut self,
        _plan: &WorldBenchmarkPlan,
    ) -> Result<(WorldBenchmarkDataset, WorldBenchmarkPreparationObservation)> {
        Ok((self.dataset.clone(), self.preparation.clone()))
    }
}

impl WorldBenchmarkOperationPort for DeterministicWorldBenchmarkFixture {
    fn observe(
        &mut self,
        _plan: &WorldBenchmarkPlan,
        dataset: &WorldBenchmarkDataset,
        operation: WorldBenchmarkOperation,
        _repetition: u32,
    ) -> Result<WorldBenchmarkOperationObservation> {
        let facts = fixture_facts(dataset, operation)?;
        instrument_world_benchmark_facts(operation, &facts)
    }
}

impl WorldBenchmarkResourcePort for DeterministicWorldBenchmarkFixture {
    fn observe_resources(
        &mut self,
        _operation: WorldBenchmarkOperation,
        _repetition: u32,
    ) -> Result<WorldBenchmarkResourceObservation> {
        Ok(WorldBenchmarkResourceObservation {
            duration_nanoseconds: None,
            peak_memory_bytes: None,
        })
    }
}

impl WorldBenchmarkSnapshotPort for DeterministicWorldBenchmarkFixture {
    fn observe_snapshot(
        &mut self,
        operation: WorldBenchmarkOperation,
        _repetition: u32,
    ) -> Result<Option<WorldBenchmarkSnapshotBinding>> {
        match operation {
            WorldBenchmarkOperation::SnapshotShare => self
                .snapshot
                .clone()
                .map(Some)
                .ok_or_else(|| MoltenError::invalid_harness("opaque fixture is missing an exact snapshot binding")),
            _ => Ok(None),
        }
    }
}

impl WorldBenchmarkReceiptPort for DeterministicWorldBenchmarkFixture {
    fn publish(&mut self, record: &CanonicalWorldBenchmarkRecord) -> Result<String> {
        self.published_records.push(record.record_ref.clone());
        Ok(record.record_ref.clone())
    }
}

fn fixture_facts(
    dataset: &WorldBenchmarkDataset,
    operation: WorldBenchmarkOperation,
) -> Result<WorldBenchmarkOperationFacts> {
    let unchanged_objects = dataset.object_count.saturating_sub(dataset.changed_objects);
    let mapped_pages = dataset.page_count.saturating_sub(dataset.changed_objects);
    let snapshot_physical = dataset
        .changed_objects
        .checked_mul(dataset.page_size_bytes)
        .ok_or_else(|| MoltenError::invalid_harness("snapshot fixture physical byte count overflow"))?;
    let mut facts = WorldBenchmarkOperationFacts {
        adapter_ref: "molten-deterministic-world-benchmark-v1".to_string(),
        logical_bytes: 0,
        physical_bytes_written: 0,
        new_objects: 0,
        reused_objects: 0,
        copied_pages: 0,
        mapped_pages: 0,
        traversed_references: 0,
        compared_keys: 0,
        emitted_conflicts: 0,
        transferred_bytes: 0,
        retained_objects: 0,
        planned_deletions: 0,
        protected_deletion_candidates: 0,
        physical_measurement_independent: true,
    };
    match operation {
        WorldBenchmarkOperation::RootBranch => {
            facts.logical_bytes = ROOT_REFERENCE_BYTES;
            facts.physical_bytes_written = ROOT_REFERENCE_BYTES;
            facts.new_objects = 1;
            facts.reused_objects = dataset.object_count;
        }
        WorldBenchmarkOperation::FirstMutation | WorldBenchmarkOperation::RepeatedMutation => {
            facts.logical_bytes = dataset.mutation_bytes;
            facts.physical_bytes_written = dataset.mutation_bytes;
            facts.new_objects = dataset.changed_objects;
            facts.reused_objects = unchanged_objects;
            facts.compared_keys = dataset.key_count;
        }
        WorldBenchmarkOperation::Diff | WorldBenchmarkOperation::MergePlan => {
            facts.traversed_references = dataset.object_count;
            facts.compared_keys = dataset.key_count;
        }
        WorldBenchmarkOperation::CapsuleExport => {
            facts.logical_bytes = dataset.logical_bytes;
            facts.physical_bytes_written = dataset.logical_bytes;
            facts.traversed_references = dataset.object_count;
        }
        WorldBenchmarkOperation::Replication => {
            facts.logical_bytes = dataset.logical_bytes;
            facts.reused_objects = unchanged_objects;
            facts.transferred_bytes = dataset.mutation_bytes;
        }
        WorldBenchmarkOperation::RetentionPlan => {
            facts.traversed_references = dataset.object_count;
            facts.retained_objects = dataset.object_count;
        }
        WorldBenchmarkOperation::SnapshotShare => {
            facts.logical_bytes = dataset.logical_bytes;
            facts.physical_bytes_written = snapshot_physical;
            facts.copied_pages = dataset.changed_objects;
            facts.mapped_pages = mapped_pages;
        }
    }
    Ok(facts)
}

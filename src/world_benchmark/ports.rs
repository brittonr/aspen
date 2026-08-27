use molten_core::world_benchmark::*;

use super::CanonicalWorldBenchmarkRecord;
use crate::error::Result;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldBenchmarkResourceObservation {
    pub duration_nanoseconds: Option<u64>,
    pub peak_memory_bytes: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldBenchmarkOperationObservation {
    pub adapter_ref: String,
    pub metrics: Vec<WorldBenchmarkMetric>,
    pub physical_measurement_independent: bool,
}

pub trait WorldBenchmarkDatasetPort {
    fn prepare(
        &mut self,
        plan: &WorldBenchmarkPlan,
    ) -> Result<(WorldBenchmarkDataset, WorldBenchmarkPreparationObservation)>;
}

pub trait WorldBenchmarkOperationPort {
    fn observe(
        &mut self,
        plan: &WorldBenchmarkPlan,
        dataset: &WorldBenchmarkDataset,
        operation: WorldBenchmarkOperation,
        repetition: u32,
    ) -> Result<WorldBenchmarkOperationObservation>;
}

pub trait WorldBenchmarkResourcePort {
    fn observe_resources(
        &mut self,
        operation: WorldBenchmarkOperation,
        repetition: u32,
    ) -> Result<WorldBenchmarkResourceObservation>;
}

pub trait WorldBenchmarkSnapshotPort {
    fn observe_snapshot(
        &mut self,
        operation: WorldBenchmarkOperation,
        repetition: u32,
    ) -> Result<Option<WorldBenchmarkSnapshotBinding>>;
}

pub trait WorldBenchmarkReceiptPort {
    fn publish(&mut self, record: &CanonicalWorldBenchmarkRecord) -> Result<String>;
}

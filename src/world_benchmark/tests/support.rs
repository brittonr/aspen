use molten_core::world_benchmark::*;

use super::super::*;

pub(super) const SOURCE_REVISION: &str = "58185205ffc69ab3f7dc6cc04388885e7d43f562";
pub(super) const LOGICAL_BYTES: u64 = 4_096;
pub(super) const OBJECT_COUNT: u64 = 8;
pub(super) const KEY_COUNT: u64 = 32;
pub(super) const PAGE_COUNT: u64 = 16;
pub(super) const PAGE_SIZE_BYTES: u64 = 256;
pub(super) const MUTATION_BYTES: u64 = 512;
pub(super) const REPETITIONS: u32 = 2;
pub(super) const LOGICAL_OPERATION_COUNT: u32 = 8;
pub(super) const ADAPTER: &str = "molten-deterministic-world-benchmark-v1";

pub(super) fn reference(seed: char) -> String {
    format!("blake3:{}", seed.to_string().repeat(64))
}

pub(super) fn profile() -> WorldBenchmarkProfile {
    WorldBenchmarkProfile {
        schema: WORLD_BENCHMARK_PROFILE_SCHEMA.to_string(),
        profile_ref: reference('a'),
        source_revision: SOURCE_REVISION.to_string(),
        dataset_ref: reference('b'),
        preparation: WorldBenchmarkPreparation::Cold,
        class: WorldBenchmarkClass::Logical,
        adapters: vec![ADAPTER.to_string()],
        operations: vec![
            WorldBenchmarkOperation::RootBranch,
            WorldBenchmarkOperation::FirstMutation,
            WorldBenchmarkOperation::RepeatedMutation,
            WorldBenchmarkOperation::Diff,
            WorldBenchmarkOperation::MergePlan,
            WorldBenchmarkOperation::CapsuleExport,
            WorldBenchmarkOperation::Replication,
            WorldBenchmarkOperation::RetentionPlan,
        ],
        bounds: WorldBenchmarkBounds {
            max_operations: LOGICAL_OPERATION_COUNT,
            max_repetitions: REPETITIONS,
            max_logical_bytes: LOGICAL_BYTES,
            max_physical_bytes: LOGICAL_BYTES,
            max_objects: 64,
            max_pages: 64,
            max_references: 128,
            max_keys: 128,
            max_conflicts: 16,
            max_duration_nanoseconds: 1_000_000,
            max_peak_memory_bytes: 1_048_576,
        },
        repetitions: REPETITIONS,
        hardware_cohort: "fixture-host-v1".to_string(),
        thresholds: vec![WorldBenchmarkThreshold {
            name: "mutation-physical-write-ceiling".to_string(),
            metric: WorldBenchmarkMetricKind::PhysicalBytesWritten,
            maximum: LOGICAL_BYTES,
            operation: None,
        }],
    }
}

pub(super) fn dataset(profile: &WorldBenchmarkProfile) -> WorldBenchmarkDataset {
    WorldBenchmarkDataset {
        dataset_ref: profile.dataset_ref.clone(),
        source_revision: profile.source_revision.clone(),
        shape: WorldBenchmarkDatasetShape::Synthetic,
        preparation: profile.preparation,
        logical_bytes: LOGICAL_BYTES,
        object_count: OBJECT_COUNT,
        preexisting_objects: 0,
        changed_objects: 1,
        mutation_bytes: MUTATION_BYTES,
        key_count: KEY_COUNT,
        page_count: PAGE_COUNT,
        page_size_bytes: PAGE_SIZE_BYTES,
    }
}

pub(super) fn fixture(profile: &WorldBenchmarkProfile) -> DeterministicWorldBenchmarkFixture {
    DeterministicWorldBenchmarkFixture {
        dataset: dataset(profile),
        preparation: WorldBenchmarkPreparationObservation {
            dataset_ref: profile.dataset_ref.clone(),
            source_revision: profile.source_revision.clone(),
            preparation: profile.preparation,
            prior_objects_available: false,
            preparation_ref: reference('c'),
        },
        snapshot: None,
        published_records: Vec::new(),
    }
}

use super::super::*;

pub(super) const SOURCE_REVISION: &str = "58185205ffc69ab3f7dc6cc04388885e7d43f562";
pub(super) const OTHER_REVISION: &str = "68185205ffc69ab3f7dc6cc04388885e7d43f562";
pub(super) const REPETITIONS: u32 = 2;
pub(super) const LOGICAL_BYTES: u64 = 4_096;
pub(super) const PHYSICAL_BYTES: u64 = 512;
pub(super) const OBJECTS: u64 = 8;
pub(super) const PAGES: u64 = 16;
pub(super) const KEYS: u64 = 32;

pub(super) fn reference(seed: char) -> String {
    format!("blake3:{}", seed.to_string().repeat(64))
}

pub(super) fn logical_profile() -> WorldBenchmarkProfile {
    WorldBenchmarkProfile {
        schema: WORLD_BENCHMARK_PROFILE_SCHEMA.to_string(),
        profile_ref: reference('a'),
        source_revision: SOURCE_REVISION.to_string(),
        dataset_ref: reference('b'),
        preparation: WorldBenchmarkPreparation::Cold,
        class: WorldBenchmarkClass::Logical,
        adapters: vec!["molten-memory-fixture-v1".to_string()],
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
            max_operations: 8,
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
            name: "physical-write-ceiling".to_string(),
            metric: WorldBenchmarkMetricKind::PhysicalBytesWritten,
            maximum: LOGICAL_BYTES,
            operation: None,
        }],
    }
}

pub(super) fn opaque_profile() -> WorldBenchmarkProfile {
    let mut profile = logical_profile();
    profile.profile_ref = reference('c');
    profile.class = WorldBenchmarkClass::OpaqueExactSnapshot;
    profile.operations = vec![WorldBenchmarkOperation::SnapshotShare];
    profile.bounds.max_operations = 1;
    profile
}

pub(super) fn dataset(profile: &WorldBenchmarkProfile) -> WorldBenchmarkDataset {
    WorldBenchmarkDataset {
        dataset_ref: profile.dataset_ref.clone(),
        source_revision: profile.source_revision.clone(),
        shape: WorldBenchmarkDatasetShape::Synthetic,
        preparation: profile.preparation,
        logical_bytes: LOGICAL_BYTES,
        object_count: OBJECTS,
        preexisting_objects: 0,
        changed_objects: 1,
        mutation_bytes: PHYSICAL_BYTES,
        key_count: KEYS,
        page_count: PAGES,
        page_size_bytes: 256,
    }
}

pub(super) fn results(plan: &WorldBenchmarkPlan) -> Vec<WorldBenchmarkResult> {
    let repetition_count = usize::try_from(plan.repetitions).expect("bounded repetition count");
    let result_capacity = plan.operations.len().checked_mul(repetition_count).expect("bounded result count");
    assert!(result_capacity <= MAX_WORLD_BENCHMARK_RESULTS);
    let mut results = Vec::with_capacity(result_capacity);
    for repetition in 0..plan.repetitions {
        for operation in &plan.operations {
            let snapshot =
                (*operation == WorldBenchmarkOperation::SnapshotShare).then(|| WorldBenchmarkSnapshotBinding {
                    descriptor_ref: reference('d'),
                    source_revision: CHAOSCONTROL_SNAPSHOT_REVISION.to_string(),
                    completeness_profile: CHAOSCONTROL_SNAPSHOT_PROFILE.to_string(),
                    memory_bytes: LOGICAL_BYTES,
                    closure_members: 1,
                });
            results.push(WorldBenchmarkResult {
                operation: *operation,
                repetition,
                adapter_ref: plan.adapters[0].clone(),
                metrics: complete_world_benchmark_metrics(&[
                    (WorldBenchmarkMetricKind::LogicalBytes, LOGICAL_BYTES),
                    (WorldBenchmarkMetricKind::PhysicalBytesWritten, PHYSICAL_BYTES),
                    (WorldBenchmarkMetricKind::NewObjects, 1),
                    (WorldBenchmarkMetricKind::ReusedObjects, OBJECTS - 1),
                    (WorldBenchmarkMetricKind::CopiedPages, 1),
                    (WorldBenchmarkMetricKind::MappedPages, PAGES - 1),
                    (WorldBenchmarkMetricKind::TraversedReferences, OBJECTS),
                    (WorldBenchmarkMetricKind::ComparedKeys, KEYS),
                    (WorldBenchmarkMetricKind::TransferredBytes, PHYSICAL_BYTES),
                    (WorldBenchmarkMetricKind::RetainedObjects, OBJECTS),
                ]),
                duration_nanoseconds: Some(1_000),
                peak_memory_bytes: Some(LOGICAL_BYTES),
                snapshot,
                physical_measurement_independent: true,
            });
        }
    }
    results
}

pub(super) fn accepted_receipt(consumer: &str) -> WorldBenchmarkReceipt {
    let profile = logical_profile();
    let plan = plan_world_benchmark(&profile, SOURCE_REVISION).expect("plan");
    finalize_world_benchmark_receipt(&plan, consumer.to_string(), results(&plan), Vec::new()).expect("receipt")
}

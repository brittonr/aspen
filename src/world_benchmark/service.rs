use molten_core::world_benchmark::*;

use super::*;
use crate::error::MoltenError;
use crate::error::Result;

pub struct WorldBenchmarkPorts<'a> {
    pub datasets: &'a mut dyn WorldBenchmarkDatasetPort,
    pub operations: &'a mut dyn WorldBenchmarkOperationPort,
    pub resources: &'a mut dyn WorldBenchmarkResourcePort,
    pub snapshots: &'a mut dyn WorldBenchmarkSnapshotPort,
    pub receipts: &'a mut dyn WorldBenchmarkReceiptPort,
}

#[derive(Debug, Clone)]
pub struct WorldBenchmarkRunOutcome {
    pub plan: WorldBenchmarkPlan,
    pub plan_record: CanonicalWorldBenchmarkRecord,
    pub preparation: WorldBenchmarkPreparationObservation,
    pub receipt: WorldBenchmarkReceipt,
    pub receipt_record: CanonicalWorldBenchmarkRecord,
    pub published_receipt_ref: String,
}

// r[impl molten.world_bench.metrics]
// r[impl molten.world_bench.datasets]
// r[impl molten.world_bench.receipt]
pub fn run_world_benchmark(
    profile: &WorldBenchmarkProfile,
    current_source_revision: &str,
    consumer_id: String,
    ports: WorldBenchmarkPorts<'_>,
) -> Result<WorldBenchmarkRunOutcome> {
    let plan = plan_world_benchmark(profile, current_source_revision).map_err(core_issues)?;
    let plan_record = canonical_world_benchmark_plan(&plan)?;
    let (dataset, preparation) = ports.datasets.prepare(&plan)?;
    let dataset_issues = validate_world_benchmark_dataset(profile, &dataset);
    if !dataset_issues.is_empty() {
        return Err(core_issues(dataset_issues));
    }
    let preparation_issues = validate_world_benchmark_preparation(&plan, &preparation);
    if !preparation_issues.is_empty() {
        return Err(core_issues(preparation_issues));
    }
    let mut results = Vec::new();
    for repetition in 0..plan.repetitions {
        for operation in &plan.operations {
            let observation = ports.operations.observe(&plan, &dataset, *operation, repetition)?;
            let resources = ports.resources.observe_resources(*operation, repetition)?;
            let snapshot = ports.snapshots.observe_snapshot(*operation, repetition)?;
            let result = WorldBenchmarkResult {
                operation: *operation,
                repetition,
                adapter_ref: observation.adapter_ref,
                metrics: observation.metrics,
                duration_nanoseconds: resources.duration_nanoseconds,
                peak_memory_bytes: resources.peak_memory_bytes,
                snapshot,
                physical_measurement_independent: observation.physical_measurement_independent,
            };
            let issues = validate_world_benchmark_result(&plan, &result);
            if !issues.is_empty() {
                return Err(core_issues(issues));
            }
            results.push(result);
        }
    }
    let receipt = finalize_world_benchmark_receipt(&plan, consumer_id, results, Vec::new()).map_err(core_issues)?;
    let receipt_record = canonical_world_benchmark_receipt(&receipt)?;
    let published_receipt_ref = ports.receipts.publish(&receipt_record)?;
    if published_receipt_ref != receipt_record.record_ref {
        return Err(MoltenError::invalid_harness(
            "world benchmark receipt publication substituted the canonical record ref",
        ));
    }
    Ok(WorldBenchmarkRunOutcome {
        plan,
        plan_record,
        preparation,
        receipt,
        receipt_record,
        published_receipt_ref,
    })
}

fn core_issues(issues: Vec<WorldBenchmarkIssue>) -> MoltenError {
    MoltenError::invalid_harness(format!("world benchmark denied: {issues:?}"))
}

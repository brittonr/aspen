use molten_core::world_benchmark::*;
use preserves::IOValue;

use crate::error::MoltenError;
use crate::error::Result;

const WORLD_BENCHMARK_RECORD_CONTEXT: &str = "onixresearch.molten.world-benchmark.record.v1";
const PLAN_RECORD: &str = "molten-world-benchmark-plan-v1";
const RECEIPT_RECORD: &str = "molten-world-benchmark-receipt-v1";

#[derive(Debug, Clone)]
pub struct CanonicalWorldBenchmarkRecord {
    pub record_ref: String,
    pub value: IOValue,
    pub bytes: Vec<u8>,
}

pub fn canonical_world_benchmark_plan(plan: &WorldBenchmarkPlan) -> Result<CanonicalWorldBenchmarkRecord> {
    require_non_claims(&plan.non_claims)?;
    canonical(
        "plan",
        record(PLAN_RECORD, vec![
            field("plan-ref", string(&plan.plan_ref)),
            field("profile-ref", string(&plan.profile_ref)),
            field("source-revision", string(&plan.source_revision)),
            field("dataset-ref", string(&plan.dataset_ref)),
            field("class", string(plan.class.as_str())),
            field("preparation", string(plan.preparation.as_str())),
            field("operations", sequence(plan.operations.iter().map(|operation| string(operation.as_str())).collect())),
            field("repetitions", number(u64::from(plan.repetitions))),
            field("adapters", sequence(plan.adapters.iter().map(string).collect())),
            field("hardware-cohort", string(&plan.hardware_cohort)),
            bounds_value(&plan.bounds),
            field("thresholds", sequence(plan.thresholds.iter().map(threshold_value).collect())),
            non_claims(),
        ]),
    )
}

pub fn canonical_world_benchmark_receipt(receipt: &WorldBenchmarkReceipt) -> Result<CanonicalWorldBenchmarkRecord> {
    require_non_claims(&receipt.non_claims)?;
    if !validate_world_benchmark_receipt(receipt, &receipt.source_revision).is_empty() {
        return Err(MoltenError::invalid_harness("world benchmark receipt is invalid"));
    }
    canonical(
        "receipt",
        record(RECEIPT_RECORD, vec![
            field("receipt-ref", string(&receipt.receipt_ref)),
            field("plan-ref", string(&receipt.plan_ref)),
            field("consumer-id", string(&receipt.consumer_id)),
            field("profile-ref", string(&receipt.profile_ref)),
            field("source-revision", string(&receipt.source_revision)),
            field("dataset-ref", string(&receipt.dataset_ref)),
            field("preparation", string(receipt.preparation.as_str())),
            field("class", string(receipt.class.as_str())),
            field("adapters", sequence(receipt.adapters.iter().map(string).collect())),
            field("hardware-cohort", string(&receipt.hardware_cohort)),
            bounds_value(&receipt.bounds),
            field("results", sequence(receipt.results.iter().map(result_value).collect())),
            field(
                "threshold-results",
                sequence(receipt.threshold_results.iter().map(threshold_result_value).collect()),
            ),
            field("unsupported-rows", sequence(receipt.unsupported_rows.iter().map(unsupported_value).collect())),
            field("accepted", boolean(receipt.accepted)),
            field("deletion-authorized", boolean(false)),
            field("release-authorized", boolean(false)),
            non_claims(),
        ]),
    )
}

fn result_value(result: &WorldBenchmarkResult) -> IOValue {
    record("result", vec![
        field("operation", string(result.operation.as_str())),
        field("repetition", number(u64::from(result.repetition))),
        field("adapter-ref", string(&result.adapter_ref)),
        field(
            "metrics",
            sequence(
                result
                    .metrics
                    .iter()
                    .map(|metric| record("metric", vec![string(metric.kind.as_str()), number(metric.value)]))
                    .collect(),
            ),
        ),
        field("duration-nanoseconds", optional_number(result.duration_nanoseconds)),
        field("peak-memory-bytes", optional_number(result.peak_memory_bytes)),
        field("snapshot", snapshot_value(result.snapshot.as_ref())),
        field("physical-measurement-independent", boolean(result.physical_measurement_independent)),
    ])
}

fn snapshot_value(snapshot: Option<&WorldBenchmarkSnapshotBinding>) -> IOValue {
    snapshot.map_or_else(
        || record("none", Vec::new()),
        |snapshot| {
            record("some", vec![record("snapshot", vec![
                field("descriptor-ref", string(&snapshot.descriptor_ref)),
                field("source-revision", string(&snapshot.source_revision)),
                field("completeness-profile", string(&snapshot.completeness_profile)),
                field("memory-bytes", number(snapshot.memory_bytes)),
                field("closure-members", number(snapshot.closure_members)),
                field("semantic-equivalence", boolean(false)),
            ])])
        },
    )
}

fn threshold_value(threshold: &WorldBenchmarkThreshold) -> IOValue {
    record("threshold", vec![
        field("name", string(&threshold.name)),
        field("metric", string(threshold.metric.as_str())),
        field("maximum", number(threshold.maximum)),
        field(
            "operation",
            threshold
                .operation
                .map_or_else(|| record("all", Vec::new()), |operation| record("one", vec![string(operation.as_str())])),
        ),
    ])
}

fn threshold_result_value(threshold: &WorldBenchmarkThresholdResult) -> IOValue {
    record("threshold-result", vec![
        field("name", string(&threshold.name)),
        field("metric", string(threshold.metric.as_str())),
        field("observed-maximum", number(threshold.observed_maximum)),
        field("admitted-maximum", number(threshold.admitted_maximum)),
        field("passed", boolean(threshold.passed)),
    ])
}

fn unsupported_value(row: &WorldBenchmarkUnsupportedRow) -> IOValue {
    record("unsupported", vec![
        field("operation", string(row.operation.as_str())),
        field("reason", string(&row.reason)),
    ])
}

fn bounds_value(bounds: &WorldBenchmarkBounds) -> IOValue {
    record("bounds", vec![
        field("max-operations", number(u64::from(bounds.max_operations))),
        field("max-repetitions", number(u64::from(bounds.max_repetitions))),
        field("max-logical-bytes", number(bounds.max_logical_bytes)),
        field("max-physical-bytes", number(bounds.max_physical_bytes)),
        field("max-objects", number(bounds.max_objects)),
        field("max-pages", number(bounds.max_pages)),
        field("max-references", number(bounds.max_references)),
        field("max-keys", number(bounds.max_keys)),
        field("max-conflicts", number(bounds.max_conflicts)),
        field("max-duration-nanoseconds", number(bounds.max_duration_nanoseconds)),
        field("max-peak-memory-bytes", number(bounds.max_peak_memory_bytes)),
    ])
}

fn canonical(kind: &str, value: IOValue) -> Result<CanonicalWorldBenchmarkRecord> {
    let bytes = crate::preserves_rail::canonical_bytes(&value)?;
    let mut hasher = blake3::Hasher::new_derive_key(WORLD_BENCHMARK_RECORD_CONTEXT);
    update(&mut hasher, kind)?;
    let length = u64::try_from(bytes.len())
        .map_err(|_| MoltenError::invalid_harness("world benchmark record length exceeds u64"))?;
    hasher.update(&length.to_be_bytes());
    hasher.update(&bytes);
    Ok(CanonicalWorldBenchmarkRecord {
        record_ref: format!("blake3:{}", hasher.finalize().to_hex()),
        value,
        bytes,
    })
}

fn require_non_claims(non_claims: &[String]) -> Result<()> {
    if non_claims != world_benchmark_non_claims() {
        return Err(MoltenError::invalid_harness("world benchmark non-claims are incomplete"));
    }
    Ok(())
}

fn update(hasher: &mut blake3::Hasher, value: &str) -> Result<()> {
    let length = u64::try_from(value.len())
        .map_err(|_| MoltenError::invalid_harness("world benchmark identity length exceeds u64"))?;
    hasher.update(&length.to_be_bytes());
    hasher.update(value.as_bytes());
    Ok(())
}

fn non_claims() -> IOValue {
    field("non-claims", sequence(WORLD_BENCHMARK_NON_CLAIMS.iter().map(string).collect()))
}

fn field(label: &'static str, value: IOValue) -> IOValue {
    record(label, vec![value])
}

fn optional_number(value: Option<u64>) -> IOValue {
    value.map_or_else(|| record("none", Vec::new()), |value| record("some", vec![number(value)]))
}

fn boolean(value: bool) -> IOValue {
    record(if value { "true" } else { "false" }, Vec::new())
}

fn number(value: u64) -> IOValue {
    crate::preserves_rail::u64_value(value)
}

fn string(value: impl AsRef<str>) -> IOValue {
    crate::preserves_rail::string(value.as_ref())
}

fn sequence(values: Vec<IOValue>) -> IOValue {
    crate::preserves_rail::sequence(values)
}

fn record(label: &'static str, fields: Vec<IOValue>) -> IOValue {
    crate::preserves_rail::record(label, fields)
}

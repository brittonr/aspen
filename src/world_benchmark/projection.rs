use molten_core::world_benchmark::*;
use serde::Deserialize;

use crate::error::MoltenError;
use crate::error::Result;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct InputProjection {
    profile: ProfileProjection,
    dataset: DatasetProjection,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProfileProjection {
    schema: String,
    profile_ref: String,
    source_revision: String,
    dataset_ref: String,
    preparation: String,
    class: String,
    adapters: Vec<String>,
    operations: Vec<String>,
    bounds: BoundsProjection,
    repetitions: u32,
    hardware_cohort: String,
    thresholds: Vec<ThresholdProjection>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct BoundsProjection {
    max_operations: u32,
    max_repetitions: u32,
    max_logical_bytes: u64,
    max_physical_bytes: u64,
    max_objects: u64,
    max_pages: u64,
    max_references: u64,
    max_keys: u64,
    max_conflicts: u64,
    max_duration_nanoseconds: u64,
    max_peak_memory_bytes: u64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ThresholdProjection {
    name: String,
    metric: String,
    maximum: u64,
    operation: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DatasetProjection {
    dataset_ref: String,
    source_revision: String,
    shape: String,
    preparation: String,
    logical_bytes: u64,
    object_count: u64,
    preexisting_objects: u64,
    changed_objects: u64,
    mutation_bytes: u64,
    key_count: u64,
    page_count: u64,
    page_size_bytes: u64,
}

// r[impl molten.world_bench.profile]
pub fn decode_world_benchmark_input(
    bytes: &[u8],
    current_source_revision: &str,
) -> Result<(WorldBenchmarkProfile, WorldBenchmarkDataset)> {
    let input: InputProjection = serde_json::from_slice(bytes)
        .map_err(|error| MoltenError::invalid_harness(format!("world benchmark projection is invalid: {error}")))?;
    let profile = project_profile(input.profile)?;
    let dataset = project_dataset(input.dataset)?;
    let profile_issues = validate_world_benchmark_profile(&profile, current_source_revision);
    if !profile_issues.is_empty() {
        return Err(MoltenError::invalid_harness(format!(
            "world benchmark profile projection denied: {profile_issues:?}"
        )));
    }
    let dataset_issues = validate_world_benchmark_dataset(&profile, &dataset);
    if !dataset_issues.is_empty() {
        return Err(MoltenError::invalid_harness(format!(
            "world benchmark dataset projection denied: {dataset_issues:?}"
        )));
    }
    Ok((profile, dataset))
}

fn project_profile(profile: ProfileProjection) -> Result<WorldBenchmarkProfile> {
    Ok(WorldBenchmarkProfile {
        schema: profile.schema,
        profile_ref: profile.profile_ref,
        source_revision: profile.source_revision,
        dataset_ref: profile.dataset_ref,
        preparation: preparation(&profile.preparation)?,
        class: class(&profile.class)?,
        adapters: profile.adapters,
        operations: profile.operations.iter().map(|value| operation(value)).collect::<Result<Vec<_>>>()?,
        bounds: WorldBenchmarkBounds {
            max_operations: profile.bounds.max_operations,
            max_repetitions: profile.bounds.max_repetitions,
            max_logical_bytes: profile.bounds.max_logical_bytes,
            max_physical_bytes: profile.bounds.max_physical_bytes,
            max_objects: profile.bounds.max_objects,
            max_pages: profile.bounds.max_pages,
            max_references: profile.bounds.max_references,
            max_keys: profile.bounds.max_keys,
            max_conflicts: profile.bounds.max_conflicts,
            max_duration_nanoseconds: profile.bounds.max_duration_nanoseconds,
            max_peak_memory_bytes: profile.bounds.max_peak_memory_bytes,
        },
        repetitions: profile.repetitions,
        hardware_cohort: profile.hardware_cohort,
        thresholds: profile
            .thresholds
            .into_iter()
            .map(|threshold| {
                Ok(WorldBenchmarkThreshold {
                    name: threshold.name,
                    metric: metric(&threshold.metric)?,
                    maximum: threshold.maximum,
                    operation: if threshold.operation == "all" {
                        None
                    } else {
                        Some(operation(&threshold.operation)?)
                    },
                })
            })
            .collect::<Result<Vec<_>>>()?,
    })
}

fn project_dataset(dataset: DatasetProjection) -> Result<WorldBenchmarkDataset> {
    Ok(WorldBenchmarkDataset {
        dataset_ref: dataset.dataset_ref,
        source_revision: dataset.source_revision,
        shape: match dataset.shape.as_str() {
            "synthetic" => WorldBenchmarkDatasetShape::Synthetic,
            "downstream-shaped" => WorldBenchmarkDatasetShape::DownstreamShaped,
            _ => return Err(invalid_value("dataset shape", &dataset.shape)),
        },
        preparation: preparation(&dataset.preparation)?,
        logical_bytes: dataset.logical_bytes,
        object_count: dataset.object_count,
        preexisting_objects: dataset.preexisting_objects,
        changed_objects: dataset.changed_objects,
        mutation_bytes: dataset.mutation_bytes,
        key_count: dataset.key_count,
        page_count: dataset.page_count,
        page_size_bytes: dataset.page_size_bytes,
    })
}

fn preparation(value: &str) -> Result<WorldBenchmarkPreparation> {
    match value {
        "cold" => Ok(WorldBenchmarkPreparation::Cold),
        "declared-warm" => Ok(WorldBenchmarkPreparation::DeclaredWarm),
        _ => Err(invalid_value("preparation", value)),
    }
}

fn class(value: &str) -> Result<WorldBenchmarkClass> {
    match value {
        "logical" => Ok(WorldBenchmarkClass::Logical),
        "opaque-exact-snapshot" => Ok(WorldBenchmarkClass::OpaqueExactSnapshot),
        _ => Err(invalid_value("class", value)),
    }
}

fn operation(value: &str) -> Result<WorldBenchmarkOperation> {
    match value {
        "root-branch" => Ok(WorldBenchmarkOperation::RootBranch),
        "first-mutation" => Ok(WorldBenchmarkOperation::FirstMutation),
        "repeated-mutation" => Ok(WorldBenchmarkOperation::RepeatedMutation),
        "diff" => Ok(WorldBenchmarkOperation::Diff),
        "merge-plan" => Ok(WorldBenchmarkOperation::MergePlan),
        "capsule-export" => Ok(WorldBenchmarkOperation::CapsuleExport),
        "replication" => Ok(WorldBenchmarkOperation::Replication),
        "retention-plan" => Ok(WorldBenchmarkOperation::RetentionPlan),
        "snapshot-share" => Ok(WorldBenchmarkOperation::SnapshotShare),
        _ => Err(invalid_value("operation", value)),
    }
}

fn metric(value: &str) -> Result<WorldBenchmarkMetricKind> {
    WorldBenchmarkMetricKind::ALL
        .into_iter()
        .find(|kind| kind.as_str() == value)
        .ok_or_else(|| invalid_value("metric", value))
}

fn invalid_value(kind: &str, value: &str) -> MoltenError {
    MoltenError::invalid_harness(format!("unknown world benchmark {kind}: {value}"))
}

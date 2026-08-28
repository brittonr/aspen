use std::collections::BTreeSet;

use super::super::*;
use super::sorted_issues;
use super::valid_reference;
use super::valid_revision;

const FIXED_OPERATION_ISSUE_COUNT: usize = 3;
const FIXED_ADAPTER_ISSUE_COUNT: usize = 2;
const ISSUES_PER_ADAPTER: usize = 2;
const FIXED_THRESHOLD_ISSUE_COUNT: usize = 1;
const ISSUES_PER_THRESHOLD: usize = 3;
const MAX_OPERATION_ISSUES: usize = MAX_WORLD_BENCHMARK_OPERATIONS + FIXED_OPERATION_ISSUE_COUNT;
const MAX_ADAPTER_ISSUES: usize = MAX_WORLD_BENCHMARK_ADAPTERS * ISSUES_PER_ADAPTER + FIXED_ADAPTER_ISSUE_COUNT;
const MAX_THRESHOLD_ISSUES: usize = MAX_WORLD_BENCHMARK_THRESHOLDS * ISSUES_PER_THRESHOLD + FIXED_THRESHOLD_ISSUE_COUNT;

pub fn validate_world_benchmark_profile(
    profile: &WorldBenchmarkProfile,
    current_source_revision: &str,
) -> Vec<WorldBenchmarkIssue> {
    let mut issues = Vec::new();
    if profile.schema != WORLD_BENCHMARK_PROFILE_SCHEMA {
        issues.push(WorldBenchmarkIssue::SchemaMismatch);
    }
    if !valid_reference(&profile.profile_ref) {
        issues.push(WorldBenchmarkIssue::InvalidReference("profile_ref"));
    }
    if !valid_reference(&profile.dataset_ref) {
        issues.push(WorldBenchmarkIssue::InvalidReference("dataset_ref"));
    }
    if !valid_revision(&profile.source_revision) {
        issues.push(WorldBenchmarkIssue::InvalidRevision);
    } else if profile.source_revision != current_source_revision {
        issues.push(WorldBenchmarkIssue::StaleRevision);
    }
    if profile.preparation == WorldBenchmarkPreparation::Unknown {
        issues.push(WorldBenchmarkIssue::UnknownPreparation);
    }
    issues.extend(validate_operations(profile));
    issues.extend(validate_adapters(profile));
    issues.extend(validate_bounds(profile));
    issues.extend(validate_thresholds(profile));
    if profile.hardware_cohort.is_empty() || profile.hardware_cohort.len() > MAX_WORLD_BENCHMARK_TEXT_BYTES {
        issues.push(WorldBenchmarkIssue::InvalidHardwareCohort);
    }
    sorted_issues(issues)
}

pub fn validate_world_benchmark_dataset(
    profile: &WorldBenchmarkProfile,
    dataset: &WorldBenchmarkDataset,
) -> Vec<WorldBenchmarkIssue> {
    let mut issues = Vec::new();
    if dataset.dataset_ref != profile.dataset_ref || dataset.source_revision != profile.source_revision {
        issues.push(WorldBenchmarkIssue::DatasetMismatch);
    }
    if dataset.preparation == WorldBenchmarkPreparation::Unknown {
        issues.push(WorldBenchmarkIssue::UnknownPreparation);
    } else if dataset.preparation != profile.preparation {
        issues.push(WorldBenchmarkIssue::PreparationDrift);
    }
    if dataset.preexisting_objects > dataset.object_count || dataset.changed_objects > dataset.object_count {
        issues.push(WorldBenchmarkIssue::DatasetBoundsExceeded("object_counts"));
    }
    if dataset.mutation_bytes > dataset.logical_bytes {
        issues.push(WorldBenchmarkIssue::DatasetBoundsExceeded("mutation_bytes"));
    }
    if dataset.logical_bytes > profile.bounds.max_logical_bytes {
        issues.push(WorldBenchmarkIssue::DatasetBoundsExceeded("logical_bytes"));
    }
    if dataset.object_count > profile.bounds.max_objects {
        issues.push(WorldBenchmarkIssue::DatasetBoundsExceeded("object_count"));
    }
    if dataset.key_count > profile.bounds.max_keys {
        issues.push(WorldBenchmarkIssue::DatasetBoundsExceeded("key_count"));
    }
    if dataset.page_count > profile.bounds.max_pages {
        issues.push(WorldBenchmarkIssue::DatasetBoundsExceeded("page_count"));
    }
    if dataset.page_count.checked_mul(dataset.page_size_bytes).is_none() {
        issues.push(WorldBenchmarkIssue::DatasetBoundsExceeded("page_bytes_overflow"));
    }
    sorted_issues(issues)
}

fn validate_operations(profile: &WorldBenchmarkProfile) -> Vec<WorldBenchmarkIssue> {
    let mut issues = Vec::with_capacity(MAX_OPERATION_ISSUES);
    if profile.operations.is_empty() {
        issues.push(WorldBenchmarkIssue::EmptyOperations);
    }
    if profile.operations.len() > MAX_WORLD_BENCHMARK_OPERATIONS
        || u32::try_from(profile.operations.len()).is_err()
        || u32::try_from(profile.operations.len()).is_ok_and(|count| count > profile.bounds.max_operations)
    {
        issues.push(WorldBenchmarkIssue::OperationLimitExceeded);
    }
    let bounded_operations =
        profile.operations.iter().take(MAX_WORLD_BENCHMARK_OPERATIONS).copied().collect::<Vec<_>>();
    let unique = bounded_operations.iter().copied().collect::<BTreeSet<_>>();
    if unique.len() != bounded_operations.len() {
        issues.push(WorldBenchmarkIssue::OperationLimitExceeded);
    }
    for operation in &bounded_operations {
        let is_mismatch = match profile.class {
            WorldBenchmarkClass::Logical => *operation == WorldBenchmarkOperation::SnapshotShare,
            WorldBenchmarkClass::OpaqueExactSnapshot => *operation != WorldBenchmarkOperation::SnapshotShare,
        };
        if is_mismatch {
            issues.push(WorldBenchmarkIssue::OperationClassMismatch(operation.as_str()));
        }
    }
    issues
}

fn validate_adapters(profile: &WorldBenchmarkProfile) -> Vec<WorldBenchmarkIssue> {
    let mut issues = Vec::with_capacity(MAX_ADAPTER_ISSUES);
    if profile.adapters.is_empty() {
        issues.push(WorldBenchmarkIssue::EmptyAdapters);
    }
    if profile.adapters.len() > MAX_WORLD_BENCHMARK_ADAPTERS {
        issues.push(WorldBenchmarkIssue::AdapterLimitExceeded);
    }
    let mut unique = BTreeSet::new();
    for adapter in profile.adapters.iter().take(MAX_WORLD_BENCHMARK_ADAPTERS) {
        if adapter.is_empty() || adapter.len() > MAX_WORLD_BENCHMARK_TEXT_BYTES {
            issues.push(WorldBenchmarkIssue::InvalidAdapter(adapter.clone()));
        }
        if !unique.insert(adapter) {
            issues.push(WorldBenchmarkIssue::DuplicateAdapter(adapter.clone()));
        }
    }
    issues
}

fn validate_bounds(profile: &WorldBenchmarkProfile) -> Vec<WorldBenchmarkIssue> {
    let mut issues = Vec::new();
    if profile.repetitions == 0
        || profile.repetitions > MAX_WORLD_BENCHMARK_REPETITIONS
        || profile.repetitions > profile.bounds.max_repetitions
    {
        issues.push(WorldBenchmarkIssue::RepetitionLimitExceeded);
    }
    let bounds = &profile.bounds;
    if bounds.max_operations == 0
        || bounds.max_repetitions == 0
        || bounds.max_logical_bytes == 0
        || bounds.max_physical_bytes == 0
        || bounds.max_objects == 0
        || bounds.max_pages == 0
        || bounds.max_references == 0
        || bounds.max_keys == 0
        || bounds.max_duration_nanoseconds == 0
        || bounds.max_peak_memory_bytes == 0
    {
        issues.push(WorldBenchmarkIssue::DatasetBoundsExceeded("zero_bound"));
    }
    issues
}

fn validate_thresholds(profile: &WorldBenchmarkProfile) -> Vec<WorldBenchmarkIssue> {
    let mut issues = Vec::with_capacity(MAX_THRESHOLD_ISSUES);
    if profile.thresholds.len() > MAX_WORLD_BENCHMARK_THRESHOLDS {
        issues.push(WorldBenchmarkIssue::ThresholdLimitExceeded);
    }
    let mut names = BTreeSet::new();
    for threshold in profile.thresholds.iter().take(MAX_WORLD_BENCHMARK_THRESHOLDS) {
        if threshold.name.is_empty() || threshold.name.len() > MAX_WORLD_BENCHMARK_TEXT_BYTES {
            issues.push(WorldBenchmarkIssue::InvalidThreshold(threshold.name.clone()));
        }
        if !names.insert(&threshold.name) {
            issues.push(WorldBenchmarkIssue::DuplicateThreshold(threshold.name.clone()));
        }
        if threshold.operation.is_some_and(|operation| !profile.operations.contains(&operation)) {
            issues.push(WorldBenchmarkIssue::InvalidThreshold(threshold.name.clone()));
        }
    }
    issues
}

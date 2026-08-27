use std::collections::BTreeSet;

use super::super::*;
use super::metric_name;
use super::sorted_issues;
use super::valid_reference;

const ISSUES_PER_METRIC: usize = 3;
const MAX_METRIC_ISSUES: usize = WORLD_BENCHMARK_METRIC_COUNT * ISSUES_PER_METRIC;

pub fn complete_world_benchmark_metrics(values: &[(WorldBenchmarkMetricKind, u64)]) -> Vec<WorldBenchmarkMetric> {
    let supplied = values.iter().copied().collect::<std::collections::BTreeMap<_, _>>();
    WorldBenchmarkMetricKind::ALL
        .into_iter()
        .map(|kind| WorldBenchmarkMetric {
            kind,
            value: supplied.get(&kind).copied().unwrap_or_default(),
        })
        .collect()
}

pub fn validate_world_benchmark_preparation(
    plan: &WorldBenchmarkPlan,
    observation: &WorldBenchmarkPreparationObservation,
) -> Vec<WorldBenchmarkIssue> {
    let mut issues = Vec::new();
    if observation.dataset_ref != plan.dataset_ref || observation.source_revision != plan.source_revision {
        issues.push(WorldBenchmarkIssue::DatasetMismatch);
    }
    if observation.preparation == WorldBenchmarkPreparation::Unknown {
        issues.push(WorldBenchmarkIssue::UnknownPreparation);
    } else if observation.preparation != plan.preparation {
        issues.push(WorldBenchmarkIssue::PreparationDrift);
    }
    if plan.preparation == WorldBenchmarkPreparation::Cold && observation.prior_objects_available {
        issues.push(WorldBenchmarkIssue::HiddenPrepopulation);
    }
    if !valid_reference(&observation.preparation_ref) {
        issues.push(WorldBenchmarkIssue::InvalidReference("preparation_ref"));
    }
    sorted_issues(issues)
}

pub fn validate_world_benchmark_result(
    plan: &WorldBenchmarkPlan,
    result: &WorldBenchmarkResult,
) -> Vec<WorldBenchmarkIssue> {
    let mut issues = Vec::new();
    if !plan.operations.contains(&result.operation) {
        issues.push(WorldBenchmarkIssue::UnexpectedOperation(result.operation.as_str()));
    }
    if result.repetition >= plan.repetitions {
        issues.push(WorldBenchmarkIssue::InvalidRepetition);
    }
    if !plan.adapters.contains(&result.adapter_ref) {
        issues.push(WorldBenchmarkIssue::InvalidAdapter(result.adapter_ref.clone()));
    }
    issues.extend(validate_metrics(&plan.bounds, result));
    if !result.physical_measurement_independent {
        issues.push(WorldBenchmarkIssue::PhysicalMeasurementCollapsed);
    }
    issues.extend(validate_resources(&plan.bounds, result));
    issues.extend(validate_snapshot(plan, result));
    sorted_issues(issues)
}

fn validate_metrics(bounds: &WorldBenchmarkBounds, result: &WorldBenchmarkResult) -> Vec<WorldBenchmarkIssue> {
    if result.metrics.len() > WORLD_BENCHMARK_METRIC_COUNT {
        return vec![WorldBenchmarkIssue::ResultLimitExceeded];
    }
    let mut issues = Vec::with_capacity(MAX_METRIC_ISSUES);
    let mut present = BTreeSet::new();
    for metric in &result.metrics {
        if !present.insert(metric.kind) {
            issues.push(WorldBenchmarkIssue::DuplicateMetric(metric_name(metric.kind)));
        }
        let maximum = metric_bound(bounds, metric.kind);
        if metric.value > maximum {
            issues.push(WorldBenchmarkIssue::MetricBoundExceeded(metric_name(metric.kind)));
        }
    }
    for kind in WorldBenchmarkMetricKind::ALL {
        if !present.contains(&kind) {
            issues.push(WorldBenchmarkIssue::MissingMetric(metric_name(kind)));
        }
    }
    issues
}

const fn metric_bound(bounds: &WorldBenchmarkBounds, kind: WorldBenchmarkMetricKind) -> u64 {
    match kind {
        WorldBenchmarkMetricKind::LogicalBytes => bounds.max_logical_bytes,
        WorldBenchmarkMetricKind::PhysicalBytesWritten | WorldBenchmarkMetricKind::TransferredBytes => {
            bounds.max_physical_bytes
        }
        WorldBenchmarkMetricKind::NewObjects
        | WorldBenchmarkMetricKind::ReusedObjects
        | WorldBenchmarkMetricKind::RetainedObjects
        | WorldBenchmarkMetricKind::PlannedDeletions => bounds.max_objects,
        WorldBenchmarkMetricKind::CopiedPages | WorldBenchmarkMetricKind::MappedPages => bounds.max_pages,
        WorldBenchmarkMetricKind::TraversedReferences => bounds.max_references,
        WorldBenchmarkMetricKind::ComparedKeys => bounds.max_keys,
        WorldBenchmarkMetricKind::EmittedConflicts => bounds.max_conflicts,
    }
}

fn validate_resources(bounds: &WorldBenchmarkBounds, result: &WorldBenchmarkResult) -> Vec<WorldBenchmarkIssue> {
    let mut issues = Vec::new();
    if result.duration_nanoseconds.is_some_and(|value| value > bounds.max_duration_nanoseconds) {
        issues.push(WorldBenchmarkIssue::MetricBoundExceeded("duration_nanoseconds"));
    }
    if result.peak_memory_bytes.is_some_and(|value| value > bounds.max_peak_memory_bytes) {
        issues.push(WorldBenchmarkIssue::MetricBoundExceeded("peak_memory_bytes"));
    }
    issues
}

fn validate_snapshot(plan: &WorldBenchmarkPlan, result: &WorldBenchmarkResult) -> Vec<WorldBenchmarkIssue> {
    let mut issues = Vec::new();
    match plan.class {
        WorldBenchmarkClass::Logical => {
            if result.snapshot.is_some() {
                issues.push(WorldBenchmarkIssue::SnapshotBindingUnexpected);
            }
        }
        WorldBenchmarkClass::OpaqueExactSnapshot => match result.snapshot.as_ref() {
            None => issues.push(WorldBenchmarkIssue::SnapshotBindingMissing),
            Some(snapshot) => {
                if snapshot.source_revision != CHAOSCONTROL_SNAPSHOT_REVISION {
                    issues.push(WorldBenchmarkIssue::SnapshotRevisionMismatch);
                }
                if snapshot.completeness_profile != CHAOSCONTROL_SNAPSHOT_PROFILE {
                    issues.push(WorldBenchmarkIssue::SnapshotProfileMismatch);
                }
                if !valid_reference(&snapshot.descriptor_ref) {
                    issues.push(WorldBenchmarkIssue::SnapshotDescriptorInvalid);
                }
            }
        },
    }
    issues
}

use super::super::*;
use super::support::*;

// r[verify molten.world_bench.profile]
// r[verify molten.world_bench.metrics]
#[test]
fn stable_profiles_results_and_receipts_preserve_exact_metric_classes() {
    let profile = logical_profile();
    let plan = plan_world_benchmark(&profile, SOURCE_REVISION).expect("valid plan");
    let repeated = plan_world_benchmark(&profile, SOURCE_REVISION).expect("stable plan");
    assert_eq!(plan, repeated);
    assert!(validate_world_benchmark_dataset(&profile, &dataset(&profile)).is_empty());
    let receipt = finalize_world_benchmark_receipt(&plan, "molten".to_string(), results(&plan), Vec::new())
        .expect("valid receipt");
    assert!(receipt.accepted);
    assert!(receipt.threshold_results.iter().all(|threshold| threshold.passed));
    assert!(validate_world_benchmark_receipt(&receipt, SOURCE_REVISION).is_empty());
    assert_eq!(receipt.results[0].metrics.len(), WORLD_BENCHMARK_METRIC_COUNT);
}

// r[verify molten.world_bench.verification]
#[test]
fn misleading_profile_dataset_and_metric_inputs_fail_closed() {
    let mut profile = logical_profile();
    profile.preparation = WorldBenchmarkPreparation::Unknown;
    profile.hardware_cohort.clear();
    profile.source_revision = OTHER_REVISION.to_string();
    profile.thresholds[0].name.clear();
    let issues = validate_world_benchmark_profile(&profile, SOURCE_REVISION);
    assert!(issues.contains(&WorldBenchmarkIssue::UnknownPreparation));
    assert!(issues.contains(&WorldBenchmarkIssue::InvalidHardwareCohort));
    assert!(issues.contains(&WorldBenchmarkIssue::StaleRevision));
    assert!(issues.iter().any(|issue| matches!(issue, WorldBenchmarkIssue::InvalidThreshold(_))));

    let profile = logical_profile();
    let mut drifted = dataset(&profile);
    drifted.preexisting_objects = 1;
    drifted.preparation = WorldBenchmarkPreparation::DeclaredWarm;
    assert!(validate_world_benchmark_dataset(&profile, &drifted).contains(&WorldBenchmarkIssue::PreparationDrift));

    let plan = plan_world_benchmark(&profile, SOURCE_REVISION).expect("plan");
    let mut result = results(&plan).remove(0);
    result.metrics.pop();
    result.physical_measurement_independent = false;
    let result_issues = validate_world_benchmark_result(&plan, &result);
    assert!(result_issues.iter().any(|issue| matches!(issue, WorldBenchmarkIssue::MissingMetric(_))));
    assert!(result_issues.contains(&WorldBenchmarkIssue::PhysicalMeasurementCollapsed));
}

// r[verify molten.world_bench.snapshot_profiles]
#[test]
fn logical_and_opaque_snapshot_cohorts_never_compare_as_equivalent() {
    let logical = accepted_receipt("molten");
    let opaque_profile = opaque_profile();
    let opaque_plan = plan_world_benchmark(&opaque_profile, SOURCE_REVISION).expect("opaque plan");
    let opaque =
        finalize_world_benchmark_receipt(&opaque_plan, "molten".to_string(), results(&opaque_plan), Vec::new())
            .expect("opaque receipt");
    let comparison = compare_world_benchmark_receipts(&logical, &opaque);
    assert!(!comparison.comparable);
    assert!(comparison.diagnostics.contains(&"benchmark-class-mismatch".to_string()));
}

// r[verify molten.world_bench.receipt]
#[test]
fn receipt_overclaims_and_unsupported_rows_are_not_accepted() {
    let profile = logical_profile();
    let plan = plan_world_benchmark(&profile, SOURCE_REVISION).expect("plan");
    let mut receipt = finalize_world_benchmark_receipt(&plan, "molten".to_string(), results(&plan), vec![
        WorldBenchmarkUnsupportedRow {
            operation: WorldBenchmarkOperation::CapsuleExport,
            reason: "adapter-unavailable".to_string(),
        },
    ])
    .expect("bounded receipt");
    assert!(!receipt.accepted);
    receipt.accepted = true;
    receipt.non_claims = vec!["finite run proves big-O".to_string()];
    let issues = validate_world_benchmark_receipt(&receipt, SOURCE_REVISION);
    assert!(issues.contains(&WorldBenchmarkIssue::UnsupportedRowsPresent));
    assert!(issues.contains(&WorldBenchmarkIssue::ReceiptOverclaim));
    assert!(issues.contains(&WorldBenchmarkIssue::ReceiptIdentityMismatch));
}

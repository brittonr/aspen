use super::super::*;
use super::sorted_issues;
use super::valid_reference;
use super::valid_revision;

pub fn finalize_world_benchmark_receipt(
    plan: &WorldBenchmarkPlan,
    consumer_id: String,
    results: Vec<WorldBenchmarkResult>,
    unsupported_rows: Vec<WorldBenchmarkUnsupportedRow>,
) -> Result<WorldBenchmarkReceipt, Vec<WorldBenchmarkIssue>> {
    let expected_results = plan
        .operations
        .len()
        .checked_mul(usize::try_from(plan.repetitions).map_err(|_| vec![WorldBenchmarkIssue::ResultLimitExceeded])?)
        .ok_or_else(|| vec![WorldBenchmarkIssue::ResultLimitExceeded])?;
    if results.len() != expected_results || results.len() > MAX_WORLD_BENCHMARK_RESULTS {
        return Err(vec![WorldBenchmarkIssue::ResultLimitExceeded]);
    }
    let mut issues =
        results.iter().flat_map(|result| validate_world_benchmark_result(plan, result)).collect::<Vec<_>>();
    issues.extend(validate_complete_result_matrix(plan, &results));
    if !issues.is_empty() {
        return Err(sorted_issues(issues));
    }
    let threshold_results = evaluate_thresholds(&plan.thresholds, &results);
    let is_accepted = unsupported_rows.is_empty();
    let mut receipt = WorldBenchmarkReceipt {
        schema: WORLD_BENCHMARK_RECEIPT_SCHEMA.to_string(),
        receipt_ref: String::new(),
        plan_ref: plan.plan_ref.clone(),
        consumer_id,
        profile_ref: plan.profile_ref.clone(),
        source_revision: plan.source_revision.clone(),
        dataset_ref: plan.dataset_ref.clone(),
        preparation: plan.preparation,
        class: plan.class,
        adapters: plan.adapters.clone(),
        hardware_cohort: plan.hardware_cohort.clone(),
        bounds: plan.bounds.clone(),
        results,
        threshold_results,
        unsupported_rows,
        accepted: is_accepted,
        non_claims: world_benchmark_non_claims(),
    };
    receipt.receipt_ref = identify_world_benchmark_receipt(&receipt).map_err(|issue| vec![issue])?;
    Ok(receipt)
}

pub fn validate_world_benchmark_receipt(
    receipt: &WorldBenchmarkReceipt,
    current_source_revision: &str,
) -> Vec<WorldBenchmarkIssue> {
    let mut issues = Vec::new();
    if receipt.schema != WORLD_BENCHMARK_RECEIPT_SCHEMA {
        issues.push(WorldBenchmarkIssue::SchemaMismatch);
    }
    if !valid_reference(&receipt.receipt_ref)
        || !valid_reference(&receipt.plan_ref)
        || !valid_reference(&receipt.profile_ref)
        || !valid_reference(&receipt.dataset_ref)
    {
        issues.push(WorldBenchmarkIssue::InvalidReference("receipt_binding"));
    }
    if !valid_revision(&receipt.source_revision) {
        issues.push(WorldBenchmarkIssue::InvalidRevision);
    } else if receipt.source_revision != current_source_revision {
        issues.push(WorldBenchmarkIssue::StaleRevision);
    }
    if receipt.preparation == WorldBenchmarkPreparation::Unknown {
        issues.push(WorldBenchmarkIssue::UnknownPreparation);
    }
    if receipt.non_claims != world_benchmark_non_claims() {
        issues.push(WorldBenchmarkIssue::ReceiptOverclaim);
    }
    if receipt.accepted && !receipt.unsupported_rows.is_empty() {
        issues.push(WorldBenchmarkIssue::UnsupportedRowsPresent);
    }
    match identify_world_benchmark_receipt(receipt) {
        Ok(reference) if reference != receipt.receipt_ref => issues.push(WorldBenchmarkIssue::ReceiptIdentityMismatch),
        Err(issue) => issues.push(issue),
        Ok(_) => {}
    }
    sorted_issues(issues)
}

pub fn compare_world_benchmark_receipts(
    left: &WorldBenchmarkReceipt,
    right: &WorldBenchmarkReceipt,
) -> WorldBenchmarkComparison {
    let mut diagnostics = Vec::new();
    if left.class != right.class {
        diagnostics.push("benchmark-class-mismatch".to_string());
    }
    if left.profile_ref != right.profile_ref
        || left.dataset_ref != right.dataset_ref
        || left.source_revision != right.source_revision
        || left.preparation != right.preparation
        || left.hardware_cohort != right.hardware_cohort
        || left.adapters != right.adapters
    {
        diagnostics.push("benchmark-cohort-mismatch".to_string());
    }
    WorldBenchmarkComparison {
        schema: WORLD_BENCHMARK_COMPARISON_SCHEMA.to_string(),
        left_receipt_ref: left.receipt_ref.clone(),
        right_receipt_ref: right.receipt_ref.clone(),
        comparable: diagnostics.is_empty(),
        diagnostics,
        non_claims: world_benchmark_non_claims(),
    }
}

fn evaluate_thresholds(
    thresholds: &[WorldBenchmarkThreshold],
    results: &[WorldBenchmarkResult],
) -> Vec<WorldBenchmarkThresholdResult> {
    thresholds
        .iter()
        .map(|threshold| {
            let observed_maximum = results
                .iter()
                .filter(|result| threshold.operation.is_none_or(|operation| result.operation == operation))
                .filter_map(|result| result.metric(threshold.metric))
                .max()
                .unwrap_or_default();
            WorldBenchmarkThresholdResult {
                name: threshold.name.clone(),
                metric: threshold.metric,
                observed_maximum,
                admitted_maximum: threshold.maximum,
                passed: observed_maximum <= threshold.maximum,
            }
        })
        .collect()
}

fn validate_complete_result_matrix(
    plan: &WorldBenchmarkPlan,
    results: &[WorldBenchmarkResult],
) -> Vec<WorldBenchmarkIssue> {
    let mut issues = Vec::with_capacity(MAX_WORLD_BENCHMARK_RESULTS);
    for repetition in 0..plan.repetitions {
        for operation in &plan.operations {
            let count = results
                .iter()
                .filter(|result| result.repetition == repetition && result.operation == *operation)
                .count();
            if count != 1 {
                issues.push(WorldBenchmarkIssue::ResultLimitExceeded);
            }
        }
    }
    issues
}

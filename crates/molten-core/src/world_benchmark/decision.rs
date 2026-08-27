use std::collections::BTreeMap;
use std::collections::BTreeSet;

use super::*;

// r[impl molten.world_bench.extraction_decision]
pub fn classify_world_benchmark_extraction(
    evidence: &[WorldBenchmarkExtractionEvidence],
    policy: &WorldBenchmarkExtractionPolicy,
) -> Result<WorldBenchmarkExtractionDecision, Vec<WorldBenchmarkIssue>> {
    if policy.minimum_accepted_receipts_per_consumer == 0 || policy.minimum_credible_consumers == 0 {
        return Err(vec![WorldBenchmarkIssue::ExtractionPolicyInvalid]);
    }
    let accepted = evidence.iter().filter(|item| item.receipt.accepted).collect::<Vec<_>>();
    if accepted.is_empty() {
        return Err(vec![WorldBenchmarkIssue::ExtractionEvidenceInvalid]);
    }
    let required_receipts = usize::try_from(policy.minimum_accepted_receipts_per_consumer)
        .map_err(|_| vec![WorldBenchmarkIssue::ExtractionPolicyInvalid])?;
    let required_consumers = usize::try_from(policy.minimum_credible_consumers)
        .map_err(|_| vec![WorldBenchmarkIssue::ExtractionPolicyInvalid])?;
    let mut by_consumer = BTreeMap::<String, Vec<&WorldBenchmarkExtractionEvidence>>::new();
    for item in &accepted {
        by_consumer.entry(item.receipt.consumer_id.clone()).or_default().push(item);
    }
    let credible = by_consumer
        .iter()
        .filter(|(_, receipts)| receipts.len() >= required_receipts)
        .map(|(consumer, _)| consumer.clone())
        .collect::<BTreeSet<_>>();
    let repeated_product_neutral_failures = credible
        .iter()
        .filter(|consumer| {
            by_consumer.get(*consumer).is_some_and(|receipts| {
                receipts.iter().filter(|item| item.product_neutral_limit_failed).count() >= required_receipts
            })
        })
        .cloned()
        .collect::<Vec<_>>();
    let is_any_threshold_failure =
        accepted.iter().any(|item| item.receipt.threshold_results.iter().any(|threshold| !threshold.passed));
    let is_any_owned_adapter_failure = accepted
        .iter()
        .any(|item| item.owned_adapter && item.receipt.threshold_results.iter().any(|threshold| !threshold.passed));
    let is_shared_admitted = credible.len() >= required_consumers
        && repeated_product_neutral_failures.len() >= required_consumers
        && (!policy.require_product_neutral_limit || !repeated_product_neutral_failures.is_empty());
    let disposition = if is_shared_admitted {
        WorldBenchmarkExtractionDisposition::EvaluateSharedComponent
    } else if is_any_owned_adapter_failure || is_any_threshold_failure {
        WorldBenchmarkExtractionDisposition::OptimizeInPlace
    } else {
        WorldBenchmarkExtractionDisposition::RetainCurrent
    };
    let diagnostics = match disposition {
        WorldBenchmarkExtractionDisposition::RetainCurrent => vec!["accepted-requirements-pass".to_string()],
        WorldBenchmarkExtractionDisposition::OptimizeInPlace => {
            vec!["bounded-owned-or-single-consumer-limit".to_string()]
        }
        WorldBenchmarkExtractionDisposition::EvaluateSharedComponent => {
            vec!["repeated-product-neutral-limit-across-credible-consumers".to_string()]
        }
    };
    let mut accepted_receipt_refs = accepted.iter().map(|item| item.receipt.receipt_ref.clone()).collect::<Vec<_>>();
    accepted_receipt_refs.sort();
    accepted_receipt_refs.dedup();
    Ok(WorldBenchmarkExtractionDecision {
        schema: WORLD_BENCHMARK_EXTRACTION_SCHEMA.to_string(),
        disposition,
        accepted_receipt_refs,
        credible_consumers: credible.into_iter().collect(),
        diagnostics,
        creates_repository: false,
        approves_dependency: false,
        non_claims: world_benchmark_non_claims(),
    })
}

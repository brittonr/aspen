use super::super::*;
use super::support::*;

const REQUIRED_RECEIPTS: u32 = 2;
const REQUIRED_CONSUMERS: u32 = 2;

// r[verify molten.world_bench.extraction_decision]
#[test]
fn extraction_requires_repeated_product_neutral_limits_across_consumers() {
    let policy = WorldBenchmarkExtractionPolicy {
        minimum_accepted_receipts_per_consumer: REQUIRED_RECEIPTS,
        minimum_credible_consumers: REQUIRED_CONSUMERS,
        require_product_neutral_limit: true,
    };
    let retain =
        classify_world_benchmark_extraction(&[evidence("molten", false, false)], &WorldBenchmarkExtractionPolicy {
            minimum_accepted_receipts_per_consumer: 1,
            minimum_credible_consumers: REQUIRED_CONSUMERS,
            require_product_neutral_limit: true,
        })
        .expect("retain decision");
    assert_eq!(retain.disposition, WorldBenchmarkExtractionDisposition::RetainCurrent);

    let optimize = classify_world_benchmark_extraction(
        &[failing_evidence("molten", true, false)],
        &WorldBenchmarkExtractionPolicy {
            minimum_accepted_receipts_per_consumer: 1,
            minimum_credible_consumers: REQUIRED_CONSUMERS,
            require_product_neutral_limit: true,
        },
    )
    .expect("optimize decision");
    assert_eq!(optimize.disposition, WorldBenchmarkExtractionDisposition::OptimizeInPlace);

    let shared = classify_world_benchmark_extraction(
        &[
            failing_evidence("molten", false, true),
            failing_evidence("molten", false, true),
            failing_evidence("another-consumer", false, true),
            failing_evidence("another-consumer", false, true),
        ],
        &policy,
    )
    .expect("shared evaluation decision");
    assert_eq!(shared.disposition, WorldBenchmarkExtractionDisposition::EvaluateSharedComponent);
    assert!(!shared.creates_repository);
    assert!(!shared.approves_dependency);
}

#[test]
fn one_timing_miss_never_creates_a_shared_component_decision() {
    let decision = classify_world_benchmark_extraction(
        &[failing_evidence("molten", false, false)],
        &WorldBenchmarkExtractionPolicy {
            minimum_accepted_receipts_per_consumer: 1,
            minimum_credible_consumers: REQUIRED_CONSUMERS,
            require_product_neutral_limit: true,
        },
    )
    .expect("single consumer decision");
    assert_eq!(decision.disposition, WorldBenchmarkExtractionDisposition::OptimizeInPlace);
    assert!(!decision.creates_repository);
}

fn evidence(
    consumer: &str,
    owned_adapter: bool,
    product_neutral_limit_failed: bool,
) -> WorldBenchmarkExtractionEvidence {
    WorldBenchmarkExtractionEvidence {
        receipt: accepted_receipt(consumer),
        owned_adapter,
        product_neutral_limit_failed,
    }
}

fn failing_evidence(
    consumer: &str,
    owned_adapter: bool,
    product_neutral_limit_failed: bool,
) -> WorldBenchmarkExtractionEvidence {
    let mut evidence = evidence(consumer, owned_adapter, product_neutral_limit_failed);
    evidence.receipt.threshold_results[0].passed = false;
    evidence.receipt.threshold_results[0].observed_maximum =
        evidence.receipt.threshold_results[0].admitted_maximum.saturating_add(1);
    evidence.receipt.receipt_ref = identify_world_benchmark_receipt(&evidence.receipt).expect("receipt identity");
    evidence
}

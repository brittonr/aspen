use super::super::*;
use super::support::BASELINE_SAMPLE_COUNTS;
use super::support::MODERATE_IMPROVEMENT_SAMPLE_COUNTS;
use super::support::fixture_bytes_ref;
use super::support::fixture_component_bytes;
use super::support::fixture_materialized;
use super::support::fixture_optimization;
use super::support::fixture_ref;
use super::support::fixture_run;

#[test]
fn recorded_only_receipts_bind_runs_comparisons_and_external_evidence() {
    // r[verify molten.wasm_performance.evidence]
    let profile = supported_performance_profile().expect("supported performance profile");
    let bytes = fixture_component_bytes();
    let (suite, bundle, materialized) =
        fixture_materialized(&profile, PerformanceArtifactKind::PortableComponent, &bytes, fixture_bytes_ref(&bytes));
    let baseline = fixture_run(&profile, &suite, &materialized, BASELINE_SAMPLE_COUNTS);
    let candidate = fixture_run(&profile, &suite, &materialized, MODERATE_IMPROVEMENT_SAMPLE_COUNTS);
    let ComparisonDecision::Comparable(comparison) =
        compare_benchmark_runs(&profile, &baseline, &candidate).expect("performance comparison")
    else {
        panic!("fixture runs must be comparable");
    };
    let optimization = fixture_optimization();
    let input = PerformanceReceiptInput {
        run: candidate,
        comparison_peer_run: Some(baseline),
        comparison: Some(comparison),
        optimization_profile_ref: optimization_profile_ref(&optimization),
        mantle_evidence_refs: bundle.mantle_stage_receipt_refs.clone(),
        valence_evidence_refs: bundle.valence_sidecar_refs.clone(),
        conformance_receipt_refs: vec![optimization.deterministic_conformance_ref],
    };
    let receipt = build_performance_receipt(input.clone()).expect("performance receipt");
    validate_performance_receipt(&receipt).expect("performance receipt validates");
    validate_performance_receipt_against(&receipt, &input).expect("performance receipt matches expected run");
    assert_eq!(receipt.evidence_role, PerformanceEvidenceRole::RecordedOnly);
    assert!(receipt.non_claims.iter().any(|claim| claim == "not-release-eligibility"));
    let summary = performance_receipt_summary(&receipt);
    assert!(summary.contains("recorded-comparison"));
    assert!(summary.contains("role=recorded-only"));
    assert!(summary.contains("non-normative"));

    let mut missing_peer = input.clone();
    missing_peer.comparison_peer_run = None;
    assert!(build_performance_receipt(missing_peer).is_err());

    let mut fabricated = input;
    let fabricated_comparison = fabricated.comparison.as_mut().expect("comparison fixture");
    fabricated_comparison.phases[0].class = RegressionClass::Regression;
    fabricated_comparison.comparison_ref = benchmark_comparison_ref(fabricated_comparison);
    assert!(build_performance_receipt(fabricated).is_err());
}

#[test]
fn self_consistent_stale_overclaiming_and_incomplete_receipts_fail_contextual_validation() {
    // r[verify molten.wasm_performance.evidence]
    // r[verify molten.wasm_performance.validation]
    let profile = supported_performance_profile().expect("supported performance profile");
    let bytes = fixture_component_bytes();
    let (suite, bundle, materialized) =
        fixture_materialized(&profile, PerformanceArtifactKind::PortableComponent, &bytes, fixture_bytes_ref(&bytes));
    let run = fixture_run(&profile, &suite, &materialized, BASELINE_SAMPLE_COUNTS);
    let optimization = fixture_optimization();
    let input = PerformanceReceiptInput {
        run,
        comparison_peer_run: None,
        comparison: None,
        optimization_profile_ref: optimization_profile_ref(&optimization),
        mantle_evidence_refs: bundle.mantle_stage_receipt_refs.clone(),
        valence_evidence_refs: bundle.valence_sidecar_refs.clone(),
        conformance_receipt_refs: vec![optimization.deterministic_conformance_ref],
    };
    let receipt = build_performance_receipt(input.clone()).expect("performance receipt");

    let mut stale_input = input.clone();
    stale_input.run.host_class_ref = fixture_ref("other-host-class");
    stale_input.run.run_ref = benchmark_run_ref(&stale_input.run);
    let stale = build_performance_receipt(stale_input).expect("self-consistent stale receipt");
    assert!(validate_performance_receipt_against(&stale, &input).is_err());

    let mut overclaim = receipt.clone();
    overclaim.non_claims = vec!["proves-runtime-superiority".to_string()];
    overclaim.receipt_ref = crate::preserves_rail::canonical_hash(&performance_receipt_value(&overclaim))
        .expect("self-consistent overclaim receipt hash");
    assert!(validate_performance_receipt(&overclaim).is_err());

    let mut incomplete = input;
    incomplete.valence_evidence_refs.clear();
    assert!(build_performance_receipt(incomplete).is_err());
}

use super::super::*;

#[test]
fn recorded_only_receipts_bind_runs_comparisons_and_external_evidence() {
    // r[verify molten.wasm_performance.evidence]
    let profile = supported().expect("supported performance profile");
    let bytes = super::support::fixture_component_bytes();
    let (suite, bundle, materialized) = super::support::fixture_materialized(
        &profile,
        ArtifactKind::PortableComponent,
        &bytes,
        super::support::fixture_bytes_ref(&bytes),
    );
    let baseline = super::support::fixture_run(&profile, &suite, &materialized, super::support::BASELINE_SAMPLE_COUNTS);
    let candidate = super::support::fixture_run(
        &profile,
        &suite,
        &materialized,
        super::support::MODERATE_IMPROVEMENT_SAMPLE_COUNTS,
    );
    let ComparisonDecision::Comparable(comparison) =
        compare_benchmark_runs(&profile, &baseline, &candidate).expect("performance comparison")
    else {
        panic!("fixture runs must be comparable");
    };
    let optimization = super::support::fixture_optimization();
    let input = ReceiptInput {
        run: candidate,
        comparison_peer_run: Some(baseline),
        comparison: Some(comparison),
        optimization_profile_ref: profile_ref(&optimization),
        mantle_evidence_refs: bundle.mantle_stage_receipt_refs.clone(),
        valence_evidence_refs: bundle.valence_sidecar_refs.clone(),
        conformance_receipt_refs: vec![optimization.deterministic_conformance_ref],
    };
    let receipt = build_receipt(input.clone()).expect("performance receipt");
    validate_receipt(&receipt).expect("performance receipt validates");
    validate_receipt_against(&receipt, &input).expect("performance receipt matches expected run");
    assert_eq!(receipt.evidence_role, EvidenceRole::RecordedOnly);
    assert!(receipt.non_claims.iter().any(|claim| claim == "not-release-eligibility"));
    let summary = receipt_summary(&receipt);
    assert!(summary.contains("recorded-comparison"));
    assert!(summary.contains("role=recorded-only"));
    assert!(summary.contains("non-normative"));

    let mut missing_peer = input.clone();
    missing_peer.comparison_peer_run = None;
    assert!(build_receipt(missing_peer).is_err());

    let mut fabricated = input;
    let fabricated_comparison = fabricated.comparison.as_mut().expect("comparison fixture");
    fabricated_comparison.phases[0].class = RegressionClass::Regression;
    fabricated_comparison.comparison_ref = benchmark_ref(fabricated_comparison);
    assert!(build_receipt(fabricated).is_err());
}

#[test]
fn self_consistent_stale_overclaiming_and_incomplete_receipts_fail_contextual_validation() {
    // r[verify molten.wasm_performance.evidence]
    // r[verify molten.wasm_performance.validation]
    let profile = supported().expect("supported performance profile");
    let bytes = super::support::fixture_component_bytes();
    let (suite, bundle, materialized) = super::support::fixture_materialized(
        &profile,
        ArtifactKind::PortableComponent,
        &bytes,
        super::support::fixture_bytes_ref(&bytes),
    );
    let run = super::support::fixture_run(&profile, &suite, &materialized, super::support::BASELINE_SAMPLE_COUNTS);
    let optimization = super::support::fixture_optimization();
    let input = ReceiptInput {
        run,
        comparison_peer_run: None,
        comparison: None,
        optimization_profile_ref: profile_ref(&optimization),
        mantle_evidence_refs: bundle.mantle_stage_receipt_refs.clone(),
        valence_evidence_refs: bundle.valence_sidecar_refs.clone(),
        conformance_receipt_refs: vec![optimization.deterministic_conformance_ref],
    };
    let receipt = build_receipt(input.clone()).expect("performance receipt");

    let mut stale_input = input.clone();
    stale_input.run.host_class_ref = super::support::fixture_ref("other-host-class");
    stale_input.run.run_ref = benchmark_run_ref(&stale_input.run);
    let stale = build_receipt(stale_input).expect("self-consistent stale receipt");
    assert!(validate_receipt_against(&stale, &input).is_err());

    let mut overclaim = receipt.clone();
    overclaim.non_claims = vec!["proves-runtime-superiority".to_string()];
    overclaim.receipt_ref = crate::preserves_rail::canonical_hash(&receipt_value(&overclaim))
        .expect("self-consistent overclaim receipt hash");
    assert!(validate_receipt(&overclaim).is_err());

    let mut incomplete = input;
    incomplete.valence_evidence_refs.clear();
    assert!(build_receipt(incomplete).is_err());
}

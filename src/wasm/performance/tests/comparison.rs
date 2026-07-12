use super::super::*;
use super::support::BASELINE_SAMPLE_COUNTS;
use super::support::IMPROVED_SAMPLE_COUNTS;
use super::support::MODERATE_IMPROVEMENT_SAMPLE_COUNTS;
use super::support::REGRESSION_SAMPLE_COUNTS;
use super::support::fixture_alternate_component_bytes;
use super::support::fixture_bundle;
use super::support::fixture_bytes_ref;
use super::support::fixture_component_bytes;
use super::support::fixture_host;
use super::support::fixture_ref;
use super::support::fixture_run;
use super::support::phase_samples;

fn comparison_fixture()
-> (PerformanceProfile, BenchmarkSuite, MaterializedPerformanceArtifact, MaterializedPerformanceArtifact) {
    let profile = supported_performance_profile().expect("supported performance profile");
    let source_bytes = fixture_component_bytes();
    let source_ref = fixture_bytes_ref(&source_bytes);
    let baseline_bundle = fixture_bundle(PerformanceArtifactKind::PortableComponent, &source_bytes, source_ref.clone());
    let candidate_bytes = fixture_alternate_component_bytes();
    let candidate_bundle = fixture_bundle(PerformanceArtifactKind::WizerComponent, &candidate_bytes, source_ref);
    let mut suite = profile.fast.clone();
    suite.materialization_bundle_refs = vec![baseline_bundle.bundle_ref.clone(), candidate_bundle.bundle_ref.clone()];
    suite.materialization_bundle_refs.sort();
    let baseline =
        verify_performance_materialization(&suite, &baseline_bundle, &source_bytes).expect("baseline materialization");
    let candidate = verify_performance_materialization(&suite, &candidate_bundle, &candidate_bytes)
        .expect("candidate materialization");
    (profile, suite, baseline, candidate)
}

#[test]
fn compatible_runs_produce_deterministic_effect_confidence_and_regression_classes() {
    // r[verify molten.wasm_performance.comparison]
    // r[verify molten.wasm_performance.functional_core]
    let (profile, suite, baseline_artifact, candidate_artifact) = comparison_fixture();
    let baseline = fixture_run(&profile, &suite, &baseline_artifact, BASELINE_SAMPLE_COUNTS);
    let improved = fixture_run(&profile, &suite, &candidate_artifact, IMPROVED_SAMPLE_COUNTS);
    let repeated = compare_benchmark_runs(&profile, &baseline, &improved).expect("comparison");
    let ComparisonDecision::Comparable(first) = repeated else {
        panic!("compatible runs must compare");
    };
    let ComparisonDecision::Comparable(second) =
        compare_benchmark_runs(&profile, &baseline, &improved).expect("repeated comparison")
    else {
        panic!("repeated compatible runs must compare");
    };
    assert_eq!(first, second);
    assert!(first.phases.iter().all(|phase| phase.class == RegressionClass::Improvement));
    assert!(first.phases.iter().all(|phase| phase.candidate_ratio_ppm < profile.comparison.parts_per_million));
    assert!(first.comparison_ref.starts_with("blake3:"));

    let regression = fixture_run(&profile, &suite, &candidate_artifact, REGRESSION_SAMPLE_COUNTS);
    let ComparisonDecision::Comparable(regression) =
        compare_benchmark_runs(&profile, &baseline, &regression).expect("regression comparison")
    else {
        panic!("compatible regression runs must compare");
    };
    assert!(regression.phases.iter().all(|phase| phase.class == RegressionClass::Regression));

    let unchanged = fixture_run(&profile, &suite, &candidate_artifact, BASELINE_SAMPLE_COUNTS);
    let ComparisonDecision::Comparable(unchanged) =
        compare_benchmark_runs(&profile, &baseline, &unchanged).expect("unchanged comparison")
    else {
        panic!("compatible unchanged runs must compare");
    };
    assert!(unchanged.phases.iter().all(|phase| phase.class == RegressionClass::NoSignificantChange));
}

#[test]
fn incompatible_host_runtime_suite_and_undersampled_runs_are_never_ranked() {
    // r[verify molten.wasm_performance.comparison]
    // r[verify molten.wasm_performance.validation]
    let (profile, suite, baseline_artifact, candidate_artifact) = comparison_fixture();
    let baseline = fixture_run(&profile, &suite, &baseline_artifact, BASELINE_SAMPLE_COUNTS);
    let mut incompatible = fixture_run(&profile, &suite, &candidate_artifact, MODERATE_IMPROVEMENT_SAMPLE_COUNTS);
    incompatible.host_class_ref = fixture_ref("different-host-class");
    incompatible.run_ref = benchmark_run_ref(&incompatible);
    let ComparisonDecision::Incompatible { blockers } =
        compare_benchmark_runs(&profile, &baseline, &incompatible).expect("incompatible report")
    else {
        panic!("cross-host runs must not be ranked");
    };
    assert!(blockers.iter().any(|blocker| blocker.contains("host class")));

    let mut cross_runtime = incompatible.clone();
    cross_runtime.host_class_ref = baseline.host_class_ref.clone();
    cross_runtime.engine_cohort_ref = fixture_ref("other-runtime");
    cross_runtime.run_ref = benchmark_run_ref(&cross_runtime);
    let ComparisonDecision::Incompatible { blockers } =
        compare_benchmark_runs(&profile, &baseline, &cross_runtime).expect("cross-runtime report")
    else {
        panic!("cross-runtime runs must not be ranked");
    };
    assert!(blockers.iter().any(|blocker| blocker.contains("engine cohort")));

    let mut stale_suite = suite.clone();
    stale_suite.measurement = "nanoseconds".to_string();
    assert!(validate_suite_instance(&profile, &stale_suite).is_err());
    let mut renamed_suite = suite.clone();
    renamed_suite.suite_id = "unreviewed-suite".to_string();
    assert!(validate_suite_instance(&profile, &renamed_suite).is_err());

    let mut too_few = phase_samples(BASELINE_SAMPLE_COUNTS);
    for phase in &mut too_few {
        phase.samples.pop();
    }
    assert!(
        build_benchmark_run(BenchmarkRunInput {
            profile: &profile,
            suite: &suite,
            materialized: &baseline_artifact,
            host: &fixture_host(&suite, &baseline_artifact),
            benchmark_ref: suite.workload_refs[0].clone(),
            recorded_effect_refs: vec![fixture_ref("recorded-host-effect")],
            phases: too_few,
        })
        .is_err()
    );
}

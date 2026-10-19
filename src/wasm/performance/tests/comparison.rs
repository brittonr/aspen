use super::super::*;

fn fixture() -> (Profile, BenchmarkSuite, MaterializedArtifact, MaterializedArtifact) {
    let profile = supported().expect("supported performance profile");
    let source_bytes = super::support::fixture_component_bytes();
    let source_ref = super::support::fixture_bytes_ref(&source_bytes);
    let baseline_bundle =
        super::support::fixture_bundle(ArtifactKind::PortableComponent, &source_bytes, source_ref.clone());
    let candidate_bytes = super::support::fixture_alternate_component_bytes();
    let candidate_bundle = super::support::fixture_bundle(ArtifactKind::WizerComponent, &candidate_bytes, source_ref);
    let mut suite = profile.fast.clone();
    suite.materialization_bundle_refs = vec![baseline_bundle.bundle_ref.clone(), candidate_bundle.bundle_ref.clone()];
    suite.materialization_bundle_refs.sort();
    let baseline = verify(&suite, &baseline_bundle, &source_bytes).expect("baseline materialization");
    let candidate = verify(&suite, &candidate_bundle, &candidate_bytes).expect("candidate materialization");
    (profile, suite, baseline, candidate)
}

#[test]
fn compatible_runs_produce_deterministic_effect_confidence_and_regression_classes() {
    // r[verify molten.wasm_performance.comparison]
    // r[verify molten.wasm_performance.functional_core]
    let (profile, suite, baseline_artifact, candidate_artifact) = fixture();
    let baseline =
        super::support::fixture_run(&profile, &suite, &baseline_artifact, super::support::BASELINE_SAMPLE_COUNTS);
    let improved =
        super::support::fixture_run(&profile, &suite, &candidate_artifact, super::support::IMPROVED_SAMPLE_COUNTS);
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

    let regression =
        super::support::fixture_run(&profile, &suite, &candidate_artifact, super::support::REGRESSION_SAMPLE_COUNTS);
    let ComparisonDecision::Comparable(regression) =
        compare_benchmark_runs(&profile, &baseline, &regression).expect("regression comparison")
    else {
        panic!("compatible regression runs must compare");
    };
    assert!(regression.phases.iter().all(|phase| phase.class == RegressionClass::Regression));

    let unchanged =
        super::support::fixture_run(&profile, &suite, &candidate_artifact, super::support::BASELINE_SAMPLE_COUNTS);
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
    let (profile, suite, baseline_artifact, candidate_artifact) = fixture();
    let baseline =
        super::support::fixture_run(&profile, &suite, &baseline_artifact, super::support::BASELINE_SAMPLE_COUNTS);
    let mut incompatible = super::support::fixture_run(
        &profile,
        &suite,
        &candidate_artifact,
        super::support::MODERATE_IMPROVEMENT_SAMPLE_COUNTS,
    );
    incompatible.host_class_ref = super::support::fixture_ref("different-host-class");
    incompatible.run_ref = benchmark_run_ref(&incompatible);
    let ComparisonDecision::Incompatible { blockers } =
        compare_benchmark_runs(&profile, &baseline, &incompatible).expect("incompatible report")
    else {
        panic!("cross-host runs must not be ranked");
    };
    assert!(blockers.iter().any(|blocker| blocker.contains("host class")));

    let mut cross_runtime = incompatible.clone();
    cross_runtime.host_class_ref = baseline.host_class_ref.clone();
    cross_runtime.engine_cohort_ref = super::support::fixture_ref("other-runtime");
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

    let mut too_few = super::support::phase_samples(super::support::BASELINE_SAMPLE_COUNTS);
    for phase in &mut too_few {
        phase.samples.pop();
    }
    assert!(
        build_benchmark_run(BenchmarkRunInput {
            profile: &profile,
            suite: &suite,
            materialized: &baseline_artifact,
            host: &super::support::fixture_host(&suite, &baseline_artifact),
            benchmark_ref: suite.workload_refs[0].clone(),
            recorded_effect_refs: vec![super::support::fixture_ref("recorded-host-effect")],
            phases: too_few,
        })
        .is_err()
    );
}

#[test]
fn phase_count_preflight_builder_preserves_admitted_shapes() {
    let (profile, suite, artifact, _) = fixture();
    let host = super::support::fixture_host(&suite, &artifact);
    assert_eq!(suite.phases.len(), Phase::ALL.len());
    let build = |phases: Vec<PhaseSamples>, benchmark_ref: String| {
        build_benchmark_run(BenchmarkRunInput {
            profile: &profile,
            suite: &suite,
            materialized: &artifact,
            host: &host,
            benchmark_ref,
            recorded_effect_refs: vec![super::support::fixture_ref("recorded-host-effect")],
            phases,
        })
    };

    let mut unordered = super::support::phase_samples(super::support::BASELINE_SAMPLE_COUNTS);
    unordered.swap(0, 2);
    let exact = build(unordered, suite.workload_refs[0].clone()).expect("three phases normalize in suite order");
    assert_eq!(exact.phases.iter().map(|phase| phase.phase).collect::<Vec<_>>(), Phase::ALL);
    validate_benchmark_run(&exact).expect("exact reviewed phase count validates");

    let mut malformed = super::support::phase_samples(super::support::BASELINE_SAMPLE_COUNTS);
    malformed[0].event.clear();
    let error =
        build(malformed, suite.workload_refs[0].clone()).expect_err("admitted malformed phase shape still fails");
    assert!(error.blockers.contains(&"benchmark compilation phase has no event or too few samples".to_string()));

    let mut duplicate = super::support::phase_samples(super::support::BASELINE_SAMPLE_COUNTS);
    duplicate[2] = duplicate[1].clone();
    let error =
        build(duplicate, suite.workload_refs[0].clone()).expect_err("three admitted groups retain duplicate rejection");
    assert!(error.blockers.contains(&"benchmark phase/event groups must be sorted and unique".to_string()));
    assert!(error.blockers.contains(&"benchmark run omits the execution phase".to_string()));

    let mut missing = super::support::phase_samples(super::support::BASELINE_SAMPLE_COUNTS);
    missing.pop();
    let error = build(missing, suite.workload_refs[0].clone()).expect_err("two groups retain missing-phase error");
    assert!(error.blockers.contains(&"benchmark run omits the execution phase".to_string()));

    let mut excess = super::support::phase_samples(super::support::BASELINE_SAMPLE_COUNTS);
    excess.push(excess[0].clone());
    let error =
        build(excess.clone(), "invalid".to_string()).expect_err("invalid workload still precedes phase-count check");
    assert_eq!(error.blockers, vec!["benchmark workload identity is malformed or absent from the exact suite"]);
    let error = build(excess, suite.workload_refs[0].clone()).expect_err("four groups are denied before normalization");
    assert_eq!(error.blockers, vec!["benchmark run exceeds reviewed suite phase count"]);
}

#[test]
fn phase_count_preflight_validator_preserves_admitted_shapes() {
    let (profile, suite, artifact, _) = fixture();
    let run = super::support::fixture_run(&profile, &suite, &artifact, super::support::BASELINE_SAMPLE_COUNTS);
    assert_eq!(run.phases.len(), Phase::ALL.len());
    validate_benchmark_run(&run).expect("three reviewed phase groups validate");

    let mut malformed = run.clone();
    malformed.phases[0].event.clear();
    malformed.run_ref = benchmark_run_ref(&malformed);
    let error = validate_benchmark_run(&malformed).expect_err("admitted malformed phase still fails");
    assert!(error.blockers.contains(&"benchmark compilation phase has no event or too few samples".to_string()));

    let mut duplicate = run.clone();
    duplicate.phases[2] = duplicate.phases[1].clone();
    duplicate.run_ref = benchmark_run_ref(&duplicate);
    let error = validate_benchmark_run(&duplicate).expect_err("three admitted groups retain duplicate rejection");
    assert!(error.blockers.contains(&"benchmark phase/event groups must be sorted and unique".to_string()));

    let mut empty = run.clone();
    empty.phases.clear();
    empty.run_ref = benchmark_run_ref(&empty);
    let error = validate_benchmark_run(&empty).expect_err("empty admitted run retains specific error");
    assert_eq!(error.blockers, vec!["benchmark run has no phase samples"]);

    let mut excess = run;
    excess.phases.push(excess.phases[0].clone());
    excess.run_ref = "invalid".to_string();
    let error = validate_benchmark_run(&excess).expect_err("four forged groups are denied before formatting");
    assert_eq!(error.blockers, vec!["benchmark run exceeds reviewed suite phase count"]);
}

const SAMPLE_COUNT: usize = 3;
const CONSTANT_COUNT: u64 = 100;
const RAMP_MIDDLE: u64 = 2;
const RAMP_COUNTS: [u64; SAMPLE_COUNT] = [RAMP_MIDDLE - 1, RAMP_MIDDLE, RAMP_MIDDLE + 1];
const RAMP_CONFIDENCE: u64 = 1_131_606;

fn assert_statistics(
    baseline_counts: [u64; SAMPLE_COUNT],
    candidate_counts: [u64; SAMPLE_COUNT],
    expected: crate::wasm_performance::PhaseComparison,
) -> crate::wasm_performance::PerformanceResult<()> {
    let (profile, suite, baseline_artifact, candidate_artifact) = super::comparison_fixture();
    let baseline = super::fixture_run(&profile, &suite, &baseline_artifact, baseline_counts);
    let candidate = super::fixture_run(&profile, &suite, &candidate_artifact, candidate_counts);
    let before = (baseline.clone(), candidate.clone());
    let phases = baseline
        .phases
        .iter()
        .map(|phase| crate::wasm_performance::PhaseComparison {
            phase: phase.phase,
            event: phase.event.clone(),
            ..expected.clone()
        })
        .collect();
    let mut comparison = crate::wasm_performance::BenchmarkComparison {
        baseline_run_ref: baseline.run_ref.clone(),
        candidate_run_ref: candidate.run_ref.clone(),
        suite_ref: baseline.suite_ref.clone(),
        phases,
        comparison_ref: String::new(),
    };
    comparison.comparison_ref = crate::wasm_performance::benchmark_comparison_ref(&comparison);
    assert_eq!(
        crate::wasm_performance::compare_benchmark_runs(&profile, &baseline, &candidate)?,
        crate::wasm_performance::ComparisonDecision::Comparable(comparison)
    );
    assert_eq!((baseline, candidate), before);
    Ok(())
}

fn constant_statistics(
    count: u64,
) -> crate::wasm_performance::PerformanceResult<crate::wasm_performance::PhaseComparison> {
    let profile = crate::wasm_performance::supported_performance_profile()?;
    let mean = u128::from(count) * u128::from(profile.comparison.parts_per_million);
    Ok(crate::wasm_performance::PhaseComparison {
        phase: crate::wasm_performance::PerformancePhase::Compilation,
        event: String::new(),
        baseline_mean_scaled: mean,
        candidate_mean_scaled: mean,
        baseline_confidence_half_width_scaled: 0,
        candidate_confidence_half_width_scaled: 0,
        candidate_ratio_ppm: profile.comparison.parts_per_million,
        ratio_confidence_half_width_ppm: 0,
        class: crate::wasm_performance::RegressionClass::NoSignificantChange,
    })
}

#[test]
fn constant_samples_preserve_fields_and_identity() -> crate::wasm_performance::PerformanceResult<()> {
    assert_statistics(
        [CONSTANT_COUNT; SAMPLE_COUNT],
        [CONSTANT_COUNT; SAMPLE_COUNT],
        constant_statistics(CONSTANT_COUNT)?,
    )
}

#[test]
fn nonsquare_variance_preserves_floor_confidence() -> crate::wasm_performance::PerformanceResult<()> {
    let mut expected = constant_statistics(RAMP_MIDDLE)?;
    expected.baseline_confidence_half_width_scaled = u128::from(RAMP_CONFIDENCE);
    expected.candidate_confidence_half_width_scaled = u128::from(RAMP_CONFIDENCE);
    expected.ratio_confidence_half_width_ppm = RAMP_CONFIDENCE;
    assert_statistics(RAMP_COUNTS, RAMP_COUNTS, expected)
}

#[test]
fn threshold_equality_is_not_a_regression() -> crate::wasm_performance::PerformanceResult<()> {
    let profile = crate::wasm_performance::supported_performance_profile()?;
    let mut expected = constant_statistics(CONSTANT_COUNT)?;
    expected.candidate_mean_scaled = u128::from(CONSTANT_COUNT + 1) * u128::from(profile.comparison.parts_per_million);
    expected.candidate_ratio_ppm += profile.comparison.practical_threshold_ppm;
    assert_statistics([CONSTANT_COUNT; SAMPLE_COUNT], [CONSTANT_COUNT + 1; SAMPLE_COUNT], expected)
}

fn replace_counts(run: &mut crate::wasm_performance::BenchmarkRun, count: u64) {
    for phase in &mut run.phases {
        for sample in &mut phase.samples {
            sample.count = count;
        }
    }
    run.run_ref = crate::wasm_performance::benchmark_run_ref(run);
}

#[test]
fn zero_baseline_and_candidate_overflow_keep_error_precedence() {
    let (profile, suite, baseline_artifact, candidate_artifact) = super::comparison_fixture();
    let mut baseline = super::fixture_run(&profile, &suite, &baseline_artifact, RAMP_COUNTS);
    let mut candidate = super::fixture_run(&profile, &suite, &candidate_artifact, RAMP_COUNTS);
    replace_counts(&mut baseline, 0);
    let zero_before = (baseline.clone(), candidate.clone());
    assert_eq!(
        crate::wasm_performance::compare_benchmark_runs(&profile, &baseline, &candidate),
        Err(crate::wasm_performance::PerformanceDenial::new("benchmark baseline mean cannot be zero"))
    );
    assert_eq!((&baseline, &candidate), (&zero_before.0, &zero_before.1));

    let extreme_counts = [0, u64::MAX];
    for phase in &mut candidate.phases {
        phase.samples.truncate(extreme_counts.len());
        assert_eq!(phase.samples.len(), extreme_counts.len());
        for (sample, count) in phase.samples.iter_mut().zip(extreme_counts) {
            sample.count = count;
        }
    }
    candidate.run_ref = crate::wasm_performance::benchmark_run_ref(&candidate);
    let overflow_before = (baseline.clone(), candidate.clone());
    assert_eq!(
        crate::wasm_performance::compare_benchmark_runs(&profile, &baseline, &candidate),
        Err(crate::wasm_performance::PerformanceDenial::new("benchmark squared deviation overflowed"))
    );
    assert_eq!((baseline, candidate), overflow_before);
}

#[test]
fn empty_and_singleton_phases_reject_without_mutation() {
    let (profile, suite, baseline_artifact, candidate_artifact) = super::comparison_fixture();
    let baseline = super::fixture_run(&profile, &suite, &baseline_artifact, RAMP_COUNTS);
    let candidate = super::fixture_run(&profile, &suite, &candidate_artifact, RAMP_COUNTS);
    for retained in [0, 1] {
        let mut invalid = candidate.clone();
        for phase in &mut invalid.phases {
            phase.samples.truncate(retained);
        }
        invalid.run_ref = crate::wasm_performance::benchmark_run_ref(&invalid);
        let before = (baseline.clone(), invalid.clone());
        assert_eq!(
            crate::wasm_performance::compare_benchmark_runs(&profile, &baseline, &invalid),
            Err(crate::wasm_performance::PerformanceDenial::from_blockers([
                "benchmark compilation phase has no event or too few samples".to_string(),
                "benchmark execution phase has no event or too few samples".to_string(),
                "benchmark instantiation phase has no event or too few samples".to_string(),
            ]))
        );
        assert_eq!((&baseline, &invalid), (&before.0, &before.1));
    }
}

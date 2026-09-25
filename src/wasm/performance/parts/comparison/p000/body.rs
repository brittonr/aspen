const MIN_STATISTICAL_SAMPLES: usize = 2;
const NORMAL_95_MULTIPLIER_MILLI: u128 = 1_960;
const MILLI_SCALE: u128 = 1_000;
const BINARY_SEARCH_HALVING_SHIFT: u32 = 1;
const MAX_RECORDED_EFFECT_REFS: usize = 128;

#[derive(Debug, Clone)]
pub struct BenchmarkRunInput<'a> {
    pub profile: &'a super::model::PerformanceProfile,
    pub suite: &'a super::model::BenchmarkSuite,
    pub materialized: &'a super::model::MaterializedPerformanceArtifact,
    pub host: &'a super::model::BenchmarkHostFacts,
    pub benchmark_ref: String,
    pub recorded_effect_refs: Vec<String>,
    pub phases: Vec<super::model::PhaseSamples>,
}

pub fn build_benchmark_run(
    input: BenchmarkRunInput<'_>,
) -> super::model::PerformanceResult<super::model::BenchmarkRun> {
    super::profile::validate_performance_profile(input.profile)?;
    validate_suite_instance(input.profile, input.suite)?;
    validate_run_inputs(&input)?;
    let phases = normalize_phase_groups(input.profile, input.suite, input.phases)?;
    let mut run = super::model::BenchmarkRun {
        suite_ref: super::profile::performance_suite_ref(input.suite),
        run_ref: String::new(),
        benchmark_ref: input.benchmark_ref,
        consumer: input.materialized.consumer,
        source_component_ref: input.materialized.source_component_ref.clone(),
        component_ref: input.materialized.artifact_ref.clone(),
        component_profile_ref: input.materialized.component_profile_ref.clone(),
        performance_profile_ref: super::profile::performance_profile_ref(input.profile),
        engine_cohort_ref: input.suite.engine_cohort_ref.clone(),
        engine_artifact_ref: input.suite.engine_artifact_ref.clone(),
        runner_artifact_ref: input.suite.runner_artifact_ref.clone(),
        runtime_configuration_ref: input.materialized.runtime_configuration_ref.clone(),
        target: input.host.target.clone(),
        host_class_ref: input.host.host_class_ref.clone(),
        measurement: input.host.measurement.clone(),
        resource_envelope_ref: input.suite.resource_envelope_ref.clone(),
        recorded_effect_refs: super::model::sorted_unique(&input.recorded_effect_refs),
        phases,
    };
    run.run_ref = benchmark_run_ref(&run);
    Ok(run)
}

pub fn validate_benchmark_run(run: &super::model::BenchmarkRun) -> super::model::PerformanceResult<()> {
    let mut blockers = [
        ("suite", run.suite_ref.as_str()),
        ("benchmark", run.benchmark_ref.as_str()),
        ("source component", run.source_component_ref.as_str()),
        ("component", run.component_ref.as_str()),
        ("component profile", run.component_profile_ref.as_str()),
        ("performance profile", run.performance_profile_ref.as_str()),
        ("engine cohort", run.engine_cohort_ref.as_str()),
        ("engine artifact", run.engine_artifact_ref.as_str()),
        ("runner artifact", run.runner_artifact_ref.as_str()),
        ("runtime configuration", run.runtime_configuration_ref.as_str()),
        ("host class", run.host_class_ref.as_str()),
        ("resource envelope", run.resource_envelope_ref.as_str()),
    ]
    .into_iter()
    .filter(|(_, value)| !super::model::valid_content_ref(value))
    .map(|(label, _)| format!("benchmark run {label} ref is malformed"))
    .collect::<Vec<_>>();
    if run.target.trim().is_empty() || run.measurement.trim().is_empty() {
        blockers.push("benchmark run target or measurement is empty".to_string());
    }
    if run.recorded_effect_refs.len() > MAX_RECORDED_EFFECT_REFS
        || !super::model::valid_ref_collection(&run.recorded_effect_refs)
    {
        blockers
            .push("benchmark run recorded-effect refs are missing, malformed, duplicate, or over bound".to_string());
    }
    validate_normalized_phases(&run.phases, &mut blockers);
    if run.run_ref != benchmark_run_ref(run) {
        blockers.push("benchmark run identity does not match its canonical samples".to_string());
    }
    if blockers.is_empty() {
        Ok(())
    } else {
        Err(super::model::PerformanceDenial::from_blockers(blockers))
    }
}

pub fn compare_benchmark_runs(
    profile: &super::model::PerformanceProfile,
    baseline: &super::model::BenchmarkRun,
    candidate: &super::model::BenchmarkRun,
) -> super::model::PerformanceResult<super::model::ComparisonDecision> {
    super::profile::validate_performance_profile(profile)?;
    validate_benchmark_run(baseline)?;
    validate_benchmark_run(candidate)?;
    let blockers = compatibility_blockers(baseline, candidate);
    if !blockers.is_empty() {
        return Ok(super::model::ComparisonDecision::Incompatible { blockers });
    }
    let mut phases = Vec::with_capacity(baseline.phases.len());
    for (baseline_phase, candidate_phase) in baseline.phases.iter().zip(&candidate.phases) {
        phases.push(compare_phase(profile, baseline_phase, candidate_phase)?);
    }
    let mut comparison = super::model::BenchmarkComparison {
        baseline_run_ref: baseline.run_ref.clone(),
        candidate_run_ref: candidate.run_ref.clone(),
        suite_ref: baseline.suite_ref.clone(),
        phases,
        comparison_ref: String::new(),
    };
    comparison.comparison_ref = benchmark_comparison_ref(&comparison);
    Ok(super::model::ComparisonDecision::Comparable(comparison))
}

pub fn benchmark_run_ref(run: &super::model::BenchmarkRun) -> String {
    let mut lines = vec![
        format!("suite-ref:{}", run.suite_ref),
        format!("benchmark-ref:{}", run.benchmark_ref),
        format!("consumer:{}", run.consumer.as_str()),
        format!("source-component-ref:{}", run.source_component_ref),
        format!("component-ref:{}", run.component_ref),
        format!("component-profile-ref:{}", run.component_profile_ref),
        format!("performance-profile-ref:{}", run.performance_profile_ref),
        format!("engine-cohort-ref:{}", run.engine_cohort_ref),
        format!("engine-artifact-ref:{}", run.engine_artifact_ref),
        format!("runner-artifact-ref:{}", run.runner_artifact_ref),
        format!("runtime-configuration-ref:{}", run.runtime_configuration_ref),
        format!("target:{}", run.target),
        format!("host-class-ref:{}", run.host_class_ref),
        format!("measurement:{}", run.measurement),
        format!("resource-envelope-ref:{}", run.resource_envelope_ref),
    ];
    lines.extend(run.recorded_effect_refs.iter().map(|value| format!("recorded-effect-ref:{value}")));
    for phase in &run.phases {
        lines.push(format!("phase:{}", phase.phase.as_str()));
        lines.push(format!("event:{}", phase.event));
        lines.extend(
            phase
                .samples
                .iter()
                .map(|sample| format!("sample:{}:{}:{}", sample.process, sample.iteration, sample.count)),
        );
    }
    super::model::content_ref(lines.join("\n").as_bytes())
}

pub fn benchmark_comparison_ref(comparison: &super::model::BenchmarkComparison) -> String {
    let mut lines = vec![
        format!("baseline-run-ref:{}", comparison.baseline_run_ref),
        format!("candidate-run-ref:{}", comparison.candidate_run_ref),
        format!("suite-ref:{}", comparison.suite_ref),
    ];
    for phase in &comparison.phases {
        lines.extend([
            format!("phase:{}", phase.phase.as_str()),
            format!("event:{}", phase.event),
            format!("baseline-mean-scaled:{}", phase.baseline_mean_scaled),
            format!("candidate-mean-scaled:{}", phase.candidate_mean_scaled),
            format!("baseline-confidence-half-width-scaled:{}", phase.baseline_confidence_half_width_scaled),
            format!("candidate-confidence-half-width-scaled:{}", phase.candidate_confidence_half_width_scaled),
            format!("candidate-ratio-ppm:{}", phase.candidate_ratio_ppm),
            format!("ratio-confidence-half-width-ppm:{}", phase.ratio_confidence_half_width_ppm),
            format!("class:{}", phase.class.as_str()),
        ]);
    }
    super::model::content_ref(lines.join("\n").as_bytes())
}

pub fn validate_suite_instance(
    profile: &super::model::PerformanceProfile,
    suite: &super::model::BenchmarkSuite,
) -> super::model::PerformanceResult<()> {
    let template = match suite.lane {
        super::model::BenchmarkLane::Fast => &profile.fast,
        super::model::BenchmarkLane::Deep => &profile.deep,
    };
    let mut blockers = Vec::new();
    if suite.suite_id != template.suite_id
        || suite.measurement != template.measurement
        || suite.pin_to_single_core != template.pin_to_single_core
        || suite.host_class_ref != template.host_class_ref
        || suite.resource_envelope_ref != template.resource_envelope_ref
        || suite.engine_cohort_ref != template.engine_cohort_ref
        || suite.phases != template.phases
        || suite.sampling != template.sampling
    {
        blockers.push("benchmark suite instance changes its reviewed lane configuration".to_string());
    }
    if !super::model::valid_ref_collection(&suite.materialization_bundle_refs)
        || !super::model::valid_ref_collection(&suite.workload_refs)
        || !super::model::valid_content_ref(&suite.engine_artifact_ref)
        || !super::model::valid_content_ref(&suite.runner_artifact_ref)
    {
        blockers.push(
            "benchmark suite instance requires sorted exact bundle, workload, engine, and runner refs".to_string(),
        );
    }
    if blockers.is_empty() {
        Ok(())
    } else {
        Err(super::model::PerformanceDenial::from_blockers(blockers))
    }
}

fn validate_run_inputs(input: &BenchmarkRunInput<'_>) -> super::model::PerformanceResult<()> {
    let mut blockers = Vec::new();
    if !input
        .suite
        .materialization_bundle_refs
        .iter()
        .any(|value| value == &input.materialized.mantle_bundle_ref)
    {
        blockers.push("benchmark run materialization is not named by the exact suite".to_string());
    }
    if input.host.target != input.materialized.target
        || input.host.cpu_features != input.materialized.cpu_features
        || input.host.host_class_ref != input.suite.host_class_ref
        || input.host.measurement != input.suite.measurement
    {
        blockers.push("benchmark host facts differ from the admitted artifact or suite".to_string());
    }
    if !super::model::valid_content_ref(&input.benchmark_ref)
        || !input.suite.workload_refs.iter().any(|value| value == &input.benchmark_ref)
    {
        blockers.push("benchmark workload identity is malformed or absent from the exact suite".to_string());
    }
    if input.recorded_effect_refs.len() > MAX_RECORDED_EFFECT_REFS
        || !super::model::valid_ref_collection(&input.recorded_effect_refs)
    {
        blockers.push("benchmark run requires bounded sorted recorded-effect refs".to_string());
    }
    if blockers.is_empty() {
        Ok(())
    } else {
        Err(super::model::PerformanceDenial::from_blockers(blockers))
    }
}

fn normalize_phase_groups(
    profile: &super::model::PerformanceProfile,
    suite: &super::model::BenchmarkSuite,
    mut phases: Vec<super::model::PhaseSamples>,
) -> super::model::PerformanceResult<Vec<super::model::PhaseSamples>> {
    for phase in &mut phases {
        phase.samples.sort_by_key(|sample| (sample.process, sample.iteration));
    }
    phases.sort_by(|left, right| (left.phase, &left.event).cmp(&(right.phase, &right.event)));
    let mut blockers = Vec::new();
    validate_normalized_phases(&phases, &mut blockers);
    let expected_samples = suite.sampling.expected_samples_per_phase()?;
    // Each phase adds at most a sample-count and a sample-value blocker.
    const BLOCKERS_PER_PHASE: usize = 2;
    blockers.reserve(phases.len().saturating_mul(BLOCKERS_PER_PHASE));
    for phase in &phases {
        let sample_count = u32::try_from(phase.samples.len()).map_err(|error| {
            super::model::PerformanceDenial::new(format!("benchmark phase sample count is unsupported: {error}"))
        })?;
        if sample_count < suite.sampling.min_samples_per_phase
            || sample_count > suite.sampling.max_samples_per_phase
            || sample_count != expected_samples
        {
            blockers.push(format!("benchmark {} phase sample count differs from the suite", phase.phase.as_str()));
        }
        if phase
            .samples
            .iter()
            .any(|sample| sample.count == 0 || sample.count > profile.comparison.max_sample_value)
        {
            blockers.push(format!("benchmark {} phase contains a zero or over-bound sample", phase.phase.as_str()));
        }
    }
    blockers.extend(
        super::model::PerformancePhase::ALL
            .into_iter()
            .filter(|required| !phases.iter().any(|phase| phase.phase == *required))
            .map(|required| format!("benchmark run omits the {} phase", required.as_str())),
    );
    if blockers.is_empty() {
        Ok(phases)
    } else {
        Err(super::model::PerformanceDenial::from_blockers(blockers))
    }
}

use super::model::BenchmarkComparison;
use super::model::BenchmarkHostFacts;
use super::model::BenchmarkLane;
use super::model::BenchmarkRun;
use super::model::BenchmarkSuite;
use super::model::ComparisonDecision;
use super::model::MaterializedPerformanceArtifact;
use super::model::PerformanceDenial;
use super::model::PerformancePhase;
use super::model::PerformanceProfile;
use super::model::PerformanceResult;
use super::model::PerformanceSample;
use super::model::PhaseComparison;
use super::model::PhaseSamples;
use super::model::RegressionClass;
use super::model::content_ref;
use super::model::sorted_unique;
use super::model::valid_content_ref;
use super::model::valid_ref_collection;
use super::profile::performance_profile_ref;
use super::profile::performance_suite_ref;
use super::profile::validate_performance_profile;

const MIN_STATISTICAL_SAMPLES: usize = 2;
const NORMAL_95_MULTIPLIER_MILLI: u128 = 1_960;
const MILLI_SCALE: u128 = 1_000;
const BINARY_SEARCH_DIVISOR: u128 = 2;
const _: () = assert!(BINARY_SEARCH_DIVISOR > 0);
const MAX_RECORDED_EFFECT_REFS: usize = 128;

#[derive(Debug, Clone)]
pub struct BenchmarkRunInput<'a> {
    pub profile: &'a PerformanceProfile,
    pub suite: &'a BenchmarkSuite,
    pub materialized: &'a MaterializedPerformanceArtifact,
    pub host: &'a BenchmarkHostFacts,
    pub benchmark_ref: String,
    pub recorded_effect_refs: Vec<String>,
    pub phases: Vec<PhaseSamples>,
}

pub fn build_benchmark_run(input: BenchmarkRunInput<'_>) -> PerformanceResult<BenchmarkRun> {
    validate_performance_profile(input.profile)?;
    validate_suite_instance(input.profile, input.suite)?;
    validate_run_inputs(&input)?;
    let phases = normalize_phase_groups(input.profile, input.suite, input.phases)?;
    let mut run = BenchmarkRun {
        suite_ref: performance_suite_ref(input.suite),
        run_ref: String::new(),
        benchmark_ref: input.benchmark_ref,
        consumer: input.materialized.consumer,
        source_component_ref: input.materialized.source_component_ref.clone(),
        component_ref: input.materialized.artifact_ref.clone(),
        component_profile_ref: input.materialized.component_profile_ref.clone(),
        performance_profile_ref: performance_profile_ref(input.profile),
        engine_cohort_ref: input.suite.engine_cohort_ref.clone(),
        engine_artifact_ref: input.suite.engine_artifact_ref.clone(),
        runner_artifact_ref: input.suite.runner_artifact_ref.clone(),
        runtime_configuration_ref: input.materialized.runtime_configuration_ref.clone(),
        target: input.host.target.clone(),
        host_class_ref: input.host.host_class_ref.clone(),
        measurement: input.host.measurement.clone(),
        resource_envelope_ref: input.suite.resource_envelope_ref.clone(),
        recorded_effect_refs: sorted_unique(&input.recorded_effect_refs),
        phases,
    };
    run.run_ref = benchmark_run_ref(&run);
    Ok(run)
}

pub fn validate_benchmark_run(run: &BenchmarkRun) -> PerformanceResult<()> {
    let mut blockers = Vec::new();
    for (label, value) in [
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
    ] {
        if !valid_content_ref(value) {
            blockers.push(format!("benchmark run {label} ref is malformed"));
        }
    }
    if run.target.trim().is_empty() || run.measurement.trim().is_empty() {
        blockers.push("benchmark run target or measurement is empty".to_string());
    }
    if run.recorded_effect_refs.len() > MAX_RECORDED_EFFECT_REFS || !valid_ref_collection(&run.recorded_effect_refs) {
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
        Err(PerformanceDenial::from_blockers(blockers))
    }
}

pub fn compare_benchmark_runs(
    profile: &PerformanceProfile,
    baseline: &BenchmarkRun,
    candidate: &BenchmarkRun,
) -> PerformanceResult<ComparisonDecision> {
    validate_performance_profile(profile)?;
    validate_benchmark_run(baseline)?;
    validate_benchmark_run(candidate)?;
    let blockers = compatibility_blockers(baseline, candidate);
    if !blockers.is_empty() {
        return Ok(ComparisonDecision::Incompatible { blockers });
    }
    let mut phases = Vec::with_capacity(baseline.phases.len());
    for (baseline_phase, candidate_phase) in baseline.phases.iter().zip(&candidate.phases) {
        phases.push(compare_phase(profile, baseline_phase, candidate_phase)?);
    }
    let mut comparison = BenchmarkComparison {
        baseline_run_ref: baseline.run_ref.clone(),
        candidate_run_ref: candidate.run_ref.clone(),
        suite_ref: baseline.suite_ref.clone(),
        phases,
        comparison_ref: String::new(),
    };
    comparison.comparison_ref = benchmark_comparison_ref(&comparison);
    Ok(ComparisonDecision::Comparable(comparison))
}

pub fn benchmark_run_ref(run: &BenchmarkRun) -> String {
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
    content_ref(lines.join("\n").as_bytes())
}

pub fn benchmark_comparison_ref(comparison: &BenchmarkComparison) -> String {
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
    content_ref(lines.join("\n").as_bytes())
}

pub fn validate_suite_instance(profile: &PerformanceProfile, suite: &BenchmarkSuite) -> PerformanceResult<()> {
    let template = match suite.lane {
        BenchmarkLane::Fast => &profile.fast,
        BenchmarkLane::Deep => &profile.deep,
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
    if !valid_ref_collection(&suite.materialization_bundle_refs)
        || !valid_ref_collection(&suite.workload_refs)
        || !valid_content_ref(&suite.engine_artifact_ref)
        || !valid_content_ref(&suite.runner_artifact_ref)
    {
        blockers.push(
            "benchmark suite instance requires sorted exact bundle, workload, engine, and runner refs".to_string(),
        );
    }
    if blockers.is_empty() {
        Ok(())
    } else {
        Err(PerformanceDenial::from_blockers(blockers))
    }
}

fn validate_run_inputs(input: &BenchmarkRunInput<'_>) -> PerformanceResult<()> {
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
    if !valid_content_ref(&input.benchmark_ref)
        || !input.suite.workload_refs.iter().any(|value| value == &input.benchmark_ref)
    {
        blockers.push("benchmark workload identity is malformed or absent from the exact suite".to_string());
    }
    if input.recorded_effect_refs.len() > MAX_RECORDED_EFFECT_REFS || !valid_ref_collection(&input.recorded_effect_refs)
    {
        blockers.push("benchmark run requires bounded sorted recorded-effect refs".to_string());
    }
    if blockers.is_empty() {
        Ok(())
    } else {
        Err(PerformanceDenial::from_blockers(blockers))
    }
}

fn normalize_phase_groups(
    profile: &PerformanceProfile,
    suite: &BenchmarkSuite,
    mut phases: Vec<PhaseSamples>,
) -> PerformanceResult<Vec<PhaseSamples>> {
    for phase in &mut phases {
        phase.samples.sort_by_key(|sample| (sample.process, sample.iteration));
    }
    phases.sort_by(|left, right| (left.phase, &left.event).cmp(&(right.phase, &right.event)));
    let mut blockers = Vec::new();
    validate_normalized_phases(&phases, &mut blockers);
    let expected_samples = suite.sampling.expected_samples_per_phase()?;
    for phase in &phases {
        let sample_count = u32::try_from(phase.samples.len())
            .map_err(|error| PerformanceDenial::new(format!("benchmark phase sample count is unsupported: {error}")))?;
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
    for required in PerformancePhase::ALL {
        if !phases.iter().any(|phase| phase.phase == required) {
            blockers.push(format!("benchmark run omits the {} phase", required.as_str()));
        }
    }
    if blockers.is_empty() {
        Ok(phases)
    } else {
        Err(PerformanceDenial::from_blockers(blockers))
    }
}

fn validate_normalized_phases(phases: &[PhaseSamples], blockers: &mut Vec<String>) {
    if phases.is_empty() {
        blockers.push("benchmark run has no phase samples".to_string());
        return;
    }
    let mut previous_key = None;
    for phase in phases {
        if phase.event.trim().is_empty() || phase.samples.len() < MIN_STATISTICAL_SAMPLES {
            blockers.push(format!("benchmark {} phase has no event or too few samples", phase.phase.as_str()));
        }
        let key = (phase.phase, phase.event.as_str());
        if previous_key.is_some_and(|previous| previous >= key) {
            blockers.push("benchmark phase/event groups must be sorted and unique".to_string());
        }
        previous_key = Some(key);
        let mut previous_sample = None;
        for sample in &phase.samples {
            let sample_key = (sample.process, sample.iteration);
            if previous_sample.is_some_and(|previous| previous >= sample_key) {
                blockers
                    .push(format!("benchmark {} samples must be sorted with unique coordinates", phase.phase.as_str()));
            }
            previous_sample = Some(sample_key);
        }
    }
}

fn compatibility_blockers(baseline: &BenchmarkRun, candidate: &BenchmarkRun) -> Vec<String> {
    let mut blockers = Vec::new();
    for (label, left, right) in [
        ("suite", baseline.suite_ref.as_str(), candidate.suite_ref.as_str()),
        ("benchmark", baseline.benchmark_ref.as_str(), candidate.benchmark_ref.as_str()),
        ("source component", baseline.source_component_ref.as_str(), candidate.source_component_ref.as_str()),
        (
            "component profile",
            baseline.component_profile_ref.as_str(),
            candidate.component_profile_ref.as_str(),
        ),
        (
            "performance profile",
            baseline.performance_profile_ref.as_str(),
            candidate.performance_profile_ref.as_str(),
        ),
        ("engine cohort", baseline.engine_cohort_ref.as_str(), candidate.engine_cohort_ref.as_str()),
        ("engine artifact", baseline.engine_artifact_ref.as_str(), candidate.engine_artifact_ref.as_str()),
        ("runner artifact", baseline.runner_artifact_ref.as_str(), candidate.runner_artifact_ref.as_str()),
        (
            "runtime configuration",
            baseline.runtime_configuration_ref.as_str(),
            candidate.runtime_configuration_ref.as_str(),
        ),
        ("target", baseline.target.as_str(), candidate.target.as_str()),
        ("host class", baseline.host_class_ref.as_str(), candidate.host_class_ref.as_str()),
        ("measurement", baseline.measurement.as_str(), candidate.measurement.as_str()),
        (
            "resource envelope",
            baseline.resource_envelope_ref.as_str(),
            candidate.resource_envelope_ref.as_str(),
        ),
    ] {
        if left != right {
            blockers.push(format!("benchmark runs have incompatible {label}"));
        }
    }
    if baseline.consumer != candidate.consumer {
        blockers.push("benchmark runs have incompatible component consumers".to_string());
    }
    if baseline.recorded_effect_refs != candidate.recorded_effect_refs {
        blockers.push("benchmark runs have incompatible recorded effects".to_string());
    }
    let baseline_keys = baseline.phases.iter().map(|phase| (phase.phase, &phase.event)).collect::<Vec<_>>();
    let candidate_keys = candidate.phases.iter().map(|phase| (phase.phase, &phase.event)).collect::<Vec<_>>();
    if baseline_keys != candidate_keys {
        blockers.push("benchmark runs have incompatible phase/event groups".to_string());
    }
    blockers.sort();
    blockers.dedup();
    blockers
}

fn compare_phase(
    profile: &PerformanceProfile,
    baseline: &PhaseSamples,
    candidate: &PhaseSamples,
) -> PerformanceResult<PhaseComparison> {
    let scale = u128::from(profile.comparison.parts_per_million);
    let baseline_stats = summarize_samples(&baseline.samples, scale)?;
    let candidate_stats = summarize_samples(&candidate.samples, scale)?;
    if baseline_stats.mean_scaled == 0 {
        return Err(PerformanceDenial::new("benchmark baseline mean cannot be zero"));
    }
    let ratio_ppm = candidate_stats
        .mean_scaled
        .checked_mul(scale)
        .and_then(|value| value.checked_div(baseline_stats.mean_scaled))
        .ok_or_else(|| PerformanceDenial::new("benchmark effect-size ratio overflowed"))?;
    let combined_confidence = baseline_stats
        .confidence_half_width_scaled
        .checked_add(candidate_stats.confidence_half_width_scaled)
        .ok_or_else(|| PerformanceDenial::new("benchmark confidence interval overflowed"))?;
    let ratio_confidence_ppm = combined_confidence
        .checked_mul(scale)
        .and_then(|value| value.checked_div(baseline_stats.mean_scaled))
        .ok_or_else(|| PerformanceDenial::new("benchmark ratio confidence interval overflowed"))?;
    let practical_delta = baseline_stats
        .mean_scaled
        .checked_mul(u128::from(profile.comparison.practical_threshold_ppm))
        .and_then(|value| value.checked_div(scale))
        .ok_or_else(|| PerformanceDenial::new("benchmark practical threshold overflowed"))?;
    let baseline_lower = baseline_stats.mean_scaled.saturating_sub(baseline_stats.confidence_half_width_scaled);
    let baseline_upper = baseline_stats
        .mean_scaled
        .checked_add(baseline_stats.confidence_half_width_scaled)
        .ok_or_else(|| PerformanceDenial::new("benchmark baseline confidence upper bound overflowed"))?;
    let candidate_lower = candidate_stats.mean_scaled.saturating_sub(candidate_stats.confidence_half_width_scaled);
    let candidate_upper = candidate_stats
        .mean_scaled
        .checked_add(candidate_stats.confidence_half_width_scaled)
        .ok_or_else(|| PerformanceDenial::new("benchmark candidate confidence upper bound overflowed"))?;
    let class = if candidate_upper
        .checked_add(practical_delta)
        .is_some_and(|candidate_with_threshold| candidate_with_threshold < baseline_lower)
    {
        RegressionClass::Improvement
    } else if baseline_upper
        .checked_add(practical_delta)
        .is_some_and(|baseline_with_threshold| baseline_with_threshold < candidate_lower)
    {
        RegressionClass::Regression
    } else {
        RegressionClass::NoSignificantChange
    };
    Ok(PhaseComparison {
        phase: baseline.phase,
        event: baseline.event.clone(),
        baseline_mean_scaled: baseline_stats.mean_scaled,
        candidate_mean_scaled: candidate_stats.mean_scaled,
        baseline_confidence_half_width_scaled: baseline_stats.confidence_half_width_scaled,
        candidate_confidence_half_width_scaled: candidate_stats.confidence_half_width_scaled,
        candidate_ratio_ppm: u64::try_from(ratio_ppm)
            .map_err(|error| PerformanceDenial::new(format!("benchmark ratio is unsupported: {error}")))?,
        ratio_confidence_half_width_ppm: u64::try_from(ratio_confidence_ppm)
            .map_err(|error| PerformanceDenial::new(format!("benchmark ratio confidence is unsupported: {error}")))?,
        class,
    })
}

struct SampleSummary {
    mean_scaled: u128,
    confidence_half_width_scaled: u128,
}

fn summarize_samples(samples: &[PerformanceSample], scale: u128) -> PerformanceResult<SampleSummary> {
    let count = u128::try_from(samples.len())
        .map_err(|error| PerformanceDenial::new(format!("benchmark sample count is unsupported: {error}")))?;
    if samples.len() < MIN_STATISTICAL_SAMPLES {
        return Err(PerformanceDenial::new("benchmark comparison requires at least two samples"));
    }
    let sum = samples.iter().try_fold(0_u128, |total, sample| {
        total
            .checked_add(u128::from(sample.count))
            .ok_or_else(|| PerformanceDenial::new("benchmark sample sum overflowed"))
    })?;
    let mean_scaled = sum
        .checked_mul(scale)
        .and_then(|value| value.checked_div(count))
        .ok_or_else(|| PerformanceDenial::new("benchmark scaled mean overflowed"))?;
    let squared_deviation_sum = samples.iter().try_fold(0_u128, |total, sample| {
        let scaled_sample = u128::from(sample.count)
            .checked_mul(scale)
            .ok_or_else(|| PerformanceDenial::new("benchmark scaled sample overflowed"))?;
        let deviation = scaled_sample.abs_diff(mean_scaled);
        let squared = deviation
            .checked_mul(deviation)
            .ok_or_else(|| PerformanceDenial::new("benchmark squared deviation overflowed"))?;
        total
            .checked_add(squared)
            .ok_or_else(|| PerformanceDenial::new("benchmark deviation sum overflowed"))
    })?;
    let sample_variance = squared_deviation_sum
        .checked_div(count - 1)
        .ok_or_else(|| PerformanceDenial::new("benchmark sample variance is undefined"))?;
    let variance_of_mean = sample_variance
        .checked_div(count)
        .ok_or_else(|| PerformanceDenial::new("benchmark mean variance is undefined"))?;
    let standard_error = integer_sqrt(variance_of_mean);
    let confidence_half_width_scaled = standard_error
        .checked_mul(NORMAL_95_MULTIPLIER_MILLI)
        .and_then(|value| value.checked_div(MILLI_SCALE))
        .ok_or_else(|| PerformanceDenial::new("benchmark confidence interval overflowed"))?;
    Ok(SampleSummary {
        mean_scaled,
        confidence_half_width_scaled,
    })
}

fn integer_sqrt(value: u128) -> u128 {
    if value <= 1 {
        return value;
    }
    let mut low = 1_u128;
    let mut high = value;
    while low < high {
        let half_range = (high - low)
            .checked_div(BINARY_SEARCH_DIVISOR)
            .expect("binary-search divisor is a nonzero constant");
        let midpoint = low + half_range;
        if midpoint > value / midpoint {
            high = midpoint;
        } else {
            let next = midpoint + 1;
            if next > value / next {
                return midpoint;
            }
            low = next;
        }
    }
    low
}

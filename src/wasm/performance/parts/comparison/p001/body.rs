
fn validate_normalized_phases(
    phases: &[super::model::PhaseSamples],
    blockers: &mut impl crate::bounded::VecSink<String>,
) {
    if phases.is_empty() {
        blockers.push_item("benchmark run has no phase samples".to_string());
        return;
    }
    let mut previous_key = None;
    for phase in phases {
        if phase.event.trim().is_empty() || phase.samples.len() < MIN_STATISTICAL_SAMPLES {
            blockers.push_item(format!("benchmark {} phase has no event or too few samples", phase.phase.as_str()));
        }
        let key = (phase.phase, phase.event.as_str());
        if previous_key.is_some_and(|previous| previous >= key) {
            blockers.push_item("benchmark phase/event groups must be sorted and unique".to_string());
        }
        previous_key = Some(key);
        let mut previous_sample = None;
        for sample in &phase.samples {
            let sample_key = (sample.process, sample.iteration);
            if previous_sample.is_some_and(|previous| previous >= sample_key) {
                blockers.push_item(format!(
                    "benchmark {} samples must be sorted with unique coordinates",
                    phase.phase.as_str()
                ));
            }
            previous_sample = Some(sample_key);
        }
    }
}

fn compatibility_blockers(
    baseline: &super::model::BenchmarkRun,
    candidate: &super::model::BenchmarkRun,
) -> Vec<String> {
    let mut blockers = [
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
    ]
    .into_iter()
    .filter(|(_, left, right)| left != right)
    .map(|(label, ..)| format!("benchmark runs have incompatible {label}"))
    .collect::<Vec<_>>();
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
    profile: &super::model::PerformanceProfile,
    baseline: &super::model::PhaseSamples,
    candidate: &super::model::PhaseSamples,
) -> super::model::PerformanceResult<super::model::PhaseComparison> {
    let scale = u128::from(profile.comparison.parts_per_million);
    let baseline_stats = summarize_samples(&baseline.samples, scale)?;
    let candidate_stats = summarize_samples(&candidate.samples, scale)?;
    if baseline_stats.mean_scaled == 0 {
        return Err(super::model::PerformanceDenial::new("benchmark baseline mean cannot be zero"));
    }
    let ratio_ppm = candidate_stats
        .mean_scaled
        .checked_mul(scale)
        .and_then(|value| value.checked_div(baseline_stats.mean_scaled))
        .ok_or_else(|| super::model::PerformanceDenial::new("benchmark effect-size ratio overflowed"))?;
    let combined_confidence = baseline_stats
        .confidence_half_width_scaled
        .checked_add(candidate_stats.confidence_half_width_scaled)
        .ok_or_else(|| super::model::PerformanceDenial::new("benchmark confidence interval overflowed"))?;
    let ratio_confidence_ppm = combined_confidence
        .checked_mul(scale)
        .and_then(|value| value.checked_div(baseline_stats.mean_scaled))
        .ok_or_else(|| super::model::PerformanceDenial::new("benchmark ratio confidence interval overflowed"))?;
    let practical_delta = baseline_stats
        .mean_scaled
        .checked_mul(u128::from(profile.comparison.practical_threshold_ppm))
        .and_then(|value| value.checked_div(scale))
        .ok_or_else(|| super::model::PerformanceDenial::new("benchmark practical threshold overflowed"))?;
    let baseline_lower = baseline_stats.mean_scaled.saturating_sub(baseline_stats.confidence_half_width_scaled);
    let baseline_upper = baseline_stats
        .mean_scaled
        .checked_add(baseline_stats.confidence_half_width_scaled)
        .ok_or_else(|| super::model::PerformanceDenial::new("benchmark baseline confidence upper bound overflowed"))?;
    let candidate_lower = candidate_stats.mean_scaled.saturating_sub(candidate_stats.confidence_half_width_scaled);
    let candidate_upper = candidate_stats
        .mean_scaled
        .checked_add(candidate_stats.confidence_half_width_scaled)
        .ok_or_else(|| super::model::PerformanceDenial::new("benchmark candidate confidence upper bound overflowed"))?;
    let class = if candidate_upper
        .checked_add(practical_delta)
        .is_some_and(|candidate_with_threshold| candidate_with_threshold < baseline_lower)
    {
        super::model::RegressionClass::Improvement
    } else if baseline_upper
        .checked_add(practical_delta)
        .is_some_and(|baseline_with_threshold| baseline_with_threshold < candidate_lower)
    {
        super::model::RegressionClass::Regression
    } else {
        super::model::RegressionClass::NoSignificantChange
    };
    Ok(super::model::PhaseComparison {
        phase: baseline.phase,
        event: baseline.event.clone(),
        baseline_mean_scaled: baseline_stats.mean_scaled,
        candidate_mean_scaled: candidate_stats.mean_scaled,
        baseline_confidence_half_width_scaled: baseline_stats.confidence_half_width_scaled,
        candidate_confidence_half_width_scaled: candidate_stats.confidence_half_width_scaled,
        candidate_ratio_ppm: u64::try_from(ratio_ppm).map_err(|error| {
            super::model::PerformanceDenial::new(format!("benchmark ratio is unsupported: {error}"))
        })?,
        ratio_confidence_half_width_ppm: u64::try_from(ratio_confidence_ppm).map_err(|error| {
            super::model::PerformanceDenial::new(format!("benchmark ratio confidence is unsupported: {error}"))
        })?,
        class,
    })
}

struct SampleSummary {
    mean_scaled: u128,
    confidence_half_width_scaled: u128,
}

fn summarize_samples(
    samples: &[super::model::PerformanceSample],
    scale: u128,
) -> super::model::PerformanceResult<SampleSummary> {
    let count = u128::try_from(samples.len()).map_err(|error| {
        super::model::PerformanceDenial::new(format!("benchmark sample count is unsupported: {error}"))
    })?;
    if samples.len() < MIN_STATISTICAL_SAMPLES {
        return Err(super::model::PerformanceDenial::new("benchmark comparison requires at least two samples"));
    }
    let sum = samples.iter().try_fold(0_u128, |total, sample| {
        total
            .checked_add(u128::from(sample.count))
            .ok_or_else(|| super::model::PerformanceDenial::new("benchmark sample sum overflowed"))
    })?;
    let mean_scaled = sum
        .checked_mul(scale)
        .and_then(|value| value.checked_div(count))
        .ok_or_else(|| super::model::PerformanceDenial::new("benchmark scaled mean overflowed"))?;
    let squared_deviation_sum = samples.iter().try_fold(0_u128, |total, sample| {
        let scaled_sample = u128::from(sample.count)
            .checked_mul(scale)
            .ok_or_else(|| super::model::PerformanceDenial::new("benchmark scaled sample overflowed"))?;
        let deviation = scaled_sample.abs_diff(mean_scaled);
        let squared = deviation
            .checked_mul(deviation)
            .ok_or_else(|| super::model::PerformanceDenial::new("benchmark squared deviation overflowed"))?;
        total
            .checked_add(squared)
            .ok_or_else(|| super::model::PerformanceDenial::new("benchmark deviation sum overflowed"))
    })?;
    let sample_variance = squared_deviation_sum
        .checked_div(count - 1)
        .ok_or_else(|| super::model::PerformanceDenial::new("benchmark sample variance is undefined"))?;
    let variance_of_mean = sample_variance
        .checked_div(count)
        .ok_or_else(|| super::model::PerformanceDenial::new("benchmark mean variance is undefined"))?;
    let standard_error = integer_sqrt(variance_of_mean);
    let confidence_half_width_scaled = standard_error
        .checked_mul(NORMAL_95_MULTIPLIER_MILLI)
        .and_then(|value| value.checked_div(MILLI_SCALE))
        .ok_or_else(|| super::model::PerformanceDenial::new("benchmark confidence interval overflowed"))?;
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
        let half_range = (high - low) >> BINARY_SEARCH_HALVING_SHIFT;
        let midpoint = low + half_range;
        let Some(midpoint_quotient) = value.checked_div(midpoint) else {
            high = midpoint;
            continue;
        };
        if midpoint > midpoint_quotient {
            high = midpoint;
        } else {
            let next = midpoint + 1;
            let Some(next_quotient) = value.checked_div(next) else {
                return midpoint;
            };
            if next > next_quotient {
                return midpoint;
            }
            low = next;
        }
    }
    low
}

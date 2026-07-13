use std::collections::BTreeMap;
use std::collections::BTreeSet;

use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
struct SeriesAccumulator {
    descriptor_id: String,
    metric_name: String,
    unit: String,
    kind: MetricKind,
    aggregation: MetricAggregation,
    value: i64,
    source_sample_refs: Vec<String>,
    latest_observed_tick: u64,
    latest_sample_ref: String,
}

// r[impl molten.fabric_observability.pure_core]
pub fn aggregate_metric_samples(
    profile: &ObservationProfile,
    descriptors: &[MetricDescriptor],
    samples: &[MetricSample],
    as_of_tick: u64,
) -> Result<Vec<AggregatedSeries>, Vec<ObservabilityIssue>> {
    let mut issues = validate_observation_profile(profile);
    if descriptors.len() > profile.bounds.max_descriptors {
        issues.push(ObservabilityIssue::CollectionLimitExceeded("metric-descriptors"));
    }
    let descriptor_index = index_descriptors(profile, descriptors, &mut issues);
    let mut series = BTreeMap::<SeriesIdentity, SeriesAccumulator>::new();
    let mut sample_refs = BTreeSet::new();
    for sample in samples {
        if !sample_refs.insert(sample.sample_ref.clone()) {
            issues.push(ObservabilityIssue::DuplicateValue("metric-sample-ref"));
            continue;
        }
        let Some(descriptor) = descriptor_index.get(&sample.descriptor_ref) else {
            issues.push(ObservabilityIssue::DescriptorMissing(sample.descriptor_ref.clone()));
            continue;
        };
        match validate_metric_sample(profile, descriptor, sample, as_of_tick) {
            Ok(sanitized) => accumulate_sample(profile, descriptor, &sanitized, &mut series, &mut issues),
            Err(mut sample_issues) => issues.append(&mut sample_issues),
        }
    }
    if !issues.is_empty() {
        return Err(issues);
    }
    Ok(series
        .into_iter()
        .map(|(identity, accumulator)| AggregatedSeries {
            identity,
            descriptor_id: accumulator.descriptor_id,
            metric_name: accumulator.metric_name,
            unit: accumulator.unit,
            kind: accumulator.kind,
            aggregation: accumulator.aggregation,
            value: accumulator.value,
            source_sample_refs: accumulator.source_sample_refs,
            latest_observed_tick: accumulator.latest_observed_tick,
        })
        .collect())
}

fn index_descriptors<'a>(
    profile: &ObservationProfile,
    descriptors: &'a [MetricDescriptor],
    issues: &mut Vec<ObservabilityIssue>,
) -> BTreeMap<String, &'a MetricDescriptor> {
    let mut index = BTreeMap::new();
    let mut ids = BTreeSet::new();
    for descriptor in descriptors {
        issues.extend(validate_metric_descriptor(profile, descriptor));
        if index.insert(descriptor.descriptor_ref.clone(), descriptor).is_some() {
            issues.push(ObservabilityIssue::DuplicateValue("metric-descriptor-ref"));
        }
        if !ids.insert(descriptor.descriptor_id.clone()) {
            issues.push(ObservabilityIssue::DuplicateValue("metric-descriptor-id"));
        }
    }
    index
}

fn accumulate_sample(
    profile: &ObservationProfile,
    descriptor: &MetricDescriptor,
    sample: &MetricSample,
    series: &mut BTreeMap<SeriesIdentity, SeriesAccumulator>,
    issues: &mut Vec<ObservabilityIssue>,
) {
    let identity = SeriesIdentity {
        descriptor_ref: descriptor.descriptor_ref.clone(),
        labels: sample.labels.clone(),
    };
    if !series.contains_key(&identity) && series.len() >= profile.bounds.max_series {
        issues.push(ObservabilityIssue::CollectionLimitExceeded("metric-series"));
        return;
    }
    match series.get_mut(&identity) {
        Some(accumulator) => update_accumulator(accumulator, sample, issues),
        None => {
            series.insert(identity, SeriesAccumulator {
                descriptor_id: descriptor.descriptor_id.clone(),
                metric_name: descriptor.name.clone(),
                unit: descriptor.unit.clone(),
                kind: descriptor.kind,
                aggregation: descriptor.aggregation,
                value: sample.value,
                source_sample_refs: vec![sample.sample_ref.clone()],
                latest_observed_tick: sample.context.observed_tick,
                latest_sample_ref: sample.sample_ref.clone(),
            });
        }
    }
}

fn update_accumulator(
    accumulator: &mut SeriesAccumulator,
    sample: &MetricSample,
    issues: &mut Vec<ObservabilityIssue>,
) {
    match accumulator.aggregation {
        MetricAggregation::Sum => match accumulator.value.checked_add(sample.value) {
            Some(value) => accumulator.value = value,
            None => issues.push(ObservabilityIssue::ArithmeticOverflow),
        },
        MetricAggregation::Last => {
            let sample_order = (sample.context.observed_tick, sample.sample_ref.as_str());
            let current_order = (accumulator.latest_observed_tick, accumulator.latest_sample_ref.as_str());
            if sample_order > current_order {
                accumulator.value = sample.value;
                accumulator.latest_observed_tick = sample.context.observed_tick;
                accumulator.latest_sample_ref.clone_from(&sample.sample_ref);
            }
        }
        MetricAggregation::Minimum => accumulator.value = accumulator.value.min(sample.value),
        MetricAggregation::Maximum => accumulator.value = accumulator.value.max(sample.value),
    }
    accumulator.source_sample_refs.push(sample.sample_ref.clone());
    accumulator.source_sample_refs.sort();
    accumulator.source_sample_refs.dedup();
    accumulator.latest_observed_tick = accumulator.latest_observed_tick.max(sample.context.observed_tick);
}

use std::collections::BTreeSet;

use super::*;
use crate::fabric::valid_blake3_ref;
use crate::fabric::valid_fabric_token;

const SECRET_PREFIX: &str = "secret:";
const TOKEN_PREFIX: &str = "token:";
const TICKET_PREFIX: &str = "iroh-ticket:";
const PRIVATE_KEY_MARKER: &str = "-----begin private key";
const UNIX_ABSOLUTE_PATH_PREFIX: &str = "/";

// r[impl molten.fabric_observability.model]
// r[impl molten.fabric_observability.bounds_redaction]
pub fn validate_observation_profile(profile: &ObservationProfile) -> Vec<ObservabilityIssue> {
    let mut issues = Vec::new();
    validate_schema("observation-profile-schema", &profile.schema, OBSERVATION_PROFILE_SCHEMA, &mut issues);
    validate_token("observation-profile-id", &profile.profile_id, &mut issues);
    validate_ref("observation-profile-ref", &profile.profile_ref, &mut issues);
    validate_bounds(&profile.bounds, &mut issues);
    validate_redaction_rules(profile, &mut issues);
    validate_non_claims(&profile.non_claims, &mut issues);
    issues
}

pub fn validate_metric_descriptor(
    profile: &ObservationProfile,
    descriptor: &MetricDescriptor,
) -> Vec<ObservabilityIssue> {
    let mut issues = validate_observation_profile(profile);
    validate_schema("metric-descriptor-schema", &descriptor.schema, METRIC_DESCRIPTOR_SCHEMA, &mut issues);
    validate_token("metric-descriptor-id", &descriptor.descriptor_id, &mut issues);
    validate_ref("metric-descriptor-ref", &descriptor.descriptor_ref, &mut issues);
    validate_ref("metric-descriptor-profile-ref", &descriptor.profile_ref, &mut issues);
    if descriptor.profile_ref != profile.profile_ref {
        issues.push(ObservabilityIssue::ProfileMismatch);
    }
    validate_token("metric-name", &descriptor.name, &mut issues);
    validate_token("metric-unit", &descriptor.unit, &mut issues);
    validate_text("metric-description", &descriptor.description, &mut issues);
    if descriptor.allowed_label_names.len() > profile.bounds.max_labels_per_sample {
        issues.push(ObservabilityIssue::CollectionLimitExceeded("metric-allowed-label-names"));
    }
    validate_sorted_unique_tokens("metric-allowed-label-names", &descriptor.allowed_label_names, &mut issues);
    if descriptor.kind == MetricKind::Counter && descriptor.aggregation != MetricAggregation::Sum {
        issues.push(ObservabilityIssue::CounterRequiresSum);
    }
    issues
}

pub fn validate_context(profile: &ObservationProfile, context: &ObservationContext) -> Vec<ObservabilityIssue> {
    let mut issues = Vec::new();
    validate_token("observation-source-id", &context.source_id, &mut issues);
    validate_ref("observation-source-ref", &context.source_ref, &mut issues);
    validate_ref("observation-context-profile-ref", &context.profile_ref, &mut issues);
    if context.profile_ref != profile.profile_ref {
        issues.push(ObservabilityIssue::ProfileMismatch);
    }
    if context.generation == 0 {
        issues.push(ObservabilityIssue::ZeroBound("observation-generation"));
    }
    if context.valid_until_tick < context.observed_tick {
        issues.push(ObservabilityIssue::ObservationStale(context.source_id.clone()));
    }
    validate_ref("observation-resource-ref", &context.resource_ref, &mut issues);
    validate_refs("observation-evidence-refs", &context.evidence_refs, &mut issues);
    validate_non_claims(&context.non_claims, &mut issues);
    issues
}

pub fn sanitize_metric_labels(
    profile: &ObservationProfile,
    descriptor: &MetricDescriptor,
    labels: &[MetricLabel],
) -> Result<Vec<MetricLabel>, Vec<ObservabilityIssue>> {
    let mut issues = validate_metric_descriptor(profile, descriptor);
    if labels.len() > profile.bounds.max_labels_per_sample {
        issues.push(ObservabilityIssue::CollectionLimitExceeded("metric-labels"));
    }
    let mut names = BTreeSet::new();
    let mut sanitized = Vec::with_capacity(labels.len());
    for label in labels {
        validate_label(profile, descriptor, label, &mut names, &mut sanitized, &mut issues);
    }
    sanitized.sort();
    if issues.is_empty() { Ok(sanitized) } else { Err(issues) }
}

pub fn validate_metric_sample(
    profile: &ObservationProfile,
    descriptor: &MetricDescriptor,
    sample: &MetricSample,
    as_of_tick: u64,
) -> Result<MetricSample, Vec<ObservabilityIssue>> {
    let mut issues = validate_context(profile, &sample.context);
    validate_schema("metric-sample-schema", &sample.schema, METRIC_SAMPLE_SCHEMA, &mut issues);
    validate_ref("metric-sample-ref", &sample.sample_ref, &mut issues);
    validate_ref("metric-sample-descriptor-ref", &sample.descriptor_ref, &mut issues);
    if sample.descriptor_ref != descriptor.descriptor_ref {
        issues.push(ObservabilityIssue::DescriptorIncompatible);
    }
    if as_of_tick > sample.context.valid_until_tick {
        issues.push(ObservabilityIssue::ObservationStale(sample.context.source_id.clone()));
    }
    let labels = match sanitize_metric_labels(profile, descriptor, &sample.labels) {
        Ok(labels) => labels,
        Err(mut label_issues) => {
            issues.append(&mut label_issues);
            Vec::new()
        }
    };
    if issues.is_empty() {
        let mut sanitized = sample.clone();
        sanitized.labels = labels;
        Ok(sanitized)
    } else {
        Err(issues)
    }
}

pub fn validate_event(
    profile: &ObservationProfile,
    event: &ObservationEvent,
    as_of_tick: u64,
) -> Result<ObservationEvent, Vec<ObservabilityIssue>> {
    let mut issues = validate_context(profile, &event.context);
    validate_schema("observation-event-schema", &event.schema, OBSERVATION_EVENT_SCHEMA, &mut issues);
    validate_ref("observation-event-ref", &event.event_ref, &mut issues);
    validate_token("observation-event-kind", &event.event_kind, &mut issues);
    if event.detail.len() > profile.bounds.max_event_detail_bytes {
        issues.push(ObservabilityIssue::CollectionLimitExceeded("event-detail-bytes"));
    }
    if looks_sensitive(&event.detail) {
        issues.push(ObservabilityIssue::LabelRequiresRedaction("event-detail".to_string()));
    }
    if as_of_tick > event.context.valid_until_tick {
        issues.push(ObservabilityIssue::ObservationStale(event.context.source_id.clone()));
    }
    let event_descriptor = event_attribute_descriptor(profile, event);
    let attributes = match sanitize_metric_labels(profile, &event_descriptor, &event.attributes) {
        Ok(attributes) => attributes,
        Err(mut attribute_issues) => {
            issues.append(&mut attribute_issues);
            Vec::new()
        }
    };
    if issues.is_empty() {
        let mut sanitized = event.clone();
        sanitized.attributes = attributes;
        Ok(sanitized)
    } else {
        Err(issues)
    }
}

pub fn validate_adapter_profile(
    observation_profile: &ObservationProfile,
    adapter: &ObservationAdapterProfile,
) -> Vec<ObservabilityIssue> {
    let mut issues = validate_observation_profile(observation_profile);
    validate_schema(
        "observation-adapter-profile-schema",
        &adapter.schema,
        OBSERVATION_ADAPTER_PROFILE_SCHEMA,
        &mut issues,
    );
    validate_token("observation-adapter-id", &adapter.adapter_id, &mut issues);
    validate_ref("observation-adapter-ref", &adapter.adapter_ref, &mut issues);
    validate_ref("observation-adapter-profile-ref", &adapter.profile_ref, &mut issues);
    if adapter.profile_ref != observation_profile.profile_ref {
        issues.push(ObservabilityIssue::ProfileMismatch);
    }
    if adapter.max_queued_bytes == 0 {
        issues.push(ObservabilityIssue::ZeroBound("adapter-max-queued-bytes"));
    }
    if adapter.max_queued_bytes > observation_profile.bounds.max_queued_bytes {
        issues.push(ObservabilityIssue::QueueBoundExceeded);
    }
    if adapter.timeout_ticks == 0 {
        issues.push(ObservabilityIssue::ZeroBound("adapter-timeout-ticks"));
    }
    validate_refs("observation-adapter-evidence-refs", &adapter.evidence_refs, &mut issues);
    validate_non_claims(&adapter.non_claims, &mut issues);
    issues
}

pub fn validate_snapshot_batch(
    profile: &ObservationProfile,
    snapshots: &[ObservationSnapshot],
    as_of_tick: u64,
) -> Vec<ObservabilityIssue> {
    let mut issues = Vec::new();
    if snapshots.len() > profile.bounds.max_snapshots {
        issues.push(ObservabilityIssue::CollectionLimitExceeded("observation-snapshots"));
    }
    let mut identities = BTreeSet::new();
    let mut prior_tick: Option<u64> = None;
    for snapshot in snapshots {
        issues.extend(validate_snapshot(profile, snapshot, as_of_tick));
        if !identities.insert((snapshot.snapshot_id.clone(), snapshot.generation)) {
            issues.push(ObservabilityIssue::DuplicateValue("observation-snapshot-identity"));
        }
        if let Some(previous_tick) = prior_tick {
            match previous_tick.checked_add(profile.bounds.min_export_interval_ticks) {
                Some(next_tick) if snapshot.as_of_tick < next_tick => {
                    issues.push(ObservabilityIssue::ExportFrequencyExceeded);
                }
                Some(_) => {}
                None => issues.push(ObservabilityIssue::ArithmeticOverflow),
            }
        }
        prior_tick = Some(snapshot.as_of_tick);
    }
    issues
}

pub fn validate_snapshot(
    profile: &ObservationProfile,
    snapshot: &ObservationSnapshot,
    as_of_tick: u64,
) -> Vec<ObservabilityIssue> {
    let mut issues = validate_observation_profile(profile);
    validate_schema("observation-snapshot-schema", &snapshot.schema, OBSERVATION_SNAPSHOT_SCHEMA, &mut issues);
    validate_token("observation-snapshot-id", &snapshot.snapshot_id, &mut issues);
    validate_ref("observation-snapshot-profile-ref", &snapshot.profile_ref, &mut issues);
    if snapshot.profile_ref != profile.profile_ref {
        issues.push(ObservabilityIssue::ProfileMismatch);
    }
    if snapshot.generation == 0 {
        issues.push(ObservabilityIssue::ZeroBound("observation-snapshot-generation"));
    }
    if snapshot.valid_until_tick < snapshot.as_of_tick || as_of_tick > snapshot.valid_until_tick {
        issues.push(ObservabilityIssue::ObservationStale(snapshot.snapshot_id.clone()));
    }
    if snapshot.series.len() > profile.bounds.max_series {
        issues.push(ObservabilityIssue::CollectionLimitExceeded("snapshot-series"));
    }
    validate_snapshot_series(profile, &snapshot.series, &mut issues);
    if snapshot.event_refs.len() > profile.bounds.max_events {
        issues.push(ObservabilityIssue::CollectionLimitExceeded("snapshot-events"));
    }
    for (field, refs) in [
        ("snapshot-event-refs", snapshot.event_refs.as_slice()),
        ("snapshot-health-refs", snapshot.health_refs.as_slice()),
        ("snapshot-integrity-result-refs", snapshot.integrity_result_refs.as_slice()),
        ("snapshot-adapter-outcome-refs", snapshot.adapter_outcome_refs.as_slice()),
        ("snapshot-evidence-refs", snapshot.evidence_refs.as_slice()),
    ] {
        validate_refs(field, refs, &mut issues);
    }
    validate_non_claims(&snapshot.non_claims, &mut issues);
    issues
}

fn validate_snapshot_series(
    profile: &ObservationProfile,
    series: &[AggregatedSeries],
    issues: &mut Vec<ObservabilityIssue>,
) {
    let mut identities = BTreeSet::new();
    for item in series {
        if !identities.insert(item.identity.clone()) {
            issues.push(ObservabilityIssue::DuplicateValue("snapshot-series-identity"));
        }
        validate_ref("snapshot-series-descriptor-ref", &item.identity.descriptor_ref, issues);
        validate_token("snapshot-series-descriptor-id", &item.descriptor_id, issues);
        validate_token("snapshot-series-metric-name", &item.metric_name, issues);
        validate_token("snapshot-series-unit", &item.unit, issues);
        if item.identity.labels.len() > profile.bounds.max_labels_per_sample {
            issues.push(ObservabilityIssue::CollectionLimitExceeded("snapshot-series-labels"));
        }
        for label in &item.identity.labels {
            if !matches!(label.class, LabelClass::Public | LabelClass::Redacted)
                || (label.class == LabelClass::Public && looks_sensitive(&label.value))
            {
                issues.push(ObservabilityIssue::LabelRequiresRedaction(label.name.clone()));
            }
            if label.value.len() > profile.bounds.max_label_value_bytes {
                issues.push(ObservabilityIssue::LabelValueTooLarge(label.name.clone()));
            }
        }
        validate_refs("snapshot-source-sample-refs", &item.source_sample_refs, issues);
    }
}

fn validate_bounds(bounds: &ObservationBounds, issues: &mut Vec<ObservabilityIssue>) {
    for (field, value) in [
        ("max-descriptors", bounds.max_descriptors),
        ("max-labels-per-sample", bounds.max_labels_per_sample),
        ("max-label-name-bytes", bounds.max_label_name_bytes),
        ("max-label-value-bytes", bounds.max_label_value_bytes),
        ("max-series", bounds.max_series),
        ("max-events", bounds.max_events),
        ("max-event-detail-bytes", bounds.max_event_detail_bytes),
        ("max-snapshots", bounds.max_snapshots),
        ("max-scan-items", bounds.max_scan_items),
        ("max-findings", bounds.max_findings),
        ("max-diagnostics", bounds.max_diagnostics),
    ] {
        if value == 0 {
            issues.push(ObservabilityIssue::ZeroBound(field));
        }
    }
    if bounds.max_queued_bytes == 0 {
        issues.push(ObservabilityIssue::ZeroBound("max-queued-bytes"));
    }
    if bounds.min_export_interval_ticks == 0 {
        issues.push(ObservabilityIssue::ZeroBound("min-export-interval-ticks"));
    }
}

fn validate_redaction_rules(profile: &ObservationProfile, issues: &mut Vec<ObservabilityIssue>) {
    let mut keys = BTreeSet::new();
    for rule in &profile.redaction_rules {
        validate_token("redaction-label-name", &rule.label_name, issues);
        if !rule.class.requires_redaction() {
            issues.push(ObservabilityIssue::RedactionRuleMissing(rule.label_name.clone()));
        }
        if !valid_redaction_marker(&rule.marker) {
            issues.push(ObservabilityIssue::RedactionMarkerInvalid(rule.label_name.clone()));
        }
        if !keys.insert((rule.label_name.clone(), rule.class)) {
            issues.push(ObservabilityIssue::DuplicateValue("redaction-rules"));
        }
    }
}

fn validate_label(
    profile: &ObservationProfile,
    descriptor: &MetricDescriptor,
    label: &MetricLabel,
    names: &mut BTreeSet<String>,
    sanitized: &mut Vec<MetricLabel>,
    issues: &mut Vec<ObservabilityIssue>,
) {
    if !names.insert(label.name.clone()) {
        issues.push(ObservabilityIssue::DuplicateValue("metric-label-name"));
    }
    if label.name.is_empty()
        || label.name.len() > profile.bounds.max_label_name_bytes
        || !valid_fabric_token(&label.name)
    {
        issues.push(ObservabilityIssue::MalformedToken("metric-label-name"));
    }
    if !descriptor.allowed_label_names.contains(&label.name) {
        issues.push(ObservabilityIssue::UnsupportedLabel(label.name.clone()));
    }
    if label.value.len() > profile.bounds.max_label_value_bytes {
        issues.push(ObservabilityIssue::LabelValueTooLarge(label.name.clone()));
    }
    if label.class == LabelClass::Public && looks_sensitive(&label.value) {
        issues.push(ObservabilityIssue::LabelRequiresRedaction(label.name.clone()));
        return;
    }
    if label.class.requires_redaction() {
        match profile
            .redaction_rules
            .iter()
            .find(|rule| rule.label_name == label.name && rule.class == label.class)
        {
            Some(rule) => sanitized.push(MetricLabel {
                name: label.name.clone(),
                value: rule.marker.clone(),
                class: LabelClass::Redacted,
            }),
            None => issues.push(ObservabilityIssue::RedactionRuleMissing(label.name.clone())),
        }
    } else {
        sanitized.push(label.clone());
    }
}

fn event_attribute_descriptor(profile: &ObservationProfile, event: &ObservationEvent) -> MetricDescriptor {
    let mut names = event.attributes.iter().map(|attribute| attribute.name.clone()).collect::<Vec<_>>();
    names.sort();
    names.dedup();
    MetricDescriptor {
        schema: METRIC_DESCRIPTOR_SCHEMA.to_string(),
        descriptor_id: format!("{}-attributes", event.event_kind),
        descriptor_ref: event.event_ref.clone(),
        profile_ref: profile.profile_ref.clone(),
        name: "event_attributes".to_string(),
        unit: "event".to_string(),
        kind: MetricKind::Gauge,
        aggregation: MetricAggregation::Last,
        allowed_label_names: names,
        description: "bounded event attributes".to_string(),
    }
}

fn validate_non_claims(claims: &[ObservabilityNonClaim], issues: &mut Vec<ObservabilityIssue>) {
    if claims.len() != REQUIRED_OBSERVABILITY_NON_CLAIMS.len() {
        issues.push(ObservabilityIssue::CollectionLimitExceeded("observability-non-claims"));
    }
    let supplied = claims.iter().copied().collect::<BTreeSet<_>>();
    if supplied.len() != claims.len() {
        issues.push(ObservabilityIssue::DuplicateValue("observability-non-claim"));
    }
    for required in REQUIRED_OBSERVABILITY_NON_CLAIMS {
        if !supplied.contains(&required) {
            issues.push(ObservabilityIssue::MissingNonClaim(required.as_str()));
        }
    }
}

fn validate_sorted_unique_tokens(field: &'static str, values: &[String], issues: &mut Vec<ObservabilityIssue>) {
    if values.windows(ADJACENT_PAIR_WIDTH).any(|pair| pair[0] >= pair[1]) {
        issues.push(ObservabilityIssue::DuplicateValue(field));
    }
    for value in values {
        validate_token(field, value, issues);
    }
}

fn validate_refs(field: &'static str, refs: &[String], issues: &mut Vec<ObservabilityIssue>) {
    if refs.len() > MAX_OBSERVATION_REFS {
        issues.push(ObservabilityIssue::CollectionLimitExceeded(field));
    }
    if refs.windows(ADJACENT_PAIR_WIDTH).any(|pair| pair[0] >= pair[1]) {
        issues.push(ObservabilityIssue::DuplicateValue(field));
    }
    for reference in refs {
        validate_ref(field, reference, issues);
    }
}

fn validate_schema(field: &'static str, actual: &str, expected: &str, issues: &mut Vec<ObservabilityIssue>) {
    if actual != expected {
        issues.push(ObservabilityIssue::SchemaMismatch(field));
    }
}

fn validate_token(field: &'static str, value: &str, issues: &mut Vec<ObservabilityIssue>) {
    if value.is_empty() {
        issues.push(ObservabilityIssue::EmptyField(field));
    } else if value.len() > MAX_OBSERVATION_TEXT_BYTES || !valid_fabric_token(value) {
        issues.push(ObservabilityIssue::MalformedToken(field));
    }
}

fn validate_text(field: &'static str, value: &str, issues: &mut Vec<ObservabilityIssue>) {
    if value.is_empty() {
        issues.push(ObservabilityIssue::EmptyField(field));
    } else if value.len() > MAX_OBSERVATION_TEXT_BYTES || value.chars().any(char::is_control) {
        issues.push(ObservabilityIssue::MalformedToken(field));
    }
}

fn validate_ref(field: &'static str, value: &str, issues: &mut Vec<ObservabilityIssue>) {
    if !valid_blake3_ref(value) {
        issues.push(ObservabilityIssue::MalformedRef(field));
    }
}

fn looks_sensitive(value: &str) -> bool {
    let normalized = value.to_ascii_lowercase();
    normalized.starts_with(SECRET_PREFIX)
        || normalized.starts_with(TOKEN_PREFIX)
        || normalized.starts_with(TICKET_PREFIX)
        || normalized.starts_with(UNIX_ABSOLUTE_PATH_PREFIX)
        || normalized.contains(PRIVATE_KEY_MARKER)
}

fn valid_redaction_marker(marker: &str) -> bool {
    !marker.is_empty()
        && marker.len() <= MAX_OBSERVATION_TEXT_BYTES
        && valid_fabric_token(marker)
        && !looks_sensitive(marker)
}

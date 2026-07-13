use std::collections::hash_map::DefaultHasher;
use std::hash::Hash;
use std::hash::Hasher;

use super::*;

const SYNTHETIC_REF_CHUNK_HEX_CHARS: usize = 16;
const SYNTHETIC_REF_CHUNK_REPETITIONS: usize = 4;
const GENERATION_ONE: u64 = 1;
const OBSERVED_TICK: u64 = 10;
const FRESH_UNTIL_TICK: u64 = 20;
const STALE_TICK: u64 = 21;
const SAMPLE_VALUE_LEFT: i64 = 7;
const SAMPLE_VALUE_RIGHT: i64 = 11;
const EXPECTED_SUM: i64 = SAMPLE_VALUE_LEFT + SAMPLE_VALUE_RIGHT;
const EXPECTED_LENGTH: u64 = 4;
const MAX_DESCRIPTORS: usize = 8;
const MAX_LABELS: usize = 4;
const MAX_LABEL_NAME_BYTES: usize = 32;
const MAX_LABEL_VALUE_BYTES: usize = 64;
const MAX_SERIES: usize = 8;
const MAX_EVENTS: usize = 8;
const MAX_EVENT_DETAIL_BYTES: usize = 128;
const MAX_QUEUED_BYTES: u64 = 4_096;
const MAX_SNAPSHOTS: usize = 8;
const MAX_SCAN_ITEMS: usize = 16;
const MAX_FINDINGS: usize = 8;
const MAX_DIAGNOSTICS: usize = 8;
const MIN_EXPORT_INTERVAL_TICKS: u64 = 2;
const ADAPTER_TIMEOUT_TICKS: u64 = 5;
const PAYLOAD_BYTES: u64 = 128;
const QUEUED_AT_LIMIT: u64 = MAX_QUEUED_BYTES - PAYLOAD_BYTES;
const QUEUED_NEAR_LIMIT: u64 = QUEUED_AT_LIMIT + 1;

fn test_ref(label: &str) -> String {
    let mut hasher = DefaultHasher::new();
    label.hash(&mut hasher);
    let digest = hasher.finish();
    let chunk = format!("{digest:0width$x}", width = SYNTHETIC_REF_CHUNK_HEX_CHARS);
    format!("blake3:{}", chunk.repeat(SYNTHETIC_REF_CHUNK_REPETITIONS))
}

fn profile() -> ObservationProfile {
    ObservationProfile {
        schema: OBSERVATION_PROFILE_SCHEMA.to_string(),
        profile_id: "bounded-production".to_string(),
        profile_ref: test_ref("observation-profile"),
        bounds: ObservationBounds {
            max_descriptors: MAX_DESCRIPTORS,
            max_labels_per_sample: MAX_LABELS,
            max_label_name_bytes: MAX_LABEL_NAME_BYTES,
            max_label_value_bytes: MAX_LABEL_VALUE_BYTES,
            max_series: MAX_SERIES,
            max_events: MAX_EVENTS,
            max_event_detail_bytes: MAX_EVENT_DETAIL_BYTES,
            max_queued_bytes: MAX_QUEUED_BYTES,
            max_snapshots: MAX_SNAPSHOTS,
            max_scan_items: MAX_SCAN_ITEMS,
            max_findings: MAX_FINDINGS,
            max_diagnostics: MAX_DIAGNOSTICS,
            min_export_interval_ticks: MIN_EXPORT_INTERVAL_TICKS,
        },
        redaction_rules: vec![
            RedactionRule {
                label_name: "credential".to_string(),
                class: LabelClass::Credential,
                marker: "redacted".to_string(),
            },
            RedactionRule {
                label_name: "path".to_string(),
                class: LabelClass::PrivatePath,
                marker: "redacted".to_string(),
            },
        ],
        non_claims: REQUIRED_OBSERVABILITY_NON_CLAIMS.to_vec(),
    }
}

fn context(source_id: &str, scope: ClaimScope) -> ObservationContext {
    ObservationContext {
        source_id: source_id.to_string(),
        source_ref: test_ref(source_id),
        profile_ref: profile().profile_ref,
        scope,
        generation: GENERATION_ONE,
        observed_tick: OBSERVED_TICK,
        valid_until_tick: FRESH_UNTIL_TICK,
        resource_ref: test_ref("resource"),
        evidence_refs: vec![test_ref("evidence")],
        non_claims: REQUIRED_OBSERVABILITY_NON_CLAIMS.to_vec(),
    }
}

fn descriptor() -> MetricDescriptor {
    MetricDescriptor {
        schema: METRIC_DESCRIPTOR_SCHEMA.to_string(),
        descriptor_id: "requests-total".to_string(),
        descriptor_ref: test_ref("requests-descriptor"),
        profile_ref: profile().profile_ref,
        name: "requests_total".to_string(),
        unit: "request".to_string(),
        kind: MetricKind::Counter,
        aggregation: MetricAggregation::Sum,
        allowed_label_names: vec!["credential".to_string(), "service".to_string()],
        description: "bounded request count".to_string(),
    }
}

fn sample(label: &str, value: i64) -> MetricSample {
    MetricSample {
        schema: METRIC_SAMPLE_SCHEMA.to_string(),
        sample_ref: test_ref(label),
        descriptor_ref: descriptor().descriptor_ref,
        context: context("extension-a", ClaimScope::SystemExtension),
        labels: vec![
            MetricLabel {
                name: "credential".to_string(),
                value: "secret:must-not-export".to_string(),
                class: LabelClass::Credential,
            },
            MetricLabel {
                name: "service".to_string(),
                value: "content-replication".to_string(),
                class: LabelClass::Public,
            },
        ],
        value,
    }
}

// r[verify molten.fabric_observability.model]
// r[verify molten.fabric_observability.pure_core]
// r[verify molten.fabric_observability.bounds_redaction]
#[test]
fn aggregation_is_stable_bounded_and_redacts_classified_values() {
    let profile = profile();
    let descriptor = descriptor();
    let left = sample("sample-left", SAMPLE_VALUE_LEFT);
    let right = sample("sample-right", SAMPLE_VALUE_RIGHT);
    let forward = aggregate_metric_samples(
        &profile,
        std::slice::from_ref(&descriptor),
        &[left.clone(), right.clone()],
        OBSERVED_TICK,
    )
    .expect("forward aggregation");
    let reverse = aggregate_metric_samples(&profile, std::slice::from_ref(&descriptor), &[right, left], OBSERVED_TICK)
        .expect("reverse aggregation");
    assert_eq!(forward, reverse);
    assert_eq!(forward.len(), 1);
    assert_eq!(forward[0].value, EXPECTED_SUM);
    assert!(
        forward[0].identity.labels.iter().any(|label| label.name == "credential"
            && label.value == "redacted"
            && label.class == LabelClass::Redacted)
    );
    assert!(forward[0].identity.labels.iter().all(|label| !label.value.contains("must-not-export")));
}

// r[verify molten.fabric_observability.pure_core]
// r[verify molten.fabric_observability.bounds_redaction]
#[test]
fn malformed_descriptors_unknown_labels_and_secret_misclassification_deny() {
    let profile = profile();
    let mut malformed = descriptor();
    malformed.aggregation = MetricAggregation::Last;
    let issues = validate_metric_descriptor(&profile, &malformed);
    assert!(issues.contains(&ObservabilityIssue::CounterRequiresSum));

    let mut secret = sample("secret-public", SAMPLE_VALUE_LEFT);
    secret.labels = vec![MetricLabel {
        name: "service".to_string(),
        value: "token:unclassified-secret".to_string(),
        class: LabelClass::Public,
    }];
    let issues = validate_metric_sample(&profile, &descriptor(), &secret, OBSERVED_TICK)
        .expect_err("misclassified secret denied");
    assert!(issues.contains(&ObservabilityIssue::LabelRequiresRedaction("service".to_string())));

    let mut unknown = sample("unknown-label", SAMPLE_VALUE_LEFT);
    unknown.labels.push(MetricLabel {
        name: "peer-id".to_string(),
        value: "unbounded-peer".to_string(),
        class: LabelClass::UnboundedIdentifier,
    });
    let issues =
        validate_metric_sample(&profile, &descriptor(), &unknown, OBSERVED_TICK).expect_err("unknown label denied");
    assert!(issues.contains(&ObservabilityIssue::UnsupportedLabel("peer-id".to_string())));
    assert!(issues.contains(&ObservabilityIssue::RedactionRuleMissing("peer-id".to_string())));

    let duplicate = sample("duplicate-sample", SAMPLE_VALUE_LEFT);
    let issues = aggregate_metric_samples(
        &profile,
        std::slice::from_ref(&descriptor()),
        &[duplicate.clone(), duplicate],
        OBSERVED_TICK,
    )
    .expect_err("duplicate sample identity denied");
    assert!(issues.contains(&ObservabilityIssue::DuplicateValue("metric-sample-ref")));
}

// r[verify molten.fabric_observability.health_scope]
// r[verify molten.fabric_observability.pure_core]
#[test]
fn health_is_deterministic_freshness_bound_and_cannot_silently_promote_scope() {
    let profile = profile();
    let input = HealthInput {
        schema: HEALTH_INPUT_SCHEMA.to_string(),
        health_ref: test_ref("extension-health"),
        context: context("extension-a", ClaimScope::SystemExtension),
        state: HealthState::Healthy,
        diagnostic_refs: Vec::new(),
    };
    let extension_policy = ReadinessPolicy {
        schema: READINESS_POLICY_SCHEMA.to_string(),
        policy_ref: test_ref("extension-readiness-policy"),
        target_scope: ClaimScope::SystemExtension,
        required_source_ids: vec!["extension-a".to_string()],
        scope_evidence_refs: Vec::new(),
        allow_degraded: false,
        as_of_tick: OBSERVED_TICK,
    };
    let first =
        evaluate_health_readiness(&profile, &extension_policy, HealthState::Unavailable, std::slice::from_ref(&input));
    let second =
        evaluate_health_readiness(&profile, &extension_policy, HealthState::Unavailable, std::slice::from_ref(&input));
    assert_eq!(first, second);
    assert_eq!(first.readiness, ReadinessDecision::Pass);

    let mut cluster_policy = extension_policy.clone();
    cluster_policy.target_scope = ClaimScope::Cluster;
    let denied =
        evaluate_health_readiness(&profile, &cluster_policy, HealthState::Healthy, std::slice::from_ref(&input));
    assert_eq!(denied.readiness, ReadinessDecision::Deny);
    assert!(denied.issues.contains(&ObservabilityIssue::ClaimScopeOverreach));

    let mut stale_policy = extension_policy;
    stale_policy.as_of_tick = STALE_TICK;
    let unavailable = evaluate_health_readiness(&profile, &stale_policy, HealthState::Healthy, &[input]);
    assert_eq!(unavailable.readiness, ReadinessDecision::Unavailable);
    assert_eq!(unavailable.state, HealthState::Unavailable);
}

fn integrity_plan() -> IntegrityPlan {
    IntegrityPlan {
        schema: INTEGRITY_PLAN_SCHEMA.to_string(),
        plan_ref: test_ref("integrity-plan"),
        profile_ref: profile().profile_ref,
        scope_ref: test_ref("durable-namespace"),
        generation: GENERATION_ONE,
        read_only: true,
        require_complete: true,
        max_items: MAX_SCAN_ITEMS,
        max_findings: MAX_FINDINGS,
        targets: vec![IntegrityTarget {
            item_ref: test_ref("item-a"),
            kind: IntegrityTargetKind::Content,
            expected_content_ref: Some(test_ref("content-a")),
            expected_length: Some(EXPECTED_LENGTH),
        }],
        resource_ref: test_ref("scan-resource"),
        policy_refs: vec![test_ref("scan-policy")],
        evidence_refs: vec![test_ref("scan-evidence")],
        non_claims: REQUIRED_OBSERVABILITY_NON_CLAIMS.to_vec(),
    }
}

fn scan(status: ScanItemStatus, content_ref: Option<String>) -> ScanObservation {
    ScanObservation {
        schema: SCAN_OBSERVATION_SCHEMA.to_string(),
        observation_ref: test_ref("scan-observation"),
        plan_ref: integrity_plan().plan_ref,
        item_ref: test_ref("item-a"),
        kind: IntegrityTargetKind::Content,
        status,
        observed_content_ref: content_ref,
        observed_length: Some(EXPECTED_LENGTH),
        evidence_refs: vec![test_ref("scan-item-evidence")],
    }
}

fn complete_scan() -> ScanCompletion {
    ScanCompletion {
        scanned_items: 1,
        declared_items: 1,
        exhausted: true,
        cancelled: false,
        unavailable: false,
    }
}

// r[verify molten.fabric_observability.integrity_readonly]
#[test]
fn integrity_pass_failure_and_partial_results_never_mutate() {
    let profile = profile();
    let plan = integrity_plan();
    let passed = evaluate_integrity_plan(
        &profile,
        &plan,
        &[scan(ScanItemStatus::Present, Some(test_ref("content-a")))],
        &complete_scan(),
    );
    assert_eq!(passed.decision, IntegrityDecision::Pass);
    assert!(passed.complete);
    assert!(!passed.mutation_performed);

    let corrupt = evaluate_integrity_plan(
        &profile,
        &plan,
        &[scan(ScanItemStatus::Corrupt, Some(test_ref("corrupt-content")))],
        &complete_scan(),
    );
    assert_eq!(corrupt.decision, IntegrityDecision::Fail);
    assert!(corrupt.findings.iter().any(|finding| finding.class == FindingClass::Corrupt));
    assert!(corrupt.findings.iter().all(|finding| !finding.grants_mutation_authority));
    assert!(!corrupt.mutation_performed);

    let partial_completion = ScanCompletion {
        scanned_items: 0,
        declared_items: 1,
        exhausted: false,
        cancelled: false,
        unavailable: false,
    };
    let partial = evaluate_integrity_plan(&profile, &plan, &[], &partial_completion);
    assert_eq!(partial.decision, IntegrityDecision::Partial);
    assert!(!partial.complete);

    let overclaimed = evaluate_integrity_plan(&profile, &plan, &[], &complete_scan());
    assert_ne!(overclaimed.decision, IntegrityDecision::Pass);
    assert!(!overclaimed.complete);

    let mut mutating_plan = plan;
    mutating_plan.read_only = false;
    let denied = evaluate_integrity_plan(&profile, &mutating_plan, &[], &partial_completion);
    assert_eq!(denied.decision, IntegrityDecision::Deny);
    assert!(denied.issues.contains(&ObservabilityIssue::MutationWithoutAuthority));
}

// r[verify molten.fabric_observability.integrity_readonly]
// r[verify molten.fabric_observability.health_scope]
#[test]
fn findings_and_telemetry_are_not_authority() {
    let finding_ref = test_ref("finding");
    assert_eq!(admit_integrity_mutation(&finding_ref, None), AuthorityDecision::Deny);
    assert_eq!(observation_authority_decision(), AuthorityDecision::Deny);
    let authority = IntegrityMutationAuthority {
        schema: INTEGRITY_MUTATION_AUTHORITY_SCHEMA.to_string(),
        operation: IntegrityMutationOperation::Quarantine,
        authority_ref: test_ref("quarantine-authority"),
        policy_ref: test_ref("quarantine-policy"),
        finding_refs: vec![finding_ref.clone()],
    };
    assert_eq!(admit_integrity_mutation(&finding_ref, Some(&authority)), AuthorityDecision::Admit);
    assert_eq!(admit_integrity_mutation(&test_ref("other-finding"), Some(&authority)), AuthorityDecision::Deny);
}

fn adapter_profile(required: bool, drop_on_backpressure: bool) -> ObservationAdapterProfile {
    ObservationAdapterProfile {
        schema: OBSERVATION_ADAPTER_PROFILE_SCHEMA.to_string(),
        adapter_id: "prometheus-export".to_string(),
        adapter_ref: test_ref("prometheus-adapter"),
        profile_ref: profile().profile_ref,
        class: ObservationAdapterClass::Prometheus,
        max_queued_bytes: MAX_QUEUED_BYTES,
        timeout_ticks: ADAPTER_TIMEOUT_TICKS,
        drop_on_backpressure,
        required,
        evidence_refs: vec![test_ref("adapter-evidence")],
        non_claims: REQUIRED_OBSERVABILITY_NON_CLAIMS.to_vec(),
    }
}

fn delivery_request() -> AdapterDeliveryRequest {
    AdapterDeliveryRequest {
        operation_ref: test_ref("export-operation"),
        adapter_ref: test_ref("prometheus-adapter"),
        payload_ref: test_ref("snapshot-payload"),
        payload_bytes: PAYLOAD_BYTES,
        submitted_tick: OBSERVED_TICK,
        deadline_tick: OBSERVED_TICK + ADAPTER_TIMEOUT_TICKS,
    }
}

fn runtime_observation() -> AdapterRuntimeObservation {
    AdapterRuntimeObservation {
        available: true,
        queued_bytes: 0,
        completed_tick: OBSERVED_TICK,
        dropped_observations: 0,
        cancelled: false,
        failure: None,
    }
}

// r[verify molten.fabric_observability.failure_semantics]
// r[verify molten.fabric_observability.adapter_contract]
#[test]
fn adapter_terminal_failures_are_explicit_bounded_and_policy_visible() {
    let profile = profile();
    let adapter = adapter_profile(true, false);
    let request = delivery_request();
    let exported = evaluate_adapter_delivery(&profile, &adapter, &request, &runtime_observation(), None);
    assert_eq!(exported.kind, AdapterOutcomeKind::Exported);
    assert!(!exported.service_policy_signal);

    let mut exact_bound_runtime = runtime_observation();
    exact_bound_runtime.queued_bytes = QUEUED_AT_LIMIT;
    let exact_bound = evaluate_adapter_delivery(&profile, &adapter, &request, &exact_bound_runtime, None);
    assert_eq!(exact_bound.kind, AdapterOutcomeKind::Exported);

    let mut unavailable_runtime = runtime_observation();
    unavailable_runtime.available = false;
    let unavailable = evaluate_adapter_delivery(&profile, &adapter, &request, &unavailable_runtime, None);
    assert_eq!(unavailable.kind, AdapterOutcomeKind::Unavailable);
    assert!(unavailable.service_policy_signal);

    let mut pressure_runtime = runtime_observation();
    pressure_runtime.queued_bytes = QUEUED_NEAR_LIMIT;
    let pressure = evaluate_adapter_delivery(&profile, &adapter, &request, &pressure_runtime, None);
    assert_eq!(pressure.kind, AdapterOutcomeKind::Backpressure);

    let dropping_adapter = adapter_profile(false, true);
    let dropped = evaluate_adapter_delivery(&profile, &dropping_adapter, &request, &pressure_runtime, None);
    assert_eq!(dropped.kind, AdapterOutcomeKind::Dropped);
    assert!(!dropped.service_policy_signal);

    let mut timeout_runtime = runtime_observation();
    timeout_runtime.completed_tick = request.deadline_tick + 1;
    let timeout = evaluate_adapter_delivery(&profile, &adapter, &request, &timeout_runtime, None);
    assert_eq!(timeout.kind, AdapterOutcomeKind::Timeout);

    let mut cancelled_runtime = runtime_observation();
    cancelled_runtime.cancelled = true;
    let cancelled = evaluate_adapter_delivery(&profile, &adapter, &request, &cancelled_runtime, None);
    assert_eq!(cancelled.kind, AdapterOutcomeKind::Cancelled);

    let stale = evaluate_adapter_delivery(&profile, &adapter, &request, &runtime_observation(), Some(OBSERVED_TICK));
    assert_eq!(stale.kind, AdapterOutcomeKind::Stale);

    for (failure, expected) in [
        (AdapterFailureClass::PermissionDenied, AdapterOutcomeKind::PermissionDenied),
        (AdapterFailureClass::UnsupportedCapability, AdapterOutcomeKind::Unsupported),
        (AdapterFailureClass::CorruptInput, AdapterOutcomeKind::Corrupt),
        (AdapterFailureClass::AdapterFailure, AdapterOutcomeKind::Failed),
    ] {
        let mut failed_runtime = runtime_observation();
        failed_runtime.failure = Some(failure);
        let failed = evaluate_adapter_delivery(&profile, &adapter, &request, &failed_runtime, None);
        assert_eq!(failed.kind, expected);
    }

    let mut dropped_runtime = runtime_observation();
    dropped_runtime.dropped_observations = 1;
    let dropped = evaluate_adapter_delivery(&profile, &adapter, &request, &dropped_runtime, None);
    assert_eq!(dropped.kind, AdapterOutcomeKind::Dropped);
}

// r[verify molten.fabric_observability.model]
// r[verify molten.fabric_observability.bounds_redaction]
#[test]
fn event_and_snapshot_validation_rejects_stale_or_secret_views() {
    let profile = profile();
    let event = ObservationEvent {
        schema: OBSERVATION_EVENT_SCHEMA.to_string(),
        event_ref: test_ref("event"),
        event_kind: "extension-transition".to_string(),
        severity: EventSeverity::Info,
        context: context("extension-a", ClaimScope::SystemExtension),
        detail: "bounded-transition".to_string(),
        attributes: vec![MetricLabel {
            name: "path".to_string(),
            value: "/private/state/root".to_string(),
            class: LabelClass::PrivatePath,
        }],
    };
    let sanitized = validate_event(&profile, &event, OBSERVED_TICK).expect("redacted event");
    assert_eq!(sanitized.attributes[0].value, "redacted");

    let mut secret_event = event;
    secret_event.detail = "secret:raw-event-payload".to_string();
    assert!(validate_event(&profile, &secret_event, OBSERVED_TICK).is_err());

    let snapshot = ObservationSnapshot {
        schema: OBSERVATION_SNAPSHOT_SCHEMA.to_string(),
        snapshot_id: "extension-snapshot".to_string(),
        profile_ref: profile.profile_ref.clone(),
        scope: ClaimScope::SystemExtension,
        generation: GENERATION_ONE,
        as_of_tick: OBSERVED_TICK,
        valid_until_tick: FRESH_UNTIL_TICK,
        series: Vec::new(),
        event_refs: vec![test_ref("event")],
        health_refs: vec![test_ref("health")],
        integrity_result_refs: Vec::new(),
        adapter_outcome_refs: Vec::new(),
        evidence_refs: vec![test_ref("snapshot-evidence")],
        non_claims: REQUIRED_OBSERVABILITY_NON_CLAIMS.to_vec(),
    };
    assert!(validate_snapshot(&profile, &snapshot, OBSERVED_TICK).is_empty());
    let issues = validate_snapshot(&profile, &snapshot, STALE_TICK);
    assert!(issues.contains(&ObservabilityIssue::ObservationStale("extension-snapshot".to_string())));

    let mut next_snapshot = snapshot.clone();
    next_snapshot.snapshot_id = "extension-snapshot-next".to_string();
    next_snapshot.as_of_tick = OBSERVED_TICK + 1;
    let issues = validate_snapshot_batch(&profile, &[snapshot, next_snapshot], OBSERVED_TICK);
    assert!(issues.contains(&ObservabilityIssue::ExportFrequencyExceeded));
}

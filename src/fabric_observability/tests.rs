use std::collections::BTreeMap;

use super::*;

const GENERATION_ONE: u64 = 1;
const GENERATION_TWO: u64 = GENERATION_ONE + 1;
const ADJACENT_PAIR_WIDTH: usize = 2;
const SIMULATION_RECORD_LIMIT: usize = 1;
const OBSERVABILITY_PORT_COUNT: usize = 2;
const OBSERVED_TICK: u64 = 100;
const FRESH_UNTIL_TICK: u64 = 200;
const ADAPTER_TIMEOUT_TICKS: u64 = 10;
const MAX_DESCRIPTORS: usize = 16;
const MAX_LABELS: usize = 4;
const MAX_LABEL_NAME_BYTES: usize = 32;
const MAX_LABEL_VALUE_BYTES: usize = 64;
const MAX_SERIES: usize = 16;
const MAX_EVENTS: usize = 16;
const MAX_EVENT_DETAIL_BYTES: usize = 128;
const MAX_QUEUED_BYTES: u64 = 8_192;
const MAX_SNAPSHOTS: usize = 8;
const MAX_SCAN_ITEMS: usize = 16;
const MAX_FINDINGS: usize = 8;
const MAX_DIAGNOSTICS: usize = 8;
const MIN_EXPORT_INTERVAL_TICKS: u64 = 2;
const SAMPLE_VALUE: i64 = 9;
const MAX_SCAN_BYTES: u64 = 1_024;

fn test_ref(label: &str) -> String {
    crate::preserves_rail::content_ref_from_bytes(label.as_bytes())
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
        redaction_rules: vec![RedactionRule {
            label_name: "credential".to_string(),
            class: LabelClass::Credential,
            marker: "redacted".to_string(),
        }],
        non_claims: REQUIRED_OBSERVABILITY_NON_CLAIMS.to_vec(),
    }
}

fn context() -> ObservationContext {
    ObservationContext {
        source_id: "extension-a".to_string(),
        source_ref: test_ref("extension-a"),
        profile_ref: profile().profile_ref,
        scope: ClaimScope::SystemExtension,
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

fn sample() -> MetricSample {
    MetricSample {
        schema: METRIC_SAMPLE_SCHEMA.to_string(),
        sample_ref: test_ref("sample"),
        descriptor_ref: descriptor().descriptor_ref,
        context: context(),
        labels: vec![
            MetricLabel {
                name: "credential".to_string(),
                value: "secret:never-export".to_string(),
                class: LabelClass::Credential,
            },
            MetricLabel {
                name: "service".to_string(),
                value: "extension-a".to_string(),
                class: LabelClass::Public,
            },
        ],
        value: SAMPLE_VALUE,
    }
}

fn snapshot() -> ObservationSnapshot {
    let profile = profile();
    let series =
        aggregate_metric_samples(&profile, &[descriptor()], &[sample()], OBSERVED_TICK).expect("aggregate sample");
    ObservationSnapshot {
        schema: OBSERVATION_SNAPSHOT_SCHEMA.to_string(),
        snapshot_id: "extension-snapshot".to_string(),
        profile_ref: profile.profile_ref,
        scope: ClaimScope::SystemExtension,
        generation: GENERATION_ONE,
        as_of_tick: OBSERVED_TICK,
        valid_until_tick: FRESH_UNTIL_TICK,
        series,
        event_refs: Vec::new(),
        health_refs: Vec::new(),
        integrity_result_refs: Vec::new(),
        adapter_outcome_refs: Vec::new(),
        evidence_refs: vec![test_ref("snapshot-evidence")],
        non_claims: REQUIRED_OBSERVABILITY_NON_CLAIMS.to_vec(),
    }
}

fn adapter(class: ObservationAdapterClass) -> ObservationAdapterProfile {
    ObservationAdapterProfile {
        schema: OBSERVATION_ADAPTER_PROFILE_SCHEMA.to_string(),
        adapter_id: format!("{}-adapter", class.as_str()),
        adapter_ref: test_ref(&format!("{}-adapter", class.as_str())),
        profile_ref: profile().profile_ref,
        class,
        max_queued_bytes: MAX_QUEUED_BYTES,
        timeout_ticks: ADAPTER_TIMEOUT_TICKS,
        drop_on_backpressure: false,
        required: true,
        evidence_refs: vec![test_ref("adapter-evidence")],
        non_claims: REQUIRED_OBSERVABILITY_NON_CLAIMS.to_vec(),
    }
}

#[derive(Debug)]
struct RecordingSink {
    calls: usize,
    completion: SinkCompletion,
    payload: Vec<u8>,
}

impl ObservationSink for RecordingSink {
    fn emit(&mut self, _media_type: &str, payload: &[u8], _payload_ref: &str) -> SinkCompletion {
        self.calls += 1;
        self.payload = payload.to_vec();
        self.completion.clone()
    }
}

fn success_sink() -> RecordingSink {
    RecordingSink {
        calls: 0,
        completion: SinkCompletion {
            completed_tick: OBSERVED_TICK,
            dropped_observations: 0,
            failure: None,
        },
        payload: Vec::new(),
    }
}

fn export_request(
    adapter: &ObservationAdapterProfile,
    snapshot: &ObservationSnapshot,
    format: ExportFormat,
) -> AdapterDeliveryRequest {
    let canonical = canonical_observation_snapshot(&profile(), snapshot, OBSERVED_TICK).expect("snapshot");
    let payload = match format {
        ExportFormat::Prometheus => render_prometheus_snapshot(snapshot).expect("prometheus"),
        ExportFormat::OpenTelemetryJson => render_opentelemetry_snapshot(snapshot).expect("otel"),
        ExportFormat::TracingReference => canonical.artifact_ref.as_bytes().to_vec(),
    };
    AdapterDeliveryRequest {
        operation_ref: test_ref("export-operation"),
        adapter_ref: adapter.adapter_ref.clone(),
        payload_ref: canonical.artifact_ref,
        payload_bytes: u64::try_from(payload.len()).expect("payload length"),
        submitted_tick: OBSERVED_TICK,
        deadline_tick: OBSERVED_TICK + ADAPTER_TIMEOUT_TICKS,
    }
}

// r[verify molten.fabric_observability.adapter_contract]
#[test]
fn observability_and_integrity_ports_are_versioned_exact_and_non_authoritative() {
    let descriptors = fabric_observability_port_descriptors(&profile().profile_ref);
    let registry = crate::fabric::build_fabric_port_registry(&descriptors).expect("observability port registry");
    assert_eq!(registry.descriptors().len(), OBSERVABILITY_PORT_COUNT);
    assert!(
        registry
            .descriptors()
            .iter()
            .all(|descriptor| descriptor.class == crate::fabric::FabricPortClass::Evidence)
    );

    let mut malformed = descriptors;
    malformed[0].conformance_refs.clear();
    assert!(crate::fabric::build_fabric_port_registry(&malformed).is_err());
}

// r[verify molten.fabric_observability.adapter_contract]
// r[verify molten.fabric_observability.bounds_redaction]
// r[verify molten.fabric_observability.final_validation]
#[test]
fn prometheus_opentelemetry_and_tracing_shells_export_only_bounded_public_views() {
    let profile = profile();
    let snapshot = snapshot();
    let mut canonical_refs = Vec::new();
    let mut prometheus_payload = Vec::new();
    for (class, format) in [
        (ObservationAdapterClass::Prometheus, ExportFormat::Prometheus),
        (ObservationAdapterClass::OpenTelemetry, ExportFormat::OpenTelemetryJson),
        (ObservationAdapterClass::Tracing, ExportFormat::TracingReference),
    ] {
        let adapter = adapter(class);
        let request = export_request(&adapter, &snapshot, format);
        let mut sink = success_sink();
        let execution = execute_snapshot_export(
            &profile,
            &adapter,
            &snapshot,
            &request,
            &ExportShellState {
                available: true,
                queued_bytes: 0,
                cancelled: false,
            },
            None,
            format,
            &mut sink,
        )
        .expect("export");
        assert_eq!(execution.outcome.artifact.kind, AdapterOutcomeKind::Exported);
        assert_eq!(sink.calls, 1);
        assert!(!sink.payload.windows("never-export".len()).any(|window| window == b"never-export"));
        canonical_refs.push(execution.payload_ref);
        if format == ExportFormat::Prometheus {
            prometheus_payload = sink.payload;
        }
    }
    assert!(canonical_refs.windows(ADJACENT_PAIR_WIDTH).all(|pair| pair[0] == pair[1]));

    let tracing_adapter = adapter(ObservationAdapterClass::Tracing);
    let event = ObservationEvent {
        schema: OBSERVATION_EVENT_SCHEMA.to_string(),
        event_ref: test_ref("adapter-state-change"),
        event_kind: "adapter-state-change".to_string(),
        severity: EventSeverity::Info,
        context: context(),
        detail: "exporter admitted".to_string(),
        attributes: vec![MetricLabel {
            name: "service".to_string(),
            value: "extension-a".to_string(),
            class: LabelClass::Public,
        }],
    };
    let canonical_event = canonical_observation_event(&profile, &event, OBSERVED_TICK).expect("canonical event");
    let event_request = AdapterDeliveryRequest {
        operation_ref: test_ref("event-export-operation"),
        adapter_ref: tracing_adapter.adapter_ref.clone(),
        payload_ref: canonical_event.artifact_ref.clone(),
        payload_bytes: u64::try_from(canonical_event.artifact_ref.len()).expect("event reference length"),
        submitted_tick: OBSERVED_TICK,
        deadline_tick: OBSERVED_TICK + ADAPTER_TIMEOUT_TICKS,
    };
    let mut event_sink = success_sink();
    let tracing_event = execute_event_export(
        &profile,
        &tracing_adapter,
        &event,
        &event_request,
        &ExportShellState {
            available: true,
            queued_bytes: 0,
            cancelled: false,
        },
        None,
        &mut event_sink,
    )
    .expect("tracing event export");
    assert_eq!(tracing_event.payload, tracing_event.payload_ref.as_bytes());
    assert!(
        execute_event_export(
            &profile,
            &adapter(ObservationAdapterClass::Prometheus),
            &event,
            &event_request,
            &ExportShellState {
                available: true,
                queued_bytes: 0,
                cancelled: false,
            },
            None,
            &mut event_sink,
        )
        .is_err()
    );

    let simulation_adapter = adapter(ObservationAdapterClass::DeterministicSimulation);
    let simulation_request = export_request(&simulation_adapter, &snapshot, ExportFormat::Prometheus);
    let mut simulation_sink =
        DeterministicSimulationSink::new(OBSERVED_TICK, SIMULATION_RECORD_LIMIT).expect("simulation sink");
    let simulation = execute_snapshot_export(
        &profile,
        &simulation_adapter,
        &snapshot,
        &simulation_request,
        &ExportShellState {
            available: true,
            queued_bytes: 0,
            cancelled: false,
        },
        None,
        ExportFormat::Prometheus,
        &mut simulation_sink,
    )
    .expect("simulation export");
    assert_eq!(simulation.outcome.artifact.kind, AdapterOutcomeKind::Exported);
    assert_eq!(simulation.payload, prometheus_payload);
    assert_eq!(simulation_sink.emitted_refs(), &[simulation.payload_ref]);
}

// r[verify molten.fabric_observability.failure_semantics]
// r[verify molten.fabric_observability.final_validation]
#[test]
fn exporter_unavailable_backpressure_and_sink_failure_are_terminal_without_hidden_retry() {
    let profile = profile();
    let snapshot = snapshot();
    let adapter = adapter(ObservationAdapterClass::Prometheus);
    let request = export_request(&adapter, &snapshot, ExportFormat::Prometheus);
    let mut sink = success_sink();
    let unavailable = execute_snapshot_export(
        &profile,
        &adapter,
        &snapshot,
        &request,
        &ExportShellState {
            available: false,
            queued_bytes: 0,
            cancelled: false,
        },
        None,
        ExportFormat::Prometheus,
        &mut sink,
    )
    .expect("unavailable outcome");
    assert_eq!(unavailable.outcome.artifact.kind, AdapterOutcomeKind::Unavailable);
    assert_eq!(sink.calls, 0);

    let mut pressure_sink = success_sink();
    let pressure = execute_snapshot_export(
        &profile,
        &adapter,
        &snapshot,
        &request,
        &ExportShellState {
            available: true,
            queued_bytes: MAX_QUEUED_BYTES,
            cancelled: false,
        },
        None,
        ExportFormat::Prometheus,
        &mut pressure_sink,
    )
    .expect("backpressure outcome");
    assert_eq!(pressure.outcome.artifact.kind, AdapterOutcomeKind::Backpressure);
    assert_eq!(pressure_sink.calls, 0);

    let mut failed_sink = RecordingSink {
        calls: 0,
        completion: SinkCompletion {
            completed_tick: OBSERVED_TICK,
            dropped_observations: 0,
            failure: Some(AdapterFailureClass::AdapterFailure),
        },
        payload: Vec::new(),
    };
    let failed = execute_snapshot_export(
        &profile,
        &adapter,
        &snapshot,
        &request,
        &ExportShellState {
            available: true,
            queued_bytes: 0,
            cancelled: false,
        },
        None,
        ExportFormat::Prometheus,
        &mut failed_sink,
    )
    .expect("failed outcome");
    assert_eq!(failed.outcome.artifact.kind, AdapterOutcomeKind::Failed);
    assert_eq!(failed_sink.calls, 1);
    assert!(failed.payload.is_empty());
}

fn integrity_plan(bytes: &[u8]) -> IntegrityPlan {
    IntegrityPlan {
        schema: INTEGRITY_PLAN_SCHEMA.to_string(),
        plan_ref: test_ref("integrity-plan"),
        profile_ref: profile().profile_ref,
        scope_ref: test_ref("integrity-scope"),
        generation: GENERATION_ONE,
        read_only: true,
        require_complete: true,
        max_items: MAX_SCAN_ITEMS,
        max_findings: MAX_FINDINGS,
        targets: vec![IntegrityTarget {
            item_ref: test_ref("integrity-item"),
            kind: IntegrityTargetKind::DurableRecord,
            expected_content_ref: Some(crate::preserves_rail::content_ref_from_bytes(bytes)),
            expected_length: Some(u64::try_from(bytes.len()).expect("fixture length")),
        }],
        resource_ref: test_ref("integrity-resource"),
        policy_refs: vec![test_ref("integrity-policy")],
        evidence_refs: vec![test_ref("integrity-evidence")],
        non_claims: REQUIRED_OBSERVABILITY_NON_CLAIMS.to_vec(),
    }
}

// r[verify molten.fabric_observability.integrity_readonly]
// r[verify molten.fabric_observability.final_validation]
#[test]
fn capability_rooted_durable_scan_detects_corruption_partial_and_overbound_without_mutation() {
    let original = b"durable-record";
    let workspace = temp_dir("observability-durable-scan");
    let namespace =
        crate::node_state::NodeStateNamespace::open(crate::node_state::NodeStateNamespaceKind::Ledger, &workspace)
            .expect("ledger namespace");
    let path = crate::node_state::NodeStatePath::parse("record.bin").expect("record path");
    namespace.write(&path, original).expect("write record");
    let plan = integrity_plan(original);
    let binding = DurableScanBinding {
        item_ref: plan.targets[0].item_ref.clone(),
        path: path.clone(),
    };
    let control = ScanShellControl {
        max_items: MAX_SCAN_ITEMS,
        max_item_bytes: MAX_SCAN_BYTES,
        cancelled: false,
    };
    let passed = scan_durable_namespace(&profile(), &plan, &namespace, std::slice::from_ref(&binding), &control)
        .expect("scan pass");
    assert_eq!(passed.result.artifact.decision, IntegrityDecision::Pass);
    assert_eq!(namespace.read(&path, MAX_SCAN_BYTES).expect("readback"), original);

    let corrupt = b"corrupt-record";
    namespace.write(&path, corrupt).expect("write corruption fixture");
    let failed = scan_durable_namespace(&profile(), &plan, &namespace, std::slice::from_ref(&binding), &control)
        .expect("scan corruption");
    assert_eq!(failed.result.artifact.decision, IntegrityDecision::Fail);
    assert!(failed.result.artifact.findings.iter().any(|finding| finding.class == FindingClass::ContentMismatch));
    assert_eq!(namespace.read(&path, MAX_SCAN_BYTES).expect("corrupt readback"), corrupt);

    let cancelled =
        scan_durable_namespace(&profile(), &plan, &namespace, std::slice::from_ref(&binding), &ScanShellControl {
            cancelled: true,
            ..control.clone()
        })
        .expect("cancelled scan");
    assert_eq!(cancelled.result.artifact.decision, IntegrityDecision::Cancelled);

    let overbound = scan_durable_namespace(&profile(), &plan, &namespace, &[binding], &ScanShellControl {
        max_item_bytes: 1,
        ..control
    })
    .expect("overbound scan");
    assert!(overbound.result.artifact.findings.iter().any(|finding| finding.class == FindingClass::OverBound));
    std::fs::remove_dir_all(workspace).expect("remove integrity scan workspace");
}

struct FixtureContentSource {
    values: BTreeMap<String, Vec<u8>>,
}

impl ReadOnlyContentSource for FixtureContentSource {
    fn observe_bounded(&self, item_ref: &str, max_bytes: u64) -> ReadOnlyContentObservation {
        match self.values.get(item_ref) {
            Some(bytes) if u64::try_from(bytes.len()).is_ok_and(|length| length <= max_bytes) => {
                ReadOnlyContentObservation {
                    status: ScanItemStatus::Present,
                    bytes: Some(bytes.clone()),
                    evidence_refs: vec![test_ref("content-source-evidence")],
                }
            }
            Some(_) => ReadOnlyContentObservation {
                status: ScanItemStatus::OverBound,
                bytes: None,
                evidence_refs: vec![test_ref("content-source-evidence")],
            },
            None => ReadOnlyContentObservation {
                status: ScanItemStatus::Missing,
                bytes: None,
                evidence_refs: vec![test_ref("content-source-evidence")],
            },
        }
    }
}

// r[verify molten.fabric_observability.adapter_contract]
// r[verify molten.fabric_observability.integrity_readonly]
#[test]
fn content_verification_and_simulation_sources_share_read_only_scan_semantics() {
    let bytes = b"content-value";
    let mut plan = integrity_plan(bytes);
    plan.targets[0].kind = IntegrityTargetKind::Content;
    let source = FixtureContentSource {
        values: BTreeMap::from([(plan.targets[0].item_ref.clone(), bytes.to_vec())]),
    };
    let execution = scan_content_source(&profile(), &plan, &source, &ScanShellControl {
        max_items: MAX_SCAN_ITEMS,
        max_item_bytes: MAX_SCAN_BYTES,
        cancelled: false,
    })
    .expect("content scan");
    assert_eq!(execution.result.artifact.decision, IntegrityDecision::Pass);
}

// r[verify molten.fabric_observability.health_scope]
// r[verify molten.fabric_observability.final_validation]
#[test]
fn node_and_extension_health_project_to_scoped_canonical_readiness_and_operator_snapshot() {
    let profile = profile();
    let extension_state = crate::system_extension::LifecycleState {
        generation: GENERATION_ONE,
        phase: crate::system_extension::LifecyclePhase::Running,
        restart_attempts: 0,
        health: crate::system_extension::HealthState::Healthy,
        checkpoint_ref: None,
    };
    let extension_source_ref = test_ref("extension-a");
    let node_source_ref = test_ref("node-a");
    let resource_ref = test_ref("health-resource");
    let extension = system_extension_health_input(
        health_projection(
            "extension-a",
            &extension_source_ref,
            &profile.profile_ref,
            &resource_ref,
            ClaimScope::SystemExtension,
        ),
        &extension_state,
    );
    canonical_health_input(&profile, &extension, OBSERVED_TICK).expect("extension health");
    let node = node_health_input(
        health_projection("node-a", &node_source_ref, &profile.profile_ref, &resource_ref, ClaimScope::LocalComponent),
        "pass",
    )
    .expect("node health");
    canonical_health_input(&profile, &node, OBSERVED_TICK).expect("node health canonical");

    let failed_extension = system_extension_health_input(
        health_projection(
            "extension-a",
            &extension_source_ref,
            &profile.profile_ref,
            &resource_ref,
            ClaimScope::SystemExtension,
        ),
        &crate::system_extension::LifecycleState {
            generation: GENERATION_TWO,
            phase: crate::system_extension::LifecyclePhase::Running,
            restart_attempts: 1,
            health: crate::system_extension::HealthState::Failed,
            checkpoint_ref: None,
        },
    );
    assert_eq!(failed_extension.state, HealthState::Failed);
    assert_eq!(failed_extension.context.generation, GENERATION_TWO);
    canonical_health_input(&profile, &failed_extension, OBSERVED_TICK).expect("failed extension health");
    assert!(
        node_health_input(
            health_projection(
                "node-a",
                &node_source_ref,
                &profile.profile_ref,
                &resource_ref,
                ClaimScope::LocalComponent,
            ),
            "ambient-healthy",
        )
        .is_err()
    );

    let policy = ReadinessPolicy {
        schema: READINESS_POLICY_SCHEMA.to_string(),
        policy_ref: test_ref("extension-readiness"),
        target_scope: ClaimScope::SystemExtension,
        required_source_ids: vec!["extension-a".to_string()],
        scope_evidence_refs: Vec::new(),
        allow_degraded: false,
        as_of_tick: OBSERVED_TICK,
    };
    canonical_readiness_policy(&profile, &policy).expect("readiness policy");
    let decision =
        evaluate_health_readiness(&profile, &policy, HealthState::Unavailable, std::slice::from_ref(&extension));
    let canonical_decision = canonical_health_decision(&decision).expect("health decision");
    assert_eq!(decision.readiness, ReadinessDecision::Pass);

    let operator = bounded_operator_snapshot(&profile, SnapshotBuildInput {
        snapshot_id: "operator-extension-a",
        profile_ref: &profile.profile_ref,
        scope: ClaimScope::SystemExtension,
        generation: GENERATION_ONE,
        as_of_tick: OBSERVED_TICK,
        valid_until_tick: FRESH_UNTIL_TICK,
        series: snapshot().series,
        event_refs: Vec::new(),
        health_refs: vec![canonical_decision.artifact_ref],
        integrity_result_refs: Vec::new(),
        adapter_outcome_refs: Vec::new(),
        evidence_refs: vec![test_ref("operator-evidence")],
    })
    .expect("operator snapshot");
    assert_eq!(operator.artifact.scope, ClaimScope::SystemExtension);
    assert_eq!(observation_authority_decision(), AuthorityDecision::Deny);
}

fn health_projection<'a>(
    source_id: &'a str,
    source_ref: &'a str,
    profile_ref: &'a str,
    resource_ref: &'a str,
    scope: ClaimScope,
) -> HealthProjectionInput<'a> {
    HealthProjectionInput {
        source_id,
        source_ref,
        profile_ref,
        scope,
        generation: GENERATION_ONE,
        observed_tick: OBSERVED_TICK,
        valid_until_tick: FRESH_UNTIL_TICK,
        resource_ref,
        evidence_refs: vec![test_ref("health-evidence")],
        diagnostic_refs: Vec::new(),
    }
}

fn temp_dir(name: &str) -> std::path::PathBuf {
    crate::test_support::cleanup_stale_molten_temp_dirs();
    static TEMP_DIR_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let nonce = TEMP_DIR_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("molten-{name}-{}-{nonce}", std::process::id()));
    if dir.exists() {
        std::fs::remove_dir_all(&dir).expect("remove stale temp dir");
    }
    std::fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

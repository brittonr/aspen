use super::*;

const GENERATION_ONE: u64 = 1;
const GENERATION_TWO: u64 = GENERATION_ONE + 1;
const ADJACENT_PAIR_WIDTH: usize = 2;
const SIMULATION_RECORD_LIMIT: u64 = 1;
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
    let exports = [
        (ObservationAdapterClass::Prometheus, ExportFormat::Prometheus),
        (ObservationAdapterClass::OpenTelemetry, ExportFormat::OpenTelemetryJson),
        (ObservationAdapterClass::Tracing, ExportFormat::TracingReference),
    ];
    let mut canonical_refs = Vec::with_capacity(exports.len());
    let mut prometheus_payload = Vec::new();
    for (class, format) in exports {
        let adapter = adapter(class);
        let request = export_request(&adapter, &snapshot, format);
        let mut sink = success_sink();
        let execution = execute_snapshot_export(
            AdapterDelivery {
                profile: &profile,
                adapter: &adapter,
                request: &request,
                state: &shell_state(true, 0),
                last_export_tick: None,
            },
            &snapshot,
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

    assert_tracing_event_export_only(&profile);

    let simulation_adapter = adapter(ObservationAdapterClass::DeterministicSimulation);
    let simulation_request = export_request(&simulation_adapter, &snapshot, ExportFormat::Prometheus);
    let mut simulation_sink =
        DeterministicSimulationSink::new(OBSERVED_TICK, SIMULATION_RECORD_LIMIT).expect("simulation sink");
    let simulation = execute_snapshot_export(
        AdapterDelivery {
            profile: &profile,
            adapter: &simulation_adapter,
            request: &simulation_request,
            state: &shell_state(true, 0),
            last_export_tick: None,
        },
        &snapshot,
        ExportFormat::Prometheus,
        &mut simulation_sink,
    )
    .expect("simulation export");
    assert_eq!(simulation.outcome.artifact.kind, AdapterOutcomeKind::Exported);
    assert_eq!(simulation.payload, prometheus_payload);
    assert_eq!(simulation_sink.emitted_refs(), &[simulation.payload_ref]);
}

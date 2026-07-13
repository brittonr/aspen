use serde_json::json;

use super::*;
use crate::error::MoltenError;
use crate::error::Result;

const PROMETHEUS_MEDIA_TYPE: &str = "text/plain; version=0.0.4";
const OTLP_JSON_MEDIA_TYPE: &str = "application/x-ndjson; profile=molten-otlp-v1";
const TRACING_MEDIA_TYPE: &str = "application/x-molten-tracing-ref";
const OTLP_EVENT_MEDIA_TYPE: &str = "application/json; profile=molten-otel-event-v1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportShellState {
    pub available: bool,
    pub queued_bytes: u64,
    pub cancelled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SinkCompletion {
    pub completed_tick: u64,
    pub dropped_observations: u64,
    pub failure: Option<AdapterFailureClass>,
}

pub trait ObservationSink {
    fn emit(&mut self, media_type: &str, payload: &[u8], payload_ref: &str) -> SinkCompletion;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportExecution {
    pub media_type: String,
    pub payload: Vec<u8>,
    pub payload_ref: String,
    pub outcome: CanonicalArtifact<AdapterOutcome>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    Prometheus,
    OpenTelemetryJson,
    TracingReference,
}

// r[impl molten.fabric_observability.adapter_contract]
// r[impl molten.fabric_observability.failure_semantics]
pub fn execute_snapshot_export(
    profile: &ObservationProfile,
    adapter: &ObservationAdapterProfile,
    snapshot: &ObservationSnapshot,
    request: &AdapterDeliveryRequest,
    state: &ExportShellState,
    last_export_tick: Option<u64>,
    format: ExportFormat,
    sink: &mut dyn ObservationSink,
) -> Result<ExportExecution> {
    require_export_class(adapter.class, format)?;
    let canonical = canonical_observation_snapshot(profile, snapshot, request.submitted_tick)?;
    let (media_type, payload) = render_snapshot(format, &canonical)?;
    execute_rendered_export(
        profile,
        adapter,
        request,
        state,
        last_export_tick,
        media_type,
        payload,
        canonical.artifact_ref,
        sink,
    )
}

pub fn execute_event_export(
    profile: &ObservationProfile,
    adapter: &ObservationAdapterProfile,
    event: &ObservationEvent,
    request: &AdapterDeliveryRequest,
    state: &ExportShellState,
    last_export_tick: Option<u64>,
    sink: &mut dyn ObservationSink,
) -> Result<ExportExecution> {
    if !matches!(
        adapter.class,
        ObservationAdapterClass::Tracing
            | ObservationAdapterClass::OpenTelemetry
            | ObservationAdapterClass::DeterministicSimulation
    ) {
        return Err(MoltenError::invalid_harness(
            "event export requires tracing, OpenTelemetry, or deterministic simulation adapter",
        ));
    }
    let canonical = canonical_observation_event(profile, event, request.submitted_tick)?;
    let (media_type, payload) = render_event(adapter.class, &canonical)?;
    execute_rendered_export(
        profile,
        adapter,
        request,
        state,
        last_export_tick,
        media_type,
        payload,
        canonical.artifact_ref,
        sink,
    )
}

fn execute_rendered_export(
    profile: &ObservationProfile,
    adapter: &ObservationAdapterProfile,
    request: &AdapterDeliveryRequest,
    state: &ExportShellState,
    last_export_tick: Option<u64>,
    media_type: &str,
    payload: Vec<u8>,
    payload_ref: String,
    sink: &mut dyn ObservationSink,
) -> Result<ExportExecution> {
    validate_request_binding(request, &payload_ref, payload.len())?;
    let preflight_runtime = AdapterRuntimeObservation {
        available: state.available,
        queued_bytes: state.queued_bytes,
        completed_tick: request.submitted_tick,
        dropped_observations: 0,
        cancelled: state.cancelled,
        failure: None,
    };
    let preflight = evaluate_adapter_delivery(profile, adapter, request, &preflight_runtime, last_export_tick);
    if preflight.kind != AdapterOutcomeKind::Exported {
        return Ok(ExportExecution {
            media_type: media_type.to_string(),
            payload: Vec::new(),
            payload_ref,
            outcome: canonical_adapter_outcome(profile, adapter, &preflight)?,
        });
    }
    let completion = sink.emit(media_type, &payload, &payload_ref);
    let terminal_runtime = AdapterRuntimeObservation {
        available: state.available,
        queued_bytes: state.queued_bytes,
        completed_tick: completion.completed_tick,
        dropped_observations: completion.dropped_observations,
        cancelled: state.cancelled,
        failure: completion.failure,
    };
    let outcome = evaluate_adapter_delivery(profile, adapter, request, &terminal_runtime, last_export_tick);
    let exported_payload = if outcome.kind == AdapterOutcomeKind::Exported {
        payload
    } else {
        Vec::new()
    };
    Ok(ExportExecution {
        media_type: media_type.to_string(),
        payload: exported_payload,
        payload_ref,
        outcome: canonical_adapter_outcome(profile, adapter, &outcome)?,
    })
}

fn render_event(
    class: ObservationAdapterClass,
    canonical: &CanonicalArtifact<ObservationEvent>,
) -> Result<(&'static str, Vec<u8>)> {
    if class == ObservationAdapterClass::Tracing {
        return Ok((TRACING_MEDIA_TYPE, canonical.artifact_ref.as_bytes().to_vec()));
    }
    let attributes = canonical
        .artifact
        .attributes
        .iter()
        .map(|attribute| (attribute.name.clone(), attribute.value.clone()))
        .collect::<std::collections::BTreeMap<_, _>>();
    let payload = serde_json::to_vec(&json!({
        "schema": "molten.opentelemetry.event.v1",
        "event_ref": canonical.artifact_ref,
        "kind": canonical.artifact.event_kind,
        "severity": canonical.artifact.severity.as_str(),
        "detail": canonical.artifact.detail,
        "attributes": attributes,
        "observed_tick": canonical.artifact.context.observed_tick,
    }))
    .map_err(|error| MoltenError::invalid_harness(format!("OpenTelemetry event rendering failed: {error}")))?;
    Ok((OTLP_EVENT_MEDIA_TYPE, payload))
}

pub fn render_prometheus_snapshot(snapshot: &ObservationSnapshot) -> Result<Vec<u8>> {
    let mut output = String::new();
    for series in &snapshot.series {
        output.push_str("# TYPE ");
        output.push_str(&series.metric_name);
        output.push(' ');
        output.push_str(match series.kind {
            MetricKind::Counter => "counter",
            MetricKind::Gauge => "gauge",
        });
        output.push('\n');
        output.push_str(&series.metric_name);
        if !series.identity.labels.is_empty() {
            output.push('{');
            for (index, label) in series.identity.labels.iter().enumerate() {
                if index > 0 {
                    output.push(',');
                }
                output.push_str(&label.name);
                output.push_str("=\"");
                output.push_str(&escape_prometheus_label(&label.value));
                output.push('"');
            }
            output.push('}');
        }
        output.push(' ');
        output.push_str(&series.value.to_string());
        output.push('\n');
    }
    Ok(output.into_bytes())
}

pub fn render_opentelemetry_snapshot(snapshot: &ObservationSnapshot) -> Result<Vec<u8>> {
    let mut lines = Vec::with_capacity(snapshot.series.len());
    for series in &snapshot.series {
        let labels = series
            .identity
            .labels
            .iter()
            .map(|label| (label.name.clone(), label.value.clone()))
            .collect::<std::collections::BTreeMap<_, _>>();
        lines.push(
            serde_json::to_string(&json!({
                "schema": "molten.opentelemetry.metric.v1",
                "descriptor_ref": series.identity.descriptor_ref,
                "name": series.metric_name,
                "unit": series.unit,
                "kind": series.kind.as_str(),
                "aggregation": series.aggregation.as_str(),
                "value": series.value,
                "labels": labels,
                "sample_refs": series.source_sample_refs,
                "observed_tick": series.latest_observed_tick,
            }))
            .map_err(|error| MoltenError::invalid_harness(format!("OpenTelemetry JSON rendering failed: {error}")))?,
        );
    }
    let mut payload = lines.join("\n").into_bytes();
    if !payload.is_empty() {
        payload.push(b'\n');
    }
    Ok(payload)
}

pub struct DeterministicSimulationSink {
    completion_tick: u64,
    max_records: usize,
    emitted_refs: Vec<String>,
}

impl DeterministicSimulationSink {
    pub fn new(completion_tick: u64, max_records: usize) -> Result<Self> {
        if max_records == 0 {
            return Err(MoltenError::invalid_harness("deterministic observation sink record bound must be positive"));
        }
        Ok(Self {
            completion_tick,
            max_records,
            emitted_refs: Vec::with_capacity(max_records),
        })
    }

    pub fn emitted_refs(&self) -> &[String] {
        &self.emitted_refs
    }
}

impl ObservationSink for DeterministicSimulationSink {
    fn emit(&mut self, _media_type: &str, _payload: &[u8], payload_ref: &str) -> SinkCompletion {
        if self.emitted_refs.len() >= self.max_records {
            return SinkCompletion {
                completed_tick: self.completion_tick,
                dropped_observations: 1,
                failure: None,
            };
        }
        self.emitted_refs.push(payload_ref.to_string());
        SinkCompletion {
            completed_tick: self.completion_tick,
            dropped_observations: 0,
            failure: None,
        }
    }
}

pub struct TracingReferenceSink {
    completed_tick: u64,
}

impl TracingReferenceSink {
    pub const fn new(completed_tick: u64) -> Self {
        Self { completed_tick }
    }
}

impl ObservationSink for TracingReferenceSink {
    fn emit(&mut self, media_type: &str, _payload: &[u8], payload_ref: &str) -> SinkCompletion {
        tracing::info!(
            target: "molten_fabric_observability",
            media_type,
            canonical_observation_ref = payload_ref,
            "canonical fabric observation exported"
        );
        SinkCompletion {
            completed_tick: self.completed_tick,
            dropped_observations: 0,
            failure: None,
        }
    }
}

fn render_snapshot(
    format: ExportFormat,
    canonical: &CanonicalArtifact<ObservationSnapshot>,
) -> Result<(&'static str, Vec<u8>)> {
    match format {
        ExportFormat::Prometheus => Ok((PROMETHEUS_MEDIA_TYPE, render_prometheus_snapshot(&canonical.artifact)?)),
        ExportFormat::OpenTelemetryJson => {
            Ok((OTLP_JSON_MEDIA_TYPE, render_opentelemetry_snapshot(&canonical.artifact)?))
        }
        ExportFormat::TracingReference => Ok((TRACING_MEDIA_TYPE, canonical.artifact_ref.as_bytes().to_vec())),
    }
}

fn require_export_class(class: ObservationAdapterClass, format: ExportFormat) -> Result<()> {
    let matches = matches!(
        (class, format),
        (ObservationAdapterClass::Prometheus, ExportFormat::Prometheus)
            | (ObservationAdapterClass::OpenTelemetry, ExportFormat::OpenTelemetryJson)
            | (ObservationAdapterClass::Tracing, ExportFormat::TracingReference)
            | (ObservationAdapterClass::DeterministicSimulation, _)
    );
    if matches {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness("observation adapter class does not match requested exporter format"))
    }
}

fn validate_request_binding(
    request: &AdapterDeliveryRequest,
    expected_payload_ref: &str,
    payload_len: usize,
) -> Result<()> {
    let payload_bytes = u64::try_from(payload_len)
        .map_err(|_| MoltenError::invalid_harness("export payload length does not fit u64"))?;
    if request.payload_ref != expected_payload_ref || request.payload_bytes != payload_bytes {
        return Err(MoltenError::invalid_harness(
            "export request does not bind the canonical snapshot ref and rendered byte length",
        ));
    }
    Ok(())
}

fn escape_prometheus_label(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            '\n' => escaped.push_str("\\n"),
            other => escaped.push(other),
        }
    }
    escaped
}

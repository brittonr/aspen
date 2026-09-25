
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
) -> crate::error::Result<(&'static str, Vec<u8>)> {
    match format {
        ExportFormat::Prometheus => Ok((PROMETHEUS_MEDIA_TYPE, render_prometheus_snapshot(&canonical.artifact)?)),
        ExportFormat::OpenTelemetryJson => {
            Ok((OTLP_JSON_MEDIA_TYPE, render_opentelemetry_snapshot(&canonical.artifact)?))
        }
        ExportFormat::TracingReference => Ok((TRACING_MEDIA_TYPE, canonical.artifact_ref.as_bytes().to_vec())),
    }
}

fn require_export_class(class: ObservationAdapterClass, format: ExportFormat) -> crate::error::Result<()> {
    let is_matches = matches!(
        (class, format),
        (ObservationAdapterClass::Prometheus, ExportFormat::Prometheus)
            | (ObservationAdapterClass::OpenTelemetry, ExportFormat::OpenTelemetryJson)
            | (ObservationAdapterClass::Tracing, ExportFormat::TracingReference)
            | (ObservationAdapterClass::DeterministicSimulation, _)
    );
    if is_matches {
        Ok(())
    } else {
        Err(crate::error::MoltenError::invalid_harness(
            "observation adapter class does not match requested exporter format",
        ))
    }
}

fn validate_request_binding(
    request: &AdapterDeliveryRequest,
    expected_payload_ref: &str,
    payload_len: usize,
) -> crate::error::Result<()> {
    let payload_bytes = u64::try_from(payload_len)
        .map_err(|_| crate::error::MoltenError::invalid_harness("export payload length does not fit u64"))?;
    if request.payload_ref != expected_payload_ref || request.payload_bytes != payload_bytes {
        return Err(crate::error::MoltenError::invalid_harness(
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

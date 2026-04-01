## ADDED Requirements

### Requirement: OTLP metrics export

The system SHALL support pushing metrics to an OpenTelemetry Collector via OTLP when the `otlp` feature is enabled and `--otlp-endpoint` is configured. All metrics in the `metrics` crate registry SHALL be forwarded.

#### Scenario: Metrics pushed to OTLP collector

- **WHEN** `aspen-node` starts with `--otlp-endpoint http://collector:4317`
- **THEN** all `metrics::counter!`, `metrics::gauge!`, and `metrics::histogram!` observations SHALL be pushed to the OTLP gRPC endpoint at the configured push interval

#### Scenario: OTLP disabled by default

- **WHEN** `aspen-node` starts without `--otlp-endpoint`
- **THEN** no OTLP exporter SHALL be installed
- **AND** there SHALL be zero runtime overhead from the OTLP feature

#### Scenario: OTLP feature not compiled

- **WHEN** `aspen-node` is built without the `otlp` feature flag
- **THEN** the `--otlp-endpoint` CLI flag SHALL NOT be available
- **AND** no `opentelemetry-otlp` dependency SHALL be linked

### Requirement: OTLP trace export

The system SHALL support pushing distributed traces to an OTLP collector. Traces ingested via `TraceIngest` and traces generated internally SHALL both be forwarded.

#### Scenario: Ingested traces forwarded to collector

- **WHEN** a client sends `TraceIngest` with spans and OTLP export is enabled
- **THEN** the spans SHALL be stored in KV (existing behavior)
- **AND** the spans SHALL be forwarded to the OTLP collector

#### Scenario: RPC handler spans forwarded

- **WHEN** the RPC instrumentation layer creates a span for a request and OTLP is enabled
- **THEN** the span SHALL be exported to the OTLP collector with trace_id, operation name, duration, and handler name

### Requirement: OTLP resource attributes

All OTLP exports SHALL include resource attributes identifying the Aspen node: `service.name=aspen-node`, `service.instance.id=<node_id>`, `aspen.cluster.cookie=<cookie>`.

#### Scenario: Node identification in exported metrics

- **WHEN** the OTLP exporter sends a metric batch
- **THEN** the resource attributes SHALL include `service.name`, `service.instance.id`, and `aspen.cluster.cookie`

## ADDED Requirements

### Requirement: Structured metrics registry

The system SHALL use the `metrics` crate facade for all internal metric recording. A `PrometheusHandle` exporter SHALL be installed at node startup. The `GetMetrics` RPC SHALL return text generated from this registry instead of hardcoded format strings.

#### Scenario: Registry-generated Prometheus output

- **WHEN** a client sends `GetMetrics`
- **THEN** the response SHALL contain Prometheus text format output generated from the `metrics` registry
- **AND** the output SHALL include all metrics registered via `metrics::counter!`, `metrics::gauge!`, and `metrics::histogram!` calls across the codebase

#### Scenario: New metric automatically appears in export

- **WHEN** a new `metrics::counter!("aspen.foo.bar")` call is added to any crate
- **THEN** the metric SHALL appear in the `GetMetrics` Prometheus output without modifying the export handler

### Requirement: Metric naming convention

All metrics emitted by Aspen internals SHALL follow the naming convention `aspen.<subsystem>.<metric_name>` with snake_case. Labels SHALL use lowercase keys.

#### Scenario: Consistent naming

- **WHEN** the RPC handler emits a latency metric
- **THEN** the metric name SHALL be `aspen.rpc.duration_ms`, not `rpc_latency` or `aspen_rpc_duration_milliseconds`

#### Scenario: Prometheus export normalizes dots to underscores

- **WHEN** the Prometheus exporter renders `aspen.rpc.duration_ms`
- **THEN** the output SHALL contain `aspen_rpc_duration_ms` (dots normalized to underscores per Prometheus convention)

### Requirement: Self-emitted metrics use in-process registry only

Metrics emitted by the cluster itself (RPC latency, connection pool, write batcher, snapshots) SHALL be recorded in the in-process `metrics` registry. They SHALL NOT be written to KV via `MetricIngest` to avoid the observability system observing itself.

#### Scenario: Internal metrics available via Prometheus but not KV

- **WHEN** `aspen.rpc.duration_ms` is recorded
- **THEN** the metric SHALL appear in `GetMetrics` Prometheus output
- **AND** the metric SHALL NOT appear in `MetricQuery` results (which reads from KV)

#### Scenario: User-submitted metrics go to KV

- **WHEN** a client sends `MetricIngest` with custom application metrics
- **THEN** those metrics SHALL be stored in KV with TTL
- **AND** those metrics SHALL be queryable via `MetricQuery`

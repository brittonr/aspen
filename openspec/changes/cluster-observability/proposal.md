## Why

Aspen clusters are hard to debug under load. The 3-node dogfood runs exposed repeated blind spots: QUIC stream contention during git push caused 21-140ms node unreachability blips, 656MB snapshots silently exceeded limits, write forwarding failures appeared as "unexpected response type" with no latency data to isolate the cause. The existing observability stack has the wire types and storage (metrics, traces, alerts all exist in KV) but the cluster itself doesn't instrument its own internals. We're building the telescope but not pointing it at ourselves.

## What Changes

- Add per-RPC latency histograms and error counters at the `CoreHandler` dispatch layer, covering all 334 request types grouped by category
- Expose `ConnectionPoolMetrics` through the client API — connection health, stream counts, QUIC retries — queryable from CLI and TUI
- Instrument the write batcher: batch sizes, flush latency, forwarding counts, follower skip rate
- Track snapshot transfers: size, duration, frequency, success/failure per node pair
- Replace hand-rolled Prometheus `format!()` strings with a proper metrics registry that auto-exports all registered metrics
- Upgrade the TUI metrics view from the current 5-column node table to show latency sparklines, connection pool status, and alert state
- Add an OTLP export path so cluster metrics and traces can flow to external systems (Grafana, Jaeger) without polling

## Capabilities

### New Capabilities

- `rpc-instrumentation`: Per-handler latency histograms, error rate counters, request throughput tracking at the RPC dispatch layer
- `network-observability`: Connection pool metrics exposure, QUIC stream health, snapshot transfer tracking through the client API
- `metrics-registry`: Structured metrics registry replacing hardcoded Prometheus format strings, with auto-discovery and export
- `otlp-export`: OpenTelemetry protocol export for metrics and traces to external collectors
- `tui-observability`: TUI metrics view upgrade with sparklines, connection health, alert status, and latency percentiles

### Modified Capabilities

- `transport`: Connection pool metrics wired through to the client API (currently struct exists but is unexposed)
- `core`: `GetMetrics` handler uses the new registry instead of hardcoded format strings

## Impact

- **crates/aspen-rpc-core/**: Instrumentation middleware wrapping `RequestHandler::handle`
- **crates/aspen-core-essentials-handler/**: `CoreHandler` dispatch instrumentation, `GetMetrics` rewrite
- **crates/aspen-raft/**: Connection pool metrics exposure, write batcher counters, snapshot transfer tracking
- **crates/aspen-client-api/**: New RPC messages for connection pool and network metrics queries
- **crates/aspen-client/**: Client SDK methods for querying network metrics
- **crates/aspen-cli/**: New subcommands for network/connection diagnostics
- **crates/aspen-tui/**: Metrics view overhaul
- **crates/aspen-cluster/**: OTLP export configuration and bootstrap wiring
- **Dependencies**: `metrics` crate (or `prometheus-client`), `tracing-opentelemetry`, `opentelemetry-otlp`

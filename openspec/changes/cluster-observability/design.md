## Context

Aspen has a functioning observability data path: wire types for metrics, traces, and alerts exist in `aspen-client-api`, the `CoreHandler` processes ingest/query RPCs, data lives in KV with TTL, and the CLI has `metric`/`trace`/`alert` subcommands. The alert evaluator runs on the leader node.

What's missing is the cluster instrumenting *itself*. The Prometheus export is 9 hardcoded metrics in a `format!()` string. The connection pool tracks health/stream counts in a `ConnectionPoolMetrics` struct that nothing reads. The write batcher has a `size_bytes` field with a comment saying "not currently used." Snapshot transfers have no size/duration tracking. The TUI metrics view shows a static 5-column table.

The 3-node dogfood runs revealed this gap repeatedly: QUIC contention, snapshot size limits, write forwarding failures, and ReadIndex timeouts were all diagnosed from `tracing::warn!` output and napkin entries rather than metrics.

## Goals / Non-Goals

**Goals:**

- Every RPC request records latency and success/failure, queryable by operation category
- Connection pool health, stream counts, and QUIC retry stats are accessible from CLI and TUI
- Write batcher batch sizes, flush frequency, and forwarding rate are tracked
- Snapshot transfer size, duration, and outcome are recorded per transfer
- The Prometheus export endpoint is generated from a registry, not hardcoded
- The TUI metrics view shows time-series data (sparklines), connection health, and active alerts
- Metrics and traces can be pushed to an external OTLP collector

**Non-Goals:**

- Custom query language for metrics (PromQL-like) — use label filters and aggregation, not a full query engine
- Dashboarding UI beyond the TUI — external tools (Grafana) handle this via OTLP export
- Replacing `tracing` crate usage for structured logging — that stays as-is
- Per-key or per-tenant metrics isolation — metrics are cluster-scoped
- Historical analytics or long-term storage — TTL-based retention stays (max 7d)

## Decisions

### 1. Instrument at the `RequestHandler::handle` dispatch, not inside each handler

**Choice**: Add timing/counting in the `ClientProtocolHandler` dispatch loop that iterates registered handlers, wrapping each `handler.handle(request, ctx)` call.

**Why not per-handler**: There are 15+ handler implementations across feature-gated crates. Instrumenting each one means 15+ places to maintain, and new handlers silently skip instrumentation. The dispatch loop in `ClientProtocolHandler` is the single choke point — instrument once, cover everything.

**What gets recorded**: Operation name (from `ClientRpcRequest` variant name), handler name (from `RequestHandler::name()`), latency histogram, success/error counter.

**Alternative considered**: Middleware wrapper trait (decorator pattern around `RequestHandler`). Rejected because the `can_handle` → `handle` two-step in the dispatch loop makes wrapping awkward — you'd need to wrap the loop, not individual handlers.

### 2. Use the `metrics` crate facade, not `prometheus-client`

**Choice**: Use the [`metrics`](https://docs.rs/metrics) crate (facade pattern) with `metrics-exporter-prometheus` for the Prometheus text endpoint.

**Why**: The `metrics` crate is a facade (like `log` for logging) — call sites use `metrics::histogram!()` / `metrics::counter!()` macros that are zero-cost when no exporter is installed. This means instrumentation code compiles even without an exporter, tests don't need setup, and swapping exporters (Prometheus → OTLP → StatsD) is a one-line change at initialization.

**Alternative considered**: `prometheus-client` (the official Prometheus Rust client). Heavier — requires registry threading, type-safe metric families. Good for libraries exporting to Prometheus specifically, but we want multi-export (Prometheus + OTLP) and the facade pattern fits Aspen's plugin architecture.

### 3. Connection pool metrics exposed via a new `GetNetworkMetrics` RPC

**Choice**: Add a `GetNetworkMetrics` request/response pair to `aspen-client-api`. The handler calls `connection_pool.metrics()` (which already exists) and returns the data.

**Why not fold into `GetMetrics`**: `GetMetrics` returns Prometheus text format. Network diagnostics need structured data (per-peer connection state, stream counts per connection) that doesn't reduce well to flat counters. Keep them separate: `GetMetrics` for dashboards, `GetNetworkMetrics` for debugging.

### 4. Write batcher metrics via `metrics` crate counters in hot path

**Choice**: Add `metrics::counter!("aspen.write_batcher.flush_count")` / `metrics::histogram!("aspen.write_batcher.batch_size")` calls in `flush_batch()` and `write()`. The `size_bytes` field on `PendingWrite` that's currently unused feeds the batch size histogram.

**Why not a separate metrics struct**: The write batcher already has internal state (`BatcherState`). Adding another struct just for metrics creates synchronization overhead in a latency-sensitive path. `metrics` crate macros are lock-free atomic increments.

### 5. Snapshot transfer tracking via structured events

**Choice**: Emit `metrics::counter!` and `metrics::histogram!` at snapshot send/receive boundaries in the Raft network layer. Track: size_bytes, duration_ms, source_node, target_node, success/failure.

**Why not store in KV**: Snapshot transfers happen during Raft leadership changes and catch-up — exactly when the KV store might be unavailable or transitioning. Using the in-process metrics registry avoids circular dependency.

### 6. OTLP export as an optional feature flag

**Choice**: New `otlp` feature flag on `aspen-cluster` that pulls in `opentelemetry-otlp` + `metrics-exporter-opentelemetry`. When enabled and configured (`--otlp-endpoint`), metrics and traces are pushed to the collector. When disabled, zero overhead.

**Why feature-gated**: OTLP pulls in tonic/gRPC or reqwest/HTTP dependencies. Single-node deployments or testing don't need it. The `metrics` facade makes this clean — install the OTLP exporter at startup, all existing `metrics::*!()` calls automatically flow there.

### 7. TUI sparklines from metric query API

**Choice**: The TUI polls `MetricQuery` for recent data points (last 5 minutes, 1-second resolution) and renders with ratatui's `Sparkline` widget. Connection pool data comes from `GetNetworkMetrics`.

**Why not a streaming push**: The TUI already has a polling loop for cluster state. Adding a push channel would require a new ALPN or multiplexing on `TUI_ALPN`. Polling at 2-5s intervals is adequate for a human-readable dashboard and simpler to implement.

## Risks / Trade-offs

**[Metrics hot-path overhead]** → `metrics` crate macros compile to atomic increments (~2ns). Histogram recording is ~10ns. On a 3ms Raft write, this is noise. Benchmarked in `metrics` crate docs.

**[KV storage cost for self-metrics]** → RPC latency histograms stored in KV (`_sys:metrics:*`) grow with cardinality. Bound by: max 50 metric names auto-emitted, 24h TTL, 1-second resolution = ~4.3M data points/day worst case. At ~100 bytes each, that's ~430MB — near the limit. **Mitigation**: Self-emitted metrics use 10-second resolution (8,640 points/day/metric = ~43MB total). Critical metrics only; verbose per-operation histograms stay in-process (Prometheus scrape) and don't go to KV.

**[OTLP dependency weight]** → Feature-gated. Zero impact when disabled. When enabled, adds ~2MB to binary size (tonic + HTTP transport). Acceptable for production deployments that want external dashboards.

**[TUI polling load]** → One `MetricQuery` RPC per 2 seconds per visible metric. With 5 visible sparklines, that's 2.5 RPS of read traffic — trivial compared to normal workload. Uses follower reads when available.

## Open Questions

- Should self-emitted metrics (RPC latency, connection pool) bypass the normal `MetricIngest` RPC path and write directly to the metrics registry? This avoids the "observability system observing itself" loop but creates two ingest paths.
- What's the right histogram bucket layout for RPC latency? Raft writes are 2-5ms, reads are <1ms, snapshot transfers are seconds. A single bucket set won't fit all three well. Per-operation-category buckets add complexity.

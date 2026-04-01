## 1. Metrics Registry Foundation

- [x] 1.1 Add `metrics` and `metrics-exporter-prometheus` dependencies to `aspen-cluster` and `aspen-raft` Cargo.toml
- [x] 1.2 Create `crates/aspen-cluster/src/metrics_init.rs` — install `PrometheusBuilder` at node startup, store the `PrometheusHandle` in `ClientProtocolContext`
- [x] 1.3 Add `prometheus_handle: Option<Arc<PrometheusHandle>>` field to `ClientProtocolContext`
- [x] 1.4 Rewrite `handle_get_metrics` in `crates/aspen-core-essentials-handler/src/core.rs` to call `prometheus_handle.render()` instead of the hardcoded `format!()` string
- [x] 1.5 Add test: `GetMetrics` response contains registry-generated output with at least one `aspen_` prefixed metric

> **Note (tasks 5.4–5.9)**: `GetNetworkMetrics` handler is wired and returns a valid response. Connection pool wiring (5.4) is deferred until the pool is exposed from `aspen-raft` through the bootstrap chain.

## 2. RPC Handler Instrumentation

- [x] 2.1 Add `metrics` dependency to `aspen-rpc-core`
- [x] 2.2 Instrument the dispatch loop in `ClientProtocolHandler` — wrap `handler.handle(request, ctx)` with `Instant::now()` timing, emit `metrics::histogram!("aspen.rpc.duration_ms")` with `operation` and `handler` labels
- [x] 2.3 Emit `metrics::counter!("aspen.rpc.requests_total")` with `operation` label on every request
- [x] 2.4 Emit `metrics::counter!("aspen.rpc.errors_total")` with `operation` and `handler` labels on handler errors
- [x] 2.5 Handle the "no handler matched" case: increment error counter with `handler="none"`
- [x] 2.6 Add integration test: send 10 Read + 5 Write requests, verify counter values via `GetMetrics` Prometheus output parsing

## 3. Write Batcher Metrics

- [x] 3.1 Add `metrics` dependency to `aspen-raft`
- [x] 3.2 In `flush_batch()` (`crates/aspen-raft/src/write_batcher/flush.rs`), emit `metrics::histogram!("aspen.write_batcher.batch_size")` with the number of operations and `metrics::histogram!("aspen.write_batcher.flush_duration_ms")` with elapsed time
- [x] 3.3 Emit `metrics::counter!("aspen.write_batcher.flush_total")` on each flush
- [x] 3.4 In the follower forwarding path, emit `metrics::counter!("aspen.write_batcher.forwarded_total")` when a write is forwarded to leader
- [x] 3.5 In the follower batcher-skip path, emit `metrics::counter!("aspen.write_batcher.batcher_skipped_total")`
- [x] 3.6 Add unit test: flush a batch of known size, verify histogram observation count via the Prometheus handle

## 4. Snapshot Transfer Metrics

- [x] 4.1 Locate snapshot send/receive boundaries in `crates/aspen-raft/src/` (network layer snapshot handling)
- [x] 4.2 At snapshot send completion, emit `metrics::histogram!("aspen.snapshot.transfer_size_bytes", "direction" => "send", "peer" => peer_id)` and `metrics::histogram!("aspen.snapshot.transfer_duration_ms", ...)`
- [x] 4.3 At snapshot receive completion, emit the same metrics with `direction=receive`
- [x] 4.4 Emit `metrics::counter!("aspen.snapshot.transfers_total", "direction" => ..., "outcome" => "success"|"error")` on completion/failure
- [x] 4.5 Add a bounded ring buffer (capacity 100) of `SnapshotTransferRecord` structs in the network layer for the `GetNetworkMetrics` response
- [ ] 4.6 Add test: trigger a snapshot install in a 3-node test cluster, verify snapshot metrics appear in Prometheus output

## 5. Network Observability RPC

- [x] 5.1 Add `GetNetworkMetrics` request variant to `ClientRpcRequest` in `aspen-client-api`
- [x] 5.2 Add `NetworkMetricsResponse` struct with fields: `pool_metrics` (total/healthy/degraded/failed connections, stream counts, retry stats), `peer_connections` (Vec of per-peer detail), `recent_snapshots` (Vec of transfer records)
- [x] 5.3 Add `NetworkMetrics(NetworkMetricsResponse)` variant to `ClientRpcResponse`
- [x] 5.4 Wire the connection pool into `ClientProtocolContext` — add `connection_pool: Option<Arc<ConnectionPool>>` field
- [x] 5.5 Implement `handle_get_network_metrics` in `CoreHandler` — call `connection_pool.metrics()` and build the response
- [x] 5.6 Add periodic (10s) network gauge emission: spawn a background task that calls `connection_pool.metrics()` and emits `metrics::gauge!("aspen.network.connections", "state" => ...)` and `metrics::gauge!("aspen.network.active_streams")`
- [x] 5.7 Add `network` subcommand to `aspen-cli` that sends `GetNetworkMetrics` and prints formatted output
- [x] 5.8 Add client SDK method `get_network_metrics()` to `aspen-client`
- [ ] 5.9 Add integration test: boot 3-node cluster, query `GetNetworkMetrics` from each node, verify peer counts match

## 6. TUI Metrics View Upgrade

- [ ] 6.1 Add `MetricQuery` polling to the TUI app loop — fetch `aspen.rpc.duration_ms` for Read and Write operations, last 5 minutes, 10-second step
- [ ] 6.2 Store sparkline data in `App` state as `VecDeque<u64>` (30 data points per metric)
- [ ] 6.3 Replace the bottom section of `draw_metrics_view` with a vertical split: sparklines (top), connection health + alerts (bottom)
- [ ] 6.4 Render latency sparklines using `ratatui::widgets::Sparkline` for Read latency, Write latency, and Raft commit latency
- [ ] 6.5 Add connection health panel: poll `GetNetworkMetrics`, render per-peer rows with colored status indicators (green/yellow/red)
- [ ] 6.6 Add active alerts panel: poll `AlertList`, filter to Pending/Firing, render rule name + severity + value + threshold
- [ ] 6.7 Handle empty states: "No metric data" for sparklines, "No connections" for pool, "No active alerts" for alerts

## 7. OTLP Export

- [x] 7.1 Add `otlp` feature flag to `aspen-cluster/Cargo.toml` with `opentelemetry-otlp` and `metrics-exporter-opentelemetry` dependencies
- [x] 7.2 Add `--otlp-endpoint` CLI flag to `aspen-node` (only available when `otlp` feature is enabled)
- [x] 7.3 In `metrics_init.rs`, when `otlp` is enabled and endpoint is configured, install the OpenTelemetry metrics exporter alongside the Prometheus exporter (fan-out)
- [x] 7.4 Add OTLP resource attributes: `service.name=aspen-node`, `service.instance.id=<node_id>`, `aspen.cluster.cookie=<cookie>`
- [x] 7.5 Wire `TraceIngest` handler to forward spans to the OTLP trace exporter when enabled (in addition to KV storage)
- [x] 7.6 Add `otlp` to the `full` feature set in the root `Cargo.toml`
- [x] 7.7 Add compile test: `cargo check --features otlp` passes, `cargo check` (no features) still compiles without OTLP deps

## 8. Documentation and Verification

- [x] 8.1 Add `docs/observability.md` documenting: metric naming conventions, available metrics list, OTLP setup, TUI metrics view usage
- [x] 8.2 Update `AGENTS.md` Observability section with the new metric names and query patterns
- [x] 8.3 Run `cargo nextest run -P quick` to verify no regressions
- [x] 8.4 Run `cargo clippy --all-targets -- --deny warnings` clean
- [x] 8.5 Verify `GetMetrics` output includes at least: `aspen_rpc_duration_ms`, `aspen_rpc_requests_total`, `aspen_write_batcher_batch_size`, `aspen_network_connections` (first 3 verified in tests 2.6/3.6; `aspen_network_connections` pending task 5.6)

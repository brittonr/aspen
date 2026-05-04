# Observability

Aspen clusters emit metrics, traces, and alerts through a layered observability stack.

## Metrics

### Architecture

Internal metrics use the [`metrics`](https://docs.rs/metrics) crate facade. A `PrometheusBuilder` recorder is installed at node startup. All `metrics::counter!()`, `metrics::gauge!()`, and `metrics::histogram!()` calls across the codebase are captured by this recorder and rendered on demand via `GetMetrics`.

User-submitted metrics (via `MetricIngest`) are stored in KV with TTL and queried via `MetricQuery`. Internal metrics stay in-process only — they appear in Prometheus output but not in KV queries.

### Naming Convention

All internal metrics follow `aspen.<subsystem>.<metric_name>` with snake_case:

```
aspen.rpc.duration_ms          # histogram
aspen.rpc.requests_total       # counter
aspen.rpc.errors_total         # counter
aspen.write_batcher.batch_size # histogram
aspen.raft.term                # gauge
```

Prometheus export normalizes dots to underscores: `aspen.rpc.duration_ms` → `aspen_rpc_duration_ms`.

### Available Metrics

#### RPC Instrumentation

| Metric | Type | Labels | Description |
|--------|------|--------|-------------|
| `aspen.rpc.requests_total` | counter | `operation` | Total RPC requests by operation name |
| `aspen.rpc.duration_ms` | histogram | `operation`, `handler` | Per-request latency in milliseconds |
| `aspen.rpc.errors_total` | counter | `operation`, `handler` | Failed requests by operation and handler |

The RPC `operation` label uses `ClientRpcRequest::variant_name()`, so Prometheus series names stay aligned with the wire/API request variant names rather than handler-local aliases.

#### Raft State

| Metric | Type | Labels | Description |
|--------|------|--------|-------------|
| `aspen.raft.term` | gauge | `node_id` | Current Raft term |
| `aspen.raft.state` | gauge | `node_id` | Raft state (0=Learner, 1=Follower, 2=Candidate, 3=Leader) |
| `aspen.raft.is_leader` | gauge | `node_id` | 1 if leader, 0 otherwise |
| `aspen.raft.last_log_index` | gauge | `node_id` | Last log index |
| `aspen.raft.last_applied_index` | gauge | `node_id` | Last applied log index |
| `aspen.raft.snapshot_index` | gauge | `node_id` | Snapshot index |
| `aspen.node.uptime_seconds` | gauge | `node_id` | Node uptime |

#### Write Batcher

| Metric | Type | Labels | Description |
|--------|------|--------|-------------|
| `aspen.write_batcher.batch_size` | histogram | — | Number of operations per batch flush |
| `aspen.write_batcher.flush_duration_ms` | histogram | — | Time to flush a batch through Raft |
| `aspen.write_batcher.flush_total` | counter | — | Total batch flushes |
| `aspen.write_batcher.forwarded_total` | counter | — | Writes forwarded from follower to leader |
| `aspen.write_batcher.batcher_skipped_total` | counter | — | Follower writes that skipped the batcher |

#### Snapshot Transfers

| Metric | Type | Labels | Description |
|--------|------|--------|-------------|
| `aspen.snapshot.transfer_size_bytes` | histogram | `direction`, `peer` | Snapshot size |
| `aspen.snapshot.transfer_duration_ms` | histogram | `direction`, `peer` | Transfer duration |
| `aspen.snapshot.transfers_total` | counter | `direction`, `outcome` | Transfer count (success/error) |

#### Iroh Endpoint (bridged)

Iroh's internal metrics are sampled every 10 seconds and bridged into the Prometheus registry under the `iroh.` prefix:

| Metric | Type | Description |
|--------|------|-------------|
| `iroh.socket.send_ipv4` | gauge | IPv4 packets sent |
| `iroh.socket.send_ipv6` | gauge | IPv6 packets sent |
| `iroh.socket.send_relay` | gauge | Packets sent via relay |
| `iroh.socket.recv_data_ipv4` | gauge | IPv4 data packets received |
| `iroh.socket.recv_data_relay` | gauge | Relay data packets received |
| `iroh.socket.recv_datagrams` | gauge | QUIC datagrams received |
| `iroh.socket.num_conns_opened` | gauge | Connections opened |
| `iroh.socket.num_conns_closed` | gauge | Connections closed |
| `iroh.socket.paths_direct` | gauge | Direct (holepunched) paths |
| `iroh.socket.paths_relay` | gauge | Relayed paths |
| `iroh.socket.holepunch_attempts` | gauge | Holepunch attempts |
| `iroh.socket.relay_home_change` | gauge | Relay home changes |
| `iroh.net_report.reports` | gauge | Net reports executed |
| `iroh.portmapper.*` | gauge | Portmapper metrics |

These are emitted as gauges (point-in-time counter snapshots) since iroh uses its own `iroh_metrics::Counter` system rather than the `metrics` crate.

### Querying Metrics

```bash
# Prometheus text format (all registered metrics)
aspen-cli metrics

# Query user-submitted metrics from KV
aspen-cli metric query --name "my.app.latency" --start "1h ago"

# List registered metric names
aspen-cli metric list
```

### Network Diagnostics

```bash
# Connection pool health, per-peer detail, recent snapshots
aspen-cli network
```

## Traces

Distributed tracing uses W3C Trace Context. Spans are ingested via `TraceIngest` and stored in KV.

```bash
aspen-cli trace list --limit 10
aspen-cli trace get <trace_id>
aspen-cli trace search --operation "kv.write" --min-duration 5ms
```

## Alerts

Alert rules monitor metric thresholds with state machine transitions: Ok → Pending → Firing → Ok.

```bash
aspen-cli alert create --name high-latency --metric aspen.rpc.duration_ms \
  --threshold 10 --comparison greater-than --severity warning

aspen-cli alert list
aspen-cli alert get high-latency
```

The alert evaluator runs on the leader node at a configurable interval (default 60s).

## TUI

The TUI metrics view (key `2`) shows cluster summary, node table, and (when available) latency sparklines, connection health, and active alerts.

## ADDED Requirements

### Requirement: Connection pool metrics queryable via RPC

The system SHALL expose connection pool metrics through a `GetNetworkMetrics` RPC request. The response SHALL include per-peer connection health, total/healthy/degraded/failed connection counts, active stream counts, and QUIC retry statistics.

#### Scenario: Query connection pool from CLI

- **WHEN** a client sends a `GetNetworkMetrics` request
- **THEN** the response SHALL contain `total_connections`, `healthy_connections`, `degraded_connections`, `failed_connections`, `total_active_streams`, `raft_streams_opened`, `bulk_streams_opened`, `read_index_retry_count`, and `read_index_retry_success_count`

#### Scenario: Per-peer connection detail

- **WHEN** a client sends `GetNetworkMetrics` and the node has connections to peers 2 and 3
- **THEN** the response SHALL include per-peer entries with `peer_id`, `connection_state` (healthy/degraded/failed), `active_streams`, and `last_activity_us`

### Requirement: Snapshot transfer history

The system SHALL maintain a bounded ring buffer (last 100 entries) of snapshot transfer records, queryable via the `GetNetworkMetrics` response.

#### Scenario: Recent snapshot transfers visible

- **WHEN** node 1 has completed 3 snapshot transfers in the last hour
- **THEN** `GetNetworkMetrics` response SHALL include a `recent_snapshots` list with entries containing `peer_id`, `direction` (send/receive), `size_bytes`, `duration_ms`, `outcome` (success/error), and `timestamp_us`

#### Scenario: Ring buffer bounds transfers

- **WHEN** more than 100 snapshot transfers have occurred
- **THEN** the `recent_snapshots` list SHALL contain only the most recent 100 entries

### Requirement: Network metrics in metrics registry

The system SHALL periodically (every 10 seconds) sample connection pool state and emit gauge metrics to the `metrics` crate registry for Prometheus export.

#### Scenario: Connection counts as gauges

- **WHEN** the node has 2 healthy and 1 degraded connection
- **THEN** the Prometheus export SHALL include `aspen_network_connections{state="healthy"} 2` and `aspen_network_connections{state="degraded"} 1`

#### Scenario: Active stream count as gauge

- **WHEN** the node has 5 active QUIC streams
- **THEN** the Prometheus export SHALL include `aspen_network_active_streams 5`

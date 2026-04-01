## ADDED Requirements

### Requirement: Connection pool metrics exposure

The transport layer SHALL expose connection pool metrics through a queryable interface. The `ConnectionPoolMetrics` struct SHALL be accessible from the `ClientProtocolContext` so RPC handlers can read and return it.

#### Scenario: Handler reads connection pool metrics

- **WHEN** the `GetNetworkMetrics` handler calls `connection_pool.metrics()`
- **THEN** the returned `ConnectionPoolMetrics` SHALL reflect the current state of all peer connections including healthy/degraded/failed counts and stream statistics

#### Scenario: Per-peer connection detail available

- **WHEN** the connection pool has entries for peers 2 and 3
- **THEN** the metrics interface SHALL provide per-peer detail: peer ID, connection health state, active stream count, and last activity timestamp

### Requirement: Snapshot transfer event emission

The Raft network layer SHALL emit `metrics` crate observations for every snapshot transfer at the send and receive boundaries.

#### Scenario: Snapshot send emits metrics

- **WHEN** the Raft network layer completes sending a snapshot
- **THEN** it SHALL record `aspen.snapshot.transfer_size_bytes` and `aspen.snapshot.transfer_duration_ms` histograms with `direction=send` and `peer=<target_node_id>` labels

#### Scenario: Snapshot receive emits metrics

- **WHEN** the Raft network layer completes receiving a snapshot
- **THEN** it SHALL record the same histogram metrics with `direction=receive` and `peer=<source_node_id>` labels

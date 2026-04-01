## ADDED Requirements

### Requirement: TUI latency sparklines

The TUI metrics view SHALL display sparkline charts for key latency metrics: RPC read latency, RPC write latency, and Raft commit latency. Sparklines SHALL show the last 5 minutes of data at 10-second granularity (30 data points).

#### Scenario: Sparkline renders recent latency

- **WHEN** the user switches to the Metrics view (key '2')
- **THEN** the view SHALL display sparkline widgets for `aspen.rpc.duration_ms{operation="Read"}`, `aspen.rpc.duration_ms{operation="Write"}`, and `aspen.raft.commit_duration_ms`
- **AND** each sparkline SHALL show 30 data points covering the last 5 minutes

#### Scenario: Sparkline updates on poll

- **WHEN** the TUI polls for new metric data (every 5 seconds)
- **THEN** the sparklines SHALL shift left and append the latest data point

### Requirement: TUI connection health panel

The TUI metrics view SHALL display a connection health panel showing per-peer connection state, active stream counts, and recent snapshot transfers.

#### Scenario: Healthy cluster connections

- **WHEN** the node has healthy connections to 2 peers
- **THEN** the connection panel SHALL show each peer with a green status indicator, peer ID, active stream count, and last activity timestamp

#### Scenario: Degraded connection highlighted

- **WHEN** a connection to peer 3 is in degraded state
- **THEN** the connection panel SHALL show peer 3 with a yellow status indicator

### Requirement: TUI active alerts panel

The TUI metrics view SHALL display active alerts (status Pending or Firing) in a panel below the sparklines.

#### Scenario: Firing alert displayed

- **WHEN** alert rule "high-rpc-latency" is in Firing state with value 15.2ms > threshold 10ms
- **THEN** the alerts panel SHALL display the rule name, severity, current value, threshold, and time since firing

#### Scenario: No active alerts

- **WHEN** no alert rules are in Pending or Firing state
- **THEN** the alerts panel SHALL display "No active alerts" in a dimmed style

### Requirement: TUI metrics view layout

The TUI metrics view SHALL be organized in a vertical layout: cluster summary (existing, top), sparklines (middle), connection health + alerts (bottom split).

#### Scenario: Full metrics view composition

- **WHEN** the user views the Metrics tab
- **THEN** the layout SHALL be: cluster summary (8 lines) → latency sparklines (12 lines) → connection health (left half of remaining) + active alerts (right half of remaining)

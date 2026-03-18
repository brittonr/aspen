## ADDED Requirements

### Requirement: Workers start as not ready

A newly registered worker SHALL have `is_ready = false` until Raft log catchup is confirmed. A non-ready worker SHALL have zero available capacity for job dispatch.

#### Scenario: Fresh worker registration

- **WHEN** a worker registers with the `DistributedWorkerCoordinator`
- **THEN** its `is_ready` field SHALL be `false`
- **AND** `available_capacity()` SHALL return `0.0`

#### Scenario: Non-ready worker excluded from dispatch

- **WHEN** the job dispatcher evaluates workers for a new job
- **AND** a worker has `is_ready = false`
- **THEN** that worker SHALL NOT be selected for job dispatch

### Requirement: Workers become ready after Raft catchup

A worker SHALL transition to `is_ready = true` when its heartbeat reports a Raft log lag below `LEARNER_LAG_THRESHOLD` (100 entries). The readiness check SHALL use the existing `is_learner_caught_up` verified function from `aspen-raft`.

#### Scenario: Worker catches up

- **WHEN** a worker's heartbeat reports `raft_log_lag = Some(50)`
- **AND** `LEARNER_LAG_THRESHOLD` is 100
- **THEN** the coordinator SHALL set `is_ready = true`
- **AND** subsequent `available_capacity()` calls SHALL return a value based on load (not 0.0)

#### Scenario: Worker still lagging

- **WHEN** a worker's heartbeat reports `raft_log_lag = Some(200)`
- **AND** `LEARNER_LAG_THRESHOLD` is 100
- **THEN** `is_ready` SHALL remain `false`

#### Scenario: Unknown lag

- **WHEN** a worker's heartbeat reports `raft_log_lag = None`
- **THEN** `is_ready` SHALL remain `false`

### Requirement: Readiness resets on health loss

When a worker transitions to `HealthStatus::Unhealthy` (missed heartbeats), its `is_ready` field SHALL reset to `false`. The worker SHALL re-earn readiness by reporting lag below threshold after recovery.

#### Scenario: Worker loses health and recovers

- **WHEN** a ready worker misses heartbeats and becomes `Unhealthy`
- **THEN** `is_ready` SHALL be set to `false`
- **AND WHEN** the worker resumes heartbeating with `raft_log_lag = Some(10)`
- **THEN** `is_ready` SHALL be set back to `true`

### Requirement: Heartbeat carries Raft log lag

The `WorkerStats` sent during heartbeat SHALL include `raft_log_lag: Option<u64>`. The value SHALL be computed by reading Raft metrics from `ClusterController::get_metrics()` and comparing the leader's last log index against the local node's matched index using `compute_learner_lag`.

#### Scenario: Metrics available

- **WHEN** `get_metrics()` returns valid Raft metrics with replication data
- **THEN** the heartbeat SHALL include `raft_log_lag = Some(lag)` where lag is computed via `compute_learner_lag`

#### Scenario: Metrics unavailable

- **WHEN** `get_metrics()` fails or returns no replication data
- **THEN** the heartbeat SHALL include `raft_log_lag = None`

### Requirement: Verified readiness function

A pure function `is_worker_ready(lag: Option<u64>, threshold: u64, is_healthy: bool) -> bool` SHALL exist in `crates/aspen-coordination/src/verified/worker.rs`. It SHALL return `true` only when `is_healthy` is `true` AND `lag` is `Some(n)` where `n < threshold`.

#### Scenario: All conditions met

- **WHEN** `is_worker_ready(Some(50), 100, true)` is called
- **THEN** the result SHALL be `true`

#### Scenario: Unhealthy worker

- **WHEN** `is_worker_ready(Some(0), 100, false)` is called
- **THEN** the result SHALL be `false`

#### Scenario: Lag at threshold

- **WHEN** `is_worker_ready(Some(100), 100, true)` is called
- **THEN** the result SHALL be `false` (threshold is exclusive)

#### Scenario: No lag data

- **WHEN** `is_worker_ready(None, 100, true)` is called
- **THEN** the result SHALL be `false`

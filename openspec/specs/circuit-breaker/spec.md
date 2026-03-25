## ADDED Requirements

### Requirement: Generic circuit breaker state machine

The system SHALL provide a `CircuitBreaker` struct in `aspen-core` that implements a three-state circuit breaker: closed (normal), open (rejecting), and half-open (probing).

#### Scenario: Stays closed under threshold

- **WHEN** fewer than `threshold` consecutive failures are recorded
- **THEN** `record_failure()` SHALL return `false` (do not reject)
- **AND** `is_open()` SHALL return `false`

#### Scenario: Trips open at threshold

- **WHEN** `threshold` consecutive failures are recorded (default: 5)
- **THEN** `record_failure()` SHALL return `true` (reject)
- **AND** `is_open()` SHALL return `true`
- **AND** the breaker SHALL stay open for `open_duration` (default: 30s)

#### Scenario: Success closes the breaker

- **WHEN** `record_success()` is called while the breaker is open
- **THEN** the breaker SHALL transition to closed
- **AND** the consecutive failure counter SHALL reset to 0

#### Scenario: Auto-close after timeout

- **WHEN** `open_duration` elapses without any calls
- **THEN** `is_open()` SHALL return `false` (auto-closed)

#### Scenario: Saturating failure counter

- **WHEN** failures are recorded beyond `u32::MAX`
- **THEN** the counter SHALL saturate at `u32::MAX` (no overflow)

### Requirement: Circuit breaker observability

State transitions SHALL be observable via metrics and logging.

#### Scenario: Open transition emits metric

- **WHEN** the breaker transitions from closed to open
- **THEN** a counter metric `aspen_{component}_circuit_open_total` SHALL be incremented
- **AND** a warn-level log SHALL be emitted with the failure count and open duration

#### Scenario: Close transition logs recovery

- **WHEN** the breaker transitions from open to closed via `record_success()`
- **THEN** an info-level log SHALL be emitted indicating recovery

### Requirement: Circuit breaker on snix service calls

The `IrohBlobService`, `RaftDirectoryService`, and `RaftPathInfoService` SHALL use circuit breakers to guard their underlying storage calls.

#### Scenario: Blob store degraded

- **WHEN** iroh-blobs returns errors for 5 consecutive blob operations
- **THEN** subsequent blob operations SHALL fail immediately with a circuit-open error
- **AND** the error message SHALL indicate the breaker is open

#### Scenario: Raft KV degraded

- **WHEN** Raft KV returns errors for 5 consecutive directory or pathinfo operations
- **THEN** subsequent operations SHALL fail immediately with a circuit-open error

### Requirement: Circuit breaker on castore irpc client

The `IrpcBlobService` and `IrpcDirectoryService` clients SHALL use circuit breakers to avoid flooding an unreachable remote node.

#### Scenario: Remote castore node unreachable

- **WHEN** the irpc client fails to connect or receive responses 5 consecutive times
- **THEN** subsequent calls SHALL fail immediately with a circuit-open error
- **UNTIL** the open_duration elapses or a probe succeeds

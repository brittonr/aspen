## ADDED Requirements

### Requirement: Queue-depth hysteresis on castore server

The `CastoreServer` irpc handler SHALL implement hysteresis-based backpressure using queue depth as the signal.

#### Scenario: Activate rejection at high water mark

- **WHEN** the internal message queue reaches 80% of its capacity
- **THEN** new incoming requests SHALL be rejected with a backpressure error
- **AND** a warn-level log SHALL be emitted

#### Scenario: Deactivate rejection at low water mark

- **WHEN** the queue depth drops below 60% of capacity after being in rejection mode
- **THEN** new incoming requests SHALL be accepted again
- **AND** an info-level log SHALL be emitted

#### Scenario: No oscillation near threshold

- **WHEN** the queue depth fluctuates between 60% and 80%
- **THEN** the system SHALL NOT oscillate between accepting and rejecting
- **AND** the current mode (accepting or rejecting) SHALL persist until the opposing threshold is crossed

### Requirement: Backpressure observability

Queue depth and rejection state SHALL be observable.

#### Scenario: Queue depth metric

- **WHEN** the castore server is running
- **THEN** a gauge metric `aspen_castore_queue_depth` SHALL reflect the current queue depth

#### Scenario: Rejection metric

- **WHEN** a request is rejected due to backpressure
- **THEN** a counter metric `aspen_castore_backpressure_rejections_total` SHALL be incremented

### Requirement: Backpressure error propagation

Rejected requests SHALL return an error that clients can distinguish from other failures.

#### Scenario: Client receives backpressure signal

- **WHEN** a castore irpc client receives a backpressure rejection
- **THEN** the error SHALL be distinguishable from connection failures, timeouts, or application errors
- **AND** the client MAY retry after a brief delay

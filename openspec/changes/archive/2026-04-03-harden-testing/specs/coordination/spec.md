## MODIFIED Requirements

### Requirement: Distributed Locks

The system SHALL provide distributed locks with fencing tokens, configurable TTL, and automatic expiration. At most one holder SHALL exist at any given time.

#### Scenario: Acquire and release

- GIVEN no lock exists for key `"my-lock"`
- WHEN client A acquires the lock with TTL 30 seconds
- THEN client A SHALL receive a `LockGuard` with a fencing token ≥ 1
- AND a subsequent acquire by client B SHALL fail until client A releases or the lock expires

#### Scenario: Fencing token monotonicity

- GIVEN a lock has been acquired and released 5 times
- WHEN the 6th acquisition occurs
- THEN the fencing token SHALL be strictly greater than all previous tokens

#### Scenario: TTL expiration

- GIVEN client A holds a lock with TTL 10 seconds
- WHEN 10 seconds pass without renewal
- THEN the lock SHALL be considered expired
- AND client B SHALL be able to acquire it

#### Scenario: Proptest model-based lock verification

- GIVEN a proptest model tracking lock state (holder, fencing token, expiry)
- WHEN arbitrary sequences of acquire, release, renew, and expire operations are generated
- THEN the model and the real implementation SHALL agree on lock holder identity, fencing token value, and expiry status after every operation

## ADDED Requirements

### Requirement: Proptest model-based queue verification

The system SHALL include proptest model-based tests for distributed queues that verify enqueue, dequeue, and acknowledge operations against an in-memory model.

#### Scenario: Queue ordering preserved under random operations

- **WHEN** proptest generates a sequence of enqueue and dequeue operations
- **THEN** dequeued items SHALL appear in FIFO order matching the model
- **AND** acknowledged items SHALL not be re-delivered

#### Scenario: Queue state consistent after crash replay

- **WHEN** proptest generates a sequence of operations with simulated crashes at random points
- **THEN** replaying the committed operations SHALL produce a queue state matching the model

### Requirement: Proptest model-based barrier verification

The system SHALL include proptest model-based tests for distributed barriers that verify participant join, wait, and release semantics.

#### Scenario: Barrier releases all participants at threshold

- **WHEN** proptest generates N participants joining a barrier with threshold N
- **THEN** all N participants SHALL be released simultaneously
- **AND** no participant SHALL be released before the threshold is met

### Requirement: Proptest model-based election verification

The system SHALL include proptest model-based tests for leader elections that verify candidacy, term advancement, and leader uniqueness.

#### Scenario: At most one leader per term

- **WHEN** proptest generates concurrent election attempts across multiple candidates
- **THEN** at most one candidate SHALL win per term
- **AND** term numbers SHALL be monotonically increasing

### Requirement: Proptest model-based rate limiter verification

The system SHALL include proptest model-based tests for distributed rate limiters that verify token consumption and refill against a model.

#### Scenario: Rate limiter respects configured limits

- **WHEN** proptest generates request sequences with varying timestamps
- **THEN** the number of allowed requests in any window SHALL not exceed the configured limit
- **AND** the implementation SHALL match the model's allow/deny decisions

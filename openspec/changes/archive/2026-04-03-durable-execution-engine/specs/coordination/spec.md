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

## ADDED Requirements

### Requirement: Durable timers as workflow primitives

The `DurableTimerManager` SHALL be usable as a first-class coordination primitive by the `DurableWorkflowExecutor`. Timers scheduled through the executor SHALL be persisted in the KV store, recoverable after node restarts, and integrated with the workflow event recording lifecycle.

#### Scenario: Timer survives leader failover

- GIVEN a `DurableTimer` scheduled via `DurableTimerManager` with fire_at_ms in 10 seconds
- WHEN the leader node is killed before the timer fires
- AND a new leader is elected
- THEN the `TimerService` on the new leader SHALL detect the timer via its KV scan
- AND the timer SHALL fire within the poll interval (100ms) of becoming ready

#### Scenario: Timer cancellation removes KV entry

- GIVEN a `DurableTimer` that has been scheduled and persisted in KV
- WHEN the timer is cancelled via `cancel_timer()`
- THEN the KV entry SHALL be deleted
- AND the timer SHALL NOT fire

#### Scenario: Workflow timer integrated with event recording

- GIVEN a durable workflow that calls `sleep(duration)`
- WHEN the executor processes the sleep
- THEN it SHALL call `DurableTimerManager::schedule_timer()` to persist the timer
- AND it SHALL record a `TimerScheduled` event in the workflow's event store
- AND when the timer fires, it SHALL record a `TimerFired` event

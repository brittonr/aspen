# Coordination Specification

## Purpose

Distributed coordination primitives built on top of Raft consensus. Provides locks, leader elections, barriers, queues, semaphores, sequence generators, rate limiters, and watch/notify. All primitives are linearizable and formally verified with Verus.

## Requirements

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

### Requirement: Leader Elections

The system SHALL provide leader election with lease-based leadership, heartbeats, and automatic failover.

#### Scenario: Elect leader

- GIVEN 3 candidates contend for election `"scheduler"`
- WHEN the election runs
- THEN exactly one candidate SHALL be elected leader
- AND the leader SHALL maintain its position via periodic heartbeats

#### Scenario: Leader failover

- GIVEN node A is the elected leader
- WHEN node A fails to send a heartbeat within the lease period
- THEN another candidate SHALL be elected as the new leader

### Requirement: Distributed Barriers

The system SHALL provide barriers that block participants until a target count is reached, then release all simultaneously.

#### Scenario: Barrier release

- GIVEN a barrier with target count 3
- WHEN 3 participants arrive at the barrier
- THEN all 3 SHALL be released simultaneously

#### Scenario: Partial barrier

- GIVEN a barrier with target count 3
- WHEN only 2 participants have arrived
- THEN both SHALL remain blocked until the 3rd arrives

### Requirement: Distributed Queues

The system SHALL provide FIFO queues with push, pop, and peek operations, backed by Raft consensus.

#### Scenario: FIFO ordering

- GIVEN items A, B, C pushed to queue `"tasks"` in order
- WHEN 3 pop operations execute
- THEN they SHALL return A, B, C in that order

#### Scenario: Empty queue

- GIVEN an empty queue
- WHEN a pop operation executes
- THEN it SHALL return `None`

### Requirement: Distributed Semaphores

The system SHALL provide counting semaphores that limit concurrent access to a shared resource.

#### Scenario: Permit acquisition

- GIVEN a semaphore `"db-pool"` with max permits 5
- WHEN 5 clients acquire permits
- THEN a 6th acquire attempt SHALL block until a permit is released

### Requirement: Sequence Generators

The system SHALL provide monotonically increasing, cluster-wide unique sequence generators.

#### Scenario: Uniqueness

- GIVEN a sequence generator `"order-id"`
- WHEN two calls to `next()` execute (from any node)
- THEN each SHALL return a distinct value

#### Scenario: Monotonicity

- GIVEN a sequence generator
- WHEN values v1 and v2 are generated in causal order
- THEN v2 SHALL be strictly greater than v1

### Requirement: Rate Limiters

The system SHALL provide distributed rate limiters using token-bucket or sliding-window algorithms.

#### Scenario: Rate limit enforcement

- GIVEN a rate limiter allowing 100 requests per second
- WHEN 101 requests arrive within 1 second
- THEN the 101st request SHALL be rejected or delayed

### Requirement: Watch and Notify

The system SHALL provide a watch mechanism that notifies subscribers when a key or key prefix changes.

#### Scenario: Key change notification

- GIVEN a client watching key `"config/db-url"`
- WHEN another client updates that key
- THEN the watcher SHALL receive a notification with the new value

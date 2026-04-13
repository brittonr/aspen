## MODIFIED Requirements

### Requirement: Distributed Locks

The system SHALL provide linearizable distributed locks with fencing tokens, TTL-based expiration, explicit release, and lease renewal. At most one live holder SHALL own a given lock at a time.

#### Scenario: Acquire, renew, and release a lock

- **WHEN** a client acquires lock `"deploy:prod"` with a TTL and later renews it before expiry
- **THEN** the lock SHALL remain owned by the same holder with the same fencing token
- **AND** a later explicit release SHALL make the lock available without reusing that token

#### Scenario: Expired lock may be taken over

- **WHEN** holder A acquires lock `"deploy:prod"` and does not renew it before its deadline
- **AND** holder B later acquires the same lock
- **THEN** holder B SHALL receive a fencing token strictly greater than holder A's token
- **AND** holder A's stale token SHALL no longer authorize release or renewal

## ADDED Requirements

### Requirement: Coordination primitive catalog

The coordination subsystem SHALL provide a catalog of distributed primitives for exclusion, sequencing, rate control, synchronization, queuing, discovery, and worker placement.

#### Scenario: Coordination crate exposes the primitive families

- **WHEN** a caller depends on `crates/aspen-coordination`
- **THEN** the subsystem SHALL expose primitives for locks, lock sets, leader election, counters, sequences, rate limiting, barriers, semaphores, read-write locks, queues, service registry, and worker coordination
- **AND** the multi-resource lock-set rules SHALL remain governed by the dedicated `distributed-lockset` spec

### Requirement: Leader election

The system SHALL provide leader election backed by an exclusive lease. At most one candidate SHALL hold leadership for a given election key at a time, and each new leadership term SHALL carry a strictly increasing fencing token.

#### Scenario: Exactly one live leader per election key

- **WHEN** two candidates compete for the same election key
- **THEN** at most one candidate SHALL report `Leader` state at any instant
- **AND** the losing candidate SHALL remain `Follower` until the current leader steps down or loses its lease

#### Scenario: Stepdown releases leadership

- **WHEN** the current leader requests graceful stepdown
- **THEN** the leader lease SHALL be released
- **AND** another candidate may acquire leadership without waiting for the full old TTL to expire

#### Scenario: Re-election increments the term token

- **WHEN** a candidate loses leadership and later becomes leader again
- **THEN** the later leadership term SHALL have a fencing token strictly greater than the earlier term

### Requirement: Atomic counters

The system SHALL provide unsigned and signed atomic counters backed by linearizable compare-and-swap semantics.

#### Scenario: Unsigned counter increments and saturating subtract

- **WHEN** a client increments an unsigned counter and later subtracts more than the current value
- **THEN** the increment SHALL be applied atomically
- **AND** the subtraction SHALL saturate at `0` instead of underflowing

#### Scenario: Compare-and-set only succeeds on the expected value

- **WHEN** a client performs compare-and-set from value `10` to `20`
- **AND** the stored value is not `10`
- **THEN** the operation SHALL report failure
- **AND** the stored counter value SHALL remain unchanged

#### Scenario: Signed counter supports negative values

- **WHEN** a client adds `-5` to a signed counter with current value `0`
- **THEN** the resulting value SHALL be `-5`
- **AND** later positive additions SHALL compose from that signed state

### Requirement: Sequence generators

The system SHALL provide globally unique, monotonically increasing sequence generators and explicit range reservation.

#### Scenario: `next()` returns strictly increasing IDs

- **WHEN** a client calls `next()` repeatedly on one sequence
- **THEN** each returned ID SHALL be greater than the previous ID
- **AND** no returned ID SHALL repeat

#### Scenario: Reserved ranges are disjoint

- **WHEN** two reserve operations each reserve `N` IDs from the same sequence
- **THEN** the returned ranges SHALL not overlap
- **AND** the second range SHALL begin after the first reserved range ends

#### Scenario: `current()` reports the next available ID

- **WHEN** a client reserves five IDs from a new sequence
- **THEN** the reserve result SHALL return the first ID in that batch
- **AND** `current()` SHALL report the next ID that would be handed out after the reservation state already consumed

### Requirement: Distributed rate limiting

The system SHALL provide token-bucket rate limiters with bounded burst capacity, gradual refill over time, retry hints on exhaustion, and explicit reset.

#### Scenario: Burst capacity is enforced

- **WHEN** a limiter is configured with burst capacity `5`
- **THEN** up to five immediate token acquisitions MAY succeed without waiting
- **AND** a further acquisition before refill SHALL be rejected as rate limited

#### Scenario: Exhaustion reports retry timing

- **WHEN** a client requests tokens from an exhausted limiter
- **THEN** the system SHALL report that the request was rate limited
- **AND** it SHALL include a retry-after delay derived from the configured refill rate

#### Scenario: Blocking acquire waits only until timeout

- **WHEN** a client requests blocking acquisition with a timeout
- **THEN** the call SHALL retry until tokens become available or the timeout is reached
- **AND** it SHALL not wait indefinitely once the timeout budget is exhausted

### Requirement: Barriers

The system SHALL provide double-barrier coordination with enter, ready, leave, and completion phases for a named participant set.

#### Scenario: Barrier becomes ready only after the required participants arrive

- **WHEN** a barrier requires three participants
- **AND** only two participants have entered
- **THEN** the barrier SHALL remain in `waiting` phase
- **AND** no caller SHALL observe `ready` until the third participant arrives

#### Scenario: Last participant completes the barrier

- **WHEN** all participants have entered a barrier and the final participant later leaves
- **THEN** the barrier SHALL transition to completion
- **AND** subsequent status checks SHALL observe no active barrier state for that name

### Requirement: Semaphores

The system SHALL provide distributed counting semaphores with bounded capacity, TTL-backed holders, and blocking or non-blocking acquisition.

#### Scenario: Permit acquisitions never exceed capacity

- **WHEN** holders acquire permits from a semaphore with capacity `5`
- **THEN** the total live permits held across all non-expired holders SHALL never exceed `5`
- **AND** requests that would exceed capacity SHALL wait or fail instead of over-allocating

#### Scenario: Reacquiring by the same holder refreshes its lease

- **WHEN** holder `worker-7` already holds permits for a semaphore and tries to acquire again before expiry
- **THEN** the operation SHALL refresh that holder's TTL
- **AND** it SHALL not silently create a second independent holder record for the same holder ID

#### Scenario: Holder count is bounded

- **WHEN** a semaphore already has the maximum allowed number of live holders
- **THEN** an additional distinct holder SHALL be rejected
- **AND** the rejection SHALL occur before the holder set grows past the configured bound

### Requirement: Read-write locks

The system SHALL provide distributed read-write locks with multiple-reader or single-writer exclusivity, writer-preference fairness, TTL-backed ownership, and write fencing tokens.

#### Scenario: Readers may share the lock

- **WHEN** no writer holds or is waiting on a read-write lock
- **THEN** multiple readers MAY acquire the lock concurrently
- **AND** status SHALL report read mode with the current reader count

#### Scenario: Writers exclude new readers

- **WHEN** a writer holds the lock or is queued waiting to acquire it
- **THEN** new reader acquisitions SHALL not bypass that writer
- **AND** write acquisition SHALL still require all current readers to release or expire first

#### Scenario: Downgrade preserves the writer token

- **WHEN** a writer downgrades an exclusive lock to a read lock
- **THEN** the holder SHALL remain present as a reader
- **AND** the write fencing token SHALL be preserved across the downgrade transition

### Requirement: Distributed queues

The system SHALL provide FIFO queues with visibility timeouts, acknowledgment, negative acknowledgment, dead-letter handling, deduplication, and bounded peek/status operations.

#### Scenario: FIFO dequeue order follows enqueue order

- **WHEN** three items are enqueued to the same queue without message-group constraints
- **THEN** dequeue SHALL return them in the order they were enqueued
- **AND** item identifiers SHALL increase monotonically with enqueue order

#### Scenario: Visibility timeout causes redelivery

- **WHEN** a consumer dequeues an item and does not acknowledge it before the visibility deadline
- **THEN** the item SHALL become visible again for later dequeue
- **AND** the next delivery SHALL record an increased delivery-attempt count

#### Scenario: Dead-letter queue captures repeated failures

- **WHEN** an item exceeds the queue's maximum delivery attempts or is explicitly nacked to DLQ
- **THEN** it SHALL be removed from the main visible queue
- **AND** it SHALL appear in the dead-letter queue until redriven or deleted

#### Scenario: Deduplication suppresses duplicate enqueue

- **WHEN** a producer enqueues the same queue item twice with the same deduplication ID inside the deduplication window
- **THEN** the queue SHALL not create a second distinct live item for that duplicate request

### Requirement: Service registry

The system SHALL provide TTL-backed service registration and discovery with fencing tokens, health tracking, metadata updates, and filtered lookup.

#### Scenario: Register and discover healthy instances

- **WHEN** a service instance registers under service `"api-gateway"`
- **THEN** discovery for `"api-gateway"` SHALL return that instance while its lease remains live
- **AND** the registry SHALL return the instance's address and metadata

#### Scenario: Heartbeat renews a service lease

- **WHEN** a registered instance sends a heartbeat with its current fencing token
- **THEN** the registry SHALL extend the instance deadline
- **AND** the instance SHALL remain discoverable without changing its fencing token

#### Scenario: Stale fencing token cannot mutate registration

- **WHEN** an instance is re-registered and receives a newer fencing token
- **THEN** later health or metadata updates using the old token SHALL be rejected
- **AND** only the current token holder MAY mutate or remove that instance record

#### Scenario: Discovery filters narrow results

- **WHEN** a caller requests discovery with health, tag, version-prefix, or limit filters
- **THEN** the registry SHALL return only instances matching those filters

### Requirement: Worker coordination

The system SHALL provide distributed worker coordination for registering workers, tracking capacity and liveness, routing work with bounded strategies, and supporting bounded work stealing.

#### Scenario: Worker registration makes a worker discoverable

- **WHEN** a worker registers itself with the coordinator and keeps its lease healthy
- **THEN** routing decisions MAY include that worker as an eligible target
- **AND** expired or unhealthy workers SHALL be excluded from new placement decisions

#### Scenario: Assignment respects worker capacity

- **WHEN** the coordinator assigns work to a worker group
- **THEN** it SHALL not assign a task to a worker that is already at its declared capacity
- **AND** the assignment result SHALL identify at most one active owner for a given task

#### Scenario: Work stealing is bounded

- **WHEN** the coordinator rebalances work by stealing from a busy worker
- **THEN** each steal operation SHALL respect the configured steal-batch limits
- **AND** it SHALL not overload the destination worker above its capacity ceiling

### Requirement: Coordination RPC contract

The system SHALL expose one coordination request/response contract for locks, lock sets, counters, sequences, rate limiters, barriers, semaphores, read-write locks, queues, and service registry operations, independent of whether the backing implementation is native or plugin-backed.

#### Scenario: Same wire contract across backend implementations

- **WHEN** a coordination client sends a request for a supported primitive family
- **THEN** the request and response schema SHALL stay the same regardless of whether Aspen serves that operation natively or through a plugin-backed handler

#### Scenario: Lock-set RPC returns canonical member tokens

- **WHEN** a client acquires a lock set through the coordination RPC surface
- **THEN** the response SHALL include canonical member tokens suitable for later renew and release operations
- **AND** the detailed lock-set semantics SHALL match the dedicated `distributed-lockset` contract

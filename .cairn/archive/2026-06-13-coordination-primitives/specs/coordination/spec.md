# Coordination Delta: Primitive Control-Plane Services

### Requirement: Coordination primitive records are canonical
r[molten.coordination.primitive_model] Molten MUST define canonical coordination service manifest, request, receipt, fencing-token, state-snapshot, status-assertion, and apply-report records for the admitted initial primitive set: locks, queues, semaphores, rate limiters, elections, barriers, and service registry entries. Counter and sequence-generator primitives are future service ids and MUST NOT be implied until their own admitted manifest support exists.

#### Scenario: Primitive record refs are stable
- GIVEN the same coordination manifest, request, and resulting state
- WHEN Molten renders the coordination records
- THEN their canonical refs are stable
- AND unsupported primitive ids are rejected before mutation.

### Requirement: Coordination results are reflected as dataspace assertions
r[molten.coordination.dataspace_interface] Molten MUST expose coordination demand, grants, denials, expiry, release, and service readiness through canonical coordination request, receipt, and status-assertion records rather than ambient shared state.

#### Scenario: Lock-held fact is observable
- GIVEN an admitted lock acquire request commits through the coordination runtime
- WHEN observers inspect coordination status assertions
- THEN the lock-held fact is visible as a dataspace assertion bound to the coordination receipt and state ref.

### Requirement: Coordination queues are not default mailboxes
r[molten.coordination.no_default_mailbox] Coordination queues MUST NOT replace ordinary actor mailboxes, dataspace assertions, choreography steps, gossip, blobs, or job traffic; they are explicit control-plane primitives selected by coordination service manifests and requests.

#### Scenario: Actor message does not mutate coordination state
- GIVEN ordinary actor traffic that is not a coordination request
- WHEN the traffic is delivered through the runtime dataspace
- THEN coordination state is unchanged
- AND no Raft/control-plane coordination commit is emitted.

### Requirement: Coordination receipts bind every decision
r[molten.coordination.receipts] Molten MUST emit canonical receipts for admitted and denied coordination acquire, grant, release, enqueue, dequeue, read, election, barrier, registry, duplicate-replay, capacity-denial, and cleanup-relevant outcomes.

#### Scenario: Denial still records evidence
- GIVEN a coordination request is denied for missing resource evidence or invalid state
- WHEN Molten evaluates the request
- THEN it emits a deny receipt with diagnostics and the pre-mutation state ref
- AND no protected mutation commits.

### Requirement: Lock leases use fencing tokens
r[molten.coordination.lock_fencing] Molten MUST implement local deterministic lock and election grants with monotonic fencing tokens, lease epochs, owner binding, duplicate operation-id replay, and stale-token denial before protected release or mutation commits.

#### Scenario: Stale fencing token is denied
- GIVEN lock token `1` has been superseded or the release presents token `0`
- WHEN a client attempts a protected release with the stale token
- THEN Molten emits a deny receipt
- AND the held lock state remains protected.

### Requirement: Queue operations are explicit and bounded
r[molten.coordination.queue_visibility] Molten MUST implement explicit queue enqueue/dequeue requests with FIFO ordering, operation-id replay, capacity/resource denial, and status assertions. Visibility-timeout, ack, nack, retry, and DLQ policies MAY be added only by a future admitted extension and MUST NOT be inferred from ordinary actor mailboxes.

#### Scenario: FIFO dequeue is receipt-backed
- GIVEN two queue enqueue requests commit in order
- WHEN a dequeue request commits
- THEN the first item is removed first
- AND the receipt and status assertion bind the resulting queue state.

### Requirement: Service registry state is coordination state
r[molten.coordination.service_registry] Molten MUST represent service demand, readiness, health, exposed refs, registry updates, and discovery as coordination requests and status assertions served through control-plane commit or read-index evidence.

#### Scenario: Registry read binds read-index evidence
- GIVEN a service endpoint registration has committed
- WHEN a reader requests the service registry entry
- THEN Molten emits a read-index-backed receipt
- AND the status assertion contains the endpoint ref.

### Requirement: Semaphore and rate-limit primitives are bounded
r[molten.coordination.rate_limit_semaphore] Molten MUST implement semaphore and rate-limit coordination operations with manifest-defined bounds, deterministic pass/deny receipts, and resource-governance evidence refs.

#### Scenario: Semaphore capacity denial is deterministic
- GIVEN all semaphore permits are held
- WHEN another acquire request arrives
- THEN Molten emits a deny receipt
- AND the semaphore state is unchanged.

### Requirement: Coordination mutations use control-plane contracts
r[molten.coordination.raft_contract] Coordination mutations MUST execute through an admitted Raft/control-plane state-machine contract with commit receipts, and coordination reads MUST use read-index evidence by default.

#### Scenario: Queue mutation commits through control plane
- GIVEN an admitted queue enqueue request
- WHEN Molten applies the request
- THEN the coordination receipt binds a control-plane commit receipt
- AND the status assertion is published only after the commit.

### Requirement: Coordination replay is deterministic
r[molten.coordination.replay_backend] Coordination replay MUST reuse recorded coordination decisions by operation id, returning prior receipts for duplicate operations and allowing deterministic fixtures to validate recorded request, receipt, state, and assertion refs without performing a second live mutation.

#### Scenario: Duplicate operation returns prior receipt
- GIVEN a coordination operation id has already committed
- WHEN the same request is replayed
- THEN Molten returns the prior receipt ref
- AND the state machine is not advanced a second time.

### Requirement: Coordination invariants are bounded and checkable
r[molten.coordination.trellis_predicates] Molten MUST expose bounded, deterministic checks for fencing monotonicity, mutual exclusion, FIFO queue behavior, semaphore bounds, read-index use, and no-stale-token acceptance in coordination receipts or property fixtures.

#### Scenario: Fencing invariant is checked
- GIVEN a generated coordination run with lock acquisition and stale release attempts
- WHEN the property fixture evaluates the run
- THEN stale tokens are denied
- AND accepted tokens are monotonic for the resource.

### Requirement: Coordination integration tests use dataspace assertions
r[molten.coordination.integration_tests] Molten SHOULD test coordination requests that produce lock, queue, semaphore, and registry status assertions observable through the runtime/catalog surface while preserving the control-plane mutation boundary.

#### Scenario: Queue result is observable
- GIVEN a queue request commits through the coordination runtime
- WHEN the result is inspected as coordination evidence
- THEN the status assertion describes the committed queue state
- AND the mutation path remains control-plane-bound.

### Requirement: Lock behavior is tested
r[molten.coordination.lock_tests] Molten SHOULD test mutual exclusion, fencing-token monotonicity, duplicate operation-id replay, release, expiry-or-stale denial, and stale-token rejection for lock/election-style coordination primitives.

#### Scenario: Lock test observes stale denial
- GIVEN the coordination test suite runs
- WHEN a stale lock release is attempted
- THEN the deny receipt is asserted
- AND the lock remains held by the admitted owner.

### Requirement: Queue behavior is tested
r[molten.coordination.queue_tests] Molten SHOULD test FIFO ordering, duplicate enqueue replay, queue overflow, resource denial, and explicit queue status assertions for coordination queues.

#### Scenario: Queue test observes FIFO order
- GIVEN the queue test enqueues two items
- WHEN it dequeues once
- THEN the first item is removed
- AND the second remains at the head of the queue.

### Requirement: Ordinary traffic bypasses Raft by default
r[molten.coordination.no_raft_ordinary_traffic_tests] Molten SHOULD test or statically check that ordinary actor messages, dataspace assertions, choreography steps, gossip, blob transfer, and job traffic do not route through coordination/Raft unless an explicit coordination request is applied.

#### Scenario: No coordination request means no coordination mutation
- GIVEN a runtime state snapshot and ordinary actor-message-shaped data
- WHEN no coordination request is applied
- THEN the coordination state ref remains unchanged
- AND no coordination commit receipt is emitted.

### Requirement: Coordination property tests cover deterministic invariants
r[molten.coordination.property_tests] Molten SHOULD include bounded Hegel property tests for mutual exclusion, FIFO queue ordering, semaphore bounds, operation-id replay, and deterministic backend replay behavior.

#### Scenario: Generated run preserves bounds
- GIVEN generated lock, queue, and semaphore operations within supported bounds
- WHEN the property test applies them
- THEN lock fencing remains monotonic, queue order remains FIFO, and semaphore permits never exceed the manifest capacity.

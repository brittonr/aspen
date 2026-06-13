# Coordination Delta: Control-Plane Services

### Requirement: Coordination mutations are control-plane commands
r[molten.coordination_services_control_plane.spec.control_plane] Coordination mutations MUST execute through admitted control-plane state-machine commands and MUST NOT be implemented as ordinary actor message side effects.

#### Scenario: Lock acquire commits through control plane
- GIVEN an admitted lock acquire request
- WHEN the request is applied
- THEN a Raft/control-plane commit receipt is bound in the coordination receipt
- AND a lock-held dataspace assertion is published after commit

#### Scenario: Actor message cannot mutate lock state
- GIVEN an ordinary actor message claiming to acquire a lock
- WHEN it is delivered through the dataspace
- THEN it does not mutate coordination state without a coordination request receipt

### Requirement: Fencing tokens are monotonic and checked
r[molten.coordination_services_control_plane.spec.fencing] Lock and lease operations MUST issue monotonic fencing tokens and MUST reject stale tokens before protected operations commit.

#### Scenario: Stale token denied
- GIVEN lock token `10` has been superseded by token `11`
- WHEN a client uses token `10` for a protected operation
- THEN Molten emits a stale-token denial receipt
- AND no protected mutation commits

### Requirement: Coordination state is observable through dataspace assertions
r[molten.coordination_services_control_plane.spec.dataspace_reflection] Committed coordination outcomes MUST be reflected as local dataspace assertions for observers, while mutation remains gated by the control plane.

#### Scenario: Queue depth assertion updates
- GIVEN a queue enqueue commits
- WHEN the state machine applies the command
- THEN Molten publishes a queue-depth assertion with receipt refs
- AND observers see the update through normal dataspace routing

### Requirement: Coordination records are canonical
r[molten.coordination_services_control_plane.manifest] Molten MUST define canonical coordination service manifest, request, receipt, fencing token, state snapshot, and status assertion records.

#### Scenario: Coordination record refs are stable
- GIVEN the same coordination manifest and request fields
- WHEN Molten renders the records
- THEN their canonical refs are stable
- AND receipts bind request, state, token, and assertion refs.

### Requirement: Coordination requests bind operation ids
r[molten.coordination_services_control_plane.operation_ids] Mutating coordination requests MUST bind scoped operation-id/idempotency refs before side effects commit.

#### Scenario: Duplicate operation is replayed without a second mutation
- GIVEN a mutating coordination request has already committed
- WHEN the same operation id is submitted again
- THEN Molten returns the prior semantic receipt
- AND no second state mutation commits.

### Requirement: Coordination artifacts are discoverable
r[molten.coordination_services_control_plane.ledger_catalog] Coordination artifacts MUST be classified in ledger, catalog, and MCP read-only views.

#### Scenario: Coordination receipt appears in catalog
- GIVEN a coordination receipt is imported into the local ledger
- WHEN the catalog or MCP view queries that artifact
- THEN the artifact kind identifies it as coordination evidence
- AND the view remains read-only.

### Requirement: Coordination schemas are exported
r[molten.coordination_services_control_plane.schema_constants] Coordination record schemas MUST be exported as stable Molten schema constants.

#### Scenario: Receipt carries the coordination schema
- GIVEN a coordination receipt is emitted
- WHEN the record is parsed
- THEN its schema field identifies `molten.coordination.receipt.v1`
- AND unsupported schema values are rejected.

### Requirement: Lock leases use fencing
r[molten.coordination_services_control_plane.lock_fencing] Lock acquire and release operations MUST issue monotonic fencing tokens and reject stale or owner-mismatched release requests.

#### Scenario: Duplicate acquire does not advance token
- GIVEN a lock acquire request has committed with token `1`
- WHEN the same operation id is replayed
- THEN the same receipt is returned
- AND the next fencing token is not advanced.

### Requirement: Queue and semaphore state is bounded
r[molten.coordination_services_control_plane.queue_semaphore] Queue and semaphore primitives MUST enforce FIFO ordering, capacity bounds, and deterministic denial receipts.

#### Scenario: Queue overflow denies before commit
- GIVEN a queue is at configured capacity
- WHEN another enqueue request is applied
- THEN Molten emits a queue-overflow denial receipt
- AND the queue contents remain unchanged.

### Requirement: Rate limit, election, and barrier transitions are deterministic
r[molten.coordination_services_control_plane.rate_election_barrier] Rate-limit, election, and barrier primitives MUST update state deterministically and MUST emit receipts for pass and denial outcomes.

#### Scenario: Barrier releases at the configured party count
- GIVEN a barrier requires two participants
- WHEN two distinct participants arrive
- THEN the barrier state becomes released
- AND the status assertion reflects the released state.

### Requirement: Service registry pointers are control-plane state
r[molten.coordination_services_control_plane.service_registry] Service registry pointer updates MUST execute through control-plane commits and bind endpoint evidence refs.

#### Scenario: Service registration is read through read-index
- GIVEN a service registry pointer has committed
- WHEN a read request is served
- THEN Molten emits a read-index receipt
- AND the dataspace assertion contains the endpoint ref.

### Requirement: Coordination operations are gated
r[molten.coordination_services_control_plane.authority_resource] Coordination operations MUST be gated through explicit authority, policy, resource, idempotency, and Raft read/commit evidence.

#### Scenario: Missing resource evidence denies
- GIVEN a mutating coordination request has no resource evidence
- WHEN the request is applied
- THEN Molten emits a denial receipt
- AND no control-plane commit is appended.

### Requirement: Coordination status assertions follow commits
r[molten.coordination_services_control_plane.status_assertions] Molten MUST publish coordination status assertions only after the corresponding control-plane commit or read-index decision.

#### Scenario: Lock-held assertion follows commit
- GIVEN a lock acquire request passes
- WHEN the control-plane commit receipt is emitted
- THEN a lock-held status assertion is emitted
- AND the coordination receipt binds that assertion ref.

### Requirement: Coordination reads use read-index
r[molten.coordination_services_control_plane.read_index] Coordination read requests MUST use read-index receipts by default rather than speculative local reads.

#### Scenario: Registry read binds read-index
- GIVEN a committed service registry entry
- WHEN a read request is applied
- THEN the coordination receipt binds a Raft read receipt
- AND the read receipt targets the committed registry state.

### Requirement: Active coordination evidence is retained
r[molten.coordination_services_control_plane.gc_retention] Active locks, fencing tokens, queues, barriers, and registry pointers MUST emit retention refs that can pin required evidence against unsafe GC.

#### Scenario: Active lock token is retained
- GIVEN a lock is held
- WHEN Molten emits a state snapshot
- THEN the active fencing token ref appears in retention refs
- AND the snapshot carries an active-state-retention check.

### Requirement: Lock behavior is tested
r[molten.coordination_services_control_plane.lock_tests] Molten SHOULD test acquire/release, stale fencing denial, duplicate idempotent acquire, and observer assertions.

#### Scenario: Lock tests exercise stale token denial
- GIVEN the coordination test suite runs
- WHEN stale lock release is attempted
- THEN the denial receipt is asserted
- AND the held lock remains protected.

### Requirement: Queue behavior is tested
r[molten.coordination_services_control_plane.queue_tests] Molten SHOULD test FIFO dequeue, duplicate enqueue, queue overflow, and resource denial.

#### Scenario: Queue tests exercise FIFO order
- GIVEN two enqueue requests commit
- WHEN a dequeue request commits
- THEN the first enqueued item is removed
- AND the second item remains at the queue head.

### Requirement: Service registry behavior is tested
r[molten.coordination_services_control_plane.service_registry_tests] Molten SHOULD test service registry updates and read-index reads.

#### Scenario: Registry tests exercise read-index path
- GIVEN the registry test registers a service
- WHEN it reads that service
- THEN a read-index receipt is present
- AND the endpoint ref is visible in the assertion.

### Requirement: Coordination properties are tested
r[molten.coordination_services_control_plane.property_tests] Molten SHOULD include bounded property coverage for fencing monotonicity, FIFO ordering, semaphore bounds, and the no-actor-traffic invariant.

#### Scenario: Property run preserves no-actor-traffic invariant
- GIVEN a bounded generated coordination run
- WHEN no coordination request is applied
- THEN ordinary actor-message-shaped data cannot mutate coordination state
- AND the state snapshot ref remains unchanged.

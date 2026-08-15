# Coordination Specification

## Purpose

Defines the `coordination` capability and Molten's explicit control-plane services for locks, queues, semaphores, rate limiters, elections, barriers, and service registry state.

## Requirements

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

### Requirement: Coordination generated traces preserve invariants
r[molten.coordination_state_machine_proof.generated_traces] Molten MUST provide bounded generated coordination traces that exercise implemented coordination primitives and check fencing monotonicity, mutual exclusion, FIFO queue behavior, semaphore bounds, barrier release thresholds, election consistency, and deterministic status assertions after each step.

#### Scenario: Generated trace preserves primitive invariants
- GIVEN a generated bounded sequence of coordination requests
- WHEN Molten applies the sequence through the coordination state machine
- THEN every accepted step preserves the primitive-specific invariant for its key
- AND emitted receipts and assertions bind the resulting state evidence.

### Requirement: Denied coordination operations do not mutate state
r[molten.coordination_state_machine_proof.deny_no_mutation] Molten MUST prove that denied coordination operations leave the coordination state ref unchanged while still emitting deterministic denial receipts.

#### Scenario: Stale token denial leaves state unchanged
- GIVEN a held coordination lock and a generated stale release token
- WHEN Molten applies the stale release request
- THEN the receipt decision is `deny`
- AND the coordination state ref after the request equals the state ref before the request.

### Requirement: Duplicate coordination operations do not advance twice
r[molten.coordination_state_machine_proof.duplicate_no_advance] Molten MUST prove inside generated traces that duplicate coordination operation ids return prior receipt evidence and do not apply the same state-machine mutation a second time.

#### Scenario: Duplicate generated operation replays receipt
- GIVEN a generated coordination operation id that has already committed
- WHEN the same operation id is generated again with the same request identity
- THEN Molten returns the prior receipt ref
- AND the state machine is not advanced again.

### Requirement: Coordination primitives expose pure transition cores
r[molten.coordination_state_machine_proof.primitive_transition_cores] Molten MUST expose coordination lock, queue, semaphore, rate-limit, election, barrier, and registry semantics through pure primitive transition cores that consume current state, manifest limits, request facts, replay/idempotency facts, and admission facts, and return transition results without mutating runtime state or performing control-plane, filesystem, network, ledger, or dataspace effects.

#### Scenario: Lock acquire transition returns candidate state
- GIVEN a lock is free, the request is admitted, and the manifest permits lock acquisition
- WHEN the lock acquire transition core evaluates the request
- THEN it returns a pass decision with a candidate next state, fencing token fact, status assertion fact, and receipt checks
- AND no runtime state is mutated until the shell commits the transition.

#### Scenario: Queue overflow transition preserves state
- GIVEN a queue is already at manifest capacity
- WHEN the queue enqueue transition core evaluates another enqueue request
- THEN it returns a deny decision with diagnostics
- AND the preserved-state ref matches the input state ref.

### Requirement: Coordination duplicate replay is an explicit transition kind
r[molten.coordination_state_machine_proof.replay_transition_kind] Coordination operation-id replay MUST be represented as an explicit no-advance transition kind that returns prior receipt or output refs for exact duplicates, denies conflicting duplicates, and preserves the current coordination state in both cases.

#### Scenario: Exact duplicate acquire does not advance token
- GIVEN a lock acquire operation id has already committed with a fencing token
- WHEN the same request is evaluated again
- THEN the transition kind is duplicate replay
- AND the next fencing token and lock state are not advanced.

#### Scenario: Conflicting duplicate denies without mutation
- GIVEN an operation id has already committed for one coordination request
- WHEN a different request reuses that operation id
- THEN Molten emits a deny transition
- AND no primitive state, token counter, queue contents, or registry entry changes.

### Requirement: Coordination transition receipts bind state movement
r[molten.coordination_state_machine_proof.transition_receipt_binding] Coordination receipts and status assertions MUST bind the primitive transition kind, service, operation, key, request ref, before-state ref, after-state ref or preserved-state ref, token or output facts when present, control-plane intent or commit refs when present, decision, diagnostics, and checks.

#### Scenario: Denial receipt proves no mutation
- GIVEN a stale fencing-token release is denied
- WHEN the coordination receipt is emitted
- THEN the receipt binds the stale-token diagnostic and preserved-state ref
- AND the held lock state remains unchanged.

### Requirement: Coordination generated traces cover transition matrix
r[molten.coordination_state_machine_proof.transition_matrix_tests] Molten SHOULD extend bounded generated coordination traces to cover pass, denial, exact duplicate replay, conflicting duplicate denial, and preserved-state assertions for locks, queues, semaphores, rate limits, elections, barriers, and registry entries.

#### Scenario: Generated matrix covers each primitive denial
- GIVEN the generated coordination trace suite runs
- WHEN each supported primitive receives at least one invalid or over-limit event
- THEN each invalid event emits deny evidence
- AND every denial preserves the prior state ref.

### Requirement: Coordination reads declare consistency mode
r[molten.coordination.read_consistency_modes] Molten MUST carry an explicit read consistency mode on coordination read requests, receipts, and status assertions. Coordination reads default to linearizable control-plane evidence; local-stale reads MAY be emitted only as non-authoritative observations.

#### Scenario: Coordination read defaults to linearizable evidence
- GIVEN a coordination client reads a service registry entry, lock state, queue state, semaphore state, rate-limit state, election state, or barrier state without requesting stale diagnostics
- WHEN Molten serves the read
- THEN the coordination receipt binds linearizable control-plane read evidence
- AND the status assertion identifies the read consistency mode.

#### Scenario: Local-stale coordination read is labeled
- GIVEN an operator requests a local-stale coordination status read for diagnostics
- WHEN Molten serves the read from local state
- THEN the receipt and status assertion mark the result as local-stale
- AND the result is not accepted as current coordination authority.

### Requirement: Local-stale coordination reads cannot authorize protected actions
r[molten.coordination.local_stale_boundaries] Molten MUST reject local-stale coordination read receipts wherever a decision requires current state, including mutation admission, lock ownership, fencing-token validation, release gates, election leadership, barrier release, rate-limit enforcement, membership admission, or production pass evidence.

#### Scenario: Stale lock read cannot release a lock
- GIVEN a client presents a local-stale read showing it as lock owner
- WHEN the client attempts a protected release or mutation
- THEN Molten denies the request unless separate linearizable read, commit, or fencing evidence is present
- AND diagnostics identify the stale-read boundary.

#### Scenario: Stale registry read cannot satisfy admission
- GIVEN a policy or service admission gate requires the current service registry pointer
- WHEN the gate is given only a local-stale registry receipt
- THEN the gate denies currentness
- AND requires linearizable coordination evidence.

### Requirement: Coordination supports explicit batched control-plane operations
r[molten.coordination.batched_control_plane_operations] Molten SHOULD support canonical batched or compare-and-swap-style coordination operation envelopes for low-write control-plane workflows. Batches MUST preserve per-operation ids, per-operation authority/policy/resource evidence, deterministic ordering, per-operation receipts, and a single enclosing control-plane commit or denial receipt.

#### Scenario: Valid batch commits deterministically
- GIVEN a batch contains admitted coordination operations with distinct operation ids and satisfied evidence
- WHEN the batch is applied through the control-plane state machine
- THEN Molten applies the operations in canonical batch order
- AND emits per-operation receipts plus an enclosing commit receipt.

#### Scenario: Invalid batch denies safely
- GIVEN a batch contains an operation with missing authority, stale compare input, duplicate operation id, unsupported primitive, or resource denial
- WHEN the batch is evaluated
- THEN Molten emits deterministic denial evidence for the affected operation or batch according to the manifest policy
- AND no undeclared partial mutation can be treated as committed.

### Requirement: Coordination remains small control-plane state
r[molten.coordination.small_control_plane_scope] Coordination services MUST remain scoped to small, explicit control-plane state such as locks, fencing tokens, queues, semaphores, rate limits, elections, barriers, and service registry pointers. They MUST NOT become the default storage path for job payloads, actor mailboxes, blob contents, gossip fanout, or ordinary dataspace state.

#### Scenario: Large payload stays out of coordination log
- GIVEN a job or actor request carries a large payload or ordinary message body
- WHEN the request references coordination services
- THEN coordination records carry only content refs, operation refs, or control-plane pointers where admitted
- AND the payload itself remains outside the consensus log.

#### Scenario: Ordinary queue traffic is not implied
- GIVEN ordinary actor or job traffic uses mailboxes, dataspaces, or job scheduling
- WHEN no explicit coordination request is present
- THEN Molten does not append a coordination control-plane command
- AND no coordination receipt claims authority over that ordinary traffic.

### Requirement: Coordination consumes engine-agnostic currentness evidence
r[molten.coordination.engine_agnostic_evidence] Coordination services MUST consume normalized consensus commit, read, fencing, and currentness receipts rather than Raft-specific leader, term, index, or read-index internals. Coordination decisions MAY preserve engine-specific evidence refs for audit, but mutation admission and protected-action checks MUST evaluate the common evidence fields.

#### Scenario: Lock release uses normalized currentness
- GIVEN a client attempts to release a coordination lock after reading lock ownership through an admitted consensus engine
- WHEN coordination admission evaluates the release
- THEN it checks normalized currentness, fencing epoch, operation id, and resource evidence
- AND it does not require the backing engine to expose Raft-specific fields.

#### Scenario: Engine-specific receipt without normalized fields denies
- GIVEN a coordination request presents an engine-specific receipt that lacks normalized currentness or fencing fields
- WHEN coordination admission evaluates a protected action
- THEN admission denies the receipt as insufficient authority
- AND diagnostics identify the missing normalized consensus evidence.

### Requirement: Coordination switchover gates require active engine epoch
r[molten.coordination.engine_switchover_gates] Coordination services MUST reject mutation, release, election, barrier, rate-limit, registry, and membership decisions that rely on consensus receipts from inactive, superseded, or not-yet-activated engine epochs. Coordination status readback MUST show the active engine profile and epoch for protected control-plane state.

#### Scenario: Stale epoch cannot release lock
- GIVEN a consensus engine switchover has activated a new engine epoch for a coordination group
- WHEN a client presents a lock ownership receipt from the prior engine epoch
- THEN coordination denies the lock release or protected mutation
- AND diagnostics name the stale engine epoch and active engine profile.

#### Scenario: Status readback names active engine
- GIVEN a coordination service is backed by a pluggable consensus engine registry entry
- WHEN an operator requests service status
- THEN the status assertion names the active engine profile, profile version, engine epoch, read consistency mode, and currentness evidence ref
- AND local-stale status is still labeled non-authoritative.

## ADDED Requirements

### Requirement: Trellis-backed assertion visibility
r[molten.trellis_runtime.assertion_visibility] The system SHOULD provide Trellis-backed predicates for dataspace assertion ownership, duplicate assertion deduplication, visibility, and automatic retraction.

#### Scenario: Assertion visible iff live owner maintains it
r[molten.trellis_runtime.assertion_visibility.live_owner]
- GIVEN a bounded model of assertion owners, assertion handles, owner liveness, and retractions
- WHEN Molten evaluates whether a canonical assertion is visible
- THEN the predicate reports visible exactly when at least one live admitted owner still maintains that assertion

#### Scenario: Duplicate assertion retracts only after final owner
r[molten.trellis_runtime.assertion_visibility.dedup]
- GIVEN multiple live owners asserting the same canonical value
- WHEN one but not all owners retract or terminate
- THEN the predicate preserves observer-level visibility until the final live owner withdraws the assertion

### Requirement: Trellis-backed turn commit and rollback
r[molten.trellis_runtime.turn_commit_rollback] The system SHOULD provide Trellis-backed predicates for pending-action invisibility, atomic turn commit, and rollback when a turn fails or is denied.

#### Scenario: Failed turn leaves committed state unchanged
r[molten.trellis_runtime.turn_commit_rollback.failed]
- GIVEN a prior state summary and a bounded set of pending actions
- WHEN the turn outcome is failed or denied
- THEN the predicate reports that committed actor, vat, and dataspace state remain unchanged

#### Scenario: Successful turn applies pending actions atomically
r[molten.trellis_runtime.turn_commit_rollback.commit]
- GIVEN a prior state summary, admitted pending actions, and a successful turn outcome
- WHEN the turn commits
- THEN the predicate reports that all admitted pending actions become visible together as the next committed state

### Requirement: Bounded Preserves pattern predicate subset
r[molten.trellis_runtime.preserves_pattern_subset] The system SHOULD define a bounded Trellis-friendly Preserves pattern and value subset with deterministic matching and binding order for routing and policy-visible matching predicates.

#### Scenario: Pattern match is deterministic
r[molten.trellis_runtime.preserves_pattern_subset.deterministic]
- GIVEN a bounded Preserves pattern and bounded Preserves value model
- WHEN two nodes evaluate the match
- THEN both nodes produce the same success or failure result and the same ordered bindings

### Requirement: Trellis-backed Observe delivery
r[molten.trellis_runtime.observe_delivery] The system SHOULD provide Trellis-backed predicates for Observe delivery of matching current assertions, future assertions, and matching retraction propagation.

#### Scenario: New Observe receives matching current set
r[molten.trellis_runtime.observe_delivery.current]
- GIVEN a current dataspace assertion set and a new Observe subscription with a matching pattern
- WHEN the Observe predicate evaluates initial delivery
- THEN it identifies exactly the current visible assertions that match the pattern

#### Scenario: Retraction propagates to matching observers
r[molten.trellis_runtime.observe_delivery.retraction]
- GIVEN a visible assertion delivered to an observer through an Observe subscription
- WHEN the assertion is no longer visible
- THEN the predicate identifies the corresponding observer retraction that must be emitted

### Requirement: Trellis-backed promise state machine
r[molten.trellis_runtime.promise_state] The system SHOULD provide Trellis-backed predicates for promise/vow states including pending, resolved, broken, cancelled, timed out, and causal failure propagation.

#### Scenario: Promise has one terminal result
r[molten.trellis_runtime.promise_state.terminal]
- GIVEN a pending promise and a generated sequence of resolution, failure, cancellation, or timeout events
- WHEN the predicate admits a terminal transition
- THEN no later conflicting terminal result is admitted for the same promise

### Requirement: Trellis-backed promise pipelining
r[molten.trellis_runtime.promise_pipeline] The system SHOULD provide Trellis-backed predicates for bounded promise pipelining, including queue bounds, forwarding order, and cleanup after failure.

#### Scenario: Resolved pipeline forwards in order
r[molten.trellis_runtime.promise_pipeline.order]
- GIVEN a bounded queue of pipelined calls and a promise that resolves to a reference
- WHEN the pipeline is admitted for forwarding
- THEN queued calls are forwarded in original order subject to policy admission

#### Scenario: Broken pipeline fails queued calls
r[molten.trellis_runtime.promise_pipeline.broken]
- GIVEN queued pipelined calls and a promise that breaks
- WHEN the failure predicate evaluates the queue
- THEN all queued calls fail causally and no target side effects are admitted

### Requirement: Trellis-backed revocation cleanup
r[molten.trellis_runtime.revocation_cleanup] The system SHOULD provide Trellis-backed predicates for revoked references denying future use and cleaning dependent assertions, subscriptions, pending calls, and child references.

#### Scenario: Revoked proxy invalidates dependents
r[molten.trellis_runtime.revocation_cleanup.proxy]
- GIVEN a proxy reference with dependent assertions, Observe subscriptions, pending calls, and child references
- WHEN the proxy is revoked
- THEN the predicate identifies future-use denial and the dependent cleanup actions required by policy

### Requirement: Trellis-backed actormap transactions
r[molten.trellis_runtime.actormap_transaction] The system SHOULD provide Trellis-backed predicates for actormap delta commit/rollback, spawned object visibility, and removed object invalidation.

#### Scenario: Aborted actormap delta is invisible
r[molten.trellis_runtime.actormap_transaction.abort]
- GIVEN a prior actormap and a generated turn delta with spawned, updated, and removed objects
- WHEN the turn aborts
- THEN the predicate reports that the next committed actormap equals the prior committed actormap

#### Scenario: Removed object cannot be near-called after commit
r[molten.trellis_runtime.actormap_transaction.removed]
- GIVEN an object removed by a committed actormap delta
- WHEN a later turn attempts a near call to that object id
- THEN the predicate denies the near call because the object is no longer live in the actormap

### Requirement: Trellis-backed near/far reference admission
r[molten.trellis_runtime.near_far_refs] The system SHOULD provide Trellis-backed predicates admitting synchronous calls only for live same-vat near references and requiring asynchronous semantics for far references.

#### Scenario: Cross-vat synchronous call is denied
r[molten.trellis_runtime.near_far_refs.cross_vat]
- GIVEN a caller vat id and a target reference descriptor for a different vat or session
- WHEN the caller requests synchronous invocation
- THEN the predicate denies synchronous near-call admission and requires far-call semantics

### Requirement: Trellis-backed snapshot authority subset
r[molten.trellis_runtime.snapshot_authority] The system SHOULD provide Trellis-backed predicates ensuring object snapshot authority claims are subsets of authority already held or explicitly admitted by restore policy.

#### Scenario: Snapshot cannot mint authority
r[molten.trellis_runtime.snapshot_authority.no_mint]
- GIVEN a held-authority set and a snapshot portrait claiming authority outside that set without admitted restore grant
- WHEN the snapshot authority predicate evaluates the portrait
- THEN the predicate rejects the extra authority claim before snapshot admission or restore

### Requirement: Trellis-backed service dependency admission
r[molten.trellis_runtime.service_dependencies] The system SHOULD provide Trellis-backed predicates for service dependency startup, readiness, failure, force-run, restart, reverse dependency, and shutdown admission.

#### Scenario: Dependency readiness gates startup
r[molten.trellis_runtime.service_dependencies.ready_gate]
- GIVEN a service demand assertion and a dependency assertion requiring another service state to be ready
- WHEN the required dependency state is absent and the service is not force-run
- THEN the predicate denies startup readiness for the dependent service

### Requirement: Runtime predicate receipt naming
r[molten.trellis_runtime.predicate_receipts] Runtime applications of Trellis-backed predicates SHOULD emit receipt/evidence identifiers that name the predicate, input summary, decision, and related actor/session/reference state.

#### Scenario: Predicate decision is receipt-addressable
r[molten.trellis_runtime.predicate_receipts.addressable]
- GIVEN a runtime admission decision based on a Trellis-backed predicate
- WHEN the runtime emits evidence for the decision
- THEN the receipt or trace record identifies the predicate name, bounded input summary, decision, and affected runtime state references

### Requirement: Trellis runtime predicate integration tests
r[molten.trellis_runtime.integration_tests] The system SHOULD include integration tests showing Molten runtime admission calls Trellis-backed predicates for assertion visibility, turn commit/rollback, patterns, promises, and revocation.

#### Scenario: Runtime uses predicate before commit
r[molten.trellis_runtime.integration_tests.before_commit]
- GIVEN a runtime turn that would publish assertions and update object state
- WHEN the runtime reaches the admission boundary
- THEN tests show the relevant Trellis-backed predicate is consulted before the turn commits

### Requirement: Trellis runtime predicate property tests
r[molten.trellis_runtime.property_tests] The system SHOULD use Hegel property tests over bounded models for assertion owners, turn deltas, pattern matches, promise pipelines, revocation graphs, snapshots, and service dependencies.

#### Scenario: Generated failed turns preserve committed state
r[molten.trellis_runtime.property_tests.failed_turns]
- GIVEN generated prior state summaries and generated failed turn deltas
- WHEN the bounded model evaluates rollback
- THEN the committed state after the failed turn equals the prior committed state

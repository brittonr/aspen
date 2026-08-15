## ADDED Requirements

### Requirement: Durable-state ports are canonical and narrowly scoped
r[molten.fabric_durability.port_contracts] Aspen MUST expose versioned canonical port contracts for append-oriented durable logs, ordered key/value state, immutable snapshots, and checkpoints. Descriptors MUST bind adapter profile, namespace, generation, schemas, operation classes, atomicity domain, supported durability levels, quotas, retention capabilities, conformance refs, and non-claims. Extension code MUST NOT receive backend transactions, cursors, file handles, paths, or database objects.

#### Scenario: Extension selects required storage ports
- GIVEN a system extension requires an ordered store and immutable snapshots
- WHEN activation resolves compatible admitted descriptors
- THEN it receives opaque namespace and operation ids through typed port bindings
- AND does not gain unrelated log, filesystem, or backend authority.

#### Scenario: Unsupported operation denies
- GIVEN a selected adapter does not support range deletion or a requested durability level
- WHEN the extension submits that operation
- THEN validation denies before adapter I/O
- AND no backend fallback changes the requested semantics.

### Requirement: Durable log operations preserve sequence and declared flush semantics
r[molten.fabric_durability.durable_log] Aspen MUST define append, bounded batch append, read, scan, tail inspection, declared flush, bounded truncation, and retention operations over canonical sequence positions. Append completion MUST distinguish accepted or buffered data from data durable at the selected adapter boundary. Truncation or retention MUST require explicit authority and preconditions.

#### Scenario: Durable append survives modeled process loss
- GIVEN an append is acknowledged at a durability level that includes process-loss persistence
- WHEN the adapter is restarted under that declared failure model
- THEN recovery exposes the appended record at its canonical sequence position.

#### Scenario: Buffered append is not called durable
- GIVEN an adapter accepted an append but has not reached the requested flush boundary
- WHEN completion is reported
- THEN the outcome is buffered or pending rather than durable.

### Requirement: Ordered-store operations have explicit ordering and preconditions
r[molten.fabric_durability.ordered_store] Aspen MUST define canonical byte or value ordering, point reads, bounded ordered range scans, writes, deletes, and compare or version preconditions. Range bounds, pagination, continuation tokens, schema refs, and snapshot or read consistency semantics MUST be explicit and adapter-independent.

#### Scenario: Ordered scan is deterministic
- GIVEN an unchanged namespace and canonical range with a declared read view
- WHEN the same bounded scan is evaluated repeatedly
- THEN entries and continuation boundaries follow canonical ordering.

#### Scenario: Failed precondition does not mutate
- GIVEN a write carries an expected version that does not match current state
- WHEN the adapter evaluates it
- THEN it returns a precondition-failed outcome
- AND the requested mutation is not applied.

### Requirement: Atomic batches stay within a declared domain
r[molten.fabric_durability.atomic_batch] Aspen MUST apply a validated atomic batch entirely or not at all within the adapter, namespace, generation, object classes, operation count, byte bounds, and durability level declared by its atomicity domain. Cross-adapter, unsupported cross-namespace, over-limit, or mixed-domain batches MUST deny before mutation.

#### Scenario: Valid local batch commits atomically
- GIVEN a batch of ordered-store mutations and preconditions fits one declared atomicity domain
- WHEN all preconditions pass and the adapter reports commit
- THEN all mutations become visible at one declared local commit boundary.

#### Scenario: Cross-adapter batch is rejected
- GIVEN a batch includes operations for two storage adapters
- WHEN validation runs
- THEN the batch denies before either adapter mutates
- AND diagnostics state that no cross-adapter atomicity was selected.

### Requirement: Snapshots and recovery are generation-fenced
r[molten.fabric_durability.snapshot_recovery] Aspen MUST represent snapshots and checkpoints with canonical content refs, source namespace and generation, schema and compatibility refs, coverage boundaries, durability outcome, and creation evidence. Recovery MUST inventory durable state, validate compatibility and fencing, and fail closed on gaps, corruption, incompatible schema, unresolved uncertainty, or stale generation unless an explicit repair or quarantine policy applies.

#### Scenario: Compatible checkpoint restores
- GIVEN a durable checkpoint matches the active extension state schema and generation transition policy
- WHEN recovery validates and restores it
- THEN the service receives a canonical restored-state ref and resumes from its declared coverage boundary.

#### Scenario: Corrupt checkpoint is quarantined
- GIVEN checkpoint bytes do not match their canonical content ref or required structure
- WHEN recovery inspects them
- THEN automatic activation denies or selects an explicit earlier checkpoint policy
- AND the corrupt object is not treated as valid state.

### Requirement: Effect transactions expose reserve, commit, abort, and reconcile
r[molten.fabric_durability.effect_transaction] Aspen MAY expose a canonical effect-transaction profile with reserve or prepare, commit, abort, inspect, expiry, and reconcile operations. Its descriptor MUST state scope, durability, exclusivity, expiry, idempotency, compensation, and uncertain-outcome semantics. It MUST NOT imply universal two-phase commit or atomicity across ports that do not explicitly participate.

#### Scenario: Reserved effect commits once
- GIVEN a durable exclusive reservation and matching active generation
- WHEN commit is submitted with the same effect-transaction id
- THEN the effect commits according to the profile and duplicate commit attempts return the same terminal outcome where idempotency is declared.

#### Scenario: Unresolved reservation blocks unsafe replay
- GIVEN recovery discovers a reservation with unknown external outcome
- WHEN no safe reconciliation rule is available
- THEN automatic replay denies
- AND the transaction remains explicitly uncertain or quarantined.

### Requirement: Durable outcomes represent uncertainty
r[molten.fabric_durability.uncertain_outcomes] Aspen MUST distinguish validated, buffered, durable, failed-before-mutation, failed-after-possible-mutation, cancelled-before-mutation, cancelled-after-possible-mutation, and uncertain outcomes where relevant. Retry safety and idempotency MUST be declared per operation or profile; the host MUST NOT infer them from transport or process errors.

#### Scenario: Failure after possible commit is uncertain
- GIVEN an adapter may have committed a batch before its process or response path failed
- WHEN the shell cannot prove the outcome
- THEN it returns an uncertain result with reconciliation information rather than retrying automatically.

#### Scenario: Proven pre-mutation failure is retryable only by policy
- GIVEN validation proves no mutation occurred
- WHEN failure is returned
- THEN the result records that fact
- AND any retry still follows the selected extension or host policy.

### Requirement: Live and simulated durability preserve one contract
r[molten.fabric_durability.live_sim_parity] Aspen MUST provide live and deterministic-simulation durable-state adapters that implement the same command, outcome, ordering, atomicity, durability-state, generation, and recovery contracts. The simulation adapter MUST model buffered versus durable state, declared crash boundaries, capacity, latency, and selected corruption classes rather than behaving only as an infallible in-memory map.

#### Scenario: Shared recovery fixture runs on both adapters
- GIVEN a fixture appends, flushes, checkpoints, and restarts at declared boundaries
- WHEN it runs against live and deterministic adapters
- THEN each adapter satisfies the same observable durability contract for its declared profile.

#### Scenario: Simulated crash loses only permitted state
- GIVEN simulation crashes after buffered writes but before their selected flush boundary
- WHEN recovery runs
- THEN only state permitted by the declared durability profile may be absent.

### Requirement: Durable-state evidence is bounded
r[molten.fabric_durability.evidence] Aspen MUST emit canonical evidence for namespace admission, material commit or flush boundaries selected by the extension, checkpoints, recovery, corruption, unresolved effect transactions, and aggregate resource use. The default production profile MUST NOT require heavyweight evidence for every point read, scan entry, page access, or internal log record.

#### Scenario: Checkpoint evidence binds durable coverage
- GIVEN a checkpoint completes durably
- WHEN evidence is emitted
- THEN it binds namespace, generation, content ref, schema, coverage boundary, adapter profile, durability outcome, and non-claims.

### Requirement: Local durability does not imply distributed guarantees
r[molten.fabric_durability.non_claims] Aspen MUST state that durable-state port success proves only the declared local adapter and failure-model boundary. It MUST NOT imply replication, quorum acknowledgement, consensus, distributed transactions, serializability, linearizability, remote persistence, extension correctness, or recovery correctness beyond validated fixtures.

#### Scenario: Local commit is not quorum commit
- GIVEN an atomic batch reports durable success on one node
- WHEN evidence is rendered
- THEN it does not report replicated or consensus commit unless a separate consistency service binds that local outcome into its own evidence.

### Requirement: Durable-state validation covers success and failure
r[molten.fabric_durability.final_validation] Aspen MUST run shared adapter conformance plus positive and negative tests for sequence and key ordering, preconditions, atomic batches, unsupported domains, flush boundaries, crash points, truncation, retention, quota exhaustion, snapshots, schema mismatch, corruption, stale generations, idempotency, uncertain outcomes, effect transactions, recovery, and cleanup.

#### Scenario: Conforming adapters pass
- GIVEN live and deterministic adapters implement a declared profile
- WHEN shared conformance runs
- THEN both satisfy its canonical operation and failure semantics.

#### Scenario: Adapter overclaims durability
- GIVEN an adapter reports durable success before reaching its declared persistence boundary
- WHEN crash conformance runs
- THEN the profile fails admission with a durability invariant diagnostic.

# Runtime Spine Delta: Dataspace Delivery Idempotency

### Requirement: Operation identity is canonical and scoped
r[molten.dataspace_delivery_idempotency.spec.operation_identity] Remote and local delivery operations MUST derive operation ids from canonical scoped intent records, not from transport packet ids, wall-clock time, or host-local paths.

#### Scenario: Same intent has same operation ref
- GIVEN two deliveries with identical scope, producer, consumer, sequence, intent, payload ref, and policy refs
- WHEN operation ids are derived
- THEN the operation refs are equal

#### Scenario: Payload change changes operation identity or conflicts
- GIVEN a prior operation id for a message payload
- WHEN the same sequence is reused with a different payload ref
- THEN Molten emits a conflict denial before side effects

### Requirement: Dedup happens before side effects
r[molten.dataspace_delivery_idempotency.spec.before_commit] Dedup and sequence-window checks MUST occur before committing dataspace assertions, messages, service lifecycle events, protocol transitions, or job worker actions.

#### Scenario: Duplicate suppresses side effect
- GIVEN a delivered remote assertion already committed for an operation id
- WHEN the same operation is delivered again
- THEN Molten emits a duplicate receipt
- AND the assertion is not committed a second time

#### Scenario: Future gap denies
- GIVEN a delivery window expecting sequence `5`
- WHEN sequence `9` arrives without admitted gap policy
- THEN Molten emits a gap denial receipt
- AND no semantic side effect occurs

### Requirement: Dedup windows are retained for replay
r[molten.dataspace_delivery_idempotency.spec.retention] Dedup windows required by active sessions, jobs, services, or replay logs MUST be pinned against GC until their retention policy permits removal.

#### Scenario: Active protocol pins window
- GIVEN an active protocol session with a delivery window
- WHEN ledger GC runs
- THEN the window entries required for replay remain available

# Delivery Idempotency Delta: Replay-Bound Operation Identity

### Requirement: Delivery operation identity is canonical
r[molten.delivery.operation_identity] Molten MUST derive delivery operation ids from canonical scoped intent records that bind scope ref, producer, consumer, sequence, intent/effect kind, payload or request ref, and policy refs. Capability, authority, transport, provenance, and resource refs remain separate admission evidence and MUST NOT be inferred from the operation id alone.

#### Scenario: Equivalent operations have stable refs
- GIVEN two delivery operations with identical scope, producer, consumer, sequence, intent, payload ref, and policy refs
- WHEN Molten derives their operation ids
- THEN the operation refs are identical
- AND changing the payload ref changes the operation ref or produces a conflict denial when the sequence is reused.

### Requirement: Delivery classes are explicit evidence
r[molten.delivery.classes] Molten MUST model delivery outcomes explicitly as first commit, duplicate suppression, conflict/stale/gap denial, retry-before-side-effects, and lifecycle one-shot disclosure evidence. Molten MAY add richer ephemeral, transactional, compensating, or one-shot-external class declarations only through admitted future manifests and MUST NOT imply network-level exactly-once delivery.

#### Scenario: Duplicate suppression is visible
- GIVEN a deduped operation already committed
- WHEN the same operation is checked again
- THEN the receipt decision is `duplicate`
- AND the receipt marks the side effect as `suppress` rather than silently ignoring the retry.

### Requirement: No exact-once network claim
r[molten.delivery.no_exact_once_claim] Molten MUST document and enforce that delivery idempotency receipts bound retries and local commits but do not claim network-level exactly-once delivery, remote non-execution after timeout, authority, transport trust, provenance, policy, resource admission, or execution trust.

#### Scenario: Timeout does not prove non-execution
- GIVEN a remote operation times out before reconciliation
- WHEN the caller evaluates retry safety
- THEN it must rely on operation ids, dedup receipts, or explicit reconciliation evidence
- AND it must not infer that the remote side effect did not occur.

### Requirement: Delivery receipts are canonical
r[molten.delivery.receipt_model] Molten MUST emit canonical operation-id, delivery-window, dedup-entry, idempotency-receipt, and retry-receipt records for first accepts, commits, duplicate hits, conflicts, stale sequence denials, gap denials, retry guidance, and replay rejections.

#### Scenario: Conflict denial is receipt-backed
- GIVEN a sequence already committed for one payload
- WHEN a request reuses the same scoped operation sequence with different payload or evidence
- THEN Molten emits a conflict idempotency receipt
- AND the receipt suppresses side effects before commit.

### Requirement: Dedup ledger is durable and scoped
r[molten.delivery.dedup_ledger] Molten MUST maintain local dedup windows and dedup entries keyed by scoped operation ids, storing request payload refs, semantic result refs, first receipt refs, retention refs, and replay evidence before side effects are committed.

#### Scenario: First delivery stores dedup entry
- GIVEN a first delivery for the next expected sequence
- WHEN Molten admits it
- THEN a dedup entry and updated delivery window are stored
- AND later duplicate checks can return the prior semantic result ref.

### Requirement: Conflicting duplicates deny before effects
r[molten.delivery.conflict_detection] Molten MUST reject duplicate operation ids or scoped sequence keys whose payload, evidence, or semantic identity conflicts with the first committed entry.

#### Scenario: Changed payload conflicts
- GIVEN operation sequence `1` already committed with payload ref `A`
- WHEN the same scoped sequence is checked with payload ref `B`
- THEN Molten emits a conflict receipt
- AND `should_commit_side_effect` is false.

### Requirement: Sequence windows bound replay
r[molten.delivery.sequence_windows] Molten MUST enforce per-scope sequence windows that reject stale sequences, deny or retry future gaps according to policy, and avoid global sequence coupling across independent scopes.

#### Scenario: Independent scopes do not share a global sequence
- GIVEN two different remote-topic scopes
- WHEN one scope advances to a later sequence
- THEN another scope can still accept its first sequence independently.

### Requirement: Retry schedule is deterministic evidence
r[molten.delivery.retry_schedule] Molten MUST make retry guidance deterministic by binding retry receipts to operation refs, delivery windows, expected sequence positions, diagnostics, and replay evidence before any side effect is attempted. Timeout receipts MUST be evidence only and MUST NOT prove remote non-execution.

#### Scenario: Gap retry points at expected sequence
- GIVEN a delivery window expecting sequence `2`
- WHEN sequence `4` arrives under retry gap policy
- THEN Molten emits a retry decision and stores a retry receipt
- AND the retry receipt names the expected sequence boundary.

### Requirement: Dataspace delivery checks happen before commit
r[molten.delivery.dataspace_effects] Molten MUST apply delivery idempotency checks before remote dataspace and node-control ingress deliveries commit local side effects or enqueue durable requests. Local effect-request idempotency MAY be added by future handler profiles and MUST NOT be inferred from unrelated receipts.

#### Scenario: Duplicate remote assertion suppresses commit
- GIVEN a remote dataspace assertion already committed for an operation id
- WHEN the same delivery arrives again
- THEN Molten emits a duplicate receipt
- AND the assertion is not committed a second time.

### Requirement: Storage mutation dedup is a future extension
r[molten.delivery.storage_mutations] Typed storage writes and upgrade storage migrations MUST NOT claim delivery idempotency unless they carry explicit scoped operation-id and idempotency receipt evidence. Full storage mutation dedup remains a future admitted extension of this delivery model.

#### Scenario: Storage cannot use unrelated delivery receipt as authority
- GIVEN a typed storage write has no matching operation-id/idempotency receipt evidence
- WHEN the write is evaluated
- THEN the delivery model grants no idempotency or authority for that mutation
- AND the storage subsystem must use its own admission gates.

### Requirement: Choreography delivery indices are future explicit evidence
r[molten.delivery.choreography] Choreography send, receive, and choice delivery MUST NOT rely on ambient network retries for exactly-once behavior. Protocol/session/op indices MAY be integrated as future explicit operation-id evidence and MUST remain separate from protocol authority and transport admission.

#### Scenario: Protocol authority is separate from idempotency
- GIVEN a protocol session message has a delivery operation id
- WHEN protocol admission is evaluated
- THEN the idempotency evidence does not replace protocol authority, policy, resource, or transport checks.

### Requirement: Remote jobs and upgrades bind operation evidence where present
r[molten.delivery.remote_jobs_upgrades] Remote job worker, control-plane, remote ingress, and upgrade-adjacent workflows SHOULD bind operation ids where side effects may be retried. Job stage operation ids and control-command operation ids MUST remain evidence only and MUST NOT grant execution, provenance, source-gate, or upgrade authority.

#### Scenario: Job operation id is evidence only
- GIVEN a job worker request carries an operation id
- WHEN the worker evaluates execution authority
- THEN the operation id can support dedup/replay evidence
- AND separate authority, provenance, policy, source-gate, and resource evidence is still required.

### Requirement: Duplicate behavior is tested
r[molten.delivery.duplicate_tests] Molten SHOULD test that duplicate operation ids return prior receipts or semantic result refs and that conflicting duplicate payloads deny before side effects.

#### Scenario: Duplicate test suppresses side effect
- GIVEN the duplicate-delivery test has admitted a first operation
- WHEN it repeats the same scoped operation
- THEN the duplicate receipt is asserted
- AND `should_commit_side_effect` is false.

### Requirement: Replay window behavior is tested
r[molten.delivery.replay_window_tests] Molten SHOULD test stale, future-gap, retry, and independent-scope replay-window behavior.

#### Scenario: Stale sequence test denies
- GIVEN a delivery window has advanced past sequence `1`
- WHEN sequence `0` arrives
- THEN the test observes a stale receipt
- AND no side effect is committed.

### Requirement: Timeout semantics are documented as non-proof
r[molten.delivery.timeout_tests] Molten SHOULD cover timeout semantics by asserting that timeout or retry evidence never proves remote non-execution and cannot replace idempotency keys, reconciliation, or admission gates.

#### Scenario: Timeout evidence is not authority
- GIVEN a timeout diagnostic exists for a remote operation
- WHEN another subsystem checks delivery evidence
- THEN the timeout diagnostic is treated as evidence only
- AND it does not grant authority or prove the remote operation was absent.

### Requirement: Delivery properties cover dedup invariants
r[molten.delivery.property_tests] Molten SHOULD include bounded Hegel-style property coverage for operation identity determinism, duplicate suppression, conflict denial, independent-scope sequence windows, and idempotent replay behavior.

#### Scenario: Generated scopes stay independent
- GIVEN generated delivery checks for independent scopes
- WHEN one scope advances
- THEN another scope's first delivery remains admissible
- AND no global sequence is required.

### Requirement: Dataspace operation identity is canonical and scoped
r[molten.dataspace_delivery_idempotency.spec.operation_identity] Remote and local delivery operations MUST derive operation ids from canonical scoped intent records, not from transport packet ids, wall-clock time, or host-local paths.

#### Scenario: Same intent has same operation ref
- GIVEN two deliveries with identical scope, producer, consumer, sequence, intent, payload ref, and policy refs
- WHEN operation ids are derived
- THEN the operation refs are equal

#### Scenario: Payload change changes operation identity or conflicts
- GIVEN a prior operation id for a message payload
- WHEN the same sequence is reused with a different payload ref
- THEN Molten emits a conflict denial before side effects

### Requirement: Dataspace dedup happens before side effects
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

### Requirement: CLI emits canonical delivery records
r[molten.delivery_idempotency_ux.spec.cli_scope_operation] The delivery idempotency CLI MUST emit canonical Preserves scope profile and operation id records using the same hashing and validation rules as the runtime delivery idempotency module.

#### Scenario: Operation id is materialized
- GIVEN a scope profile/name, producer, consumer, sequence, intent, payload ref, and policy refs
- WHEN an operator runs `molten test delivery operation-id`
- THEN Molten emits an `operation-id-v1` artifact
- AND the printed operation ref is the canonical ref of that artifact

### Requirement: CLI checks produce replay-bound idempotency receipts
r[molten.delivery_idempotency_ux.spec.cli_check_receipts] The delivery idempotency CLI MUST run checks against an explicit local store root and MUST emit the same idempotency receipt records used by runtime ingress paths.

#### Scenario: Duplicate delivery is suppressed
- GIVEN a delivery check for a scoped sequence has already emitted a `first` receipt
- WHEN the same scope, producer, consumer, sequence, payload, policy, and evidence are checked again
- THEN Molten emits a `duplicate` idempotency receipt
- AND the duplicate receipt binds the prior first receipt
- AND the side effect is marked `suppress`

### Requirement: Delivery CLI receipts are evidence only
r[molten.delivery_idempotency_ux.spec.evidence_only] Delivery idempotency CLI receipts MUST NOT grant transport, authority, provenance, policy, resource, or execution trust.

#### Scenario: Evidence-only receipt
- GIVEN a pass delivery idempotency receipt
- WHEN another subsystem needs authority or provenance admission
- THEN it must still resolve explicit authority, policy, resource, provenance, and transport evidence
- AND it must not treat the delivery idempotency receipt as a grant

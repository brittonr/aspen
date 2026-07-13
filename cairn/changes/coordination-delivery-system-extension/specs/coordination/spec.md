## ADDED Requirements

### Requirement: Durable delivery is a versioned coordination extension
r[molten.coordination_delivery.versioned_extension] Molten MUST implement visibility leases, acknowledgement, negative acknowledgement, retry, dead-letter, and redrive behavior as a separately versioned system extension. Existing base FIFO enqueue and dequeue schemas and semantics MUST NOT silently change, and ordinary actor or job traffic MUST NOT enter this extension without an explicit delivery request.

#### Scenario: Existing FIFO remains unchanged
- GIVEN a caller uses the accepted base queue schema
- WHEN it enqueues and dequeues an item
- THEN the existing deterministic FIFO semantics MUST apply
- AND no visibility lease or acknowledgement requirement may be inferred.

#### Scenario: Extension operation requires its manifest
- GIVEN a caller submits an ack operation without an admitted coordination-delivery manifest and queue profile
- WHEN request admission runs
- THEN it MUST deny before coordination mutation.

### Requirement: Claims use canonical visibility leases
r[molten.coordination_delivery.claim_lease] Molten MUST represent a claimed item with canonical queue, item ref, delivery id, consumer, attempt, fencing token, visibility deadline, consistency epoch, service generation, policy refs, and receipt refs. Claim transitions MUST be pure and MUST preserve queue state on denial.

#### Scenario: Eligible item is claimed
- GIVEN an available item and admitted current consumer request
- WHEN the claim transition commits
- THEN the item MUST move to an in-flight state with a bounded visibility lease and canonical delivery token.

#### Scenario: Concurrent claim is denied
- GIVEN an item already has a current unexpired claim
- WHEN another consumer attempts to claim it without an admitted takeover transition
- THEN the request MUST deny without changing owner, attempt, or deadline.

### Requirement: Visibility uses admitted logical time
r[molten.coordination_delivery.logical_time] Molten MUST derive visibility deadlines, lease expiry, retry eligibility, and backoff from admitted fabric logical-time facts or recorded time effects. Wall-clock reads, process timers, and local stale observations MUST NOT independently mutate delivery state.

#### Scenario: Lease expires at admitted time
- GIVEN a claim deadline and a current admitted logical-time observation at or after that deadline
- WHEN expiry evaluation commits through current consistency evidence
- THEN the item MAY become retry-eligible according to policy.

#### Scenario: Local timer cannot expire a claim alone
- GIVEN a process timer fires but no admitted currentness or logical-time transition is available
- WHEN the shell requests expiry
- THEN mutation MUST deny or defer.

### Requirement: Ack, nack, and extension are fenced
r[molten.coordination_delivery.fenced_completion] Molten MUST require the current delivery token, service generation, consistency epoch, owner or delegated authority, operation id, policy, and resource evidence for ack, nack, lease extension, and redrive mutations. Stale, expired, wrong-owner, wrong-generation, duplicate-conflicting, or local-stale evidence MUST deny without mutation.

#### Scenario: Current ack completes delivery
- GIVEN a consumer presents the current delivery token and admitted completion evidence before expiry
- WHEN ack commits
- THEN the item MUST enter its terminal acknowledged state and the receipt MUST bind the protected transition.

#### Scenario: Stale ack is rejected
- GIVEN an earlier delivery token has been superseded by retry or reassignment
- WHEN the earlier consumer submits ack
- THEN ack MUST deny and MUST NOT remove or complete the current delivery.

### Requirement: Retry, DLQ, and redrive policy is explicit
r[molten.coordination_delivery.retry_dlq_policy] Molten MUST require versioned policy for retryable failure classes, maximum attempts, logical backoff, ordering posture, dead-letter target, DLQ capacity and retention, poison-item handling, and redrive authority. Missing, malformed, unsupported, or over-bound policy MUST deny rather than select ambient defaults.

#### Scenario: Retryable nack schedules another attempt
- GIVEN a current claim is nacked with an admitted retryable failure and attempts remain
- WHEN the transition commits
- THEN the item MUST record the failed attempt and a deterministic next-eligibility fact.

#### Scenario: Exhausted item enters DLQ
- GIVEN an item reaches its admitted maximum attempts
- WHEN expiry or nack processing commits
- THEN it MUST move to the configured dead-letter state rather than being delivered again automatically.

#### Scenario: Unauthorized redrive is denied
- GIVEN a caller can read a DLQ item but lacks redrive authority
- WHEN it requests redrive
- THEN the request MUST deny without moving the item.

### Requirement: Delivery state carries refs rather than large payloads
r[molten.coordination_delivery.content_refs] Molten MUST keep large item bytes, executables, and inputs outside coordination and consistency state. Delivery records MUST carry bounded canonical content refs and metadata, and consumers MUST pass separate content, provenance, authority, policy, resource, and execution admission before fetching or executing payloads.

#### Scenario: Ref-backed item is admitted
- GIVEN a queue item names a valid canonical content ref within metadata bounds
- WHEN enqueue admission passes
- THEN coordination MAY commit the ref and bounded delivery metadata.

#### Scenario: Inline large payload is denied
- GIVEN a delivery request embeds bytes beyond the admitted metadata bound
- WHEN request validation runs
- THEN it MUST deny before appending coordination state.

### Requirement: Delivery commits through admitted consistency and durability
r[molten.coordination_delivery.consistency_durability] Molten MUST apply claim, ack, nack, expiry, retry, DLQ, and redrive mutations through an admitted consistency-group application state machine and declared durable-state profile. Mutation receipts MUST bind normalized currentness, operation id, before and after state refs, durability outcome, and active engine epoch.

#### Scenario: Claim binds current commit evidence
- GIVEN a valid claim request and active consistency group
- WHEN the claim commits durably
- THEN its receipt MUST bind normalized commit/currentness evidence and the resulting delivery state ref.

#### Scenario: Local-stale state cannot complete delivery
- GIVEN a consumer presents only local-stale queue ownership evidence
- WHEN completion admission runs
- THEN it MUST deny until current fencing or consistency evidence is supplied.

### Requirement: Delivery semantics and non-claims are tested
r[molten.coordination_delivery.final_validation] Molten MUST test positive claim, ack, nack, retry, DLQ, redrive, restart, and bounded at-least-once-style flows plus negative duplicate, stale token, wrong owner, wrong epoch, expiry race, partition, crash-after-claim, over-attempt, missing authority, inline payload, overload, and exact-once overclaim cases.

#### Scenario: Crash after claim permits bounded redelivery
- GIVEN a consumer crashes after a committed claim and before ack
- WHEN the lease expires under admitted time and currentness
- THEN policy MAY make the item eligible for another attempt
- AND evidence MUST retain the prior delivery id and attempt.

#### Scenario: Queue receipt cannot prove exact-once effects
- GIVEN one item has a passing acknowledged delivery receipt
- WHEN a claim asserts that an external side effect occurred exactly once
- THEN claim validation MUST deny unless separate idempotency and external-effect evidence supports it.

## ADDED Requirements

### Requirement: Dedup acceptance does not prove publication
r[molten.audit_f03.recover_publication] Ingress MUST reconcile dedup acceptance with exact publication observations before it reports successful enqueue or requests recovery publication.

#### Scenario: Known incomplete publication recovers
- GIVEN committed dedup intent and authoritative absence of both queue publication and dispatch
- WHEN reconciliation admits recovery for the same request
- THEN the shell publishes that request through the normal queue boundary

#### Scenario: Missing evidence cannot become silent success
- GIVEN an exact duplicate with no available queue receipt
- WHEN reconciliation lacks sufficient publication observations
- THEN ingress reports an unresolved outcome instead of successful enqueue

### Requirement: Recovery suppresses repeated effects
r[molten.audit_f03.no_blind_replay] Recovery MUST inspect exact inbox and dispatch observations and MUST NOT infer permission to repeat effects from a missing receipt.

#### Scenario: Inbox publication survives receipt loss
- GIVEN the exact inbox request exists after a crash before queue receipt publication
- WHEN recovery validates its identity
- THEN recovery reuses the publication without another enqueue or automatic dispatch

#### Scenario: Dispatched request is not enqueued again
- GIVEN matching dispatch evidence and an absent inbox and queue receipt
- WHEN the same ingress envelope returns
- THEN recovery suppresses enqueue and preserves the historical dispatch result

### Requirement: Uncertainty blocks mutation
r[molten.audit_f03.uncertainty] Ingress recovery MUST preserve unknown outcomes and block normal effects after ambiguous persistence, conflicting identity, or unavailable required observations.

#### Scenario: Reconciled observations permit a bounded result
- GIVEN healthy reopened storage and consistent exact operation observations
- WHEN reconciliation evaluates those facts
- THEN it returns the supported publication result without changing operation identity

#### Scenario: Commit error does not prove absence
- GIVEN an ambiguous commit or synchronization error
- WHEN ingress recovery evaluates the request
- THEN it preserves uncertainty and performs no blind replay or successful acknowledgement

### Requirement: Rejection preserves prior state
r[molten.audit_f03.preserve_state] Rejected ingress recovery MUST preserve prior dedup, inbox, and dispatch state and MUST NOT rewrite conflicting evidence as success.

#### Scenario: Exact duplicate preserves identity
- GIVEN matching durable publication and dedup evidence
- WHEN duplicate reconciliation completes
- THEN the existing request identity and publication remain unchanged

#### Scenario: Conflicting payload denies recovery
- GIVEN queue evidence whose payload does not match the dedup request reference
- WHEN recovery evaluates the conflict
- THEN recovery denies mutation and retains the conflicting observations for diagnosis

### Requirement: Recovery validation states its limits
r[molten.audit_f03.validation] F03 validation MUST include normal repository publication-boundary tests and distinguish static evidence, simulated faults, and executed persistence observations.

#### Scenario: Reopen tests cover both publication windows
- GIVEN tests for crashes after dedup and after enqueue before receipt publication
- WHEN those tests reopen controlled durable state
- THEN results identify observed recovery behavior and exact tested boundaries

#### Scenario: Local tests do not prove consensus
- GIVEN passing local recovery tests or the original static finding
- WHEN completion receipts state their claims
- THEN they do not claim power-loss durability, exactly-once external effects, or consensus recovery

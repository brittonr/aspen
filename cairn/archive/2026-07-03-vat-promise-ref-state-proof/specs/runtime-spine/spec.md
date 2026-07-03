## ADDED Requirements

### Requirement: Vat promise states transition lawfully
r[molten.vat_ref_state_proof.promise_lifecycle] Molten MUST prove promise and vow states transition only through declared pending, resolved, broken, cancelled, timed-out, or causal-failure paths and reject unresolved, stale, or contradictory pipeline use.

#### Scenario: Unresolved pipeline use denies
- GIVEN a pending promise with no admitted resolution or pipeline proof
- WHEN a dependent call attempts to use the promised value as resolved
- THEN the predicate receipt decision is `deny`
- AND no authority or message delivery is emitted from the unresolved value.

### Requirement: Near far refs preserve locality and authority
r[molten.vat_ref_state_proof.reference_lifetime] Molten MUST prove that near refs remain local, far refs remain asynchronous, distributed handoffs require admission, and stale or revoked distributed refs deny before use.

#### Scenario: Stale far ref use denies
- GIVEN a distributed far ref with a revocation or stale-session marker
- WHEN an actor attempts to use it
- THEN the predicate receipt decision is `deny`
- AND diagnostics identify stale or revoked reference evidence.

### Requirement: Vat rollback and cleanup do not leak live state
r[molten.vat_ref_state_proof.rollback_cleanup] Molten MUST prove that actormap rollback and revocation cleanup remove pending calls, live refs, observers, assertions, and authority snapshots that depend on the rolled-back or revoked state.

#### Scenario: Rollback leaves no assertion leak
- GIVEN a transaction that staged an assertion and then denied
- WHEN rollback evidence is replayed
- THEN the final snapshot omits the staged assertion
- AND a cleanup or predicate receipt binds the rollback decision.

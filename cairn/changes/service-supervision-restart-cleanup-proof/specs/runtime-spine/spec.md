## ADDED Requirements

### Requirement: Service dependency wait performs no start side effects
r[molten.service_state_machine_proof.dependency_wait_no_start] Molten MUST prove that service demand evaluation emits dependency-wait lifecycle evidence and performs no actor, adapter, readiness, or resource start side effects when required dependencies or admission evidence are missing or stale.

#### Scenario: Missing dependency waits without start
- GIVEN a demanded service whose required dependency has no ready status assertion
- WHEN service demand evaluation runs
- THEN Molten emits dependency-wait lifecycle evidence
- AND no service-owned readiness assertion or adapter start side effect is committed.

### Requirement: Service restart traces are bounded and replayable
r[molten.service_state_machine_proof.bounded_restart_trace] Molten MUST prove restart decisions are bounded by explicit restart policy, authority state, logical resource budgets, and recorded lifecycle refs, and exhausted restart budgets MUST deny deterministically.

#### Scenario: Restart budget exhaustion denies
- GIVEN service failure evidence after the configured restart budget is exhausted
- WHEN supervision evaluates the restart decision
- THEN the lifecycle or supervisor receipt decision is `deny`
- AND diagnostics identify the exhausted restart bound.

### Requirement: Service cleanup is ownership-bound and idempotent
r[molten.service_state_machine_proof.cleanup_idempotence] Molten MUST prove service cleanup retracts only service-owned assertions, subscriptions, live refs, and admitted resources, and repeated cleanup attempts leave runtime state unchanged while producing stable cleanup evidence.

#### Scenario: Repeated service cleanup is stable
- GIVEN a service whose owned cleanup has already completed
- WHEN cleanup is requested again with the same evidence
- THEN Molten emits stable cleanup evidence or an idempotent no-op receipt
- AND runtime state remains unchanged.

# Coordination Delta: resource reconciliation controllers

### Requirement: Reconciliation cores compute pure action plans
r[molten.reconciliation.pure_plan_core] Molten controller reconciliation MUST separate a pure deterministic planning core from the imperative shell. The core MUST accept resource desired-state refs, observed-state summaries, generation, dependency summaries, policy/authority summaries, and prior receipt refs, and MUST return a no-op, action plan, retry plan, or denial diagnostics without performing I/O, reading clocks, mutating stores, or invoking adapters.

#### Scenario: Desired and observed state already match
- GIVEN a resource whose desired-state ref matches the admitted observed-state summary for the current generation
- WHEN the reconciliation core evaluates the resource
- THEN it returns a no-op plan with condition candidates and no effect intents.

#### Scenario: Core cannot rely on ambient state
- GIVEN a reconciliation decision that requires data not present in the explicit input summaries
- WHEN the core evaluates the resource
- THEN it denies or returns an incomplete-input diagnostic instead of reading logs, files, clocks, stores, or adapters.

### Requirement: Reconciliation work queues are deterministic and idempotent
r[molten.reconciliation.idempotent_work_queue] Molten SHOULD schedule reconciliation through deterministic work-queue summaries that bind resource refs, generations, causes, coalescing decisions, retry attempts, named backoff profiles, and terminal failure conditions. Work queues MUST reject pass claims for skipped generations, duplicate semantic work, unbounded retry, or unnamed backoff values.

#### Scenario: Queue coalesces repeated current-generation events
- GIVEN multiple watch events for the same resource generation that require the same reconcile cause
- WHEN the queue summarizes pending work
- THEN it may coalesce them into one reconcile item while binding the coalesced event refs.

#### Scenario: Unbounded retry denies
- GIVEN a failing reconcile item without a named retry budget or backoff profile
- WHEN the queue attempts to schedule another retry
- THEN Molten denies the retry schedule
- AND diagnostics identify the missing governance input.

### Requirement: Reconciliation success binds admitted effects and status
r[molten.reconciliation.effect_commit_receipts] Molten MUST report reconciliation success only when an action plan for the current resource generation is admitted and every required effect, commit, and status update is bound by receipt refs. Status success MUST deny for stale generations, duplicate semantic commits, missing effect receipts, or success claims without an admitted plan.

#### Scenario: Admitted plan commits and updates status
- GIVEN a reconciliation plan for the current generation with admitted effect intents and successful effect receipts
- WHEN the controller records completion
- THEN Molten emits a reconciliation receipt binding the plan ref, admission refs, effect refs, status update refs, and generation.

#### Scenario: Success without effect receipt denies
- GIVEN a controller claims resource reconciliation success but omits a required effect receipt
- WHEN Molten evaluates the completion claim
- THEN it denies the success status
- AND diagnostics identify the missing effect evidence.

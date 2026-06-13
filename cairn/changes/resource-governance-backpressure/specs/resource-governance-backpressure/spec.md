# resource governance backpressure Delta Spec

## ADDED Requirements

### Requirement: Define canonical resource grant and consumption records with scope, kind, amount, rate/window, expiry, parent pool, policy refs, and evidence refs
r[molten.resources.grant_model] Define canonical resource grant and consumption records with scope, kind, amount, rate/window, expiry, parent pool, policy refs, and evidence refs.

### Requirement: Define initial resource kinds for turns, CPU/fuel, memory, mailbox slots, assertions, blob bytes, storage bytes, network messages/bytes, effect calls, and trace bytes
r[molten.resources.kinds] Define initial resource kinds for turns, CPU/fuel, memory, mailbox slots, assertions, blob bytes, storage bytes, network messages/bytes, effect calls, and trace bytes.

### Requirement: Document that resource grants do not imply data access or capability authority
r[molten.resources.no_data_authority] Document that resource grants do not imply data access or capability authority.

### Requirement: Emit receipts for grant, consume, throttle, deny, renew, revoke, and cleanup decisions
r[molten.resources.receipts] Emit receipts for grant, consume, throttle, deny, renew, revoke, and cleanup decisions.

### Requirement: Enforce deterministic mailbox bounds and overflow behavior
r[molten.resources.mailbox_bounds] Enforce deterministic mailbox bounds and overflow behavior.

### Requirement: Enforce actor turn budgets and deterministic cancellation/yield points
r[molten.resources.turn_budgets] Enforce actor turn budgets and deterministic cancellation/yield points.

### Requirement: Enforce dataspace assertion and subscription count limits
r[molten.resources.assertion_bounds] Enforce dataspace assertion and subscription count limits.

### Requirement: Add deterministic scheduler fairness/backpressure policy independent of OS thread timing
r[molten.resources.scheduler_fairness] Add deterministic scheduler fairness/backpressure policy independent of OS thread timing.

### Requirement: Wire Wasmtime execution to admitted fuel/epoch/deadline budgets
r[molten.resources.wasmtime_fuel] Wire Wasmtime execution to admitted fuel/epoch/deadline budgets.

### Requirement: Add cooperative budget checkpoints for Steel and native actors
r[molten.resources.steel_native_budgets] Add cooperative budget checkpoints for Steel and native actors.

### Requirement: Enforce blob, storage, network, remote sync, and trace-volume budgets in adapters
r[molten.resources.blob_storage_network] Enforce blob, storage, network, remote sync, and trace-volume budgets in adapters.

### Requirement: Feed resource budgets into distributed job DAG placement and fusion decisions
r[molten.resources.job_dag_planning] Feed resource budgets into distributed job DAG placement and fusion decisions.

### Requirement: Add tests for deterministic queue overflow, throttling, denial, and supervisor signaling
r[molten.resources.backpressure_tests] Add tests for deterministic queue overflow, throttling, denial, and supervisor signaling.

### Requirement: Add replay tests proving budget decisions reproduce under the same profile and seed/log
r[molten.resources.replay_tests] Add replay tests proving budget decisions reproduce under the same profile and seed/log.

### Requirement: Add tests that revoked/expired budgets deny future work and clean up dependent state
r[molten.resources.revocation_tests] Add tests that revoked/expired budgets deny future work and clean up dependent state.

### Requirement: Add Hegel property tests for budget monotonicity, queue bounds, and no-silent-drop invariants
r[molten.resources.property_tests] Add Hegel property tests for budget monotonicity, queue bounds, and no-silent-drop invariants.


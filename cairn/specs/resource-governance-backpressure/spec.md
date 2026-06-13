# Resource Governance Backpressure Specification

## Purpose

Defines the `resource-governance-backpressure` capability.

## Requirements

### Requirement: System MUST Define canonical resource grant and consumption records with scope, kind, amount, rate/window, expiry, parent pool, policy refs, and evidence refs
r[molten.resources.grant_model] The system MUST Define canonical resource grant and consumption records with scope, kind, amount, rate/window, expiry, parent pool, policy refs, and evidence refs.

### Requirement: System MUST Define initial resource kinds for turns, CPU/fuel, memory, mailbox slots, assertions, blob bytes, storage bytes, network messages/bytes, effect calls, and trace bytes
r[molten.resources.kinds] The system MUST Define initial resource kinds for turns, CPU/fuel, memory, mailbox slots, assertions, blob bytes, storage bytes, network messages/bytes, effect calls, and trace bytes.

### Requirement: System MUST Document that resource grants do not imply data access or capability authority
r[molten.resources.no_data_authority] The system MUST Document that resource grants do not imply data access or capability authority.

### Requirement: System MUST Emit receipts for grant, consume, throttle, deny, renew, revoke, and cleanup decisions
r[molten.resources.receipts] The system MUST Emit receipts for grant, consume, throttle, deny, renew, revoke, and cleanup decisions.

### Requirement: System MUST Enforce deterministic mailbox bounds and overflow behavior
r[molten.resources.mailbox_bounds] The system MUST Enforce deterministic mailbox bounds and overflow behavior.

### Requirement: System MUST Enforce actor turn budgets and deterministic cancellation/yield points
r[molten.resources.turn_budgets] The system MUST Enforce actor turn budgets and deterministic cancellation/yield points.

### Requirement: System MUST Enforce dataspace assertion and subscription count limits
r[molten.resources.assertion_bounds] The system MUST Enforce dataspace assertion and subscription count limits.

### Requirement: System MUST Add deterministic scheduler fairness/backpressure policy independent of OS thread timing
r[molten.resources.scheduler_fairness] The system MUST Add deterministic scheduler fairness/backpressure policy independent of OS thread timing.

### Requirement: System MUST Wire Wasmtime execution to admitted fuel/epoch/deadline budgets
r[molten.resources.wasmtime_fuel] The system MUST Wire Wasmtime execution to admitted fuel/epoch/deadline budgets.

### Requirement: System MUST Add cooperative budget checkpoints for Steel and native actors
r[molten.resources.steel_native_budgets] The system MUST Add cooperative budget checkpoints for Steel and native actors.

### Requirement: System MUST Enforce blob, storage, network, remote sync, and trace-volume budgets in adapters
r[molten.resources.blob_storage_network] The system MUST Enforce blob, storage, network, remote sync, and trace-volume budgets in adapters.

### Requirement: System MUST Feed resource budgets into distributed job DAG placement and fusion decisions
r[molten.resources.job_dag_planning] The system MUST Feed resource budgets into distributed job DAG placement and fusion decisions.

### Requirement: System MUST Add tests for deterministic queue overflow, throttling, denial, and supervisor signaling
r[molten.resources.backpressure_tests] The system MUST Add tests for deterministic queue overflow, throttling, denial, and supervisor signaling.

### Requirement: System MUST Add replay tests proving budget decisions reproduce under the same profile and seed/log
r[molten.resources.replay_tests] The system MUST Add replay tests proving budget decisions reproduce under the same profile and seed/log.

### Requirement: System MUST Add tests that revoked/expired budgets deny future work and clean up dependent state
r[molten.resources.revocation_tests] The system MUST Add tests that revoked/expired budgets deny future work and clean up dependent state.

### Requirement: System MUST Add Hegel property tests for budget monotonicity, queue bounds, and no-silent-drop invariants
r[molten.resources.property_tests] The system MUST Add Hegel property tests for budget monotonicity, queue bounds, and no-silent-drop invariants.

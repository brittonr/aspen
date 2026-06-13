## Phase 1: Resource model

- [x] [serial] r[molten.resources.grant_model] Define canonical resource grant and consumption records with scope, kind, amount, rate/window, expiry, parent pool, policy refs, and evidence refs.
- [x] [serial] r[molten.resources.kinds] Define initial resource kinds for turns, CPU/fuel, memory, mailbox slots, assertions, blob bytes, storage bytes, network messages/bytes, effect calls, and trace bytes.
- [x] [parallel] r[molten.resources.no_data_authority] Document that resource grants do not imply data access or capability authority.
- [x] [parallel] r[molten.resources.receipts] Emit receipts for grant, consume, throttle, deny, renew, revoke, and cleanup decisions.

## Phase 2: Local backpressure

- [x] [serial] r[molten.resources.mailbox_bounds] Enforce deterministic mailbox bounds and overflow behavior.
- [x] [serial] r[molten.resources.turn_budgets] Enforce actor turn budgets and deterministic cancellation/yield points.
- [x] [serial] r[molten.resources.assertion_bounds] Enforce dataspace assertion and subscription count limits.
- [x] [parallel] r[molten.resources.scheduler_fairness] Add deterministic scheduler fairness/backpressure policy independent of OS thread timing.

## Phase 3: Adapter budgets

- [x] [serial] r[molten.resources.wasmtime_fuel] Wire Wasmtime execution to admitted fuel/epoch/deadline budgets.
- [x] [parallel] r[molten.resources.steel_native_budgets] Add cooperative budget checkpoints for Steel and native actors.
- [x] [parallel] r[molten.resources.blob_storage_network] Enforce blob, storage, network, remote sync, and trace-volume budgets in adapters.
- [x] [parallel] r[molten.resources.job_dag_planning] Feed resource budgets into distributed job DAG placement and fusion decisions.

## Phase 4: Tests

- [x] [serial] r[molten.resources.backpressure_tests] Add tests for deterministic queue overflow, throttling, denial, and supervisor signaling.
- [x] [serial] r[molten.resources.replay_tests] Add replay tests proving budget decisions reproduce under the same profile and seed/log.
- [x] [parallel] r[molten.resources.revocation_tests] Add tests that revoked/expired budgets deny future work and clean up dependent state.
- [x] [parallel] r[molten.resources.property_tests] Add Hegel property tests for budget monotonicity, queue bounds, and no-silent-drop invariants.

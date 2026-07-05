# Tasks: resource-reconciliation-controllers

## Phase 1: Reconcile core and plans

- [x] [serial] r[molten.reconciliation.pure_plan_core] Define pure reconcile input summaries, plan outputs, no-op decisions, retry decisions, and denial diagnostics over resource desired/observed refs.
- [x] [parallel] r[molten.reconciliation.pure_plan_core] Add positive fixtures for no-op and planned action decisions and negative fixtures proving the core does not depend on logs, clocks, filesystem reads, or ambient adapter state.

## Phase 2: Work queue and effect binding

- [x] [serial] r[molten.reconciliation.idempotent_work_queue] Implement deterministic work-queue summaries for coalescing, generation ordering, retry attempts, named backoff profiles, and terminal failure conditions.
- [x] [parallel] r[molten.reconciliation.idempotent_work_queue] Add positive queue fixtures and negative fixtures for skipped generation, duplicate semantic work, unbounded retry, and unnamed backoff values.
- [x] [serial] r[molten.reconciliation.effect_commit_receipts] Bind action plans to admission receipts, effect receipts, status updates, and resource generation before success can be reported.
- [x] [parallel] r[molten.reconciliation.effect_commit_receipts] Add positive apply/status fixtures and negative fixtures for missing effect receipts, stale generation, duplicate commit evidence, and status success without an admitted plan.

## Phase 3: Documentation and validation

- [x] [serial] r[molten.reconciliation.pure_plan_core] Documented controller reconciliation as Kubernetes-inspired but Molten-specific, and ran focused reconciliation tests plus `cairn validate --root .`

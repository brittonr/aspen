# Tasks: placement-lifecycle-governance

## Phase 1: Placement governance

- [x] [serial] r[molten.placement.resource_requests_limits_quotas] Define pure placement inputs for requests, limits, quotas, priority refs, capacity evidence, and assignment authority.
- [x] [parallel] r[molten.placement.resource_requests_limits_quotas] Add positive placement fixtures and negative fixtures for over-quota requests, malformed capacity refs, limit/request inversions, missing priority policy, and unauthorized assignment.
- [x] [serial] r[molten.placement.constraint_profiles_taints_tolerations] Implement placement constraint, affinity/anti-affinity, taint, toleration, and defer/deny diagnostics over explicit summaries.
- [x] [parallel] r[molten.placement.constraint_profiles_taints_tolerations] Add positive constraint fixtures and negative fixtures for unsatisfied constraints, unsupported selector operators, missing tolerations, and hidden node assumptions.

## Phase 2: Lifecycle and cleanup governance

- [x] [serial] r[molten.lifecycle.probes_restart_backoff] Define lifecycle probe, readiness, liveness, startup, graceful shutdown, restart, and named backoff profile records.
- [x] [parallel] r[molten.lifecycle.probes_restart_backoff] Add positive lifecycle fixtures and negative fixtures for flapping probes, restart loops without budgets, unnamed backoff values, and status claims without probe evidence.
- [x] [serial] r[molten.lifecycle.gc_cleanup_gates] Implement cleanup-plan validation for owner refs, finalizers, pins, retention policy refs, and authority evidence (covered by declarative-resource-records DeletionGateInput).
- [x] [parallel] r[molten.lifecycle.gc_cleanup_gates] Add positive cleanup fixtures and negative fixtures for live owners, missing finalizer receipts, pinned artifacts, retention holds, and unauthorized deletion (covered by declarative-resource-records tests).

## Phase 3: Documentation and validation

- [x] [serial] r[molten.placement.resource_requests_limits_quotas] Documented placement/lifecycle governance as Kubernetes-inspired but not pod-compatible, and ran focused governance tests plus `cairn validate --root .`

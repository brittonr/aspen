# Tasks: local-multiprocess-cluster-tier

## Phase 1: Plan and receipt core

- [x] [serial] r[molten.testing.local_multiprocess_cluster_tier.middle_tier] Define a cluster-oriented local multiprocess plan and executable-run receipt model over explicit state-root and transport handles.
- [x] [parallel] r[molten.testing.local_multiprocess_cluster_tier.cleanup_negatives] Add pure diagnostics for collisions, stale tickets, child timeouts, orphaned processes, missing receipts, and cleanup failure.

## Phase 2: Runner shell

- [x] [serial] r[molten.testing.local_multiprocess_cluster_tier.middle_tier] Wire a focused local multiprocess cluster harness command or test helper that spawns child `molten` processes and records canonical receipts.
- [x] [parallel] r[molten.testing.local_multiprocess_cluster_tier.cleanup_negatives] Add negative runner fixtures for timeout, orphan, stale ticket, missing workflow receipt, and failed cleanup paths.

## Phase 3: Documentation and validation

- [x] [parallel] r[molten.testing.local_multiprocess_cluster_tier.middle_tier] Document when to use local multiprocess versus CLI, simulation, and VM profiles.
- [x] [serial] r[molten.testing.local_multiprocess_cluster_tier.cleanup_negatives] Run focused local multiprocess tests, cluster CLI tests, and traceability coverage updates.

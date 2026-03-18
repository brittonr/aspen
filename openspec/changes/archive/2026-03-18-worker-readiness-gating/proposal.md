## Why

Workers receive jobs immediately after registration, before their Raft log has caught up. A freshly joined node that hasn't replicated cluster state can be dispatched work it can't correctly execute — it may read stale KV data, miss coordination state, or fail jobs that depend on recent writes. The learner promotion system already checks log lag via `is_learner_caught_up` and `compute_learner_lag`, but this check only runs during manual `promote-learner` operations. The job routing path has no equivalent gate.

## What Changes

- Workers start in a `Starting` state after registration instead of immediately being marked `Healthy`
- Workers transition to `Ready` only after Raft log catchup is confirmed (reusing the existing lag threshold from learner promotion: <100 entries behind leader)
- The job dispatcher skips workers that aren't `Ready`, same as it already skips workers under PSI pressure
- The `DistributedWorkerCoordinator` heartbeat path checks log lag on each tick and promotes `Starting` → `Ready` when caught up

## Capabilities

### New Capabilities

- `worker-readiness`: Readiness gating for distributed workers — lifecycle states, catchup verification, and dispatch filtering

### Modified Capabilities

- `jobs`: Job dispatch SHALL skip workers in `Starting` state, same as it skips pressure-exceeded workers

## Impact

- `crates/aspen-jobs/src/distributed_pool.rs` — worker registration starts as `Starting`, heartbeat checks catchup
- `crates/aspen-coordination/src/worker_coordinator/` — readiness filter added to load balancing and work stealing
- `crates/aspen-raft/src/learner_promotion.rs` — extract `is_learner_caught_up` / `compute_learner_lag` for reuse (may already be in `verified/`)
- No API changes — readiness is internal to the dispatch path
- No new dependencies

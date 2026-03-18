## Context

Workers register with the `DistributedWorkerCoordinator` and immediately start heartbeating as `HealthStatus::Healthy`. The job dispatcher uses `calculate_available_capacity_f32(load, is_healthy)` to filter — unhealthy workers get 0.0 capacity. But "healthy" currently means "heartbeat received," not "ready to do useful work."

A node that just joined the cluster hasn't replicated Raft state. The learner promotion system in `aspen-raft` already has verified pure functions for checking this:

- `compute_learner_lag(leader_last_log, learner_matched) -> u64` — saturating subtraction
- `is_learner_caught_up(lag, threshold) -> bool` — threshold is `LEARNER_LAG_THRESHOLD` (100 entries)

These live in `crates/aspen-raft/src/verified/membership.rs` with Verus specs in `crates/aspen-raft/verus/membership_spec.rs`. They're only called from `learner_promotion.rs` during manual promotion. The job dispatch path doesn't use them.

The capacity function already has the right shape: `calculate_available_capacity_f32(load, is_healthy)` returns 0.0 when `is_healthy` is false. Readiness can be folded into this same boolean — a worker that isn't ready is effectively unhealthy for scheduling purposes.

## Goals / Non-Goals

**Goals:**

- Workers don't receive jobs until their node's Raft log is caught up
- Reuse the existing `compute_learner_lag` / `is_learner_caught_up` verified functions
- Fit into the existing health/capacity filtering path with minimal new code

**Non-Goals:**

- Readiness probes for arbitrary application-level checks (just Raft catchup for now)
- Changes to the learner promotion flow itself
- New RPC endpoints or API surface
- Liveness/readiness distinction as separate concepts — a non-ready worker is simply filtered out of dispatch, same as an unhealthy one

## Decisions

**1. Readiness is a property of WorkerInfo, not a new state machine**

Add `is_ready: bool` to `WorkerInfo` in `crates/aspen-coordination/src/worker_coordinator/types.rs`. It starts `false` at registration and flips to `true` when the heartbeat path confirms Raft catchup. No new enum, no state transitions to manage.

The `available_capacity()` method already calls `calculate_available_capacity_f32(self.load, self.health == HealthStatus::Healthy)`. Change it to `self.health == HealthStatus::Healthy && self.is_ready`. A non-ready worker gets 0.0 capacity. Every downstream consumer (load balancing, work stealing, round-robin selection) automatically skips it.

Alternative considered: a `WorkerReadiness` enum with `Starting`, `CatchingUp`, `Ready`. Rejected — adds complexity for no benefit. The only question is "can this worker receive jobs?" which is a bool.

**2. Heartbeat carries Raft lag, readiness computed locally**

The distributed pool's heartbeat task already sends `WorkerStats` to the coordinator. Add `raft_log_lag: Option<u64>` to `WorkerStats`. The pool reads lag from Raft metrics (already available via `ClusterController::get_metrics()`). The coordinator calls `is_learner_caught_up(lag, LEARNER_LAG_THRESHOLD)` and sets `is_ready`.

Alternative considered: a separate readiness check RPC from coordinator to each worker. Rejected — the heartbeat already exists and runs every few seconds. Piggybacking lag on it avoids new RPCs.

**3. Readiness is one-way (with re-check on health change)**

Once `is_ready` flips to `true`, it stays `true` under normal operation. If the worker becomes `Unhealthy` (missed heartbeats), `is_ready` resets to `false` and must be re-earned on recovery. This prevents a worker that was partitioned and fell behind from immediately receiving jobs when it reconnects.

**4. Add a verified pure function for the readiness decision**

Add `is_worker_ready(lag: Option<u64>, threshold: u64, is_healthy: bool) -> bool` to `crates/aspen-coordination/src/verified/worker.rs`. It returns `true` only when the worker is healthy AND lag is known AND lag < threshold. Add a Verus spec in `crates/aspen-coordination/verus/worker_ops_spec.rs` to prove:

- Non-healthy workers are never ready
- Unknown lag (None) is never ready
- Lag at or above threshold is never ready

This follows the existing pattern — `calculate_available_capacity_f32` and `is_worker_alive` are both verified pure functions with Verus specs.

## Risks / Trade-offs

**[Cold start delay]** → Workers won't serve jobs until Raft catches up. On a fresh cluster with an empty log, this is near-instant (lag = 0). On a cluster with history, a new node may take seconds to catch up. The 100-entry threshold is the same one used for learner promotion, which is already validated as reasonable.

**[Single-node clusters]** → A single-node cluster is always leader and always caught up. Lag is 0. The `is_ready` flag flips on the first heartbeat. No behavioral change.

**[Raft metrics unavailability]** → If `get_metrics()` fails or returns no replication data, lag is `None` and the worker stays not-ready. This is conservative — better to delay jobs than dispatch to a node with unknown state. The worker will retry on the next heartbeat.

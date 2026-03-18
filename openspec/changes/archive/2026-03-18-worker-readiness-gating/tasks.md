## 1. Verified readiness function

- [x] 1.1 Add `is_worker_ready(lag: Option<u64>, threshold: u64, is_healthy: bool) -> bool` to `crates/aspen-coordination/src/verified/worker.rs`
- [x] 1.2 Add unit tests: all-conditions-met, unhealthy, lag-at-threshold, lag-above, no-lag-data
- [x] 1.3 Add Verus spec for `is_worker_ready` in `crates/aspen-coordination/verus/worker_ops_spec.rs`
- [x] 1.4 Export from `crates/aspen-coordination/src/verified/mod.rs`

## 2. WorkerInfo readiness field

- [x] 2.1 Add `is_ready: bool` field to `WorkerInfo` in `crates/aspen-coordination/src/worker_coordinator/types.rs`, default `false`
- [x] 2.2 Update `available_capacity()` to return `0.0` when `!self.is_ready` (pass `self.health == HealthStatus::Healthy && self.is_ready` to `calculate_available_capacity_f32`)

## 3. Heartbeat carries Raft lag

- [x] 3.1 Add `raft_log_lag: Option<u64>` to `WorkerStats` in `crates/aspen-coordination/src/worker_coordinator/types.rs`
- [x] 3.2 In `DistributedPool::start_background_tasks_spawn_heartbeat` (`crates/aspen-jobs/src/distributed_pool.rs`), read Raft metrics via `ClusterController::get_metrics()` and compute lag with `compute_learner_lag`, include in heartbeat stats

## 4. Coordinator readiness check

- [x] 4.1 In the coordinator's heartbeat processing path, call `is_worker_ready(stats.raft_log_lag, LEARNER_LAG_THRESHOLD, worker.health == Healthy)` and update `worker.is_ready`
- [x] 4.2 Reset `is_ready = false` when a worker transitions to `Unhealthy`

## 5. Tests

- [x] 5.1 Unit test: freshly registered worker has `is_ready = false` and `available_capacity() == 0.0`
- [x] 5.2 Unit test: worker with lag below threshold transitions to ready after heartbeat
- [x] 5.3 Unit test: worker with lag above threshold stays not ready
- [x] 5.4 Unit test: ready worker that becomes unhealthy resets to not ready
- [x] 5.5 Integration test: submit a job to a cluster, verify it's not dispatched to a worker that hasn't caught up

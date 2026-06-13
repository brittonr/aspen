## Phase 1: DAG artifact model

- [x] [serial] r[molten.job_dag.model] Define immutable distributed job DAG artifacts with nodes, edges, schemas, stage artifacts, data refs, effect manifests, policy refs, and evidence refs.
- [x] [serial] r[molten.job_dag.stage_kinds] Define initial stage kinds for source, map, filter, reduce, and materialize.
- [x] [serial] r[molten.job_dag.canonical_hashing] Hash DAG definitions and root output requests as content-addressed job ids.
- [x] [parallel] r[molten.job_dag.no_mobile_closures] Document and enforce that stages are admitted artifacts or bounded built-ins, not arbitrary live heap closures.

## Phase 2: Local execution and memoization

- [x] [serial] r[molten.job_dag.local_executor] Implement a local deterministic executor for source/map/filter/reduce/materialize over content refs or typed durable refs.
- [x] [serial] r[molten.job_dag.memo_keys] Define memo keys over stage artifact ids, input refs, dependency closure hash, schema refs, handler profile, seed/config, and policy refs.
- [x] [serial] r[molten.job_dag.memo_receipts] Emit trace records and canonical receipts for memo hits, misses, stage execution, and result materialization.
- [x] [parallel] r[molten.job_dag.eval_cache_integration] Reuse the evaluation cache for deterministic sub-DAG memoization.

## Phase 3: Planning, fusion, and profiles

- [x] [serial] r[molten.job_dag.planner] Add a planner that proposes stage order and placement from dependency, locality, cache, handler profile, capability, resource, and policy data.
- [x] [serial] r[molten.job_dag.fusion] Preview conservative adjacent-stage fusion only when schema, effect, policy, materialization, and trace constraints allow.
- [x] [parallel] r[molten.job_dag.profiling_profile] Add a deterministic profiling profile that records estimated data movement, stage costs, and hot spots.
- [x] [parallel] r[molten.job_dag.chaos_profile] Bind job/profile tests to bounded chaos handler/schedule evidence where fault profiles are exercised.

## Phase 4: Remote execution and tests

- [x] [serial] r[molten.job_dag.remote_sync] Use remote artifact sync/loopback sync to move admitted stage artifacts and dependencies to data peers before execution.
- [x] [serial] r[molten.job_dag.remote_admission] Require each target peer to admit data access, handler binding, placement, resource profile, source-gate evidence, and stage execution locally.
- [x] [parallel] r[molten.job_dag.basic_tests] Add tests for local source/map/filter/reduce/materialize DAGs, memoized reruns, planning/profile/fusion, loopback sync, target admission, and worker execution.
- [x] [parallel] r[molten.job_dag.property_tests] Add Hegel property tests for DAG hash determinism, fusion safety preconditions, memo-key stability, and no-ordinary-Raft-traffic invariants.

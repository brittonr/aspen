## Phase 1: DAG artifact model

- [ ] [serial] r[molten.job_dag.model] Define immutable distributed job DAG artifacts with nodes, edges, schemas, stage artifacts, data refs, effect manifests, policy refs, and evidence refs.
- [ ] [serial] r[molten.job_dag.stage_kinds] Define initial stage kinds for source, map, filter, reduce, and materialize.
- [ ] [serial] r[molten.job_dag.canonical_hashing] Hash DAG definitions and root output requests as content-addressed job ids.
- [ ] [parallel] r[molten.job_dag.no_mobile_closures] Document that stages are admitted artifacts, not arbitrary live heap closures.

## Phase 2: Local execution and memoization

- [ ] [serial] r[molten.job_dag.local_executor] Implement a local deterministic executor for source/map/filter/reduce/materialize over content refs or typed durable refs.
- [ ] [serial] r[molten.job_dag.memo_keys] Define memo keys over stage artifact ids, input refs, dependency closure hash, schema refs, handler profile, seed/config, and policy refs.
- [ ] [serial] r[molten.job_dag.memo_receipts] Emit trace records and Cairn receipts for memo hits, misses, stage execution, and result materialization.
- [ ] [parallel] r[molten.job_dag.eval_cache_integration] Reuse the evaluation cache for deterministic sub-DAG memoization.

## Phase 3: Planning, fusion, and profiles

- [ ] [serial] r[molten.job_dag.planner] Add a planner that proposes placement from data locality, cache availability, handler profiles, capabilities, resource limits, and policy.
- [ ] [serial] r[molten.job_dag.fusion] Fuse adjacent stages only when schema, effect, policy, materialization, and trace constraints allow.
- [ ] [parallel] r[molten.job_dag.profiling_profile] Add a profiling execution profile that records estimated data movement, stage costs, and hot spots.
- [ ] [parallel] r[molten.job_dag.chaos_profile] Add a chaos execution profile with deterministic faults, delays, reorders, and partitions.

## Phase 4: Remote execution and tests

- [ ] [serial] r[molten.job_dag.remote_sync] Use remote artifact sync to move admitted stage artifacts and dependencies to data peers before execution.
- [ ] [serial] r[molten.job_dag.remote_admission] Require each target peer to admit data access, handler binding, placement, and stage execution locally.
- [ ] [parallel] r[molten.job_dag.basic_tests] Add tests for local source/map/filter/reduce/materialize DAGs and memoized reruns.
- [ ] [parallel] r[molten.job_dag.property_tests] Add Hegel property tests for DAG hash determinism, fusion safety preconditions, memo-key stability, and no-ordinary-raft-traffic invariant.

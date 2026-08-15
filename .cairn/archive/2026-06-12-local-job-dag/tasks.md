## Phase 1: Canonical local job model

- [x] [serial] r[molten.local_job_dag.spec.canonical_identity] Define canonical `job-dag-v1`, `job-node-v1`, `job-edge-v1`, and `job-output-request-v1` records with schemas, stage artifact refs, data refs, effect manifests, policy refs, evidence refs, and checks.
- [x] [serial] r[molten.local_job_dag.spec.no_mobile_closures] Define first local stage kinds: `source`, `map`, `filter`, `reduce`, and `materialize`, with bounded Preserves-oriented stage-operation artifacts.
- [x] [serial] r[molten.local_job_dag.spec.canonical_identity] Compute job refs and output-request refs from canonical Preserves records, never from mutable names, paths, mtimes, or short ids.
- [x] [parallel] r[molten.local_job_dag.spec.no_mobile_closures] Reject raw closures, host paths, process commands, and ambient environment-dependent stage configs before execution.

## Phase 2: Deterministic local executor

- [x] [serial] r[molten.local_job_dag.spec.local_execution] Validate DAG shape through Trellis topo-sort/order checks and execute stages in deterministic topological order with canonical tie-breaking by node id.
- [x] [serial] r[molten.local_job_dag.spec.local_execution] Implement source stages over inline canonical values, typed-storage refs, and chunk/content refs with verification/effect receipts.
- [x] [serial] r[molten.local_job_dag.spec.local_execution] Implement bounded deterministic map and filter stage operations over Preserves value streams.
- [x] [serial] r[molten.local_job_dag.spec.local_execution] Implement deterministic reduce stages with explicit reducer artifacts and recorded reduction order.
- [x] [parallel] r[molten.local_job_dag.spec.local_execution] Materialize outputs as inline refs, typed-storage refs, or chunk manifests according to explicit output policy.
- [x] [parallel] r[molten.local_job_dag.spec.local_execution] Ensure local execution uses explicit effect/evidence boundaries for storage, chunk, and materialization observations.

## Phase 3: Memoization, receipts, and indexes

- [x] [serial] r[molten.local_job_dag.spec.memo_keys] Define eval-cache memo keys over job ref, output request ref, stage id/artifact ref, input refs, dependency closure, schema refs, handler profile, policy/capability refs, effect-handle refs, and tool version refs.
- [x] [serial] r[molten.local_job_dag.spec.memo_keys] Reuse `eval_cache` for stage/sub-DAG memo hits, misses, stale-denials, and trace-only observations.
- [x] [serial] r[molten.local_job_dag.spec.receipts] Define `job-dag-receipt-v1` records for install, run, stage, memo-hit, memo-miss, materialize, and deny operations.
- [x] [parallel] r[molten.local_job_dag.spec.receipts] Classify job DAGs, output requests, stage receipts, run receipts, and materialization receipts in the local ledger and catalog views.
- [x] [parallel] r[molten.local_job_dag.spec.memo_keys] Revalidate policy-current cache hits and deny stale policy/capability/revocation inputs before returning semantic outputs.

## Phase 4: CLI and catalog visibility

- [x] [serial] r[molten.local_job_dag.spec.cli_identity] Add `molten test job install` and `show` commands that print full refs and canonical install/show receipts.
- [x] [serial] r[molten.local_job_dag.spec.cli_identity] Add `molten test job run` and `status` commands for deterministic local runs, output refs, stage receipts, and run receipts.
- [x] [serial] r[molten.local_job_dag.spec.receipts] Add `molten test job receipt-show` for canonical job receipts stored in the local ledger.
- [x] [parallel] r[molten.local_job_dag.spec.receipts] Expose job DAG graph, stage refs, output refs, memo receipts, and diagnostics through catalog views with existing visibility/redaction hooks.

## Phase 5: Tests and properties

- [x] [serial] r[molten.local_job_dag.spec.canonical_identity] Add tests proving canonical DAG/output-request refs are stable and independent of names, paths, mtimes, and short ids.
- [x] [serial] r[molten.local_job_dag.spec.local_execution] Add tests for source → map → filter → materialize pipelines over Preserves values and typed-storage/chunk refs.
- [x] [serial] r[molten.local_job_dag.spec.local_execution] Add tests for deterministic reduce order and output materialization refs.
- [x] [serial] r[molten.local_job_dag.spec.memo_keys] Add tests for memoized reruns, changed-input misses, policy-current stale-denials, and memo receipt binding.
- [x] [parallel] r[molten.local_job_dag.spec.no_mobile_closures] Add tests rejecting raw closures, host paths, process commands, unknown stage kinds, malformed schemas, and hidden/unauthorized refs.
- [x] [parallel] r[molten.local_job_dag.spec.canonical_identity] Add Hegel properties for DAG hash determinism, topological order determinism, memo-key stability, materialization ref stability, and no-mobile-closure safety.

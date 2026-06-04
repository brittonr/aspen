## Phase 1: Planning artifacts

- [x] [serial] r[molten.local_job_dag_planning.plan_dto] Define canonical `job-plan-v1` and per-stage planning records binding job ref, output request ref, Trellis order, dependencies, placement proposals, cache projection, policy refs, resource refs, and checks.
- [x] [serial] r[molten.local_job_dag_planning.trellis_order_binding] Build plans from the same Trellis node-index/topo/dependency-readiness adapter used by local execution.
- [x] [parallel] r[molten.local_job_dag_planning.advisory_placement] Keep placement proposals advisory and local-only; do not grant remote authority or execute stages during planning.

## Phase 2: Deterministic profiles

- [x] [serial] r[molten.local_job_dag_planning.profile_dto] Define canonical `job-profile-v1` records with stage count, edge count, materialization boundaries, deterministic byte estimates, per-stage profiles, cache projections, and checks.
- [x] [serial] r[molten.local_job_dag_planning.no_wall_clock_profile] Ensure profiles do not use wall-clock time, system load, mtimes, network state, or runtime side effects.
- [x] [parallel] r[molten.local_job_dag_planning.cache_projection] Include eval-cache availability projections as advisory evidence only, not semantic cache hits.

## Phase 3: Fusion preview

- [x] [serial] r[molten.local_job_dag_planning.fusion_dto] Define canonical `job-fusion-plan-v1` and chain records for preview-only fusion opportunities.
- [x] [serial] r[molten.local_job_dag_planning.map_filter_only] Admit fusion only for adjacent pure `map`/`filter` stages in Trellis order.
- [x] [serial] r[molten.local_job_dag_planning.boundary_rejection] Reject fusion across reduce, materialize, schema, effect, policy, or materialization boundaries.

## Phase 4: Receipts, CLI, tests

- [x] [serial] r[molten.local_job_dag_planning.receipts] Emit `job-plan-receipt-v1`, `job-profile-receipt-v1`, and `job-fusion-receipt-v1` binding artifact refs and checks.
- [x] [serial] r[molten.local_job_dag_planning.cli] Add `molten test job plan`, `profile`, and `fusion-preview` commands with artifact and receipt outputs.
- [x] [parallel] r[molten.local_job_dag_planning.ledger_catalog_classification] Classify planning artifacts and receipts in the local ledger/catalog surface.
- [x] [parallel] r[molten.local_job_dag_planning.tests] Add unit/CLI coverage for Trellis-bound plans, deterministic profiles, fusion preview, and boundary rejection.

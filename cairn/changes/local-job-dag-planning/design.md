## Context

`local-job-dag` introduced canonical job DAGs and a deterministic local runner. Its topology and readiness checks are now backed by Trellis primitives:

- canonical node ids are mapped to numeric Trellis indices,
- `trellis::topo_sort::topo_sort` proposes the execution order,
- `trellis::topo_sort::is_topo_order` validates it,
- `trellis::job_dag::all_deps_satisfied` and `unsatisfied_count` gate per-stage readiness.

The next layer should expose planning and profiling without executing the job. Plans are advisory artifacts. They explain local placement and optimization opportunities, but do not bypass policy or perform side effects.

## Goals

- Define canonical planning artifacts for job plans, profiles, and fusion previews.
- Reuse the same Trellis topology/readiness adapter as local execution.
- Keep placement local/advisory in this slice.
- Estimate deterministic costs without wall-clock timing or live runtime observations.
- Preview safe map/filter fusion opportunities while preserving evidence boundaries.
- Emit receipts for plan/profile/fusion artifacts.
- Provide CLI commands for local workflows and transcript gates.

## Non-Goals

- No remote execution or peer placement yet.
- No production cost model based on timing, load, or network measurement.
- No actual fused execution in this slice; fusion is preview-only.
- No fusion across effect, policy, schema, reduce, materialize, or explicit materialization boundaries.
- No cache hits as authority; cache projections are advisory unless a later run validates actual eval-cache keys.

## Plan artifact

```preserves
<job-plan-v1 "molten.job-dag.plan.v1"
  <job <job-ref>>
  <request <output-request-ref>>
  <stage-order ["stage-id" ...]>
  <stages [<job-stage-plan-v1 ...> ...]>
  <policy [<policy-ref> ...]>
  <checks [<check "trellis-topo-order" "pass"> ...]>>
```

Each stage plan binds:

- stage id,
- Trellis numeric index,
- dependency stage ids,
- placement proposal (`local` in this slice),
- cache projection (`eligible` or `not-cacheable`),
- policy refs,
- resource refs reserved for the resource-governance layer,
- checks naming Trellis dependency binding and advisory placement.

## Profile artifact

```preserves
<job-profile-v1 "molten.job-dag.profile.v1"
  <job <job-ref>>
  <request <output-request-ref>>
  <stage-count n>
  <edge-count n>
  <materialization-boundaries n>
  <estimated-bytes <config n> <known-cache-entries n>>
  <stages [<job-stage-profile-v1 ...> ...]>
  <checks [<check "deterministic-profile" "pass"> ...]>>
```

The first profile is deterministic and static: it counts canonical config bytes, stages, edges, materialization boundaries, and known cache-index entries when a cache root is supplied. It does not use elapsed time, system load, file mtimes, or network state.

## Fusion preview artifact

```preserves
<job-fusion-plan-v1 "molten.job-dag.fusion-plan.v1"
  <job <job-ref>>
  <request <output-request-ref>>
  <chains [<job-fusion-chain-v1 ...> ...]>
  <checks [<check "fusion-is-preview-only" "pass"> ...]>>
```

A chain is admitted only for adjacent Trellis-ordered `map`/`filter` stages connected by a `stream` edge, with no schema ref, no effect manifests, and no policy refs on either endpoint. Reduce/materialize stages are never fused in this slice.

## Receipts

Planning emits separate receipts:

- `job-plan-receipt-v1`,
- `job-profile-receipt-v1`,
- `job-fusion-receipt-v1`.

Each receipt binds job ref, request ref, artifact ref, diagnostics, and checks. Receipts are local evidence, not proof of remote admissibility.

## CLI

Add:

- `molten test job plan <job-ref-or-file> --registry <path> [--output-request file] [--out file] [--receipt-out file]`
- `molten test job profile <job-ref-or-file> --registry <path> [--cache path] [--output-request file] [--out file] [--receipt-out file]`
- `molten test job fusion-preview <job-ref-or-file> --registry <path> [--output-request file] [--out file] [--receipt-out file]`

## Tests

Required coverage:

- plan stage order follows Trellis topo order,
- profiles are deterministic and wall-clock-free,
- fusion preview admits pure adjacent map/filter stages,
- fusion preview rejects reduce/materialize/schema/effect/policy boundaries,
- CLI emits canonical artifacts and receipts.

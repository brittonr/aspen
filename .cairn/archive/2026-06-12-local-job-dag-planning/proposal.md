## Why

Molten now has a local deterministic job DAG runner backed by Trellis topology/readiness primitives. Before adding remote execution, Molten needs a planning layer that can explain how a job would run: the Trellis-derived stage order, local placement proposals, cache projections, deterministic profile estimates, and safe fusion opportunities.

Planning must be evidence-bearing and advisory. It should not execute stage logic, observe wall-clock time, move data, or mint authority. It should produce canonical artifacts and receipts that future remote scheduling can refine without changing the local DAG semantics.

## What Changes

- Add canonical local job planning artifacts: `job-plan-v1`, `job-profile-v1`, and `job-fusion-plan-v1`.
- Bind planning to the same Trellis-backed node-id-to-index mapping, topo order, and dependency readiness used by local execution.
- Add local placement proposals for each stage, with policy/resource/capability refs carried as evidence inputs rather than ambient authority.
- Add deterministic profiling estimates for stage count, edge count, materialization boundaries, config bytes, and cache availability projections.
- Add fusion preview for adjacent pure `map`/`filter` stages only, refusing fusion across schema, effect, policy, reduce, materialize, or materialization boundaries.
- Emit plan/profile/fusion receipts with canonical refs and checks.
- Add CLI commands: `molten test job plan`, `molten test job profile`, and `molten test job fusion-preview`.

## Impact

This gives Molten a readable, receipt-backed local planning surface over job DAGs. It is the bridge from local execution to future distributed scheduling while preserving the rule that remote execution must be separately admitted by the receiving peer.

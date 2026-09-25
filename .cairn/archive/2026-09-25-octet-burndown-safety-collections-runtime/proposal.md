# Proposal: Octet burn-down, collection growth in runtime and adapters

## Why

`unbounded_collection_growth` is a critical Octet family and drives the strict gate's `no-critical-findings` failure.
After `octet-burndown-safety-collections-validators`, the pinned Octet run still reports 105 findings in 56 distinct
sites. All of them are in the runtime, adapter, and fixture modules. This change clears them.

## What Changes

- Structural bounds, as in the validators slice. Filter/map loops become iterator chains that keep order,
  last-wins map semantics, and first-error propagation. Loops over already-bounded inputs reserve exactly the per-loop
  maximum, with named multipliers.
- Named limits where growth is not bounded by an already-bounded input. Each limit comes from an existing profile or
  config limit where one exists, and each has at-limit and one-past tests:
  - `parse_cluster_manifest`: `MAX_CLUSTER_MANIFEST_NODES` = the existing `MAX_CLUSTER_LIFECYCLE_ITEMS`.
  - `parse_run_index`: `MAX_RUN_INDEX_ENTRIES` = molten-core `MAX_RUN_ARTIFACTS` + 1, now `pub`. An oversized index still
    reaches the core too-many-artifacts diagnostic.
  - Reference-world scheduler choices: the admitted manifest `max-choices` bound.
  - Materialization source listing: the admitted policy `max_members`.
  - World-distribution closure commits: the admitted `max_closure_objects`.
  - Branch conflict-record reads: `MAX_WORLD_HEAD_CONFLICT_RECORDS` (256). No existing limit covers stored conflict sets,
    so a read denies rather than materializing an unbounded list.
  - The prolly snapshot walk keeps its existing `max_graph_facts` bound, now expressed on the block count.

## Impact

- **Files**: 36 files in `src/{addressable_actor,cluster*,content_store_adapter,fabric_*,materialization,nixos,prolly_map,system_extension,wasm,world_*}`,
  `crates/molten-core/src/cluster_harness.rs`, and `tests/fabric_simulation_boundary.rs`.
- **Testing**: the pinned Octet root and lib runs; `cargo fmt --check`; `cargo clippy --workspace --all-targets -D
  warnings`; focused and full workspace tests; harness fixture receipt comparison; the flake checks for touched surfaces.

## Out of Scope

- Accepted specifications do not change. Inputs within the existing bounds produce identical outputs and receipts.
  Inputs past a bound are denied, where some of them used to produce a larger collection or a later diagnostic.

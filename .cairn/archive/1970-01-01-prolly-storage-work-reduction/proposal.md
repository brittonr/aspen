# Proposal: Reduce real storage work through structural sharing

## Why

The 2026-09-09 static review (base `87c289bf68c4`) found that Molten's Prolly pilot
achieves structural sharing in storage but not in computation:

- `plan_edits` rebuilds the entire map through a `BTreeMap` clone plus `build_map`
  for every edit batch; unchanged blocks are reused in storage only.
- `diff_maps` validates both complete snapshots, scans flattened entry sequences, and
  computes `skipped_equal_nodes` afterward from closure intersection. The metric
  measures sharing; it does not demonstrate avoided reads or decodes.
- `load_prolly_snapshot` opens a fresh Redb read transaction for every block, and the
  adapter copies bytes into a `Vec` per node.

These are identifiable sources of work, not measured bottlenecks; the standard
profile is capped at 4,096 entries. This change therefore first adds measurements,
then converts sharing into avoided work under an exact canonical-root equality
condition, with the rebuild-first implementation retained as a reference oracle.

## What Changes

- Add operation-level measurements for the Prolly paths: blocks read, blocks
  decoded, subtrees not traversed, transactions opened, and bytes copied.
  r[molten.prolly.metrics]
- Add an application-owned, bounded read-session operation to `ProllyBlockStorePort`
  so the Redb adapter can hold one read transaction per operation while the pure core
  sees only bounded block requests and observations. r[molten.prolly.readsession]
- Implement a demand-driven diff that compares child identities and key ranges and
  skips matching validated subtrees, requesting blocks only for differing regions,
  with exact output equality against the current implementation. The
  `skipped_equal_nodes` metric is split into `shared_node_count` versus actual
  avoided traversal. r[molten.prolly.diff]
- Implement an incremental edit planner bounded by work limits with a rebuild
  fallback, required to produce exactly the same canonical root as the rebuild-first
  reference including under content-defined boundary shifts.
  r[molten.prolly.incremental]
- Extend the existing dataspace-access cache principles to a decoded immutable-node
  cache keyed by canonical node reference, profile, and codec/validation version,
  with advisory-only retention that cannot change canonical outputs.
  r[molten.prolly.cache]

## Impact

- `crates/molten-core/src/prolly_map/` (operations, tree read/build, profile
  limits) and `src/prolly_map/` (port, adapter, service).
- `ProllyBlockStorePort` gains a read-session capability; both adapters update.
- New measurement and oracle-comparison test scaffolding.

## Out of Scope

- Replacing Redb, importing LLAMA-style page caches, custom epoch reclamation, or a
  new lock-free tree.
- Raising the 4,096-entry standard profile cap; benefit must be established within
  the existing profile first.
- Any change that weakens snapshot validation: a matching root string never confers
  validity on an untrusted snapshot.

## Affected Specs

- `prolly-storage-work-reduction`: measurements, bounded read sessions, genuine
  subtree-skipping diff, incremental edits with exact canonical equality, and the
  decoded-node cache.

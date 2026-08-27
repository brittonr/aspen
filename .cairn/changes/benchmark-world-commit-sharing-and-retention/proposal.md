## Why

World commits can branch by referencing immutable roots, and existing content and VM stores can share unchanged objects. The roadmap has no bounded evidence for branch cost, changed-byte amplification, diff cost, replication reuse, or retention and garbage-collection planning cost.

Without measurements, the stack cannot decide whether current Molten stores are sufficient or a product-neutral branchable-state component is justified.

## What Changes

- Add typed Nickel benchmark profiles for logical roots, content chunks, exact VM snapshots, replication, retention, and garbage-collection planning.
- Measure root-only branch creation, first mutation, repeated mutation, diff, merge planning, capsule export, replicated reuse, pin changes, and GC planning.
- Record logical bytes, physical bytes, new and reused objects, mapped or copied pages, operation counts, bounded duration observations, and peak memory observations.
- Keep logical and opaque snapshot cohorts separate and reject cross-profile performance equivalence claims.
- Add deterministic synthetic datasets plus one reviewed downstream-shaped fixture.
- Emit canonical benchmark receipts bound to source revision, profile, dataset, adapter, hardware cohort, limits, and results.
- Add a pure extraction decision that returns retain-current, optimize-in-place, or evaluate-shared-component from supplied accepted receipts and policy.

## Dependencies

- World commit core, semantic diff and merge, snapshot profiles, and replication and retention changes.
- Existing Molten content manifests, Redb adapters, simulation, retention export, and ChaosControl copy-on-write observations.

## Non-Goals

- An asymptotic-complexity proof from finite benchmark runs.
- A universal performance, latency, memory, kernel, filesystem, or hardware claim.
- Automatic creation of a branchable-state repository or automatic store replacement.
- Garbage-collection deletion authority or proof that unavailable objects are deleted.

## Impact

- **Core**: benchmark profiles, metric records, comparison rules, acceptance policy, and extraction decisions.
- **Shell**: dataset materialization, instrumented operations, resource observations, and receipt writing.
- **Configuration**: typed Nickel datasets, cohorts, limits, and named acceptance thresholds.
- **Testing**: positive stable and sharing cases plus negative profile mixing, stale revision, missing metric, unexplained threshold, warm-cache confusion, hidden prepopulation, unsafe GC claim, and benchmark-overclaim cases.

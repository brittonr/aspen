## Why

The world-commit roadmap separates identity, heads, merge, snapshots, authority, effects, replication, and replay into bounded changes. Separate passing APIs do not prove that an operator can complete one coherent workflow.

Molten needs a dogfood composition root that exercises the real contracts without creating a second runtime or a stack-global Realm service.

## What Changes

- Add typed `molten world` operator requests for checkpoint, branch, run, diff, conflicts, replay, simulate, verify, promote, export, import, and garbage-collection planning.
- Add a pure workflow planner that binds exact commit, branch, profile, policy, limit, and expected-generation inputs.
- Make mutating commands preview-first and require exact plan identity plus fresh admission before apply.
- Compose existing world-commit cores and application-owned ports at one visible CLI composition root.
- Add one logical end-to-end fixture and one exact opaque-profile restore and replay fixture.
- Emit one canonical workflow receipt that links each operation receipt without replacing its owner or claim boundary.
- Add deterministic operator summaries and first-blocker diagnostics.
- Keep optional witness and executable-extent profiles visible as supported, blocked, or unavailable.

## Dependencies

- All current Molten world-commit roadmap changes through replay capsules.
- Existing Molten simulation, content-store, effect, authority, reconciliation, and operator-receipt mechanisms.
- ChaosControl snapshot descriptors for the opaque fixture.

## Non-Goals

- A new top-level `realm` binary, daemon, hosted control plane, or workflow engine.
- Implicit latest-head selection, ambient credential discovery, or raw command-string execution.
- Automatic conflict resolution, policy override, effect completion, or garbage-collection authority.
- A whole-stack correctness or production-readiness claim from one dogfood run.

## Impact

- **Core**: workflow requests, plans, blockers, operation graph, receipt linkage, and summaries.
- **Shell**: CLI parsing, explicit adapters, preview/apply orchestration, and bounded output persistence.
- **Schemas**: workflow request, plan, receipt, and operator-summary records.
- **Testing**: complete logical and opaque workflows plus negative stale plan, missing profile, wrong generation, denied authority, conflict, uncertain effect, incomplete capsule, unavailable witness, and GC-overclaim cases.

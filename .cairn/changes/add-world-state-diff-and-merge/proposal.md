## Why

World commits and branch heads permit divergent histories. They do not define how Molten compares or combines semantic state.

Choregraph can validate a merge envelope, but applications retain merge meaning. Schema Identity Core and Schema Migration Core describe type lineage and migration paths. Neither merges Molten runtime state.

A generic byte merge would be unsafe for tasks, effects, authority, scheduler state, or opaque memory. Molten needs typed, bounded, default-deny diff and merge contracts.

## What Changes

- Add deterministic world-root diff reports with explicit comparable, changed, unavailable, and incompatible classes.
- Add a base-left-right merge plan over one exact common ancestor and two or more declared heads.
- Define closed root merge modes, including identical-only, ancestor replacement, keyed durable-value merge, and exact application-handler identity.
- Require schema compatibility or an admitted migration plan before value comparison.
- Keep task, scheduler, authority, effect-attempt, external-observation, and opaque-machine roots non-mergeable by default.
- Run application handlers as pure bounded functions over already-loaded canonical values.
- Produce conflict artifacts without partial state mutation or head movement.
- Publish a merge result only as a new world commit with all declared parents.

## Dependencies

- `introduce-world-commit-core` and `add-world-branch-head-protocol`.
- Choregraph merge envelopes.
- Schema Identity Core and Schema Migration Core.
- Existing Molten typed durable values and Preserves canonicalization.

## Non-Goals

- Generic heap, process, device, task, authority, or side-effect merge.
- Automatic conflict winner selection.
- Applying migrations, executing effects, or promoting the result during pure merge planning.

## Impact

- **Core**: diff classes, merge profiles, common-ancestor inputs, pure handlers, conflict values, and result plans.
- **Shell**: bounded object loading, handler selection, migration admission, result-root publication, and detached reports.
- **Schemas**: diff, conflict, merge-plan, and merge-result Preserves records.
- **Testing**: clean merges plus negative incompatible schema, ambiguous ancestor, stale handler, unmergeable root, conflict, bound exhaustion, and partial-publication cases.

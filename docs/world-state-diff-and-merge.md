# World state diff and merge

Molten compares every typed world root. It merges only roots admitted by one exact profile.

## Dependencies

The merge core uses these reviewed inputs:

- Choregraph branch history at `b3e08e19750f53bdbcae970cdf58a47a791ed20b`.
- Schema Migration Core at `4fe90e130f2871cf69a6febcdc70785adca98aea`.
- Schema Identity Core at `2562c8aa38a034061f9af9f3e17280494a5b8de2`.

Choregraph owns structural history. It does not own Molten merge meaning.

Schema Migration Core selects a declared migration path. It does not execute migrations or prove migrated data correctness.

## Conservative diff

Diff classifies each supplied root as equal, changed, absent, unavailable, incompatible, or excluded by profile.

A missing object is unavailable. A missing root is absent. Neither case can appear as equal.

Diff output grants no merge authority.

## Exact base and sources

Merge requires one verified common ancestor. Ambiguous or missing bases fail before handler execution.

The plan records every declared source head. A successful merge commit keeps all source heads as parents.

## Closed merge modes

The initial profile supports four modes:

- `identical-only` accepts the same root on both sides.
- `ancestor-replacement` accepts one changed side when the other equals the base.
- `keyed-durable-values` performs a bounded three-way key merge.
- `application-handler` calls one exact pure handler over loaded canonical bytes.

Unknown modes fail closed.

Tasks, scheduler state, effects, time, entropy, authority observations, and opaque snapshots are runtime-sensitive. Divergent values in these classes do not merge.

## Schema and migration admission

Equal original schema identities are the normal path. A mismatch requires an admitted migration binding; equality obtained by rewriting schema metadata is never admission evidence.

The binding names exact source and target schemas, a migration plan, and a checked migration profile identifier. Molten world-merge maintainers own admission policy and shell ordering; the migration adapter owns execution mechanics, not permission to execute.

The preparation flow preserves the original request until pure admission and result matching complete:

1. The core checks the whole original request and every requested conversion before any migration adapter is called. It checks the original kind, side, root, source schema/content, availability, binding, request context, and input/output bounds. A denial on any conversion prevents all migration calls.
2. The shell passes an immutable admitted conversion token and loaded source bytes to the migration adapter. The adapter returns an explicit result carrying the conversion ref, target schema, result content ref, output bytes, and keyed values.
3. The core validates the result against that token, including identity and bounds, before accepting it as materialized migration evidence. Failed, empty, oversized, substituted, or stale results prevent merge handling and publication. The result ref identifies the structured output, including keyed values; it is not merely a hash of the byte field.
4. The planner consumes the original request and validated results. It matches them against the conversions required by the originals, then normalizes internal values for reduction. It replaces migrated keyed values as well as bytes and schema. Missing evidence cannot be repaired by handing the planner already-normalized target-schema metadata.

These checks establish exact binding and bounded materialization, not arbitrary semantic correctness or application-schema validation of malformed output bytes. Migration planning itself never executes code.

### Canonical plan cutover

The current canonical carrier is `molten.world-merge-plan.v2`. Its eight fields are the schema identifier, plan ref, base head, source heads, migrations, outputs, conflicts, and non-claims. Each `merge-migration` entry records:

- `conversion-ref`, `kind`, and `side` (`base`, `left`, or `right`);
- `original-root`, `source-schema`, and `source-ref` for the admitted original content;
- `target-schema`, `migration-plan`, and `migration-profile`;
- `max-output-bytes` and the validated `result-ref`.

The durable capability is an inspectable, content-addressed link from each converted original input to its admitted bounded result, retained even after internal schema normalization. The immediate outcome is admission-before-execution and denial-before-publication, rather than a target-schema equality shortcut.

`schemas/preserves-boundaries/molten-world-merge-plan-v2.preserves` is the current carrier marker. The v1 marker remains historical evidence only: v1 plans lack these conversion bindings, and there is no executable v1 plan reader or automatic upgrade that grants admission. Replan from the original commits, roots, loaded content, and reviewed profile through the current preparation API; do not relabel v1 bytes or treat an old successful plan as current migration permission.

## Pure handlers

An application handler binds behavior, input schema, output schema, policy, and output bounds.

The handler receives loaded bytes only. An effect request denies the handler result.

Handler identity does not prove handler correctness.

## Conflicts

Concurrent incompatible key or root changes create deterministic conflict artifacts.

Molten does not choose by timestamp, arrival order, or content identity. An unresolved conflict prevents merge-commit publication.

## Publish-last shell

The shell persists every generated root before it publishes a merge commit.

When reduction selects converted content, it emits a generated target-schema root with the new bytes instead of reusing the old source root identity. The shell canonically constructs and persists that generated root before publishing the commit. An unchanged selected root may still be reused. Migration result refs bind materialization evidence; they are not a substitute for the generated world-root identity.

If one root publication fails, no successful merge commit is published. Already written immutable roots remain harmless unreferenced objects.

The shell rechecks current merge authority before publication. The standalone `world-merge merge-publish` command stays fail-closed without that adapter.

## Operator commands

`molten world-merge` provides these commands:

- `diff` compares three explicit commits.
- `merge-plan` emits one bounded conservative plan.
- `conflict-inspect` validates and prints one conflict artifact.
- `merge-publish` validates input and fails closed without composed authority, migration, and handler adapters.

## Repeatability and ownership

World-merge maintainers can repeat the focused core and shell checks from the repository development shell:

```sh
cargo test -p molten-core world_merge
cargo test -p molten --lib world_merge
```

The public preparation/publication path is the integration proof boundary: record adapter calls and publications, deny an unadmitted binding before any call, accept an admitted conversion with retained canonical evidence, reject stale or substituted results, and publish converted bytes under a new target-schema root. Repeat with mixed source/target inputs and keyed outputs so schema normalization cannot hide a retained source map or old root identity. A `merge-plan` CLI run without a migration adapter is not proof that conversion execution worked, and a serialized v2 plan is evidence rather than a transferable execution capability. These commands describe the repeatability path, not a claim that all validation rails have passed.

## Non-claims

Merge evidence does not prove semantic correctness, migration correctness, handler correctness, branch movement, effect release, remote convergence, or release eligibility.

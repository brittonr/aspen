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

The current canonical carrier is `molten.world-merge-plan.v3`. Its eight fields are the schema identifier, plan ref, base head, source heads, migrations, outputs, conflicts, and non-claims. Each `merge-output` entry is a root kind plus exactly one explicit case:

- `absent` records no root value for the new inventory;
- `selected` binds one existing cross-root reference;
- `generated` binds the target schema ref. Generated values and handler bytes enter the plan identity through a payload reference.

The three cases are a closed typed enum in the core, so a carrier cannot declare absence and also supply a selected root or generated payload. Plan identity is derived from the case tag rather than from field emptiness, and a generated empty value has a different identity from an absent root. A plan whose stored identity does not match its current outputs denies with `PlanIdentityMismatch` before any publication port runs.

Each `merge-migration` entry records:

- `conversion-ref`, `kind`, and `side` (`base`, `left`, or `right`);
- `original-root`, `source-schema`, and `source-ref` for the admitted original content;
- `target-schema`, `migration-plan`, and `migration-profile`;
- `max-output-bytes` and the validated `result-ref`.

The durable capability is an inspectable, content-addressed link from each converted original input to its admitted bounded result, retained even after internal schema normalization. The immediate outcome is admission-before-execution and denial-before-publication, rather than a target-schema equality shortcut.

`schemas/preserves-boundaries/molten-world-merge-plan-v3.preserves` is the current carrier marker. The v1 and v2 markers remain historical evidence only. Neither has an executable reader or automatic upgrade: v1 plans lack conversion bindings, and v2 outputs carry the overloaded optional selection this contract replaces. Replan from the original commits, roots, loaded content, and reviewed profile through the current preparation API; do not relabel old bytes or infer a case from which fields happen to be empty.

## Pure handlers

An application handler binds behavior, input schema, output schema, policy, and output bounds.

The handler receives loaded bytes only. An effect request denies the handler result.

Handler identity does not prove handler correctness.

## Conflicts

Concurrent incompatible key or root changes create deterministic conflict artifacts.

Molten does not choose by timestamp, arrival order, or content identity. An unresolved conflict prevents merge-commit publication.

## Publish-last shell

The shell persists every generated root before it publishes a merge commit.

An admitted absent root is omitted from the new root inventory. Absence never calls the generated-root port, never requires a schema, and never authorizes deletion of retained content; old snapshots and retention holds remain separate protections.

When reduction selects converted content, it emits a generated target-schema root with the new bytes instead of reusing the old source root identity. The shell canonically constructs and persists that generated root before publishing the commit. An unchanged selected root may still be reused. Migration result refs bind materialization evidence; they are not a substitute for the generated world-root identity.

If one root publication fails, no successful merge commit is published. Already written immutable roots remain harmless unreferenced objects.

The shell rechecks current merge authority before publication. The standalone `world-merge merge-publish` command stays fail-closed without that adapter.

The plan seal binds generated handler-byte presence as well as byte and keyed-value
content. An absent handler payload and a present empty payload produce distinct
canonical roots and cannot be substituted after planning. Plans prepared before
this presence binding must be prepared again; publication does not upgrade them.

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

The [F06 execution record](../.cairn/archive/2026-10-20-admit-world-merge-migrations-before-execution/evidence/2026-10-20-verification.md)
retains the admitted-conversion, denied-input, substituted-result, and post-plan
mutation observations, canonical target-payload readback, and the exact executed
validation checkpoints. Earlier receipts remain bound to their recorded sources.
The separate absent-output carrier correction remains outside F06 completion.

## Non-claims

Merge evidence does not prove semantic correctness, migration correctness, handler correctness, branch movement, effect release, remote convergence, or release eligibility.

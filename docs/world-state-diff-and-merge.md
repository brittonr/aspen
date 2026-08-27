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

Equal schema identities are the normal path. A mismatch requires an admitted migration binding.

The binding names exact source and target schemas, a migration plan, and a checked migration profile identifier.

The shell must materialize migration output before a pure merge handler runs. Planning does not execute migration code.

## Pure handlers

An application handler binds behavior, input schema, output schema, policy, and output bounds.

The handler receives loaded bytes only. An effect request denies the handler result.

Handler identity does not prove handler correctness.

## Conflicts

Concurrent incompatible key or root changes create deterministic conflict artifacts.

Molten does not choose by timestamp, arrival order, or content identity. An unresolved conflict prevents merge-commit publication.

## Publish-last shell

The shell persists every generated root before it publishes a merge commit.

If one root publication fails, no successful merge commit is published. Already written immutable roots remain harmless unreferenced objects.

The shell rechecks current merge authority before publication. The standalone `world-merge merge-publish` command stays fail-closed without that adapter.

## Operator commands

`molten world-merge` provides these commands:

- `diff` compares three explicit commits.
- `merge-plan` emits one bounded conservative plan.
- `conflict-inspect` validates and prints one conflict artifact.
- `merge-publish` validates input and fails closed without composed authority, migration, and handler adapters.

## Non-claims

Merge evidence does not prove semantic correctness, migration correctness, handler correctness, branch movement, effect release, remote convergence, or release eligibility.

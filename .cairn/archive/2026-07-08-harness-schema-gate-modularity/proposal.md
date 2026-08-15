## Why

Harness, schema, and gate code carries release/test orchestration, evidence fixtures, and validation logic that should not be coupled to the runtime core. Shard-heavy harness modules make it hard to distinguish reusable deterministic validators from dogfood or integration shells.

## What Changes

- Separate harness schema models, pure gate decisions, fixture builders, and integration shells.
- Move reusable test/evidence helpers toward a harness/testkit boundary.
- Keep runtime modules consuming canonical gate outcomes rather than importing harness orchestration.
- Add positive and negative tests for extracted gate decisions.

## Impact

Runtime code becomes smaller and less dependent on test orchestration. Harness code becomes a clearer release-evidence tool rather than an implicit runtime dependency.

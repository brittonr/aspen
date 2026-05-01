# Delta consistency fixtures

Fixture matrix for `scripts/check-openspec-delta-consistency.py`:

- `valid-add`: added requirement and scenario IDs pass.
- `missing-requirement-id`: added requirement without an ID fails.
- `missing-scenario-id`: added scenario without an ID fails.
- `valid-modify`: modified requirement targeting an existing main-spec ID passes.
- `missing-modified-target`: modified requirement without a main-spec target fails.
- `valid-removal`: removed requirement targeting an existing main-spec ID passes.
- `missing-removal-target`: removed requirement without a main-spec target fails.
- `migration-note`: missing modified target with `Migration note:` emits a warning and exits successfully.
- `feature-conflict`: conflicting default-feature/no-default-features language emits a warning; `--warnings-as-errors` fails.

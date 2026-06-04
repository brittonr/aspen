# Change: sealed-repro-bundles

## Why

Repro bundles wrapped reports and could be gated, but they did not carry their own pass receipt. After policy, capability, budget, replay, and gate receipts became deterministic evidence rails, exported repro bundles should become portable sealed pass artifacts rather than loose report directories.

## What

- Export report repro bundles with a seal record and embedded report gate receipt.
- Include artifact refs for the report, suite, actor registry, effect log, policy gate, capability gate, budget gate, and embedded gate receipt.
- Validate sealed bundles by recomputing embedded report refs and exact gate receipts before accepting the bundle as pass evidence.
- Continue treating failure repro bundles as diagnostics only.
- Write the embedded report gate receipt alongside `refs.preserves` during CLI repro export.

## Impact

`molten test repro export` now emits sealed report repro bundles by default. Older unsealed report bundles remain parseable for compatibility, but sealed bundles provide the first-class portable pass artifact shape.

## Why

OpenSpec reviews repeatedly flag incoherent specs: modified requirements absent from main specs, missing IDs, ambiguous compile-fail language, or feature contracts that conflict across artifacts. These can be caught before implementation.

## What Changes

- Add deterministic consistency checks for delta specs.
- Fail missing requirement/scenario IDs.
- Fail `MODIFIED` requirements that do not exist in the current main spec unless explicitly marked as repair/migration.
- Warn on conflicting feature contract phrases across proposal, specs, and design.

## Capabilities

### New Capabilities
- `openspec-governance.delta-spec-consistency`: Delta specs are internally coherent and traceable to existing specs when modifying behavior.

## Impact

- **Files**: OpenSpec validation/gate helpers, spec fixtures.
- **Testing**: Positive added/modified specs; negative missing-ID and missing-main-requirement fixtures.

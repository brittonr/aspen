## Why

Dogfood receipts already provide durable operator evidence for full Aspen self-hosting runs, but each stage currently records only start and finish timestamps. Operators and follow-up automation need an explicit elapsed duration field so slow or regressed stages can be compared without parsing timestamps or logs.

## What Changes

- Add optional per-stage elapsed milliseconds to dogfood run receipts.
- Populate elapsed milliseconds for every newly recorded full-run stage, including automatic receipt publication and cleanup stages.
- Render elapsed milliseconds in human-readable receipt summaries.
- Preserve compatibility with existing local and cluster-published receipts that do not have the new field.

## Capabilities

### Modified Capabilities
- `dogfood-evidence.stage-receipts`: stage receipts include explicit elapsed timing evidence.
- `dogfood-evidence.receipt-inspection.show`: receipt summaries surface the stage duration when present.

## Impact

- **Files**: `crates/aspen-dogfood/src/receipt.rs`, `crates/aspen-dogfood/src/main.rs`, `openspec/specs/dogfood-evidence/spec.md`.
- **APIs**: JSON receipt schema gains optional `elapsed_ms`; old receipts continue to parse via serde default.
- **Dependencies**: No new dependencies.
- **Testing**: focused dogfood receipt tests, strict OpenSpec validation for the change/domain, and whitespace checks.

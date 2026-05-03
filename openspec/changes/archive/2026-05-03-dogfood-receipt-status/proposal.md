## Why

`receipts list` is the first operator view of dogfood history. It must not classify a run by only the final cleanup stage because a failed build/deploy followed by successful stop is still failed acceptance evidence.

## What Changes

- Derive receipt list status from all stages.
- Show any failed stage as `failed`, otherwise running/pending/skipped/succeeded/empty by deterministic precedence.
- Add tests for a failed middle stage followed by successful cleanup.

## Impact

- **Files**: `crates/aspen-dogfood/src/main.rs`, dogfood evidence spec.
- **APIs**: No new CLI syntax; improved summary semantics.
- **Testing**: dogfood package tests and existing receipt smoke paths.

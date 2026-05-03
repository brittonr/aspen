## Why

Dogfood runs now produce durable JSON receipts, but operators still need to know the receipt directory and open raw JSON by hand. That makes post-run evidence hard to inspect, compare, or cite during debugging.

## What Changes

- Add first-class `aspen-dogfood receipts list` and `aspen-dogfood receipts show <run-id-or-path>` commands.
- Keep inspection local and read-only: commands only parse receipt files from the configured receipts directory or an explicit path.
- Render a concise operator summary by default and preserve raw JSON access with `--json`.

## Capabilities

### New Capabilities

- `dogfood-receipt-inspection`: Operators can enumerate prior dogfood receipt evidence without knowing filesystem conventions.
- `dogfood-receipt-show`: Operators can inspect a single receipt by run id or path, including stage outcomes and failure summaries.

## Impact

- **Files**: `crates/aspen-dogfood/src/main.rs`, `crates/aspen-dogfood/src/receipt.rs`, OpenSpec dogfood evidence spec.
- **APIs**: Adds CLI subcommands only; receipt schema remains `aspen.dogfood.run-receipt.v1`.
- **Dependencies**: No new dependencies expected.
- **Testing**: `cargo nextest run -p aspen-dogfood`, `openspec validate dogfood-receipt-inspection --json`, and `git diff --check`.

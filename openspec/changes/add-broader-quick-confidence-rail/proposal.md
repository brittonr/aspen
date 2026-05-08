## Why

After focused runtime-host and dogfood slices land, operators need a broader-but-still-local confidence rail that can be run before or after high-risk changes without paying the full cost of every VM/gated proof. The rail should compose existing quick checks and clearly state what it does not prove.

## What Changes

- Add a broader quick confidence rail requirement over selected Cargo/nextest, harness, docs, and OpenSpec checks.
- Require clear output/receipt of included checks and skipped gated proofs.
- Preserve the boundary that this rail is not a replacement for full dogfood or gated runtime-host execution.

## Capabilities

### Modified Capabilities
- `test-harness-runtime`: Adds a quick confidence rail contract for local preflight confidence.

## Impact

- **Files**: scripts/Nix app/check wiring, test-harness reports, docs.
- **APIs**: No public runtime API expected.
- **Testing**: the new rail itself, plus OpenSpec and whitespace checks.

## Why

Aspen's runtime-host work now has strong per-host receipts, but `main` still needs a fresh dogfood acceptance receipt that operators can inspect without replaying chat history. A current dogfood run should produce durable, secret-safe acceptance evidence for the self-hosting path.

## What Changes

- Add a fresh dogfood acceptance evidence requirement for current `main`.
- Require receipt inspection/diagnosis output to be captured or referenced as operator evidence.
- Preserve the boundary that a failed dogfood run becomes diagnostic evidence, not acceptance.

## Capabilities

### Modified Capabilities
- `dogfood-evidence`: Adds current acceptance receipt capture and operator readback expectations.

## Impact

- **Files**: dogfood receipt docs/tests and evidence artifacts may change during implementation.
- **APIs**: No new public API is required by the spec baseline.
- **Testing**: `nix run .#dogfood-local -- full`, receipt list/show/diagnose checks, OpenSpec validation, and whitespace checks.

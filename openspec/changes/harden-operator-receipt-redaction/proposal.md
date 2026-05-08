## Why

Dogfood and runtime-host receipts are becoming the operator trust surface for Aspen. Before publishing or collecting more receipts, Aspen should harden redaction guarantees so tickets, cookies, private keys, connection strings, and raw secrets cannot leak through receipt summaries or diagnosis output.

## What Changes

- Add redaction requirements for operator-visible dogfood/runtime evidence output.
- Require negative tests with secret markers across receipt rendering and diagnosis surfaces.
- Keep raw protected files out of public artifacts.

## Capabilities

### Modified Capabilities
- `dogfood-evidence`: Strengthens secret-safe operator receipt rendering and diagnosis behavior.

## Impact

- **Files**: receipt rendering helpers, CLI diagnosis/list/show tests, docs/evidence notes.
- **APIs**: May introduce pure render/diagnose helpers if needed for tests.
- **Testing**: focused unit tests with secret markers, receipt CLI/render tests, OpenSpec validation, whitespace checks.

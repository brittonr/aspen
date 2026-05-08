## Why

Recent runtime-host work added stronger suite manifest validation, generated inventory expectations, and docs guardrails. The `aspen-testing` harness API should be reviewed so reusable test inventory and assertion helpers are clearly separated from adapter-specific VM, patchbay, network, and runtime dependencies.

## What Changes

- Add a follow-up public API review requirement for `aspen-testing` and related core harness crates.
- Require negative dependency checks for reusable defaults versus adapter surfaces.
- Require operator-friendly inventory/report APIs to remain stable enough for runtime-host acceptance checks.

## Capabilities

### Modified Capabilities
- `testing-harness-extraction`: Adds a follow-up API review focused on runtime-host inventory and reusable harness boundaries.

## Impact

- **Files**: `crates/aspen-testing*`, suite inventory APIs, harness docs/tests, dependency policy checks.
- **APIs**: Possible public helper reshaping or re-export tightening during implementation.
- **Testing**: crate tests, negative dependency checks, harness check/export, OpenSpec validation, whitespace checks.

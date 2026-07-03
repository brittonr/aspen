## Why

Lifecycle transition receipts are proof-carrying evidence only if their canonical refs are stable and bind the transition they claim to decide. Replaying the same transition input should produce identical transition refs, receipt refs, decisions, diagnostics, and rendered values.

## What Changes

- Add requirements for lifecycle receipt determinism.
- Add requirements for receipt-to-transition binding and tamper rejection.
- Require positive and negative evidence for stable canonical receipt generation.

## Impact

- **Files**: lifecycle receipt rendering, parsers or validators if needed, and lifecycle tests.
- **Testing**: deterministic replay of identical inputs, input-drift ref changes, and tampered receipt denial.

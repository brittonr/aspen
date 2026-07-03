## Why

A denied lifecycle transition is only useful as proof evidence if the denial is deterministic and explains exactly which predicate failed. Invalid jumps, mismatched actions, malformed refs, and combined failures should fail closed without panics or ambiguous diagnostics.

## What Changes

- Add requirements for deterministic lifecycle denial diagnostics.
- Require denial receipts to bind the transition ref, decision, diagnostics, and checks.
- Make positive and negative diagnostic coverage explicit in lifecycle proof tasks.

## Impact

- **Files**: lifecycle receipt diagnostics and lifecycle tests.
- **Testing**: malformed input, invalid transition, action mismatch, and combined-failure tests.

## Why

Replay can only be trusted as deterministic evidence when the recorded effect log is complete, ordered, profile-bound, and used exactly once by replay. Existing replay receipts deny live-effect fallback, but effect logs should also fail closed for gaps, duplicates, stale request/response bindings, handler-profile mismatches, and unused entries.

## What Changes

- Add effect-log validation that requires monotonic sequence metadata, no gaps, no duplicate request refs, and no unused recorded responses.
- Bind effect request and response refs directly to the replay handler profile, run identity, turn/boundary position, and effect kind.
- Deny replay when a recorded response belongs to a different request, profile, effect kind, or run identity.
- Add positive and negative tests for valid logs, missing entries, extra entries, reordered entries, duplicate entries, request/response mismatch, profile mismatch, and live effect fallback.

## Impact

- **Files**: deterministic replay core, harness report validation, effect-log parser/DTOs, replay CLI diagnostics, and tests.
- **Testing**: focused effect-log unit tests, harness replay regression tests, CLI malformed-log tests, and release dogfood replay smoke after implementation.
- **Boundaries**: effect-log checks remain replay evidence only and do not grant authority, policy, provenance, resource, transport, source-gate, or execution trust.

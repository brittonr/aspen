# Change: executor-resource-gate-receipts

## Why

Executor resource receipts are currently present inside Steel/Wasm execution events, but gate receipts should name those checks directly so pass evidence visibly binds execution resource limits, byte bounds, and reviewed executor outputs.

## What

- Add an `executor-execution-receipts` artifact ref to gate receipts, derived from all Steel/Wasm execution receipts in the report.
- Add explicit gate checks for Steel resource bounds, Wasm ABI byte bounds, guest memory bounds, and executor output-ref binding.
- Validate the new receipt refs/checks when parsing gate receipts.

## Impact

Gate receipts become a concise pass-evidence summary for executor hardening without replaying full report text manually. This does not relax executor validation; it promotes already-recomputed evidence to the signed/gated artifact surface.

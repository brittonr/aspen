# Design: executor-resource-gate-receipts

## Receipt derivation

During gate checking, Molten scans report observations for Steel and Wasm execution receipt events. It builds a canonical Preserves sequence of those receipts and stores its hash as the `executor-execution-receipts` artifact ref in the gate receipt.

## Checks

The gate receipt check list includes:

- `executor-execution-receipt-binding` — the gate receipt names the canonical aggregate ref for execution receipts.
- `steel-resource-bounds` — Steel execution receipts include bounded fuel/hostcall/IO resource evidence.
- `wasm-abi-byte-bounds` — Wasm ABI receipts bind input/output refs and output byte limits.
- `wasm-guest-memory-bounds` — Wasm execution receipts include guest memory-bound checks.
- `executor-output-ref-binding` — executor output refs remain report/replay-bound.

## Compatibility

The report schema is unchanged. Only newly emitted gate receipts include the aggregate execution receipt ref and checks; gate receipt parsing requires them for current pass evidence.

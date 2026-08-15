## Why

The `iroh-sam-dataspace` slice proves remote SAM envelopes, local Iroh-shaped transport, live Iroh boundary helpers, deterministic delivery logs, and gate receipts at the library level. Molten now needs an operator-visible path so the milestone is demonstrable without writing Rust tests.

## What Changes

- Add CLI commands under `molten test remote` for building envelopes, publishing/delivering them through the deterministic local Iroh-shaped adapter, running the two-peer service-ready scenario, and checking remote dataspace gate receipts.
- Add an example fixture for the remote service-ready scenario.
- Emit canonical Preserves artifacts for envelopes, transport receipts, admission receipts, delivery logs, and remote dataspace gate receipts.
- Keep deterministic replay mandatory for pass evidence; unrecorded live transport remains non-gateable.

## Impact

Operators and tests can exercise the remote SAM/Iroh milestone through stable CLI commands. The CLI remains an imperative shell around canonical Preserves artifacts; it does not add new semantics or bypass admission/evidence requirements.

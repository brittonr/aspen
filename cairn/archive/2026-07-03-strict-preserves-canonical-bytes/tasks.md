# Tasks: strict-preserves-canonical-bytes

## Phase 1: Core decode

- [x] [serial] r[molten.preserves_canonical_bytes.strict_decode] Add a pure strict canonical packed-byte decoder in `preserves_rail`.
- [x] [parallel] r[molten.preserves_canonical_bytes.strict_decode] Add positive tests proving Molten-produced canonical bytes roundtrip unchanged.
- [x] [parallel] r[molten.preserves_canonical_bytes.noncanonical_denial] Add negative tests for parseable non-canonical bytes, truncated bytes, trailing bytes, and tampered bytes.

## Phase 2: Boundary adoption

- [x] [serial] r[molten.preserves_canonical_bytes.trust_boundaries] Route ledger, chunk store, typed storage, remote dataspace, node Iroh, Iroh exchange, and Wasm executor canonical byte reads through strict decode.
- [x] [parallel] r[molten.preserves_canonical_bytes.noncanonical_denial] Ensure each external boundary emits deterministic diagnostics or deny receipts when strict decode fails.

## Phase 3: Evidence and validation

- [x] [serial] r[molten.preserves_canonical_bytes.strict_decode] r[molten.preserves_canonical_bytes.trust_boundaries] Run focused Preserves, ledger, transport, storage, and Wasm tests.

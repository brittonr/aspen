## Why

The remaining Raft non-crypto proof gaps are small but non-trivial arithmetic and contiguity facts in apply-request and write-batcher specs. These should be closed separately from chain cryptographic assumptions so Raft's operational model has no avoidable trusted arithmetic or accounting markers.

## What Changes

- **Apply request proofs**: Close version increment and batch last-applied facts.
- **Batcher add/flush proofs**: Close byte-accounting and ordered-batch contiguity facts.
- **Evidence**: Run Raft Verus root and focused write-batcher/KV apply tests.

## Capabilities

### Modified Capabilities
- `raft`: Non-crypto Raft Verus proof markers are reduced to zero or narrowed to explicit model blockers.
- `verus-proof-trust`: Raft arithmetic/accounting trust is separated from cryptographic chain trust.

## Impact

- **Files**: `crates/aspen-raft/verus/{apply_request_spec.rs,batcher_add_spec.rs,batcher_flush_spec.rs}`.
- **APIs**: No public runtime API changes expected.
- **Dependencies**: None expected.
- **Testing**: `nix develop -c verus --triggers-mode silent --crate-type=lib crates/aspen-raft/verus/lib.rs`, focused `aspen-raft` write-batcher and apply/KV tests.

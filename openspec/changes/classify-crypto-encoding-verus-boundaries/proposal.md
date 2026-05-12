## Why

Many remaining `external_body` markers are not ordinary proof gaps: they model cryptographic, MAC, hash, and tuple encoding assumptions that Verus cannot prove from Aspen source alone. These must be classified as explicit trusted boundaries, replaced by smaller verified shape/admission facts where possible, and backed by runtime/library tests instead of being silently counted as generic proof debt.

## What Changes

- **Crypto/hash classification**: Classify Blake3, HMAC, collision resistance, determinism, and output-length assumptions.
- **Encoding classification**: Split tuple and little-endian encoding markers into shape/order facts that can be proved versus library axioms that must remain trusted.
- **Boundary evidence**: Add comments/docs/tests that make each residual axiom intentional and auditable.

## Capabilities

### Modified Capabilities
- `verus-proof-trust`: Cryptographic and encoding trust boundaries are explicit, minimized, and tested at runtime where applicable.
- `commit-dag`: Commit hash assumptions are documented separately from provable mutation/parent/data shape properties.
- `raft-integrity`: Chain hash and verification assumptions distinguish shape proofs from cryptographic collision assumptions.
- `secrets`: MAC assumptions are explicit and tied to key/path/value sensitivity tests.
- `core`: Tuple encoding order/roundtrip assumptions are split into provable structural facts and library boundary axioms.

## Impact

- **Files**: `crates/aspen-core/verus/tuple_spec.rs`, `crates/aspen-commit-dag/verus/commit_hash_spec.rs`, `crates/aspen-raft/verus/{chain_hash_spec.rs,chain_verify_spec.rs}`, `crates/aspen-secrets/verus/mac_spec.rs`, plus any boundary docs/tests added.
- **APIs**: No runtime public API changes expected.
- **Dependencies**: None expected.
- **Testing**: Verus roots for touched crates plus existing runtime hash/MAC/tuple tests or added boundary tests.

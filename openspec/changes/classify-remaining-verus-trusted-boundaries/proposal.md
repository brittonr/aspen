## Why

Aspen's OpenSpec queue is drained and the direct Verus proof-gap sweep has reduced the remaining `external_body` inventory to three clusters:

- `crates/aspen-core/verus/tuple_spec.rs` — tuple encoding/order/roundtrip axioms and some structural recursive helpers.
- `crates/aspen-raft/verus/chain_verify_spec.rs` — Blake3 collision-resistance, chain-link tamper detection, and chain extension facts.
- `crates/aspen-secrets/verus/mac_spec.rs` — HMAC-SHA256 output/key/collision assumptions and MAC sensitivity wrappers.

These are no longer safe to treat as ordinary scalar proof gaps. Blindly deleting trusted bodies would either fail verification or encode cryptographic/encoding claims as unsound local proofs.

## What Changes

- **Inventory**: Capture the final remaining `external_body` set as an intentional classification target.
- **Boundary policy**: Distinguish structural facts that should be proved from crypto/encoding assumptions that should remain trusted but explicitly documented.
- **Verification rails**: Require crate-local Verus roots and focused runtime tests for any helper/model change.
- **Follow-up shape**: Split implementation into small slices: tuple structural helpers first, chain structural lemma narrowing second, MAC wrapper/axiom cleanup third.

## Capabilities

### Modified Capabilities
- `verus-proof-trust`: Adds a concrete residual-boundary classification contract for tuple encoding, Raft chain verification, and SOPS MAC specs.

## Impact

- **Files**: OpenSpec artifacts under `openspec/changes/classify-remaining-verus-trusted-boundaries/`; future implementation will touch `crates/aspen-core/verus/tuple_spec.rs`, `crates/aspen-raft/verus/chain_verify_spec.rs`, and `crates/aspen-secrets/verus/mac_spec.rs`.
- **APIs**: No runtime or public API changes from this proposal.
- **Dependencies**: None.
- **Testing**: `openspec validate --all --strict`, crate Verus roots, and focused runtime tests for affected crates.

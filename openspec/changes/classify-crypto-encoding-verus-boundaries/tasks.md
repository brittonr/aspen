## Phase 1: Inventory and classification

- [ ] [serial] Generate a fresh per-function inventory for tuple, commit-hash, chain-hash, chain-verify, and MAC `external_body` markers.
- [ ] [serial] Classify each marker as `prove-local`, `encoding-library-assumption`, or `crypto-security-assumption` in an auditable note or source comments.

## Phase 2: Local proof minimization

- [ ] [parallel] Close provable local shape/admission markers in `commit_hash_spec.rs` and `chain_hash_spec.rs` without asserting crypto security.
- [ ] [parallel] Close provable tuple size/shape/comparison wrapper facts in `tuple_spec.rs` while preserving explicit encoding axioms.
- [ ] [parallel] Close provable MAC output-length/wrapper facts in `mac_spec.rs` while preserving explicit HMAC security axioms.
- [ ] [parallel] Close provable chain verification control-flow facts in `chain_verify_spec.rs` while preserving explicit collision-resistance assumptions.

## Phase 3: Evidence and validation

- [ ] [depends:classification] Add or identify focused runtime tests for tuple ordering/roundtrip, commit hash determinism, chain hash behavior, and MAC sensitivity/output shape.
- [ ] [depends:local-proof-minimization] Run Verus roots for core, commit-dag, raft, and secrets.
- [ ] [serial] Recount residual markers and ensure every remaining crypto/encoding `external_body` is intentionally classified.
- [ ] [serial] Sync/archive this OpenSpec after classification and local minimization are complete.

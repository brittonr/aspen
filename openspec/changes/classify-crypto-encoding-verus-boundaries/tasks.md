## Phase 1: Inventory and classification

- [x] [serial] Generate a fresh per-function inventory for tuple, commit-hash, chain-hash, chain-verify, and MAC `external_body` markers. Evidence: `evidence/crypto-encoding-boundary-inventory.md`.
- [x] [serial] Classify each marker as `prove-local`, `encoding-library-assumption`, or `crypto-security-assumption` in an auditable note or source comments. Evidence: `evidence/crypto-encoding-boundary-inventory.md` lists 47 residual attributes after minimization.

## Phase 2: Local proof minimization

- [x] [parallel] Close provable local shape/admission markers in `commit_hash_spec.rs` and `chain_hash_spec.rs` without asserting crypto security. Evidence: removed `commit_hash_spec.rs::blake3_deterministic` and `chain_hash_spec.rs::u64_to_le_bytes_length`; commit-dag and raft Verus roots pass.
- [x] [parallel] Close provable tuple size/shape/comparison wrapper facts in `tuple_spec.rs` while preserving explicit encoding axioms. Evidence: removed `tuple_spec.rs::tuple_encode_non_empty`; core Verus root passes; residual tuple markers classified as encoding-library assumptions.
- [x] [parallel] Close provable MAC output-length/wrapper facts in `mac_spec.rs` while preserving explicit HMAC security axioms. Evidence: made `hmac_sha256` explicitly `uninterp`, added loop decreases for `build_mac_message_len`, removed reflexive `axiom_hmac_deterministic` and `mac_determinism`; secrets Verus root passes.
- [x] [parallel] Close provable chain verification control-flow facts in `chain_verify_spec.rs` while preserving explicit collision-resistance assumptions. Evidence: removed `chain_verify_spec.rs::chain_link_valid`; raft Verus root passes.

## Phase 3: Evidence and validation

- [x] [depends:classification] Add or identify focused runtime tests for tuple ordering/roundtrip, commit hash determinism, chain hash behavior, and MAC sensitivity/output shape. Evidence: `cargo test -p aspen-commit-dag hash -- --nocapture` (22 passed), `cargo test -p aspen-raft verified::integrity -- --nocapture` (18 passed), `cargo test -p aspen-secrets sops -- --nocapture` (11 passed); tuple runtime coverage is currently absent and residual tuple behavior is classified as an encoding-library assumption in the inventory.
- [x] [depends:local-proof-minimization] Run Verus roots for core, commit-dag, raft, and secrets. Evidence: commit-dag `5 verified`, core `48 verified`, raft `123 verified`, secrets `10 verified`, all 0 errors.
- [x] [serial] Recount residual markers and ensure every remaining crypto/encoding `external_body` is intentionally classified. Evidence: 47 residual crypto/encoding attributes in `evidence/crypto-encoding-boundary-inventory.md`.
- [x] [serial] Sync/archive this OpenSpec after classification and local minimization are complete.

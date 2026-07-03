# Tasks: secrets-redaction-state-proof

## Phase 1: Confidentiality decision core

- [x] [serial] r[molten.secrets_state_proof.exact_reveal_binding] Extract or define pure reveal/decrypt binding checks for exact secret refs, encrypted-ref ids, commitments, authority, policy, resource, effect, and output refs.
- [x] [parallel] r[molten.secrets_state_proof.redaction_profile_gate] Define pure redaction profile admissibility checks for gate-preserving, diagnostic, and private encrypted profiles.
- [x] [parallel] r[molten.secrets_state_proof.cleanup_retention_gate] Bind secret cleanup decisions to retention admission/apply/execution evidence.

## Phase 2: Positive and negative tests

- [x] [parallel] r[molten.secrets_state_proof.exact_reveal_binding] Add authorized reveal/decrypt pass tests and ciphertext-only, stale reveal, wrong encrypted-ref id, and commitment mismatch denial tests.
- [x] [parallel] r[molten.secrets_state_proof.redaction_profile_gate] Add tests denying diagnostic redaction as pass evidence and accepting only proven gate-preserving transforms.
- [x] [parallel] r[molten.secrets_state_proof.cleanup_retention_gate] Add cleanup denial tests for missing retention apply refs and cleanup pass tests with bound retention evidence.

## Phase 3: Evidence and validation

- [x] [serial] r[molten.secrets_state_proof.exact_reveal_binding] r[molten.secrets_state_proof.redaction_profile_gate] r[molten.secrets_state_proof.cleanup_retention_gate] Bind proof refs and run `cargo test secrets repro gate`.

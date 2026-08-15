# Tasks: provenance-trust-state-proof

## Phase 1: Trust-state core

- [x] [serial] r[molten.provenance_state_proof.profile_thresholds] Define or expose a pure operation-profile threshold table for provenance trust states.
- [x] [parallel] r[molten.provenance_state_proof.build_verification_binding] Harden build verification matching for artifact ref, build record ref, provenance record ref, and receipt decision.
- [x] [parallel] r[molten.provenance_state_proof.evidence_only_boundary] Add diagnostics for provenance-only evidence used as authority, policy, resource, source-gate, transport, retention, or execution trust.

## Phase 2: Tests

- [x] [parallel] r[molten.provenance_state_proof.profile_thresholds] Add positive tests for reviewed low-risk admission and reproducible/policy-trusted sensitive admission.
- [x] [parallel] r[molten.provenance_state_proof.profile_thresholds] r[molten.provenance_state_proof.build_verification_binding] Add negative tests for missing, denied, stale, wrong-artifact, weak-trust, wrong-profile, and mismatched build verification cases.
- [x] [parallel] r[molten.provenance_state_proof.evidence_only_boundary] Add node/job/remote-sync tests showing provenance receipts do not replace other gates.

## Phase 3: Evidence and validation

- [x] [serial] r[molten.provenance_state_proof.profile_thresholds] r[molten.provenance_state_proof.build_verification_binding] r[molten.provenance_state_proof.evidence_only_boundary] Bind proof refs and run `cargo test provenance node job`.

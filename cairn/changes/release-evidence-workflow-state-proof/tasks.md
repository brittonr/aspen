# Tasks: release-evidence-workflow-state-proof

## Phase 1: Release workflow validator

- [ ] [serial] r[molten.release_workflow_state_proof.ordered_workflow] Define pure ordered release workflow validation over dogfood, bundle export, verify, signed-member, promotion, signed promotion, summary, export, and export-verify evidence.
- [ ] [parallel] r[molten.release_workflow_state_proof.signature_binding] Harden signature checks for subject ref, key id, purpose, currentness, revocation, and required signed-member class.
- [ ] [parallel] r[molten.release_workflow_state_proof.evidence_only_boundary] Add downstream evidence-only misuse diagnostics.

## Phase 2: Tests

- [ ] [parallel] r[molten.release_workflow_state_proof.ordered_workflow] Add complete release workflow pass tests and promotion-before-bundle-verify denial tests.
- [ ] [parallel] r[molten.release_workflow_state_proof.signature_binding] Add missing member, duplicate path, tampered member, unsigned required member, wrong signer, wrong purpose, revoked key, and stale proof ref denial tests.
- [ ] [parallel] r[molten.release_workflow_state_proof.evidence_only_boundary] Add subsystem gate tests proving release receipts cannot replace authority, policy, provenance, source-gate, retention, resource, transport, or destructive-operation evidence.

## Phase 3: Evidence and validation

- [ ] [serial] r[molten.release_workflow_state_proof.ordered_workflow] r[molten.release_workflow_state_proof.signature_binding] r[molten.release_workflow_state_proof.evidence_only_boundary] Bind proof refs and run `cargo test dogfood receipts catalog`.

# Tasks: layered-proof-evidence-contract

## Phase 1: Layer model

- [ ] [serial] r[molten.evidence.layered_proof.contract] Define proof evidence layers and cross-layer binding rules.
- [ ] [serial] r[molten.evidence.layered_proof.pure_core_receipts] Identify pure-core proof evidence requirements.
- [ ] [serial] r[molten.evidence.layered_proof.gate_receipts] Identify gate proof evidence requirements.

## Phase 2: Replay, release, and readback

- [ ] [parallel] r[molten.evidence.layered_proof.replay_receipts] Bind replay evidence to gate/core refs.
- [ ] [parallel] r[molten.evidence.layered_proof.release_receipts] Bind release evidence to replay/gate/core refs.
- [ ] [parallel] r[molten.evidence.layered_proof.operator_readbacks] Keep operator readbacks non-normative and evidence-only.
- [ ] [serial] r[molten.evidence.layered_proof.cross_layer_boundary] Deny stale, cyclic, wrong-scope, or diagnostic-as-pass layer graphs.

## Phase 3: Hegel RS and docs

- [ ] [parallel] r[molten.evidence.layered_proof.hegel_properties] Add Hegel RS generated layer graph properties.
- [ ] [serial] r[molten.evidence.layered_proof.docs] Document the layered proof model and evidence-only boundaries.

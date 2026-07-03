# Tasks: evidence-chain-head-transition-proof

## Phase 1: Head transition law

- [x] [serial] r[molten.evidence_chain_state_machine_proof.head_transition_continuity] Add tests proving genesis and append links advance chain heads only when head-before, head-after, appended-link, payload, and predicate receipt refs are continuous.
- [x] [parallel] r[molten.evidence_chain_state_machine_proof.head_transition_continuity] Add negative tests for stale observed heads, tampered payload refs, missing predicate receipts, and duplicate sequence conflicts.

## Phase 2: Gap and fork denial

- [x] [serial] r[molten.evidence_chain_state_machine_proof.gap_fork_denial] Add generated or fixture chain segments that include linear, gap, and fork cases.
- [x] [parallel] r[molten.evidence_chain_state_machine_proof.gap_fork_denial] Assert gap and fork verification emits deny receipts with first-invalid-link diagnostics.

## Phase 3: Checkpoint and retention

- [x] [serial] r[molten.evidence_chain_state_machine_proof.checkpoint_anchor_preservation] Add tests proving checkpoints bind verified ranges and retention/GC preserves links and payload artifacts reachable from retained heads, anchors, checkpoints, or signed receipts.
- [x] [parallel] r[molten.evidence_chain_state_machine_proof.checkpoint_anchor_preservation] Add negative tests for missing checkpoint content, stale checkpoint refs, and unanchored garbage-collection candidates.

## Phase 4: Validation

- [x] [serial] r[molten.evidence_chain_state_machine_proof.head_transition_continuity] r[molten.evidence_chain_state_machine_proof.gap_fork_denial] r[molten.evidence_chain_state_machine_proof.checkpoint_anchor_preservation] Add traceability evidence and run focused chain/evidence tests.

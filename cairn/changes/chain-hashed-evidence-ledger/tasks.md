# Tasks: chain-hashed-evidence-ledger

## Phase 1: Canonical link model

- [x] [serial] r[molten.evidence.chain_hashing.link_model] Define canonical `chain-link-v1` DTOs with scope/id/epoch, sequence, previous link ref, payload kind/ref/schema, context refs, producer refs, Trellis predicate refs, and checks.
- [x] [serial] r[molten.evidence.chain_hashing.link_identity] Compute link identity only from canonical Preserves bytes and keep payload refs unchanged by linking.
- [x] [serial] r[molten.evidence.chain_hashing.genesis_append] Implement pure validation for genesis and append links, including previous-link binding and sequence monotonicity.
- [x] [parallel] r[molten.evidence.chain_hashing.no_global_chain] Document and test that chain hashing is scoped evidence continuity, not a global message-ordering chain or cryptocurrency ledger.

## Phase 2: Ledger storage and receipts

- [x] [serial] r[molten.evidence.chain_hashing.ledger_index] Store chain links as immutable ledger artifacts and derive rebuildable indexes for chain scope/id/epoch, parent/child, sequence, payload ref, heads, anchors, and checkpoints.
- [x] [serial] r[molten.evidence.chain_hashing.append_receipts] Emit canonical append receipts naming head-before, head-after, appended link ref, payload ref, and continuity checks.
- [x] [serial] r[molten.evidence.chain_hashing.verify_receipts] Emit canonical segment verify receipts for anchor-to-head validation with gap, fork, stale-head, and missing-payload diagnostics.
- [x] [parallel] r[molten.evidence.chain_hashing.signed_receipts] Integrate signed receipt envelopes so append/verify receipts can be signed and links can payload signed receipt refs without changing subject hashes.

## Phase 3: Trellis predicates and checkpoints

- [x] [serial] r[molten.evidence.chain_hashing.trellis_append_predicates] Add Trellis-backed bounded predicates for genesis validity, append validity, no-gap segments, no-fork policy, descent from anchor, and checkpoint range coverage.
- [x] [serial] r[molten.evidence.chain_hashing.control_plane_checkpoints] Add optional Trellis/Raft control-plane checkpoint commands for accepted chain heads and verified ranges.
- [x] [parallel] r[molten.evidence.chain_hashing.fork_policy] Represent fork evidence and make production profiles reject unexpected forks while diagnostic profiles can retain fork artifacts.
- [x] [parallel] r[molten.evidence.chain_hashing.anchor_policy] Add policy fixtures for trusted anchors, expected heads, stale-head rejection, and checkpoint freshness.

## Phase 4: Gate and runtime integration

- [x] [serial] r[molten.evidence.chain_hashing.gate_receipts] Extend gate receipts so configured evidence profiles can require chain continuity, anchor descent, and checkpoint freshness for pass evidence.
- [x] [parallel] r[molten.evidence.chain_hashing.turn_journals] Add optional actor/session turn-journal chains that bind input, admission, effect-log, trace, and before/after state refs without introducing a global actor-message head.
- [x] [parallel] r[molten.evidence.chain_hashing.artifact_lineage] Add artifact/chunk publication lineage chains for manifests, catalog entries, and remote sync receipts.
- [x] [parallel] r[molten.evidence.chain_hashing.iroh_exchange] Allow Iroh exchange of chain segments and checkpoints with local verification before ledger import.

## Phase 5: Tests

- [x] [serial] r[molten.evidence.chain_hashing.identity_tests] Add tests that identical canonical links produce stable refs and that payload refs are preserved.
- [x] [serial] r[molten.evidence.chain_hashing.tamper_tests] Add tests rejecting changed previous refs, sequence gaps, mismatched payload refs, stale heads, and unavailable payloads.
- [x] [serial] r[molten.evidence.chain_hashing.fork_tests] Add tests that two children from the same parent are detected and rejected under no-fork policy.
- [x] [parallel] r[molten.evidence.chain_hashing.checkpoint_tests] Add tests for accepted anchors/checkpoints, checkpoint freshness, and Raft/Trellis control-plane checkpoint receipts.
- [x] [parallel] r[molten.evidence.chain_hashing.property_tests] Add Hegel property tests over bounded chain segments for append determinism, no-gap validation, fork detection, and anchor descent.

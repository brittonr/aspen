# Tasks: eventual-propagation-surfaces

## Phase 1: Surface model

- [x] [serial] r[molten.eventual_surface.manifest] Define `eventual-surface-manifest-v1` with scope, carrier, payload schema, idempotency key, merge law, retraction/tombstone policy, anti-entropy policy, replay evidence, and authority boundaries.
- [x] [serial] r[molten.eventual_surface.merge_law] Implement pure validation for deterministic merge/convergence law declarations before any surface can claim eventual consistency.
- [x] [parallel] r[molten.eventual_surface.not_consensus] Update docs and receipt checks to label gossip/docs/federation as propagation surfaces, not consensus or authority.

## Phase 2: Evidence and replay

- [x] [serial] r[molten.eventual_surface.replay_boundary] Require recorded delivery logs, snapshots, or anti-entropy receipts before live propagation evidence can satisfy deterministic pass gates.
- [x] [parallel] r[molten.eventual_surface.anti_entropy_status] Represent anti-entropy, missing-set, import, denial, and peer availability status as local dataspace assertions.
- [x] [parallel] r[molten.eventual_surface.remote_sync_boundary] Ensure receiver-driven artifact sync imports only after verification/admission and never from propagation hints alone.

## Phase 3: Tests and diagnostics

- [x] [serial] r[molten.eventual_surface.positive_negative_tests] Add positive convergence fixtures and negative tests for missing merge law, conflicting state without resolver, stale tombstone, unrecorded live timing, and propagation-as-authority attempts.
- [x] [parallel] r[molten.eventual_surface.diagnostics] Add diagnostics that distinguish propagation delivered, merged, replayable, authoritative, and consensus-backed states.

## Phase 4: Validation

- [x] [serial] r[molten.eventual_surface.validation] Run focused eventual-surface tests, remote dataspace/federation/sync tests, formatting, and Cairn validation before archiving.

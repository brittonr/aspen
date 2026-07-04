# Tasks: multinode-failure-repro-bundles

## Phase 1: Bundle schema and sealing

- [ ] [parallel] r[molten.testing.multinode.failure_repro_bundle] Define a sealed multinode failure repro bundle schema for scenario fixture, topology, scheduler, seed, fault plan, commands, node summaries, receipts, diagnostics, logs, redaction policy, and replay status.
- [ ] [parallel] r[molten.testing.multinode.failure_repro_privacy_and_replay] Define privacy and replay validation rules for redacted diagnostics, encrypted private attachments, deterministic replay, non-replayable VM observations, and diagnostic-only bundles.

## Phase 2: Verify, unpack, and replay

- [ ] [serial] r[molten.testing.multinode.failure_repro_bundle] Add pure verify logic that recomputes embedded refs, checks seal metadata, classifies replay support, and emits canonical verification receipts.
- [ ] [serial] r[molten.testing.multinode.failure_repro_privacy_and_replay] Add unpack and replay paths that materialize only verified redacted content and fail closed for private or tampered content.

## Phase 3: Fixtures and docs

- [ ] [parallel] r[molten.testing.multinode.failure_repro_bundle] Add positive fixtures for simulation failure replay and VM failure verification.
- [ ] [parallel] r[molten.testing.multinode.failure_repro_privacy_and_replay] Add negative fixtures for tampered topology, tampered receipt, missing redaction transform, missing reveal, stale fixture, unsealed bundle, and diagnostic-only pass misuse.
- [ ] [serial] r[molten.testing.multinode.failure_repro_bundle] Run focused repro bundle tests and `cairn validate --root .`, or record the blocker and next best check.

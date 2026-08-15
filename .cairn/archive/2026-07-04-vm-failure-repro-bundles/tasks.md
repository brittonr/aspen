# Tasks: vm-failure-repro-bundles

## Phase 1: Bundle model

- [x] [parallel] r[molten.testing.multinode.vm_failure_repro_export] Define VM failure repro bundle inputs for scenario fixture refs, topology refs, node summaries, child receipts, validation refs, diagnostics, replay status, redaction policy, privacy markers, and caveats.
- [x] [parallel] r[molten.testing.multinode.vm_failure_repro_privacy_gate] Extend failure bundle verification and pass-gate logic for VM-specific tamper, privacy, reveal, redaction, stale-ref, and diagnostic-only denial cases.

## Phase 2: VM export shell

- [x] [serial] r[molten.testing.multinode.vm_failure_repro_export] Add VM shard or aggregate export plumbing that writes a sealed diagnostic bundle when validation denies, unavailable host support is recorded, or required evidence is missing.
- [x] [serial] r[molten.testing.multinode.vm_failure_repro_privacy_gate] Ensure private attachments require exact reveal receipts before materialization and remain rejected for pass-gate use.

## Phase 3: Positive and negative coverage

- [x] [parallel] r[molten.testing.multinode.vm_failure_repro_export] Add positive tests for denied shard export, unavailable fault export, and non-replayable VM/live observation classification.
- [x] [parallel] r[molten.testing.multinode.vm_failure_repro_privacy_gate] Add negative tests for tampered topology, stale node summary, missing redaction policy, private-without-reveal, unsealed bundle, and diagnostic bundle used as pass evidence.
- [x] [serial] r[molten.testing.multinode.vm_failure_repro_export] Run focused failure bundle verification tests and a smallest failing VM-shard fixture or dry-run export, or record host-support blockers and next best checks.

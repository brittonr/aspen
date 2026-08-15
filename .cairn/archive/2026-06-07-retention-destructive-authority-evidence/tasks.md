## Phase 1: Evidence plumbing

- [x] [serial] r[molten.retention.destructive_evidence_inputs] Add explicit destructive retention evidence inputs for ledger GC, chunk GC, and cache invalidation.
- [x] [serial] r[molten.retention.apply_requires_authority] Deny apply-mode destructive candidates when requester, policy, authority, or evidence refs are missing.
- [x] [serial] r[molten.retention.reference_index_plumbing] Thread retained refs, remote refs, and reference-index completeness through subsystem retention evaluations.

## Phase 2: Receipts, CLI, and tests

- [x] [parallel] r[molten.retention.cli_evidence_flags] Expose common retention evidence flags in destructive CLI commands.
- [x] [parallel] r[molten.retention.evidence_summary_receipts] Bind retention evidence summaries in subsystem GC/invalidation receipts.
- [x] [parallel] r[molten.retention.destructive_evidence_tests] Test missing authority/policy/evidence, incomplete index, retained refs, and remote uncertainty denial paths.

## Phase 1: Receipt binding

- [x] [serial] r[molten.testing.repro_reveal_encrypted_ref_binding.receipt_field] Add canonical reveal receipt encrypted-ref binding while keeping non-repro legacy reveal receipts parseable.
- [x] [serial] r[molten.testing.repro_reveal_encrypted_ref_binding.unpack_match] Require repro unpack reveal receipts to match exact bundle encrypted refs through the dedicated binding.

## Phase 2: Fail-closed coverage

- [x] [parallel] r[molten.testing.repro_reveal_encrypted_ref_binding.unpack_match] Reject stale or replayed reveal receipts that only match via secret or commitment refs.
- [x] [parallel] r[molten.testing.repro_reveal_encrypted_ref_binding.partial_coverage_denial] Preserve partial-coverage denial when any encrypted ref lacks a matching reveal receipt.
- [x] [parallel] r[molten.testing.repro_reveal_encrypted_ref_binding.evidence_only] Document that reveal binding materializes private repro content only and does not make bundles gate-preserving.

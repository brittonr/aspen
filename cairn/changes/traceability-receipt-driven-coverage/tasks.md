# Tasks: traceability-receipt-driven-coverage

## Phase 1: Derivation core

- [ ] [serial] r[molten.testing.receipt_driven_traceability.source_model] Define receipt-backed coverage source inputs.
- [ ] [serial] r[molten.testing.receipt_driven_traceability.coverage_derivation] Implement pure derivation from validated receipts to coverage entries.
- [ ] [serial] r[molten.testing.receipt_driven_traceability.stale_receipt_denial] Deny stale, duplicate, wrong-kind, and wrong-requirement receipts.

## Phase 2: CLI and policy

- [ ] [parallel] r[molten.testing.receipt_driven_traceability.raw_claim_policy] Label raw coverage strings as compatibility-only and make release policy able to require receipt-backed entries.
- [ ] [parallel] r[molten.testing.receipt_driven_traceability.nix_gate] Expose receipt-backed traceability through the release/Nix gate surface.

## Phase 3: Hegel RS and docs

- [ ] [parallel] r[molten.testing.receipt_driven_traceability.hegel_properties] Add Hegel RS properties for deterministic derivation and stale-receipt denial.
- [ ] [serial] r[molten.testing.receipt_driven_traceability.docs] Document receipt-driven coverage and compatibility-only tuple handling.

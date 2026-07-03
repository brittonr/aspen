# Tasks: proof-verification-run-receipts

## Phase 1: Receipt core

- [ ] [serial] r[molten.testing.verification_run_receipts.schema] Define canonical `verification-run-receipt-v1` DTOs and pure validation.
- [ ] [serial] r[molten.testing.verification_run_receipts.command_binding] Bind normalized argv, target, toolchain/profile refs, exit status, and captured output refs.
- [ ] [serial] r[molten.testing.verification_run_receipts.artifact_binding] Bind produced artifact refs and fail closed on stale or malformed refs.

## Phase 2: Traceability integration

- [ ] [serial] r[molten.testing.verification_run_receipts.traceability] Accept receipt-backed positive and negative coverage in traceability scanning.
- [ ] [parallel] r[molten.testing.verification_run_receipts.compatibility] Keep raw coverage strings as compatibility input while marking receipt-backed evidence as preferred.

## Phase 3: Hegel RS and fixtures

- [ ] [parallel] r[molten.testing.verification_run_receipts.hegel_properties] Add Hegel RS property tests for stable refs, binding drift, and stale artifact denial.
- [ ] [parallel] r[molten.testing.verification_run_receipts.fixtures] Add positive and negative CLI fixtures for pass receipts, deny receipts, stale targets, and tampered artifact refs.
- [ ] [serial] r[molten.testing.verification_run_receipts.docs] Document receipt-backed traceability usage and examples.

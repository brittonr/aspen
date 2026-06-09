## Phase 1: Bundle schema and CLI

- [x] [serial] r[molten.operator_dogfood_release_evidence_bundle.export] Add canonical release evidence bundle export for dogfood Nix outputs.
- [x] [serial] r[molten.operator_dogfood_release_evidence_bundle.verify] Add bundle verification receipts that recompute output refs and deny stale or tampered members.
- [x] [parallel] r[molten.operator_dogfood_release_evidence_bundle.evidence_only] Preserve the evidence-only release review boundary.

## Phase 2: Nix and tests

- [x] [serial] r[molten.operator_dogfood_release_evidence_bundle.nix_check] Preserve and verify release bundles in the `dogfood-local-node` Nix check output.
- [x] [serial] r[molten.operator_dogfood_release_evidence_bundle.tests] Cover bundle export, verification pass, and stale-member denial in tests.
- [x] [parallel] r[molten.operator_dogfood_release_evidence_bundle.docs] Document release bundle commands and output artifacts.

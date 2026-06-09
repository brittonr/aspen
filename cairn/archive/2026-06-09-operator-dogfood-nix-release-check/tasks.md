## Phase 1: Nix release check

- [x] [serial] r[molten.operator_dogfood_nix_release_check.check] Add a Nix check that runs `molten dogfood local-node` with an explicit temporary state root.
- [x] [serial] r[molten.operator_dogfood_nix_release_check.nextest_dependency] Bind the dogfood check to the existing hermetic nextest check output.
- [x] [parallel] r[molten.operator_dogfood_nix_release_check.artifacts] Store dogfood report, release gate receipt, human summary, and nextest dependency marker as Nix check outputs.

## Phase 2: Docs and validation

- [x] [serial] r[molten.operator_dogfood_nix_release_check.docs] Document the Nix dogfood release check and evidence-only boundary.
- [x] [serial] r[molten.operator_dogfood_nix_release_check.validation] Validate Nix dogfood check, Cairn gates, and Rust checks before archiving.

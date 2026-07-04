# Tasks: contract-diagnostic-locality

- [ ] [serial] r[molten.project.contract_diagnostics.locality] Refactor selected contract modules from large opaque predicates toward field-level contracts plus small named cross-field predicates.
- [ ] [parallel] r[molten.project.contract_diagnostics.locality] Update negative fixture names or expectations so malformed ref, enum, path, duplicate, stale-reference, and cross-field failures identify the intended invariant.
- [ ] [serial] r[molten.project.contract_diagnostics.no_validation_weakening] Run positive and negative fixture validation before and after diagnostic refactors to prove invalid fixtures still fail.
- [ ] [parallel] r[molten.project.contract_diagnostics.no_validation_weakening] Document any diagnostics that remain opaque because Nickel cannot expose a more precise field path without weakening the contract.
- [ ] [serial] r[molten.project.contract_diagnostics.locality] Run focused Nickel fixture validation and `cairn validate --root .`, or record the blocker and next best check.

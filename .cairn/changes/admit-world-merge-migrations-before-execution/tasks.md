# Tasks

## Baseline and contract

- [x] [serial] Run the existing world-merge core and shell tests and retain baseline failures before edits. r[molten.audit_f06.validation]
- [x] [serial] Add a failing public-preparation regression for an unadmitted source-to-target binding with a recording migration adapter. r[molten.audit_f06.admission] r[molten.audit_f06.validation]
- [x] [serial] Define the typed admitted conversion and prepared-result contracts, including original identity, target binding, and named bounds. r[molten.audit_f06.binding]

## Core and shell

- [x] [serial] Implement pure admission over original schema and binding facts before adapter dispatch. r[molten.audit_f06.admission]
- [x] [serial] Update shell execution and result validation so normalized schemas cannot erase admission evidence. r[molten.audit_f06.binding] r[molten.audit_f06.publication]
- [x] [parallel] Add positive unchanged-value and admitted-conversion cases with exact call order and result identity. r[molten.audit_f06.validation]
- [x] [parallel] Add negative missing, malformed, stale, oversized, failed, and substituted result cases with unchanged heads and no publication. r[molten.audit_f06.publication] r[molten.audit_f06.validation]

## Compatibility and validation

- [x] [serial] Version changed canonical carriers where required and verify compatibility without admitting historical evidence as current authority. r[molten.audit_f06.binding] r[molten.audit_f06.publication]
- [x] [serial] Update world-merge documentation and add combined migration/output cases with the absent-root change. r[molten.audit_f06.binding] r[molten.audit_f06.validation]
- [x] [serial] Rerun focused tests, workspace tests, Clippy with denied warnings, scoped strict Octet, relevant Nix checks, and Cairn validation and gates. Record exact results and blockers. r[molten.audit_f06.validation]

Executed observations are recorded in [the F06 verification record](evidence/2026-10-20-verification.md). The pre-fix baseline was replayed in an isolated `514ca673c` checkout after resumption; no chronological pre-edit capture in the active tree is claimed. Combined coverage exercises absent input sides with converted output; it does not mark the separate absent-output carrier change complete. No lifecycle completion or publication is implied by these implementation boxes.

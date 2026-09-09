# Tasks

All tasks remain proposed. This package grants no implementation or publication permission.

## Baseline and current-run contract

- [ ] [serial] Run current node-loop and restart receipt tests before core edits and retain the exact baseline. r[molten.audit_f14.validation]
- [ ] [serial] Add a failing normal regression for shutdown R, restart, and duplicate R with no repeated shutdown effects and no false stopped observation. r[molten.audit_f14.duplicate_scope] r[molten.audit_f14.validation]
- [ ] [serial] Review startup and active-lock identities, including identical startup content across distinct runs, before selecting the explicit run binding. r[molten.audit_f14.current_binding]

## Core and shell

- [ ] [serial] Implement pure classification that separates duplicate provenance from current stopped, active, and unresolved observations. r[molten.audit_f14.duplicate_scope] r[molten.audit_f14.current_binding]
- [ ] [serial] Supply exact current-run observations through node-host capabilities and preserve shutdown admission from the F01 package. r[molten.audit_f14.current_binding] r[molten.audit_f14.preserve_state]
- [ ] [parallel] Add positive current shutdown and same-run duplicate controls with exact current-run bindings. r[molten.audit_f14.current_binding] r[molten.audit_f14.validation]
- [ ] [parallel] Add negative historical duplicates, equal startup content, missing artifacts, read failures, and conflicting bindings with unchanged active state. r[molten.audit_f14.preserve_state] r[molten.audit_f14.validation]

## Compatibility and evidence

- [ ] [serial] Keep legacy receipts historical and version any changed canonical outcome fields without rewriting old evidence or automatically reexecuting requests. r[molten.audit_f14.duplicate_scope] r[molten.audit_f14.validation]
- [ ] [serial] Update node operator and replay documentation so duplicate success does not imply current shutdown. r[molten.audit_f14.current_binding] r[molten.audit_f14.validation]
- [ ] [serial] Repeat focused core and adapter tests, then run workspace tests, Clippy with denied warnings, scoped strict Octet, relevant Nix checks, and required Cairn gates. Record exact results and blockers. r[molten.audit_f14.validation]

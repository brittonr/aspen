# Tasks

Implementation proceeds under the operator's bounded local completion authorization. Checked tasks refer to recorded work; required acceptance remains open until its gates pass. This package does not authorize publication.

## Baseline and current-run contract

- [x] [serial] Run current node-loop and restart receipt tests before core edits and retain the exact baseline. r[molten.audit_f14.validation]
- [x] [serial] Add a failing normal regression for shutdown R, restart, and duplicate R with no repeated shutdown effects and no false stopped observation. r[molten.audit_f14.duplicate_scope] r[molten.audit_f14.validation]
- [x] [serial] Review startup and active-lock identities, including identical startup content across distinct runs, before selecting the explicit run binding. r[molten.audit_f14.current_binding]

## Core and shell

- [x] [serial] Implement pure classification that separates duplicate provenance from current stopped, active, and unresolved observations. r[molten.audit_f14.duplicate_scope] r[molten.audit_f14.current_binding]
- [x] [serial] Supply exact current-run observations through node-host capabilities and preserve shutdown admission from the F01 package. r[molten.audit_f14.current_binding] r[molten.audit_f14.preserve_state]
- [x] [parallel] Add positive current shutdown and same-run duplicate controls with exact current-run bindings. r[molten.audit_f14.current_binding] r[molten.audit_f14.validation]
- [x] [parallel] Add negative historical duplicates, equal startup content, missing artifacts, read failures, and conflicting bindings with unchanged active state. r[molten.audit_f14.preserve_state] r[molten.audit_f14.validation]

## Compatibility and evidence

- [x] [serial] Keep legacy receipts historical and version any changed canonical outcome fields without rewriting old evidence or automatically reexecuting requests. r[molten.audit_f14.duplicate_scope] r[molten.audit_f14.validation]
- [x] [serial] Update node operator and replay documentation so duplicate success does not imply current shutdown. r[molten.audit_f14.current_binding] r[molten.audit_f14.validation]
- [x] [serial] Repeat focused core and adapter tests, then run workspace tests, Clippy with denied warnings, scoped strict Octet, relevant Nix checks, and required Cairn gates. Record exact results and blockers. r[molten.audit_f14.validation]

## Resumed acceptance after host recovery

The unchanged required Nix checks passed after the recorded host recovery. The
hermetic nextest run passed all 1,716 tests with no skips and emitted canonical
CI receipt `blake3:bc660b19ac58e502f7f4624e1157eabdb31b3787c71bfb62dafbdf83d5947a41`.
[Resumed acceptance](evidence/resumed-acceptance.json) links the new execution to
the unchanged attempt-17 runtime, source, feature and bounded traceability evidence.
The historical blocked attempt remains intact; native lifecycle gates, review,
sync and archive are recorded separately and are not inferred from test counts.


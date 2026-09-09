## Implementation tasks

All tasks remain proposed. This package grants no implementation permission.

- [ ] [serial] Run `local_node_init_run_status_stop_and_restart_recovery_are_receipted` and existing shutdown dispatch controls before core edits. Record baseline failures. r[molten.audit_f01.validation]
- [ ] [serial] Add the empty-authority F01 reproduction to normal repository tests, with a valid shutdown control. r[molten.audit_f01.admission] r[molten.audit_f01.validation]
- [ ] [serial] Define pure shutdown admission inputs and typed effect plans without filesystem or ambient state access. r[molten.audit_f01.admission]
- [ ] [serial] Route direct stop and queued dispatch through admission before protected shell effects. r[molten.audit_f01.admission]
- [ ] [serial] Add adapter call-order tests for valid shutdown, denied evidence, and effect errors. r[molten.audit_f01.observed_effects] r[molten.audit_f01.validation]
- [ ] [serial] Compare lifecycle state before and after missing authority, policy, resource, and malformed-binding rejection. r[molten.audit_f01.preserve_state]
- [ ] [serial] Review receipt compatibility, replay interpretation, and partial-effect outcomes with F14 owners. r[molten.audit_f01.observed_effects]
- [ ] [serial] Document shutdown admission, denial evidence, maintenance ownership, and bounded claims. r[molten.audit_f01.admission] r[molten.audit_f01.validation]
- [ ] [serial] Repeat focused baseline and regression tests after edits. Record executed results separately from static F01 evidence. r[molten.audit_f01.validation]
- [ ] [serial] Run focused Octet error checks and Clippy with `-D warnings`, without weakening existing policy. r[molten.audit_f01.validation]
- [ ] [serial] Run workspace tests across all targets and compatible features, plus relevant Nix node-state checks and required flake gates. r[molten.audit_f01.validation]
- [ ] [serial] Run required Cairn validation, traceability, and package gates before any completion review. Record blockers without pass claims. r[molten.audit_f01.validation]

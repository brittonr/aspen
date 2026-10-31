## Implementation tasks

Checked tasks record executed baseline and producer review only. The user-authorized `produce-exact-node-startup-binding` prerequisite now defines the producer contract and implementation is in progress; acceptance remains blocked until actual producer execution, independent signing, real normal-startup positive/negative controls and final gates are recorded. Authored code/tests do not check off unexecuted evidence.

- [x] [serial] Run existing node lifecycle and runtime source-gate tests before core edits. Record baseline outcomes. r[molten.audit_f02.validation]
- [x] [serial] Add the F02 normal-startup reproduction to repository tests, alongside a valid real-artifact control. r[molten.audit_f02.real_evidence] r[molten.audit_f02.validation]
- [x] [serial] Review the real artifact producer, exact source-to-binary binding, and startup evidence input compatibility. r[molten.audit_f02.candidate_binding]
- [x] [serial] Define pure admission over supplied candidate, policy, profile, toolchain, and artifact bindings. r[molten.audit_f02.candidate_binding]
- [x] [serial] Make the startup shell load and validate real artifacts before activation or destructive restart preparation. r[molten.audit_f02.real_evidence] r[molten.audit_f02.candidate_binding]
- [x] [serial] Isolate synthetic composition to explicit tests without normal-path or compatibility-wrapper fallback. r[molten.audit_f02.fixture_isolation]
- [x] [serial] Add adapter tests for valid artifacts, missing files, read errors, stale bindings, tampering, and synthetic inputs. r[molten.audit_f02.validation]
- [x] [serial] Assert unchanged active locks and prior lifecycle evidence after rejected startup. r[molten.audit_f02.candidate_binding] r[molten.audit_f02.validation]
- [x] [serial] Review startup receipt versioning and legacy replay behavior without upgrading synthetic evidence into real execution. r[molten.audit_f02.candidate_binding] r[molten.audit_f02.fixture_isolation]
- [x] [serial] Document operator evidence refresh, intentional fail-closed compatibility changes, ownership, and source-gate nonclaims. r[molten.audit_f02.real_evidence] r[molten.audit_f02.validation]
- [x] [serial] Repeat focused baseline and regression tests after edits with explicit executed-versus-static labels. r[molten.audit_f02.validation]
- [x] [serial] Run focused Octet error checks and Clippy with `-D warnings`, without policy allowances. r[molten.audit_f02.validation]
- [x] [serial] Run workspace tests across all targets and compatible features, relevant Nix source-gate checks, and required flake gates. r[molten.audit_f02.validation]
- [x] [serial] Run required Cairn validation, traceability, and package gates before completion review. Preserve blocked-check evidence. r[molten.audit_f02.validation]

## 2026-10-21 prerequisite review

The historical [executed baseline and producer review](evidence/2026-10-21-prerequisite-review.md) identified the missing source-to-binary binding; normal startup was unchanged at that review. The [2026-10-29 producer cutover](evidence/2026-10-29-producer-cutover.md) records the authorized implementation and distinguishes it from pending actual acceptance evidence. No fixture, configuration hash or unsigned matching reference is promoted into authoritative candidate evidence.

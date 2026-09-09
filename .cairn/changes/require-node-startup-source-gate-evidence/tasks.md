## Implementation tasks

All tasks remain proposed. This package grants no implementation permission.

- [ ] [serial] Run existing node lifecycle and runtime source-gate tests before core edits. Record baseline outcomes. r[molten.audit_f02.validation]
- [ ] [serial] Add the F02 normal-startup reproduction to repository tests, alongside a valid real-artifact control. r[molten.audit_f02.real_evidence] r[molten.audit_f02.validation]
- [ ] [serial] Review the real artifact producer, exact source-to-binary binding, and startup evidence input compatibility. r[molten.audit_f02.candidate_binding]
- [ ] [serial] Define pure admission over supplied candidate, policy, profile, toolchain, and artifact bindings. r[molten.audit_f02.candidate_binding]
- [ ] [serial] Make the startup shell load and validate real artifacts before activation or destructive restart preparation. r[molten.audit_f02.real_evidence] r[molten.audit_f02.candidate_binding]
- [ ] [serial] Isolate synthetic composition to explicit tests without normal-path or compatibility-wrapper fallback. r[molten.audit_f02.fixture_isolation]
- [ ] [serial] Add adapter tests for valid artifacts, missing files, read errors, stale bindings, tampering, and synthetic inputs. r[molten.audit_f02.validation]
- [ ] [serial] Assert unchanged active locks and prior lifecycle evidence after rejected startup. r[molten.audit_f02.candidate_binding] r[molten.audit_f02.validation]
- [ ] [serial] Review startup receipt versioning and legacy replay behavior without upgrading synthetic evidence into real execution. r[molten.audit_f02.candidate_binding] r[molten.audit_f02.fixture_isolation]
- [ ] [serial] Document operator evidence refresh, intentional fail-closed compatibility changes, ownership, and source-gate nonclaims. r[molten.audit_f02.real_evidence] r[molten.audit_f02.validation]
- [ ] [serial] Repeat focused baseline and regression tests after edits with explicit executed-versus-static labels. r[molten.audit_f02.validation]
- [ ] [serial] Run focused Octet error checks and Clippy with `-D warnings`, without policy allowances. r[molten.audit_f02.validation]
- [ ] [serial] Run workspace tests across all targets and compatible features, relevant Nix source-gate checks, and required flake gates. r[molten.audit_f02.validation]
- [ ] [serial] Run required Cairn validation, traceability, and package gates before completion review. Preserve blocked-check evidence. r[molten.audit_f02.validation]

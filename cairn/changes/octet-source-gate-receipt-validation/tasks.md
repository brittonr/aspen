## Phase 1: Downstream source-gate records

- [x] [serial] r[molten.octet_source_gate_receipt_validation.spec.content_validation] Define canonical `octet-source-gate-requirement-v1` with consumer, subject ref, required profile, source scope, current config/profile refs, required evidence classes, freshness rule, and checks.
- [x] [serial] r[molten.octet_source_gate_receipt_validation.spec.content_validation] Define canonical `octet-source-gate-validation-v1` with decision, requirement ref, Octet gate receipt ref, policy/status/summary/findings/object-corpus/fingerprint refs, counts, diagnostics, and checks.
- [x] [parallel] r[molten.octet_source_gate_receipt_validation.spec.content_validation] Classify source-gate requirements and validation receipts in the local ledger/catalog.
- [x] [parallel] r[molten.octet_source_gate_receipt_validation.spec.current_strict_scope] Define deterministic source-scope profiles for node startup, remote job admission, and upgrade planning consumers.

## Phase 2: Shared strict receipt validator

- [x] [serial] r[molten.octet_source_gate_receipt_validation.spec.content_validation] Parse referenced artifacts as canonical `octet-gate-receipt-v1` values; reject raw summaries, raw status files, process output, unknown schemas, and malformed Preserves.
- [x] [serial] r[molten.octet_source_gate_receipt_validation.spec.current_strict_scope] Require gate receipt decision `pass`, strict profile evidence, and checks that exclude warning-only/quarantine/advisory source evidence.
- [x] [serial] r[molten.octet_source_gate_receipt_validation.spec.current_strict_scope] Recompute/load current Octet config/profile/toolchain refs from workspace metadata, command scope, pass-through args, `Cargo.toml`, and `dylint.toml`; reject stale receipts.
- [x] [parallel] r[molten.octet_source_gate_receipt_validation.spec.tamper_denial] Verify command/status/summary/structured-findings/object-corpus/fingerprint refs are present, canonical, and match the gate receipt counts/checks.
- [x] [parallel] r[molten.octet_source_gate_receipt_validation.spec.current_strict_scope] Verify object-corpus and fingerprint evidence covers the required source-scope profile for the downstream consumer.
- [x] [parallel] r[molten.octet_source_gate_receipt_validation.spec.tamper_denial] Deny uncovered critical findings even if the receipt shape claims pass.

## Phase 3: Consumer enforcement

- [x] [serial] r[molten.octet_source_gate_receipt_validation.spec.consumer_binding] Require node startup to validate strict Octet gate receipts by content and bind pass validation refs before starting production adapters.
- [x] [serial] r[molten.octet_source_gate_receipt_validation.spec.consumer_binding] Require remote job admission to validate strict Octet gate receipts by content before executable-artifact readiness or target execution can be admitted.
- [x] [serial] r[molten.octet_source_gate_receipt_validation.spec.consumer_binding] Require upgrade planning to validate strict Octet gate receipts by content before name moves, migrations, irreversible tasks, or transcript-gated upgrade work can pass.
- [x] [parallel] r[molten.octet_source_gate_receipt_validation.spec.consumer_binding] Bind source-gate validation receipt refs and check labels into node startup, job admission, and upgrade receipts.
- [x] [parallel] r[molten.octet_source_gate_receipt_validation.spec.consumer_binding] Ensure consumers emit denial receipts and perform no adapter starts, admission grants, name moves, migrations, or job execution when source-gate validation fails.

## Phase 4: CLI/tests and diagnostics

- [x] [serial] r[molten.octet_source_gate_receipt_validation.spec.content_validation] Add a local validation command shape, e.g. `molten test octet source-gate validate --consumer ... --subject ... --gate-receipt ...`, for fixtures and operator diagnostics.
- [x] [parallel] r[molten.octet_source_gate_receipt_validation.spec.current_strict_scope] Add tests proving denied, warning-only, quarantine-profile, missing, and malformed Octet gate receipts fail downstream validation.
- [x] [parallel] r[molten.octet_source_gate_receipt_validation.spec.tamper_denial] Add tests proving stale config/profile refs and tampered object-corpus/fingerprint refs deny.
- [x] [parallel] r[molten.octet_source_gate_receipt_validation.spec.consumer_binding] Add consumer tests proving node startup, remote job admission, and upgrade planning deny before side effects when validation fails and bind validation refs when it passes.
- [x] [parallel] r[molten.octet_source_gate_receipt_validation.spec.tamper_denial] Preserve rejected Octet artifact refs and deterministic diagnostics in validation receipts without allowing diagnostics to stand in for pass evidence.

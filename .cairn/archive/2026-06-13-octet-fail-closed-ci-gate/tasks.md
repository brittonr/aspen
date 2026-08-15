## Phase 1: Canonical gate policy and receipt

- [x] [serial] r[molten.octet_fail_closed_ci.gate_policy] Define canonical `octet-gate-policy-v1` with profile, command, required artifacts, deny statuses, critical lint classes, quarantine policy, and checks.
- [x] [serial] r[molten.octet_fail_closed_ci.gate_receipt] Define canonical `octet-gate-receipt-v1` with decision, artifact refs, counts, critical/uncovered finding counts, baseline/review refs, diagnostics, and checks.
- [x] [serial] r[molten.octet_fail_closed_ci.ledger_classification] Classify Octet gate policies, receipts, status artifacts, summary artifacts, object corpus receipts, and fingerprint evidence in the local ledger/catalog.
- [x] [parallel] r[molten.octet_fail_closed_ci.artifact_ref_binding] Bind `command.txt`, `status.json`, `summary.txt`, structured findings, object corpus receipts, and fingerprint evidence by canonical content refs.

## Phase 2: Fail-closed evaluator

- [x] [serial] r[molten.octet_fail_closed_ci.status_semantics] Treat `warning-only` as a deny status for strict profiles even when the cargo-octet process exit code is `0`.
- [x] [serial] r[molten.octet_fail_closed_ci.missing_artifact_denial] Deny when required Octet artifacts are missing, malformed, stale, unsupported, or not bound to the expected command/config/profile/toolchain.
- [x] [serial] r[molten.octet_fail_closed_ci.critical_lint_denial] Deny unreviewed critical findings for panic, unwrap/expect, ambient time, unbounded loops, secret rendering, harness backdoors, authority typing, and critical resource-shape lints.
- [x] [parallel] r[molten.octet_fail_closed_ci.object_corpus_denial] Deny strict source-gate pass claims when configured critical paths lack object corpus/fingerprint evidence.

## Phase 3: CLI and CI integration

- [x] [serial] r[molten.octet_fail_closed_ci.cli_gate] Add a local command shape such as `molten test octet gate --artifacts target/octet --profile strict-ci --receipt-out ...`.
- [x] [serial] r[molten.octet_fail_closed_ci.ci_command_shape] Document and wire the strict CI sequence: Octet check, object corpus receipt, Octet gate receipt, harness gates, and Cairn strict validation.
- [x] [parallel] r[molten.octet_fail_closed_ci.release_admission_binding] Require strict Octet gate receipt refs for release, upgrade, node-runtime startup, and remote admission evidence paths once burn-down completes.
- [x] [parallel] r[molten.octet_fail_closed_ci.diagnostic_output] Preserve raw Octet status/summary/findings as diagnostic artifacts even when the gate denies.

## Phase 4: Tests

- [x] [serial] r[molten.octet_fail_closed_ci.warning_only_test] Add a fixture proving `status=warning-only` denies under strict profile.
- [x] [serial] r[molten.octet_fail_closed_ci.missing_status_test] Add fixtures for missing/malformed/stale `status.json`, missing object corpus receipt, and mismatched config/profile hash denial.
- [x] [serial] r[molten.octet_fail_closed_ci.critical_lint_test] Add fixtures proving unreviewed critical lint findings deny even under quarantine profile.
- [x] [parallel] r[molten.octet_fail_closed_ci.receipt_binding_test] Add tests that tampering with command/status/summary/object-corpus refs changes or denies the gate receipt.

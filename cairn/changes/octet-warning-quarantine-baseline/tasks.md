## Phase 1: Baseline and comparison DTOs

- [x] [serial] r[molten.octet_warning_quarantine.baseline_dto] Define canonical `octet-warning-baseline-v1` with scope, config/profile/toolchain refs, source snapshot ref, finding keys, expiry, allowed profiles, burn-down target, review refs, and checks.
- [x] [serial] r[molten.octet_warning_quarantine.receipt_dto] Define canonical `octet-baseline-receipt-v1` with decision, baseline ref, current run refs, new/removed/unchanged findings, unreviewed critical findings, expiry status, diagnostics, and checks.
- [x] [serial] r[molten.octet_warning_quarantine.finding_key] Implement stable finding keys over lint id, crate, normalized path, source span or function/object fingerprint, message category, config/profile hash, and surface classification.
- [x] [parallel] r[molten.octet_warning_quarantine.ledger_classification] Classify baselines and baseline receipts in the local ledger/catalog with clear quarantine status.

## Phase 2: Fail-closed comparison

- [x] [serial] r[molten.octet_warning_quarantine.no_new_findings] Deny quarantine profile when any new, moved, unkeyed, escalated, or malformed finding appears.
- [x] [serial] r[molten.octet_warning_quarantine.critical_review_required] Deny unreviewed critical findings even when they appear in the baseline unless a review receipt covers the exact finding and profile.
- [x] [serial] r[molten.octet_warning_quarantine.expiry_enforced] Deny expired baselines and require explicit baseline refresh receipts with burn-down evidence.
- [x] [parallel] r[molten.octet_warning_quarantine.shrink_target] Require each baseline refresh to meet a configured shrink target or attach review receipts explaining deferred findings.

## Phase 3: Gate integration

- [x] [serial] r[molten.octet_warning_quarantine.quarantine_profile] Add a `quarantine-ci` Octet gate profile that admits only covered existing findings and emits quarantine status in the gate receipt.
- [x] [serial] r[molten.octet_warning_quarantine.strict_profile_separation] Ensure strict release/admission/upgrade profiles do not treat quarantine receipts as source-gate pass evidence after the transition deadline.
- [x] [parallel] r[molten.octet_warning_quarantine.catalog_visibility] Render baseline counts, expiry, burn-down target, critical findings, and review refs in catalog views.
- [x] [parallel] r[molten.octet_warning_quarantine.review_manifest_binding] Bind suppressions and review manifests to Cairn/content refs rather than local comments or hidden config.

## Phase 4: Tests

- [x] [serial] r[molten.octet_warning_quarantine.new_warning_test] Add fixtures proving a single new warning denies quarantine profile.
- [x] [serial] r[molten.octet_warning_quarantine.removed_warning_test] Add fixtures proving removed warnings shrink the baseline and do not require review.
- [x] [serial] r[molten.octet_warning_quarantine.expired_baseline_test] Add fixtures proving expired baselines deny even when findings match.
- [x] [parallel] r[molten.octet_warning_quarantine.critical_baseline_test] Add fixtures proving critical findings require exact review receipt coverage and cannot be silently quarantined.

# Tasks: cluster-failure-repro-bundles

## Phase 1: Bundle core

- [x] [serial] r[molten.testing.cluster_failure_repro_bundles.bundle_schema] Define sealed cluster failure repro bundle payload, bundle, verification, and pass-gate denial values.
- [x] [parallel] r[molten.testing.cluster_failure_repro_bundles.privacy_and_nonpass] Add fail-closed diagnostics for tampering, stale refs, missing redaction evidence, private attachments without reveal, and diagnostic-only pass attempts.

## Phase 2: Export and verify shell

- [x] [serial] r[molten.testing.cluster_failure_repro_bundles.bundle_schema] Add export paths for cluster lifecycle, local multiprocess, VM unavailable, VM fault validation, reconciliation, and drift denials.
- [x] [parallel] r[molten.testing.cluster_failure_repro_bundles.privacy_and_nonpass] Add verify/unpack tests for valid diagnostic bundles and negative tamper/private/non-pass cases.

## Phase 3: Documentation and validation

- [x] [parallel] r[molten.testing.cluster_failure_repro_bundles.bundle_schema] Document how operators export, verify, and inspect cluster failure bundles.
- [x] [serial] r[molten.testing.cluster_failure_repro_bundles.privacy_and_nonpass] Run focused bundle tests, repro/redaction tests, and traceability coverage updates.

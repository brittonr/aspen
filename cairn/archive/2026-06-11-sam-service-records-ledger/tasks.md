## Phase 1: Canonical service DTOs

- [x] [serial] r[molten.sam_service_records_ledger.spec.canonical_records] Define canonical `service-manifest-v1` DTOs with explicit owner authority, target actor/artifact refs, dependencies, provided assertions, restart policy, policy refs, resource refs, effect refs, and checks.
- [x] [serial] r[molten.sam_service_records_ledger.spec.canonical_records] Define `service-demand-v1`, `service-status-v1`, `service-supervisor-v1`, `service-restart-policy-v1`, and `service-cleanup-receipt-v1` DTOs.
- [x] [serial] r[molten.sam_service_records_ledger.spec.canonical_records] Define `service-lifecycle-receipt-v1` with operation, decision, service id, manifest/status refs, authority/resource/effect refs, diagnostics, and checks.
- [x] [parallel] r[molten.sam_service_records_ledger.spec.canonical_records] Export schema constants and parser helpers for service records.

## Phase 2: Ledger/catalog/MCP visibility

- [x] [serial] r[molten.sam_service_records_ledger.spec.catalog_redaction] Classify service manifests, demand/status records, supervisor records, restart policies, lifecycle receipts, and cleanup receipts in the local ledger.
- [x] [serial] r[molten.sam_service_records_ledger.spec.catalog_redaction] Add catalog and read-only MCP visibility for service record refs, states, dependency ids, and receipt refs.
- [x] [parallel] r[molten.sam_service_records_ledger.spec.catalog_redaction] Render safe service summaries while preserving hidden-ref and secret-marker redaction by default.
- [x] [parallel] r[molten.sam_service_records_ledger.spec.catalog_redaction] Ensure rendered text cannot satisfy service pass-evidence checks.

## Phase 3: Validation and denial shape

- [x] [serial] r[molten.sam_service_records_ledger.spec.canonical_records] Reject unknown or malformed service records before runtime admission.
- [x] [parallel] r[molten.sam_service_records_ledger.spec.explicit_boundaries] Deny manifests that omit owner authority, policy refs, resource refs, or effect profile refs.
- [x] [parallel] r[molten.sam_service_records_ledger.spec.canonical_records] Enforce bounded dependency, provided assertion, diagnostics, and check vectors.
- [x] [parallel] r[molten.sam_service_records_ledger.spec.canonical_records] Add tests that identical service records produce stable canonical refs.

## Phase 4: Tests

- [x] [serial] r[molten.sam_service_records_ledger.spec.canonical_records] Add parse/render roundtrip tests for every service record type.
- [x] [serial] r[molten.sam_service_records_ledger.spec.catalog_redaction] Add ledger/catalog/MCP tests for service manifest and lifecycle receipt visibility.
- [x] [parallel] r[molten.sam_service_records_ledger.spec.catalog_redaction] Add tests proving secret/confidential service payload markers are redacted in summaries.
- [x] [parallel] r[molten.sam_service_records_ledger.spec.explicit_boundaries] Add Hegel properties for canonical ref stability and explicit-boundary denial.

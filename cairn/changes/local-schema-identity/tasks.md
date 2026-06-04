## Phase 1: Canonical identity model

- [x] [serial] r[molten.local_schema_identity.identity_dto] Define canonical `schema-identity-v1` artifacts with mode, schema ref, normalized shape ref, structural fingerprint, optional brand ref, metadata refs, policy refs, evidence refs, and checks.
- [x] [serial] r[molten.local_schema_identity.shape_normalization] Define the first bounded normalized Preserves shape representation and deterministic normalization rules.
- [x] [serial] r[molten.local_schema_identity.structural_fingerprint] Compute domain-separated structural fingerprints over normalized shapes, independent of names, docs, paths, aliases, and registry metadata.
- [x] [parallel] r[molten.local_schema_identity.no_unison_typechecker] Document that Unison unique/structural types are prior art only and Molten does not adopt Unison's typechecker, syntax, hash format, or UCM workflow.

## Phase 2: Aliases and compatibility decisions

- [x] [serial] r[molten.local_schema_identity.alias_dto] Define canonical `schema-alias-v1` artifacts with directional from/to refs, scope, policy refs, evidence refs, and checks proving aliases are not names.
- [x] [serial] r[molten.local_schema_identity.compatibility_dto] Define canonical `schema-compatibility-v1` decisions for exact artifact match, structural match, brand match, admitted alias, migration available, mismatch requiring migration, and policy denial.
- [x] [serial] r[molten.local_schema_identity.compatibility_rules] Implement fail-closed compatibility rules for structural, unique, branded-structural, alias, migration, and policy-denied cases.
- [x] [parallel] r[molten.local_schema_identity.receipts] Emit and parse canonical receipts for fingerprint, alias admission, compatibility checks, and storage-boundary decisions.

## Phase 3: Registry and typed-storage integration

- [x] [serial] r[molten.local_schema_identity.registry_indexes] Index schema identity and alias artifacts in the local artifact registry by schema ref, mode, fingerprint, brand ref, alias from/to refs, policy refs, and evidence refs.
- [x] [serial] r[molten.local_schema_identity.registry_queries] Add registry queries for structurally equivalent schema identities, admitted aliases, and nominal dependents.
- [x] [serial] r[molten.local_schema_identity.storage_loads] Integrate compatibility decisions into typed-storage load paths, preserving exact-ref fast path and fail-closed behavior when identity evidence is absent.
- [x] [serial] r[molten.local_schema_identity.storage_migrations] Preserve migration-recipe admission for incompatible schemas and include compatibility/migration evidence in migration receipts.

## Phase 4: CLI and inspection

- [x] [serial] r[molten.local_schema_identity.cli_identity] Add `molten test schema identity` or `fingerprint` to create schema identity artifacts from normalized shape files/refs and print full refs.
- [x] [serial] r[molten.local_schema_identity.cli_alias_compat] Add `molten test schema alias` and `compat` commands with explicit policy/evidence refs and receipt output.
- [x] [parallel] r[molten.local_schema_identity.cli_search] Add `molten test schema search-fingerprint` over the local artifact registry.
- [x] [parallel] r[molten.local_schema_identity.ledger_classification] Classify schema identity, alias, compatibility, and receipt artifacts in the local evidence ledger.

## Phase 5: Tests and properties

- [x] [serial] r[molten.local_schema_identity.structural_tests] Add tests proving equal structural shapes are compatible despite metadata/name differences.
- [x] [serial] r[molten.local_schema_identity.unique_alias_tests] Add tests proving equal-shape unique schemas are incompatible without exact ref or admitted directional alias.
- [x] [serial] r[molten.local_schema_identity.storage_tests] Add typed-storage tests for exact match, structural match, unique mismatch denial, alias admission, and migration-available evidence.
- [x] [parallel] r[molten.local_schema_identity.property_tests] Add Hegel properties for fingerprint determinism, alias directionality/scope safety, brand matching, and compatibility-result invariants.

# local schema identity Delta Spec

## ADDED Requirements

### Requirement: Define canonical `schema-identity-v1` artifacts with mode, schema ref, normalized shape ref, structural fingerprint, optional brand ref, metadata refs, policy refs, evidence refs, and checks
r[molten.local_schema_identity.identity_dto] Define canonical `schema-identity-v1` artifacts with mode, schema ref, normalized shape ref, structural fingerprint, optional brand ref, metadata refs, policy refs, evidence refs, and checks.

### Requirement: Define the first bounded normalized Preserves shape representation and deterministic normalization rules
r[molten.local_schema_identity.shape_normalization] Define the first bounded normalized Preserves shape representation and deterministic normalization rules.

### Requirement: Compute domain-separated structural fingerprints over normalized shapes, independent of names, docs, paths, aliases, and registry metadata
r[molten.local_schema_identity.structural_fingerprint] Compute domain-separated structural fingerprints over normalized shapes, independent of names, docs, paths, aliases, and registry metadata.

### Requirement: Document that Unison unique/structural types are prior art only and Molten does not adopt Unison's typechecker, syntax, hash format, or UCM workflow
r[molten.local_schema_identity.no_unison_typechecker] Document that Unison unique/structural types are prior art only and Molten does not adopt Unison's typechecker, syntax, hash format, or UCM workflow.

### Requirement: Define canonical `schema-alias-v1` artifacts with directional from/to refs, scope, policy refs, evidence refs, and checks proving aliases are not names
r[molten.local_schema_identity.alias_dto] Define canonical `schema-alias-v1` artifacts with directional from/to refs, scope, policy refs, evidence refs, and checks proving aliases are not names.

### Requirement: Define canonical `schema-compatibility-v1` decisions for exact artifact match, structural match, brand match, admitted alias, migration available, mismatch requiring migration, and policy denial
r[molten.local_schema_identity.compatibility_dto] Define canonical `schema-compatibility-v1` decisions for exact artifact match, structural match, brand match, admitted alias, migration available, mismatch requiring migration, and policy denial.

### Requirement: Implement fail-closed compatibility rules for structural, unique, branded-structural, alias, migration, and policy-denied cases
r[molten.local_schema_identity.compatibility_rules] Implement fail-closed compatibility rules for structural, unique, branded-structural, alias, migration, and policy-denied cases.

### Requirement: Emit and parse canonical receipts for fingerprint, alias admission, compatibility checks, and storage-boundary decisions
r[molten.local_schema_identity.receipts] Emit and parse canonical receipts for fingerprint, alias admission, compatibility checks, and storage-boundary decisions.

### Requirement: Index schema identity and alias artifacts in the local artifact registry by schema ref, mode, fingerprint, brand ref, alias from/to refs, policy refs, and evidence refs
r[molten.local_schema_identity.registry_indexes] Index schema identity and alias artifacts in the local artifact registry by schema ref, mode, fingerprint, brand ref, alias from/to refs, policy refs, and evidence refs.

### Requirement: Add registry queries for structurally equivalent schema identities, admitted aliases, and nominal dependents
r[molten.local_schema_identity.registry_queries] Add registry queries for structurally equivalent schema identities, admitted aliases, and nominal dependents.

### Requirement: Integrate compatibility decisions into typed-storage load paths, preserving exact-ref fast path and fail-closed behavior when identity evidence is absent
r[molten.local_schema_identity.storage_loads] Integrate compatibility decisions into typed-storage load paths, preserving exact-ref fast path and fail-closed behavior when identity evidence is absent.

### Requirement: Preserve migration-recipe admission for incompatible schemas and include compatibility/migration evidence in migration receipts
r[molten.local_schema_identity.storage_migrations] Preserve migration-recipe admission for incompatible schemas and include compatibility/migration evidence in migration receipts.

### Requirement: Add `molten test schema identity` or `fingerprint` to create schema identity artifacts from normalized shape files/refs and print full refs
r[molten.local_schema_identity.cli_identity] Add `molten test schema identity` or `fingerprint` to create schema identity artifacts from normalized shape files/refs and print full refs.

### Requirement: Add `molten test schema alias` and `compat` commands with explicit policy/evidence refs and receipt output
r[molten.local_schema_identity.cli_alias_compat] Add `molten test schema alias` and `compat` commands with explicit policy/evidence refs and receipt output.

### Requirement: Add `molten test schema search-fingerprint` over the local artifact registry
r[molten.local_schema_identity.cli_search] Add `molten test schema search-fingerprint` over the local artifact registry.

### Requirement: Classify schema identity, alias, compatibility, and receipt artifacts in the local evidence ledger
r[molten.local_schema_identity.ledger_classification] Classify schema identity, alias, compatibility, and receipt artifacts in the local evidence ledger.

### Requirement: Add tests proving equal structural shapes are compatible despite metadata/name differences
r[molten.local_schema_identity.structural_tests] Add tests proving equal structural shapes are compatible despite metadata/name differences.

### Requirement: Add tests proving equal-shape unique schemas are incompatible without exact ref or admitted directional alias
r[molten.local_schema_identity.unique_alias_tests] Add tests proving equal-shape unique schemas are incompatible without exact ref or admitted directional alias.

### Requirement: Add typed-storage tests for exact match, structural match, unique mismatch denial, alias admission, and migration-available evidence
r[molten.local_schema_identity.storage_tests] Add typed-storage tests for exact match, structural match, unique mismatch denial, alias admission, and migration-available evidence.

### Requirement: Add Hegel properties for fingerprint determinism, alias directionality/scope safety, brand matching, and compatibility-result invariants
r[molten.local_schema_identity.property_tests] Add Hegel properties for fingerprint determinism, alias directionality/scope safety, brand matching, and compatibility-result invariants.


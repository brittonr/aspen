# Local Schema Identity Specification

## Purpose

Defines the `local-schema-identity` capability.

## Requirements

### Requirement: System MUST Define canonical `schema-identity-v1` artifacts with mode, schema ref, normalized shape ref, structural fingerprint, optional brand ref, metadata refs, policy refs, evidence refs, and checks
r[molten.local_schema_identity.identity_dto] The system MUST Define canonical `schema-identity-v1` artifacts with mode, schema ref, normalized shape ref, structural fingerprint, optional brand ref, metadata refs, policy refs, evidence refs, and checks.

### Requirement: System MUST Define the first bounded normalized Preserves shape representation and deterministic normalization rules
r[molten.local_schema_identity.shape_normalization] The system MUST Define the first bounded normalized Preserves shape representation and deterministic normalization rules.

### Requirement: System MUST Compute domain-separated structural fingerprints over normalized shapes, independent of names, docs, paths, aliases, and registry metadata
r[molten.local_schema_identity.structural_fingerprint] The system MUST Compute domain-separated structural fingerprints over normalized shapes, independent of names, docs, paths, aliases, and registry metadata.

### Requirement: System MUST Document that Unison unique/structural types are prior art only and Molten does not adopt Unison's typechecker, syntax, hash format, or UCM workflow
r[molten.local_schema_identity.no_unison_typechecker] The system MUST Document that Unison unique/structural types are prior art only and Molten does not adopt Unison's typechecker, syntax, hash format, or UCM workflow.

### Requirement: System MUST Define canonical `schema-alias-v1` artifacts with directional from/to refs, scope, policy refs, evidence refs, and checks proving aliases are not names
r[molten.local_schema_identity.alias_dto] The system MUST Define canonical `schema-alias-v1` artifacts with directional from/to refs, scope, policy refs, evidence refs, and checks proving aliases are not names.

### Requirement: System MUST Define canonical `schema-compatibility-v1` decisions for exact artifact match, structural match, brand match, admitted alias, migration available, mismatch requiring migration, and policy denial
r[molten.local_schema_identity.compatibility_dto] The system MUST Define canonical `schema-compatibility-v1` decisions for exact artifact match, structural match, brand match, admitted alias, migration available, mismatch requiring migration, and policy denial.

### Requirement: System MUST Implement fail-closed compatibility rules for structural, unique, branded-structural, alias, migration, and policy-denied cases
r[molten.local_schema_identity.compatibility_rules] The system MUST Implement fail-closed compatibility rules for structural, unique, branded-structural, alias, migration, and policy-denied cases.

### Requirement: System MUST Emit and parse canonical receipts for fingerprint, alias admission, compatibility checks, and storage-boundary decisions
r[molten.local_schema_identity.receipts] The system MUST Emit and parse canonical receipts for fingerprint, alias admission, compatibility checks, and storage-boundary decisions.

### Requirement: System MUST Index schema identity and alias artifacts in the local artifact registry by schema ref, mode, fingerprint, brand ref, alias from/to refs, policy refs, and evidence refs
r[molten.local_schema_identity.registry_indexes] The system MUST Index schema identity and alias artifacts in the local artifact registry by schema ref, mode, fingerprint, brand ref, alias from/to refs, policy refs, and evidence refs.

### Requirement: System MUST Add registry queries for structurally equivalent schema identities, admitted aliases, and nominal dependents
r[molten.local_schema_identity.registry_queries] The system MUST Add registry queries for structurally equivalent schema identities, admitted aliases, and nominal dependents.

### Requirement: System MUST Integrate compatibility decisions into typed-storage load paths, preserving exact-ref fast path and fail-closed behavior when identity evidence is absent
r[molten.local_schema_identity.storage_loads] The system MUST Integrate compatibility decisions into typed-storage load paths, preserving exact-ref fast path and fail-closed behavior when identity evidence is absent.

### Requirement: System MUST Preserve migration-recipe admission for incompatible schemas and include compatibility/migration evidence in migration receipts
r[molten.local_schema_identity.storage_migrations] The system MUST Preserve migration-recipe admission for incompatible schemas and include compatibility/migration evidence in migration receipts.

### Requirement: System MUST Add `molten test schema identity` or `fingerprint` to create schema identity artifacts from normalized shape files/refs and print full refs
r[molten.local_schema_identity.cli_identity] The system MUST Add `molten test schema identity` or `fingerprint` to create schema identity artifacts from normalized shape files/refs and print full refs.

### Requirement: System MUST Add `molten test schema alias` and `compat` commands with explicit policy/evidence refs and receipt output
r[molten.local_schema_identity.cli_alias_compat] The system MUST Add `molten test schema alias` and `compat` commands with explicit policy/evidence refs and receipt output.

### Requirement: System MUST Add `molten test schema search-fingerprint` over the local artifact registry
r[molten.local_schema_identity.cli_search] The system MUST Add `molten test schema search-fingerprint` over the local artifact registry.

### Requirement: System MUST Classify schema identity, alias, compatibility, and receipt artifacts in the local evidence ledger
r[molten.local_schema_identity.ledger_classification] The system MUST Classify schema identity, alias, compatibility, and receipt artifacts in the local evidence ledger.

### Requirement: System MUST Add tests proving equal structural shapes are compatible despite metadata/name differences
r[molten.local_schema_identity.structural_tests] The system MUST Add tests proving equal structural shapes are compatible despite metadata/name differences.

### Requirement: System MUST Add tests proving equal-shape unique schemas are incompatible without exact ref or admitted directional alias
r[molten.local_schema_identity.unique_alias_tests] The system MUST Add tests proving equal-shape unique schemas are incompatible without exact ref or admitted directional alias.

### Requirement: System MUST Add typed-storage tests for exact match, structural match, unique mismatch denial, alias admission, and migration-available evidence
r[molten.local_schema_identity.storage_tests] The system MUST Add typed-storage tests for exact match, structural match, unique mismatch denial, alias admission, and migration-available evidence.

### Requirement: System MUST Add Hegel properties for fingerprint determinism, alias directionality/scope safety, brand matching, and compatibility-result invariants
r[molten.local_schema_identity.property_tests] The system MUST Add Hegel properties for fingerprint determinism, alias directionality/scope safety, brand matching, and compatibility-result invariants.

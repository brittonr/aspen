# Local Schema Identity Specification

## Purpose

Defines Molten's local schema identity capability: structural and unique schema identity, normalized fingerprints, explicit aliases and migrations, trust-boundary compatibility receipts, registry search, typed-storage integration, and protocol/effect/policy-contract schema compatibility helpers.

## Requirements

### Requirement: Schema identity modes MUST be explicit and canonical
r[molten.schema_identity.model] Molten MUST define schema artifact identity modes for structural, unique, and branded-structural schemas.
r[molten.local_schema_identity.identity_dto] Molten MUST define canonical `schema-identity-v1` artifacts with mode, schema ref, normalized shape ref, structural fingerprint, optional brand ref, metadata refs, policy refs, evidence refs, and checks.

#### Scenario: Mode controls compatibility
- GIVEN two schema identities with equal shape
- WHEN their modes are unique
- THEN they are not compatible unless exact ref, alias, or migration evidence admits the relationship.

### Requirement: Normalized shapes and structural fingerprints MUST be deterministic and domain-separated
r[molten.schema_identity.structural_fingerprint] Molten MUST compute domain-separated structural fingerprints over normalized schema shapes.
r[molten.local_schema_identity.shape_normalization] Molten MUST define the first bounded normalized Preserves shape representation and deterministic normalization rules.
r[molten.local_schema_identity.structural_fingerprint] Molten MUST compute domain-separated structural fingerprints over normalized shapes, independent of names, docs, paths, aliases, and registry metadata.

#### Scenario: Field order normalizes
- GIVEN two record shapes with the same fields in different order
- WHEN Molten computes structural fingerprints
- THEN the fingerprints match.

### Requirement: Unique schema identity MUST be artifact identity plus admitted alias metadata
r[molten.schema_identity.unique_ids] Molten MUST treat unique schema identity as schema artifact id plus admitted alias metadata, not mutable names.
r[molten.local_schema_identity.alias_dto] Molten MUST define canonical `schema-alias-v1` artifacts with directional from/to refs, scope, policy refs, evidence refs, and checks proving aliases are not names.

#### Scenario: Name cannot alias unique schema
- GIVEN two unique schemas with the same shape and different artifact refs
- WHEN no explicit alias or migration is present
- THEN compatibility is denied.

### Requirement: Unison type identity MUST remain non-normative prior art
r[molten.schema_identity.no_unison_typechecker] Molten MUST document that Unison unique and structural type ideas are prior art only and MUST NOT claim Unison typechecker or hash-format compatibility.
r[molten.local_schema_identity.no_unison_typechecker] Molten MUST document that Unison unique/structural types are prior art only and Molten does not adopt Unison's typechecker, syntax, hash format, or UCM workflow.

#### Scenario: Identity checks record boundary
- GIVEN a schema identity record
- WHEN Molten renders it
- THEN checks make clear names are not identity and content addressing is not trust.

### Requirement: Compatibility results MUST be structured and fail closed
r[molten.schema_identity.compatibility_result] Molten MUST define structured compatibility results for exact artifact match, structural match, brand match, alias, migration available, mismatch, and policy denial.
r[molten.local_schema_identity.compatibility_dto] Molten MUST define canonical `schema-compatibility-v1` decisions for exact artifact match, structural match, brand match, admitted alias, migration available, mismatch requiring migration, and policy denial.
r[molten.local_schema_identity.compatibility_rules] Molten MUST implement fail-closed compatibility rules for structural, unique, branded-structural, alias, migration, and policy-denied cases.

#### Scenario: Policy denial wins
- GIVEN otherwise compatible schemas and a policy denial
- WHEN Molten computes compatibility
- THEN the decision is denied-by-policy.

### Requirement: Policy gates MUST admit aliases and compatibility overrides explicitly
r[molten.schema_identity.policy_gate] Molten MUST gate schema alias and compatibility override decisions through explicit policy refs and evidence refs.

#### Scenario: Alias requires policy refs
- GIVEN a schema alias
- WHEN Molten renders it
- THEN the alias binds policy and evidence refs and records policy-admission-required.

### Requirement: Compatibility decisions and boundary uses MUST be receipted
r[molten.schema_identity.receipts] Molten MUST emit receipts for schema compatibility decisions at trust boundaries.
r[molten.local_schema_identity.receipts] Molten MUST emit and parse canonical receipts for fingerprint, alias admission, compatibility checks, and storage-boundary decisions.

#### Scenario: Receipt binds compatibility ref
- GIVEN a compatibility decision
- WHEN Molten emits a trust-boundary receipt
- THEN the receipt binds the compatibility ref, expected schema ref, actual schema ref, and pass or deny decision.

### Requirement: Registry search MUST distinguish structural equivalents, aliases, and nominal dependents
r[molten.schema_identity.semantic_search] Molten MUST support registry queries for structurally equivalent schemas and nominal dependents.
r[molten.local_schema_identity.registry_indexes] Molten MUST index schema identity and alias artifacts in the local artifact registry by schema ref, mode, fingerprint, brand ref, alias from/to refs, policy refs, and evidence refs.
r[molten.local_schema_identity.registry_queries] Molten MUST add registry queries for structurally equivalent schema identities, admitted aliases, and nominal dependents.

#### Scenario: Fingerprint search finds identity artifacts
- GIVEN schema identity artifacts in the registry
- WHEN Molten searches by structural fingerprint
- THEN matching identities are returned subject to registry visibility and dependency rules.

### Requirement: Typed storage MUST use schema identity for writes, loads, and migrations
r[molten.schema_identity.storage_integration] Molten MUST use schema identity decisions in typed-storage writes, loads, and migrations.
r[molten.local_schema_identity.storage_loads] Molten MUST integrate compatibility decisions into typed-storage load paths, preserving exact-ref fast path and fail-closed behavior when identity evidence is absent.
r[molten.local_schema_identity.storage_migrations] Molten MUST preserve migration-recipe admission for incompatible schemas and include compatibility/migration evidence in migration receipts.

#### Scenario: Storage load honors compatibility decision
- GIVEN a stored value schema and expected schema
- WHEN a compatibility decision admits alias or migration
- THEN typed storage may load or migrate the value; otherwise it denies before returning data.

### Requirement: Protocol, effect, and policy contract schemas MUST cite schema identity decisions
r[molten.schema_identity.choreography_payloads] Molten MUST use schema identity decisions in choreography payload registries and protocol upgrade checks.
r[molten.schema_identity.effect_schemas] Molten MUST use schema identity decisions for effect-request and effect-response schemas.
r[molten.schema_identity.policy_contract_schemas] Molten MUST use schema identity decisions for Nickel and Steel contract input/output schemas.

#### Scenario: Protocol payload alias is explicit
- GIVEN actual and expected protocol payload schema refs
- WHEN compatibility is admitted for protocol scope
- THEN the protocol payload boundary may treat the schemas as compatible evidence-only.

#### Scenario: Effect and policy schema aliases are explicit
- GIVEN actual and expected effect or policy contract schema refs
- WHEN compatibility is admitted for the corresponding scope
- THEN the boundary can cite compatibility evidence while policy denial still wins.

### Requirement: Schema identity CLI and ledger classification MUST expose full refs
r[molten.local_schema_identity.cli_identity] Molten MUST add `molten test schema identity` or `fingerprint` to create schema identity artifacts from normalized shape files/refs and print full refs.
r[molten.local_schema_identity.cli_alias_compat] Molten MUST add `molten test schema alias` and `compat` commands with explicit policy/evidence refs and receipt output.
r[molten.local_schema_identity.cli_search] Molten MUST add `molten test schema search-fingerprint` over the local artifact registry.
r[molten.local_schema_identity.ledger_classification] Molten MUST classify schema identity, alias, compatibility, and receipt artifacts in the local evidence ledger.

#### Scenario: CLI identity command prints canonical refs
- GIVEN a normalized schema shape file or ref
- WHEN the schema identity CLI command runs
- THEN it emits a canonical identity artifact and prints full schema identity and receipt refs.

### Requirement: Schema identity tests MUST cover structural, unique, migration, storage, and property invariants
r[molten.schema_identity.structural_tests] Molten MUST add tests showing structural schemas with equal normalized shapes are compatible.
r[molten.schema_identity.unique_tests] Molten MUST add tests showing unique schemas with equal shapes are incompatible without explicit alias or migration.
r[molten.schema_identity.migration_tests] Molten MUST add tests showing mismatches can be admitted only through migration recipe artifacts.
r[molten.schema_identity.property_tests] Molten MUST add Hegel property tests for fingerprint determinism, alias safety, and compatibility-result invariants.
r[molten.local_schema_identity.structural_tests] Molten MUST add tests proving equal structural shapes are compatible despite metadata/name differences.
r[molten.local_schema_identity.unique_alias_tests] Molten MUST add tests proving equal-shape unique schemas are incompatible without exact ref or admitted directional alias.
r[molten.local_schema_identity.storage_tests] Molten MUST add typed-storage tests for exact match, structural match, unique mismatch denial, alias admission, and migration-available evidence.
r[molten.local_schema_identity.property_tests] Molten MUST add Hegel properties for fingerprint determinism, alias directionality/scope safety, brand matching, and compatibility-result invariants.

#### Scenario: Generated structural shapes are deterministic
- GIVEN generated bounded schema shapes
- WHEN fingerprints and compatibility are computed repeatedly
- THEN refs and decisions are stable.

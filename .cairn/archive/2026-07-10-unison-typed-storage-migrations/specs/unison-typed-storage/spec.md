# Unison Typed Storage Delta: Migration-Gated Durable Values

## ADDED Requirements

### Requirement: Stored values bind type and artifact identity
r[molten.typed_storage.value_type_artifact_bindings] Molten MUST store typed values with canonical records that bind value ref, schema ref, schema identity mode, producing artifact ref, optional intended consumer refs, storage handler profile, policy refs, capability refs, retention refs, provenance refs, and evidence refs.

#### Scenario: Typed value read has exact bindings
- GIVEN a stored value was produced by artifact A under schema S
- WHEN Molten reads the value for a consumer artifact C
- THEN the read decision can inspect S, A, C, policy refs, capability refs, retention refs, and evidence refs.

#### Scenario: Missing schema binding denies typed read
- GIVEN a stored value lacks a schema ref or schema identity mode
- WHEN a caller requests a typed read
- THEN Molten denies the typed read before returning the value as satisfying a schema contract.

### Requirement: Migration recipes are gated artifacts
r[molten.typed_storage.migration_recipe_gate] Molten MUST gate migration recipes as artifacts that bind source schema, target schema, executable recipe artifact, effect manifest, handler profile, policy refs, provenance refs, source-gate refs, test evidence, rollback metadata, and lineage refs.

#### Scenario: Admitted migration writes lineage
- GIVEN a migration recipe from schema S1 to S2 has passing policy, provenance, source-gate, handler profile, and test evidence
- WHEN Molten applies the migration
- THEN it emits preflight, execution, output-validation, and lineage receipts.

#### Scenario: Stale migration recipe denies
- GIVEN a migration recipe was admitted for an older source schema ref
- WHEN a stored value with a different source schema requests migration
- THEN Molten denies before mutation
- AND diagnostics name the stale source-schema binding.

### Requirement: Compatibility receipts are required for changed expectations
r[molten.typed_storage.compatibility_receipts_required] Molten MUST require schema compatibility, alias, or migration receipts before reading a stored value under a different expected schema or artifact contract.

#### Scenario: Structural compatibility permits read with evidence
- GIVEN an expected schema is structurally compatible with the stored schema under admitted policy
- WHEN Molten evaluates the read
- THEN it may permit the read
- AND the read receipt binds the compatibility evidence.

#### Scenario: Unique schema mismatch denies without migration
- GIVEN a stored value has unique schema S1 and a caller expects unique schema S2
- WHEN no admitted alias or migration receipt connects S1 to S2
- THEN Molten denies the read as satisfying S2.

### Requirement: Stored values do not serialize executable authority
r[molten.typed_storage.no_function_serialization] Molten MUST reject arbitrary serialized functions, closures, mutable names, or raw decoder claims as storage identity or migration authority.

#### Scenario: Decoder artifact ref is admitted separately
- GIVEN a stored value references a decoder artifact ref with provenance, effect manifest, and policy evidence
- WHEN Molten evaluates a read path that needs the decoder
- THEN the decoder must pass normal artifact and execution admission.

#### Scenario: Serialized function identity denies
- GIVEN a stored value embeds a serialized function or closure as the decoder or migration authority
- WHEN Molten validates the storage record
- THEN typed storage admission denies
- AND no migration or typed read proceeds from that function payload.

### Requirement: Typed storage migration validation covers positive and negative paths
r[molten.typed_storage.migration_validation] Molten MUST include positive and negative fixtures for compatible reads, admitted migrations, missing schema refs, wrong unique identity, stale migration recipes, unadmitted decoders, and function serialization denial.

#### Scenario: Admitted migration fixture passes
- GIVEN a stored value, source schema, target schema, migration recipe, and passing evidence refs
- WHEN validation runs
- THEN Molten emits passing migration and lineage receipts.

#### Scenario: Wrong unique identity fixture denies
- GIVEN a stored value with unique schema S1 is read as unique schema S2 without alias or migration evidence
- WHEN validation runs
- THEN Molten denies
- AND records the unique identity mismatch.
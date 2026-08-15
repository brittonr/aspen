# Schema Identity Delta

### Requirement: Schema identity modes are explicit
r[molten.schema_identity.model] Molten MUST define schema artifact identity modes for structural, unique, and branded-structural schemas.

#### Scenario: Mode controls compatibility
- GIVEN two schema identities with equal shape
- WHEN their modes are unique
- THEN they are not compatible unless exact ref, alias, or migration evidence admits the relationship.

### Requirement: Structural fingerprints are canonical
r[molten.schema_identity.structural_fingerprint] Molten MUST compute domain-separated structural fingerprints over normalized schema shapes.

#### Scenario: Field order normalizes
- GIVEN two record shapes with the same fields in different order
- WHEN Molten computes structural fingerprints
- THEN the fingerprints match.

### Requirement: Unique ids are artifact identities
r[molten.schema_identity.unique_ids] Molten MUST treat unique schema identity as schema artifact id plus admitted alias metadata, not mutable names.

#### Scenario: Name cannot alias unique schema
- GIVEN two unique schemas with the same shape and different artifact refs
- WHEN no explicit alias or migration is present
- THEN compatibility is denied.

### Requirement: Unison typechecking is not adopted
r[molten.schema_identity.no_unison_typechecker] Molten MUST document that Unison unique and structural type ideas are prior art only and MUST NOT claim Unison typechecker or hash-format compatibility.

#### Scenario: Identity checks record boundary
- GIVEN a schema identity record
- WHEN Molten renders it
- THEN checks make clear names are not identity and content addressing is not trust.

### Requirement: Compatibility results are structured
r[molten.schema_identity.compatibility_result] Molten MUST define structured compatibility results for exact artifact match, structural match, brand match, alias, migration available, mismatch, and policy denial.

#### Scenario: Policy denial wins
- GIVEN otherwise compatible schemas and a policy denial
- WHEN Molten computes compatibility
- THEN the decision is denied-by-policy.

### Requirement: Policy gates admit overrides
r[molten.schema_identity.policy_gate] Molten MUST gate schema alias and compatibility override decisions through explicit policy refs and evidence refs.

#### Scenario: Alias requires policy refs
- GIVEN a schema alias
- WHEN Molten renders it
- THEN the alias binds policy and evidence refs and records policy-admission-required.

### Requirement: Compatibility receipts are emitted
r[molten.schema_identity.receipts] Molten MUST emit receipts for schema compatibility decisions at trust boundaries.

#### Scenario: Receipt binds compatibility ref
- GIVEN a compatibility decision
- WHEN Molten emits a trust-boundary receipt
- THEN the receipt binds the compatibility ref, expected schema ref, actual schema ref, and pass or deny decision.

### Requirement: Semantic schema search is supported
r[molten.schema_identity.semantic_search] Molten MUST support registry queries for structurally equivalent schemas and nominal dependents.

#### Scenario: Fingerprint search finds identity artifacts
- GIVEN schema identity artifacts in the registry
- WHEN Molten searches by structural fingerprint
- THEN matching identities are returned subject to registry visibility and dependency rules.

### Requirement: Typed storage uses schema identity
r[molten.schema_identity.storage_integration] Molten MUST use schema identity decisions in typed-storage writes, loads, and migrations.

#### Scenario: Storage load honors compatibility decision
- GIVEN a stored value schema and expected schema
- WHEN a compatibility decision admits alias or migration
- THEN typed storage may load or migrate the value; otherwise it denies before returning data.

### Requirement: Choreography payload schemas use identity
r[molten.schema_identity.choreography_payloads] Molten MUST use schema identity decisions in choreography payload registries and protocol upgrade checks.

#### Scenario: Protocol payload alias is explicit
- GIVEN actual and expected protocol payload schema refs
- WHEN compatibility is admitted for protocol scope
- THEN the protocol payload boundary may treat the schemas as compatible evidence-only.

### Requirement: Effect schemas use identity
r[molten.schema_identity.effect_schemas] Molten MUST use schema identity decisions for effect-request and effect-response schemas.

#### Scenario: Effect schema alias is explicit
- GIVEN actual and expected effect request schema refs
- WHEN compatibility is admitted for effect scope
- THEN effect binding can cite the compatibility evidence without treating names as identity.

### Requirement: Policy contract schemas use identity
r[molten.schema_identity.policy_contract_schemas] Molten MUST use schema identity decisions for Nickel and Steel contract input/output schemas.

#### Scenario: Policy contract schema alias is explicit
- GIVEN actual and expected policy contract schema refs
- WHEN compatibility is admitted for policy scope
- THEN the policy boundary can cite compatibility evidence while policy denial still wins.

### Requirement: Structural tests cover normalized shape compatibility
r[molten.schema_identity.structural_tests] Molten MUST add tests showing structural schemas with equal normalized shapes are compatible.

#### Scenario: Structural schemas match
- GIVEN two structural schema identities with equal fingerprints
- WHEN compatibility is computed
- THEN the decision is structural-match.

### Requirement: Unique tests cover nominal mismatch
r[molten.schema_identity.unique_tests] Molten MUST add tests showing unique schemas with equal shapes are incompatible without explicit alias or migration.

#### Scenario: Unique schemas need evidence
- GIVEN two unique schemas with equal shapes and no alias or migration
- WHEN compatibility is computed
- THEN the decision is mismatch-requires-migration.

### Requirement: Migration tests cover explicit admission
r[molten.schema_identity.migration_tests] Molten MUST add tests showing mismatches can be admitted only through migration recipe artifacts.

#### Scenario: Migration admits mismatch
- GIVEN incompatible schema refs and a migration recipe ref
- WHEN compatibility is computed
- THEN the decision is migration-available.

### Requirement: Property tests cover invariants
r[molten.schema_identity.property_tests] Molten MUST add Hegel property tests for fingerprint determinism, alias safety, and compatibility-result invariants.

#### Scenario: Generated structural shapes are deterministic
- GIVEN generated bounded schema shapes
- WHEN fingerprints and compatibility are computed repeatedly
- THEN refs and decisions are stable.

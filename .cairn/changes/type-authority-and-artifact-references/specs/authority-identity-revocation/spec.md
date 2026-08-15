# Authority Identity Revocation Specification Delta

## ADDED Requirements

### Requirement: Authority-bearing references are nominal Rust types

r[molten.authority.nominal_references.types] Molten MUST represent statically known principal, node, actor, service, session, context, delegation, revocation, key, policy, resource, evidence, artifact, operation, and receipt references with distinct private Rust types after admission.

#### Scenario: Same-domain reference is accepted

r[molten.authority.nominal_references.types.same_domain]
- GIVEN a valid typed session reference
- WHEN a pure core API requires a session reference
- THEN the API MUST accept it without another expected-domain string.

#### Scenario: Cross-domain reference does not compile

r[molten.authority.nominal_references.compile_time]
- GIVEN session and authority-context references use distinct marker domains
- WHEN source passes an authority-context reference to an API that requires a session reference
- THEN the source MUST fail compilation with a type mismatch.

#### Scenario: Malformed reference fails before core use

r[molten.authority.nominal_references.validation]
- GIVEN untrusted reference text is empty, oversized, malformed, uses an unsupported domain, or has invalid canonical spelling
- WHEN reference construction runs
- THEN construction MUST fail with deterministic diagnostics
- AND the invalid value MUST NOT enter the admitted core.

### Requirement: Reference migration follows a reviewed inventory

r[molten.authority.nominal_references.inventory] Molten MUST maintain a bounded migration inventory that classifies selected raw values as domain core, wire compatibility, display metadata, external protocol, or deferred.

#### Scenario: Reference inventory classifies every selected raw value

r[molten.authority.nominal_references.inventory.complete]
- GIVEN a selected authority or artifact module contains raw reference fields or signatures
- WHEN migration planning runs
- THEN every selected value MUST have one reviewed classification and domain owner
- AND unclassified values MUST block cohort completion.

### Requirement: Preserves wire records admit typed references

r[molten.authority.nominal_references.wire_boundary] Molten MUST preserve canonical Preserves records and MUST convert their reference fields into typed core values before authority, execution, artifact, retention, or replay decisions.

#### Scenario: Preserves record admits typed core refs

r[molten.authority.nominal_references.wire_boundary.valid]
- GIVEN a supported Preserves record has valid domain-tagged references
- WHEN wire admission runs
- THEN Molten MUST construct the matching typed core references without changing canonical bytes.

#### Scenario: Wire domain mismatch fails

r[molten.authority.nominal_references.wire_boundary.invalid]
- GIVEN a policy ref appears in an evidence field, a delegation ref appears in a revocation field, or another declared role is mismatched
- WHEN wire admission runs
- THEN Molten MUST fail closed with a deterministic role diagnostic.

### Requirement: Authority admission keeps exact reference roles

r[molten.authority.nominal_references.authority_core] Authority and capability admission MUST use exact typed holder, session, context, delegation, revocation, key, policy, resource, and evidence references.

#### Scenario: Typed reference does not grant authority

r[molten.authority.nominal_references.authority_tests]
- GIVEN all supplied references have valid syntax and matching Rust domains but current scope, caveat, expiry, revocation, key, policy, or resource evidence denies the action
- WHEN authority admission runs
- THEN the decision MUST remain `deny`
- AND typed reference possession MUST NOT bypass normal admission.

### Requirement: Execution and artifact references remain distinct

r[molten.authority.nominal_references.execution_core] Effect, handler, node-control, session, operation, and resource APIs MUST retain exact typed reference roles through pure admission decisions.

r[molten.authority.nominal_references.artifact_core] Artifact binding, provenance, evidence, operation, and receipt APIs MUST retain exact typed reference roles without moving binding, effect, or authority semantics into generic reference constructors.

#### Scenario: Artifact and evidence linkage stay distinct

r[molten.authority.nominal_references.artifact_core.distinct]
- GIVEN artifact and evidence references contain equal digest bytes under different domains
- WHEN linkage is evaluated
- THEN Molten MUST preserve both roles and MUST NOT treat either value as the other.

### Requirement: Evidence migration preserves canonical history

r[molten.authority.nominal_references.evidence_core] Retention, replay, cache, and historical receipt paths MUST preserve evidence-only roles and canonical identities during nominal reference migration.

#### Scenario: Canonical Preserves bytes remain stable

r[molten.authority.nominal_references.compatibility]
- GIVEN an accepted authority, artifact, or receipt fixture is encoded before and after migration
- WHEN canonical Preserves bytes and refs are compared
- THEN they MUST remain equal unless a separate versioned schema change approves the difference.

#### Scenario: Historical replay remains evidence only

r[molten.authority.nominal_references.evidence_core.replay]
- GIVEN historical receipts contain typed references whose current authority has expired or been revoked
- WHEN replay runs
- THEN replay CAN validate historical linkage
- AND it MUST NOT mint current authority.

### Requirement: Octet guards migrated reference scopes

r[molten.authority.nominal_references.octet] After the policy becomes available, Molten MUST declare migrated pure-core reference domains to the reviewed Octet nominal-domain policy.

#### Scenario: Raw reference regression is rejected

r[molten.authority.nominal_references.octet.guard]
- GIVEN a migrated core scope reintroduces a declared reference as `String`
- WHEN Octet checks the scope
- THEN the selected nominal-domain check MUST fail.

### Requirement: Nominal references do not grant authority

r[molten.authority.nominal_references.docs] Molten documentation and receipts MUST state that typed references prove local category separation and checked syntax only.

#### Scenario: Boundary remains visible

r[molten.authority.nominal_references.final_checks]
- GIVEN typed references pass construction and role admission
- WHEN Molten states the supported claim
- THEN it MUST NOT claim current authority, evidence truth, transport trust, runtime correctness, semantic equivalence, or release eligibility.

## ADDED Requirements

### Requirement: Plugin grants bind referenced extension descriptors
r[molten.plugin_grant_contract_binding.descriptor_binding] Repository-owned plugin capability grant authoring contracts SHOULD validate grants against the referenced plugin extension contract descriptor when the contract artifact is available to the fixture.

#### Scenario: Grant descriptor mismatch fails authoring
- GIVEN a plugin capability grant that names operation `storage.read`
- AND the referenced extension contract does not contain the grant's hostcall descriptor ref for that operation
- WHEN Nickel evaluates the bound grant fixture
- THEN export fails before generated grant evidence can be refreshed.

### Requirement: Plugin grant schemas and replay class match descriptor
r[molten.plugin_grant_contract_binding.schema_replay_binding] A bound plugin capability grant MUST match the referenced descriptor's input schema ref, output schema ref, and replay class unless a reviewed migration fixture explicitly authorizes the difference.

#### Scenario: Input schema mismatch fails export
- GIVEN a bound grant whose input schema ref differs from the referenced hostcall descriptor
- WHEN Nickel evaluates the grant
- THEN export fails with a schema-binding invariant.

### Requirement: Plugin grant resource scope is not broader than descriptor scope
r[molten.plugin_grant_contract_binding.resource_scope] A bound plugin capability grant MUST NOT delegate resource scope, effect refs, or authority refs broader than the referenced extension descriptor and grant attenuation allow.

#### Scenario: Resource over-delegation fails authoring
- GIVEN a grant whose resource scope is absent from the descriptor's admitted resource refs
- WHEN Nickel evaluates the bound grant
- THEN export fails before runtime plugin admission can consume the grant.

### Requirement: Revocation and attenuation invariants are authoring-time checked
r[molten.plugin_grant_contract_binding.revocation_attenuation] Plugin grant authoring contracts MUST reject inverted validity windows, delegation depth beyond the maximum, revoked grants without revocation evidence, and unsupported replay classes.

#### Scenario: Revoked grant without evidence fails
- GIVEN a grant fixture marked revoked with an empty revocation ref list
- WHEN Nickel evaluates the grant
- THEN export fails before the fixture can be checked into generated evidence.

### Requirement: Bound grant fixtures migrate reviewed storage grant exports
r[molten.plugin_grant_contract_binding.fixture_migration] Checked-in plugin storage grant fixtures SHOULD use the bound grant contract when the referenced extension contract is available, and generated drift MUST be reviewed before refresh.

#### Scenario: Storage grant uses bound contract
- GIVEN the storage plugin extension contract and matching storage grant fixture
- WHEN Nickel evaluates the bound grant fixture
- THEN export succeeds through the descriptor-binding contract
- AND generated envelope drift remains reviewable.

### Requirement: Bound grant failures are negatively covered
r[molten.plugin_grant_contract_binding.negative_grant_bindings] Bound grant contracts SHOULD include negative fixtures for wrong contract or descriptor, schema mismatch, operation mismatch, resource over-scope, replay mismatch, missing revocation evidence, and inverted validity.

#### Scenario: Resource over-scope fails bound export
- GIVEN a storage grant whose resource scope is absent from the referenced descriptor
- WHEN Nickel evaluates the bound grant fixture
- THEN export fails before runtime admission can consume the grant.

### Requirement: Runtime plugin gates remain authoritative
r[molten.plugin_grant_contract_binding.runtime_boundary] Bound grant Nickel validation MUST remain authoring-time evidence only and MUST NOT replace runtime plugin hostcall, lifecycle, authority, resource, effect, policy, provenance, or execution admission gates.

#### Scenario: Runtime still denies stale grant
- GIVEN a grant export that previously passed authoring checks
- AND runtime evidence shows the plugin manifest or extension contract has changed
- WHEN hostcall admission evaluates the grant
- THEN runtime denies unless the current canonical contract, manifest, authority, resource, effect, policy, and lifecycle evidence all match.

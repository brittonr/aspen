## ADDED Requirements

### Requirement: Plugin extension envelopes couple schema ids to payload contracts
r[molten.nickel_envelope_payload_contracts.schema_payload_coupling] Repository-owned Nickel plugin extension export envelopes MUST validate the payload against the contract selected by the envelope schema id.

#### Scenario: Contract schema requires contract payload
- GIVEN an envelope whose schema id declares a plugin extension contract export
- WHEN Nickel evaluates the envelope
- THEN the payload must satisfy the plugin extension contract authoring contract
- AND a grant-shaped or untyped payload fails export.

### Requirement: Plugin extension export identity is payload-bound
r[molten.nickel_envelope_payload_contracts.identity_binding] Plugin extension contract and grant envelopes SHOULD bind `export_identity` to reviewed payload fields such as extension id/version or plugin id/operation.

#### Scenario: Identity mismatch fails export
- GIVEN a plugin capability grant envelope whose payload names operation `storage.read`
- AND the envelope export identity names a different operation
- WHEN Nickel evaluates the fixture
- THEN export fails before generated JSON or Preserves evidence can be refreshed.

### Requirement: Typed envelope fixtures are drift-gated
r[molten.nickel_envelope_payload_contracts.fixture_migration] Checked-in generated plugin envelope exports MUST be regenerated from the typed envelope contracts and compared by the repository drift gate.

#### Scenario: Generated envelope drift is detected
- GIVEN a source envelope fixture changes its typed payload or metadata
- WHEN the drift gate exports the JSON
- THEN the gate fails until the checked-in generated output is reviewed and refreshed.

### Requirement: Typed envelope failures are negatively covered
r[molten.nickel_envelope_payload_contracts.negative_envelopes] Typed plugin extension envelope contracts SHOULD include negative fixtures for wrong payload type, identity mismatch, wrong schema id, unsupported source, and missing metadata.

#### Scenario: Wrong payload type fails export
- GIVEN an envelope with the contract schema id and a grant-shaped payload
- WHEN Nickel evaluates the typed contract envelope
- THEN export fails before generated JSON can be refreshed.

### Requirement: Envelope hardening remains authoring-time only
r[molten.nickel_envelope_payload_contracts.runtime_boundary] Typed Nickel envelope contracts MUST NOT become runtime authority for plugin admission, hostcalls, policy, resources, provenance, transport, or execution.

#### Scenario: Runtime consumes checked evidence only
- GIVEN a plugin envelope fixture passes Nickel evaluation
- WHEN runtime plugin admission runs
- THEN admission still depends on checked Preserves evidence and Rust semantic gates
- AND runtime does not execute Nickel as live authority.

## ADDED Requirements

### Requirement: Replay coverage matrices are catalog-searchable
r[molten.catalog.replay_coverage.matrix_search] The catalog SHOULD classify replay coverage matrices by artifact kind, decision, subsystem names, workflow names, replay eligibility classes, missing-evidence diagnostics, and referenced replay index refs.

#### Scenario: Matrix is found by subsystem
- GIVEN an imported replay coverage matrix with a node-control row
- WHEN catalog search filters by `replay-coverage-subsystem:node-control`
- THEN the matrix evidence is returned without granting replay pass authority.

### Requirement: Replay coverage readback is read-only
r[molten.catalog.replay_coverage.readonly] Replay coverage MCP or catalog readback MUST remain read-only and MUST NOT replace replay verification, subsystem gates, source gates, policy, provenance, authority, transport, release, or retention checks.

#### Scenario: MCP readback binds request only
- GIVEN an MCP request searches replay coverage matrices
- WHEN the request succeeds
- THEN the response receipt binds the read-only request and response
- AND mutating catalog tools remain denied.

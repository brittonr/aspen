# Runtime Spine Delta: retention remote clearance live workflow

### Requirement: Remote clearance request and response artifacts
r[molten.retention.remote_clearance_request_response] Molten MUST represent peer-produced remote retention clearance workflows as canonical request and response artifacts that bind requester, peer, object, retention class, action, remote ref, policy, authority, supporting evidence, clearance value, diagnostics, and checks.

#### Scenario: Peer response binds request and clearance
- GIVEN a requester asks a peer for destructive remote GC clearance
- WHEN the peer emits a clearance response
- THEN the response binds the exact request ref and embedded clearance ref before the requester may import it

### Requirement: Remote clearance import gate
r[molten.retention.remote_clearance_import_gate] Molten MUST fail closed when importing remote clearance responses unless the request, response, and embedded clearance are current, passing, untampered, and scope-matching for the expected peer, remote ref, object, class, action, policy, and authority.

#### Scenario: Import stores only passing clearance
- GIVEN a response with stale, revoked, retained, wrong-peer, wrong-remote, wrong-request, or tampered clearance evidence
- WHEN the requester imports the response
- THEN Molten emits a denial receipt, does not store the clearance locally, and destructive admission still lacks clearance

r[molten.retention.remote_clearance_workflow_diagnostics] Molten MUST surface remote clearance workflow diagnostics without treating request, response, clearance, or import receipts as authority, policy, resource, provenance, transport, execution, or source-gate trust.

#### Scenario: Workflow diagnostics remain evidence-only
- GIVEN an imported remote clearance response
- WHEN the import receipt is rendered or supplied to destructive retention flows
- THEN diagnostics identify the clearance workflow decision while local authority and policy admissions remain separate requirements

### Requirement: Remote clearance workflow CLI and tests
r[molten.retention.remote_clearance_workflow_cli] Molten MUST expose CLI commands for building remote clearance requests, producing peer responses, importing responses, and showing workflow artifacts.

#### Scenario: Operator imports peer clearance
- GIVEN an operator has a request and a peer response
- WHEN the operator runs the import command
- THEN Molten writes an import receipt and stores the embedded clearance only if all workflow bindings pass

r[molten.retention.remote_clearance_workflow_tests] Molten MUST test pass import, retained or stale peer denial, wrong request or peer denial, tampered response denial, and destructive admission using imported peer clearance.

#### Scenario: Tests prove workflow fail-closed behavior
- GIVEN destructive cleanup depends on remote clearance produced through the workflow
- WHEN incomplete or mismatched workflow evidence is supplied
- THEN tests verify denial receipts are auditable and selected content remains intact

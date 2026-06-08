# Runtime Spine Delta: retention remote clearance live transport

### Requirement: Remote clearance live transport receipts
r[molten.retention.remote_clearance_live_transport] Molten MUST carry `retention-remote-gc-clearance-request-v1` and `retention-remote-gc-clearance-response-v1` artifacts through node-control live workflow receipts that bind request ref, response ref, requester, peer, remote ref, object ref, action, and diagnostics.

#### Scenario: Live workflow binds request and response
- GIVEN a requester sends a remote retention clearance request over live node-control workflow transport
- WHEN the peer responds
- THEN Molten records live workflow evidence that binds the exact request and response refs without treating transport delivery as deletion authority

### Requirement: Remote clearance live import gate
r[molten.retention.remote_clearance_live_import_gate] Molten MUST still import live remote clearance responses through `retention-remote-gc-clearance-import-v1` before destructive retention admission may use the embedded peer clearance.

#### Scenario: Import remains the deletion-safety gate
- GIVEN a live response from a peer
- WHEN the response is retained, stale, revoked, wrong-scope, or tampered
- THEN Molten emits denial evidence, does not store the embedded clearance locally, and destructive admission lacks remote clearance

### Requirement: Remote clearance live CLI and diagnostics
r[molten.retention.remote_clearance_live_cli] Molten MUST expose deterministic CLI support for exercising remote clearance request/respond/import over live loopback transport.

#### Scenario: Operator runs loopback live clearance
- GIVEN an operator has retention policy, authority, requester, peer, object, and remote refs
- WHEN the operator runs the live loopback clearance command
- THEN Molten emits request, response, import, and live workflow receipts that can be inspected by ref

r[molten.retention.remote_clearance_live_diagnostics] Molten MUST surface live transport diagnostics for missing, retained, stale, revoked, wrong-peer, wrong-request, wrong-remote, and tampered response evidence without treating live receipts as authority, policy, resource, provenance, execution, source-gate, or remote-GC clearance trust.

#### Scenario: Live diagnostics remain evidence-only
- GIVEN a live clearance workflow denial
- WHEN the live receipt is rendered or supplied with destructive evidence
- THEN diagnostics identify the transport and clearance failure while local authority and policy admissions remain separate requirements

### Requirement: Remote clearance live tests
r[molten.retention.remote_clearance_live_tests] Molten MUST test passing live loopback import, retained or stale peer denial, wrong peer or request denial, tampered response denial, and destructive admission using imported live clearance.

#### Scenario: Tests prove live transport fail-closed behavior
- GIVEN destructive cleanup depends on live peer clearance
- WHEN live transport, request, response, or import evidence is incomplete or mismatched
- THEN tests verify denial receipts are auditable and selected content remains intact

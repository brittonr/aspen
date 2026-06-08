## Requirements

### Requirement: Remote clearance live multi-host request send

r[molten.retention.remote_clearance_live_multihost_request] Molten MUST expose a requester-side live command that stores a canonical `retention-remote-gc-clearance-request-v1` artifact and sends a node-control live ingress request bound to the request ref, requester node, peer node, topic, sequence, authority, policy, resource, peer-bootstrap, and evidence refs.

#### Scenario: Requester sends clearance request evidence

- GIVEN a requester has a remote clearance request scope and a peer live ticket
- WHEN the requester runs the live request-send command
- THEN Molten writes the request artifact, node-control request artifact, and send receipt without treating the send receipt as clearance or authority

### Requirement: Remote clearance live multi-host response send

r[molten.retention.remote_clearance_live_multihost_response] Molten MUST expose a peer-side live command that reads a request artifact, stores a canonical `retention-remote-gc-clearance-response-v1` artifact, and sends a node-control live ingress request bound to the response ref and original request ref back to the requester.

#### Scenario: Peer sends clearance response evidence

- GIVEN a peer has received a remote clearance request artifact
- WHEN the peer runs the live response-send command
- THEN Molten writes the response artifact, node-control request artifact, and send receipt without storing requester-side clearance

### Requirement: Remote clearance live multi-host import workflow

r[molten.retention.remote_clearance_live_multihost_import] Molten MUST expose a requester-side import workflow command that imports a peer response through `retention-remote-gc-clearance-import-v1` and stores `retention-remote-gc-clearance-live-workflow-v1` evidence binding request, response, import, send, receive, and ingress refs.

#### Scenario: Requester imports live peer response

- GIVEN a requester has the original request, peer response, node-control send receipts, receive receipts, and ingress refs
- WHEN the requester runs the live import workflow command
- THEN Molten emits an import receipt and live workflow receipt that bind all provided evidence refs

### Requirement: Remote clearance live multi-host import gate

r[molten.retention.remote_clearance_live_multihost_import_gate] Molten MUST keep `retention-remote-gc-clearance-import-v1` as the only live multi-host step that stores embedded peer clearance for destructive admission.

#### Scenario: Live transport evidence is not deletion clearance

- GIVEN live request and response send receipts exist without a passing import receipt
- WHEN destructive retention admission evaluates remote peer clearance
- THEN Molten denies because live transport receipts are not accepted as remote clearance

### Requirement: Remote clearance live multi-host diagnostics

r[molten.retention.remote_clearance_live_multihost_diagnostics] Molten MUST surface multi-host live diagnostics for missing or denied send, receive, ingress, wrong-peer, wrong-request, wrong-remote, retained, stale, revoked, and tampered-response evidence without treating live transport receipts as authority, policy, resource, provenance, execution, source-gate, or remote-GC trust.

#### Scenario: Denied transport fails closed

- GIVEN a live send receipt is denied or lacks a matching transport receipt
- WHEN the requester assembles the live workflow
- THEN Molten records transport diagnostics and the live workflow fails closed unless the import and evidence bindings pass

### Requirement: Remote clearance live multi-host tests

r[molten.retention.remote_clearance_live_multihost_tests] Molten MUST test request send, response send, final workflow assembly, denied transport evidence, and destructive admission through imported peer clearance.

#### Scenario: Tests prove multi-host safety boundary

- GIVEN multi-host live remote-clearance tests execute
- WHEN live evidence is complete or incomplete
- THEN passing cases require imported peer clearance and incomplete transport records fail closed with diagnostics

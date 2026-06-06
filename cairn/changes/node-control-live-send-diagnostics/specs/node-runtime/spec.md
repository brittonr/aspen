# Node Runtime Delta: Live Send Diagnostics

### Requirement: Live send tickets expose expected receiver guards
r[molten.node_control_live_send_diagnostics.spec.expected_ticket_guards] Node-control live send MUST support optional expected receiver node, topic, and endpoint guards, and MUST deny before live transport when the parsed live ticket does not match those expectations.

#### Scenario: Wrong topic denies before transport
- GIVEN a live ticket for topic `node-control`
- WHEN live send is invoked with a different expected topic
- THEN the send receipt decision is deny
- AND diagnostics identify the ticket topic mismatch.

### Requirement: Sender state roots preflight imported peer admissions
r[molten.node_control_live_send_diagnostics.spec.peer_import_preflight] State-root-bound node-control live send MUST resolve supplied peer bootstrap refs from the sender node ledger before transport and validate that a live peer-admission receipt binds the ticket, peer, node, topic, sequence, and expiry.

#### Scenario: Missing peer admission suggests import
- GIVEN a sender state root without the referenced peer-admission artifact
- WHEN live send is invoked with that peer bootstrap ref
- THEN the send receipt decision is deny before transport
- AND diagnostics suggest `live-ticket-import --peer-admission`.

### Requirement: Sender state roots preflight imported authority grants
r[molten.node_control_live_send_diagnostics.spec.authority_import_preflight] State-root-bound node-control live send MUST resolve supplied authority refs from the sender node ledger before transport and validate grant peer, node, operation, scopes, epoch, expiry, and revocation bounds.

#### Scenario: Missing grant suggests import
- GIVEN a sender state root without the referenced authority grant artifact
- WHEN live send is invoked with that authority ref
- THEN the send receipt decision is deny before transport
- AND diagnostics suggest `authority-grant-import`.

### Requirement: Live-send receipts classify denial causes
r[molten.node_control_live_send_diagnostics.spec.receipt_checks] `node-control-live-send-receipt-v1` MUST classify receiver ticket expectation binding, receiver-address availability/support, operation-id binding, sender state-root evidence, and join/publish success through canonical check labels.

#### Scenario: Receipt checks expose categories
- GIVEN a live send denial caused by wrong ticket binding or missing imports
- WHEN the send receipt is rendered
- THEN the checks sequence includes failing category labels
- AND diagnostics include concrete operator guidance.

### Requirement: Diagnostics remain non-authority
r[molten.node_control_live_send_diagnostics.spec.non_authority] Live-send diagnostics, import hints, and sender-side preflight receipts MUST NOT satisfy receiver-side peer bootstrap, authority, policy/resource, delivery-idempotency, or provenance gates.

#### Scenario: Receiver still denies missing original refs
- GIVEN a send receipt with diagnostics or import hints
- WHEN receiver ingress evaluates missing original evidence
- THEN enqueue still denies before side effects
- AND transport diagnostics are not treated as authority.

### Requirement: Tests cover diagnostic categories
r[molten.node_control_live_send_diagnostics.spec.tests] Automated tests MUST cover expected ticket mismatch and missing sender-side import diagnostics.

#### Scenario: CLI emits diagnostic receipt
- GIVEN receiver-created ticket, peer-admission, and grant files that are not imported into the sender state root
- WHEN live send runs with expected guard options
- THEN the CLI writes a deny send receipt containing import hints and category checks.

# Node Runtime Delta: Live Import UX

### Requirement: Live tickets import through canonical receipts
r[molten.node_control_live_import_ux.spec.ticket_import_receipt] Node-control live ticket import MUST emit a canonical `node-control-live-ticket-import-receipt-v1` receipt binding state root, ticket ref, node, topic, endpoint, optional peer-admission ref, as-of sequence, imported refs, diagnostics, and checks.

#### Scenario: Ticket import emits pass receipt
- GIVEN a live ticket and matching peer-admission artifact
- WHEN `live-ticket-import` validates them against the expected node, topic, peer, and sequence
- THEN the ticket and admission are imported into the node ledger
- AND the import receipt decision is pass.

### Requirement: Ticket import validates peer admission freshness
r[molten.node_control_live_import_ux.spec.ticket_admission_freshness] Live ticket import MUST validate supplied peer-admission schema/version, ticket binding, node/topic binding, peer binding when requested, not-before sequence, and expiry before importing the admission artifact.

#### Scenario: Stale admission denies import
- GIVEN a live ticket and a peer admission whose expiry is older than the requested as-of sequence
- WHEN `live-ticket-import` runs
- THEN the import receipt decision is deny
- AND the ticket/admission artifacts are not admitted as imported refs.

### Requirement: Authority grants import through canonical receipts
r[molten.node_control_live_import_ux.spec.authority_import_receipt] Node-control authority grant import MUST emit a canonical `node-control-authority-grant-import-receipt-v1` receipt binding state root, grant ref, peer, node, operations, target/resource scopes, as-of epoch, imported refs, diagnostics, and checks.

#### Scenario: Authority import emits pass receipt
- GIVEN a grant matching the expected peer, node, operation, scopes, and epoch
- WHEN `authority-grant-import` runs
- THEN the grant is imported into the node ledger
- AND the import receipt decision is pass.

### Requirement: Authority import validates grant bounds
r[molten.node_control_live_import_ux.spec.authority_binding] Authority grant import MUST validate peer/node binding, operation allowance, target/resource scope coverage, epoch not-before, expiry, and revocation refs before importing the grant artifact.

#### Scenario: Wrong operation denies import
- GIVEN a grant that does not allow the requested operation
- WHEN `authority-grant-import --operation` names that operation
- THEN the import receipt decision is deny
- AND the grant artifact is not admitted as an imported ref.

### Requirement: Import receipts are not authority or provenance
r[molten.node_control_live_import_ux.spec.import_non_authority] Live ticket and authority-grant import receipts MUST NOT satisfy peer bootstrap, operation authority, policy/resource, delivery-idempotency, or payload provenance gates.

#### Scenario: Receiver ingress still resolves original evidence
- GIVEN passing import receipts but missing live peer-admission, authority grant, or provenance refs
- WHEN receiver ingress evaluates the live envelope
- THEN enqueue still denies before side effects
- AND diagnostics identify the missing original evidence.

### Requirement: CLI exposes live import UX
r[molten.node_control_live_import_ux.spec.cli] The CLI MUST expose `live-ticket-import` and `authority-grant-import` commands with expected binding options, as-of bounds, and receipt output.

#### Scenario: Two-state-root workflow imports remote evidence
- GIVEN receiver-created ticket, admission, and authority grant files
- WHEN a sender state root imports them through the CLI commands
- THEN canonical import receipts are written
- AND stdout reports decisions, imported-ref counts, and diagnostic counts.

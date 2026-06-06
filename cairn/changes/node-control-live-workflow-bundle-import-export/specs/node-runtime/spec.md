# Node Runtime Delta: Live Workflow Bundle Import/Export

### Requirement: Live workflow bundles carry handoff members
r[molten.node_control_live_workflow_bundle.spec.bundle_artifact] Node-control live workflow bundle export MUST produce a canonical `node-control-live-workflow-bundle-v1` artifact that carries a live ticket, peer admission, authority grant, optional supporting receipts, and recomputable member refs.

#### Scenario: Bundle exports matching members
- GIVEN a live ticket, matching peer admission, matching authority grant, and a live-send receipt
- WHEN the operator exports a live workflow bundle
- THEN the bundle contains the member values and refs
- AND the export receipt decision is pass.

### Requirement: Bundle import validates ticket and admission bindings
r[molten.node_control_live_workflow_bundle.spec.ticket_admission_import] Node-control live workflow bundle import MUST validate ticket node/topic/endpoint expectations and peer admission ticket/node/topic/peer/freshness bindings before importing members.

#### Scenario: Wrong peer denies import
- GIVEN a bundle whose peer admission is for a different peer than expected
- WHEN the bundle is imported with an expected peer
- THEN the import receipt decision is deny
- AND ticket/admission members are not imported as passing bundle evidence.

### Requirement: Bundle import validates authority grants
r[molten.node_control_live_workflow_bundle.spec.authority_import] Node-control live workflow bundle import MUST validate authority grant peer, node, operations, scopes, epoch, expiry, and revocation bounds before importing grant evidence.

#### Scenario: Missing operation denies import
- GIVEN a bundle whose authority grant does not allow the expected operation
- WHEN the bundle is imported with that expected operation
- THEN the import receipt decision is deny
- AND diagnostics identify the missing operation.

### Requirement: Malformed bundle members fail closed
r[molten.node_control_live_workflow_bundle.spec.malformed_members] Node-control live workflow bundle import MUST fail closed on missing member fields, malformed member records, or member ref mismatches before importing ticket, admission, authority, or receipt evidence.

#### Scenario: Missing member fields reject import
- GIVEN a bundle-shaped value with missing member fields
- WHEN the bundle is imported
- THEN import fails closed before member artifacts are imported
- AND no bundle member is treated as authority or provenance.

### Requirement: Bundle import enables sender-side live-send preflight
r[molten.node_control_live_workflow_bundle.spec.sender_preflight] A passing node-control live workflow bundle import MUST materialize the underlying live ticket, peer admission, and authority grant artifacts into the sender state root so live-send sender-side preflight can resolve those refs.

#### Scenario: Imported bundle removes missing-evidence diagnostics
- GIVEN a sender state root without peer admission or authority grant artifacts
- WHEN a valid bundle is imported into that state root
- THEN a subsequent state-root-bound live-send can resolve the peer bootstrap and authority refs
- AND live-send diagnostics no longer report missing sender-state-root evidence for those refs.

### Requirement: Bundle receipts are not authority
r[molten.node_control_live_workflow_bundle.spec.non_authority] Node-control live workflow bundles and bundle import/export receipts MUST NOT satisfy receiver-side authority, policy/resource, delivery-idempotency, or provenance gates by themselves.

#### Scenario: Receiver still requires original refs
- GIVEN a bundle import receipt and no original authority grant ref in the live envelope
- WHEN receiver live ingress evaluates the envelope
- THEN receiver-side admission denies before enqueue
- AND the bundle receipt is not treated as authority.

### Requirement: CLI coverage exercises bundle handoff
r[molten.node_control_live_workflow_bundle.spec.cli_tests] Automated CLI tests MUST cover bundle export/import receipts and sender preflight resolution after bundle import.

#### Scenario: CLI bundle import writes receipts
- GIVEN receiver-created live handoff members
- WHEN the CLI exports and imports a workflow bundle
- THEN bundle, export receipt, and import receipt artifact kinds are recognized
- AND a follow-up live-send no longer reports missing peer admission or authority grant evidence.

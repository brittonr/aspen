# Node Runtime Delta: Authority Delegation

### Requirement: Authority grants are canonical delegation artifacts
r[molten.node_control_authority_delegation.spec.grant_artifacts] Node-control authority delegation MUST be represented by canonical `node-control-authority-grant-v1` artifacts that bind peer id, node id, allowed operations, target/resource scope, epoch/expiry, policy refs, revocation refs, evidence refs, and checks.

#### Scenario: Grant fixture is importable
- GIVEN a node state root
- WHEN a grant fixture is created for a peer, node, and operation
- THEN the grant has a stable artifact ref
- AND it can be imported into the local node ledger for live ingress authority lookup.

### Requirement: Live ingress resolves authority refs before enqueue
r[molten.node_control_authority_delegation.spec.live_pre_enqueue_gate] Live node-control ingress MUST resolve authority refs to admitted grant artifacts before delivery idempotency or queue side effects.

#### Scenario: Live grant admits request
- GIVEN a live envelope from an admitted peer with a grant for the node and operation
- WHEN ingress delivery runs
- THEN authority delegation passes
- AND the request may proceed to idempotency and durable inbox enqueue.

### Requirement: Delegation fails closed
r[molten.node_control_authority_delegation.spec.fail_closed] Live authority delegation MUST deny before side effects when the grant is unknown, not a grant, bound to the wrong peer/node/operation/scope, expired, not yet valid, or revoked.

#### Scenario: Wrong operation is denied
- GIVEN a live envelope requesting `status`
- AND the referenced grant only allows `shutdown`
- WHEN ingress delivery runs
- THEN no request is enqueued
- AND the ingress/authority receipts contain denial diagnostics.

### Requirement: Transport is not authority
r[molten.node_control_authority_delegation.spec.transport_non_authority] Live Iroh transport, neighbor events, endpoint ids, peer bootstrap refs, and capability offers MUST NOT satisfy node-control operation authority without a resolved delegation grant.

#### Scenario: Unknown live authority ref is denied
- GIVEN a live envelope with valid transport delivery but no admitted authority grant in the node ledger
- WHEN ingress delivery runs
- THEN the envelope is denied before enqueue
- AND transport receipt evidence is not treated as authority.

### Requirement: Delegation is separate from provenance
r[molten.node_control_authority_delegation.spec.provenance_separation] Authority delegation MUST only decide whether a peer may request an operation; install/run payload trust MUST remain governed by provenance gates.

#### Scenario: Delegated install still needs provenance
- GIVEN a peer has an admitted install delegation
- WHEN the install payload lacks admitted provenance evidence
- THEN authority may pass
- BUT dispatch still denies the install side effect at the provenance gate.

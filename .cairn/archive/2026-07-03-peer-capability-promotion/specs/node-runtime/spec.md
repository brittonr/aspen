## ADDED Requirements

### Requirement: Node runtime applies promotions only after gates pass
r[molten.peer_promotion.node_apply_boundary] Molten MUST update node-local peer session read models for promoted or demoted capabilities only after promotion apply or demotion receipts pass all authority, policy, resource, expiry, revocation, and approval gates.

#### Scenario: Failed promotion leaves read model unchanged
- GIVEN a peer promotion apply operation fails because the issuer is revoked
- WHEN the node runtime updates peer-session state
- THEN the prior session capabilities remain unchanged
- AND the failed promotion receipt is stored only as denial evidence.

### Requirement: Promotion apply does not perform subsystem side effects
r[molten.peer_promotion.apply_no_subsystem_side_effects] Molten SHOULD limit promotion apply to capability/session state changes and MUST NOT automatically execute node-control operations, job work, retention actions, sync imports, relay publication, or Raft membership changes.

#### Scenario: Publisher promotion does not publish
- GIVEN a peer is promoted from subscriber to scoped publisher
- WHEN promotion apply passes
- THEN the session records the new publish capability
- AND no message is published until a separate publish operation passes its own gates.

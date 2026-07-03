## ADDED Requirements

### Requirement: Node-control supports generic peer handoff bundles
r[molten.peer_handoff.node_control_compat] Molten MUST preserve existing node-control live workflow receipt semantics while allowing node-control handoff export, verify, gate, import, and apply flows to use the generic peer handoff bundle model.

#### Scenario: Existing node-control bundle remains readable
- GIVEN a node-control live workflow bundle produced before the generic handoff model
- WHEN the compatibility parser reads the bundle
- THEN it can summarize the existing ticket, peer admission, authority grant, and receipt refs
- AND it does not reinterpret the bundle as authority beyond the embedded grant artifacts.

### Requirement: Handoff validation covers subsystem consumers
r[molten.peer_handoff.consumer_scope_binding] Molten MUST require node-control, remote dataspace, job worker, retention clearance, and remote artifact sync consumers to check the handoff scope before using imported peer evidence.

#### Scenario: Job handoff cannot satisfy node-control scope
- GIVEN a peer handoff bundle is scoped to a job worker pool
- WHEN a node-control live send tries to use that handoff as peer bootstrap evidence
- THEN node-control preflight denies the scope mismatch
- AND diagnostics name the expected node-control topic or operation scope.

### Requirement: Peer handoff validation is reproducible
r[molten.peer_handoff.validation] Molten SHOULD validate generic handoff work with focused handoff tests, node-control bundle compatibility tests, remote dataspace/job/retention/sync consumer tests, formatting, and Cairn validation before the change is archived.

#### Scenario: Consumer regression is caught
- GIVEN a subsystem consumer accepts a handoff bundle whose declared scope does not match the operation
- WHEN focused peer handoff validation runs
- THEN the negative consumer fixture fails
- AND the change cannot complete until the scope denial is restored.

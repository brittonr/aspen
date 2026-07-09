# Node Runtime Delta: Iroh ALPN Routing Registry

### Requirement: Iroh ALPN registry entries are canonical
r[molten.node_runtime.iroh_alpn_registry_model] Molten MUST define canonical registry entries for Molten-owned Iroh ALPN protocols, binding symbolic name, ALPN bytes or deterministic string encoding, owner namespace, handler profile, supported schema/profile versions, lifecycle state, limit profile refs, required admission evidence, and receipt schema refs.

#### Scenario: Registry entry binds handler ownership
- GIVEN a Molten-owned Iroh protocol handler is proposed for installation
- WHEN the router admission core evaluates it
- THEN the handler descriptor references a canonical registry entry with matching symbolic name, ALPN, owner namespace, and handler profile.

#### Scenario: Deprecated entry is explicit
- GIVEN an ALPN exists only for migration or diagnostic compatibility
- WHEN it appears in the registry
- THEN the registry marks its lifecycle state explicitly and prevents it from becoming a production default without a reviewed registry update.

### Requirement: ALPN registry validation fails closed
r[molten.node_runtime.iroh_alpn_registry_validation] Molten MUST validate ALPN registry entries for deterministic encoding, non-empty owner namespace, unique ALPN bytes within the active registry, supported lifecycle state, and compatible limit/profile refs before live router mutation.

#### Scenario: Duplicate ALPN denies registry admission
- GIVEN two active registry entries declare the same ALPN bytes
- WHEN registry validation runs
- THEN validation denies the registry and no live handler map is updated from it.

#### Scenario: Malformed ALPN denies before advertising
- GIVEN a proposed handler names an empty, malformed, or unsupported ALPN encoding
- WHEN router admission evaluates the handler
- THEN the admission denies before the endpoint advertises the ALPN.

### Requirement: Router handler ownership is enforced
r[molten.node_runtime.iroh_alpn_handler_ownership] Runtime router install, replacement, removal, and shutdown operations MUST check registry ownership, current generation, handler profile compatibility, and lifecycle state before mutating the live advertised ALPN map.

#### Scenario: Wrong owner cannot replace handler
- GIVEN an active handler owned by one protocol namespace
- WHEN a replacement request from a different owner namespace targets the same ALPN
- THEN router admission denies with an owner-mismatch diagnostic and the existing generation remains active.

#### Scenario: Replacement records registry generation
- GIVEN a valid replacement for an active handler
- WHEN router admission passes
- THEN the router receipt binds the registry entry ref, prior generation, new generation, handler profile, and prior-handler shutdown evidence.

### Requirement: ALPN routing evidence is not operation authority
r[molten.node_runtime.iroh_alpn_non_authority] Molten MUST NOT treat ALPN negotiation, endpoint identity, router registry entries, router receipts, stream sessions, or framed-envelope receipts as authority for node-control, protocol-session, artifact import, retention, execution, provenance, resource, policy, or source-gate side effects.

#### Scenario: Valid ALPN without authority denies operation
- GIVEN a peer opens a connection using a registered ALPN and sends a well-formed frame
- WHEN the downstream operation lacks matching authority, policy, resource, or subsystem evidence
- THEN operation admission denies before side effects and diagnostics state that ALPN routing is not authority.

#### Scenario: Unsupported ALPN denies before frame delivery
- GIVEN an incoming connection offers an ALPN that is absent, removed, or unsupported by the current registry generation
- WHEN the router evaluates the connection
- THEN the router emits deny evidence before delivering any frame to node-control, protocol-session, plugin, dataspace, or service state.
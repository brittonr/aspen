## ADDED Requirements

### Requirement: Runtime-managed Iroh protocol router
r[molten.node_runtime.iroh_protocol_router] Molten MUST provide a runtime-managed Iroh protocol router boundary that installs, replaces, removes, and shuts down ALPN protocol handlers only after explicit admission by authority, policy, resource, and evidence inputs.

#### Scenario: Admitted ALPN handler is installed
- GIVEN a node-control request to install an Iroh protocol handler with valid ALPN, authority refs, policy refs, resource refs, and supporting evidence refs
- WHEN the router admission core evaluates the request
- THEN it returns a pass decision with the installed handler descriptor and generation
- AND the live router shell advertises the ALPN only after the pass receipt is recorded.

#### Scenario: Unsupported ALPN denies before delivery
- GIVEN an incoming Iroh connection for an ALPN that is not registered or no longer registered
- WHEN the router evaluates the connection
- THEN Molten emits deny evidence for unsupported ALPN
- AND no envelope frame is delivered to node-control, protocol-session, plugin, dataspace, or service state.

#### Scenario: Replacement advances generation and shuts down prior handler
- GIVEN an existing registered ALPN handler
- WHEN an admitted replacement is applied
- THEN the router records the previous handler generation, advances the replacement generation, and binds shutdown evidence for the previous handler
- AND new connections use the replacement handler while existing connections follow the configured drain policy.

### Requirement: Iroh protocol router receipts
r[molten.node_runtime.iroh_protocol_router_receipts] Molten MUST emit canonical router receipts for protocol install, replacement, removal, shutdown, unsupported-ALPN denial, and stale-generation denial.

#### Scenario: Removed handler no longer advertises ALPN
- GIVEN a registered ALPN handler with a current generation
- WHEN an admitted remove request succeeds
- THEN the router receipt records decision `pass`, operation `remove`, the removed generation, and handler shutdown evidence
- AND subsequent connection attempts for that ALPN deny before frame delivery.

#### Scenario: Stale generation cannot replace handler
- GIVEN a replacement request that references a stale prior generation
- WHEN the router admission core evaluates the request
- THEN it emits a deny receipt with stale-generation diagnostics
- AND the live advertised ALPN map remains unchanged.

### Requirement: Framed canonical envelope stream over Iroh
r[molten.node_runtime.iroh_framed_envelope_stream] Molten MUST support a bounded framed-envelope stream over Iroh bidirectional connections where each frame carries canonical Preserves envelope bytes, declared envelope refs, peer/node ids, sequence, ALPN, and limit-profile evidence.

#### Scenario: Valid frame delivers canonical envelope
- GIVEN a framed stream session for an admitted ALPN and a frame whose canonical Preserves bytes hash to the declared envelope ref
- WHEN the framed-envelope validator checks the frame against configured byte and sequence limits
- THEN it emits a pass receipt binding the frame length, actual envelope ref, declared envelope ref, ALPN, peer, node, and sequence
- AND the envelope may be handed to the normal node-control or protocol-session admission path.

#### Scenario: Oversized frame denies before parsing payload
- GIVEN a frame whose declared length exceeds the configured max frame bytes
- WHEN the framed-envelope validator receives the frame
- THEN it emits a deny receipt for oversized frame
- AND the payload is not parsed, delivered, or written into runtime state.

#### Scenario: Declared envelope ref mismatch denies
- GIVEN a frame with canonical Preserves bytes whose hash differs from the declared envelope ref
- WHEN the framed-envelope validator checks the frame
- THEN it emits a deny receipt with declared and actual refs
- AND the frame is excluded from deterministic pass evidence and live delivery.

### Requirement: Iroh service-session streaming patterns
r[molten.node_runtime.iroh_service_session_streaming] Molten SHOULD model local and remote service interactions over admitted Iroh framed streams with explicit unary request/response, server-streaming, client-streaming, and bidirectional-streaming session descriptors while preserving canonical Preserves envelope identity for every remote frame.

#### Scenario: Unary request response binds same local and remote model
- GIVEN a service method that can run locally or over an admitted Iroh framed stream
- WHEN Molten records the request and response session
- THEN both local and remote paths bind the same service id, operation id, interaction kind, capability refs, policy refs, resource refs, request ref, and response ref
- AND the remote path additionally binds ALPN, peer, node, stream, and frame receipts.

#### Scenario: Streaming session applies per-frame admission
- GIVEN a server-streaming, client-streaming, or bidirectional service session over Iroh
- WHEN a stream update frame is received
- THEN each update is validated as a bounded canonical Preserves envelope with sequence and flow-control evidence
- AND malformed, oversized, stale, or unauthorized updates deny without mutating service state.

#### Scenario: Postcard-only IRPC wire format is not canonical Molten boundary
- GIVEN an IRPC-style Rust service interaction pattern
- WHEN Molten exposes that interaction across node or process boundaries
- THEN the canonical Molten boundary remains versioned Preserves envelope frames
- AND postcard or Rust-only message serialization may only be an internal implementation detail behind explicit conversion evidence.

# Fabric Transport Specification

## Purpose

Defines the `fabric-transport` capability.

## Requirements

### Requirement: Extensions use a canonical transport port
r[molten.fabric_transport.port_contract] Aspen MUST expose versioned canonical transport commands and events for protocol registration, dial, accept, session state, streams, framed messages, optional datagrams, flow-control credits, cancellation, close, and failures. Transport ids and handles MUST be opaque, canonical, scoped to the active service generation, and independent of live-adapter runtime types.

#### Scenario: Protocol core uses adapter-neutral values
- GIVEN a running extension has an admitted transport binding
- WHEN its protocol core dials a peer and opens a stream
- THEN it submits canonical commands and receives canonical events
- AND it does not receive an Iroh, socket, executor, or simulator-owned object.

#### Scenario: Unsupported capability denies
- GIVEN an extension requests datagrams from a transport profile that does not declare them
- WHEN command validation runs
- THEN the command denies before adapter I/O
- AND diagnostics identify the unsupported capability and selected profile.

### Requirement: Protocol registration is unique and generation-fenced
r[molten.fabric_transport.protocol_registration] Aspen MUST require an explicit capability and admitted descriptor before a system extension can own a protocol or ALPN. Active registrations MUST bind protocol identity and version, extension identity, service generation, listener bounds, framing profile, and cleanup policy. Conflicting, duplicate, stale-generation, unauthorized, or unsupported registrations MUST deny.

#### Scenario: Admitted protocol activates
- GIVEN a running service generation holds protocol-registration authority and a compatible transport binding
- WHEN it registers an unowned protocol descriptor
- THEN the registry atomically activates routing to that generation.

#### Scenario: Replacement cannot race a stale listener
- GIVEN a service generation is draining and a replacement generation is admitted
- WHEN registration ownership transfers
- THEN new sessions route only to the replacement after atomic activation
- AND stale callbacks cannot accept sessions for the old generation.

### Requirement: Sessions and streams have canonical lifecycle events
r[molten.fabric_transport.session_streams] Aspen MUST define legal lifecycle transitions and terminal outcomes for outbound and inbound sessions, streams, framed messages, optional datagrams, half-close, full close, reset, refusal, timeout, cancellation, and adapter failure. Events MUST correlate to the originating command or listener and active service generation.

#### Scenario: Bidirectional stream completes
- GIVEN an active session and available stream budget
- WHEN the extension opens a bidirectional stream, exchanges bounded frames, and closes both directions
- THEN events preserve per-stream ordering and end in one canonical terminal state.

#### Scenario: Event for unknown handle is rejected
- GIVEN an adapter emits an event for an unknown, closed, or wrong-generation transport id
- WHEN event routing runs
- THEN routing denies or quarantines the adapter event
- AND does not deliver it to extension code.

### Requirement: Transport flow control and cancellation are bounded
r[molten.fabric_transport.flow_control] Aspen MUST enforce admitted bounds for listeners, sessions, streams, frames, queued events, queued bytes, in-flight bytes, and operation deadlines. The port MUST expose explicit credit or readiness, backpressure, cancellation, and terminal events so neither adapter nor extension requires an unbounded hidden queue.

#### Scenario: Sender observes backpressure
- GIVEN a stream has consumed its admitted send credit
- WHEN the extension submits another frame
- THEN the port delays or rejects it according to the declared profile
- AND restores progress only through an explicit credit, readiness, cancellation, or terminal event.

#### Scenario: Oversized frame is denied before callback
- GIVEN a remote peer sends a frame larger than the admitted framing bound
- WHEN the live or simulated adapter parses the outer frame
- THEN it denies or resets according to policy before delivering the payload to extension code.

### Requirement: Transport identity is separate from authority
r[molten.fabric_transport.identity_separation] Aspen MUST represent authenticated transport peer identity, membership identity, application principal, trust decision, and capability authority as separate canonical refs. Connectivity or possession of a transport identity MUST NOT by itself grant cluster membership, service access, protocol authority, or extension capabilities.

#### Scenario: Authenticated peer still requires service authorization
- GIVEN a transport adapter authenticates a remote peer identity
- WHEN that peer opens an extension protocol
- THEN service policy evaluates the relevant membership, principal, and capability evidence separately before request admission.

#### Scenario: Unknown peer has no ambient cluster authority
- GIVEN an authenticated transport peer is absent from the admitted membership view
- WHEN it sends a cluster-internal protocol message
- THEN the message denies unless an explicit bootstrap or guest policy permits it.

### Requirement: Live and simulated transport preserve one contract
r[molten.fabric_transport.live_sim_parity] Aspen MUST provide live and deterministic-simulation transport adapters that implement the same canonical command, event, lifecycle, flow-control, cancellation, identity, and failure contract. Protocol state-transition code SHOULD run unchanged across those adapters and MUST NOT branch on hidden adapter state.

#### Scenario: Shared trace matches
- GIVEN a deterministic protocol fixture that excludes declared adapter-specific metadata
- WHEN it runs against live loopback and simulation adapters
- THEN canonical protocol transitions and terminal outcomes match the fixture's allowed trace set.

#### Scenario: Adapter-specific behavior is declared
- GIVEN one adapter supports an optional transport capability or reports different diagnostic metadata
- WHEN its profile is registered
- THEN the difference is explicit in capability and non-claim fields
- AND it is not presented as base-port behavior.

### Requirement: Transport failures and delivery non-claims are explicit
r[molten.fabric_transport.failure_semantics] Aspen MUST classify local refusal, remote refusal, disconnect, reset, timeout, partition, malformed input, overload, cancellation, adapter failure, and uncertain delivery. The base transport port MUST NOT claim durable delivery, exact-once delivery, transactional messaging, global ordering, automatic retry safety, membership, consensus, or protocol-level success.

#### Scenario: Delivery is uncertain after disconnect
- GIVEN a frame was submitted and the connection fails before a definitive acknowledgement boundary
- WHEN the port reports completion
- THEN it reports an uncertain-delivery outcome rather than success or non-delivery certainty.

#### Scenario: Retry is extension-owned
- GIVEN an operation ends with an uncertain or retryable transport outcome
- WHEN no higher-level retry policy was selected
- THEN the base port does not automatically repeat the operation.

### Requirement: Transport evidence and readback are bounded
r[molten.fabric_transport.evidence] Aspen MUST emit canonical evidence for protocol registration, ownership transfer, listener activation and cleanup, material session failures, and selected aggregate traffic/resource boundaries. Operator readback MUST remain bounded and MUST NOT expose payloads or secrets by default.

#### Scenario: Registration evidence is inspectable
- GIVEN a protocol listener is active
- WHEN an authorized operator requests status
- THEN readback identifies the protocol, owning extension generation, transport profile, framing and resource bounds, and latest lifecycle evidence ref.

#### Scenario: Traffic evidence avoids packet receipts
- GIVEN a high-throughput extension exchanges many frames
- WHEN the default production evidence profile is active
- THEN aggregate counters and semantic boundary refs may be recorded
- AND one receipt per packet or frame is not required.

### Requirement: Transport validation covers positive and negative paths
r[molten.fabric_transport.final_validation] Aspen MUST run shared adapter conformance plus positive and negative tests for registration, generation fencing, framing, stream lifecycle, identity separation, backpressure, cancellation, malformed input, partitions, timeouts, uncertain delivery, drain, and cleanup.

#### Scenario: Shared adapter fixture passes
- GIVEN conforming live-loopback and deterministic-simulation adapters
- WHEN the shared suite runs
- THEN both satisfy the canonical transport contract.

#### Scenario: Non-conforming adapter denies production admission
- GIVEN an adapter loses terminal events, bypasses bounds, leaks implementation handles, or grants transport identity ambient authority
- WHEN conformance and admission run
- THEN production admission denies with a specific failed invariant.

### Requirement: Cross-process endpoints are explicit and capability-bound
r[molten.fabric_transport.cross_process_endpoint] Aspen MUST represent a live cross-process endpoint as a versioned canonical descriptor binding the exact transport profile, protocol and ALPN, public endpoint identity, owning extension and service, active generation, admitted locator cohort and disclosure policy, framing and resource profile, validity cohort, and descriptor identity. Export and import MUST exclude private key material, bearer authority, ambient paths, and raw adapter handles. Default evidence and status MUST bind redacted locator classes or refs rather than render raw direct or relay locators. Endpoint possession or successful import MUST NOT grant membership, protocol authority, application capability, or trust.

#### Scenario: Exact endpoint descriptor is admitted
- GIVEN a listener generation holds the required transport and protocol-registration capabilities
- WHEN it exports a descriptor and a client imports it with matching profile, protocol, owner, generation, endpoint identity, and peer context
- THEN endpoint admission returns a canonical dial plan
- AND the descriptor remains connectivity information rather than authority.

#### Scenario: Stale or mismatched descriptor denies before dial
- GIVEN an imported descriptor has a stale generation, wrong protocol or profile, unexpected endpoint identity, incompatible peer context, or an undeclared locator cohort
- WHEN endpoint admission runs
- THEN it denies before adapter I/O
- AND diagnostics identify the mismatched binding without selecting a fallback endpoint.

#### Scenario: Default readback redacts topology
- GIVEN an admitted endpoint descriptor contains direct or relay locator details needed for explicit handoff
- WHEN default evidence or operator status is rendered
- THEN it emits only admitted redacted locator classes or canonical refs
- AND it does not expose the raw locator details.

### Requirement: Cross-process listeners are supervised and generation-fenced
r[molten.fabric_transport.cross_process_listener] Aspen MUST expose explicit start, ready, accept, drain, cancellation, close, cleanup, failure, and replacement transitions for long-lived live listeners. Listener identity and callbacks MUST bind protocol ownership, service generation, transport profile, endpoint descriptor, listener and session bounds, and cleanup policy. Descriptor publication MUST occur only after endpoint setup, exact ALPN activation, registration ownership, and readiness pass. Drain, registration revocation, capability revocation, or profile revocation MUST stop new accepts before bounded existing-session handling, and stale or replaced generations MUST NOT deliver protocol callbacks.

#### Scenario: Listener drains and cleans up
- GIVEN an admitted listener is ready and owns active bounded sessions
- WHEN its service generation enters drain
- THEN new accepts stop before existing sessions follow the declared grace and cancellation policy
- AND the listener reaches a terminal state with cleanup evidence.

#### Scenario: Stale listener callback is fenced
- GIVEN protocol ownership has transferred to a replacement generation
- WHEN the old listener reports a new accept or frame
- THEN routing rejects or quarantines the callback before extension delivery
- AND the replacement generation remains the only active owner.

#### Scenario: Descriptor is not published before readiness
- GIVEN endpoint setup succeeded but exact ALPN activation, registration ownership, or readiness is incomplete or revoked
- WHEN descriptor publication is requested
- THEN publication denies
- AND no client-admissible endpoint artifact is emitted.

### Requirement: Cross-process sessions preserve the canonical transport contract
r[molten.fabric_transport.cross_process_session] Aspen MUST route cross-process dial, accept, session, stream, frame, acknowledgement, credit, cancellation, close, and failure observations through the accepted canonical transport command and event contract. The live shell MUST enforce outer framing and declared session, stream, queue, byte, credit, deadline, and cancellation bounds before callback delivery. Failure after submission and before a definitive acknowledgement MUST remain uncertain delivery, and the base shell MUST NOT retry automatically.

#### Scenario: Separate processes exchange one bounded frame
- GIVEN distinct listener and client processes hold compatible admitted endpoint, protocol, profile, identity, capability, and resource inputs
- WHEN the client opens a session and exchanges a frame through the declared acknowledgement boundary
- THEN both processes observe canonical adapter-neutral lifecycle events
- AND no Iroh, QUIC, socket, executor, or child-process handle enters protocol code.

#### Scenario: Disconnect before acknowledgement is uncertain
- GIVEN a frame has been submitted across a live cross-process session
- WHEN the connection terminates before the configured acknowledgement boundary
- THEN the terminal event reports uncertain delivery
- AND the transport shell does not repeat the frame without an extension-owned retry decision.

### Requirement: Distinct-process evidence is required for distributed transport claims
r[molten.fabric_transport.distinct_process_evidence] Aspen MUST require parent-observed separate listener and client process lifecycles, explicit endpoint handoff, distinct invocation identities, bounded readiness and teardown, matching child terminal statuses, and matching endpoint/profile/protocol/service-generation evidence before classifying a transport run as distinct-process live evidence. Child-authored separation claims, same-process loopback, structurally plausible receipts, shared hidden adapter state, or ambient socket exchange MUST NOT satisfy that classification. Evidence MUST exclude payload bytes, secrets, raw process ids, raw locators, and runtime handles.

#### Scenario: Receipt-first two-process run is classified correctly
- GIVEN the receipt-first harness launches listener and client roles as separate bounded child processes
- WHEN endpoint handoff, frame exchange, acknowledgement or failure classification, cancellation, and cleanup complete
- THEN the parent receipt binds both participant identities and canonical transport evidence
- AND offline verification can distinguish the run from same-process loopback.

#### Scenario: Same-process loopback cannot promote a live claim
- GIVEN the existing Iroh loopback fixture passes its canonical transport checks
- WHEN a consumer requests distinct-process transport evidence
- THEN admission denies the promotion
- AND reports that separate participant and endpoint-handoff evidence is missing.

#### Scenario: Child-only process claims are insufficient
- GIVEN listener and client artifacts claim distinct roles without matching parent-observed child starts, endpoint handoff, and terminal statuses
- WHEN offline verification classifies the run
- THEN distinct-process admission denies
- AND diagnostics identify the missing independent process observations.

### Requirement: Cross-process transport validation covers denial and cleanup paths
r[molten.fabric_transport.cross_process_validation] Aspen MUST run positive and negative pure-core, live-shell, and distinct-process tests for endpoint export and import, atomic readiness and publication, registration/capability/profile revocation, locator disclosure, protocol/profile/peer compatibility, generation fencing, listener replacement, framing, flow control, deadlines, acknowledgement, uncertain delivery, cancellation, drain, parent-observed child separation, child teardown, cleanup, identity separation, handle leakage, ambient fallback, and non-claims.

#### Scenario: Conforming separate-process adapter passes
- GIVEN a live listener and client shell implement the canonical contract with bounded resources and terminal cleanup
- WHEN shared and distinct-process conformance runs
- THEN the positive exchange and required negative paths pass
- AND the resulting evidence remains scoped to the exact run and profile.

#### Scenario: Leaked task or ambient fallback denies conformance
- GIVEN a shell leaves a listener or child alive after terminal cancellation, uses an undeclared ambient socket, bypasses endpoint admission, or leaks an adapter handle
- WHEN conformance and production admission run
- THEN admission denies with a specific failed invariant
- AND no distributed transport claim is emitted.

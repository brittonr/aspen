## ADDED Requirements

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

# Choreography Delta: ChoRus-Inspired Typed Facade

## ADDED Requirements

### Requirement: ChoRus is a design reference only
r[molten.choreography.chorus_design_reference] Molten MAY use ChoRus as a reference for choreographic-programming ergonomics, but MUST NOT treat ChoRus documentation, APIs, derive macros, transports, JSON encoding, runtime runner, or projection behavior as authoritative semantics, evidence, authority, policy admission, or compatibility targets.

#### Scenario: Reference cannot satisfy protocol gate
- GIVEN a protocol-session gate requires install, endpoint, message, operation, terminal-state, and receipt refs
- WHEN a caller presents ChoRus API usage or ChoRus runtime behavior as evidence
- THEN Molten rejects that evidence unless the normal Molten canonical refs and receipts are also present.

#### Scenario: Dependency scan remains clean
- GIVEN the Molten dependency manifest and lockfile
- WHEN the ChoRus-inspired facade is built
- THEN `chorus_lib` and ChoRus transports are absent unless a future Cairn change explicitly admits a licensed source import with separate non-authority gates.

### Requirement: Typed facade derives from admitted manifests
r[molten.choreography.typed_facade_from_manifest] Molten SHOULD expose or generate typed Rust choreography facades only from protocol manifests that have passed deterministic registry lowering, Trellis projectability, endpoint projection, and protocol install receipt binding.

#### Scenario: Projectable manifest emits facade
- GIVEN a finite protocol manifest with roles, labels, payload schemas, policy refs, capability refs, and resource refs
- WHEN installation passes Trellis projectability and emits endpoint refs
- THEN the typed facade binds those manifest, registry, endpoint, and install receipt refs before use.

#### Scenario: Rejected manifest emits no facade
- GIVEN a manifest whose lowered Trellis global choreography is not projectable
- WHEN Molten attempts to expose a typed facade for that manifest
- THEN facade generation denies and records diagnostics instead of producing role APIs for an inadmissible protocol.

### Requirement: Facade operators are Sans-IO transition cores
r[molten.choreography.facade_epp_as_di] ChoRus-inspired facade operators MUST follow an EPP-as-dependency-injection shape in which the injected implementation is a deterministic Sans-IO transition core returning explicit local-computation, send, receive, branch, offer, diagnostic, receipt-input, and next-state outputs without filesystem, network, store, clock, random, async runtime, tracing, stdout/stderr, or receipt persistence effects.

#### Scenario: Send returns descriptor, not effect
- GIVEN a projected endpoint whose next action is a send
- WHEN the typed facade operator evaluates the send
- THEN it returns a canonical protocol-message descriptor and receipt input facts for the shell to admit rather than sending through a transport.

#### Scenario: Denial has no shell intent
- GIVEN a facade operation is malformed, stale, out of step, or missing required evidence
- WHEN the Sans-IO core evaluates it
- THEN it returns a deny decision with diagnostics and no committed state delta or side-effect intent.

### Requirement: Role-scoped payload access stays explicit
r[molten.choreography.role_scoped_payloads] Typed choreography facades MUST preserve role-scoped payload ownership by making wrong-role unwraps unrepresentable where Rust types can express that fact and by denying wrong-role, wrong-peer, wrong-label, wrong-payload-tag, stale-sequence, or missing-evidence access at dynamic boundaries before actor delivery or shell side effects.

#### Scenario: Local role can access owned payload
- GIVEN a typed facade value located at the local role and a matching projected local endpoint step
- WHEN the local role evaluates a permitted local computation
- THEN the facade exposes the payload only for that role and records the canonical input ref used by the transition.

#### Scenario: Wrong role cannot unwrap payload
- GIVEN a payload value located at one protocol role
- WHEN another role or external message attempts to unwrap, consume, or claim that payload without a matching projected receive step and evidence
- THEN the facade or transition core rejects the access before actor delivery.

### Requirement: Facade runner and projection preserve parity
r[molten.choreography.facade_runner_projection_parity] A ChoRus-inspired in-memory facade runner MUST evaluate the same Trellis-projected endpoint states and canonical transition refs as the dataspace-backed endpoint interpreter so positive fixtures can compare runner and projected execution without logs, clocks, live transport, or ambient state.

#### Scenario: Runner and projected interpreter agree
- GIVEN a projectable request/response protocol and the same canonical payload refs, authority facts, resource facts, and replay facts
- WHEN the in-memory runner and projected endpoint interpreter execute the admitted trace
- THEN they produce the same message refs, before-state refs, after-state refs, branch refs, operation decisions, and terminal-state refs.

#### Scenario: Runner cannot mask missing evidence
- GIVEN the projected interpreter would deny a transition for missing authority, policy, resource, or replay evidence
- WHEN the in-memory runner evaluates the same transition inputs
- THEN it also denies and reports the missing evidence instead of treating the test context as authority.

### Requirement: ChoRus transports and JSON identity are not adopted
r[molten.choreography.no_chorus_transport_adoption] Molten MUST NOT adopt ChoRus local blocking queues, ChoRus HTTP transport, ChoRus retry behavior, or serde_json payload identity for protocol semantics; all runtime protocol traffic MUST remain canonical Molten protocol-message records with Preserves identity, BLAKE3 refs, and carrier-specific evidence kept separate.

#### Scenario: Local fixture uses protocol records
- GIVEN a typed facade local fixture sends a payload between roles
- WHEN the fixture records the transition
- THEN the authoritative identity is the canonical protocol-message record ref, not a queue item, HTTP request, or JSON string.

#### Scenario: Carrier cannot define semantics
- GIVEN the same canonical protocol-message record is carried through local dataspace or remote Iroh evidence
- WHEN endpoint interpretation evaluates it
- THEN the result depends on protocol state and canonical message refs while carrier receipts remain separate evidence.

### Requirement: Facade generation emits non-claim receipts
r[molten.choreography.facade_codegen_receipts] Typed facade generation SHOULD emit deterministic generation receipts that bind the protocol manifest ref, install receipt ref, registry refs, projected endpoint refs, generator ref, generated artifact ref, and explicit non-claims that the facade grants no authority, policy admission, resource grant, provenance approval, transport trust, or ChoRus compatibility.

#### Scenario: Generated facade is auditable
- GIVEN a facade artifact generated for an admitted protocol
- WHEN an operator inspects its generation receipt
- THEN the receipt identifies the source manifest, install receipt, projected endpoints, generator, output artifact, and non-claim checks.

#### Scenario: Stale facade denies
- GIVEN a facade artifact generated from an older manifest or endpoint projection
- WHEN Molten evaluates it against a newer protocol install receipt
- THEN the mismatch is diagnosed and the stale facade cannot satisfy admission or gate requirements.

### Requirement: Facade tests cover pass and denial
r[molten.choreography.facade_positive_negative_tests] Molten MUST include positive tests for projectable generated typed facades and runner/projection parity, and negative tests for non-projectable manifests, wrong roles, wrong labels, wrong payload tags, missing evidence, stale or replayed operations, ChoRus dependency drift, and attempts to use ChoRus transports or JSON identity as protocol evidence.

#### Scenario: Happy-path facade reaches terminal state
- GIVEN a generated typed facade for a projectable two-role workflow
- WHEN admitted send, receive, and terminal operations execute through the in-memory runner and projected interpreter
- THEN both paths reach terminal state and record passing canonical operation evidence.

#### Scenario: Invalid facade inputs fail closed
- GIVEN a generated typed facade receives malformed, wrong-role, wrong-label, wrong-payload, missing-evidence, stale, or replayed input
- WHEN the transition core evaluates the input
- THEN the operation denies before shell effects and records diagnostics for the failed condition.

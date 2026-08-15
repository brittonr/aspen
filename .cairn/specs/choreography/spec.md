# Choreography Specification

## Purpose

Defines the `choreography` capability.

## Requirements

### Requirement: Trellis-backed choreography core
r[molten.choreography.trellis_core] Molten MUST use Trellis finite global choreography, local choreography, projectability, endpoint projection, and one-step semantics as the authoritative choreography core for protocol manifests.

#### Scenario: Global protocol projects to local endpoint
- GIVEN a finite Molten protocol lowered to a Trellis global choreography
- WHEN the choreography passes Trellis projectability admission
- THEN Molten derives a Trellis local endpoint for each declared role.

### Requirement: Protocol manifest model
r[molten.choreography.manifest] Molten MUST define a protocol manifest model naming protocol id, roles, labels, payload schemas, global choreography, policy refs, capability refs, and resource refs before lowering to Trellis ids.

#### Scenario: Manifest names protocol surface
- GIVEN a protocol manifest for a client, worker, and auditor workflow
- WHEN Molten parses the manifest
- THEN every role, label, payload schema, global step, policy ref, capability ref, and resource ref needed to compile the artifact is explicit.

### Requirement: Deterministic choreography id registries
r[molten.choreography.id_registry] Molten MUST deterministically map manifest role names, label names, and payload declarations to Trellis role ids, label ids, and payload tags.

#### Scenario: Same manifest lowers to same ids
- GIVEN two equivalent copies of the same protocol manifest
- WHEN Molten lowers each copy to a protocol installation artifact
- THEN both artifacts use the same role ids, label ids, payload tags, and protocol hash.

### Requirement: Manifest compiler
r[molten.choreography.compiler] Molten MUST compile valid finite protocol manifests into Trellis global choreography artifacts plus metadata for role maps, label maps, payload registries, protocol hashes, and policy references.

#### Scenario: Compiler emits Trellis artifact
- GIVEN a valid manifest with a finite send and branch workflow
- WHEN Molten compiles the manifest
- THEN the output includes the lowered Trellis global choreography and enough metadata to inspect original role, label, and payload names.

### Requirement: ChoRus is non-normative
r[molten.choreography.no_chorus_contract] Molten MUST NOT depend on ChoRus or `chorus_lib` for protocol semantics, admission, projection, runtime execution, or evidence validation.

#### Scenario: Choreography semantics do not require ChoRus
- GIVEN the Molten dependency manifest and protocol implementation
- WHEN a developer inspects the authoritative choreography path
- THEN Trellis primitives and Molten receipts define protocol semantics without a ChoRus runtime dependency.

### Requirement: Trellis projectability gate
r[molten.choreography.projectability_gate] Molten MUST reject protocol installation unless the lowered Trellis global choreography passes Trellis projectability admission.

#### Scenario: Non-projectable manifest is rejected
- GIVEN a manifest whose lowered global choreography cannot be projected consistently for participating roles
- WHEN Molten attempts to install the protocol
- THEN installation emits a denial receipt before endpoint runtime state or dataspace subscription state is admitted.

### Requirement: Endpoint projection state
r[molten.choreography.endpoint_projection] Molten MUST project admitted protocols to per-role Trellis local endpoints and expose each endpoint's next expected local action for runtime dispatch and inspection.

#### Scenario: Actor sees next local action
- GIVEN an admitted protocol session for a local worker role
- WHEN the runtime inspects the projected endpoint
- THEN it reports whether the worker is expected to send, receive, choose, offer, or end.

### Requirement: Protocol installation receipt
r[molten.choreography.installation_receipt] Molten MUST record protocol installation receipts that bind manifest content, Trellis admission decision, role map, label map, payload registry, projected endpoint refs, policy refs, capability refs, and resource refs.

#### Scenario: Installed protocol is inspectable
- GIVEN an admitted protocol installation
- WHEN an operator inspects the protocol artifact
- THEN the inspection can recover the manifest hash, Trellis admission decision, registries, endpoint refs, and receipt reference.

### Requirement: Payload registry validation
r[molten.choreography.payload_registry] Molten MUST validate protocol payload tags against declared payload schemas and canonical Preserves body-or-reference encoding before delivering a protocol message to an endpoint interpreter.

#### Scenario: Payload tag mismatch is rejected
- GIVEN a protocol message with a payload tag that does not match the declared current local step
- WHEN the endpoint interpreter attempts to consume the message
- THEN the message is rejected before actor delivery.

### Requirement: Dataspace local interpreter
r[molten.choreography.local_interpreter] Molten MUST provide a local endpoint interpreter for Trellis local endpoints that advances over canonical Molten protocol messages rather than ad hoc send/receive calls.

#### Scenario: Local interpreter advances endpoint
- GIVEN a projected local endpoint whose next step is a receive from a peer
- WHEN a matching admitted protocol-message record is available
- THEN the interpreter consumes the message and advances the endpoint to the next local state.

### Requirement: Protocol-message envelope
r[molten.choreography.protocol_envelope] Molten MUST represent each protocol runtime message as a canonical record carrying protocol id/ref, session id, from role, to role, label, payload tag, operation sequence, body or content ref, and evidence refs.

#### Scenario: Envelope identifies protocol step
- GIVEN a protocol-message envelope published into the runtime
- WHEN the runtime routes the envelope
- THEN it can match the envelope to protocol id, session id, sender role, receiver role, label, payload tag, sequence, and local endpoint step.

### Requirement: Send and receive transitions
r[molten.choreography.send_receive] Molten MUST implement local send and receive transitions by publishing or consuming matching protocol-message records through admitted runtime operations.

#### Scenario: Send publishes admitted protocol message
- GIVEN a projected endpoint whose next action is a send to another role
- WHEN the local actor provides a payload matching the expected payload tag with required admission evidence
- THEN the runtime records one matching protocol-message record, emits receipt evidence, and advances the local endpoint state.

#### Scenario: Receive consumes matching protocol message
- GIVEN a projected endpoint whose next action is a receive from another role
- WHEN a matching admitted protocol-message record is available
- THEN the runtime validates the record, records receipt evidence, and advances the local endpoint state.

### Requirement: Branching transitions
r[molten.choreography.branching] Molten MUST implement Trellis internal choice and offer transitions with explicit selected-label evidence that decider and non-decider roles validate before advancing.

#### Scenario: Decider records branch choice
- GIVEN a projected endpoint whose next action is an internal choice
- WHEN the decider selects an admitted branch label
- THEN the runtime records selected-label evidence and advances the decider endpoint to the selected branch.

#### Scenario: Non-decider accepts offered branch
- GIVEN a projected endpoint whose next action is an offer from a decider
- WHEN selected-label evidence for an available branch is observed
- THEN the runtime advances the non-decider endpoint to the corresponding branch body.

### Requirement: Session endpoint state
r[molten.choreography.session_state] Molten MUST track protocol-session endpoint state and reject messages or branch decisions inconsistent with the current projected local step.

#### Scenario: Out-of-step message is rejected
- GIVEN a session endpoint currently waiting for label `approved` from the worker role
- WHEN the runtime sees a message with a different label or sender for the same session
- THEN the endpoint interpreter rejects that message for the current step.

### Requirement: Sequence and replay admission
r[molten.choreography.sequence_replay] Molten MUST apply bounded sequence or replay checks to protocol-message records before delivery to endpoint interpreters.

#### Scenario: Duplicate operation is rejected
- GIVEN a protocol session has already consumed a protocol message ref
- WHEN the same message is observed again for that session
- THEN the runtime rejects the duplicate before actor delivery.

### Requirement: Choreography policy boundary
r[molten.choreography.policy_boundary] Molten MUST route protocol installation, send, receive, branch choice, and externally-carried protocol delivery through explicit policy, authority, resource, carrier, or receipt gates before side effects are committed.

#### Scenario: Missing send capability denies transition
- GIVEN a projected endpoint whose next action is a send
- WHEN the local actor lacks required authority or resource evidence
- THEN the runtime denies the send before publishing any protocol message.

### Requirement: Cairn choreography receipts
r[molten.choreography.cairn_receipts] Molten MUST validate choreography installation, operation, and session lifecycle receipts through canonical parsers and lifecycle gates before treating those receipts as evidence for later admission or inspection.

#### Scenario: Invalid operation receipt is excluded
- GIVEN a protocol-message envelope references a malformed operation receipt
- WHEN the runtime evaluates the envelope's evidence
- THEN the malformed receipt is excluded and cannot satisfy admission requirements.

### Requirement: Dataspace choreography observability
r[molten.choreography.dataspace_observability] Molten MUST emit or classify structured protocol evidence for installation and endpoint transitions including protocol id/ref, session id, local role, transition kind, policy decision, and receipt reference.

#### Scenario: Transition emits trace event
- GIVEN an endpoint interpreter successfully advances a send or receive step
- WHEN evidence is recorded
- THEN the record identifies protocol id/ref, session id, role, transition, decision, and receipt reference.

### Requirement: Transport-neutral protocol messages
r[molten.choreography.remote_ready] Protocol-message records MUST remain transport-neutral so local runtime delivery and remote dataspace/Iroh carrier envelopes can share the same choreography semantics.

#### Scenario: Remote bridge preserves protocol envelope
- GIVEN a protocol-message record that is valid in the local runtime
- WHEN a remote dataspace carrier transports it to a peer
- THEN the receiving runtime validates the same protocol id, session id, roles, label, payload tag, payload body/ref, and evidence before local delivery.

### Requirement: Choreography integration tests
r[molten.choreography.integration_tests] Molten MUST include tests for manifest lowering, projectability rejection, endpoint projection, local send/receive, branch offer, replay rejection, transport-neutral delivery, receipt classification, and lifecycle gate validation.

#### Scenario: Local workflow completes
- GIVEN a client/server protocol manifest with sends and branch coverage
- WHEN the local protocol interpreter runs projected endpoints with admitted evidence
- THEN endpoints reach terminal state and the runtime records send, receive, branch, offer, and receipt evidence.

### Requirement: Choreography property tests
r[molten.choreography.property_tests] Molten SHOULD use bounded Hegel property tests for generated finite protocol manifests to check projection, endpoint-state, and interpreter invariants beyond hand-written examples.

#### Scenario: Generated projectable protocols preserve endpoint progress
- GIVEN a generated finite projectable choreography within supported bounds
- WHEN Molten lowers, projects, and steps matching endpoints through the local interpreter model
- THEN endpoint state advances only according to the projected local choreography.

### Requirement: Protocol installation is Trellis-gated
r[molten.trellis_protocol_session.spec.install_gate] A protocol manifest MUST be installed only after deterministic lowering to Trellis, projectability validation, endpoint projection, and canonical installation receipt emission.

#### Scenario: Projectable protocol installs
- GIVEN a finite protocol manifest with roles, labels, payload schemas, and policy refs
- WHEN Molten installs the protocol
- THEN Trellis projectability passes
- AND a protocol installation receipt binds the manifest, registries, endpoint refs, and checks

#### Scenario: Non-projectable protocol denies
- GIVEN a protocol manifest that Trellis rejects as non-projectable
- WHEN installation is attempted
- THEN Molten emits a denial receipt
- AND no endpoint/session state is admitted

### Requirement: Session messages follow projected local state
r[molten.trellis_protocol_session.spec.endpoint_state] Protocol send, receive, choice, and offer operations MUST match the current projected local endpoint state before any dataspace message is committed.

#### Scenario: Expected send commits
- GIVEN a session endpoint whose local state expects a send with label `request`
- WHEN the actor sends a matching protocol message with a valid payload tag
- THEN the local endpoint advances
- AND the operation receipt binds the prior state, message ref, and next state

#### Scenario: Wrong label denies
- GIVEN a session endpoint whose local state expects label `request`
- WHEN a message uses label `response`
- THEN the operation denies before publishing the message

### Requirement: Protocol traffic remains transport-neutral
r[molten.trellis_protocol_session.spec.transport_neutral] Protocol message semantics MUST be defined by canonical protocol records and endpoint state, not by Iroh, local channel, or any carrier transport.

#### Scenario: Same message over local and remote carrier
- GIVEN the same canonical protocol message bytes
- WHEN they are delivered over a local dataspace and over a remote dataspace envelope
- THEN endpoint interpretation yields the same protocol operation result
- AND carrier-specific receipts remain separate evidence refs

### Requirement: Protocol lifecycle gates replay session evidence
r[molten.trellis_protocol_session.spec.lifecycle_gate] Protocol lifecycle gate receipts MUST replay install and operation receipts against canonical endpoint state, message evidence, and terminal state refs before accepting a completed session lifecycle.

#### Scenario: Valid lifecycle gates
- GIVEN a protocol install receipt, initial endpoint states, protocol messages, operation receipts, and terminal next states from a completed request/response session
- WHEN Molten gates the lifecycle
- THEN it emits a passing protocol session gate receipt
- AND the receipt binds install, protocol, session ids, state refs, operation refs, message refs, diagnostics, and non-authority checks

#### Scenario: Missing terminal evidence denies
- GIVEN the same lifecycle evidence with a required next state removed
- WHEN Molten gates the lifecycle
- THEN it emits a denial receipt
- AND diagnostics identify missing replay or terminal-state evidence

### Requirement: Protocol endpoint transitions are legal for projected state
r[molten.protocol_state_machine_proof.endpoint_transition_legality] Molten MUST prove that protocol operation receipts for send, receive, branch, and offer operations are accepted only when the operation matches the projected local endpoint state, peer, label, prior state, and next state.

#### Scenario: Wrong branch label denies
- GIVEN a projected endpoint state with a bounded set of legal branch labels
- WHEN a protocol operation receipt uses a label that is not legal for the projected state
- THEN the operation receipt decision is `deny`
- AND diagnostics identify the missing or ambiguous branch transition.

### Requirement: Protocol lifecycle replay is complete
r[molten.protocol_state_machine_proof.lifecycle_replay_completeness] Molten MUST prove that protocol lifecycle gate receipts replay install and operation receipts against canonical endpoint states, message evidence, and terminal state refs before accepting a completed session lifecycle.

#### Scenario: Missing terminal state denies lifecycle gate
- GIVEN protocol operation evidence with a required terminal next-state ref removed
- WHEN Molten evaluates the protocol lifecycle gate
- THEN the gate receipt decision is `deny`
- AND diagnostics identify missing terminal or replay evidence.

### Requirement: Generated protocol session traces preserve projection invariants
r[molten.protocol_state_machine_proof.generated_session_traces] Molten SHOULD include bounded generated or fixture-derived protocol session traces that cover linear send/receive and branch/offer paths while preserving projected endpoint transition invariants.

#### Scenario: Generated branch trace reaches terminal state
- GIVEN a bounded projected protocol with a branch or offer path
- WHEN Molten replays a generated legal session trace
- THEN every operation receipt passes
- AND the lifecycle gate reaches the expected terminal state refs.

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

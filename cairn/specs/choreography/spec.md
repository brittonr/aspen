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

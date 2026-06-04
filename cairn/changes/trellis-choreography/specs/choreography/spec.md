## ADDED Requirements

### Requirement: Trellis-backed choreography core
r[molten.choreography.trellis_core] The system MUST use Trellis global choreography, local choreography, projectability, endpoint projection, and one-step semantics as the authoritative finite choreography core for Molten protocols.

#### Scenario: Global protocol projects to local endpoint
r[molten.choreography.trellis_core.project]
- GIVEN a finite Molten protocol lowered to a Trellis global choreography
- WHEN the choreography is admitted as projectable by Trellis
- THEN Molten can derive a Trellis local choreography endpoint for each declared role

### Requirement: Protocol manifest model
r[molten.choreography.manifest] The system MUST define a protocol manifest model that names the protocol id, roles, labels, payload schemas, global choreography, and policy references before lowering to Trellis ids.

#### Scenario: Manifest names protocol surface
r[molten.choreography.manifest.names]
- GIVEN a protocol manifest for a client, worker, and auditor workflow
- WHEN Molten loads the manifest
- THEN the manifest identifies every role, label, payload schema, global step, and policy reference needed to compile the protocol artifact

### Requirement: Deterministic choreography id registries
r[molten.choreography.id_registry] The system MUST deterministically map manifest role names, label names, and payload declarations to Trellis `RoleId`, `LabelId`, and `PayloadTag` values.

#### Scenario: Same manifest lowers to same ids
r[molten.choreography.id_registry.stable]
- GIVEN two equivalent copies of the same protocol manifest
- WHEN Molten lowers each copy to a Trellis choreography artifact
- THEN both artifacts use the same role ids, label ids, payload tags, and protocol hash

### Requirement: Manifest compiler
r[molten.choreography.compiler] The system MUST compile valid protocol manifests into Trellis `GlobalChoreo` artifacts plus metadata for role maps, label maps, payload registries, protocol hashes, and policy references.

#### Scenario: Compiler emits Trellis artifact
r[molten.choreography.compiler.emit]
- GIVEN a valid manifest with a finite send and branch workflow
- WHEN Molten compiles the manifest
- THEN the output includes the lowered Trellis global choreography and enough metadata to inspect the original role, label, and payload names

### Requirement: ChoRus is non-normative
r[molten.choreography.no_chorus_contract] The system MUST NOT depend on ChoRus or `chorus_lib` for Molten protocol semantics, admission, projection, or runtime execution.

#### Scenario: Choreography semantics do not require ChoRus
r[molten.choreography.no_chorus_contract.no_dependency]
- GIVEN the Molten dependency manifest and protocol implementation
- WHEN a developer inspects the authoritative choreography path
- THEN Trellis primitives define the protocol semantics and no ChoRus runtime dependency is required

### Requirement: Trellis projectability gate
r[molten.choreography.projectability_gate] The system MUST reject protocol installation unless the lowered Trellis global choreography passes Trellis projectability admission.

#### Scenario: Non-projectable manifest is rejected
r[molten.choreography.projectability_gate.reject]
- GIVEN a manifest whose lowered global choreography cannot be projected consistently for all participating roles
- WHEN Molten attempts to install the protocol
- THEN installation fails before any endpoint runtime state or dataspace subscription is created

### Requirement: Endpoint projection state
r[molten.choreography.endpoint_projection] The system MUST project admitted protocols to per-role Trellis `LocalChoreo` endpoints and expose each endpoint's next expected local action for runtime dispatch and inspection.

#### Scenario: Actor sees next local action
r[molten.choreography.endpoint_projection.next_action]
- GIVEN an admitted protocol session for a local worker role
- WHEN the runtime inspects the projected endpoint
- THEN it reports whether the worker is currently expected to send, receive, choose, offer, or end

### Requirement: Protocol installation receipt
r[molten.choreography.installation_receipt] The system MUST record a protocol installation receipt that binds the manifest hash, lowered Trellis artifact, Trellis admission result, role map, label map, payload registry, and policy references.

#### Scenario: Installed protocol is inspectable
r[molten.choreography.installation_receipt.inspect]
- GIVEN an admitted protocol installation
- WHEN an operator inspects the protocol artifact
- THEN the inspection can recover the manifest hash, Trellis admission decision, role and label maps, payload registry, and receipt reference

### Requirement: Payload registry validation
r[molten.choreography.payload_registry] The system MUST validate protocol payload tags against declared schemas and canonical Preserves or content-reference encoding rules before delivering a protocol message to an endpoint interpreter.

#### Scenario: Payload tag mismatch is rejected
r[molten.choreography.payload_registry.mismatch]
- GIVEN a protocol message with a payload tag that does not match the declared schema for the current local step
- WHEN the endpoint interpreter attempts to consume the message
- THEN the message is rejected before actor delivery

### Requirement: Dataspace local interpreter
r[molten.choreography.local_interpreter] The system MUST provide a local endpoint interpreter for Trellis `LocalChoreo` that executes over the Molten dataspace adapter rather than direct ad hoc send/receive calls.

#### Scenario: Local interpreter advances endpoint
r[molten.choreography.local_interpreter.advance]
- GIVEN a projected local endpoint whose next step is a receive from a peer
- WHEN the dataspace provides a matching admitted protocol-message envelope
- THEN the interpreter consumes the message and advances the endpoint to the next Trellis local state

### Requirement: Protocol-message envelope
r[molten.choreography.protocol_envelope] The system MUST represent each protocol runtime message as a Molten envelope carrying protocol id, session id, from role, to role, label, payload tag, operation index or effect id, payload body or content reference, and evidence references.

#### Scenario: Envelope identifies protocol step
r[molten.choreography.protocol_envelope.identifies_step]
- GIVEN a protocol-message envelope published into the dataspace
- WHEN the runtime routes the envelope
- THEN it can match the envelope to a protocol id, session id, expected sender role, expected receiver role, label, payload tag, and local endpoint step

### Requirement: Send and receive transitions
r[molten.choreography.send_receive] The system MUST implement Trellis local send and receive transitions by publishing or consuming matching protocol-message envelopes through admitted dataspace operations.

#### Scenario: Send publishes admitted protocol message
r[molten.choreography.send_receive.send]
- GIVEN a projected endpoint whose next action is a send to another role
- WHEN the local actor provides a payload matching the expected payload tag
- THEN the runtime admits the side effect, publishes one matching protocol-message envelope, records receipt evidence, and advances the local endpoint state

#### Scenario: Receive consumes matching protocol message
r[molten.choreography.send_receive.receive]
- GIVEN a projected endpoint whose next action is a receive from another role
- WHEN a matching admitted protocol-message envelope is available in the dataspace
- THEN the runtime validates the envelope, delivers the payload to the actor, records receipt evidence, and advances the local endpoint state

### Requirement: Branching transitions
r[molten.choreography.branching] The system MUST implement Trellis internal choice and offer transitions with explicit selected-label evidence that non-decider roles can validate before advancing.

#### Scenario: Decider records branch choice
r[molten.choreography.branching.decider]
- GIVEN a projected endpoint whose next action is an internal choice
- WHEN the decider selects an admitted branch label
- THEN the runtime records selected-label evidence and advances the decider endpoint to the selected branch

#### Scenario: Non-decider accepts offered branch
r[molten.choreography.branching.offer]
- GIVEN a projected endpoint whose next action is an offer from a decider
- WHEN selected-label evidence for an available branch is observed
- THEN the runtime advances the non-decider endpoint to the corresponding branch body

### Requirement: Session endpoint state
r[molten.choreography.session_state] The system MUST track protocol-session endpoint state and reject messages or branch decisions inconsistent with the current projected local step.

#### Scenario: Out-of-step message is rejected
r[molten.choreography.session_state.out_of_step]
- GIVEN a session endpoint currently waiting for label `approved` from the worker role
- WHEN the dataspace contains a message with a different label or sender for the same session
- THEN the endpoint interpreter rejects that message for the current step

### Requirement: Sequence and replay admission
r[molten.choreography.sequence_replay] The system MUST apply bounded sequence or replay checks to protocol-message envelopes before delivery to endpoint interpreters.

#### Scenario: Duplicate operation is rejected
r[molten.choreography.sequence_replay.duplicate]
- GIVEN a protocol session that has already consumed operation index 4 from a peer
- WHEN the same operation index is observed again for the same protocol id, session id, sender, receiver, label, and payload tag
- THEN the runtime rejects the duplicate before actor delivery

### Requirement: Choreography policy boundary
r[molten.choreography.policy_boundary] The system MUST route protocol installation, send, receive, branch choice, and external effects through Basalt, Nickel, Steel, Trellis, or Cairn policy gates as applicable before side effects occur.

#### Scenario: Missing send capability denies transition
r[molten.choreography.policy_boundary.denied_send]
- GIVEN a projected endpoint whose next action is a send
- WHEN the local actor lacks the required capability or contract admission
- THEN the runtime denies the send before publishing any dataspace assertion

### Requirement: Cairn choreography receipts
r[molten.choreography.cairn_receipts] The system MUST validate choreography installation receipts and per-operation receipts through Cairn before treating those receipts as evidence for later admission or inspection.

#### Scenario: Invalid operation receipt is excluded
r[molten.choreography.cairn_receipts.invalid]
- GIVEN a protocol-message envelope that references a malformed operation receipt
- WHEN the runtime evaluates the envelope's evidence
- THEN the malformed receipt is excluded and cannot satisfy admission requirements

### Requirement: Dataspace choreography observability
r[molten.choreography.dataspace_observability] The system MUST emit structured tracing events for choreography installation and endpoint transitions including protocol id, session id, local role, transition kind, policy decision, and receipt reference.

#### Scenario: Transition emits trace event
r[molten.choreography.dataspace_observability.transition]
- GIVEN an endpoint interpreter successfully advances a send or receive step
- WHEN tracing is enabled
- THEN the runtime emits a structured event identifying the protocol id, session id, role, transition, decision, and receipt reference

### Requirement: Transport-neutral protocol messages
r[molten.choreography.remote_ready] Protocol-message envelopes MUST remain transport-neutral so local dataspace, Iroh gossip, Iroh blobs, Iroh docs, native actors, Wasmtime actors, and Steel orchestration can share the same choreography semantics.

#### Scenario: Remote bridge preserves protocol envelope
r[molten.choreography.remote_ready.bridge]
- GIVEN a protocol-message envelope that is valid in the local dataspace
- WHEN the Iroh bridge transports it to a peer
- THEN the receiving runtime validates the same protocol id, session id, roles, label, payload tag, payload reference, and evidence before local delivery

### Requirement: Choreography integration tests
r[molten.choreography.integration_tests] The system MUST include integration tests for manifest lowering, projectability rejection, endpoint projection, local send/receive, branch offer, replay rejection, and receipt validation.

#### Scenario: Local three-role workflow completes
r[molten.choreography.integration_tests.three_role]
- GIVEN a client, worker, and auditor protocol manifest with one branch
- WHEN the local dataspace interpreter runs all three projected endpoints with admitted capabilities
- THEN each endpoint reaches `End` and the runtime records send, receive, branch, and receipt evidence

### Requirement: Choreography property tests
r[molten.choreography.property_tests] The system MUST use Hegel property-based tests for generated finite choreography manifests to check projection, endpoint-state, and interpreter invariants beyond hand-written examples.

#### Scenario: Generated projectable protocols preserve endpoint progress
r[molten.choreography.property_tests.generated_progress]
- GIVEN a generated finite projectable choreography within supported bounds
- WHEN Molten lowers, projects, and steps matching endpoints through the local interpreter model
- THEN endpoint state advances only according to the projected Trellis local choreography

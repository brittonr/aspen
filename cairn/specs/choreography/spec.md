# Choreography Specification

## Purpose

Defines the `choreography` capability.

## Requirements

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



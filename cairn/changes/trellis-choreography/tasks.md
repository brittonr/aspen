## Phase 1: Protocol artifact and Trellis lowering

- [ ] [serial] r[molten.choreography.trellis_core] Use Trellis `GlobalChoreo`, `LocalChoreo`, projectability, projection, and step semantics as the authoritative choreography core.
- [ ] [serial] r[molten.choreography.manifest] Define a protocol manifest model with protocol id, named roles, labels, payload schemas, global choreography, and policy references.
- [ ] [serial] r[molten.choreography.id_registry] Add deterministic role, label, and payload registries that lower manifest names to Trellis `RoleId`, `LabelId`, and `PayloadTag` values.
- [ ] [serial] r[molten.choreography.compiler] Compile valid manifests into Trellis `GlobalChoreo` artifacts with stable manifest/protocol hashes.
- [ ] [parallel] r[molten.choreography.no_chorus_contract] Keep ChoRus as non-normative API inspiration only; do not depend on `chorus_lib` for Molten protocol semantics.

## Phase 2: Admission and endpoint projection

- [ ] [serial] r[molten.choreography.projectability_gate] Reject protocol installation unless Trellis admits the lowered global choreography as projectable.
- [ ] [serial] r[molten.choreography.endpoint_projection] Project admitted protocols to per-role Trellis `LocalChoreo` endpoints and expose the projected next-action state.
- [ ] [parallel] r[molten.choreography.installation_receipt] Record protocol installation receipts that include manifest hash, Trellis admission result, role map, label map, payload registry, and policy references.
- [ ] [parallel] r[molten.choreography.payload_registry] Validate payload tags against declared schemas and canonical Preserves/body-or-blob encoding rules.

## Phase 3: Dataspace interpreter

- [ ] [serial] r[molten.choreography.local_interpreter] Add an endpoint interpreter for Trellis `LocalChoreo` over the Molten local dataspace.
- [ ] [serial] r[molten.choreography.protocol_envelope] Define protocol-message envelope fields for protocol id, session id, roles, labels, payload tag, op index, body or content ref, and evidence refs.
- [ ] [serial] r[molten.choreography.send_receive] Implement send and receive transitions by publishing or consuming matching protocol-message envelopes through admitted dataspace operations.
- [ ] [serial] r[molten.choreography.branching] Implement internal choice and offer transitions with explicit selected-label evidence.
- [ ] [parallel] r[molten.choreography.session_state] Track session-local projected endpoint state and reject messages inconsistent with the current local step.
- [ ] [parallel] r[molten.choreography.sequence_replay] Add bounded sequence/replay checks for protocol messages before delivery to endpoint interpreters.

## Phase 4: Policy, evidence, and integration tests

- [ ] [serial] r[molten.choreography.policy_boundary] Route protocol installation, send, receive, branch choice, and external effects through Basalt/Nickel/Steel/Trellis policy gates before side effects.
- [ ] [serial] r[molten.choreography.cairn_receipts] Validate choreography installation and per-operation receipts through Cairn before treating them as evidence.
- [ ] [parallel] r[molten.choreography.dataspace_observability] Emit tracing events for protocol id, session id, local role, transition kind, policy decision, and receipt reference.
- [ ] [parallel] r[molten.choreography.remote_ready] Keep protocol-message envelopes transport-neutral so Iroh gossip/blobs/docs can bridge them without changing choreography semantics.
- [ ] [serial] r[molten.choreography.integration_tests] Add tests for manifest lowering, projectability rejection, endpoint projection, local send/receive, branch offer, replay rejection, and receipt validation.
- [ ] [parallel] r[molten.choreography.property_tests] Add Hegel property tests for generated finite protocols that preserve projection and interpreter invariants.

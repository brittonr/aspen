## Phase 1: Protocol artifact and Trellis lowering

- [x] [serial] r[molten.choreography.trellis_core] Use Trellis `GlobalChoreo`, `LocalChoreo`, projectability, projection, and step semantics as the authoritative finite choreography core.
- [x] [serial] r[molten.choreography.manifest] Define a finite protocol manifest model with protocol id, named roles, labels, payload schemas, global choreography, and policy references.
- [x] [serial] r[molten.choreography.id_registry] Add deterministic role, label, and payload registries that lower manifest names to Trellis ids and payload tags.
- [x] [serial] r[molten.choreography.compiler] Compile valid manifests into Trellis global choreography artifacts with stable manifest/protocol hashes.
- [x] [parallel] r[molten.choreography.no_chorus_contract] Keep ChoRus as non-normative API inspiration only; do not depend on `chorus_lib` for Molten protocol semantics.

## Phase 2: Admission and endpoint projection

- [x] [serial] r[molten.choreography.projectability_gate] Reject protocol installation unless Trellis admits the lowered global choreography as projectable.
- [x] [serial] r[molten.choreography.endpoint_projection] Project admitted protocols to per-role Trellis local endpoints and expose the projected next-action state.
- [x] [parallel] r[molten.choreography.installation_receipt] Record protocol installation receipts that include manifest content, Trellis admission result, registries, endpoint refs, and policy refs.
- [x] [parallel] r[molten.choreography.payload_registry] Validate payload tags against declared schemas and canonical Preserves/body-or-ref encoding rules.

## Phase 3: Dataspace interpreter

- [x] [serial] r[molten.choreography.local_interpreter] Add a local endpoint interpreter for Trellis projected endpoint state over canonical Molten protocol messages.
- [x] [serial] r[molten.choreography.protocol_envelope] Define protocol-message fields for protocol id/ref, session id, roles, labels, payload tag, operation sequence, body or content ref, and evidence refs.
- [x] [serial] r[molten.choreography.send_receive] Implement send and receive transitions by recording or consuming matching protocol-message records through admitted operations.
- [x] [serial] r[molten.choreography.branching] Implement internal choice and offer transitions with explicit selected-label evidence.
- [x] [parallel] r[molten.choreography.session_state] Track session-local projected endpoint state and reject messages inconsistent with the current local step.
- [x] [parallel] r[molten.choreography.sequence_replay] Add bounded sequence/replay checks for protocol messages before delivery to endpoint interpreters.

## Phase 4: Policy, evidence, and integration tests

- [x] [serial] r[molten.choreography.policy_boundary] Route protocol installation, send, receive, branch choice, and carried delivery through explicit policy/authority/resource/carrier gates before side effects.
- [x] [serial] r[molten.choreography.cairn_receipts] Validate choreography installation and per-operation receipts through canonical parsers and lifecycle gate receipts before treating them as evidence.
- [x] [parallel] r[molten.choreography.dataspace_observability] Emit/classify evidence for protocol id/ref, session id, local role, transition kind, policy decision, and receipt reference.
- [x] [parallel] r[molten.choreography.remote_ready] Keep protocol-message records transport-neutral so local and remote dataspace carriers can share choreography semantics.
- [x] [serial] r[molten.choreography.integration_tests] Add tests for manifest lowering, projectability rejection, endpoint projection, local send/receive, branch offer, replay rejection, transport neutrality, and receipt validation.
- [x] [parallel] r[molten.choreography.property_tests] Add Hegel property tests for generated finite protocols that preserve projection and interpreter invariants.

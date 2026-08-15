## Phase 1: Manifest and Trellis lowering

- [x] [serial] r[molten.trellis_protocol_session.spec.install_gate] Define `protocol-manifest-v1` with roles, labels, payload schemas, global choreography, policy, capability, and resource refs.
- [x] [serial] r[molten.trellis_protocol_session.spec.install_gate] Implement deterministic role/label/payload registries that lower names to Trellis ids.
- [x] [serial] r[molten.trellis_protocol_session.spec.install_gate] Compile to Trellis `GlobalChoreo`, run projectability, and project local endpoints.
- [x] [parallel] r[molten.trellis_protocol_session.spec.install_gate] Emit `protocol-install-receipt-v1` binding manifest, registries, Trellis admission, endpoints, and policy refs.

## Phase 2: Session records and interpreter

- [x] [serial] r[molten.trellis_protocol_session.spec.endpoint_state] Define `protocol-endpoint-v1` and `protocol-session-state-v1` records with local Trellis state refs.
- [x] [serial] r[molten.trellis_protocol_session.spec.endpoint_state] Define `protocol-message-v1` as a canonical Molten envelope payload for protocol traffic.
- [x] [serial] r[molten.trellis_protocol_session.spec.endpoint_state] Implement send/receive/internal-choice/offer transitions over local dataspace operations.
- [x] [parallel] r[molten.trellis_protocol_session.spec.endpoint_state] Validate payload tags against declared schema/content refs before send or receive commits.

## Phase 3: Admission and replay

- [x] [serial] r[molten.trellis_protocol_session.spec.endpoint_state] Emit `protocol-operation-receipt-v1` for send, receive, branch, close, and denial.
- [x] [serial] r[molten.trellis_protocol_session.spec.endpoint_state] Add bounded sequence/replay windows for protocol messages before endpoint transition.
- [x] [parallel] r[molten.trellis_protocol_session.spec.endpoint_state] Gate install and per-operation actions through policy, authority, resource, and effect handles.
- [x] [parallel] r[molten.trellis_protocol_session.spec.transport_neutral] Carry protocol messages through remote dataspace envelopes without changing protocol semantics.

## Phase 4: Examples and tests

- [x] [serial] r[molten.trellis_protocol_session.spec.endpoint_state] Add a two-role request/response protocol example and CLI lifecycle.
- [x] [serial] r[molten.trellis_protocol_session.spec.endpoint_state] Test projectability rejection, endpoint projection, send/receive, branch offer, replay rejection, and receipt parsing.
- [x] [parallel] r[molten.trellis_protocol_session.spec.endpoint_state] Test wrong role, wrong label, wrong sequence, bad payload tag, stale endpoint state, and missing authority.
- [x] [parallel] r[molten.trellis_protocol_session.spec.endpoint_state] Add Hegel properties for generated finite protocols within supported Trellis bounds.

## Phase 5: Lifecycle gate receipts

- [x] [serial] r[molten.trellis_protocol_session.spec.lifecycle_gate] Define `protocol-session-gate-receipt-v1` with install/protocol/session/state/operation/message/final-state refs and non-authority checks.
- [x] [serial] r[molten.trellis_protocol_session.spec.lifecycle_gate] Replay the install receipt and passing operation receipts against canonical endpoint state before accepting a lifecycle.
- [x] [parallel] r[molten.trellis_protocol_session.spec.lifecycle_gate] Add `molten test protocol gate-lifecycle` CLI and receipt parsing/show support.
- [x] [parallel] r[molten.trellis_protocol_session.spec.lifecycle_gate] Cover passing request/response lifecycle gates and missing terminal evidence denial.

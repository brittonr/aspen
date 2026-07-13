## Phase 1: Canonical transport contract

- [x] [serial] Add canonical transport commands, events, ids, protocol descriptors, framing profiles, bounds, delivery semantics, and non-claims. r[molten.fabric_transport.port_contract]
- [x] [serial] Add pure validation for protocol identity, registration ownership, versions, generation routing, framing, credits, terminal events, and profile compatibility. r[molten.fabric_transport.protocol_registration] r[molten.fabric_transport.flow_control]
- [x] [parallel] Add positive descriptors and negative duplicate, unknown, stale-generation, malformed-frame, over-limit, and unsupported-capability fixtures. r[molten.fabric_transport.port_contract] r[molten.fabric_transport.protocol_registration]

## Phase 2: Protocol and session runtime

- [x] [serial] Integrate capability-gated protocol registration, activation, accept, drain, and cleanup with system-extension generations. r[molten.fabric_transport.protocol_registration]
- [x] [parallel] Route dial, accept, session, stream, message, datagram, close, failure, and cancellation events through the system-extension dispatcher. r[molten.fabric_transport.session_streams]
- [x] [parallel] Enforce bounded queues, frames, bytes, sessions, streams, credits, deadlines, and cancellation before callback delivery. r[molten.fabric_transport.flow_control]

## Phase 3: Adapters and identity

- [x] [serial] Implement the live Iroh transport adapter behind the canonical port without exposing adapter-owned handles to extension code. r[molten.fabric_transport.live_sim_parity]
- [x] [parallel] Implement the deterministic-simulation adapter against the same command and event contract. r[molten.fabric_transport.live_sim_parity]
- [x] [parallel] Separate transport peer identity refs from membership, application principal, capability, and trust decisions in events and admission checks. r[molten.fabric_transport.identity_separation]

## Phase 4: Failure semantics and evidence

- [x] [serial] Define disconnect, reset, timeout, partition, malformed input, local overload, remote refusal, cancellation, and uncertain-delivery outcomes without implicit retries. r[molten.fabric_transport.failure_semantics]
- [x] [parallel] Add bounded protocol/session lifecycle evidence and operator readback for registrations, active sessions, resource use, failures, and non-claims. r[molten.fabric_transport.evidence]

## Phase 5: Validation

- [x] [serial] Run shared adapter conformance, live loopback, deterministic simulation, malformed input, overload, cancellation, stale listener, drain, and cleanup tests. r[molten.fabric_transport.final_validation]
- [x] [serial] Run Cairn validation and proposal, design, and tasks gates before sync and archive. r[molten.fabric_transport.final_validation]

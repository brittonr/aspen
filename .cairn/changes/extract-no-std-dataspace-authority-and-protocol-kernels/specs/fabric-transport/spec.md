# Fabric transport deltas

## ADDED Requirements

### Requirement: Sans-I/O protocol and session state machines live in a no_std kernel
r[molten.fabric_transport.no_std_protocol_session_kernel] Protocol session transitions and sans-I/O protocol cores (`src/protocol/{session,sans/io}`) MUST live in a `#![no_std]` + `alloc` crate (`molten-protocol-session-core`) with zero socket, Tokio, or filesystem dependencies.

#### Scenario: Protocol session transitions run without std
- GIVEN a protocol session state and incoming canonical frame bytes
- WHEN `molten-protocol-session-core` steps the state machine under `#![no_std]`
- THEN it MUST emit deterministic next-state transitions and outbound frame buffers with zero live I/O.

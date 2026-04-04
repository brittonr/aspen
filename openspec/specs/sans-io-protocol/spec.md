## ADDED Requirements

### Requirement: Protocol context trait boundary

All new distributed protocol state machines MUST interact with the outside world exclusively through a protocol-specific context trait. The state machine struct MUST NOT import networking, storage, or async runtime types.

#### Scenario: Protocol sends a message

- **WHEN** a protocol state machine needs to send a message to a peer
- **THEN** it calls a method on its context trait (e.g., `ctx.send(to, msg)`) and the message is wrapped in an `Envelope` for the shell to drain and transmit

#### Scenario: Protocol reads persistent state

- **WHEN** a protocol state machine needs to read its persisted data
- **THEN** it calls `ctx.persistent_state()` which returns a reference to the state, with no disk I/O in the call path

#### Scenario: Protocol updates persistent state

- **WHEN** a protocol state machine modifies its persisted data
- **THEN** it calls `ctx.update_persistent_state(|s| { ... })` with a closure, and the shell decides when/how to flush to disk

### Requirement: In-memory test context

A `TestCtx` implementation MUST exist that records all side effects (sent messages, state mutations, alarms) in memory for deterministic test assertions.

#### Scenario: Test asserts on sent messages

- **WHEN** a protocol action causes messages to be sent via the context trait
- **THEN** the `TestCtx` stores them in a drainable vector and tests can assert on count, recipients, and message contents

#### Scenario: Test detects persistent state change

- **WHEN** a protocol action calls `update_persistent_state`
- **THEN** the `TestCtx` sets a dirty flag that tests can check and reset via `persistent_state_change_check_and_reset()`

### Requirement: Envelope-based message routing

Protocol messages MUST be wrapped in `Envelope<M> { to: NodeId, from: NodeId, msg: M }` so the shell layer can route them without understanding message contents.

#### Scenario: Multi-protocol node

- **WHEN** a node runs multiple sans-IO protocols simultaneously
- **THEN** each protocol's envelopes are typed differently and the shell dispatches incoming messages to the correct protocol by ALPN or message discriminant

### Requirement: Deterministic protocol execution

Protocol state machine methods MUST be synchronous and deterministic. No `.await`, no `OsRng` calls, no `SystemTime::now()`. Time and randomness are passed as explicit parameters.

#### Scenario: Reproducible test execution

- **WHEN** the same sequence of inputs is fed to a protocol state machine with the same initial state
- **THEN** the resulting state and outbound messages are identical across runs

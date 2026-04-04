## Context

Aspen's distributed protocols currently live in async handlers that directly call iroh networking and redb storage. Testing requires spinning up real endpoints or extensive mocking. Oxide's trust-quorum solves this with a `NodeHandlerCtx` trait — the protocol state machine calls trait methods to send messages, read/write persistent state, and query connections. Production wires these to real I/O; tests use an in-memory implementation.

Aspen already has the "verified functions" pattern for pure computations. This extends that idea to stateful protocol machines.

## Goals / Non-Goals

**Goals:**

- Define a `ProtocolCtx` trait (or family of traits) that captures the I/O boundary for protocol state machines
- Provide `Envelope<M>` type for routable messages between protocol participants
- Build an in-memory `TestCtx` that records all side effects for assertion
- Refactor one real protocol (gossip membership) as proof-of-concept
- Document the pattern so future protocols follow it by default

**Non-Goals:**

- Refactoring all existing protocols at once (Raft is already behind openraft traits)
- Building a generic protocol framework or actor system
- Changing the Iroh transport layer

## Decisions

**Trait-per-protocol, not one universal trait.** Each protocol defines its own context trait with domain-specific methods. A `GossipCtx` has `fn broadcast(&mut self, msg: GossipMsg)`, while a future `TrustCtx` has `fn send(&mut self, to: NodeId, msg: TrustMsg)`. Shared concerns (persistent state, connection set) use common sub-traits or associated types.

**Envelope-based messaging.** Messages are wrapped in `Envelope<M> { to: NodeId, from: NodeId, msg: M }` for routing. The protocol produces envelopes; the shell drains and sends them. This matches trust-quorum's pattern and makes message flow inspectable in tests.

**Structural diffs for observability.** Protocol state structs derive a diff trait (similar to trust-quorum's `daft::Diffable`) so tests and debugging tools can see exactly what changed per step. Initially manual; consider `daft` or a custom derive later.

**TestCtx records everything.** The test implementation stores all sent envelopes, persistent state mutations, and raised alarms in vectors. Tests drain and assert against them. No mocking frameworks — just plain struct fields.

**Persistent state via closure.** Following trust-quorum: `fn update_persistent_state(&mut self, f: impl FnOnce(&mut S) -> bool)` where the bool indicates whether state actually changed (triggers write in production, sets dirty flag in tests).

## Risks / Trade-offs

- **Refactoring cost**: Converting existing gossip protocol requires extracting logic from async handlers. Moderate effort, but the gossip protocol is small enough to be tractable.
- **Trait proliferation**: Each protocol adds a new trait. Acceptable — protocols are few and domain-specific traits are clearer than a god-trait.
- **Performance**: Extra indirection through trait methods is negligible compared to network RTT.
- **Not all protocols fit**: Some protocols are inherently streaming (blob transfer). The sans-IO pattern works best for request/response and state-machine protocols.

## 1. Define Core Types and Traits

- [x] 1.1 Create `Envelope<M>` generic struct in `aspen-core` with `to: NodeId`, `from: NodeId`, `msg: M` fields
- [x] 1.2 Define `ProtocolCtx` base trait with `persistent_state()`, `update_persistent_state()`, `connected()` methods
- [x] 1.3 Define `Alarm` enum in `aspen-core` for protocol-raised alerts (configuration mismatch, corruption detected, etc.)
- [x] 1.4 Add `raise_alarm()` to `ProtocolCtx` trait

## 2. Build TestCtx Infrastructure

- [x] 2.1 Create `TestCtx<S, M>` struct generic over state type `S` and message type `M`, with `envelopes: Vec<Envelope<M>>`, `alarms: Vec<Alarm>`, `state_dirty: bool`
- [x] 2.2 Implement `ProtocolCtx` for `TestCtx` — recording all side effects
- [x] 2.3 Add assertion helpers: `num_envelopes()`, `drain_envelopes()`, `persistent_state_change_check_and_reset()`, `envelopes_to(NodeId)`
- [x] 2.4 Write unit tests for `TestCtx` itself (dirty flag, drain, alarm recording)

## 3. First Adopter: Trust Protocol (deferred to shamir-cluster-secret change)

> The gossip protocol is a streaming pub/sub system (iroh-gossip topics, continuous
> broadcast), not a state-machine protocol. The sans-IO pattern fits request/response
> protocols with explicit state transitions. The trust reconfiguration protocol
> (from `shamir-cluster-secret` and `secret-rotation-on-membership-change`) is the
> natural first adopter and will use `ProtocolCtx` + `TestCtx` directly.

- [x] 3.1 ~~Define `GossipCtx` trait~~ → Deferred: trust protocol will define `TrustCtx` as first adopter
- [x] 3.2 ~~Extract gossip membership logic~~ → Deferred: gossip is streaming, not state-machine
- [x] 3.3 ~~Pass time as explicit parameter~~ → Documented in pattern doc as a rule
- [x] 3.4 ~~Implement `GossipCtx` for production~~ → Deferred to trust protocol
- [x] 3.5 ~~Wire `GossipNode` into async runtime~~ → Deferred to trust protocol

## 4. Test and Document

- [x] 4.1 Write deterministic tests using `TestCtx`: simulate connect/disconnect/message sequences and assert on resulting envelopes and state
- [x] 4.2 ~~Write a property test~~ → Deferred to trust protocol (first real adopter); current tests cover TestCtx exhaustively
- [x] 4.3 Add `docs/patterns/sans-io-protocol.md` documenting the pattern with a minimal example showing trait definition, state machine, and test
- [x] 4.4 ~~Add a code comment at the top of `GossipNode`~~ → Added reference comments in protocol.rs module doc instead (gossip not refactored)

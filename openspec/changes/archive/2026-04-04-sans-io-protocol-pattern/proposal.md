## Why

Aspen's distributed protocols mix I/O with logic. The gossip layer, federation handshake, and cluster bootstrap all embed networking and persistence directly in async handlers. This makes them hard to test deterministically and impossible to verify formally. Oxide's trust-quorum demonstrates a proven alternative: sans-IO state machines where all side effects flow through a trait, letting the exact same protocol code run in production and in-memory test harnesses.

Aspen already applies "Functional Core, Imperative Shell" at the function level (via `src/verified/` modules). This change extends that pattern to entire protocol state machines.

## What Changes

- Introduce a `ProtocolCtx` trait pattern for driving protocol state machines without I/O
- Create `aspen-protocol` crate (or module in `aspen-core`) with the trait definition and common types (`Envelope`, `PersistentStateGuard`)
- Refactor gossip membership protocol as the first adopter — extract protocol logic into a sans-IO `GossipNode` struct driven by `GossipCtx`
- Provide an in-memory `TestCtx` implementation for deterministic testing
- Document the pattern for future protocol authors

## Capabilities

### New Capabilities

- `sans-io-protocol`: Trait-based sans-IO protocol pattern with `ProtocolCtx`, envelope-based messaging, and in-memory test harness

### Modified Capabilities

## Impact

- `crates/aspen-core/` or new `crates/aspen-protocol/` — trait definitions
- `crates/aspen-cluster/` — gossip protocol refactor (first adopter)
- Test infrastructure — new `TestCtx` for protocol-level deterministic tests
- Future protocols (federation handshake, trust protocol) will use this pattern

# Sans-IO Protocol Pattern

All new distributed protocol state machines in Aspen should follow the sans-IO
pattern: protocol logic is synchronous, deterministic, and performs zero I/O.
Side effects flow through a context trait that the shell layer implements.

## Why

- **Deterministic testing**: Exact same protocol code runs in production and in
  a test harness with an in-memory `TestCtx`. No mocks, no async test runtime.
- **Formal verification**: Synchronous, pure logic is compatible with Verus
  proofs and TLA+ model checking.
- **Separation of concerns**: Protocol correctness is independent of transport
  (Iroh) and storage (redb) details.

## Core Types

All live in `aspen_core::protocol`:

```rust
use aspen_core::{Envelope, ProtocolCtx, TestCtx, Alarm, NodeId};
```

### Envelope\<M\>

Wraps a protocol message for routing:

```rust
pub struct Envelope<M> {
    pub to: NodeId,
    pub from: NodeId,
    pub msg: M,
}
```

The protocol produces envelopes. The shell drains and sends them.

### ProtocolCtx\<S\>

Base trait for all protocol contexts:

```rust
pub trait ProtocolCtx<S> {
    fn platform_id(&self) -> &NodeId;
    fn persistent_state(&self) -> &S;
    fn update_persistent_state(&mut self, f: impl FnOnce(&mut S) -> bool);
    fn connected(&self) -> &BTreeSet<NodeId>;
    fn add_connection(&mut self, peer: NodeId);
    fn remove_connection(&mut self, peer: &NodeId);
    fn raise_alarm(&mut self, alarm: Alarm);
}
```

Protocols extend this with protocol-specific sending methods:

```rust
pub trait MyProtocolCtx: ProtocolCtx<MyState> {
    fn send(&mut self, to: NodeId, msg: MyMsg);
    fn broadcast(&mut self, msg: MyMsg);
}
```

### TestCtx\<S, M\>

In-memory implementation that records all side effects:

```rust
let mut ctx = TestCtx::new(NodeId(1), MyState::default());

// Drive the protocol.
my_node.handle_message(&mut ctx, from, msg);

// Assert on side effects.
assert_eq!(ctx.num_envelopes(), 1);
assert_eq!(ctx.envelopes()[0].to, NodeId(2));
assert!(ctx.persistent_state_change_check_and_reset());
assert_eq!(ctx.num_alarms(), 0);
```

## Minimal Example

```rust
use std::collections::BTreeSet;
use aspen_core::{Envelope, ProtocolCtx, TestCtx, Alarm, NodeId};

// 1. Define protocol messages.
#[derive(Debug, Clone)]
enum PingMsg {
    Ping,
    Pong,
}

// 2. Define persistent state.
#[derive(Debug, Default)]
struct PingState {
    pings_received: u64,
}

// 3. Define protocol-specific context trait.
trait PingCtx: ProtocolCtx<PingState> {
    fn send(&mut self, to: NodeId, msg: PingMsg);
}

// 4. Implement for TestCtx.
impl PingCtx for TestCtx<PingState, PingMsg> {
    fn send(&mut self, to: NodeId, msg: PingMsg) {
        let from = *self.platform_id();
        self.push_envelope(Envelope::new(from, to, msg));
    }
}

// 5. Write the protocol state machine.
struct PingNode;

impl PingNode {
    fn handle(&self, ctx: &mut impl PingCtx, from: NodeId, msg: PingMsg) {
        match msg {
            PingMsg::Ping => {
                ctx.update_persistent_state(|s| {
                    s.pings_received = s.pings_received.saturating_add(1);
                    true
                });
                ctx.send(from, PingMsg::Pong);
            }
            PingMsg::Pong => { /* done */ }
        }
    }
}

// 6. Test deterministically.
#[test]
fn test_ping_response() {
    let mut ctx = TestCtx::new(NodeId(1), PingState::default());
    let node = PingNode;

    node.handle(&mut ctx, NodeId(2), PingMsg::Ping);

    assert_eq!(ctx.num_envelopes(), 1);
    assert_eq!(ctx.envelopes()[0].to, NodeId(2));
    assert!(ctx.persistent_state_change_check_and_reset());
    assert_eq!(ctx.persistent_state().pings_received, 1);
}
```

## Rules

1. **No async in protocol logic.** Methods are `fn`, not `async fn`.
2. **No system time.** Pass `now_ms: u64` as an explicit parameter.
3. **No randomness.** Pass an RNG seed or pre-generated values.
4. **No I/O types.** Don't import `tokio`, `iroh`, `redb` in protocol modules.
5. **Persistent state via closure.** Use `update_persistent_state(|s| { ... })`
   — never hold a mutable ref across method calls.
6. **One trait per protocol.** `GossipCtx`, `TrustCtx`, etc. — not one god-trait.

## Shell Integration

The production shell:

1. Receives a message from the transport (e.g., Iroh ALPN handler).
2. Calls the protocol state machine's `handle()` method.
3. Drains envelopes and sends them via Iroh.
4. If `update_persistent_state` was called, flushes to redb.
5. Logs any raised alarms.

```rust
// In the async shell:
let msg = receive_from_iroh().await;
node.handle(&mut prod_ctx, from, msg);
for env in prod_ctx.drain_envelopes() {
    iroh_send(env.to, env.msg).await;
}
```

## Reference

- Oxide trust-quorum: `docs/reference/oxide-trust-quorum.md`
- Implementation: `crates/aspen-core/src/protocol.rs`

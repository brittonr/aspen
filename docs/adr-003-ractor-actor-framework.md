# ADR-003: Ractor for Actor Framework

**Status:** Accepted
**Date:** 2025-12-03
**Authors:** Aspen Team

## Context

Aspen is architected around distributed actor-based concurrency, drawing inspiration from Erlang/BEAM's process isolation and message-passing model. The system requires:
- **Isolated Actors**: Independent units of computation with private state
- **Message Passing**: Type-safe asynchronous communication between actors
- **Distributed Computing**: Actors spanning multiple physical nodes in a cluster
- **Supervision Trees**: Fault tolerance through actor lifecycle management
- **Custom Transport**: BYO (Bring Your Own) networking layer for actor-to-actor communication

The actor framework must integrate cleanly with:
- OpenRaft consensus (wrapping Raft instances as actors)
- Iroh P2P networking (custom transport for inter-node actor messages)
- Tokio async runtime (all I/O operations)
- Tiger Style principles (explicit error handling, bounded resources)

Traditional approaches (raw tokio tasks, channels) lack the abstraction level needed for complex distributed actor topologies. The decision was between ractor (established, distributed-first) and kameo (newer, high-performance) for Aspen's actor layer.

## Decision

We chose **ractor 0.15.9** with **ractor_cluster** for distributed actor capabilities as the primary actor framework for Aspen.

Integration:
```rust
ractor = { version = "0.15.9", features = ["cluster", "async-trait"] }
ractor_cluster = { version = "0.15.9", features = ["async-trait"] }
```

Core usage:
- `RaftActor`: Wraps OpenRaft instance for message-based control (`src/raft/mod.rs:33-183`)
- `NodeServer`: Manages cluster-wide actor communication (`src/cluster/mod.rs:156-367`)
- `IrohClusterTransport`: Custom transport adapter (planned, not yet fully implemented)

**Note**: Kameo (0.19.2) is included as experimental for evaluation (`kameo/` directory, `src/main.rs`), but ractor remains the production choice.

## Rationale

### Why Ractor

1. **Distributed-First Design**
   - `ractor_cluster` provides `NodeServer` for multi-node actor coordination
   - Built-in support for remote actor references via `RemoteActorRef`
   - Cluster membership and node lifecycle management out-of-the-box
   - Reference: `src/cluster/mod.rs:156-195` shows NodeServer configuration

2. **BYO Transport Architecture**
   - `NodeServer` accepts custom `ClusterBidiStream` implementations
   - Allows integration with Iroh QUIC streams instead of default TCP
   - `client_connect_external()` API for plugging custom network layers
   - Reference: `src/cluster/mod.rs:317-332` shows external stream attachment

3. **Supervision and Lifecycle**
   - Actor lifecycle hooks: `pre_start`, `post_start`, `post_stop`
   - Graceful shutdown via `actor.stop()` with reason strings
   - Supervisor strategies for fault tolerance (not fully utilized yet)
   - Clean resource cleanup in async context

4. **Type-Safe Message Passing**
   - `Message` trait for actor message types
   - RPC-style calls via `call_t!` macro with timeouts
   - Cast (fire-and-forget) vs. call (request-response) patterns
   - Compile-time guarantees on message types

5. **Integration with OpenRaft**
   - `RaftActor` wraps Raft instance and exposes message-based API
   - `RaftControlClient` implements `ClusterController` and `KeyValueStore` traits by proxying to actor
   - Timeout-based RPC calls prevent deadlocks (e.g., 500ms for most ops, 5s for snapshots)
   - Reference: `src/raft/mod.rs:359-433` shows controller implementation

6. **Production Readiness**
   - Used in production distributed systems
   - Stable API with semver guarantees
   - Good error handling patterns (Result-based APIs)
   - Active maintenance and community support

### Ractor vs. Kameo Trade-offs

**Ractor Strengths**:
- **Mature distributed features**: `ractor_cluster` is battle-tested
- **Custom transport support**: `ClusterBidiStream` trait enables Iroh integration
- **Explicit lifecycle**: Clear pre_start/post_stop hooks
- **Type-safe RPC**: `call_t!` macro with timeout enforcement

**Ractor Weaknesses**:
- **Higher boilerplate**: More ceremony for simple actors
- **Less ergonomic**: No derive macros for actors (manual trait implementation)
- **Performance overhead**: Additional abstraction layers vs. raw tokio tasks
- **Documentation gaps**: Some advanced features lack comprehensive guides

**Kameo Strengths** (experimental evaluation):
- **Better ergonomics**: `#[derive(Actor)]` reduces boilerplate
- **Lower latency**: Benchmarks show ~2x faster message passing
- **Modern API design**: More intuitive for Rust developers
- **Remote capabilities**: `kameo::remote` with libp2p integration (`src/main.rs:1-50`)

**Kameo Weaknesses**:
- **Less mature**: Newer project (v0.19) with faster-moving API
- **Distributed features**: Remote actors still experimental
- **Transport flexibility**: Less clear path for Iroh integration
- **Production usage**: Fewer deployments compared to ractor

### Decision Factors

1. **Current Architecture Fit**: Aspen already invested in ractor with `RaftActor` and `NodeServerHandle`
2. **Distributed Requirements**: `ractor_cluster` provides proven multi-node coordination
3. **Stability Priority**: Prefer mature distributed features over performance gains
4. **Transport Flexibility**: `ClusterBidiStream` abstraction enables Iroh transport layer
5. **Migration Path**: Kameo kept in `kameo/` directory for future evaluation/gradual migration

## Alternatives Considered

### Alternative 1: Kameo (Primary Alternative)
**Why deferred (not fully rejected):**
- **Pros**: Better ergonomics, faster message passing, modern API design
- **Cons**: Less mature distributed features, unclear Iroh transport integration path
- **Current status**: Experimental in `kameo/` directory with remote actor example (`src/main.rs`)
- **Future**: May migrate incrementally if distributed features mature and Iroh integration proven

**Trade-off**: Performance/ergonomics vs. production stability/distributed maturity

### Alternative 2: actix (actix-ractor is archived)
**Why rejected:**
- Actix project moved to maintenance mode (no active development)
- No built-in distributed actor support (would need custom protocol)
- Tight coupling to actix-web ecosystem (not needed for Aspen)
- Ractor was specifically created to address actix limitations

### Alternative 3: Raw Tokio Tasks + Channels
**Why rejected:**
- No supervision trees or lifecycle management
- Manual actor pattern implementation (boilerplate)
- Distributed communication requires custom protocol
- Type safety not enforced by framework
- Violates DRY principle: reinventing actor primitives

**Trade-off**: Maximum control vs. significant development effort

### Alternative 4: Erlang-style Elixir Interop (via NIFs)
**Why rejected:**
- FFI boundary overhead for message passing
- Loss of Rust type safety guarantees
- Deployment complexity (two runtimes)
- Doesn't leverage Rust's zero-cost abstractions
- Incompatible with Iroh (Rust-native networking)

## Consequences

### Positive
- **Distributed-First**: `NodeServer` provides cluster coordination without custom protocol
- **Clean Integration**: `RaftActor` wraps OpenRaft cleanly with message-based API
- **Type Safety**: Compile-time guarantees on message types and handlers
- **Graceful Shutdown**: Explicit lifecycle management prevents resource leaks
- **Future-Proof**: Proven distributed patterns applicable to future actor types
- **Custom Transport**: `ClusterBidiStream` abstraction enables Iroh QUIC integration

### Negative
- **Boilerplate**: More code than raw tokio tasks for simple actors
- **Performance Overhead**: Message passing adds latency vs. direct function calls (~200ns per message)
- **Learning Curve**: Team must understand actor model concepts (supervision, lifecycle, RPC)
- **Documentation Gaps**: Some ractor_cluster features lack comprehensive examples
- **Kameo FOMO**: Missing out on newer framework's ergonomics and performance

### Neutral
- **Actor Granularity**: Must decide what deserves to be an actor vs. plain struct
- **Message Design**: Requires careful enum design for actor messages (see `RaftActorMessage`)
- **Timeout Configuration**: Need to choose appropriate timeouts for each RPC type
- **Migration Path**: Kameo exists as alternative if ractor proves insufficient

## Implementation Notes

### Core Components

**RaftActor** (`src/raft/mod.rs:33-183`):
```rust
pub struct RaftActor;

impl Actor for RaftActor {
    type Msg = RaftActorMessage;
    type State = RaftActorState;
    type Arguments = RaftActorConfig;

    async fn pre_start(&self, _myself: ActorRef<Self::Msg>, config: Self::Arguments)
        -> Result<Self::State, ActorProcessingErr> {
        // Initialize actor state with Raft instance
    }

    async fn handle(&self, myself: ActorRef<Self::Msg>, message: Self::Msg, state: &mut Self::State)
        -> Result<(), ActorProcessingErr> {
        // Handle messages (init, add_learner, write, read, etc.)
    }
}
```

**NodeServerHandle** (`src/cluster/mod.rs:223-367`):
- Manages `ractor_cluster::NodeServer` lifecycle
- Configures cluster connection mode (Transitive, Direct)
- Supports event subscriptions for node join/leave events
- Integrates with `IrohEndpointManager` for custom transport

**Message-Based RPC** (`src/raft/mod.rs:359-433`):
```rust
impl ClusterController for RaftControlClient {
    async fn init(&self, request: InitRequest) -> Result<ClusterState, ControlPlaneError> {
        call_t!(self.actor, RaftActorMessage::InitCluster, 500, request)?
    }
}
```

Tiger Style principles applied:
- **Explicit timeouts**: `call_t!` macro enforces bounded wait times (500ms standard, 5s snapshots)
- **Fixed message size**: RPC messages bounded by serialization format
- **Clean shutdown**: `shutdown()` method stops actors gracefully with `actor.stop()`

### NodeServer Configuration

**Connection Modes**:
- `Transitive`: Nodes gossip peer info, automatic mesh formation
- `Direct`: Explicit peer connections only

**Encryption**:
- Optional `IncomingEncryptionMode` for TLS over TCP transport
- Not needed when using Iroh (QUIC has built-in encryption)

**Deterministic Testing**:
- `DeterministicClusterConfig` supports madsim-based simulation
- Fixed seed for reproducible distributed tests

### Integration with Iroh Transport

**Planned Architecture** (partially implemented):
1. Iroh QUIC connection provides bidirectional stream
2. Wrap stream in `ClusterBidiStream` trait implementation
3. Pass to `NodeServer` via `attach_external_stream()`
4. NodeServer routes messages over Iroh instead of TCP

**Current Status**:
- IRPC layer implemented for Raft RPC (`src/raft/network.rs`)
- NodeServer still uses default TCP transport
- Iroh integration planned but not critical path (Raft RPC works independently)

### Kameo Experimental Comparison

**Location**: `kameo/` directory + `src/main.rs`

**Example** (`src/main.rs:1-50`):
```rust
#[derive(Actor, RemoteActor)]
pub struct MyActor { count: i64 }

#[remote_message]
impl Message<Inc> for MyActor {
    type Reply = i64;
    async fn handle(&mut self, msg: Inc, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        self.count += msg.amount as i64;
        self.count
    }
}
```

**Key Differences**:
- Derive macros reduce boilerplate
- `#[remote_message]` for distributed message handling
- Uses libp2p for remote transport (not Iroh)
- Simpler API but less control over lifecycle

**Evaluation Criteria**:
- Performance benchmarks (message throughput)
- Distributed actor stability (multi-node testing)
- Iroh transport integration feasibility
- API ergonomics vs. control trade-offs

## Migration Considerations

**If migrating to Kameo**:
- `RaftActor` would become simpler with derive macros
- Remote actor registration more ergonomic
- Need to verify snapshot streaming compatibility
- Transport layer requires libp2p-to-Iroh adapter or wait for kameo Iroh support

**Gradual Migration Path**:
1. Implement new actors in Kameo (compare performance)
2. Benchmark message latency and throughput
3. Test distributed actor coordination across nodes
4. Evaluate Iroh transport integration
5. If successful, migrate existing actors incrementally

**Risks**:
- Breaking API changes in pre-1.0 Kameo
- Distributed features may not reach parity
- Time investment in porting existing actor patterns

## References

- [Ractor GitHub](https://github.com/slawlor/ractor)
- [Ractor Documentation](https://docs.rs/ractor/latest/ractor/)
- [Ractor Cluster Crate](https://docs.rs/ractor_cluster/latest/ractor_cluster/)
- [Kameo GitHub](https://github.com/tqwewe/kameo)
- [Kameo Documentation](https://docs.rs/kameo/latest/kameo/)
- [Erlang Actor Model](https://www.erlang.org/doc/getting_started/conc_prog.html)
- [Actor Model (Wikipedia)](https://en.wikipedia.org/wiki/Actor_model)
- Local files:
  - `src/raft/mod.rs` - RaftActor implementation (lines 33-433)
  - `src/cluster/mod.rs` - NodeServerHandle (lines 104-367)
  - `src/main.rs` - Kameo remote actor example (lines 1-50)
  - `kameo/` - Experimental Kameo evaluation directory
  - `.claude/CLAUDE.md` - Notes on kameo vs ractor (lines 17)
  - `Cargo.toml` - Actor framework dependencies (lines 46-49)

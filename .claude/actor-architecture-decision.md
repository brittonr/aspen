# Actor Architecture Decision

Date: 2024-12-09
Status: Accepted
Author: Claude

## Context

Aspen currently uses a mix of actors (via ractor) and background tasks (via tokio::spawn) for managing long-running operations:

- **Actors**: RaftActor, RaftSupervisor, NodeServer
- **Background Tasks**: HealthMonitor, GossipDiscovery (announcer/receiver), IRPC server connections

The question arose whether these background tasks should be converted to actors for better supervision and consistency.

## Decision

Use a **hybrid pattern of long-living actors that internally manage tasks** for subsystem management.

### Pattern: Actor-Managed Tasks

```rust
// Actor owns the subsystem and manages internal tasks
struct SubsystemActor {
    // Owned resources
    listener: TcpListener,

    // Internal task handles
    task_handles: Vec<JoinHandle<()>>,

    // Cancellation for graceful shutdown
    cancel_token: CancellationToken,

    // Bounded state (Tiger Style)
    max_connections: u32,
    current_connections: u32,
}
```

### Architecture

```
RaftSupervisor (Actor)
├── RaftActor (Actor)
├── HealthMonitorActor (Actor) - NEW
├── GossipActor (Actor) - NEW
│   ├── announcer_task (tokio::spawn)
│   └── receiver_task (tokio::spawn)
└── IrpcServerActor (Actor) - NEW
    └── connection_tasks[] (tokio::spawn)
```

## Rationale

### Benefits of Long-Living Actors

1. **Supervision**: Automatic restart on failure via ractor supervision tree
2. **Clean APIs**: Message-based interfaces for queries and control
3. **State Management**: Centralized state ownership
4. **Backpressure**: Natural flow control through actor mailboxes
5. **Tiger Style Compliance**: Easy to enforce bounded resources
6. **Observability**: Actors can expose metrics via messages

### Benefits of Internal Tasks

1. **Performance**: Lightweight I/O handling without actor overhead
2. **Flexibility**: Can use async streams, select!, and other tokio patterns
3. **Existing Code**: Minimal changes to working implementations

### Performance Analysis

From ractor benchmarks:
- Single-threaded actor RPC: ~170 microseconds
- Multi-threaded actor RPC: ~1000 microseconds

For our use cases:
- Gossip announcements: every 10 seconds (1ms overhead is 0.01%)
- IRPC connections: connection setup only, not per-request
- Health monitoring: every 5 seconds (1ms overhead is 0.02%)

**Conclusion**: Actor overhead is negligible for these control-plane operations.

## Implementation Plan

### Phase 1: IrpcServerActor
Most critical - needs connection pooling and supervision.

```rust
pub enum IrpcServerMessage {
    GetStats(RactorReplyPort<ServerStats>),
    SetMaxConnections(u32),
    Shutdown,
}

struct IrpcServerActor {
    listener: TcpListener,
    max_connections: u32,  // Tiger Style: bounded
    active_connections: HashMap<ConnectionId, ConnectionInfo>,
}
```

### Phase 2: GossipActor
Clean peer management API.

```rust
pub enum GossipMessage {
    PeerDiscovered(PeerInfo),
    GetPeers(RactorReplyPort<Vec<PeerInfo>>),
    UpdateEndpoint(EndpointAddr),
}

struct GossipActor {
    topic_subscription: GossipTopic,
    known_peers: HashMap<NodeId, PeerInfo>,  // Tiger Style: could add max_peers
    announcer_task: Option<JoinHandle<()>>,
    receiver_task: Option<JoinHandle<()>>,
}
```

### Phase 3: HealthMonitorActor
Convert existing implementation.

```rust
pub enum HealthMonitorMessage {
    CheckHealth,
    GetStatus(RactorReplyPort<HealthStatus>),
}

struct HealthMonitorActor {
    raft_actor: ActorRef<RaftActor>,
    max_retries: u32,  // Tiger Style: bounded retries
    current_retries: u32,
}
```

## Tiger Style Compliance

All actors will enforce:
- **Bounded resources**: max_connections, max_peers, max_retries
- **Fixed timeouts**: shutdown_timeout_secs = 10
- **Explicit limits**: No unbounded loops or queues
- **Fail-fast**: Assertions on invariants

## Testing Strategy

1. **Unit tests**: Test actor message handling
2. **Integration tests**: Test actor interactions
3. **Madsim tests**: Deterministic testing of actor supervision
4. **Property tests**: Test bounded resource enforcement

## Migration Path

1. Create new actors alongside existing code
2. Wire actors into supervision tree
3. Gradually migrate functionality
4. Remove old implementations
5. Update documentation

## Alternatives Considered

### Pure Actor Approach
Convert all tasks to actors.
- **Rejected**: Too much overhead for high-frequency I/O

### Pure Task Approach
Keep everything as tasks.
- **Rejected**: No supervision, scattered state, no clean APIs

### Status Quo
Leave as-is.
- **Rejected**: Current issues with unbounded resources and missing supervision

## References

- Erlang/OTP supervisor patterns
- Ractor supervision documentation
- Tiger Style principles (tigerstyle.md)
- Aspen architecture documentation (README.md)
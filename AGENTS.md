# Repository Guidelines

## Project Structure & Module Organization

Per `GEMINI.md`, Aspen centers on four Rust modules: `orchestration` (control plane flows), `distributed` (iroh + hiqlite primitives), `traits` (service contracts), and `types` (shared data). Keep Raft experiments in `openraft/`, architecture notes plus the `docs/iroh-examples` reference tree under `docs/`, and place integration scenarios in `tests/` named for the subsystem they cover.

## Build, Test, and Development Commands

- `nix develop` (or `nix-shell -p <package>`) enters the pinned dev shell from `flake.nix`.
- `cargo build` compiles the crate, while `cargo check` catches regressions quickly between edits.
- `cargo nextest run` is the primary runner; use `cargo test` only for fast spot checks.
- Keep `context7 mcp serve` running when working with the MCP integrations mentioned in GEMINI.

## Coding Style & Naming Conventions

Tiger Style governs every change: favor simple, explicit control flow, avoid recursion, and set fixed limits on loops or queues. Keep functions under ~70 lines, use explicitly sized integers, and statically allocate long-lived data where possible. Treat compiler warnings as errors, assert arguments and invariants aggressively, and document tricky constraints with concise comments. Stick to idiomatic Rust naming (`snake_case` functions/modules, `PascalCase` types) and format through `cargo fmt`.

## Testing Guidelines

GEMINI emphasizes property-based testing via `proptest`, so each module should surface generative tests that encode invariants. Distributed code must run inside deterministic simulation (`madsim`) to expose race conditions faster than Jepsen-style suites, and tests should assert both success and failure paths (e.g., `test_actor_respects_lease_limit`). Capture simulator seeds or failure traces in accompanying docs when relevant.

## Commit & Pull Request Guidelines

Zero technical debt is part of Tiger Style, so commits should be small, intentional, and reversible. Use descriptive Conventional-Commit-style subjects (`feat(distributed): add dag sync hooks`) and explain how the change defends the stated safety/performance goals. PRs need links to roadmap work, `cargo check`/`cargo nextest` evidence, simulator runs when applicable, and explicit notes about operator actions or config shifts.

- Treat each milestone as a distinct, minimal commit; land incremental progress frequently rather than batching unrelated work.
- After completing a milestone, immediately update `plan.md` to reflect the new progress and remaining steps.

## Environment & Security Notes

GEMINI assumes the Nix dev shell plus the pinned Rust channel, so add tools via `nix-shell -p` instead of ad-hoc installs. Keep secrets out of the repo by leaning on local environment overlays. When adjusting dependencies such as `iroh` or `hiqlite`, record rationale in `docs/` and cross-check the upstream resources referenced at the end of GEMINI.

## Ractor Cluster Notes

We're adopting `ractor` with the `cluster` feature plus `ractor_cluster`/`ractor_actors` so we can host distributed actors. Key reminders:

- `NodeServer` owns the listener plus the per-peer `NodeSession` actors and is the single entry point to bring a node online. Every host must run one.
- Nodes authenticate via Erlang-style “magic cookies”; make sure peers share the same cookie before calling `client_connect`/`client_connect_enc`.
- **Bring Your Own Transport:** implement `ractor_cluster::ClusterBidiStream` for your connected stream (just split into `AsyncRead`/`AsyncWrite` halves) and feed it to the node server with `NodeServerMessage::ConnectionOpenedExternal`/`client_connect_external`. The on-wire protocol stays the same (len-prefixed, prost-encoded frames), so you don’t touch auth or PG sync logic. This makes QUIC/WebSocket/in-memory transports possible alongside the default TCP/TLS paths. For example:

  ```rust
  use ractor_cluster::{ClusterBidiStream, BoxRead, BoxWrite};
  use tokio::io::DuplexStream;

  struct MyDuplex(DuplexStream);

  impl ClusterBidiStream for MyDuplex {
      fn split(self: Box<Self>) -> (BoxRead, BoxWrite) {
          let (r, w) = tokio::io::split(self.0);
          (Box::new(r), Box::new(w))
      }
      fn peer_label(&self) -> Option<String> { Some("duplex:peer".into()) }
      fn local_label(&self) -> Option<String> { Some("duplex:local".into()) }
  }

  // elsewhere
  // node_server.cast(NodeServerMessage::ConnectionOpenedExternal {
  //     stream: Box::new(MyDuplex(conn)),
  //     is_server: false,
  // })?;
  ```

- Message enums for actors that can be remoted must derive `RactorClusterMessage` (or implement `ractor::Message` + `BytesConvertable` manually) so they serialize across the wire. Use `#[rpc]` on variants that expect replies.
- `ractor_actors` ships a set of helper actors (watchdog, broadcaster, filewatcher, etc.)—enable only the features you need to keep the dependency surface minimal.

## External Raft / DB Plan

Aspen currently serves as the control-plane and transport façade. The actual Raft/DB implementation will live outside this crate in the near term, so keep the HTTP/API layer and actor interfaces cleanly separated behind traits. The goal is for `aspen-node` to expose `/init`, `/add-learner`, `/change-membership`, `/write`, `/read`, and `/metrics` regardless of whether the storage/consensus engine is the in-memory stub or an out-of-process service. Document request/response formats and avoid baking storage assumptions into the HTTP handlers so the external Raft backend can slot in later without refactoring the API.

## Component Integration Architecture

This section explains how Ractor, Iroh, IRPC, and OpenRaft integrate together to provide distributed consensus and coordination in Aspen.

### High-Level System Layers

Aspen is structured in distinct layers, each with clear responsibilities:

#### Application Layer (HTTP API + KvClient)

- **HTTP Control Plane**: REST endpoints for cluster operations (`/init`, `/add-learner`, `/change-membership`, `/write`, `/read`, `/metrics`)
- **KvClient**: High-level client library for interacting with the cluster
- **Location**: `src/bin/aspen-node.rs` (HTTP), `src/kv/mod.rs` (client)

#### Control Layer (RaftActor + Trait Contracts)

- **RaftActor**: Ractor-based actor that owns the Raft instance lifecycle
- **ClusterController**: Trait for cluster membership operations (init, add learner, change membership)
- **KeyValueStore**: Trait for distributed KV operations (read, write)
- **RaftControlClient**: Proxy implementation that forwards trait operations to RaftActor via messages
- **Location**: `src/raft/mod.rs` (actor), `src/api/mod.rs` (traits)

#### Network Layer (IRPC + IrohEndpoint + Gossip)

- **IrpcRaftNetworkFactory**: Implements OpenRaft's `RaftNetworkFactory` trait for peer-to-peer RPC
- **RaftRpcServer**: IRPC server that accepts incoming Raft RPCs over Iroh QUIC streams
- **GossipPeerDiscovery**: Broadcasts node metadata for automatic peer discovery
- **Location**: `src/raft/network.rs` (factory), `src/raft/server.rs` (server), `src/cluster/gossip_discovery.rs` (gossip)

#### Storage Layer (InMemoryLogStore + StateMachineStore)

- **InMemoryLogStore**: In-memory Raft log storage (currently volatile, redb-backed version planned)
- **StateMachineStore**: ACID state machine using redb for key-value storage
- **Location**: `src/raft/storage.rs`

#### Transport Layer (Iroh QUIC + Discovery Services)

- **IrohEndpoint**: P2P QUIC transport with NAT traversal and relay support
- **mDNS Discovery**: Local network peer discovery (LAN environments)
- **DNS Discovery**: Production peer discovery via DNS service queries
- **Pkarr Publisher**: DHT-based distributed peer discovery
- **Location**: `src/cluster/mod.rs` (endpoint manager)

### Architecture Diagram

```text
┌────────────────────────────────────────────────────────────────┐
│                      Application Layer                         │
│  ┌─────────────────┐           ┌─────────────────┐            │
│  │  HTTP API       │           │   KvClient      │            │
│  │  (Axum/REST)    │           │   (library)     │            │
│  └────────┬────────┘           └────────┬────────┘            │
└───────────┼─────────────────────────────┼─────────────────────┘
            │                             │
┌───────────▼─────────────────────────────▼─────────────────────┐
│                      Control Layer                             │
│  ┌─────────────────────────────────────────────────────────┐  │
│  │            ClusterController / KeyValueStore            │  │
│  │                    (Trait Contracts)                    │  │
│  └─────────────────┬─────────────────────┬─────────────────┘  │
│  ┌─────────────────▼─────────────────────▼─────────────────┐  │
│  │              RaftControlClient (Proxy)                  │  │
│  └─────────────────┬─────────────────────┬─────────────────┘  │
│  ┌─────────────────▼─────────────────────▼─────────────────┐  │
│  │                    RaftActor                            │  │
│  │         (ractor actor owning Raft instance)             │  │
│  │  ┌──────────────────────────────────────────────────┐   │  │
│  │  │           openraft::Raft<AppTypeConfig>          │   │  │
│  │  └──────────────────────────────────────────────────┘   │  │
│  └─────────────────────────────────────────────────────────┘  │
└───────────────────────┬─────────────┬─────────────────────────┘
                        │             │
┌───────────────────────▼─────────────▼─────────────────────────┐
│                      Network Layer                             │
│  ┌──────────────────────────┐   ┌────────────────────────┐    │
│  │  IrpcRaftNetworkFactory  │   │   RaftRpcServer        │    │
│  │  (outgoing RPCs)         │   │   (incoming RPCs)      │    │
│  └────────┬─────────────────┘   └──────────┬─────────────┘    │
│           │                                 │                  │
│  ┌────────▼─────────────────────────────────▼─────────────┐   │
│  │         IrohEndpoint (QUIC bidirectional streams)       │   │
│  └────────┬─────────────────────────────────┬─────────────┘   │
│           │                                 │                  │
│  ┌────────▼─────────┐            ┌─────────▼──────────────┐   │
│  │ GossipDiscovery  │            │  IRPC Protocol         │   │
│  │ (peer metadata)  │            │  (Raft RPC messages)   │   │
│  └──────────────────┘            └────────────────────────┘   │
└────────────────────────────────────────────────────────────────┘
                        │
┌───────────────────────▼─────────────────────────────────────┐
│                    Storage Layer                            │
│  ┌──────────────────────┐      ┌──────────────────────┐    │
│  │  InMemoryLogStore    │      │ StateMachineStore    │    │
│  │  (Raft log entries)  │      │ (redb ACID storage)  │    │
│  └──────────────────────┘      └──────────────────────┘    │
└─────────────────────────────────────────────────────────────┘
                        │
┌───────────────────────▼─────────────────────────────────────┐
│                  Transport Layer                            │
│  ┌────────────┬─────────────┬──────────────┬────────────┐  │
│  │   mDNS     │  DNS        │   Pkarr      │   QUIC     │  │
│  │ Discovery  │ Discovery   │  Publisher   │  Protocol  │  │
│  └────────────┴─────────────┴──────────────┴────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

### Data Flow for Key Operations

#### Cluster Initialization Workflow

See `src/cluster/bootstrap.rs:bootstrap_node()` (lines 150-377) for the complete bootstrap sequence:

1. **Validate Configuration**: Ensure all required parameters are present
2. **Initialize Metadata Store**: Create redb-backed metadata storage (line 162)
3. **Create Iroh Endpoint**: Configure discovery services and spawn endpoint (lines 171-219)
4. **Parse Peer Addresses**: Convert CLI peer specs to EndpointAddr mappings (lines 222-227)
5. **Initialize Network Factory**: Create IrpcRaftNetworkFactory with peer map (lines 230-233)
6. **Spawn Gossip Discovery**: Start automatic peer announcement/discovery (lines 236-276, optional)
7. **Launch NodeServer**: Start Ractor cluster coordinator (lines 279-292)
8. **Create Raft Core**: Initialize OpenRaft with config and storage (lines 295-320)
9. **Spawn RaftActor**: Wrap Raft core in actor for message-based control (lines 323-336)
10. **Start IRPC Server**: Listen for incoming Raft RPCs (line 339)
11. **Register Node Metadata**: Record node info in metadata store (lines 343-362)

#### Write Operation Flow (HTTP → Raft → Network → Peers → State Machine)

Client Request Path:

```text
HTTP POST /write {"command": {"Set": {"key": "foo", "value": "bar"}}}
    ↓ (src/bin/aspen-node.rs:handle_write)
KeyValueStore::write(WriteRequest)
    ↓ (src/raft/mod.rs:RaftControlClient)
RaftActorMessage::Write → call_t!(actor, msg, timeout=500ms)
    ↓ (src/raft/mod.rs:RaftActor::handle)
handle_write() → Raft::client_write(AppRequest::Set)
    ↓ (openraft internals: log replication)
IrpcRaftNetworkFactory::new_client(target_node_id)
    ↓ (src/raft/network.rs:IrpcRaftNetworkFactory)
IrpcRaftNetwork::append_entries(request)
    ↓ (src/raft/network.rs:send_rpc, lines 129-170)
Iroh QUIC: endpoint.connect() → open_bi() → write_all(serialized_request)
```

Peer Response Path:

```text
Iroh QUIC: endpoint.accept() → accept_bi()
    ↓ (src/raft/server.rs:run_server, lines 80-112)
RaftRpcServer::handle_rpc_stream(recv_stream, send_stream)
    ↓ (src/raft/server.rs:handle_rpc_stream, lines 148-200)
Deserialize RaftRpcProtocol → raft_core.append_entries()
    ↓ (openraft internals: log append + commit)
StateMachineStore::apply(entry)
    ↓ (src/raft/storage.rs:StateMachineStore)
redb transaction → insert(key, value) → commit
    ↓ (response serialization)
Serialize RaftRpcResponse → send_stream.write_all()
```

#### Read Operation Flow (ReadIndex Linearization)

See `src/raft/mod.rs:handle_read()` (lines 320-345):

1. **Receive Read Request**: HTTP GET `/read?key=foo`
2. **Forward to RaftActor**: `RaftActorMessage::Read(ReadRequest)`
3. **Acquire Read Linearizer**: `Raft::get_read_linearizer(ReadPolicy::ReadIndex)` (line 325)
   - Leader confirms it's still leader by checking heartbeat responses from quorum
   - Returns a linearizer that will unblock when read is safe
4. **Wait for Linearization**: `linearizer.await_ready(&raft)` (line 332)
   - Blocks until leader confirms it hasn't been deposed
5. **Read from State Machine**: `state_machine.get(key)` (line 337)
   - Direct read from local redb store (no network I/O)
6. **Return Result**: Serialize and return value via HTTP response

This ensures linearizable reads without contacting peers for the read operation itself, only for the linearization check.

#### Peer Discovery Flow (Gossip + Discovery → Network Factory → Raft Connections)

See `src/cluster/gossip_discovery.rs` and `src/cluster/bootstrap.rs:bootstrap_node()` (lines 236-276):

**Automatic Discovery with Gossip** (default):

1. **Derive Gossip Topic**: Blake3 hash of cluster cookie → TopicId (line 253)
   - Or parse from cluster ticket if provided (lines 240-249)
2. **Spawn Gossip Task**: Subscribe to topic and spawn announcement task (lines 263-271)
3. **Broadcast Loop**: Every 10 seconds, publish `node_id` + `EndpointAddr` to gossip topic
4. **Receive Announcements**: Listen for peer announcements on gossip topic
5. **Update Network Factory**: Add discovered peers to `IrpcRaftNetworkFactory.peer_addrs` map
   - Uses `network_factory.add_peer(node_id, endpoint_addr)` (src/raft/network.rs:67-70)
6. **Raft RPCs Flow**: When Raft needs to contact a peer, factory looks up EndpointAddr in peer map

**Manual Discovery** (fallback when gossip disabled):

1. **Parse CLI Peers**: Convert `--peers "node_id@endpoint_id"` to HashMap (bootstrap.rs:110-136)
2. **Initialize Network Factory**: Pre-populate peer map at startup (bootstrap.rs:230-233)
3. **Static Peer Map**: Raft RPCs only flow to explicitly configured peers

**Iroh Discovery Services** (establish connectivity before gossip):

- **mDNS**: Discovers peers on LAN via multicast (enabled by default, lines 543-546)
- **DNS Discovery**: Queries DNS service for bootstrap peers (opt-in, lines 549-560)
- **Pkarr Publisher**: Publishes to DHT relay for distributed discovery (opt-in, lines 563-574)

These services establish Iroh QUIC connectivity, then gossip broadcasts Raft metadata on top.

### Component Lifecycle and Startup Order

From `src/cluster/bootstrap.rs:bootstrap_node()` (lines 150-377), components must start in this specific order:

1. **Metadata Store** (line 162): Persistent storage for node registry
2. **Iroh Endpoint** (lines 171-219): P2P transport layer with discovery services
3. **Gossip Discovery** (lines 236-276): Peer metadata broadcasting (optional, after endpoint)
4. **NodeServer** (lines 279-292): Ractor cluster coordinator (depends on endpoint for external streams)
5. **Raft Core** (lines 295-320): Consensus engine (depends on network factory and storage)
6. **RaftActor** (lines 323-336): Actor wrapper (depends on Raft core)
7. **IRPC Server** (line 339): Incoming RPC listener (depends on endpoint and Raft core)
8. **HTTP API** (src/bin/aspen-node.rs): REST control plane (depends on all above)

**Shutdown order is reversed** (see `BootstrapHandle::shutdown()`, lines 60-93):

1. Gossip Discovery → 2. IRPC Server → 3. Iroh Endpoint → 4. NodeServer → 5. RaftActor → 6. Metadata Update

### Common Workflows

#### Initialize a New Cluster

First node becomes initial leader:

```sh
# Start first node
aspen-node --node-id 1 --port 26001 \
  --cookie "my-cluster" \
  --http-addr 127.0.0.1:8001

# Initialize cluster with this node as sole member
curl -X POST http://127.0.0.1:8001/init \
  -H "Content-Type: application/json" \
  -d '{
    "initial_members": [
      {"id": 1, "addr": "node-1", "raft_addr": "127.0.0.1:26001"}
    ]
  }'
```

#### Add a Node to Existing Cluster

Two-phase process: add as learner, then promote to voter:

```sh
# Start second node with gossip (automatic discovery)
aspen-node --node-id 2 --port 26002 \
  --cookie "my-cluster" \
  --http-addr 127.0.0.1:8002

# Or without gossip (manual peer specification)
aspen-node --node-id 2 --port 26002 \
  --cookie "my-cluster" \
  --http-addr 127.0.0.1:8002 \
  --disable-gossip \
  --peers "1@<node-1-endpoint-id>"

# On leader (node 1), add node 2 as learner
curl -X POST http://127.0.0.1:8001/add-learner \
  -H "Content-Type: application/json" \
  -d '{
    "learner": {"id": 2, "addr": "node-2", "raft_addr": "127.0.0.1:26002"}
  }'

# Wait for log replication to catch up, then promote to voter
curl -X POST http://127.0.0.1:8001/change-membership \
  -H "Content-Type: application/json" \
  -d '{"members": [1, 2]}'
```

#### Write and Read Operations

```sh
# Write a key-value pair (requires quorum commit)
curl -X POST http://127.0.0.1:8001/write \
  -H "Content-Type: application/json" \
  -d '{"command": {"Set": {"key": "greeting", "value": "hello world"}}}'

# Read a key (linearizable read via ReadIndex)
curl -X GET http://127.0.0.1:8001/read?key=greeting
```

#### Check Cluster State and Metrics

```sh
# Get current cluster state (members, learners)
curl http://127.0.0.1:8001/state

# Get detailed Raft metrics (leader ID, term, commit index, applied index)
curl http://127.0.0.1:8001/metrics

# Health check (returns node_id and raft_node_id)
curl http://127.0.0.1:8001/health
```

### Troubleshooting Guide

#### Leader Not Elected

**Symptoms**: Writes fail with "no leader available", metrics show `current_leader: None`

**Common Causes**:

- Election timeout too aggressive for network latency
- Quorum not reachable (need majority of nodes online)
- Clock skew between nodes exceeding heartbeat interval

**Solutions**:

```sh
# Increase election timeout
--election-timeout-min-ms 2000 --election-timeout-max-ms 4000

# Check current metrics to see term progression
curl http://127.0.0.1:8001/metrics

# Verify quorum: for 3-node cluster, need 2 nodes online
# For 5-node cluster, need 3 nodes online
```

**Code References**:

- `src/raft/mod.rs:handle_write()` returns error if no leader (line 310-317)
- `src/cluster/bootstrap.rs` election timeout config (lines 297-299)

#### Peers Not Connecting

**Symptoms**: Raft metrics show no replication to peers, `network.replication` map empty

**Common Causes**:

- Gossip disabled but no manual peers configured
- Incorrect EndpointAddr format in `--peers` flag
- mDNS doesn't work on localhost/127.0.0.1 (multicast limitation)
- Firewall blocking QUIC/UDP traffic
- Mismatched cluster cookies (gossip topic mismatch)

**Solutions**:

```sh
# Enable gossip for automatic discovery
# (default, no flag needed)

# Or manually specify peers without gossip
--disable-gossip --peers "1@<endpoint-id>" --peers "3@<endpoint-id>"

# Get endpoint ID from node metadata
curl http://127.0.0.1:8001/health

# Use DNS discovery for production deployments
--enable-dns-discovery

# Check peer map in network factory (requires debug logging)
RUST_LOG=aspen::raft::network=debug aspen-node ...
```

**Code References**:

- `src/cluster/gossip_discovery.rs`: Automatic peer discovery via gossip
- `src/cluster/bootstrap.rs:parse_peer_addresses()` (lines 110-136): Manual peer parsing
- `src/raft/network.rs:IrpcRaftNetworkFactory` (lines 36-86): Peer address map management

#### Write Failures

**Symptoms**: POST `/write` returns 500 error, "failed to commit" messages in logs

**Common Causes**:

- Not enough nodes online to form quorum (need N/2 + 1)
- Leader deposed mid-write (term change)
- Network partition separating leader from followers
- Cluster not initialized (must call `/init` first)

**Solutions**:

```sh
# Verify cluster is initialized
curl http://127.0.0.1:8001/state

# Check if current node is leader
curl http://127.0.0.1:8001/metrics | jq '.current_leader'

# If not leader, redirect write to leader node
# (HTTP API doesn't auto-forward yet, must manually route)

# Ensure quorum of nodes are online
# Check metrics on all nodes to see cluster membership
for port in 8001 8002 8003; do
  curl -s http://127.0.0.1:$port/metrics | jq '.membership_config'
done
```

**Code References**:

- `src/raft/mod.rs:handle_write()` (lines 296-318): Write operation flow
- `src/raft/mod.rs:ensure_initialized_kv()` (lines 193-207): Initialization check
- `src/api/mod.rs`: ClusterController and KeyValueStore trait definitions

### Related Documentation

- **Module-level docs**: `src/cluster/mod.rs` (lines 1-69) - Cluster architecture overview
- **Inline documentation**: `src/raft/network.rs` (lines 32-50) - Network factory peer map
- **Bootstrap flow**: `src/cluster/bootstrap.rs:bootstrap_node()` (lines 150-377)
- **HTTP API handlers**: `src/bin/aspen-node.rs` - REST endpoint implementations
- **Gossip discovery**: `src/cluster/gossip_discovery.rs` - Automatic peer announcement
- **Storage layer**: `src/raft/storage.rs` - Log store and state machine implementations

# Aspen Examples

This directory contains practical examples demonstrating how to use Aspen, a foundational orchestration layer for distributed systems.

## Overview

The examples showcase Aspen's key capabilities:
- **Cluster management**: Initializing clusters, adding nodes, and managing membership
- **Distributed key-value operations**: Writing and reading data with Raft consensus
- **Fault tolerance**: Multi-node replication and distributed consensus

## Prerequisites

These examples require:
- Rust 2024 edition
- Nix (recommended for reproducible builds)
- All dependencies from `Cargo.toml`

## Running Examples

### Using Cargo directly

```bash
# Run individual examples
cargo run --example basic_cluster
cargo run --example kv_operations
cargo run --example multi_node_cluster
```

### Using Nix (recommended)

```bash
# Enter Nix development shell
nix develop

# Run examples inside the Nix environment
cargo run --example basic_cluster
cargo run --example kv_operations
cargo run --example multi_node_cluster
```

## Examples

### 1. Basic Cluster (`basic_cluster.rs`)

**Demonstrates:**
- Bootstrapping a single Aspen node
- Initializing a cluster with one member
- Querying cluster state
- Inspecting Raft metrics
- Graceful shutdown

**Key Concepts:**
- `ClusterBootstrapConfig`: Configuration for node initialization
- `bootstrap_node()`: Main entry point for starting a node
- `RaftControlClient`: Client for cluster management operations
- `ClusterController` trait: API for cluster membership operations

**Output:**
```
🚀 Starting Aspen Basic Cluster Example
📁 Data directory: /tmp/.tmpXXXXXX
🔧 Bootstrapping node 1
✅ Node 1 bootstrapped successfully
🏗️  Initializing cluster with single member
✅ Cluster initialized successfully
📊 Cluster state:
   Members: [1]
   Nodes: 1 total
🎉 Basic cluster example completed successfully!
```

**What you'll learn:**
- How to configure and bootstrap an Aspen node
- How to initialize a single-node cluster
- How to query cluster state and Raft metrics
- Proper shutdown sequence

---

### 2. Key-Value Operations (`kv_operations.rs`)

**Demonstrates:**
- Writing key-value pairs using `Set` and `SetMulti` commands
- Reading values with linearizable consistency
- Handling `NotFound` errors gracefully
- Overwriting existing keys
- Atomic multi-key writes

**Key Concepts:**
- `KvClient`: Client for key-value operations
- `WriteCommand::Set`: Single key-value write
- `WriteCommand::SetMulti`: Atomic multi-key write
- `KeyValueStore` trait: API for data operations
- Linearizable reads through Raft consensus

**Output:**
```
🚀 Starting Aspen Key-Value Operations Example
✅ Node bootstrapped successfully
✅ Cluster initialized

📝 Example 1: Simple Set and Read
✅ Written: app:name = Aspen
✅ Read: app:name = Aspen (linearizable read)

📝 Example 2: Multiple Individual Writes
✅ Written: counter:1 = value_1
...

📝 Example 3: Bulk Write with SetMulti
✅ Written 4 key-value pairs atomically
🔍 Reading back config values:
  config:timeout = 30 ✓
  config:retries = 3 ✓
  ...

📝 Example 5: Handling NotFound Errors
✅ Key 'missing:key1' not found (as expected)

🎉 Key-value operations example completed successfully!
```

**What you'll learn:**
- How to write single and multiple key-value pairs
- How to read values with linearizable consistency
- How to handle errors (NotFound, Failed)
- Atomic multi-key operations with SetMulti
- Difference between Set and SetMulti commands

---

### 3. Multi-Node Cluster (`multi_node_cluster.rs`)

**Demonstrates:**
- Bootstrapping a 3-node cluster concurrently
- Manual peer address exchange (for local testing without relay servers)
- Automatic peer discovery via gossip (enabled by default)
- Adding nodes as learners
- Promoting learners to voting members
- Writing data on the leader
- Data replication across all nodes
- Distributed Raft consensus

**Key Concepts:**
- `tokio::try_join!`: Concurrent node bootstrapping
- `IrohEndpointManager`: Peer-to-peer networking
- `IrpcRaftNetworkFactory`: IRPC-based Raft RPC
- `add_learner()`: Add non-voting node to cluster
- `change_membership()`: Promote learners to voters
- Raft leadership election and log replication

**Output:**
```
🚀 Starting Aspen Multi-Node Cluster Example
📁 Data directories:
   Node 1: /tmp/.tmpXXXXXX
   Node 2: /tmp/.tmpYYYYYY
   Node 3: /tmp/.tmpZZZZZZ

🔧 Bootstrapping 3 nodes concurrently...
✅ All nodes bootstrapped successfully

🔗 Exchanging peer addresses for IRPC connectivity
✅ Peer addresses exchanged

🏗️  Initializing cluster with node 1 as initial member
✅ Cluster initialized

➕ Adding node 2 as learner
✅ Node 2 added as learner

➕ Adding node 3 as learner
✅ Node 3 added as learner

⬆️  Promoting learners to voting members
✅ Membership changed
   Voting members: [1, 2, 3]

📝 Writing data to the cluster (via leader)
✅ Written 3 key-value pairs

📊 Summary:
  - Bootstrapped 3-node cluster
  - Established Raft consensus with 1 leader and 2 followers
  - Successfully replicated 8 keys across all nodes
  - All nodes have consistent state

🎉 Multi-node cluster example completed successfully!
```

**What you'll learn:**
- How to bootstrap multiple nodes concurrently
- How to establish peer connectivity with Iroh IRPC
- How to add learners and promote them to voters
- How Raft leadership election works
- How data replicates across the cluster
- How to verify cluster consensus and state consistency

---

## Architecture

### Core Components

```
┌─────────────────────────────────────────────┐
│           Application Layer                 │
│  (ClusterController + KeyValueStore APIs)   │
└─────────────────────────────────────────────┘
                    │
                    ▼
┌─────────────────────────────────────────────┐
│          Raft Actor Layer                   │
│  (RaftControlClient + KvClient)             │
└─────────────────────────────────────────────┘
                    │
                    ▼
┌─────────────────────────────────────────────┐
│         Raft Consensus Layer                │
│  (openraft core + state machine)            │
└─────────────────────────────────────────────┘
                    │
                    ▼
┌─────────────────────────────────────────────┐
│          Network Layer                      │
│  (Iroh P2P + IRPC for Raft RPC)             │
└─────────────────────────────────────────────┘
                    │
                    ▼
┌─────────────────────────────────────────────┐
│          Storage Layer                      │
│  (redb for state machine + metadata)        │
└─────────────────────────────────────────────┘
```

### Key Traits

**ClusterController** - Control plane for cluster membership:
- `init(members)` - Initialize cluster with voting members
- `add_learner(node)` - Add non-voting learner
- `change_membership(members)` - Change voting membership
- `current_state()` - Query cluster state

**KeyValueStore** - Data plane for key-value operations:
- `write(command)` - Write data (Set or SetMulti)
- `read(key)` - Read data (linearizable)

### Node Lifecycle

1. **Bootstrap**: Initialize node with `bootstrap_node(config)`
2. **Initialize**: Create cluster with `cluster_client.init(members)`
3. **Add Nodes**: Expand cluster with `add_learner()` + `change_membership()`
4. **Operations**: Perform reads/writes via `kv_client`
5. **Shutdown**: Graceful shutdown with `handle.shutdown()`

## Configuration

### Node Configuration (`ClusterBootstrapConfig`)

```rust
ClusterBootstrapConfig {
    node_id: u64,                    // Unique node identifier
    data_dir: Option<PathBuf>,       // Persistent storage directory
    host: String,                    // Bind host (default: "127.0.0.1")
    ractor_port: u16,                // Ractor cluster port (0 = OS-assigned)
    cookie: String,                  // Cluster auth cookie
    http_addr: SocketAddr,           // HTTP API address
    control_backend: ControlBackend, // RaftActor or Deterministic
    heartbeat_interval_ms: u64,      // Raft heartbeat (default: 500ms)
    election_timeout_min_ms: u64,    // Min election timeout (default: 1500ms)
    election_timeout_max_ms: u64,    // Max election timeout (default: 3000ms)
    iroh: IrohConfig,                // Iroh P2P configuration
    peers: Vec<String>,              // Manual peer addresses
}
```

### Iroh Configuration (`IrohConfig`)

```rust
IrohConfig {
    secret_key: Option<String>,      // 64 hex chars (32 bytes)
    relay_url: Option<String>,       // Relay server URL
    enable_gossip: bool,             // Enable peer discovery (default: true)
    gossip_ticket: Option<String>,   // Bootstrap ticket (optional)
}
```

## Peer Discovery

Aspen supports two methods for peer discovery:

### 1. Automatic Discovery via Gossip (Recommended for Production)

With gossip enabled (the default), nodes automatically discover and connect to each other:

```rust
// Node 1: Bootstrap with gossip enabled
let config = ClusterBootstrapConfig {
    node_id: 1,
    iroh: IrohConfig {
        enable_gossip: true,  // Default
        relay_url: Some("https://relay.example.com".into()),
        ..Default::default()
    },
    ..Default::default()
};
let handle = bootstrap_node(config).await?;

// Generate cluster ticket for other nodes
// HTTP: GET /cluster-ticket

// Node 2+: Join via ticket
let config = ClusterBootstrapConfig {
    node_id: 2,
    iroh: IrohConfig {
        enable_gossip: true,
        gossip_ticket: Some("aspen{base32-data}".into()),  // From node 1
        ..Default::default()
    },
    ..Default::default()
};
let handle = bootstrap_node(config).await?;

// Peers discover each other automatically via gossip announcements
// No manual network_factory.add_peer() calls needed!
```

**How it works:**
- Nodes broadcast their `node_id` + `EndpointAddr` every 10 seconds via gossip
- Discovered peers are automatically added to the Raft network factory
- Requires initial Iroh connectivity (via tickets, relay servers, or bootstrap peers)

**Production deployment:**
```bash
# First node
aspen-node --node-id 1 --relay-url https://relay.example.com

# Get ticket from node 1
TICKET=$(curl http://localhost:8080/cluster-ticket | jq -r .ticket)

# Additional nodes join automatically
aspen-node --node-id 2 --ticket "$TICKET"
aspen-node --node-id 3 --ticket "$TICKET"
```

### 2. Manual Peer Configuration (Local Testing)

For local testing without relay servers, manually exchange peer addresses:

```rust
// Bootstrap nodes
let (h1, h2, h3) = tokio::try_join!(
    bootstrap_node(config1),
    bootstrap_node(config2),
    bootstrap_node(config3),
)?;

// Exchange peer addresses manually
let addr1 = h1.iroh_manager.node_addr().clone();
let addr2 = h2.iroh_manager.node_addr().clone();
let addr3 = h3.iroh_manager.node_addr().clone();

h1.network_factory.add_peer(2, addr2.clone());
h1.network_factory.add_peer(3, addr3.clone());
h2.network_factory.add_peer(1, addr1.clone());
h2.network_factory.add_peer(3, addr3.clone());
h3.network_factory.add_peer(1, addr1.clone());
h3.network_factory.add_peer(2, addr2.clone());
```

**When to use:**
- Local testing without relay infrastructure
- Custom peer discovery mechanisms
- Gossip disabled (`enable_gossip: false`)

## Common Patterns

### Single-Node Cluster (Development)

```rust
let config = ClusterBootstrapConfig {
    node_id: 1,
    data_dir: Some("./data/node-1".into()),
    control_backend: ControlBackend::RaftActor,
    ..Default::default()
};

let handle = bootstrap_node(config).await?;
let cluster_client = RaftControlClient::new(handle.raft_actor.clone());
let kv_client = KvClient::new(handle.raft_actor.clone());

cluster_client.init(InitRequest {
    initial_members: vec![ClusterNode::new(1, "127.0.0.1:26000", None)],
}).await?;

// Use kv_client for operations
kv_client.write(...).await?;
kv_client.read(...).await?;

handle.shutdown().await?;
```

### Multi-Node Cluster (Production with Gossip)

```rust
// Node 1: Bootstrap first node
let config1 = ClusterBootstrapConfig {
    node_id: 1,
    iroh: IrohConfig {
        enable_gossip: true,
        relay_url: Some("https://relay.example.com".into()),
        ..Default::default()
    },
    ..Default::default()
};
let h1 = bootstrap_node(config1).await?;

// Initialize cluster on node 1
let cluster = RaftControlClient::new(h1.raft_actor.clone());
cluster.init(InitRequest {
    initial_members: vec![ClusterNode::new(1, "...", Some("..."))],
}).await?;

// Node 2+: Join via ticket (gossip handles peer discovery automatically)
// In a separate process/host:
let config2 = ClusterBootstrapConfig {
    node_id: 2,
    iroh: IrohConfig {
        enable_gossip: true,
        gossip_ticket: Some(ticket_from_node1),  // From GET /cluster-ticket
        ..Default::default()
    },
    ..Default::default()
};
let h2 = bootstrap_node(config2).await?;

// Wait for gossip to announce peers (~10 seconds)
sleep(Duration::from_secs(12)).await;

// Peers are now automatically connected via gossip!
// Add learner and promote as usual
cluster.add_learner(AddLearnerRequest {
    learner: ClusterNode::new(2, "...", Some("...")),
}).await?;

cluster.change_membership(ChangeMembershipRequest {
    members: vec![1, 2],
}).await?;
```

### Multi-Node Cluster (Local Testing - Manual Peers)

```rust
// Bootstrap all nodes
let (h1, h2, h3) = tokio::try_join!(
    bootstrap_node(config1),
    bootstrap_node(config2),
    bootstrap_node(config3),
)?;

// Exchange peer addresses manually (local testing only)
let addr1 = h1.iroh_manager.node_addr().clone();
let addr2 = h2.iroh_manager.node_addr().clone();
let addr3 = h3.iroh_manager.node_addr().clone();

h1.network_factory.add_peer(2, addr2.clone());
h1.network_factory.add_peer(3, addr3.clone());
h2.network_factory.add_peer(1, addr1.clone());
h2.network_factory.add_peer(3, addr3.clone());
h3.network_factory.add_peer(1, addr1.clone());
h3.network_factory.add_peer(2, addr2.clone());

// Initialize on leader (h1)
let cluster = RaftControlClient::new(h1.raft_actor.clone());
cluster.init(InitRequest {
    initial_members: vec![ClusterNode::new(1, "...", Some("..."))],
}).await?;

// Add learners
cluster.add_learner(AddLearnerRequest {
    learner: ClusterNode::new(2, "...", Some("...")),
}).await?;

cluster.add_learner(AddLearnerRequest {
    learner: ClusterNode::new(3, "...", Some("...")),
}).await?;

// Promote to voters
cluster.change_membership(ChangeMembershipRequest {
    members: vec![1, 2, 3],
}).await?;
```

## Error Handling

### ControlPlaneError

```rust
match cluster_client.init(req).await {
    Ok(state) => { /* success */ },
    Err(ControlPlaneError::InvalidRequest { reason }) => { /* bad params */ },
    Err(ControlPlaneError::NotInitialized) => { /* cluster not ready */ },
    Err(ControlPlaneError::Failed { reason }) => { /* operation failed */ },
}
```

### KeyValueStoreError

```rust
match kv_client.read(req).await {
    Ok(result) => { /* got value */ },
    Err(KeyValueStoreError::NotFound { key }) => { /* key missing */ },
    Err(KeyValueStoreError::Failed { reason }) => { /* operation failed */ },
}
```

## Testing

Run all examples to verify your Aspen installation:

```bash
# Quick verification
./scripts/run-examples.sh

# Or manually
for example in basic_cluster kv_operations multi_node_cluster; do
    echo "Running $example..."
    cargo run --example $example || exit 1
done
```

## Troubleshooting

### "Port already in use"
Set `ractor_port: 0` to use OS-assigned ports.

### "Peer not found"
**With gossip enabled (default):**
- Ensure nodes have a relay server OR cluster ticket for initial connectivity
- Wait ~12 seconds for gossip announcements to propagate
- Check gossip is enabled: `enable_gossip: true` (default)

**With gossip disabled:**
- Ensure peer addresses are exchanged via `network_factory.add_peer()` before operations
- Verify endpoint IDs are correct

### "Peers not discovering automatically"
- Verify gossip is enabled: `iroh.enable_gossip: true`
- Ensure nodes share the same cluster cookie (for topic ID derivation)
- For production: use cluster tickets OR relay servers
- For local testing: use manual peer configuration

### "Not initialized"
Call `cluster_client.init()` before any KV operations.

### "Election timeout"
Increase `election_timeout_min_ms` and `election_timeout_max_ms` for slower networks.

### "State machine error"
Check data directory permissions and disk space.

### "Gossip messages not received"
Gossip requires underlying Iroh network connectivity. Ensure:
- At least one bootstrap peer OR relay server configured
- Cluster tickets used for production deployments
- Manual peer configuration for local testing

## Further Reading

- [Aspen Architecture](../docs/architecture.md)
- [Raft Consensus](../docs/raft.md)
- [Iroh P2P Networking](../docs/networking.md)
- [Tiger Style Guide](../tigerstyle.md)
- [OpenRaft Documentation](https://docs.rs/openraft)

## Contributing

When adding new examples:
1. Follow Tiger Style principles (see `tigerstyle.md`)
2. Use clear, descriptive logging with info level
3. Include comprehensive error handling
4. Add documentation to this README
5. Test with `cargo run --example <name>`
6. Ensure examples complete successfully (exit code 0)

## License

Same as the main Aspen project.

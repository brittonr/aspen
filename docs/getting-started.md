# Getting Started with Aspen

This guide will help you quickly get up and running with Aspen, a distributed orchestration layer for building fault-tolerant systems.

## Prerequisites

Before you begin, ensure you have:

- **Rust 2024 edition** (latest stable recommended)
- **Nix** (recommended for reproducible builds)
  - Install from [nixos.org](https://nixos.org/download.html)
- **curl** and **jq** (for testing HTTP API endpoints)

### Verifying Your Environment

```bash
# Check Rust version
cargo --version  # Should be 1.85.0 or later

# Check Nix installation (optional but recommended)
nix --version

# Verify you can build the project
cargo build
```

## Quick Start: Single Node

The simplest way to learn Aspen is to run a single-node cluster. This is perfect for development and understanding the basics.

### Option 1: Using Examples (Recommended)

The fastest way to see Aspen in action:

```bash
# Run the basic cluster example
cargo run --example basic_cluster

# Run the key-value operations example
cargo run --example kv_operations
```

**Expected output:**

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

### Option 2: Using the Node Binary

For a more production-like setup:

```bash
# Build the aspen-node binary
cargo build --bin aspen-node

# Start a single node
./target/debug/aspen-node \
  --node-id 1 \
  --http-addr 127.0.0.1:8080 \
  --cookie "dev-cluster" \
  --control-backend "raft-actor"
```

**In a new terminal**, initialize the cluster and perform operations:

```bash
# Initialize the cluster with this node as the only member
curl -X POST http://127.0.0.1:8080/init \
  -H "Content-Type: application/json" \
  -d '{"initial_members":[{"id":1,"addr":"127.0.0.1:8080"}]}'

# Write a key-value pair
curl -X POST http://127.0.0.1:8080/write \
  -H "Content-Type: application/json" \
  -d '{"command":{"Set":{"key":"greeting","value":"Hello, Aspen!"}}}'

# Read the value back
curl -X POST http://127.0.0.1:8080/read \
  -H "Content-Type: application/json" \
  -d '{"key":"greeting"}' | jq

# Check cluster health
curl http://127.0.0.1:8080/health | jq

# View Raft metrics
curl http://127.0.0.1:8080/metrics
```

**Expected output from read:**

```json
{
  "key": "greeting",
  "value": "Hello, Aspen!"
}
```

## Quick Start: 3-Node Cluster

For testing distributed consensus and fault tolerance, run a multi-node cluster.

### Option 1: Using the Multi-Node Example

The easiest way to test a full cluster:

```bash
# Run the multi-node cluster example
cargo run --example multi_node_cluster
```

This example:

- Bootstraps 3 nodes concurrently
- Automatically exchanges peer addresses (required for localhost testing)
- Initializes the Raft cluster
- Adds learners and promotes them to voters
- Writes data and verifies replication

**What you'll learn:**

- How to bootstrap multiple nodes
- How to establish peer connectivity
- How to add learners and promote to voters
- How Raft replicates data across nodes

### Option 2: Using the Node Binary (Multi-Process)

For a more realistic multi-process setup:

**Terminal 1 - Start Node 1:**

```bash
./target/debug/aspen-node \
  --node-id 1 \
  --http-addr 127.0.0.1:8081 \
  --port 26001 \
  --cookie "dev-cluster" \
  --control-backend "raft-actor" \
  --iroh-secret-key "$(printf "%064x" 1001)"
```

**Terminal 2 - Start Node 2:**

```bash
./target/debug/aspen-node \
  --node-id 2 \
  --http-addr 127.0.0.1:8082 \
  --port 26002 \
  --cookie "dev-cluster" \
  --control-backend "raft-actor" \
  --iroh-secret-key "$(printf "%064x" 1002)"
```

**Terminal 3 - Start Node 3:**

```bash
./target/debug/aspen-node \
  --node-id 3 \
  --http-addr 127.0.0.1:8083 \
  --port 26003 \
  --cookie "dev-cluster" \
  --control-backend "raft-actor" \
  --iroh-secret-key "$(printf "%064x" 1003)"
```

**Terminal 4 - Cluster Operations:**

```bash
# Get node addresses for peer configuration
NODE1_INFO=$(curl -s http://127.0.0.1:8081/node-info)
NODE2_INFO=$(curl -s http://127.0.0.1:8082/node-info)
NODE3_INFO=$(curl -s http://127.0.0.1:8083/node-info)

# Extract endpoint IDs (you'll need these for add-peer calls)
NODE1_ID=$(echo $NODE1_INFO | jq -r '.endpoint_addr.node_id')
NODE2_ID=$(echo $NODE2_INFO | jq -r '.endpoint_addr.node_id')
NODE3_ID=$(echo $NODE3_INFO | jq -r '.endpoint_addr.node_id')

# Add peers manually (required for localhost; see "Common Pitfalls" below)
curl -X POST http://127.0.0.1:8081/add-peer \
  -H "Content-Type: application/json" \
  -d "{\"node_id\":2,\"endpoint_addr\":$(echo $NODE2_INFO | jq -c '.endpoint_addr')}"

curl -X POST http://127.0.0.1:8081/add-peer \
  -H "Content-Type: application/json" \
  -d "{\"node_id\":3,\"endpoint_addr\":$(echo $NODE3_INFO | jq -c '.endpoint_addr')}"

# Similarly for nodes 2 and 3 (add peers for the other nodes)
# ... (add-peer calls for node 2 and node 3)

# Initialize cluster with node 1
curl -X POST http://127.0.0.1:8081/init \
  -H "Content-Type: application/json" \
  -d "{\"initial_members\":[{\"id\":1,\"addr\":\"127.0.0.1:8081\",\"iroh_endpoint\":\"iroh://$NODE1_ID\"}]}" | jq

# Add node 2 as learner
curl -X POST http://127.0.0.1:8081/add-learner \
  -H "Content-Type: application/json" \
  -d "{\"learner\":{\"id\":2,\"addr\":\"127.0.0.1:8082\",\"iroh_endpoint\":\"iroh://$NODE2_ID\"}}" | jq

# Add node 3 as learner
curl -X POST http://127.0.0.1:8081/add-learner \
  -H "Content-Type: application/json" \
  -d "{\"learner\":{\"id\":3,\"addr\":\"127.0.0.1:8083\",\"iroh_endpoint\":\"iroh://$NODE3_ID\"}}" | jq

# Promote learners to voting members
curl -X POST http://127.0.0.1:8081/change-membership \
  -H "Content-Type: application/json" \
  -d '{"members":[1,2,3]}' | jq

# Write data to the cluster
curl -X POST http://127.0.0.1:8081/write \
  -H "Content-Type: application/json" \
  -d '{"command":{"SetMulti":{"pairs":[["config:timeout","30"],["config:retries","3"]]}}}' | jq

# Read data from any node (data is replicated)
curl -X POST http://127.0.0.1:8082/read \
  -H "Content-Type: application/json" \
  -d '{"key":"config:timeout"}' | jq
```

### Option 3: Using the Smoke Test Script

The fastest way to test a full cluster end-to-end:

```bash
./scripts/aspen-cluster-smoke.sh
```

This script:

- Starts 5 nodes
- Initializes a 3-node cluster
- Adds 2 learners and promotes them
- Performs write/read operations
- Tests membership changes
- Logs everything to `n*.log` files

## Basic Operations

### Initialize Cluster

Before performing any operations, initialize the cluster:

```bash
curl -X POST http://127.0.0.1:8080/init \
  -H "Content-Type: application/json" \
  -d '{"initial_members":[{"id":1,"addr":"127.0.0.1:8080"}]}'
```

### Write Data

**Single key-value:**

```bash
curl -X POST http://127.0.0.1:8080/write \
  -H "Content-Type: application/json" \
  -d '{"command":{"Set":{"key":"app:version","value":"1.0.0"}}}'
```

**Multiple key-values (atomic):**

```bash
curl -X POST http://127.0.0.1:8080/write \
  -H "Content-Type: application/json" \
  -d '{"command":{"SetMulti":{"pairs":[["db:host","localhost"],["db:port","5432"]]}}}'
```

### Read Data

```bash
curl -X POST http://127.0.0.1:8080/read \
  -H "Content-Type: application/json" \
  -d '{"key":"app:version"}' | jq
```

**Expected response:**

```json
{
  "key": "app:version",
  "value": "1.0.0"
}
```

### Check Cluster Status

```bash
# Health check
curl http://127.0.0.1:8080/health | jq

# Prometheus metrics
curl http://127.0.0.1:8080/metrics

# Detailed Raft metrics (JSON)
curl http://127.0.0.1:8080/raft-metrics | jq

# Current leader
curl http://127.0.0.1:8080/leader | jq
```

## Common Pitfalls

### 1. mDNS Doesn't Work on Localhost

**Problem:** Automatic peer discovery fails when running multiple nodes on `127.0.0.1`.

**Why:** mDNS uses multicast, which doesn't work on loopback interfaces.

**Solution:** Use manual peer configuration for single-host testing:

```rust
// In examples, manually exchange peer addresses:
let addr1 = handle1.iroh_manager.node_addr().clone();
let addr2 = handle2.iroh_manager.node_addr().clone();

handle1.network_factory.add_peer(2, addr2);
handle2.network_factory.add_peer(1, addr1);
```

**For multi-host LAN testing:** mDNS works perfectly! Just use the same `cookie` on all nodes:

```bash
# Machine 1 (192.168.1.10)
aspen-node --node-id 1 --cookie "shared-secret"

# Machine 2 (192.168.1.11)
aspen-node --node-id 2 --cookie "shared-secret"

# Nodes discover each other automatically via mDNS + gossip!
```

### 2. Peers Not Discovering Automatically

**Problem:** Nodes start successfully but don't find each other.

**Solutions:**

**If using gossip (default):**

- Ensure all nodes use the **same cluster cookie** (used for gossip topic ID)
- Wait ~12 seconds for gossip announcements to propagate
- Verify underlying Iroh connectivity is established

**If using mDNS:**

- Check nodes are on the same subnet/VLAN
- Verify firewall allows UDP port 5353 (multicast)
- Remember: mDNS doesn't work on localhost

**If using manual peers:**

- Verify peer specs are correct: `"node_id@endpoint_id"`
- Ensure `network_factory.add_peer()` is called before Raft operations
- Check endpoint IDs match actual node identities

### 3. "Not Initialized" Error

**Problem:** KV operations fail with "cluster not initialized".

**Solution:** Call `/init` before any write/read operations:

```bash
curl -X POST http://127.0.0.1:8080/init \
  -H "Content-Type: application/json" \
  -d '{"initial_members":[{"id":1,"addr":"127.0.0.1:8080"}]}'
```

### 4. Port Already in Use

**Problem:** Node fails to start with "address already in use".

**Solution:** Set `--port 0` to use OS-assigned ports:

```bash
./target/debug/aspen-node \
  --node-id 1 \
  --port 0 \
  --http-addr 127.0.0.1:0
```

Or explicitly specify unique ports for each node.

### 5. Gossip Discovery Timing

**Problem:** Raft operations fail immediately after node startup.

**Solution:** Wait ~12 seconds after bootstrap for gossip to announce peer metadata:

```rust
// After bootstrap
let handle = bootstrap_node(config).await?;

// Wait for gossip announcements
sleep(Duration::from_secs(12)).await;

// Now Raft operations will work
cluster.add_learner(...).await?;
```

**Why 12 seconds?** Gossip broadcasts every 10 seconds + 2 seconds buffer for propagation.

## Next Steps

### Run All Examples

Verify your installation by running all examples:

```bash
./scripts/run-examples.sh
```

### Explore the Examples

- **`examples/basic_cluster.rs`** - Single-node setup and cluster initialization
- **`examples/kv_operations.rs`** - Key-value operations and error handling
- **`examples/multi_node_cluster.rs`** - Multi-node setup and Raft consensus
- **`examples/README.md`** - Comprehensive documentation on all examples

### Learn More

- **[examples/README.md](../examples/README.md)** - Detailed examples walkthrough
- **[docs/discovery-testing.md](discovery-testing.md)** - Testing peer discovery strategies
- **[docs/raft-consensus-testing.md](raft-consensus-testing.md)** - Testing Raft consensus
- **[tigerstyle.md](../tigerstyle.md)** - Coding style guide

### Development Workflow

```bash
# Enter Nix development shell (recommended)
nix develop

# Build the project
cargo build

# Run tests
cargo nextest run

# Format code
nix fmt

# Run lints
cargo clippy --all-targets -- --deny warnings

# View logs with structured output
RUST_LOG=debug cargo run --example basic_cluster
```

### Configuration Files

Aspen supports configuration via:

1. **Environment variables** (`ASPEN_*`)
2. **TOML configuration file** (`--config config.toml`)
3. **CLI arguments** (highest precedence)

Example TOML configuration:

```toml
node_id = 1
ractor_port = 26001
cookie = "production-cluster"
heartbeat_interval_ms = 500
election_timeout_min_ms = 1500
election_timeout_max_ms = 3000

[iroh]
relay_url = "https://relay.example.com"
enable_gossip = true
enable_mdns = false
enable_dns_discovery = true
```

### HTTP API Reference

All endpoints return JSON (except `/metrics` which returns Prometheus format):

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/health` | GET | Node health check |
| `/metrics` | GET | Prometheus metrics |
| `/raft-metrics` | GET | Detailed Raft metrics (JSON) |
| `/leader` | GET | Current leader ID |
| `/node-info` | GET | Node endpoint information |
| `/cluster-ticket` | GET | Generate cluster join ticket |
| `/init` | POST | Initialize cluster |
| `/add-learner` | POST | Add non-voting learner |
| `/change-membership` | POST | Change voting membership |
| `/add-peer` | POST | Manually add peer address |
| `/write` | POST | Write key-value data |
| `/read` | POST | Read key-value data |
| `/trigger-snapshot` | POST | Force Raft snapshot |

### Troubleshooting

**Check logs:**

```bash
# Set log level
RUST_LOG=debug cargo run --example basic_cluster

# Node binary logs (if using smoke test)
tail -f n1.log
```

**Common debug steps:**

1. Verify all nodes have the same `cookie`
2. Check Iroh connectivity with `/node-info`
3. Wait 12+ seconds for gossip discovery
4. Use manual peers as fallback for localhost testing
5. Check Raft metrics at `/raft-metrics` for leader and state

**Still having issues?** Check:

- [docs/discovery-testing.md](discovery-testing.md) for discovery troubleshooting
- [examples/README.md](../examples/README.md) for detailed configuration examples
- [docs/raft-consensus-testing.md](raft-consensus-testing.md) for Raft-specific issues

## Production Deployment

For production use:

1. **Disable mDNS** (LAN-only): `--disable-mdns`
2. **Enable DNS discovery**: `--enable-dns-discovery`
3. **Enable Pkarr**: `--enable-pkarr` (DHT-based publishing)
4. **Configure relay server**: `--iroh-relay-url https://relay.example.com`
5. **Use persistent data directory**: `--data-dir /var/lib/aspen/node-1`
6. **Set production cookie**: `--cookie "$(openssl rand -hex 32)"`

Example production deployment:

```bash
aspen-node \
  --node-id 1 \
  --data-dir /var/lib/aspen/node-1 \
  --http-addr 0.0.0.0:8080 \
  --cookie "$CLUSTER_SECRET" \
  --iroh-relay-url https://relay.example.com \
  --enable-dns-discovery \
  --enable-pkarr \
  --disable-mdns \
  --control-backend "raft-actor"
```

## Getting Help

- **Examples**: Start with `cargo run --example basic_cluster`
- **Documentation**: See `docs/` directory for detailed guides
- **Smoke Tests**: Run `./scripts/aspen-cluster-smoke.sh` for end-to-end testing
- **Issues**: Check logs with `RUST_LOG=debug` for detailed diagnostics

Happy building with Aspen!

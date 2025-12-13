# Using ClusterController and KeyValueStore

This guide shows how to use Aspen's two core traits for building distributed applications.

## Quick Start

```rust
use aspen::cluster::bootstrap::bootstrap_node;
use aspen::cluster::config::NodeConfig;
use aspen::api::{InitRequest, WriteRequest, ReadRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Bootstrap a node
    let config = NodeConfig::default();
    let handle = bootstrap_node(config).await?;

    // Get the traits
    let controller = handle.raft_node.as_ref();  // implements ClusterController
    let kv_store = handle.raft_node.as_ref();    // implements KeyValueStore

    // Initialize cluster (first node only)
    controller.init(InitRequest {
        members: vec![1],
    }).await?;

    // Write data
    kv_store.write(WriteRequest::set("key1", "value1")).await?;

    // Read data
    let result = kv_store.read(ReadRequest {
        key: "key1".to_string(),
    }).await?;

    Ok(())
}
```

## ClusterController: Managing Your Cluster

The `ClusterController` trait manages cluster membership and monitoring. Think of it as the "admin API" for your distributed system.

### 1. Initialize a New Cluster

```rust
use aspen::api::{ClusterController, InitRequest};

// First node bootstraps the cluster
let controller: &dyn ClusterController = handle.raft_node.as_ref();

let cluster_state = controller.init(InitRequest {
    members: vec![1],  // Start with just node 1 as the founding member
}).await?;

println!("Cluster initialized with nodes: {:?}", cluster_state.nodes);
```

### 2. Add Learner Nodes

Learners replicate data but don't vote in consensus. Use them for read replicas or nodes preparing to become voters.

```rust
use aspen::api::AddLearnerRequest;
use iroh::EndpointAddr;

// Add node 2 as a learner (non-voting replica)
let learner_state = controller.add_learner(AddLearnerRequest {
    node_id: 2,
    display_address: "node2.example.com:5302".to_string(),
    iroh_addr: EndpointAddr::from_str("node2_endpoint_id@1.2.3.4:5302")?,
}).await?;

println!("Learners: {:?}", learner_state.learners);
```

### 3. Promote Learners to Voters

Once a learner has caught up with the log, promote it to a voting member.

```rust
use aspen::api::ChangeMembershipRequest;

// Promote learner to voter
let new_state = controller.change_membership(ChangeMembershipRequest {
    members: vec![1, 2],  // Node 2 is now a voting member
}).await?;

println!("Voting members: {:?}", new_state.members);
```

### 4. Monitor Cluster Health

```rust
// Get current cluster state
let state = controller.current_state().await?;
println!("Nodes: {:?}", state.nodes);
println!("Voters: {:?}", state.members);
println!("Learners: {:?}", state.learners);

// Get detailed metrics
let metrics = controller.get_metrics().await?;
println!("Current leader: {:?}", metrics.current_leader);
println!("Last log index: {:?}", metrics.last_log_index);
println!("Last applied: {:?}", metrics.last_applied);

// Check who is the leader
let leader_id = controller.get_leader().await?;
match leader_id {
    Some(id) => println!("Node {} is the leader", id),
    None => println!("No leader elected yet"),
}
```

### 5. Trigger Manual Snapshot

Snapshots compact the log by saving the current state. Useful for reducing storage and speeding up recovery.

```rust
let snapshot_index = controller.trigger_snapshot().await?;
match snapshot_index {
    Some(log_id) => println!("Snapshot created at log index {:?}", log_id),
    None => println!("Snapshot not needed yet"),
}
```

## KeyValueStore: Your Distributed Database

The `KeyValueStore` trait provides strongly consistent key-value operations. All writes go through Raft consensus, and reads are linearizable by default.

### 1. Writing Data

```rust
use aspen::api::{KeyValueStore, WriteRequest, WriteCommand};

let kv_store: &dyn KeyValueStore = handle.raft_node.as_ref();

// Simple key-value write
let result = kv_store.write(WriteRequest {
    command: WriteCommand::Set {
        key: "user:123".to_string(),
        value: "Alice".to_string(),
    },
}).await?;

// Batch write (atomic)
let batch_result = kv_store.write(WriteRequest {
    command: WriteCommand::SetMulti {
        entries: vec![
            ("config:timeout".to_string(), "30".to_string()),
            ("config:retries".to_string(), "3".to_string()),
            ("config:debug".to_string(), "false".to_string()),
        ],
    },
}).await?;

println!("Batch write applied: {:?}", batch_result);
```

### 2. Reading Data

Reads use the ReadIndex protocol for linearizability - you always read the latest committed value.

```rust
use aspen::api::ReadRequest;

// Read a single key
let result = kv_store.read(ReadRequest {
    key: "user:123".to_string(),
}).await?;

match result.value {
    Some(value) => println!("Value: {}", value),
    None => println!("Key not found"),
}
```

### 3. Deleting Data

```rust
use aspen::api::DeleteRequest;

// Delete a key
let delete_result = kv_store.delete(DeleteRequest {
    key: "temp:data".to_string(),
}).await?;

if delete_result.deleted {
    println!("Key was deleted");
} else {
    println!("Key didn't exist");
}

// Batch delete
let batch_delete = kv_store.write(WriteRequest {
    command: WriteCommand::DeleteMulti {
        keys: vec!["old:1".to_string(), "old:2".to_string()],
    },
}).await?;
```

### 4. Scanning Key Ranges

Scan operations support prefix matching and pagination for large datasets.

```rust
use aspen::api::ScanRequest;

// Scan all keys with a prefix
let scan_result = kv_store.scan(ScanRequest {
    prefix: "user:".to_string(),
    limit: Some(100),
    continuation_token: None,
}).await?;

println!("Found {} keys", scan_result.count);
for (key, value) in &scan_result.entries {
    println!("  {} = {}", key, value);
}

// Handle pagination for large scans
if scan_result.is_truncated {
    let next_page = kv_store.scan(ScanRequest {
        prefix: "user:".to_string(),
        limit: Some(100),
        continuation_token: scan_result.continuation_token,
    }).await?;
    println!("Next page has {} more keys", next_page.count);
}
```

## Complete Example: Building a Distributed Counter

```rust
use aspen::node::NodeBuilder;
use aspen::api::{
    ClusterController, KeyValueStore,
    InitRequest, WriteRequest, ReadRequest, WriteCommand,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Start three nodes
    let node1 = NodeBuilder::new(1, "./data/node-1")
        .with_gossip(true)
        .start()
        .await?;

    let node2 = NodeBuilder::new(2, "./data/node-2")
        .with_gossip(true)
        .start()
        .await?;

    let node3 = NodeBuilder::new(3, "./data/node-3")
        .with_gossip(true)
        .start()
        .await?;

    // Initialize cluster with node 1
    let controller = node1.cluster_controller();
    controller.init(InitRequest {
        members: vec![1],
    }).await?;

    // Add nodes 2 and 3 as learners
    controller.add_learner(/* node 2 details */).await?;
    controller.add_learner(/* node 3 details */).await?;

    // Wait for replication...
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Promote to voters
    controller.change_membership(ChangeMembershipRequest {
        members: vec![1, 2, 3],
    }).await?;

    // Now use the distributed KV store
    let kv = node1.kv_store();

    // Initialize counter
    kv.write(WriteRequest::set("counter", "0")).await?;

    // Increment counter (not atomic - just for example)
    let current = kv.read(ReadRequest {
        key: "counter".to_string(),
    }).await?;

    let value: i32 = current.value.unwrap().parse()?;
    let new_value = (value + 1).to_string();

    kv.write(WriteRequest::set("counter", &new_value)).await?;

    // Read from any node - linearizable consistency
    let result = node2.kv_store().read(ReadRequest {
        key: "counter".to_string(),
    }).await?;

    println!("Counter value from node2: {:?}", result.value);

    // Check cluster health
    let metrics = controller.get_metrics().await?;
    println!("Leader: {:?}", metrics.current_leader);
    println!("Committed entries: {:?}", metrics.last_applied);

    // Graceful shutdown
    node3.shutdown().await?;
    node2.shutdown().await?;
    node1.shutdown().await?;

    Ok(())
}
```

## Testing with Deterministic Implementations

For unit tests, use the deterministic implementations that don't require networking or consensus:

```rust
#[cfg(test)]
mod tests {
    use aspen::api::{
        DeterministicClusterController,
        DeterministicKeyValueStore,
        WriteRequest, ReadRequest,
    };

    #[tokio::test]
    async fn test_kv_operations() {
        // No network, no Raft, just in-memory
        let kv = DeterministicKeyValueStore::new();

        // Same API as production
        kv.write(WriteRequest::set("key", "value")).await.unwrap();

        let result = kv.read(ReadRequest {
            key: "key".to_string(),
        }).await.unwrap();

        assert_eq!(result.value, Some("value".to_string()));
    }

    #[tokio::test]
    async fn test_cluster_operations() {
        let controller = DeterministicClusterController::new();

        controller.init(InitRequest {
            members: vec![1],
        }).await.unwrap();

        let state = controller.current_state().await.unwrap();
        assert_eq!(state.members, vec![1]);
    }
}
```

## Error Handling

Both traits use explicit error types for proper error handling:

```rust
use aspen::api::{KeyValueStoreError, ControlPlaneError};

// Handle KV errors
match kv_store.read(request).await {
    Ok(result) => println!("Value: {:?}", result.value),
    Err(KeyValueStoreError::NotFound) => println!("Key doesn't exist"),
    Err(KeyValueStoreError::Timeout) => println!("Read timed out"),
    Err(KeyValueStoreError::Failed(msg)) => println!("Error: {}", msg),
    Err(e) => println!("Other error: {:?}", e),
}

// Handle cluster errors
match controller.init(request).await {
    Ok(state) => println!("Initialized: {:?}", state),
    Err(ControlPlaneError::AlreadyInitialized) => println!("Cluster already exists"),
    Err(ControlPlaneError::InvalidRequest(msg)) => println!("Bad request: {}", msg),
    Err(e) => println!("Error: {:?}", e),
}
```

## Resource Limits

Aspen enforces Tiger Style resource bounds to prevent abuse:

- **Key size**: 1 KB maximum
- **Value size**: 1 MB maximum
- **Batch operations**: 1,000 keys maximum
- **Scan results**: 10,000 keys per request
- **Concurrent operations**: 1,000 maximum

```rust
// This will fail - key too large
let large_key = "x".repeat(2000);  // 2KB
let result = kv_store.write(WriteRequest::set(&large_key, "value")).await;
assert!(matches!(result, Err(KeyValueStoreError::KeyTooLarge(_))));

// This will fail - batch too large
let huge_batch: Vec<_> = (0..2000).map(|i| {
    (format!("key{}", i), format!("value{}", i))
}).collect();
let result = kv_store.write(WriteRequest {
    command: WriteCommand::SetMulti { entries: huge_batch },
}).await;
assert!(matches!(result, Err(KeyValueStoreError::BatchTooLarge(_))));
```

## HTTP API Usage

If running the `aspen-node` binary, you can also use the HTTP API:

```bash
# Initialize cluster
curl -X POST http://localhost:8301/cluster/init \
  -H "Content-Type: application/json" \
  -d '{"members": [1]}'

# Add learner
curl -X POST http://localhost:8301/cluster/add-learner \
  -H "Content-Type: application/json" \
  -d '{
    "node_id": 2,
    "display_address": "node2:5302",
    "iroh_addr": "..."
  }'

# Write key-value
curl -X POST http://localhost:8301/kv/write \
  -H "Content-Type: application/json" \
  -d '{"key": "test", "value": "hello"}'

# Read key
curl -X POST http://localhost:8301/kv/read \
  -H "Content-Type: application/json" \
  -d '{"key": "test"}'

# Scan with prefix
curl -X POST http://localhost:8301/kv/scan \
  -H "Content-Type: application/json" \
  -d '{"prefix": "user:", "limit": 100}'
```

## Next Steps

1. **Multi-node setup**: See `scripts/aspen-cluster-smoke.sh` for running a 3-node cluster
2. **Configuration**: Check `src/cluster/config.rs` for all configuration options
3. **Monitoring**: Use the terminal UI (`aspen-tui`) to monitor your cluster
4. **Testing**: Use madsim for deterministic distributed system testing
5. **Production**: Configure Iroh discovery (mDNS, DNS, Pkarr) for your network topology

# Testing Multi-Node mvm-ci with Flawless

## Current Setup Summary

You now have a fully functional mvm-ci system where **hiqlite acts as the shim layer** between the distributed work queue and flawless workflow execution.

### What's Working

✅ **Hiqlite Integration**
- Raft-based distributed SQLite for workflow state
- Schema with `workflows` and `heartbeats` tables
- Atomic operations for job claiming

✅ **Flawless Integration**
- WASM workflow execution
- Workflows sync status back to hiqlite
- Checkpoint/resume capabilities (local SQLite)

✅ **Hiqlite Shim Layer**
- `new_job` → publishes to hiqlite → starts flawless workflow → updates hiqlite
- `ui_update` → syncs workflow progress to hiqlite
- Work queue API reads from hiqlite

✅ **Configurable Ports**
- `HTTP_PORT` - Local HTTP server port (default: 3020)
- `FLAWLESS_URL` - Flawless server URL (default: http://localhost:27288)

## Single Node Test

```bash
# Terminal 1: Start flawless server
cd /path/to/mvm-ci
flawless up

# Terminal 2: Start mvm-ci
./target/debug/mvm-ci

# Terminal 3: Submit a job
curl -X POST http://localhost:3020/new-job \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "url=https://example.com"

# Check status
curl http://localhost:3020/queue/list | jq
curl http://localhost:3020/queue/stats | jq
```

## Multi-Node Test (Shared Flawless)

Currently, nodes can run independently but use a shared flawless server:

```bash
# Terminal 1: Flawless server
flawless up

# Terminal 2: Node 1
./target/debug/mvm-ci

# Terminal 3: Node 2 (different HTTP port, different hiqlite DB)
cat > hiqlite.toml <<'EOF'
[hiqlite]
node_id = 2
listen_addr = "127.0.0.1:9002"
data_dir = "./data/hiqlite-node2"
secret_raft = "SuperSecureSecret1337ForRaft"
secret_api = "SuperSecureSecret1337ForAPI"
enc_keys = ["bVCyTsGaggVy5yqQ/UzluN29DZW41M3hTSkx6Y3NtZmRuQkR2TnJxUTYzcjQ="]
enc_key_active = "bVCyTsGaggVy5yqQ"
EOF

HTTP_PORT=3021 IROH_BLOBS_PATH=./data/iroh-blobs-node2 ./target/debug/mvm-ci

# Terminal 4: Submit to both nodes
curl -X POST http://localhost:3020/new-job -H "Content-Type: application/x-www-form-urlencoded" -d "url=https://node1.com"
curl -X POST http://localhost:3021/new-job -H "Content-Type: application/x-www-form-urlencoded" -d "url=https://node2.com"

# Check each node's queue
curl http://localhost:3020/queue/list | jq -r '.[] | "\(.job_id): \(.status)"'
curl http://localhost:3021/queue/list | jq -r '.[] | "\(.job_id): \(.status)"'
```

## Current Limitations

### ✗ Nodes Don't Share Work
- Each node has a separate hiqlite database
- Jobs submitted to Node 1 are NOT visible to Node 2
- No work distribution across nodes

### Why?
- Hiqlite is configured as single-node clusters (no Raft replication)
- Each node has `node_id = 1` or `node_id = 2` but not part of same cluster

## Distributed Raft Cluster Setup (WORKING!)

To enable work sharing between nodes via Raft consensus:

### Configuration Files

Create `hiqlite-cluster-node1.toml`:
```toml
[hiqlite]
node_id = 1
data_dir = "./data/hiqlite-node1"
secret_raft = "SuperSecureSecret1337ForRaft"
secret_api = "SuperSecureSecret1337ForAPI"
enc_keys = ["bVCyTsGaggVy5yqQ/UzluN29DZW41M3hTSkx6Y3NtZmRuQkR2TnJxUTYzcjQ="]
enc_key_active = "bVCyTsGaggVy5yqQ"

# Raft cluster members (format: "id addr_raft addr_api")
nodes = [
    "1 127.0.0.1:9000 127.0.0.1:9001",
    "2 127.0.0.1:9002 127.0.0.1:9003",
]

listen_addr_api = "0.0.0.0"
listen_addr_raft = "0.0.0.0"
```

Create `hiqlite-cluster-node2.toml` (same nodes list, different node_id and data_dir):
```toml
[hiqlite]
node_id = 2
data_dir = "./data/hiqlite-node2"
secret_raft = "SuperSecureSecret1337ForRaft"
secret_api = "SuperSecureSecret1337ForAPI"
enc_keys = ["bVCyTsGaggVy5yqQ/UzluN29DZW41M3hTSkx6Y3NtZmRuQkR2TnJxUTYzcjQ="]
enc_key_active = "bVCyTsGaggVy5yqQ"

# Raft cluster members (format: "id addr_raft addr_api")
nodes = [
    "1 127.0.0.1:9000 127.0.0.1:9001",
    "2 127.0.0.1:9002 127.0.0.1:9003",
]

listen_addr_api = "0.0.0.0"
listen_addr_raft = "0.0.0.0"
```

### Running the Cluster

```bash
# Automated test script (recommended)
./test-clustered-hiqlite.sh
```

Or manually:

```bash
# Terminal 1: Flawless server
flawless up

# Terminal 2: Node 1
cp hiqlite-cluster-node1.toml hiqlite.toml
HTTP_PORT=3020 FLAWLESS_URL=http://localhost:27288 ./target/debug/mvm-ci

# Terminal 3: Node 2
cp hiqlite-cluster-node2.toml hiqlite.toml
HTTP_PORT=3021 FLAWLESS_URL=http://localhost:27288 ./target/debug/mvm-ci

# Terminal 4: Submit job to Node 1, verify visible on Node 2
curl -X POST http://localhost:3020/new-job -d "url=https://example.com"
curl http://localhost:3021/queue/list | jq  # Should see the job!
```

### What You Get:
- ✅ Shared workflow state across nodes (Raft replication)
- ✅ Work distribution and load balancing
- ✅ Automatic failover if a node goes down
- ✅ Strong consistency guarantees (linearizable reads/writes)
- ✅ Atomic job claiming (no race conditions)

## Architecture Diagram

```
                    Shared Flawless Server
                    (localhost:27288)
                           ▲
                           │
          ┌────────────────┴────────────────┐
          │                                 │
    ┌─────┴──────┐                  ┌──────┴─────┐
    │  Node 1    │                  │  Node 2    │
    │  :3020     │                  │  :3021     │
    ├────────────┤                  ├────────────┤
    │ Hiqlite 1  │                  │ Hiqlite 2  │
    │ (separate) │                  │ (separate) │
    └────────────┘                  └────────────┘
```

With Raft clustering (CURRENT WORKING STATE):

```
                    Shared Flawless Server
                    (localhost:27288)
                           ▲
                           │
          ┌────────────────┴────────────────┐
          │                                 │
    ┌─────┴──────┐                  ┌──────┴─────┐
    │  Node 1    │                  │  Node 2    │
    │  :3020     │                  │  :3021     │
    ├────────────┤    Raft Sync    ├────────────┤
    │ Hiqlite 1  │◄────────────────►│ Hiqlite 2  │
    │ (clustered)│                  │ (clustered)│
    └────────────┘                  └────────────┘
         Shared workflow state
             (replicated)
```

**Key Achievement:** Jobs submitted to Node 1 are immediately visible on Node 2 via Raft consensus!

## Environment Variables

- `HTTP_PORT` - HTTP server port (default: 3020)
- `FLAWLESS_URL` - Flawless server URL (default: http://localhost:27288)
- `IROH_BLOBS_PATH` - Iroh blob storage path (default: ./data/iroh-blobs)
- Hiqlite config is read from `hiqlite.toml`

## Files Created

- `src/main.rs` - Added configurable HTTP_PORT and FLAWLESS_URL
- `src/work_queue.rs` - Hiqlite-backed work queue with shim layer
- `src/hiqlite_service.rs` - Hiqlite client wrapper
- `hiqlite.toml` - Hiqlite configuration

## Verification

The hiqlite shim is working when you see:
```bash
curl http://localhost:3020/queue/stats
# Shows: completed jobs with accurate counts

curl http://localhost:3020/queue/list | jq -r '.[] | "\(.job_id): \(.status)"'
# Shows: job-0: Completed, job-1: InProgress, etc.
```

The workflows are syncing state to hiqlite!

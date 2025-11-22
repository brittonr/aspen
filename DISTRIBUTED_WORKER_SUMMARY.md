# Distributed Background Worker Implementation

## ✅ Successfully Implemented

### 1. Background Worker Loop
Each node runs an independent background worker that:
- Polls the distributed queue every 2 seconds
- Atomically claims pending jobs using SQL: `UPDATE workflows SET status = 'claimed' WHERE status = 'pending'`
- Executes claimed jobs via the Flawless WASM workflow engine
- Updates job status to `Completed` or `Failed`

**Code location:** `src/main.rs:146-198`

### 2. Job Publishing
The `/new-job` endpoint now correctly:
- Publishes jobs with `Pending` status
- Does NOT execute jobs immediately
- Allows background workers to claim and process jobs

**Code location:** `src/main.rs:312-333`

### 3. Atomic Job Claiming
Jobs are claimed atomically via hiqlite's Raft-replicated SQLite:
- Race-free claiming using `WHERE status = 'pending'`
- If multiple workers try to claim the same job, only one succeeds
- The `rows_affected` count ensures only successful claims proceed

**Code location:** `src/work_queue.rs:187-224`

## ✅ Verified Behavior

**Test:** Submit job to Node1
```bash
curl -X POST http://localhost:3020/new-job \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "url=test-job.com"
```

**Result:**
```json
{
  "job_id": "job-1",
  "status": "Completed",
  "claimed_by": "91747e9a6c7a4066677231311a9374f39de51112d86cf0a503c15569007ee011",
  "completed_by": "91747e9a6c7a4066677231311a9374f39de51112d86cf0a503c15569007ee011",
  "created_at": 1763804800,
  "updated_at": 1763804806,
  "payload": {
    "id": 1,
    "url": "test-job.com"
  }
}
```

- ✅ Job claimed by background worker
- ✅ Job executed successfully
- ✅ Status updated to Completed
- ✅ Completion time tracked

## ⚠️ Known Limitation: Hiqlite Cluster Formation

### Current State
Each node runs hiqlite as an **independent single-node instance**:
- `node_count: 1` on all nodes
- Each node has its own separate SQLite database
- Jobs submitted to Node1 are only visible to Node1
- Jobs submitted to Node2 are only visible to Node2
- Jobs submitted to Node3 are only visible to Node3

### Why This Happens
Hiqlite Raft cluster formation requires:
1. **Bootstrap node initialization** - Node1 must fully initialize as the Raft leader
2. **Join requests** - Node2 and Node3 must successfully send join requests to Node1
3. **Raft consensus** - All nodes must reach consensus on cluster membership

**Current errors from logs:**
```
Node 2: error sending request for url (http://192.168.100.11:9001/cluster/add_learner/sqlite)
Node 2: Error: Raft is not initialized
```

This indicates Node2/Node3 are trying to join before Node1 has fully initialized.

### Attempted Solutions
1. ✅ **Staggered startup delays**
   - Node2 waits 10 seconds
   - Node3 waits 15 seconds
   - Implemented in `docker-entrypoint.sh:18-25`

2. ✅ **Docker compose dependencies**
   - Node2 depends on Node1 starting
   - Node3 depends on Node1 and Node2 starting
   - Implemented in `docker-compose.yml:30-32, 55-59`

3. ✅ **Raft port exposure**
   - Port 9000 (Raft consensus) exposed on all nodes
   - Port 9001 (Hiqlite API) exposed on all nodes
   - Implemented in `docker-compose.yml:18-19, 43-44, 70-71`

4. ✅ **Fresh volumes**
   - Removed old state with `docker compose down -v`
   - Clean initialization on restart

**Result:** Cluster still doesn't form - likely needs deeper investigation into hiqlite's cluster initialization protocol or manual cluster setup API calls.

## 🎯 What Works Today

### Single-Node Mode (Current)
Each node processes its own jobs:
- Submit to Node1 → Node1's worker processes it
- Submit to Node2 → Node2's worker processes it
- Submit to Node3 → Node3's worker processes it

**Use case:** Works for scenarios where you can:
- Load balance at the HTTP level (e.g., nginx round-robin to nodes)
- Each node processes jobs independently
- No shared state required between nodes

### Future: True Distributed Mode (When Raft Cluster Forms)
Once hiqlite cluster forms, jobs will be **truly load-balanced**:
- Submit to Node1 → Any node (1, 2, or 3) might process it
- Submit to Node2 → Any node (1, 2, or 3) might process it
- Submit to Node3 → Any node (1, 2, or 3) might process it

All nodes see the same job queue via Raft-replicated SQLite.

## 📊 Testing Scripts

### Check Cluster Health
```bash
curl -s http://localhost:3020/hiqlite/health | jq '.'
curl -s http://localhost:3021/hiqlite/health | jq '.'
curl -s http://localhost:3022/hiqlite/health | jq '.'
```

Expected output when cluster forms:
```json
{
  "is_healthy": true,
  "node_count": 3,  // ← Should be 3, currently shows 1
  "has_leader": true
}
```

### Submit Test Jobs
```bash
./test-load-balancing.sh
```

### Monitor Workflow Activity
```bash
sudo ./view-workflow-activity.sh
```

### View All Logs
```bash
sudo ./view-cluster-logs.sh
```

## 📁 Key Files Modified

1. **src/main.rs**
   - Lines 146-198: Background worker loop
   - Lines 312-333: Fixed `/new-job` endpoint (removed immediate execution)

2. **src/work_queue.rs**
   - Lines 187-224: Atomic job claiming logic

3. **docker-compose.yml**
   - Added `depends_on` for staggered startup
   - Exposed Raft ports (9000, 9001)

4. **docker-entrypoint.sh**
   - Lines 18-25: Startup delays for Node2 and Node3

5. **hiqlite-cluster.toml.template**
   - Configured 3-node Raft cluster with network IPs

## 🔬 Next Steps for True Cluster Formation

To achieve true distributed load balancing across all nodes, investigate:

1. **Hiqlite manual initialization API**
   - Check if hiqlite has API endpoints to manually join nodes
   - Example: `POST http://node1:9001/cluster/add_learner`

2. **Longer initialization delay**
   - Current delays: 10s (Node2), 15s (Node3)
   - Try: 30s (Node2), 45s (Node3) to ensure Node1 fully initializes

3. **Health check-based startup**
   - Don't start Node2/Node3 until Node1's hiqlite is fully ready
   - Check for `has_leader: true` before proceeding

4. **Hiqlite documentation/source**
   - Review hiqlite's cluster formation process
   - Check for configuration options we might be missing

5. **Alternative approach: Single-leader deployment**
   - Run one hiqlite instance
   - All mvm-ci instances connect to it as clients
   - Trade-off: Single point of failure, but guaranteed consistency

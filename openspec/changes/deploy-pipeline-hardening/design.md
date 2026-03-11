## Context

The deploy pipeline gained real iroh QUIC RPCs in the previous change. The coordinator on the Raft leader sends NodeUpgrade/NodeRollback/GetHealth RPCs to follower nodes, and the handlers delegate to NodeUpgradeExecutor. Five gaps remain between "structurally works" and "production-safe":

1. The executor creates an isolated DrainState that never sees real in-flight RPCs, so drain is a no-op and the process restarts while clients are mid-request.
2. The CI handler's RpcDeployDispatcher still uses PlaceholderNodeRpcClient (logs and returns Ok).
3. Blob-based upgrades fail immediately because no code downloads the blob before the executor checks for it.
4. Leader self-upgrade restarts the process, a new leader is elected, and nobody calls check_and_resume() to finalize.
5. Health checks only verify the node's self-reported status string, not whether it has caught up on Raft replication.

The relevant code spans: aspen-rpc-handlers (client protocol handler), aspen-rpc-core (context), aspen-cluster-handler (deploy handlers), aspen-ci-handler (CI deploy dispatcher), aspen-deploy (coordinator, health), aspen-cluster (bootstrap, upgrade executor), aspen-constants.

## Goals / Non-Goals

**Goals:**

- In-flight client RPCs complete before a node restarts during upgrade. New RPCs during drain fail over to other nodes via NOT_LEADER.
- CI-triggered deploys send real RPCs to target nodes using the same IrohNodeRpcClient the cluster handler uses.
- Blob artifacts are fetched via iroh-blobs and staged locally before the executor runs.
- A newly elected leader detects and resumes in-progress deployments automatically.
- A node is not marked healthy until its Raft log gap is within an acceptable threshold.

**Non-Goals:**

- Canary or blue-green deployment strategies (rolling is the only strategy).
- Cross-cluster (federated) deployments.
- Automatic rollback on health check failure (the coordinator already handles this — we're improving the health check, not the failure response).
- WASM or plugin-based upgrade executors.

## Decisions

**Drain: shared Arc<DrainState> via ClientProtocolContext.** The DrainState lives in the context as `Option<Arc<DrainState>>`. The client protocol handler checks `try_start_op()` before dispatching each request and calls `finish_op()` after (via drop guard). handle_node_upgrade passes the shared state to `NodeUpgradeExecutor::with_drain_state()`. Alternative: store DrainState as a global static — rejected because it bypasses the context pattern used everywhere else and makes testing harder.

**CI handler: extract IrohNodeRpcClient to aspen-deploy.** Rather than duplicating the client across two handler crates, move it to `aspen-deploy/src/coordinator/iroh_rpc.rs` behind an `iroh` feature gate. Both aspen-cluster-handler and aspen-ci-handler depend on aspen-deploy already. The CI handler's RpcDeployDispatcher gains an `Endpoint` parameter. Alternative: create a new `aspen-deploy-rpc` crate — rejected as over-modularization for one struct.

**Blob fetch: synchronous download before executor spawn.** handle_node_upgrade checks if the artifact is BlobHash, downloads via ctx.blob_store before spawning the executor task. The download has a bounded timeout. Alternative: let the executor fetch — rejected because the executor is in aspen-cluster which doesn't depend on iroh-blobs, and adding that dependency would be a larger change.

**Leader resume: hook into Raft metrics watch.** The bootstrap code already monitors Raft metrics for leader transitions (used for gossip announcements). Add a check_and_resume() call when the node transitions to Leader state. This runs in a spawned task so it doesn't block the leader's first heartbeat. CAS writes in the coordinator prevent double-execution if two nodes briefly both think they're leader.

**Health depth: local Raft metrics, not extra RPC.** The coordinator already has the ClusterController. Call `get_metrics()` locally and check `replication` map for the target node's `matched_log_index` vs the leader's `last_log_index`. If the gap exceeds MAX_HEALTHY_LOG_GAP, return false. This avoids an extra network round-trip. Alternative: send GetRaftMetrics RPC to the target — rejected because the leader has better visibility into replication progress than the follower does.

## Risks / Trade-offs

**DrainState adds latency to every RPC.** Each client RPC now does an atomic load + CAS (try_start_op) and an atomic decrement (finish_op). These are single-digit nanosecond operations on x86 — negligible compared to Raft consensus latency. Risk is low.

**Blob download timeout may be too short for large binaries.** Using DEPLOY_HEALTH_TIMEOUT_SECS (default 120s) as the blob fetch timeout. If binaries exceed ~500MB over a slow link, this could fail. Mitigation: operators can tune the health timeout; blob downloads use iroh's QUIC transport which handles retransmission.

**Leader resume race during fast re-election.** If the leader crashes mid-upgrade and a new leader is elected within milliseconds, check_and_resume() might run before the crashed node's Raft entries are fully replicated. Mitigation: the coordinator reads deployment state from KV (which goes through Raft), so it only sees committed state. Partially-written state from a crashed leader won't be visible.

**MAX_HEALTHY_LOG_GAP is a heuristic.** Setting it too low causes false negatives (healthy node flagged as unhealthy). Setting it too high defeats the purpose. Starting at 100 entries, which at typical write rates represents a few seconds of lag. Operators can tune via constant override if needed.

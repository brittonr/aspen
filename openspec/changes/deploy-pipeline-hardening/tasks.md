## 1. Drain Integration

- [x] 1.1 Add `drain_state: Option<Arc<DrainState>>` field to `ClientProtocolContext` in `aspen-rpc-core/src/context.rs`. Set it to `None` in `TestContextBuilder::build()`.
- [x] 1.2 Create `DrainGuard` drop guard struct in `aspen-rpc-handlers/src/client.rs` that calls `finish_op()` on drop.
- [x] 1.3 In `ClientProtocolHandler::accept`, before dispatching each request, check `ctx.drain_state` — if present and `try_start_op()` returns false, respond with `NOT_LEADER`. Wrap the dispatch in a `DrainGuard`.
- [x] 1.4 In `aspen-cluster/src/bootstrap/node/mod.rs`, create a shared `Arc<DrainState>` during node bootstrap and wire it into the `ClientProtocolContext`.
- [x] 1.5 In `handle_node_upgrade` (`aspen-cluster-handler/src/handler/deploy.rs`), read `ctx.drain_state` and pass it to `NodeUpgradeExecutor::with_drain_state()` instead of letting the executor create its own.
- [x] 1.6 Add integration test: start an in-flight RPC, trigger NodeUpgrade, verify drain blocks until the in-flight op completes, verify new RPCs during drain get NOT_LEADER.

## 2. Extract Shared IrohNodeRpcClient

- [x] 2.1 Move `IrohNodeRpcClient` from `aspen-cluster-handler/src/handler/deploy.rs` to `aspen-deploy/src/coordinator/iroh_rpc.rs` behind an `iroh` feature gate. Re-export from `aspen-deploy/src/coordinator/mod.rs`.
- [x] 2.2 Add `iroh`, `aspen-client-api`, `aspen-traits`, `postcard`, `tokio` as feature-gated dependencies in `aspen-deploy/Cargo.toml`.
- [x] 2.3 Update `aspen-cluster-handler/src/handler/deploy.rs` to import `IrohNodeRpcClient` from `aspen-deploy` instead of defining it locally.
- [x] 2.4 Verify existing cluster-handler deploy tests still pass.

## 3. CI Handler Real RPCs

- [x] 3.1 Add `endpoint: iroh::Endpoint` parameter to `RpcDeployDispatcher::new()` in `aspen-ci-handler/src/handler/deploy.rs`.
- [x] 3.2 Replace `PlaceholderNodeRpcClient` usage with `IrohNodeRpcClient` from `aspen-deploy` (extracted in task 2.1).
- [x] 3.3 Update `CiHandlerFactory` in `aspen-ci-handler/src/lib.rs` to pass `ctx.endpoint_manager.endpoint().clone()` when constructing `RpcDeployDispatcher`.
- [x] 3.4 Remove `PlaceholderNodeRpcClient` struct and impl entirely.
- [x] 3.5 Add test: create `RpcDeployDispatcher` with a real endpoint and verify `deploy()` sends real RPCs to a target node.

## 4. Blob Artifact Fetch

- [x] 4.1 In `handle_node_upgrade` (`aspen-cluster-handler/src/handler/deploy.rs`), before spawning the executor, check if artifact is `BlobHash`. If `ctx.blob_store` is `None`, return `NodeUpgradeResult { is_accepted: false }` with error.
- [x] 4.2 When `blob_store` is `Some`, download the blob to `{staging_dir}/aspen-node-{blob_hash}` using `iroh-blobs` get/export. Bound the download with a timeout.
- [x] 4.3 Only spawn the executor after the blob is staged on disk.
- [x] 4.4 Add test: BlobHash artifact with `blob_store: None` returns rejected with clear error message.

## 5. Leader Failover Resume

- [x] 5.1 Identify the leader transition hook in `aspen-cluster/src/bootstrap/node/mod.rs` — find where Raft metrics are watched and leader state is detected.
- [x] 5.2 After a node transitions to Leader, spawn a background task that creates a `DeploymentCoordinator` and calls `check_and_resume()`. Use the existing KV store, IrohNodeRpcClient (from task 2.1), and node_id from the bootstrap context.
- [x] 5.3 Log the outcome: "resumed deployment {id}" or "no in-progress deployment" or "resume failed: {error}".
- [x] 5.4 Add test: write a Deploying record to KV with mixed node states, call `check_and_resume()`, verify remaining pending nodes get upgraded and deployment completes.

## 6. Health Check Depth

- [x] 6.1 Add `MAX_HEALTHY_LOG_GAP: u64 = 100` to `aspen-constants/src/api.rs` (or the appropriate constants module).
- [x] 6.2 Add `controller: Arc<dyn ClusterController>` field to `IrohNodeRpcClient` (it already has it from task 2.1 extraction).
- [x] 6.3 In `IrohNodeRpcClient::check_health()`, after the GetHealth RPC succeeds, call `controller.get_metrics()` locally. Check the `replication` map for the target node's `matched_log_index`. If absent or gap exceeds `MAX_HEALTHY_LOG_GAP`, return `Ok(false)`.
- [x] 6.4 Add test: mock a controller whose metrics show a large log gap for a target node, verify `check_health()` returns `Ok(false)` even though GetHealth returned "healthy".
- [x] 6.5 Add test: mock a controller whose metrics show the target node within threshold, verify `check_health()` returns `Ok(true)`.

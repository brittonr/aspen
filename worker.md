# VM as Ephemeral Aspen Node (CI Runner)

## Problem Summary

Nix build outputs in VM jobs are not being uploaded to the SNIX binary cache. The current lightweight aspen-ci-agent lacks Iroh/SNIX integration. Instead of bolting on these dependencies, the VM should run aspen-node as an ephemeral CI runner.

## Architecture: VM as CI Runner

The VM runs an Aspen node that:

1. Joins the cluster via Iroh ticket (passed at boot)
2. Registers as a worker for CI jobs (like GitLab runner)
3. Executes jobs locally (same as LocalExecutorWorker)
4. Uploads to SNIX directly (has full cluster access)
5. No Raft voting - ephemeral worker, not a consensus participant

This is essentially LocalExecutorWorker running inside an isolated VM, with full SNIX capabilities.

## Goals

- **Primary**: VM uploads nix build outputs to SNIX binary cache directly
- **Secondary**: Simplify architecture - VM is a cluster participant, not a special case
- **Bonus**: Job outputs already stored in blobs for debugging

## Implementation Status

| Phase | Description | Status |
| ----- | ----------- | ------ |
| 1 | Add "Worker Mode" to aspen-node | Done |
| 2 | Ephemeral Worker Startup Flow | Done |
| 3 | RPC-based SNIX Services | Done |
| 4 | SNIX RPC Handlers | Done |
| 5 | VM NixOS Configuration | Done |
| 6 | CloudHypervisorWorker Updates | Done |
| 7 | Job Routing for VM Workers | Pending |

## Phase 1: Add "Worker Mode" to aspen-node (Done)

**File**: `src/bin/aspen-node.rs`

Added `--worker-only` CLI flag and environment variable detection:

```rust
// CLI flag
#[arg(long)]
worker_only: bool,

// Environment detection
let worker_only_mode = args.worker_only
    || std::env::var("ASPEN_MODE")
        .map(|v| v.to_lowercase() == "ci_worker" || v.to_lowercase() == "worker_only")
        .unwrap_or(false);
```

Configure via environment:

- `ASPEN_MODE=ci_worker` - Enable worker mode
- `ASPEN_CLUSTER_TICKET` or `ASPEN_CLUSTER_TICKET_FILE` - Cluster connection ticket
- `ASPEN_WORKER_JOB_TYPES=ci_vm` - Job types to handle

## Phase 2: Ephemeral Worker Startup Flow (Done)

**File**: `src/bin/aspen-node.rs` - `run_worker_only_mode()` function

In worker mode, the node:

1. Parses cluster ticket to get Iroh connection info
2. Creates Iroh endpoint (generates ephemeral key, no persistence)
3. Connects to cluster via bootstrap peers in ticket
4. Joins gossip topic for cluster discovery
5. Creates SNIX service clients that route through Iroh RPC
6. Creates LocalExecutorWorker with SNIX services
7. Processes jobs from queue until shutdown

## Phase 3: RPC-based SNIX Services (Done)

**File**: `crates/aspen-snix/src/rpc_directory_service.rs`

```rust
pub struct RpcDirectoryService {
    endpoint: Arc<Endpoint>,
    gateway_node: PublicKey,
}

impl DirectoryService for RpcDirectoryService {
    async fn get(&self, digest: &B3Digest) -> Result<Option<Directory>, Error>;
    async fn put(&self, directory: Directory) -> Result<B3Digest, Error>;
    fn get_recursive(&self, root: &B3Digest) -> BoxStream<'_, Result<Directory, Error>>;
    fn put_multiple_start(&self) -> Box<dyn DirectoryPutter + '_>;
}
```

**File**: `crates/aspen-snix/src/rpc_pathinfo_service.rs`

```rust
pub struct RpcPathInfoService {
    endpoint: Arc<Endpoint>,
    gateway_node: PublicKey,
}

impl PathInfoService for RpcPathInfoService {
    async fn get(&self, digest: [u8; 20]) -> Result<Option<PathInfo>, Error>;
    async fn put(&self, path_info: PathInfo) -> Result<PathInfo, Error>;
    fn list(&self) -> BoxStream<'static, Result<PathInfo, Error>>;
}
```

## Phase 4: SNIX RPC Handlers (Done)

**File**: `crates/aspen-rpc-handlers/src/handlers/snix.rs`

**File**: `crates/aspen-client-api/src/messages.rs`

Added RPC message types:

```rust
// Requests
ClientRpcRequest::SnixDirectoryGet { digest: String }
ClientRpcRequest::SnixDirectoryPut { directory_bytes: String }
ClientRpcRequest::SnixPathInfoGet { digest: String }
ClientRpcRequest::SnixPathInfoPut { pathinfo_bytes: String }

// Responses
ClientRpcResponse::SnixDirectoryGetResult(SnixDirectoryGetResultResponse)
ClientRpcResponse::SnixDirectoryPutResult(SnixDirectoryPutResultResponse)
ClientRpcResponse::SnixPathInfoGetResult(SnixPathInfoGetResultResponse)
ClientRpcResponse::SnixPathInfoPutResult(SnixPathInfoPutResultResponse)
```

The `SnixHandler` uses the KV store directly with keys:

- `snix:dir:{digest}` - Directory entries (base64-encoded protobuf)
- `snix:pathinfo:{digest}` - PathInfo entries (base64-encoded protobuf)

## Phase 5: VM NixOS Configuration (Done)

**File**: `nix/vms/ci-worker-node.nix`

Changed from `aspen-ci-agent` to `aspen-node --worker-only`:

```nix
systemd.services.aspen-ci-worker = {
  description = "Aspen CI Worker (Ephemeral Node)";
  wantedBy = ["multi-user.target"];
  after = ["local-fs.target" "nix-daemon.service" "network-online.target"];

  serviceConfig = {
    ExecStart = "${aspenNodePackage}/bin/aspen-node --worker-only";
    Restart = "always";
    WorkingDirectory = "/workspace";
  };

  environment = {
    ASPEN_MODE = "ci_worker";
    ASPEN_CLUSTER_TICKET_FILE = "/workspace/.aspen-cluster-ticket";
    ASPEN_WORKER_JOB_TYPES = "ci_vm";
    ASPEN_CI_WORKSPACE_DIR = "/workspace";
  };
};
```

**File**: `flake.nix`

Updated to use `aspenNodePackage` instead of `aspenCiAgentPackage`.

## Phase 6: CloudHypervisorWorker Updates (Done)

**Files**:

- `crates/aspen-ci/src/workers/cloud_hypervisor/config.rs`
- `crates/aspen-ci/src/workers/cloud_hypervisor/vm.rs`
- `crates/aspen-ci/src/workers/cloud_hypervisor/worker.rs`

CloudHypervisorWorker transformed from a job executor to a VM pool manager:

1. **Config changes**: Added `cluster_ticket` field for VM workers to join cluster
2. **VM startup**: Writes cluster ticket to `/workspace/.aspen-cluster-ticket` during boot
3. **Removed vsock execution**: VMs no longer receive jobs via vsock protocol
4. **Worker trait simplified**: `execute()` returns error, `job_types()` returns empty list
5. **Pool management**: Maintains VM lifecycle but doesn't execute jobs

Key changes:

```rust
// CloudHypervisorWorkerConfig now includes:
pub cluster_ticket: Option<String>,

// CloudHypervisorWorker.job_types() returns empty - no direct job handling:
fn job_types(&self) -> Vec<String> {
    vec![]  // VMs register themselves as workers
}

// VM startup writes ticket to workspace:
if let Some(ref ticket) = self.config.cluster_ticket {
    let ticket_path = self.config.cluster_ticket_path(&self.id);
    tokio::fs::write(&ticket_path, ticket).await?;
}
```

The CloudHypervisorWorker is now a **VM pool manager** - VMs handle jobs directly via the cluster.

## Phase 7: Job Routing for VM Workers (Pending)

**File**: `crates/aspen-jobs/src/worker_service.rs`

Update worker registration to support remote workers:

```rust
// VM workers register with their Iroh endpoint ID
pub struct WorkerInfo {
    pub endpoint_id: PublicKey,
    pub job_types: Vec<String>,
    pub is_ephemeral: bool,
}

// Job assignment considers worker location
impl WorkerService {
    async fn assign_job(&self, job: Job) -> Result<WorkerId> {
        // Find available worker for job type
        // Prefer local workers, but route to VM workers if needed
    }
}
```

## Files Modified/Created

| File | Status | Description |
| ---- | ------ | ----------- |
| `src/bin/aspen-node.rs` | Modified | Added `--worker-only` mode and startup flow |
| `crates/aspen-snix/src/rpc_directory_service.rs` | Created | RPC client for DirectoryService |
| `crates/aspen-snix/src/rpc_pathinfo_service.rs` | Created | RPC client for PathInfoService |
| `crates/aspen-snix/src/lib.rs` | Modified | Export new RPC services |
| `crates/aspen-snix/Cargo.toml` | Modified | Add iroh, postcard dependencies |
| `crates/aspen-rpc-handlers/src/handlers/snix.rs` | Created | SNIX RPC handlers |
| `crates/aspen-rpc-handlers/src/handlers/mod.rs` | Modified | Export SnixHandler |
| `crates/aspen-rpc-handlers/src/registry.rs` | Modified | Register SnixHandler |
| `crates/aspen-rpc-handlers/Cargo.toml` | Modified | Add snix dependencies |
| `crates/aspen-client-api/src/messages.rs` | Modified | Add SNIX RPC message types |
| `nix/vms/ci-worker-node.nix` | Modified | Run aspen-node instead of aspen-ci-agent |
| `flake.nix` | Modified | Use aspenNodePackage |
| `crates/aspen-ci/src/workers/cloud_hypervisor/config.rs` | Modified | Added cluster_ticket field |
| `crates/aspen-ci/src/workers/cloud_hypervisor/vm.rs` | Modified | Write cluster ticket, remove guest agent |
| `crates/aspen-ci/src/workers/cloud_hypervisor/worker.rs` | Modified | VM pool manager, removed vsock execution |

## Key Architecture Decisions

1. **Ephemeral identity**: VM generates new Iroh key on each boot. No persistent identity.
2. **No Raft participation**: VM connects as Iroh peer only. All consensus goes through cluster nodes.
3. **RPC for metadata**: Directory/PathInfo services use RPC to cluster. Only blobs use direct Iroh transfer.
4. **Job routing**: Jobs tagged for VM execution route to VM workers via normal job queue.
5. **Graceful degradation**: If VM can't connect to cluster, job fails with clear error.

## Testing Strategy

1. **Unit tests**: RPC service serialization, worker mode config parsing
2. **Integration tests**: VM boots, joins cluster, processes job, uploads to SNIX
3. **Chaos tests**: VM disconnect during job, cluster node failure during upload

## Future Extension: Federated Build Service

The architecture should not preclude a future **federation model** where:

1. **Federated Binary Caches**: Multiple Aspen clusters share build artifacts
   - Content-addressed storage (BLAKE3) makes deduplication natural
   - Clusters can pull artifacts from federated peers
   - Build once, use everywhere

2. **Cross-Cluster Build Requests**: Any federated cluster can request builds
   - Worker pools serve multiple clusters
   - Jobs tagged with origin cluster for result routing
   - Artifacts replicate to requesting cluster's cache

3. **Design Considerations for Federation**:
   - Cluster identity: Each cluster has unique Iroh topic/gossip namespace
   - Worker affinity: Workers can join multiple cluster topics
   - Artifact routing: SNIX PathInfo includes origin cluster metadata
   - Trust model: Federated clusters share signing keys or cross-sign

4. **What to Avoid in Current Implementation**:
   - Hard-coding single cluster assumptions in worker registration
   - Storing cluster-specific data in worker identity
   - Assuming all SNIX data lives in one Raft group

The current single-cluster implementation uses the same Iroh/SNIX primitives that federation would use, just scoped to one cluster.

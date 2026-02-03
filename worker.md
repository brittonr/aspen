# VM as Ephemeral Aspen Node (CI Runner)

 Problem Summary

 Nix build outputs in VM jobs are not being uploaded to the SNIX binary cache. The current lightweight aspen-ci-agent lacks
 Iroh/SNIX integration. Instead of bolting on these dependencies, the VM should run aspen-node as an ephemeral CI runner.

 Architecture: VM as CI Runner

 The VM runs an Aspen node that:

 1. Joins the cluster via Iroh ticket (passed at boot)
 2. Registers as a worker for CI jobs (like GitLab runner)
 3. Executes jobs locally (same as LocalExecutorWorker)
 4. Uploads to SNIX directly (has full cluster access)
 5. No Raft voting - ephemeral worker, not a consensus participant

 This is essentially LocalExecutorWorker running inside an isolated VM, with full SNIX capabilities.

 Goals

- Primary: VM uploads nix build outputs to SNIX binary cache directly
- Secondary: Simplify architecture - VM is a cluster participant, not a special case
- Bonus: Job outputs already stored in blobs for debugging

 Implementation Plan

 Phase 1: Add "Worker Mode" to aspen-node

 File: src/bin/aspen-node.rs

 Add a new operational mode for ephemeral CI workers:

 /// Node operational mode
 enum NodeMode {
     /// Full cluster node with Raft consensus
     Full,
     /// Ephemeral CI worker - no Raft, connects via Iroh only
     CiWorker {
         cluster_ticket: String,
         job_types: Vec<String>,
     },
 }

 Configure via environment:

- ASPEN_MODE=ci_worker - Enable worker mode
- ASPEN_CLUSTER_TICKET=aspenv2... - Cluster connection ticket
- ASPEN_WORKER_JOB_TYPES=ci_vm,ci_shell - Job types to handle

 Phase 2: Ephemeral Worker Startup Flow

 File: src/bin/aspen-node.rs (new section)

 In worker mode, the node:

 1. Parses cluster ticket to get Iroh connection info
 2. Creates Iroh endpoint (generates ephemeral key, no persistence)
 3. Connects to cluster via bootstrap peers in ticket
 4. Creates SNIX service clients that route through Iroh:

- IrohBlobService - connects to cluster blob store
- RpcDirectoryService - calls cluster via Iroh RPC
- RpcPathInfoService - calls cluster via Iroh RPC

 5. Registers LocalExecutorWorker with SNIX services
 6. Processes jobs from queue until shutdown

 Phase 3: Add RPC-based SNIX Services for Remote Access

 File: crates/aspen-snix/src/rpc_directory_service.rs (NEW)

 Create client that calls DirectoryService via Iroh RPC:

 pub struct RpcDirectoryService {
     endpoint: Arc<Endpoint>,
     gateway_node: PublicKey,
 }

 impl DirectoryService for RpcDirectoryService {
     async fn get(&self, digest: &B3Digest) -> Result<Option<Directory>> {
         // Send RPC request via Iroh to cluster node
         // Cluster node looks up in RaftDirectoryService
     }

     async fn put(&self, directory: Directory) -> Result<B3Digest> {
         // Send RPC request via Iroh to cluster node
     }
 }

 File: crates/aspen-snix/src/rpc_pathinfo_service.rs (NEW)

 Same pattern for PathInfoService.

 Phase 4: Add SNIX RPC Handlers to Cluster Nodes

 File: crates/aspen-rpc-handlers/src/handlers/snix.rs (NEW)

 Add RPC handlers for SNIX operations:

 pub enum SnixRpcRequest {
     DirectoryGet { digest: B3Digest },
     DirectoryPut { directory: Directory },
     PathInfoGet { digest: [u8; 20] },
     PathInfoPut { path_info: PathInfo },
 }

 pub enum SnixRpcResponse {
     DirectoryGet { directory: Option<Directory> },
     DirectoryPut { digest: B3Digest },
     PathInfoGet { path_info: Option<PathInfo> },
     PathInfoPut { path_info: PathInfo },
 }

 Phase 5: Update VM NixOS Configuration

 File: nix/vms/ci-worker-node.nix

 Change VM to run aspen-node instead of aspen-ci-agent:

 {
   systemd.services.aspen-ci-worker = {
     description = "Aspen CI Worker";
     wantedBy = [ "multi-user.target" ];
     environment = {
       ASPEN_MODE = "ci_worker";
       # Ticket injected via virtiofs config file
       ASPEN_CLUSTER_TICKET_FILE = "/workspace/.aspen-cluster-ticket";
       ASPEN_WORKER_JOB_TYPES = "ci_vm";
       ASPEN_CI_WORKSPACE_DIR = "/workspace";
     };
     serviceConfig = {
       ExecStart = "${aspen-node}/bin/aspen-node";
       Restart = "on-failure";
     };
   };
 }

 Phase 6: Update CloudHypervisorWorker for New Architecture

 File: crates/aspen-ci/src/workers/cloud_hypervisor/worker.rs

 Change from "send job via vsock" to "write ticket file, let VM pick up job":

 1. Boot VM with aspen-node (not aspen-ci-agent)
 2. Write cluster ticket to /workspace/.aspen-cluster-ticket
 3. VM node joins cluster and registers as worker
 4. Job routing: Job queue routes ci_vm jobs to VM workers
 5. Results: VM worker completes job, uploads to SNIX, updates job status

 The CloudHypervisorWorker becomes a VM pool manager rather than a job executor.

 Phase 7: Job Routing for VM Workers

 File: crates/aspen-jobs/src/worker_service.rs

 Update worker registration to support remote workers:

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

 Files to Create/Modify

 | File                                                   | Changes                                    |
 |--------------------------------------------------------|--------------------------------------------|
 | src/bin/aspen-node.rs                                  | Add ci_worker mode, ephemeral startup flow |
 | crates/aspen-snix/src/rpc_directory_service.rs         | NEW - RPC client for DirectoryService      |
 | crates/aspen-snix/src/rpc_pathinfo_service.rs          | NEW - RPC client for PathInfoService       |
 | crates/aspen-rpc-handlers/src/handlers/snix.rs         | NEW - SNIX RPC handlers                    |
 | crates/aspen-client-api/src/messages.rs                | Add SNIX RPC message types                 |
 | nix/vms/ci-worker-node.nix                             | Run aspen-node instead of aspen-ci-agent   |
 | crates/aspen-ci/src/workers/cloud_hypervisor/worker.rs | Become VM pool manager                     |
 | crates/aspen-jobs/src/worker_service.rs                | Support remote worker registration         |

 Key Architecture Decisions

 1. Ephemeral identity: VM generates new Iroh key on each boot. No persistent identity.
 2. No Raft participation: VM connects as Iroh peer only. All consensus goes through cluster nodes.
 3. RPC for metadata: Directory/PathInfo services use RPC to cluster. Only blobs use direct Iroh transfer.
 4. Job routing: Jobs tagged for VM execution route to VM workers via normal job queue.
 5. Graceful degradation: If VM can't connect to cluster, job fails with clear error.

 Migration Path

 1. Phase 1: Keep existing vsock-based approach working
 2. Phase 2: Add worker mode to aspen-node, test locally
 3. Phase 3: Add RPC services, test cluster connectivity
 4. Phase 4: Update VM image to use aspen-node
 5. Phase 5: Deprecate aspen-ci-agent for VM use

 Testing Strategy

 1. Unit tests: RPC service serialization, worker mode config parsing
 2. Integration tests: VM boots, joins cluster, processes job, uploads to SNIX
 3. Chaos tests: VM disconnect during job, cluster node failure during upload

 Rollback

 The vsock-based aspen-ci-agent remains available. Set ASPEN_CI_USE_LEGACY_AGENT=1 to use old path.

 Future Extension: Federated Build Service

 The architecture should not preclude a future federation model where:

 1. Federated Binary Caches: Multiple Aspen clusters share build artifacts

- Content-addressed storage (BLAKE3) makes deduplication natural
- Clusters can pull artifacts from federated peers
- Build once, use everywhere

 2. Cross-Cluster Build Requests: Any federated cluster can request builds

- Worker pools serve multiple clusters
- Jobs tagged with origin cluster for result routing
- Artifacts replicate to requesting cluster's cache

 3. Design Considerations for Federation:

- Cluster identity: Each cluster has unique Iroh topic/gossip namespace
- Worker affinity: Workers can join multiple cluster topics
- Artifact routing: SNIX PathInfo includes origin cluster metadata
- Trust model: Federated clusters share signing keys or cross-sign

 4. What to Avoid in Current Implementation:

- Hard-coding single cluster assumptions in worker registration
- Storing cluster-specific data in worker identity
- Assuming all SNIX data lives in one Raft group

 The current single-cluster implementation uses the same Iroh/SNIX primitives that federation would use, just scoped to one
 cluster.

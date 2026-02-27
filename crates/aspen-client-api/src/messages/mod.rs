//! Client RPC protocol message definitions.
//!
//! This module defines the RPC protocol used by clients (including aspen-tui) to
//! communicate with aspen-node over Iroh P2P connections. It is separate from the
//! Raft RPC protocol which is used for cluster-internal consensus communication.
//!
//! # Architecture
//!
//! The Client RPC uses a distinct ALPN (`aspen-client`) to distinguish it from Raft RPC.
//! This allows clients to connect directly to nodes without needing HTTP.
//!
//! # Tiger Style
//!
//! - Explicit request/response pairs
//! - Bounded message sizes
//! - Fail-fast on invalid requests

// Sub-modules
pub mod automerge;
pub mod batch;
pub mod blob;
pub mod ci;
pub mod cluster;
pub mod coordination;
pub mod docs;
pub mod federation;
pub mod forge;
pub mod hooks;
pub mod jobs;
pub mod kv;
pub mod lease;
pub mod observability;
pub mod secrets;
pub mod sql;
#[cfg(feature = "auth")]
mod to_operation;
pub mod watch;

// Domain request enum re-exports
// Feature-gated re-exports
#[cfg(feature = "automerge")]
pub use automerge::AutomergeApplyChangesResultResponse;
#[cfg(feature = "automerge")]
pub use automerge::AutomergeCreateResultResponse;
#[cfg(feature = "automerge")]
pub use automerge::AutomergeDeleteResultResponse;
#[cfg(feature = "automerge")]
pub use automerge::AutomergeDocumentMetadata;
#[cfg(feature = "automerge")]
pub use automerge::AutomergeExistsResultResponse;
#[cfg(feature = "automerge")]
pub use automerge::AutomergeGenerateSyncMessageResultResponse;
#[cfg(feature = "automerge")]
pub use automerge::AutomergeGetMetadataResultResponse;
#[cfg(feature = "automerge")]
pub use automerge::AutomergeGetResultResponse;
#[cfg(feature = "automerge")]
pub use automerge::AutomergeListResultResponse;
#[cfg(feature = "automerge")]
pub use automerge::AutomergeMergeResultResponse;
#[cfg(feature = "automerge")]
pub use automerge::AutomergeReceiveSyncMessageResultResponse;
#[cfg(feature = "automerge")]
pub use automerge::AutomergeRequest;
#[cfg(feature = "automerge")]
pub use automerge::AutomergeSaveResultResponse;
pub use batch::BatchRequest;
// Re-export all types from sub-modules for backwards compatibility
pub use batch::{
    BatchCondition, BatchReadResultResponse, BatchWriteOperation, BatchWriteResultResponse,
    ConditionalBatchWriteResultResponse,
};
pub use blob::AddBlobResultResponse;
pub use blob::BlobListEntry;
pub use blob::BlobReplicatePullResultResponse;
pub use blob::BlobRequest;
pub use blob::DeleteBlobResultResponse;
pub use blob::DownloadBlobResultResponse;
pub use blob::GetBlobReplicationStatusResultResponse;
pub use blob::GetBlobResultResponse;
pub use blob::GetBlobStatusResultResponse;
pub use blob::GetBlobTicketResultResponse;
pub use blob::HasBlobResultResponse;
pub use blob::ListBlobsResultResponse;
pub use blob::ProtectBlobResultResponse;
pub use blob::RunBlobRepairCycleResultResponse;
pub use blob::TriggerBlobReplicationResultResponse;
pub use blob::UnprotectBlobResultResponse;
pub use ci::CacheDownloadResultResponse;
pub use ci::CacheEntryResponse;
#[cfg(feature = "ci")]
pub use ci::CacheMigrationCancelResultResponse;
#[cfg(feature = "ci")]
pub use ci::CacheMigrationProgressResponse;
#[cfg(feature = "ci")]
pub use ci::CacheMigrationStartResultResponse;
#[cfg(feature = "ci")]
pub use ci::CacheMigrationStatusResultResponse;
#[cfg(feature = "ci")]
pub use ci::CacheMigrationValidateResultResponse;
pub use ci::CacheQueryResultResponse;
pub use ci::CacheStatsResultResponse;
pub use ci::CiArtifactInfo;
pub use ci::CiCancelRunResponse;
pub use ci::CiGetArtifactResponse;
pub use ci::CiGetJobLogsResponse;
pub use ci::CiGetJobOutputResponse;
pub use ci::CiGetStatusResponse;
pub use ci::CiJobInfo;
pub use ci::CiListArtifactsResponse;
pub use ci::CiListRunsResponse;
pub use ci::CiLogChunkInfo;
pub use ci::CiRequest;
pub use ci::CiRunInfo;
pub use ci::CiStageInfo;
pub use ci::CiSubscribeLogsResponse;
pub use ci::CiTriggerPipelineResponse;
pub use ci::CiUnwatchRepoResponse;
pub use ci::CiWatchRepoResponse;
pub use ci::SnixDirectoryGetResultResponse;
pub use ci::SnixDirectoryPutResultResponse;
pub use ci::SnixPathInfoGetResultResponse;
pub use ci::SnixPathInfoPutResultResponse;
pub use cluster::AddLearnerResultResponse;
pub use cluster::AddPeerResultResponse;
pub use cluster::ChangeMembershipResultResponse;
pub use cluster::CheckpointWalResultResponse;
pub use cluster::ClientTicketResponse;
pub use cluster::ClusterRequest;
pub use cluster::ClusterStateResponse;
pub use cluster::ClusterTicketResponse;
pub use cluster::CompareAndSwapResultResponse;
pub use cluster::ErrorResponse;
pub use cluster::HealthResponse;
pub use cluster::InitResultResponse;
pub use cluster::MetricsResponse;
pub use cluster::NodeDescriptor;
pub use cluster::NodeInfoResponse;
pub use cluster::PromoteLearnerResultResponse;
pub use cluster::RaftMetricsResponse;
pub use cluster::ReadResultResponse;
pub use cluster::ReplicationProgress;
pub use cluster::SnapshotResultResponse;
pub use cluster::TopologyResultResponse;
pub use cluster::WriteResultResponse;
pub use coordination::BarrierResultResponse;
pub use coordination::CoordinationRequest;
pub use coordination::CounterResultResponse;
pub use coordination::LockResultResponse;
pub use coordination::QueueAckResultResponse;
pub use coordination::QueueCreateResultResponse;
pub use coordination::QueueDLQItemResponse;
pub use coordination::QueueDeleteResultResponse;
pub use coordination::QueueDequeueResultResponse;
pub use coordination::QueueDequeuedItemResponse;
pub use coordination::QueueEnqueueBatchResultResponse;
pub use coordination::QueueEnqueueItem;
pub use coordination::QueueEnqueueResultResponse;
pub use coordination::QueueExtendVisibilityResultResponse;
pub use coordination::QueueGetDLQResultResponse;
pub use coordination::QueueItemResponse;
pub use coordination::QueueNackResultResponse;
pub use coordination::QueuePeekResultResponse;
pub use coordination::QueueRedriveDLQResultResponse;
pub use coordination::QueueStatusResultResponse;
pub use coordination::RWLockResultResponse;
pub use coordination::RateLimiterResultResponse;
pub use coordination::SemaphoreResultResponse;
pub use coordination::SequenceResultResponse;
pub use coordination::ServiceDeregisterResultResponse;
pub use coordination::ServiceDiscoverResultResponse;
pub use coordination::ServiceGetInstanceResultResponse;
pub use coordination::ServiceHeartbeatResultResponse;
pub use coordination::ServiceInstanceResponse;
pub use coordination::ServiceListResultResponse;
pub use coordination::ServiceRegisterResultResponse;
pub use coordination::ServiceUpdateHealthResultResponse;
pub use coordination::ServiceUpdateMetadataResultResponse;
pub use coordination::SignedCounterResultResponse;
pub use docs::AddPeerClusterResultResponse;
pub use docs::DocsDeleteResultResponse;
pub use docs::DocsGetResultResponse;
pub use docs::DocsListEntry;
pub use docs::DocsListResultResponse;
pub use docs::DocsRequest;
pub use docs::DocsSetResultResponse;
pub use docs::DocsStatusResultResponse;
pub use docs::DocsTicketResponse;
pub use docs::KeyOriginResultResponse;
pub use docs::ListPeerClustersResultResponse;
pub use docs::PeerClusterInfo;
pub use docs::PeerClusterStatusResponse;
pub use docs::RemovePeerClusterResultResponse;
pub use docs::SetPeerClusterEnabledResultResponse;
pub use docs::UpdatePeerClusterFilterResultResponse;
pub use docs::UpdatePeerClusterPriorityResultResponse;
pub use federation::DiscoveredClusterInfo;
pub use federation::DiscoveredClusterResponse;
pub use federation::DiscoveredClustersResponse;
pub use federation::FederateRepositoryResultResponse;
pub use federation::FederatedRepoInfo;
pub use federation::FederatedRepositoriesResponse;
pub use federation::FederationRequest;
pub use federation::FederationStatusResponse;
pub use federation::ForgeFetchFederatedResultResponse;
pub use federation::TrustClusterResultResponse;
pub use federation::UntrustClusterResultResponse;
pub use forge::ForgeBlobResultResponse;
pub use forge::ForgeCommentInfo;
pub use forge::ForgeCommitInfo;
pub use forge::ForgeCommitResultResponse;
pub use forge::ForgeIssueInfo;
pub use forge::ForgeIssueListResultResponse;
pub use forge::ForgeIssueResultResponse;
pub use forge::ForgeKeyResultResponse;
pub use forge::ForgeLogResultResponse;
pub use forge::ForgeOperationResultResponse;
pub use forge::ForgePatchApproval;
pub use forge::ForgePatchInfo;
pub use forge::ForgePatchListResultResponse;
pub use forge::ForgePatchResultResponse;
pub use forge::ForgePatchRevision;
pub use forge::ForgeRefInfo;
pub use forge::ForgeRefListResultResponse;
pub use forge::ForgeRefResultResponse;
pub use forge::ForgeRepoInfo;
pub use forge::ForgeRepoListResultResponse;
pub use forge::ForgeRepoResultResponse;
pub use forge::ForgeRequest;
pub use forge::ForgeTreeEntry;
pub use forge::ForgeTreeResultResponse;
pub use forge::GitBridgeFetchResponse;
pub use forge::GitBridgeListRefsResponse;
pub use forge::GitBridgeObject;
pub use forge::GitBridgeProbeObjectsResponse;
pub use forge::GitBridgePushChunkResponse;
pub use forge::GitBridgePushCompleteResponse;
pub use forge::GitBridgePushMetadata;
pub use forge::GitBridgePushResponse;
pub use forge::GitBridgePushStartResponse;
pub use forge::GitBridgeRefInfo;
pub use forge::GitBridgeRefResult;
pub use forge::GitBridgeRefUpdate;
pub use hooks::HookHandlerInfo;
pub use hooks::HookHandlerMetrics;
pub use hooks::HookListResultResponse;
pub use hooks::HookMetricsResultResponse;
pub use hooks::HookTriggerResultResponse;
pub use hooks::HooksRequest;
pub use jobs::JobCancelResultResponse;
pub use jobs::JobDetails;
pub use jobs::JobGetResultResponse;
pub use jobs::JobListResultResponse;
pub use jobs::JobQueueStatsResultResponse;
pub use jobs::JobSubmitResultResponse;
pub use jobs::JobUpdateProgressResultResponse;
pub use jobs::JobsRequest;
pub use jobs::PriorityCount;
pub use jobs::TypeCount;
pub use jobs::WorkerCompleteJobResultResponse;
pub use jobs::WorkerDeregisterResultResponse;
pub use jobs::WorkerHeartbeatResultResponse;
pub use jobs::WorkerInfo;
pub use jobs::WorkerJobInfo;
pub use jobs::WorkerPollJobsResultResponse;
pub use jobs::WorkerRegisterResultResponse;
pub use jobs::WorkerStatusResultResponse;
pub use kv::DeleteResultResponse;
pub use kv::IndexCreateResultResponse;
pub use kv::IndexDefinitionWire;
pub use kv::IndexDropResultResponse;
pub use kv::IndexListResultResponse;
pub use kv::IndexScanResultResponse;
pub use kv::KvRequest;
pub use kv::ScanEntry;
pub use kv::ScanResultResponse;
pub use lease::LeaseGrantResultResponse;
pub use lease::LeaseInfo;
pub use lease::LeaseKeepaliveResultResponse;
pub use lease::LeaseListResultResponse;
pub use lease::LeaseRequest;
pub use lease::LeaseRevokeResultResponse;
pub use lease::LeaseTimeToLiveResultResponse;
// Metric types
pub use observability::AlertComparison;
pub use observability::AlertEvaluateResultResponse;
pub use observability::AlertGetResultResponse;
pub use observability::AlertHistoryEntry;
pub use observability::AlertListResultResponse;
pub use observability::AlertRuleResultResponse;
pub use observability::AlertRuleWire;
pub use observability::AlertRuleWithState;
pub use observability::AlertSeverity;
pub use observability::AlertStateWire;
pub use observability::AlertStatus;
pub use observability::IngestSpan;
pub use observability::MetricDataPoint;
pub use observability::MetricIngestResultResponse;
pub use observability::MetricListResultResponse;
pub use observability::MetricMetadata;
pub use observability::MetricQueryResultResponse;
pub use observability::MetricTypeWire;
pub use observability::SpanEventWire;
pub use observability::SpanStatusWire;
pub use observability::TraceGetResultResponse;
pub use observability::TraceIngestResultResponse;
pub use observability::TraceListResultResponse;
pub use observability::TraceSearchResultResponse;
pub use observability::TraceSummary;
pub use secrets::SecretsKvDeleteResultResponse;
pub use secrets::SecretsKvListResultResponse;
pub use secrets::SecretsKvMetadataResultResponse;
pub use secrets::SecretsKvReadResultResponse;
pub use secrets::SecretsKvVersionInfo;
pub use secrets::SecretsKvVersionMetadata;
pub use secrets::SecretsKvWriteResultResponse;
pub use secrets::SecretsNixCacheDeleteResultResponse;
pub use secrets::SecretsNixCacheKeyResultResponse;
pub use secrets::SecretsNixCacheListResultResponse;
pub use secrets::SecretsPkiCertificateResultResponse;
pub use secrets::SecretsPkiCrlResultResponse;
pub use secrets::SecretsPkiListResultResponse;
pub use secrets::SecretsPkiRevokeResultResponse;
pub use secrets::SecretsPkiRoleConfig;
pub use secrets::SecretsPkiRoleResultResponse;
pub use secrets::SecretsRequest;
pub use secrets::SecretsTransitDatakeyResultResponse;
pub use secrets::SecretsTransitDecryptResultResponse;
pub use secrets::SecretsTransitEncryptResultResponse;
pub use secrets::SecretsTransitKeyResultResponse;
pub use secrets::SecretsTransitListResultResponse;
pub use secrets::SecretsTransitSignResultResponse;
pub use secrets::SecretsTransitVerifyResultResponse;
pub use secrets::VaultInfo;
pub use secrets::VaultKeyValue;
pub use secrets::VaultKeysResponse;
pub use secrets::VaultListResponse;
use serde::Deserialize;
use serde::Serialize;
pub use sql::SqlCellValue;
pub use sql::SqlRequest;
pub use sql::SqlResultResponse;
pub use watch::WatchCancelResultResponse;
pub use watch::WatchCreateResultResponse;
pub use watch::WatchEventResponse;
pub use watch::WatchEventType;
pub use watch::WatchInfo;
pub use watch::WatchKeyEvent;
pub use watch::WatchRequest;
pub use watch::WatchStatusResultResponse;

/// Maximum Client RPC message size (4 MB).
///
/// Tiger Style: Bounded to prevent memory exhaustion attacks.
/// Reduced from 256MB to 4MB after implementing chunked transfer for git bridge
/// operations. Large git pushes now use GitBridgePushStart/Chunk/Complete protocol.
pub const MAX_CLIENT_MESSAGE_SIZE: usize = 4 * 1024 * 1024;

/// Maximum number of nodes in cluster state response.
///
/// Tiger Style: Bounded to prevent memory exhaustion.
pub const MAX_CLUSTER_NODES: usize = 16;

/// ALPN protocol identifier for Client RPC.
pub const CLIENT_ALPN: &[u8] = b"aspen-client";

/// Maximum concurrent Client connections.
///
/// Tiger Style: Lower limit than Raft since client connections are less critical.
pub const MAX_CLIENT_CONNECTIONS: u32 = 50;

/// Maximum concurrent streams per Client connection.
pub const MAX_CLIENT_STREAMS_PER_CONNECTION: u32 = 10;

/// Default chunk size for git bridge chunked transfers (1 MB).
///
/// Tiger Style: Bounded to prevent memory exhaustion while allowing efficient transfer.
pub const DEFAULT_GIT_CHUNK_SIZE_BYTES: u64 = 1024 * 1024;

/// Maximum chunk size for git bridge chunked transfers (4 MB).
///
/// Tiger Style: Upper bound to prevent abuse while supporting large objects.
pub const MAX_GIT_CHUNK_SIZE_BYTES: u64 = 4 * 1024 * 1024;

/// Authenticated request wrapper for client RPC.
///
/// Wraps a `ClientRpcRequest` with an optional capability token for authorization.
/// During the migration period, the token is optional for backwards compatibility.
///
/// # Wire Format
///
/// The request is serialized as a tagged enum where the first byte indicates
/// whether it's authenticated (1) or legacy (0):
/// - Legacy: `[0, request_bytes...]`
/// - Authenticated: `[1, token_bytes_len (4 bytes), token_bytes..., request_bytes...]`
#[cfg(feature = "auth")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticatedRequest {
    /// The actual RPC request.
    pub request: ClientRpcRequest,
    /// Capability token for authorization (optional during migration).
    pub token: Option<aspen_auth::CapabilityToken>,
    /// Number of proxy hops this request has taken.
    ///
    /// Incremented each time the request is forwarded to another cluster.
    /// Used for loop detection and hop limiting (max MAX_PROXY_HOPS).
    #[serde(default)]
    pub proxy_hops: u8,
}

#[cfg(feature = "auth")]
impl AuthenticatedRequest {
    /// Create an authenticated request with a token.
    pub fn new(request: ClientRpcRequest, token: aspen_auth::CapabilityToken) -> Self {
        Self {
            request,
            token: Some(token),
            proxy_hops: 0,
        }
    }

    /// Create an unauthenticated request (legacy compatibility).
    pub fn unauthenticated(request: ClientRpcRequest) -> Self {
        Self {
            request,
            token: None,
            proxy_hops: 0,
        }
    }

    /// Create a request with a specific hop count (used by proxy service).
    pub fn with_proxy_hops(
        request: ClientRpcRequest,
        token: Option<aspen_auth::CapabilityToken>,
        proxy_hops: u8,
    ) -> Self {
        Self {
            request,
            token,
            proxy_hops,
        }
    }
}

#[cfg(feature = "auth")]
impl From<ClientRpcRequest> for AuthenticatedRequest {
    fn from(request: ClientRpcRequest) -> Self {
        Self::unauthenticated(request)
    }
}
/// Maximum number of capability hints in a CapabilityUnavailable response.
///
/// Tiger Style: Bounded to prevent response bloat.
pub const MAX_CAPABILITY_HINTS: usize = 8;

/// Hint about a cluster that can serve a capability.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityHint {
    /// Public key of the discovered cluster.
    pub cluster_key: String,
    /// Human-readable cluster name.
    pub name: String,
    /// Version of the app on this cluster (if known).
    pub app_version: Option<String>,
}

/// Response returned when a request requires an app that is not loaded on this cluster.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityUnavailableResponse {
    /// The app ID required to handle this request.
    pub required_app: String,
    /// Human-readable message.
    pub message: String,
    /// Hints about clusters that can serve this capability.
    ///
    /// Tiger Style: Bounded to MAX_CAPABILITY_HINTS.
    pub hints: Vec<CapabilityHint>,
}

/// Client RPC request protocol.
///
/// Defines all operations clients can request from a node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClientRpcRequest {
    /// Get node health status.
    GetHealth,

    /// Get Raft metrics (leader, term, log indices, etc.).
    GetRaftMetrics,

    /// Get current leader node ID.
    GetLeader,

    /// Get node information including Iroh endpoint address.
    GetNodeInfo,

    /// Get cluster ticket for peer discovery.
    GetClusterTicket,

    /// Initialize the cluster.
    InitCluster,

    /// Read a key from the key-value store.
    ReadKey {
        /// Key to read.
        key: String,
    },

    /// Write a key-value pair to the store.
    WriteKey {
        /// Key to write.
        key: String,
        /// Value to write.
        value: Vec<u8>,
    },

    /// Compare-and-swap: atomically update value if current value matches expected.
    ///
    /// - `expected: None` means the key must NOT exist (create-if-absent)
    /// - `expected: Some(val)` means the key must exist with exactly that value
    CompareAndSwapKey {
        /// Key to update.
        key: String,
        /// Expected current value (None = must not exist).
        expected: Option<Vec<u8>>,
        /// New value to set if condition matches.
        new_value: Vec<u8>,
    },

    /// Compare-and-delete: atomically delete key if current value matches expected.
    CompareAndDeleteKey {
        /// Key to delete.
        key: String,
        /// Expected current value.
        expected: Vec<u8>,
    },

    /// Trigger a snapshot.
    TriggerSnapshot,

    /// Add a learner node to the cluster.
    AddLearner {
        /// ID of the learner node.
        node_id: u64,
        /// Network address of the learner.
        addr: String,
    },

    /// Change cluster membership.
    ChangeMembership {
        /// New set of voting member IDs.
        members: Vec<u64>,
    },

    /// Ping for connection health check.
    Ping,

    /// Get cluster state with all known nodes.
    ///
    /// Returns information about all nodes in the cluster including
    /// their endpoint addresses, membership status, and role.
    GetClusterState,

    // =========================================================================
    // New operations (migrated from HTTP API)
    // =========================================================================
    /// Delete a key from the key-value store.
    DeleteKey {
        /// Key to delete.
        key: String,
    },

    /// Scan keys with prefix and pagination.
    ScanKeys {
        /// Key prefix to match (empty string matches all).
        prefix: String,
        /// Maximum results (default 1000, max 10000).
        limit: Option<u32>,
        /// Continuation token from previous scan.
        continuation_token: Option<String>,
    },

    /// Get Prometheus-format metrics.
    GetMetrics,

    /// Promote a learner node to voter.
    PromoteLearner {
        /// ID of learner to promote.
        learner_id: u64,
        /// Optional voter to replace.
        replace_node: Option<u64>,
        /// Skip safety checks if true.
        is_force: bool,
    },

    /// Manually checkpoint SQLite WAL file.
    CheckpointWal,

    /// List all vaults (key namespaces).
    ListVaults,

    /// Get keys in a specific vault.
    GetVaultKeys {
        /// Name of the vault to query.
        vault_name: String,
    },

    /// Add a peer to the network factory.
    AddPeer {
        /// Node ID of the peer.
        node_id: u64,
        /// JSON-serialized EndpointAddr.
        endpoint_addr: String,
    },

    /// Get cluster ticket with multiple bootstrap peers.
    GetClusterTicketCombined {
        /// Comma-separated endpoint IDs to include.
        endpoint_ids: Option<String>,
    },

    /// Get a client ticket for overlay subscription.
    ///
    /// Returns a ticket that clients can use to connect to this cluster
    /// as part of a priority-based overlay (like Nix binary caches).
    GetClientTicket {
        /// Access level: "read" or "write".
        access: String,
        /// Priority level (0 = highest).
        priority: u32,
    },

    /// Get a docs ticket for iroh-docs subscription.
    ///
    /// Returns a ticket for subscribing to the cluster's iroh-docs
    /// namespace for real-time state synchronization.
    GetDocsTicket {
        /// Whether client should have write access to docs.
        read_write: bool,
        /// Priority level for this subscription.
        priority: u8,
    },

    // =========================================================================
    // Blob operations (content-addressed storage)
    // =========================================================================
    /// Add a blob to the store.
    ///
    /// Stores the provided bytes and returns a blob reference with the hash.
    AddBlob {
        /// Blob data to store.
        data: Vec<u8>,
        /// Optional tag to protect the blob from GC.
        tag: Option<String>,
    },

    /// Get a blob by hash.
    ///
    /// Returns the blob data if it exists.
    GetBlob {
        /// BLAKE3 hash of the blob (hex-encoded).
        hash: String,
    },

    /// Check if a blob exists.
    HasBlob {
        /// BLAKE3 hash of the blob (hex-encoded).
        hash: String,
    },

    /// Get a ticket for sharing a blob.
    ///
    /// Returns a BlobTicket that can be used to download the blob from this node.
    GetBlobTicket {
        /// BLAKE3 hash of the blob (hex-encoded).
        hash: String,
    },

    /// List blobs in the store.
    ListBlobs {
        /// Maximum number of blobs to return.
        limit: u32,
        /// Continuation token from previous list call.
        continuation_token: Option<String>,
    },

    /// Protect a blob from garbage collection.
    ProtectBlob {
        /// BLAKE3 hash of the blob (hex-encoded).
        hash: String,
        /// Tag name for the protection.
        tag: String,
    },

    /// Remove protection from a blob.
    UnprotectBlob {
        /// Tag name to remove.
        tag: String,
    },

    /// Delete a blob from the store.
    ///
    /// Removes the blob and all its data. Protected blobs cannot be deleted
    /// unless force is true.
    DeleteBlob {
        /// BLAKE3 hash of the blob (hex-encoded).
        hash: String,
        /// Force deletion even if protected.
        is_force: bool,
    },

    /// Download a blob from a remote peer using a ticket.
    ///
    /// Fetches the blob from the peer specified in the ticket and stores it locally.
    DownloadBlob {
        /// Serialized BlobTicket from the remote peer.
        ticket: String,
        /// Optional tag to protect the downloaded blob from GC.
        tag: Option<String>,
    },

    /// Download a blob by hash using DHT discovery.
    ///
    /// Queries the BitTorrent Mainline DHT for providers of the given hash,
    /// then fetches the blob from the first available provider.
    /// Requires the `global-discovery` feature to be enabled.
    DownloadBlobByHash {
        /// BLAKE3 hash of the blob to download (hex-encoded).
        hash: String,
        /// Optional tag to protect the downloaded blob from GC.
        tag: Option<String>,
    },

    /// Download a blob from a specific provider using DHT mutable item lookup.
    ///
    /// Looks up the provider's DhtNodeAddr in the DHT using BEP-44 mutable items,
    /// then fetches the blob directly from that provider.
    /// Requires the `global-discovery` feature to be enabled.
    DownloadBlobByProvider {
        /// BLAKE3 hash of the blob to download (hex-encoded).
        hash: String,
        /// Public key of the provider node (hex-encoded or base32).
        provider: String,
        /// Optional tag to protect the downloaded blob from GC.
        tag: Option<String>,
    },

    /// Get detailed status information about a blob.
    ///
    /// Returns size, completion status, and protection tags.
    GetBlobStatus {
        /// BLAKE3 hash of the blob (hex-encoded).
        hash: String,
    },

    /// Request this node to download a blob from a provider for replication.
    ///
    /// This is used by the blob replication system to coordinate transfers.
    /// The source node sends this request to target nodes, asking them to
    /// pull the blob from the source using iroh-blobs P2P transfer.
    ///
    /// Flow:
    /// 1. Source node has blob locally
    /// 2. Source sends BlobReplicatePull to target
    /// 3. Target calls download_from_peer(hash, provider) to fetch
    /// 4. Target responds with success/failure
    BlobReplicatePull {
        /// BLAKE3 hash of the blob to replicate (hex-encoded).
        hash: String,
        /// Size of the blob in bytes.
        size_bytes: u64,
        /// Public key of the provider node (hex-encoded).
        /// This is the source node that has the blob.
        provider: String,
        /// Optional tag to protect the replicated blob from GC.
        tag: Option<String>,
    },

    /// Get replication status for a blob.
    ///
    /// Returns the replica set metadata including which nodes have the blob,
    /// replication policy, and health status.
    GetBlobReplicationStatus {
        /// BLAKE3 hash of the blob (hex-encoded).
        hash: String,
    },

    /// Trigger manual replication of a blob to additional nodes.
    ///
    /// Used for explicit replication control (e.g., via CLI).
    /// If target_nodes is empty, uses automatic placement.
    TriggerBlobReplication {
        /// BLAKE3 hash of the blob to replicate (hex-encoded).
        hash: String,
        /// Specific target node IDs (empty = automatic placement).
        target_nodes: Vec<u64>,
        /// Override replication factor (0 = use default policy).
        replication_factor: u32,
    },

    /// Run a full blob repair cycle.
    ///
    /// Scans for under-replicated blobs and triggers repairs in priority order:
    /// 1. Critical blobs (0 replicas) - highest data loss risk
    /// 2. UnderReplicated blobs (below min_replicas)
    /// 3. Degraded blobs (below replication_factor)
    ///
    /// This is a fire-and-forget operation that returns immediately.
    /// Use `GetBlobReplicationStatus` to monitor individual blob progress.
    RunBlobRepairCycle,

    // =========================================================================
    // Docs operations (iroh-docs CRDT replication)
    // =========================================================================
    /// Set a key-value pair in the docs namespace.
    ///
    /// Writes directly to the iroh-docs namespace for CRDT replication.
    DocsSet {
        /// The key to set.
        key: String,
        /// The value to set.
        value: Vec<u8>,
    },

    /// Get a value from the docs namespace.
    ///
    /// Reads from the local iroh-docs replica.
    DocsGet {
        /// The key to get.
        key: String,
    },

    /// Delete a key from the docs namespace.
    ///
    /// Sets a tombstone marker for CRDT deletion.
    DocsDelete {
        /// The key to delete.
        key: String,
    },

    /// List entries in the docs namespace.
    ///
    /// Returns all entries matching an optional prefix.
    DocsList {
        /// Optional prefix filter.
        prefix: Option<String>,
        /// Maximum entries to return.
        limit: Option<u32>,
    },

    /// Get docs namespace status and sync information.
    DocsStatus,

    // =========================================================================
    // Peer cluster operations (cluster-to-cluster sync)
    // =========================================================================
    /// Add a peer cluster to sync with.
    ///
    /// Subscribes to the peer cluster's iroh-docs namespace for real-time
    /// synchronization with priority-based conflict resolution.
    AddPeerCluster {
        /// Serialized AspenDocsTicket from the peer cluster.
        ticket: String,
    },

    /// Remove a peer cluster subscription.
    RemovePeerCluster {
        /// Cluster ID of the peer to remove.
        cluster_id: String,
    },

    /// List all peer cluster subscriptions.
    ListPeerClusters,

    /// Get sync status for a specific peer cluster.
    GetPeerClusterStatus {
        /// Cluster ID of the peer.
        cluster_id: String,
    },

    /// Update the subscription filter for a peer cluster.
    UpdatePeerClusterFilter {
        /// Cluster ID of the peer.
        cluster_id: String,
        /// Filter type: "full", "include", or "exclude".
        filter_type: String,
        /// Prefixes for include/exclude filters (JSON array).
        prefixes: Option<String>,
    },

    /// Update the priority for a peer cluster.
    UpdatePeerClusterPriority {
        /// Cluster ID of the peer.
        cluster_id: String,
        /// New priority (0 = highest, lower wins conflicts).
        priority: u32,
    },

    /// Enable or disable a peer cluster subscription.
    SetPeerClusterEnabled {
        /// Cluster ID of the peer.
        cluster_id: String,
        /// Whether to enable the subscription.
        #[serde(rename = "enabled")]
        is_enabled: bool,
    },

    /// Get the origin metadata for a key.
    ///
    /// Returns information about which cluster a key was imported from,
    /// including the cluster ID, priority, and timestamp.
    GetKeyOrigin {
        /// The key to look up origin for.
        key: String,
    },

    // =========================================================================
    // SQL query operations
    // =========================================================================
    /// Execute a read-only SQL query against the state machine.
    ///
    /// Only SELECT statements are allowed. The query is validated before
    /// execution and runs with `PRAGMA query_only = ON` for safety.
    ExecuteSql {
        /// SQL query string (must be SELECT or WITH...SELECT).
        query: String,
        /// Query parameters (JSON-serialized SqlValue array).
        params: String,
        /// Consistency level: "linearizable" (default) or "stale".
        consistency: String,
        /// Maximum rows to return (default 1000, max 10000).
        limit: Option<u32>,
        /// Query timeout in milliseconds (default 5000, max 30000).
        timeout_ms: Option<u32>,
    },

    // =========================================================================
    // Coordination primitives - Distributed Lock
    // =========================================================================
    /// Acquire a distributed lock with timeout.
    ///
    /// Blocks until the lock is acquired or timeout is reached.
    /// Returns a fencing token on success for safe external operations.
    LockAcquire {
        /// Lock key (unique identifier for this lock).
        key: String,
        /// Holder ID (unique identifier for this lock holder).
        holder_id: String,
        /// Lock TTL in milliseconds (how long before auto-expire).
        ttl_ms: u64,
        /// Acquire timeout in milliseconds (how long to wait).
        timeout_ms: u64,
    },

    /// Try to acquire a distributed lock without blocking.
    ///
    /// Returns immediately with success/failure.
    LockTryAcquire {
        /// Lock key.
        key: String,
        /// Holder ID.
        holder_id: String,
        /// Lock TTL in milliseconds.
        ttl_ms: u64,
    },

    /// Release a distributed lock.
    ///
    /// The fencing token must match the current lock holder.
    LockRelease {
        /// Lock key.
        key: String,
        /// Holder ID that acquired the lock.
        holder_id: String,
        /// Fencing token from acquire operation.
        fencing_token: u64,
    },

    /// Renew a distributed lock's TTL.
    ///
    /// Extends the lock deadline without releasing it.
    LockRenew {
        /// Lock key.
        key: String,
        /// Holder ID.
        holder_id: String,
        /// Fencing token from acquire operation.
        fencing_token: u64,
        /// New TTL in milliseconds.
        ttl_ms: u64,
    },

    // =========================================================================
    // Coordination primitives - Atomic Counter
    // =========================================================================
    /// Get the current value of an atomic counter.
    CounterGet {
        /// Counter key.
        key: String,
    },

    /// Increment an atomic counter by 1.
    CounterIncrement {
        /// Counter key.
        key: String,
    },

    /// Decrement an atomic counter by 1 (saturates at 0).
    CounterDecrement {
        /// Counter key.
        key: String,
    },

    /// Add an amount to an atomic counter.
    CounterAdd {
        /// Counter key.
        key: String,
        /// Amount to add.
        amount: u64,
    },

    /// Subtract an amount from an atomic counter (saturates at 0).
    CounterSubtract {
        /// Counter key.
        key: String,
        /// Amount to subtract.
        amount: u64,
    },

    /// Set an atomic counter to a specific value.
    CounterSet {
        /// Counter key.
        key: String,
        /// New value.
        value: u64,
    },

    /// Compare-and-set an atomic counter.
    ///
    /// Only updates if current value matches expected.
    CounterCompareAndSet {
        /// Counter key.
        key: String,
        /// Expected current value.
        expected: u64,
        /// New value to set.
        new_value: u64,
    },

    // =========================================================================
    // Coordination primitives - Signed Counter
    // =========================================================================
    /// Get the current value of a signed atomic counter.
    SignedCounterGet {
        /// Counter key.
        key: String,
    },

    /// Add an amount to a signed atomic counter (can be negative).
    SignedCounterAdd {
        /// Counter key.
        key: String,
        /// Amount to add (negative to subtract).
        amount: i64,
    },

    // =========================================================================
    // Coordination primitives - Sequence Generator
    // =========================================================================
    /// Get the next unique ID from a sequence.
    SequenceNext {
        /// Sequence key.
        key: String,
    },

    /// Reserve a range of IDs from a sequence.
    ///
    /// Returns the start of the reserved range [start, start+count).
    SequenceReserve {
        /// Sequence key.
        key: String,
        /// Number of IDs to reserve.
        count: u64,
    },

    /// Get the current (next available) value of a sequence without consuming it.
    SequenceCurrent {
        /// Sequence key.
        key: String,
    },

    // =========================================================================
    // Coordination primitives - Rate Limiter
    // =========================================================================
    /// Try to acquire tokens from a rate limiter without blocking.
    ///
    /// Returns immediately with success/failure and retry_after_ms hint.
    RateLimiterTryAcquire {
        /// Rate limiter key.
        key: String,
        /// Number of tokens to acquire.
        tokens: u64,
        /// Maximum bucket capacity (in tokens).
        capacity_tokens: u64,
        /// Token refill rate per second.
        refill_rate: f64,
    },

    /// Acquire tokens from a rate limiter with timeout.
    ///
    /// Blocks until tokens are available or timeout is reached.
    RateLimiterAcquire {
        /// Rate limiter key.
        key: String,
        /// Number of tokens to acquire.
        tokens: u64,
        /// Maximum bucket capacity (in tokens).
        capacity_tokens: u64,
        /// Token refill rate per second.
        refill_rate: f64,
        /// Timeout in milliseconds.
        timeout_ms: u64,
    },

    /// Check available tokens in a rate limiter without consuming.
    RateLimiterAvailable {
        /// Rate limiter key.
        key: String,
        /// Maximum bucket capacity (in tokens).
        capacity_tokens: u64,
        /// Token refill rate per second.
        refill_rate: f64,
    },

    /// Reset a rate limiter to full capacity.
    RateLimiterReset {
        /// Rate limiter key.
        key: String,
        /// Maximum bucket capacity (in tokens).
        capacity_tokens: u64,
        /// Token refill rate per second.
        refill_rate: f64,
    },

    // =========================================================================
    // Batch operations - Atomic multi-key operations
    // =========================================================================
    /// Read multiple keys atomically.
    ///
    /// Returns all values in a single consistent snapshot.
    /// Keys that don't exist return None in their position.
    BatchRead {
        /// Keys to read (max 100).
        keys: Vec<String>,
    },

    /// Write multiple operations atomically.
    ///
    /// All operations are applied in a single Raft log entry,
    /// ensuring atomic all-or-nothing execution.
    BatchWrite {
        /// Operations to perform (max 100 total).
        operations: Vec<BatchWriteOperation>,
    },

    /// Conditional batch write (etcd-style transaction).
    ///
    /// Checks all conditions first; if all pass, executes all operations.
    /// If any condition fails, no operations are applied.
    /// Similar to etcd's `Txn().If(conditions).Then(ops).Commit()`.
    ConditionalBatchWrite {
        /// Conditions that must all be true (max 100).
        conditions: Vec<BatchCondition>,
        /// Operations to execute if all conditions pass (max 100).
        operations: Vec<BatchWriteOperation>,
    },

    // =========================================================================
    // Watch operations - Real-time key change notifications
    // =========================================================================
    /// Create a watch on keys matching a prefix.
    ///
    /// Returns a watch ID that can be used to cancel the watch.
    /// Events are delivered via the streaming WatchEvent response.
    /// Similar to etcd's Watch API.
    WatchCreate {
        /// Key prefix to watch (empty string watches all keys).
        prefix: String,
        /// Starting log index (0 = from beginning, u64::MAX = latest only).
        /// Useful for resuming watches after disconnect.
        start_index: u64,
        /// Include previous value in events (like etcd's prev_kv).
        should_include_prev_value: bool,
    },

    /// Cancel an active watch.
    WatchCancel {
        /// Watch ID returned from WatchCreate.
        watch_id: u64,
    },

    /// Get current watch status and statistics.
    WatchStatus {
        /// Watch ID to query (None = all watches for this connection).
        watch_id: Option<u64>,
    },

    // =========================================================================
    // Lease operations - Time-based resource management
    // =========================================================================
    /// Grant a new lease with specified TTL.
    ///
    /// Returns a unique lease ID that can be attached to keys.
    /// Similar to etcd's LeaseGrant.
    LeaseGrant {
        /// Time-to-live in seconds.
        ttl_seconds: u32,
        /// Optional client-provided lease ID (0 = auto-generate).
        lease_id: Option<u64>,
    },

    /// Revoke a lease and delete all attached keys.
    ///
    /// All keys attached to this lease are deleted atomically.
    /// Similar to etcd's LeaseRevoke.
    LeaseRevoke {
        /// Lease ID to revoke.
        lease_id: u64,
    },

    /// Refresh a lease's TTL (keepalive).
    ///
    /// Resets the lease deadline to TTL from now.
    /// Similar to etcd's LeaseKeepAlive (single shot).
    LeaseKeepalive {
        /// Lease ID to refresh.
        lease_id: u64,
    },

    /// Get lease information including TTL and attached keys.
    ///
    /// Similar to etcd's LeaseTimeToLive.
    LeaseTimeToLive {
        /// Lease ID to query.
        lease_id: u64,
        /// Include list of keys attached to the lease.
        should_include_keys: bool,
    },

    /// List all active leases.
    ///
    /// Similar to etcd's LeaseLeases.
    LeaseList,

    /// Write a key attached to a lease.
    ///
    /// Key will be deleted when the lease expires or is revoked.
    WriteKeyWithLease {
        /// Key to write.
        key: String,
        /// Value to write.
        value: Vec<u8>,
        /// Lease ID to attach the key to.
        lease_id: u64,
    },

    // =========================================================================
    // Distributed Barrier operations
    // =========================================================================
    /// Enter a barrier, waiting until all participants arrive.
    ///
    /// Creates the barrier if it doesn't exist. Blocks until the required
    /// number of participants have entered, or timeout is reached.
    BarrierEnter {
        /// Barrier name (unique identifier).
        name: String,
        /// Unique identifier for this participant.
        participant_id: String,
        /// Number of participants required to release the barrier.
        required_count: u32,
        /// Timeout in milliseconds (0 = no timeout).
        timeout_ms: u64,
    },

    /// Leave a barrier after work is complete.
    ///
    /// Blocks until all participants have left, ensuring coordinated cleanup.
    BarrierLeave {
        /// Barrier name.
        name: String,
        /// Participant ID that is leaving.
        participant_id: String,
        /// Timeout in milliseconds (0 = no timeout).
        timeout_ms: u64,
    },

    /// Query barrier status without blocking.
    BarrierStatus {
        /// Barrier name.
        name: String,
    },

    // =========================================================================
    // Distributed Semaphore operations
    // =========================================================================
    /// Acquire permits from a semaphore, blocking until available.
    SemaphoreAcquire {
        /// Semaphore name.
        name: String,
        /// Holder ID for tracking ownership.
        holder_id: String,
        /// Number of permits to acquire.
        permits: u32,
        /// Maximum permits (semaphore capacity in permit count).
        capacity_permits: u32,
        /// TTL in milliseconds for automatic release.
        ttl_ms: u64,
        /// Timeout in milliseconds (0 = no timeout).
        timeout_ms: u64,
    },

    /// Try to acquire permits without blocking.
    SemaphoreTryAcquire {
        /// Semaphore name.
        name: String,
        /// Holder ID for tracking ownership.
        holder_id: String,
        /// Number of permits to acquire.
        permits: u32,
        /// Maximum permits (semaphore capacity in permit count).
        capacity_permits: u32,
        /// TTL in milliseconds for automatic release.
        ttl_ms: u64,
    },

    /// Release permits back to a semaphore.
    SemaphoreRelease {
        /// Semaphore name.
        name: String,
        /// Holder ID that acquired the permits.
        holder_id: String,
        /// Number of permits to release (0 = all).
        permits: u32,
    },

    /// Query semaphore status.
    SemaphoreStatus {
        /// Semaphore name.
        name: String,
    },

    // =========================================================================
    // Read-Write Lock operations
    // =========================================================================
    /// Acquire read lock (blocking until available or timeout).
    RWLockAcquireRead {
        /// Lock name.
        name: String,
        /// Holder identifier.
        holder_id: String,
        /// TTL in milliseconds.
        ttl_ms: u64,
        /// Timeout in milliseconds (0 = no timeout).
        timeout_ms: u64,
    },

    /// Try to acquire read lock (non-blocking).
    RWLockTryAcquireRead {
        /// Lock name.
        name: String,
        /// Holder identifier.
        holder_id: String,
        /// TTL in milliseconds.
        ttl_ms: u64,
    },

    /// Acquire write lock (blocking until available or timeout).
    RWLockAcquireWrite {
        /// Lock name.
        name: String,
        /// Holder identifier.
        holder_id: String,
        /// TTL in milliseconds.
        ttl_ms: u64,
        /// Timeout in milliseconds (0 = no timeout).
        timeout_ms: u64,
    },

    /// Try to acquire write lock (non-blocking).
    RWLockTryAcquireWrite {
        /// Lock name.
        name: String,
        /// Holder identifier.
        holder_id: String,
        /// TTL in milliseconds.
        ttl_ms: u64,
    },

    /// Release read lock.
    RWLockReleaseRead {
        /// Lock name.
        name: String,
        /// Holder identifier.
        holder_id: String,
    },

    /// Release write lock.
    RWLockReleaseWrite {
        /// Lock name.
        name: String,
        /// Holder identifier.
        holder_id: String,
        /// Fencing token for verification.
        fencing_token: u64,
    },

    /// Downgrade write lock to read lock.
    RWLockDowngrade {
        /// Lock name.
        name: String,
        /// Holder identifier.
        holder_id: String,
        /// Fencing token for verification.
        fencing_token: u64,
        /// New TTL in milliseconds.
        ttl_ms: u64,
    },

    /// Query RWLock status.
    RWLockStatus {
        /// Lock name.
        name: String,
    },

    // =========================================================================
    // Queue operations
    // =========================================================================
    /// Create a distributed queue.
    QueueCreate {
        /// Queue name.
        queue_name: String,
        /// Default visibility timeout in milliseconds.
        default_visibility_timeout_ms: Option<u64>,
        /// Default item TTL in milliseconds (0 = no expiration).
        default_ttl_ms: Option<u64>,
        /// Max delivery attempts before DLQ (0 = no limit).
        max_delivery_attempts: Option<u32>,
    },

    /// Delete a queue and all its items.
    QueueDelete {
        /// Queue name.
        queue_name: String,
    },

    /// Enqueue an item to a distributed queue.
    QueueEnqueue {
        /// Queue name.
        queue_name: String,
        /// Item payload.
        payload: Vec<u8>,
        /// Optional TTL in milliseconds.
        ttl_ms: Option<u64>,
        /// Optional message group ID for FIFO ordering.
        message_group_id: Option<String>,
        /// Optional deduplication ID.
        deduplication_id: Option<String>,
    },

    /// Enqueue multiple items in a batch.
    QueueEnqueueBatch {
        /// Queue name.
        queue_name: String,
        /// Items to enqueue (payload, ttl_ms, message_group_id, deduplication_id).
        items: Vec<QueueEnqueueItem>,
    },

    /// Dequeue items from a queue with visibility timeout (non-blocking).
    QueueDequeue {
        /// Queue name.
        queue_name: String,
        /// Consumer ID.
        consumer_id: String,
        /// Maximum items to return.
        max_items: u32,
        /// Visibility timeout in milliseconds.
        visibility_timeout_ms: u64,
    },

    /// Dequeue items with blocking wait.
    QueueDequeueWait {
        /// Queue name.
        queue_name: String,
        /// Consumer ID.
        consumer_id: String,
        /// Maximum items to return.
        max_items: u32,
        /// Visibility timeout in milliseconds.
        visibility_timeout_ms: u64,
        /// Wait timeout in milliseconds.
        wait_timeout_ms: u64,
    },

    /// Peek at items without removing them.
    QueuePeek {
        /// Queue name.
        queue_name: String,
        /// Maximum items to return.
        max_items: u32,
    },

    /// Acknowledge successful processing of an item.
    QueueAck {
        /// Queue name.
        queue_name: String,
        /// Receipt handle from dequeue.
        receipt_handle: String,
    },

    /// Negative acknowledge - return to queue or move to DLQ.
    QueueNack {
        /// Queue name.
        queue_name: String,
        /// Receipt handle from dequeue.
        receipt_handle: String,
        /// Whether to move directly to DLQ.
        move_to_dlq: bool,
        /// Optional error message.
        error_message: Option<String>,
    },

    /// Extend visibility timeout for a pending item.
    QueueExtendVisibility {
        /// Queue name.
        queue_name: String,
        /// Receipt handle.
        receipt_handle: String,
        /// Additional timeout in milliseconds.
        additional_timeout_ms: u64,
    },

    /// Get queue status.
    QueueStatus {
        /// Queue name.
        queue_name: String,
    },

    /// Get items from dead letter queue.
    QueueGetDLQ {
        /// Queue name.
        queue_name: String,
        /// Maximum items to return.
        max_items: u32,
    },

    /// Move DLQ item back to main queue.
    QueueRedriveDLQ {
        /// Queue name.
        queue_name: String,
        /// Item ID in DLQ.
        item_id: u64,
    },

    // =========================================================================
    // Service Registry operations
    // =========================================================================
    /// Register a service instance.
    ServiceRegister {
        /// Service name.
        service_name: String,
        /// Unique instance identifier.
        instance_id: String,
        /// Network address (host:port).
        address: String,
        /// Version string.
        version: String,
        /// Tags for filtering (JSON array).
        tags: String,
        /// Load balancing weight.
        weight: u32,
        /// Custom metadata (JSON object).
        custom_metadata: String,
        /// TTL in milliseconds (0 = default).
        ttl_ms: u64,
        /// Optional lease ID to attach to.
        lease_id: Option<u64>,
    },

    /// Deregister a service instance.
    ServiceDeregister {
        /// Service name.
        service_name: String,
        /// Instance identifier.
        instance_id: String,
        /// Fencing token from registration.
        fencing_token: u64,
    },

    /// Discover service instances.
    ServiceDiscover {
        /// Service name.
        service_name: String,
        /// Only return healthy instances.
        healthy_only: bool,
        /// Filter by tags (JSON array).
        tags: String,
        /// Filter by version prefix.
        version_prefix: Option<String>,
        /// Maximum instances to return.
        limit: Option<u32>,
    },

    /// Discover services by name prefix.
    ServiceList {
        /// Service name prefix.
        prefix: String,
        /// Maximum services to return.
        limit: u32,
    },

    /// Get a specific service instance.
    ServiceGetInstance {
        /// Service name.
        service_name: String,
        /// Instance identifier.
        instance_id: String,
    },

    /// Send heartbeat to renew TTL.
    ServiceHeartbeat {
        /// Service name.
        service_name: String,
        /// Instance identifier.
        instance_id: String,
        /// Fencing token from registration.
        fencing_token: u64,
    },

    /// Update instance health status.
    ServiceUpdateHealth {
        /// Service name.
        service_name: String,
        /// Instance identifier.
        instance_id: String,
        /// Fencing token.
        fencing_token: u64,
        /// New health status: "healthy", "unhealthy", "unknown".
        status: String,
    },

    /// Update instance metadata.
    ServiceUpdateMetadata {
        /// Service name.
        service_name: String,
        /// Instance identifier.
        instance_id: String,
        /// Fencing token.
        fencing_token: u64,
        /// New version (optional).
        version: Option<String>,
        /// New tags (JSON array, optional).
        tags: Option<String>,
        /// New weight (optional).
        weight: Option<u32>,
        /// New custom metadata (JSON object, optional).
        custom_metadata: Option<String>,
    },

    // =========================================================================
    // Sharding operations - Topology management
    // =========================================================================
    /// Get the current shard topology.
    ///
    /// Returns the cluster's shard topology including version, shard info,
    /// and key range mappings. Clients use this for shard-aware routing.
    ///
    /// The optional `client_version` parameter enables conditional fetching:
    /// - If provided and matches current version, returns `updated: false`
    /// - Otherwise returns the full topology data
    GetTopology {
        /// Client's current topology version (for conditional fetch).
        client_version: Option<u64>,
    },

    // =========================================================================
    // Forge operations - Decentralized Git
    // =========================================================================
    /// Create a new repository.
    ForgeCreateRepo {
        /// Repository name (1-256 bytes).
        name: String,
        /// Optional description (max 4096 bytes).
        description: Option<String>,
        /// Default branch name (default: "main").
        default_branch: Option<String>,
    },

    /// Get repository information by ID.
    ForgeGetRepo {
        /// Repository ID (hex-encoded BLAKE3 hash).
        repo_id: String,
    },

    /// List repositories.
    ForgeListRepos {
        /// Maximum results (default 100, max 1000).
        limit: Option<u32>,
        /// Offset for pagination.
        offset: Option<u32>,
    },

    /// Store a blob (file content).
    ForgeStoreBlob {
        /// Repository ID.
        repo_id: String,
        /// Blob content (max 100 MB).
        content: Vec<u8>,
    },

    /// Get a blob by hash.
    ForgeGetBlob {
        /// BLAKE3 hash (hex-encoded).
        hash: String,
    },

    /// Create a tree (directory).
    ForgeCreateTree {
        /// Repository ID.
        repo_id: String,
        /// Tree entries as JSON array of {mode, name, hash}.
        entries_json: String,
    },

    /// Get a tree by hash.
    ForgeGetTree {
        /// BLAKE3 hash (hex-encoded).
        hash: String,
    },

    /// Create a commit.
    ForgeCommit {
        /// Repository ID.
        repo_id: String,
        /// Tree hash (hex-encoded).
        tree: String,
        /// Parent commit hashes (hex-encoded).
        parents: Vec<String>,
        /// Commit message.
        message: String,
    },

    /// Get a commit by hash.
    ForgeGetCommit {
        /// BLAKE3 hash (hex-encoded).
        hash: String,
    },

    /// Get commit history from a ref.
    ForgeLog {
        /// Repository ID.
        repo_id: String,
        /// Ref name (e.g., "heads/main"). Uses default branch if not specified.
        ref_name: Option<String>,
        /// Maximum commits to return (default 50, max 1000).
        limit: Option<u32>,
    },

    /// Get a ref value.
    ForgeGetRef {
        /// Repository ID.
        repo_id: String,
        /// Ref name (e.g., "heads/main", "tags/v1.0").
        ref_name: String,
    },

    /// Set a ref value.
    ForgeSetRef {
        /// Repository ID.
        repo_id: String,
        /// Ref name.
        ref_name: String,
        /// Commit hash (hex-encoded).
        hash: String,
        /// Signer's public key (hex-encoded, required for canonical refs).
        signer: Option<String>,
        /// Signature over the update (hex-encoded, required for canonical refs).
        signature: Option<String>,
        /// Timestamp in milliseconds (required for canonical refs).
        timestamp_ms: Option<u64>,
    },

    /// Delete a ref.
    ForgeDeleteRef {
        /// Repository ID.
        repo_id: String,
        /// Ref name.
        ref_name: String,
    },

    /// Compare-and-set a ref (for safe concurrent updates).
    ForgeCasRef {
        /// Repository ID.
        repo_id: String,
        /// Ref name.
        ref_name: String,
        /// Expected current hash (None = must not exist).
        expected: Option<String>,
        /// New hash.
        new_hash: String,
        /// Signer's public key (hex-encoded, required for canonical refs).
        signer: Option<String>,
        /// Signature over the update (hex-encoded, required for canonical refs).
        signature: Option<String>,
        /// Timestamp in milliseconds (required for canonical refs).
        timestamp_ms: Option<u64>,
    },

    /// List branches in a repository.
    ForgeListBranches {
        /// Repository ID.
        repo_id: String,
    },

    /// List tags in a repository.
    ForgeListTags {
        /// Repository ID.
        repo_id: String,
    },

    /// Create an issue.
    ForgeCreateIssue {
        /// Repository ID.
        repo_id: String,
        /// Issue title.
        title: String,
        /// Issue body.
        body: String,
        /// Labels.
        labels: Vec<String>,
    },

    /// List issues in a repository.
    ForgeListIssues {
        /// Repository ID.
        repo_id: String,
        /// Filter by state: "open", "closed", or None for all.
        state: Option<String>,
        /// Maximum results (default 50, max 1000).
        limit: Option<u32>,
    },

    /// Get issue details.
    ForgeGetIssue {
        /// Repository ID.
        repo_id: String,
        /// Issue ID (hex-encoded).
        issue_id: String,
    },

    /// Add a comment to an issue.
    ForgeCommentIssue {
        /// Repository ID.
        repo_id: String,
        /// Issue ID.
        issue_id: String,
        /// Comment body.
        body: String,
    },

    /// Close an issue.
    ForgeCloseIssue {
        /// Repository ID.
        repo_id: String,
        /// Issue ID.
        issue_id: String,
        /// Optional reason for closing.
        reason: Option<String>,
    },

    /// Reopen an issue.
    ForgeReopenIssue {
        /// Repository ID.
        repo_id: String,
        /// Issue ID.
        issue_id: String,
    },

    /// Create a patch (pull request equivalent).
    ForgeCreatePatch {
        /// Repository ID.
        repo_id: String,
        /// Patch title.
        title: String,
        /// Patch description.
        description: String,
        /// Base commit hash (what we're merging into).
        base: String,
        /// Head commit hash (what we're merging).
        head: String,
    },

    /// List patches in a repository.
    ForgeListPatches {
        /// Repository ID.
        repo_id: String,
        /// Filter by state: "open", "merged", "closed", or None for all.
        state: Option<String>,
        /// Maximum results (default 50, max 1000).
        limit: Option<u32>,
    },

    /// Get patch details.
    ForgeGetPatch {
        /// Repository ID.
        repo_id: String,
        /// Patch ID (hex-encoded).
        patch_id: String,
    },

    /// Update patch head (push new commits).
    ForgeUpdatePatch {
        /// Repository ID.
        repo_id: String,
        /// Patch ID.
        patch_id: String,
        /// New head commit hash.
        head: String,
        /// Optional update message.
        message: Option<String>,
    },

    /// Approve a patch.
    ForgeApprovePatch {
        /// Repository ID.
        repo_id: String,
        /// Patch ID.
        patch_id: String,
        /// Commit being approved.
        commit: String,
        /// Optional approval message.
        message: Option<String>,
    },

    /// Merge a patch.
    ForgeMergePatch {
        /// Repository ID.
        repo_id: String,
        /// Patch ID.
        patch_id: String,
        /// Merge commit hash.
        merge_commit: String,
    },

    /// Close a patch without merging.
    ForgeClosePatch {
        /// Repository ID.
        repo_id: String,
        /// Patch ID.
        patch_id: String,
        /// Optional reason for closing.
        reason: Option<String>,
    },

    /// Get the delegate key for a repository.
    ///
    /// Returns the secret key used for signing canonical ref updates.
    /// Only available to authorized users on the local node.
    ForgeGetDelegateKey {
        /// Repository ID.
        repo_id: String,
    },

    // =========================================================================
    // Federation operations - Cross-cluster discovery and sync
    // =========================================================================
    /// Get federation status.
    GetFederationStatus,

    /// List discovered clusters.
    ListDiscoveredClusters,

    /// Get details about a discovered cluster.
    GetDiscoveredCluster {
        /// Cluster public key.
        cluster_key: String,
    },

    /// Trust a cluster.
    TrustCluster {
        /// Cluster public key to trust.
        cluster_key: String,
    },

    /// Untrust a cluster.
    UntrustCluster {
        /// Cluster public key to untrust.
        cluster_key: String,
    },

    /// Federate a repository.
    FederateRepository {
        /// Repository ID.
        repo_id: String,
        /// Federation mode: "public" or "allowlist".
        mode: String,
    },

    /// List federated repositories.
    ListFederatedRepositories,

    /// Fetch a federated repository from a remote cluster.
    ForgeFetchFederated {
        /// Federated ID (format: origin:local_id).
        federated_id: String,
        /// Remote cluster public key (hex-encoded).
        remote_cluster: String,
    },

    // =========================================================================
    // Git Bridge operations (for git-remote-aspen)
    // =========================================================================
    /// List refs with their SHA-1 hashes (for git remote helper "list" command).
    GitBridgeListRefs {
        /// Repository ID (hex-encoded BLAKE3 hash).
        repo_id: String,
    },

    /// Fetch objects for a ref (for git remote helper "fetch" command).
    ///
    /// Returns git objects in dependency order that the client needs.
    GitBridgeFetch {
        /// Repository ID (hex-encoded BLAKE3 hash).
        repo_id: String,
        /// SHA-1 hashes the client wants to fetch.
        want: Vec<String>,
        /// SHA-1 hashes the client already has (for delta computation).
        have: Vec<String>,
    },

    /// Push objects and update refs (for git remote helper "push" command).
    ///
    /// Accepts git objects in raw git format and imports them into Forge.
    ///
    /// **DEPRECATED**: Use `GitBridgePushChunked` for large repositories to avoid
    /// hitting MAX_CLIENT_MESSAGE_SIZE limits. This method is limited to ~256MB
    /// total payload size.
    GitBridgePush {
        /// Repository ID (hex-encoded BLAKE3 hash).
        repo_id: String,
        /// Git objects to import (SHA-1 hash, type, raw bytes).
        objects: Vec<GitBridgeObject>,
        /// Refs to update after import.
        refs: Vec<GitBridgeRefUpdate>,
    },

    /// Start a chunked git push operation.
    ///
    /// Large git pushes are split into multiple chunks to avoid message size limits.
    /// This initializes the push session and returns a session_id for subsequent chunks.
    GitBridgePushStart {
        /// Repository ID (hex-encoded BLAKE3 hash).
        repo_id: String,
        /// Total number of objects to be pushed across all chunks.
        total_objects: u64,
        /// Total size in bytes of all objects (for progress tracking).
        total_size_bytes: u64,
        /// Refs to update after all chunks are received.
        refs: Vec<GitBridgeRefUpdate>,
        /// Optional push metadata (committer, timestamp, etc.).
        metadata: Option<GitBridgePushMetadata>,
    },

    /// Send a chunk of objects for a previously started push operation.
    ///
    /// Each chunk contains up to CHUNK_SIZE_LIMIT bytes of git objects.
    /// Chunks must be sent in order and include integrity verification.
    GitBridgePushChunk {
        /// Session ID from GitBridgePushStart response.
        session_id: String,
        /// Chunk sequence number (0-based, must be consecutive).
        chunk_id: u64,
        /// Total number of chunks expected in this push.
        total_chunks: u64,
        /// Git objects in this chunk.
        objects: Vec<GitBridgeObject>,
        /// Blake3 hash of this chunk's serialized objects for integrity.
        chunk_hash: [u8; 32],
    },

    /// Complete a chunked git push operation.
    ///
    /// Verifies all chunks were received and applies the ref updates.
    /// This is the final step that makes the push visible to other clients.
    GitBridgePushComplete {
        /// Session ID from GitBridgePushStart.
        session_id: String,
        /// Blake3 hash of all objects combined (for final verification).
        content_hash: [u8; 32],
    },

    /// Probe which objects the server already has (for incremental push).
    ///
    /// The client sends SHA-1 hashes from `git rev-list --objects` and the
    /// server reports which ones already have hash mappings. The client then
    /// only reads and sends the missing objects — typically reducing push
    /// payload by 90-99% for repositories that have been pushed before.
    ///
    /// This is a read-only operation that doesn't require Raft write consensus.
    GitBridgeProbeObjects {
        /// Repository ID (hex-encoded BLAKE3 hash).
        repo_id: String,
        /// SHA-1 hashes to check (hex-encoded, 40 characters each).
        /// Bounded by MAX_GIT_OBJECTS_PER_PUSH.
        sha1s: Vec<String>,
    },

    // =========================================================================
    // Job operations - High-level job scheduling and management
    // =========================================================================
    /// Submit a new job to the job queue system.
    JobSubmit {
        /// Job type identifier.
        job_type: String,
        /// Job payload (JSON-encoded string).
        payload: String,
        /// Priority level (0=Low, 1=Normal, 2=High, 3=Critical).
        priority: Option<u8>,
        /// Timeout in milliseconds (default: 5 minutes).
        timeout_ms: Option<u64>,
        /// Maximum retry attempts (default: 3).
        max_retries: Option<u32>,
        /// Retry delay in milliseconds (default: 1000).
        retry_delay_ms: Option<u64>,
        /// Schedule expression (cron format or interval).
        schedule: Option<String>,
        /// Tags for job filtering.
        tags: Vec<String>,
    },
    /// Get job status and details.
    JobGet {
        /// Job ID.
        job_id: String,
    },
    /// List jobs with optional filtering.
    JobList {
        /// Filter by status: pending, scheduled, running, completed, failed, cancelled.
        status: Option<String>,
        /// Filter by job type.
        job_type: Option<String>,
        /// Filter by tags (must have all specified tags).
        tags: Vec<String>,
        /// Maximum results (default 100, max 1000).
        limit: Option<u32>,
        /// Continuation token for pagination.
        continuation_token: Option<String>,
    },
    /// Cancel a job.
    JobCancel {
        /// Job ID to cancel.
        job_id: String,
        /// Optional cancellation reason.
        reason: Option<String>,
    },
    /// Update job progress (for workers).
    JobUpdateProgress {
        /// Job ID.
        job_id: String,
        /// Progress percentage (0-100).
        progress: u8,
        /// Optional progress message.
        message: Option<String>,
    },
    /// Get job queue statistics.
    JobQueueStats,
    /// Get worker pool status.
    WorkerStatus,
    /// Register a worker (for workers).
    WorkerRegister {
        /// Worker ID.
        worker_id: String,
        /// Worker capabilities (job types it can handle).
        capabilities: Vec<String>,
        /// Worker capacity (concurrent job count).
        capacity_jobs: u32,
    },
    /// Worker heartbeat (for workers).
    WorkerHeartbeat {
        /// Worker ID.
        worker_id: String,
        /// Current job IDs being processed.
        active_jobs: Vec<String>,
    },
    /// Deregister a worker (for workers).
    WorkerDeregister {
        /// Worker ID.
        worker_id: String,
    },

    // =========================================================================
    // Hook operations - Event-driven automation
    // =========================================================================
    /// List configured hook handlers.
    ///
    /// Returns information about all handlers configured on this node,
    /// including their patterns, types, and enabled status.
    HookList,

    /// Get hook execution metrics.
    ///
    /// Returns execution statistics for handlers including success/failure
    /// counts, latencies, and dropped events.
    HookGetMetrics {
        /// Optional handler name to filter metrics.
        /// If None, returns metrics for all handlers.
        handler_name: Option<String>,
    },

    /// Manually trigger a hook event for testing.
    ///
    /// Creates a synthetic event and dispatches it to matching handlers.
    /// Useful for testing handler configurations without waiting for real events.
    ///
    /// Note: payload is a JSON string (not serde_json::Value) for PostCard compatibility.
    /// PostCard cannot serialize serde_json::Value because it requires `serialize_any()`.
    HookTrigger {
        /// Event type to trigger (e.g., "write_committed", "delete_committed").
        event_type: String,
        /// Event payload as JSON string. Parse with serde_json::from_str() on the server.
        payload_json: String,
    },

    // =========================================================================
    // CI/CD operations - Pipeline management and execution
    // =========================================================================
    /// Trigger a CI pipeline run for a repository.
    ///
    /// Starts a new pipeline run by loading the `.aspen/ci.ncl` configuration
    /// from the specified repository and commit, then scheduling jobs.
    CiTriggerPipeline {
        /// Repository ID containing the CI configuration.
        repo_id: String,
        /// Git reference (e.g., "refs/heads/main", "refs/tags/v1.0").
        ref_name: String,
        /// Optional specific commit hash. If None, uses the ref's current commit.
        commit_hash: Option<String>,
    },

    /// Get pipeline run status and details.
    ///
    /// Returns the current status, stage progress, and job results
    /// for a specific pipeline run.
    CiGetStatus {
        /// Pipeline run ID.
        run_id: String,
    },

    /// List pipeline runs with optional filtering.
    ///
    /// Returns pipeline runs sorted by creation time (newest first).
    CiListRuns {
        /// Filter by repository ID.
        repo_id: Option<String>,
        /// Filter by status: pending, running, succeeded, failed, cancelled.
        status: Option<String>,
        /// Maximum results (default 50, max 500).
        limit: Option<u32>,
    },

    /// Cancel a running pipeline.
    ///
    /// Cancels all pending and running jobs in the pipeline.
    /// Jobs that are already completed are not affected.
    CiCancelRun {
        /// Pipeline run ID to cancel.
        run_id: String,
        /// Optional cancellation reason.
        reason: Option<String>,
    },

    /// Watch a repository for CI triggers.
    ///
    /// Subscribes to forge gossip events for the repository to
    /// automatically trigger CI on push events.
    CiWatchRepo {
        /// Repository ID to watch.
        repo_id: String,
    },

    /// Unwatch a repository.
    ///
    /// Removes the CI trigger subscription for the repository.
    CiUnwatchRepo {
        /// Repository ID to unwatch.
        repo_id: String,
    },

    /// List artifacts for a CI job.
    ///
    /// Returns metadata about artifacts produced by a job, including
    /// blob hashes for downloading.
    CiListArtifacts {
        /// Job ID to list artifacts for.
        job_id: String,
        /// Optional pipeline run ID for filtering.
        run_id: Option<String>,
    },

    /// Get artifact metadata and download ticket.
    ///
    /// Returns the blob ticket for downloading an artifact from the
    /// distributed blob store.
    CiGetArtifact {
        /// Blob hash of the artifact.
        blob_hash: String,
    },

    /// Get historical logs for a CI job.
    ///
    /// Returns log chunks starting from a specific index, allowing
    /// clients to fetch missed logs or replay from the beginning.
    /// Logs are stored in KV with prefix `_ci:logs:{run_id}:{job_id}:`.
    CiGetJobLogs {
        /// Pipeline run ID.
        run_id: String,
        /// Job ID within the pipeline.
        job_id: String,
        /// Starting chunk index (0 for beginning).
        start_index: u32,
        /// Maximum chunks to return (default 100, max 1000).
        limit: Option<u32>,
    },

    /// Subscribe to real-time logs for a CI job.
    ///
    /// Returns watch configuration that the client can use to establish
    /// a LOG_SUBSCRIBER_ALPN connection for streaming logs.
    CiSubscribeLogs {
        /// Pipeline run ID.
        run_id: String,
        /// Job ID within the pipeline.
        job_id: String,
        /// Starting log index for resumption after disconnect.
        from_index: Option<u64>,
    },

    /// Get full job output (stdout/stderr), resolving blob references.
    ///
    /// Unlike CiGetJobLogs which returns streaming log chunks, this returns
    /// the complete final output stored in the job result. For large outputs,
    /// the data is stored in blobs and this endpoint resolves the blob
    /// references to return the full content.
    CiGetJobOutput {
        /// Pipeline run ID.
        run_id: String,
        /// Job ID within the pipeline.
        job_id: String,
    },

    // =========================================================================
    // Secrets operations - Vault-compatible secrets management
    // =========================================================================
    // KV Secrets Engine
    /// Read a secret from the KV secrets engine.
    ///
    /// Returns the secret data and version metadata. Supports reading specific
    /// versions of secrets. Requires SecretsRead capability.
    SecretsKvRead {
        /// Mount point for the secrets engine (default: "secret").
        mount: String,
        /// Path to the secret within the mount.
        path: String,
        /// Specific version to read (None = current version).
        version: Option<u64>,
    },

    /// Write a secret to the KV secrets engine.
    ///
    /// Creates a new version of the secret. Supports check-and-set (CAS)
    /// for optimistic concurrency control. Requires SecretsWrite capability.
    SecretsKvWrite {
        /// Mount point for the secrets engine.
        mount: String,
        /// Path to the secret within the mount.
        path: String,
        /// Secret data as key-value pairs.
        data: std::collections::HashMap<String, String>,
        /// Optional CAS version for optimistic locking.
        cas: Option<u64>,
    },

    /// Soft-delete secret versions from KV.
    ///
    /// Marks versions as deleted but recoverable via undelete.
    /// If no versions specified, deletes the current version.
    SecretsKvDelete {
        /// Mount point for the secrets engine.
        mount: String,
        /// Path to the secret.
        path: String,
        /// Specific versions to delete (empty = current version).
        versions: Vec<u64>,
    },

    /// Permanently destroy secret versions from KV.
    ///
    /// Irreversibly removes version data. Cannot be recovered.
    SecretsKvDestroy {
        /// Mount point for the secrets engine.
        mount: String,
        /// Path to the secret.
        path: String,
        /// Versions to permanently destroy.
        versions: Vec<u64>,
    },

    /// Undelete soft-deleted secret versions.
    ///
    /// Recovers versions that were soft-deleted. Cannot recover destroyed versions.
    SecretsKvUndelete {
        /// Mount point for the secrets engine.
        mount: String,
        /// Path to the secret.
        path: String,
        /// Versions to undelete.
        versions: Vec<u64>,
    },

    /// List secrets under a path prefix.
    ///
    /// Returns secret names (not values). Use for navigation and discovery.
    SecretsKvList {
        /// Mount point for the secrets engine.
        mount: String,
        /// Path prefix to list under.
        path: String,
    },

    /// Get metadata for a secret.
    ///
    /// Returns version history, custom metadata, and configuration.
    SecretsKvMetadata {
        /// Mount point for the secrets engine.
        mount: String,
        /// Path to the secret.
        path: String,
    },

    /// Update metadata for a secret.
    ///
    /// Configure max_versions, cas_required, or custom metadata.
    SecretsKvUpdateMetadata {
        /// Mount point for the secrets engine.
        mount: String,
        /// Path to the secret.
        path: String,
        /// Maximum versions to retain.
        max_versions: Option<u32>,
        /// Require CAS for writes.
        cas_required: Option<bool>,
        /// Custom key-value metadata.
        custom_metadata: Option<std::collections::HashMap<String, String>>,
    },

    /// Delete a secret and all its versions.
    ///
    /// Permanently removes the secret and all version history.
    SecretsKvDeleteMetadata {
        /// Mount point for the secrets engine.
        mount: String,
        /// Path to the secret.
        path: String,
    },

    // Transit Secrets Engine
    /// Create a new encryption key in the Transit engine.
    ///
    /// Supports various key types for encryption, signing, etc.
    SecretsTransitCreateKey {
        /// Mount point for the transit engine (default: "transit").
        mount: String,
        /// Name of the key to create.
        name: String,
        /// Key type: "aes256-gcm", "xchacha20-poly1305", or "ed25519".
        key_type: String,
    },

    /// Encrypt data using a Transit key.
    ///
    /// Returns ciphertext that can only be decrypted with the same key.
    SecretsTransitEncrypt {
        /// Mount point for the transit engine.
        mount: String,
        /// Name of the encryption key.
        name: String,
        /// Plaintext data to encrypt.
        plaintext: Vec<u8>,
        /// Optional context for key derivation.
        context: Option<Vec<u8>>,
    },

    /// Decrypt ciphertext using a Transit key.
    ///
    /// Returns the original plaintext.
    SecretsTransitDecrypt {
        /// Mount point for the transit engine.
        mount: String,
        /// Name of the encryption key.
        name: String,
        /// Ciphertext to decrypt.
        ciphertext: String,
        /// Optional context for key derivation.
        context: Option<Vec<u8>>,
    },

    /// Sign data using a Transit key.
    ///
    /// Creates a cryptographic signature (Ed25519 keys only).
    SecretsTransitSign {
        /// Mount point for the transit engine.
        mount: String,
        /// Name of the signing key.
        name: String,
        /// Data to sign.
        data: Vec<u8>,
    },

    /// Verify a signature using a Transit key.
    ///
    /// Validates that a signature matches the provided data.
    SecretsTransitVerify {
        /// Mount point for the transit engine.
        mount: String,
        /// Name of the signing key.
        name: String,
        /// Original data that was signed.
        data: Vec<u8>,
        /// Signature to verify.
        signature: String,
    },

    /// Rotate a Transit key to a new version.
    ///
    /// Creates a new key version. Old versions remain for decryption.
    SecretsTransitRotateKey {
        /// Mount point for the transit engine.
        mount: String,
        /// Name of the key to rotate.
        name: String,
    },

    /// List all keys in the Transit engine.
    SecretsTransitListKeys {
        /// Mount point for the transit engine.
        mount: String,
    },

    /// Rewrap ciphertext with the latest key version.
    ///
    /// Upgrades ciphertext encrypted with an older key version.
    SecretsTransitRewrap {
        /// Mount point for the transit engine.
        mount: String,
        /// Name of the encryption key.
        name: String,
        /// Ciphertext to rewrap.
        ciphertext: String,
        /// Optional context for key derivation.
        context: Option<Vec<u8>>,
    },

    /// Generate a data key for envelope encryption.
    ///
    /// Returns a wrapped key and optionally the plaintext key.
    SecretsTransitDatakey {
        /// Mount point for the transit engine.
        mount: String,
        /// Name of the encryption key.
        name: String,
        /// Key type: "plaintext" (returns unwrapped) or "wrapped" (encrypted only).
        key_type: String,
    },

    // PKI Secrets Engine
    /// Generate a root CA certificate.
    ///
    /// Creates a self-signed root certificate authority.
    SecretsPkiGenerateRoot {
        /// Mount point for the PKI engine (default: "pki").
        mount: String,
        /// Common name for the CA certificate.
        common_name: String,
        /// Certificate validity in days (default: 3650).
        ttl_days: Option<u32>,
    },

    /// Generate an intermediate CA certificate signing request.
    ///
    /// Creates a CSR for signing by another CA.
    SecretsPkiGenerateIntermediate {
        /// Mount point for the PKI engine.
        mount: String,
        /// Common name for the intermediate CA.
        common_name: String,
    },

    /// Set the signed intermediate certificate.
    ///
    /// Imports a certificate signed by a parent CA.
    SecretsPkiSetSignedIntermediate {
        /// Mount point for the PKI engine.
        mount: String,
        /// Signed certificate in PEM format.
        certificate: String,
    },

    /// Create a role for certificate issuance.
    ///
    /// Roles define policies for what certificates can be issued.
    SecretsPkiCreateRole {
        /// Mount point for the PKI engine.
        mount: String,
        /// Role name.
        name: String,
        /// Allowed domain patterns (supports wildcards).
        allowed_domains: Vec<String>,
        /// Maximum certificate TTL in days.
        max_ttl_days: u32,
        /// Allow bare domains (not just subdomains).
        allow_bare_domains: bool,
        /// Allow wildcard certificates.
        allow_wildcards: bool,
        /// Allow subdomains of allowed domains.
        allow_subdomains: bool,
    },

    /// Issue a certificate using a role.
    ///
    /// Generates a new certificate according to role policies.
    SecretsPkiIssue {
        /// Mount point for the PKI engine.
        mount: String,
        /// Role to use for issuance.
        role: String,
        /// Common name for the certificate.
        common_name: String,
        /// Subject Alternative Names (SANs).
        alt_names: Vec<String>,
        /// Certificate TTL in days (must be <= role max_ttl).
        ttl_days: Option<u32>,
    },

    /// Revoke a certificate.
    ///
    /// Adds the certificate to the CRL.
    SecretsPkiRevoke {
        /// Mount point for the PKI engine.
        mount: String,
        /// Serial number of the certificate to revoke.
        serial: String,
    },

    /// Get the Certificate Revocation List.
    ///
    /// Returns the current CRL in PEM format.
    SecretsPkiGetCrl {
        /// Mount point for the PKI engine.
        mount: String,
    },

    /// List all issued certificates.
    ///
    /// Returns serial numbers of all certificates.
    SecretsPkiListCerts {
        /// Mount point for the PKI engine.
        mount: String,
    },

    /// Get PKI role configuration.
    SecretsPkiGetRole {
        /// Mount point for the PKI engine.
        mount: String,
        /// Role name.
        name: String,
    },

    /// List all PKI roles.
    SecretsPkiListRoles {
        /// Mount point for the PKI engine.
        mount: String,
    },

    // =========================================================================
    // Nix Cache Signing operations - Ed25519 signing keys via Transit
    // =========================================================================
    /// Create a signing key for a Nix cache.
    ///
    /// Creates an Ed25519 signing key in the Transit engine for narinfo signing.
    /// The key name matches the cache name (e.g., "cache.example.com-1").
    SecretsNixCacheCreateKey {
        /// Mount point for the Transit engine (default: "nix-cache").
        mount: String,
        /// Cache name (e.g., "cache.example.com-1").
        cache_name: String,
    },

    /// Get the public key for a cache.
    ///
    /// Returns the public key in Nix format: "{cache_name}:{base64_key}".
    /// Used for the `trusted-public-keys` configuration.
    SecretsNixCacheGetPublicKey {
        /// Mount point for the Transit engine.
        mount: String,
        /// Cache name.
        cache_name: String,
    },

    /// Rotate a cache signing key to a new version.
    ///
    /// Creates a new key version while keeping old versions for verification.
    /// Returns the new public key for configuration updates.
    SecretsNixCacheRotateKey {
        /// Mount point for the Transit engine.
        mount: String,
        /// Cache name.
        cache_name: String,
    },

    /// Delete a cache signing key.
    ///
    /// Permanently removes all versions of the signing key.
    /// Cache will no longer be able to sign narinfo files.
    SecretsNixCacheDeleteKey {
        /// Mount point for the Transit engine.
        mount: String,
        /// Cache name.
        cache_name: String,
    },

    /// List all cache signing keys.
    ///
    /// Returns names of all caches with signing keys in the mount.
    SecretsNixCacheListKeys {
        /// Mount point for the Transit engine.
        mount: String,
    },

    // =========================================================================
    // FEATURE-GATED VARIANTS (must be at end for postcard discriminant stability)
    // =========================================================================

    // -------------------------------------------------------------------------
    // Automerge operations - CRDT document management
    // -------------------------------------------------------------------------
    /// Create a new Automerge document.
    #[cfg(feature = "automerge")]
    AutomergeCreate {
        /// Optional custom document ID (auto-generated if not provided).
        document_id: Option<String>,
        /// Optional namespace for grouping.
        namespace: Option<String>,
        /// Optional human-readable title.
        title: Option<String>,
        /// Optional description.
        description: Option<String>,
        /// Optional tags for categorization.
        tags: Vec<String>,
    },

    /// Get an Automerge document.
    #[cfg(feature = "automerge")]
    AutomergeGet {
        /// Document ID.
        document_id: String,
    },

    /// Save/update an Automerge document.
    #[cfg(feature = "automerge")]
    AutomergeSave {
        /// Document ID.
        document_id: String,
        /// Serialized Automerge document bytes (base64-encoded).
        document_bytes: String,
    },

    /// Delete an Automerge document.
    #[cfg(feature = "automerge")]
    AutomergeDelete {
        /// Document ID.
        document_id: String,
    },

    /// Apply incremental changes to an Automerge document.
    #[cfg(feature = "automerge")]
    AutomergeApplyChanges {
        /// Document ID.
        document_id: String,
        /// List of change bytes (each base64-encoded).
        changes: Vec<String>,
    },

    /// Merge two Automerge documents.
    #[cfg(feature = "automerge")]
    AutomergeMerge {
        /// Target document ID (will contain merged result).
        target_document_id: String,
        /// Source document ID (will be merged into target).
        source_document_id: String,
    },

    /// List Automerge documents.
    #[cfg(feature = "automerge")]
    AutomergeList {
        /// Filter by namespace prefix.
        namespace: Option<String>,
        /// Filter by tag.
        tag: Option<String>,
        /// Maximum results (default 100, max 10000).
        limit: Option<u32>,
        /// Continuation token for pagination.
        continuation_token: Option<String>,
    },

    /// Get Automerge document metadata (without content).
    #[cfg(feature = "automerge")]
    AutomergeGetMetadata {
        /// Document ID.
        document_id: String,
    },

    /// Check if an Automerge document exists.
    #[cfg(feature = "automerge")]
    AutomergeExists {
        /// Document ID.
        document_id: String,
    },

    /// Generate a sync message for peer synchronization.
    #[cfg(feature = "automerge")]
    AutomergeGenerateSyncMessage {
        /// Document ID.
        document_id: String,
        /// Peer ID for sync state tracking.
        peer_id: String,
        /// Optional persisted sync state (base64-encoded).
        sync_state: Option<String>,
    },

    /// Receive a sync message from a peer.
    #[cfg(feature = "automerge")]
    AutomergeReceiveSyncMessage {
        /// Document ID.
        document_id: String,
        /// Peer ID for sync state tracking.
        peer_id: String,
        /// Sync message bytes (base64-encoded).
        message: String,
        /// Optional persisted sync state (base64-encoded).
        sync_state: Option<String>,
    },

    // =========================================================================
    // Nix Binary Cache operations
    // =========================================================================
    /// Query the Nix binary cache for a store path.
    ///
    /// Returns the cache entry if the store path exists in the cache.
    CacheQuery {
        /// Store path hash (the abc... part of /nix/store/abc...-name).
        store_hash: String,
    },

    /// Get cache statistics.
    ///
    /// Returns hit/miss counts, total entries, and storage usage.
    CacheStats,

    /// Get a blob ticket for downloading a NAR from the cache.
    ///
    /// Returns a BlobTicket that can be used to download the NAR
    /// via iroh-blobs P2P transfer.
    CacheDownload {
        /// Store path hash.
        store_hash: String,
    },

    // =========================================================================
    // Cache Migration operations (SNIX storage migration)
    // =========================================================================
    /// Start cache migration from legacy to SNIX format.
    ///
    /// Initiates background migration of cache entries from legacy CacheEntry
    /// format to SNIX PathInfo format. Migration runs in the background and
    /// can be monitored via CacheMigrationStatus.
    #[cfg(feature = "ci")]
    CacheMigrationStart {
        /// Batch size for migration (default: 50).
        batch_size: Option<u32>,
        /// Delay between batches in milliseconds (default: 100).
        batch_delay_ms: Option<u64>,
        /// Whether to perform a dry run without writing.
        dry_run: bool,
    },

    /// Get cache migration status.
    ///
    /// Returns the current progress of cache migration including counts
    /// of migrated, failed, and skipped entries.
    #[cfg(feature = "ci")]
    CacheMigrationStatus,

    /// Cancel an in-progress cache migration.
    ///
    /// Signals the migration worker to stop processing. The migration
    /// can be resumed later with CacheMigrationStart.
    #[cfg(feature = "ci")]
    CacheMigrationCancel,

    /// Validate cache migration completeness.
    ///
    /// Checks that all legacy entries have been migrated to SNIX format.
    /// Returns a list of any entries that failed to migrate.
    #[cfg(feature = "ci")]
    CacheMigrationValidate {
        /// Maximum entries to report in validation (default: 100).
        max_report: Option<u32>,
    },

    // =========================================================================
    // SNIX operations (for remote workers)
    // =========================================================================
    /// Get a directory from SNIX DirectoryService.
    ///
    /// Used by ephemeral workers to fetch directory metadata from the cluster.
    SnixDirectoryGet {
        /// BLAKE3 digest of the directory (hex-encoded, 64 chars).
        digest: String,
    },

    /// Put a directory to SNIX DirectoryService.
    ///
    /// Used by ephemeral workers to upload directory metadata to the cluster.
    SnixDirectoryPut {
        /// Protobuf-encoded directory (base64-encoded for transport).
        directory_bytes: String,
    },

    /// Get path info from SNIX PathInfoService.
    ///
    /// Used by ephemeral workers to fetch store path metadata from the cluster.
    SnixPathInfoGet {
        /// 20-byte Nix store path digest (hex-encoded, 40 chars).
        digest: String,
    },

    /// Put path info to SNIX PathInfoService.
    ///
    /// Used by ephemeral workers to upload store path metadata to the cluster.
    SnixPathInfoPut {
        /// Protobuf-encoded PathInfo (base64-encoded for transport).
        pathinfo_bytes: String,
    },

    /// Poll for available jobs of specific types.
    ///
    /// Used by workers to fetch jobs from the distributed queue.
    WorkerPollJobs {
        /// Worker identifier requesting jobs.
        worker_id: String,
        /// Job types this worker can handle.
        job_types: Vec<String>,
        /// Maximum number of jobs to return.
        max_jobs: usize,
        /// Visibility timeout for claimed jobs (in seconds).
        visibility_timeout_secs: u64,
    },

    /// Complete a job and report the result.
    ///
    /// Used by workers to report job completion status.
    WorkerCompleteJob {
        /// Worker identifier that processed the job.
        worker_id: String,
        /// Job ID that was completed.
        job_id: String,
        /// Receipt handle from the job polling response.
        receipt_handle: String,
        /// Execution token from the job polling response.
        execution_token: String,
        /// Job execution result (success or failure).
        is_success: bool,
        /// Error message if job failed.
        error_message: Option<String>,
        /// Job output data (logs, artifacts, etc.).
        output_data: Option<Vec<u8>>,
        /// Processing time in milliseconds.
        processing_time_ms: u64,
    },

    // =========================================================================
    // Observability operations
    // =========================================================================
    /// Ingest a batch of completed trace spans.
    ///
    /// Used by clients to report distributed tracing data to the cluster.
    /// Spans are stored in KV under `_sys:traces:{trace_id}:{span_id}`.
    /// Batch size is bounded by MAX_TRACE_BATCH_SIZE.
    TraceIngest {
        /// Batch of completed spans (max MAX_TRACE_BATCH_SIZE).
        spans: Vec<observability::IngestSpan>,
    },

    /// List recent traces with optional time range filter.
    ///
    /// Scans `_sys:traces:` prefix and aggregates unique trace_ids.
    /// Results bounded by MAX_TRACE_QUERY_RESULTS.
    TraceList {
        /// Start of time range filter (Unix microseconds, inclusive).
        start_time_us: Option<u64>,
        /// End of time range filter (Unix microseconds, exclusive).
        end_time_us: Option<u64>,
        /// Maximum traces to return (default DEFAULT_TRACE_QUERY_LIMIT).
        limit: Option<u32>,
        /// Continuation token from previous TraceList response.
        continuation_token: Option<String>,
    },

    /// Get all spans for a specific trace.
    ///
    /// Scans `_sys:traces:{trace_id}:` prefix to find all spans.
    TraceGet {
        /// Trace ID (32 hex chars).
        trace_id: String,
    },

    /// Search spans by criteria (server-side filtering).
    ///
    /// Scans all trace spans and filters in-memory. Operation match is
    /// case-insensitive substring. Results bounded by MAX_TRACE_QUERY_RESULTS.
    TraceSearch {
        /// Filter by operation name (case-insensitive substring match).
        operation: Option<String>,
        /// Minimum span duration in microseconds (inclusive).
        min_duration_us: Option<u64>,
        /// Maximum span duration in microseconds (inclusive).
        max_duration_us: Option<u64>,
        /// Filter by status: "ok", "error", or "unset".
        status: Option<String>,
        /// Maximum spans to return (default DEFAULT_TRACE_QUERY_LIMIT).
        limit: Option<u32>,
    },

    // =========================================================================
    // Metrics operations
    // =========================================================================
    /// Ingest a batch of metric data points.
    ///
    /// Data points are stored under `_sys:metrics:{name}:{timestamp_us:020}` with TTL.
    /// Metadata is upserted under `_sys:metrics_meta:{name}`.
    /// Batch size bounded by `MAX_METRIC_BATCH_SIZE`.
    MetricIngest {
        /// Batch of metric data points (max `MAX_METRIC_BATCH_SIZE`).
        data_points: Vec<observability::MetricDataPoint>,
        /// TTL in seconds for stored data points (default `METRIC_DEFAULT_TTL_SECONDS`).
        ttl_seconds: Option<u32>,
    },

    /// List available metric names and metadata.
    ///
    /// Scans `_sys:metrics_meta:` prefix.
    MetricList {
        /// Optional prefix filter on metric name.
        prefix: Option<String>,
        /// Maximum results (default `MAX_METRIC_LIST_RESULTS`).
        limit: Option<u32>,
    },

    /// Query metric data points by name and time range.
    ///
    /// Scans `_sys:metrics:{name}:` prefix, filters by time range and labels.
    MetricQuery {
        /// Metric name to query.
        name: String,
        /// Start of time range (Unix microseconds, inclusive).
        start_time_us: Option<u64>,
        /// End of time range (Unix microseconds, exclusive).
        end_time_us: Option<u64>,
        /// Label filters: only return data points matching ALL labels.
        label_filters: Vec<(String, String)>,
        /// Aggregation: "none" (raw points), "avg", "max", "min", "sum", "count".
        aggregation: Option<String>,
        /// Aggregation step/bucket size in microseconds (for time-series downsampling).
        step_us: Option<u64>,
        /// Maximum data points to return (default `DEFAULT_METRIC_QUERY_LIMIT`).
        limit: Option<u32>,
    },

    // =========================================================================
    // Alert operations
    // =========================================================================
    /// Create or update an alert rule.
    ///
    /// Stored under `_sys:alerts:rule:{name}`. Initializes state to Ok.
    /// Total rules bounded by `MAX_ALERT_RULES`.
    AlertCreate {
        /// Alert rule definition.
        rule: observability::AlertRuleWire,
    },

    /// Delete an alert rule and its state/history.
    AlertDelete {
        /// Rule name to delete.
        name: String,
    },

    /// List all alert rules with current state.
    AlertList,

    /// Get a single alert rule with state and history.
    AlertGet {
        /// Rule name to retrieve.
        name: String,
    },

    /// On-demand evaluation of an alert rule against stored metrics.
    ///
    /// Reads metric data for the rule's window, computes aggregation,
    /// compares against threshold, updates state in KV, records history.
    AlertEvaluate {
        /// Rule name to evaluate.
        name: String,
        /// Current timestamp in Unix microseconds (explicit for determinism).
        now_us: u64,
    },

    // =========================================================================
    // Index operations
    // =========================================================================
    /// Create a custom secondary index.
    ///
    /// Custom indexes enable efficient range scans on user-defined fields.
    /// Index entries are updated atomically with primary KV writes.
    IndexCreate {
        /// Index name (must be unique, max MAX_INDEX_NAME_SIZE bytes).
        name: String,
        /// Field to index (e.g., metadata field name).
        field: String,
        /// Field type: "integer", "unsignedinteger", or "string".
        field_type: String,
        /// Whether to enforce uniqueness.
        is_unique: bool,
        /// Whether to index null values.
        should_index_nulls: bool,
    },

    /// Drop a custom secondary index.
    ///
    /// Built-in indexes cannot be dropped.
    IndexDrop {
        /// Name of the index to drop.
        name: String,
    },

    /// Scan a secondary index for matching entries.
    ///
    /// Returns primary keys of entries matching the index query.
    IndexScan {
        /// Index name to scan.
        index_name: String,
        /// Scan mode: "exact", "range", or "lt".
        mode: String,
        /// Value to match (for exact) or range start. Hex-encoded bytes.
        value: String,
        /// Range end (for range mode). Hex-encoded bytes.
        end_value: Option<String>,
        /// Maximum results (default 1000, max MAX_SCAN_RESULTS).
        limit: Option<u32>,
    },

    /// List all secondary indexes.
    IndexList,

    // =========================================================================
    // Plugin management operations
    // =========================================================================
    /// Reload WASM plugins.
    ///
    /// When `name` is `Some`, reloads a single plugin. When `None`, reloads
    /// all WASM plugins. Triggers shutdown of old plugin instances, re-scans
    /// KV store for manifests, and atomically swaps the handler list.
    PluginReload {
        /// Plugin name to reload, or `None` to reload all.
        name: Option<String>,
    },
}

#[cfg(feature = "auth")]
impl ClientRpcRequest {
    /// Convert the request to an authorization operation.
    ///
    /// Returns None for operations that don't require authorization.
    pub fn to_operation(&self) -> Option<aspen_auth::Operation> {
        to_operation::to_operation(self)
    }
}

impl ClientRpcRequest {
    /// Returns the serde variant name of this request as a `&'static str`.
    ///
    /// This is a zero-allocation alternative to serializing the request to JSON
    /// just to extract the variant name. Used by WASM plugin dispatch to match
    /// requests against plugin `handles` lists without touching serde.
    ///
    /// The returned string matches the serde external-tag key exactly (e.g.
    /// `"ReadKey"`, `"Ping"`, `"ForgeCreateRepo"`).
    pub fn variant_name(&self) -> &'static str {
        match self {
            Self::AddBlob { .. } => "AddBlob",
            Self::AddLearner { .. } => "AddLearner",
            Self::AddPeer { .. } => "AddPeer",
            Self::AddPeerCluster { .. } => "AddPeerCluster",
            #[cfg(feature = "automerge")]
            Self::AutomergeApplyChanges { .. } => "AutomergeApplyChanges",
            #[cfg(feature = "automerge")]
            Self::AutomergeCreate { .. } => "AutomergeCreate",
            #[cfg(feature = "automerge")]
            Self::AutomergeDelete { .. } => "AutomergeDelete",
            #[cfg(feature = "automerge")]
            Self::AutomergeExists { .. } => "AutomergeExists",
            #[cfg(feature = "automerge")]
            Self::AutomergeGenerateSyncMessage { .. } => "AutomergeGenerateSyncMessage",
            #[cfg(feature = "automerge")]
            Self::AutomergeGet { .. } => "AutomergeGet",
            #[cfg(feature = "automerge")]
            Self::AutomergeGetMetadata { .. } => "AutomergeGetMetadata",
            #[cfg(feature = "automerge")]
            Self::AutomergeList { .. } => "AutomergeList",
            #[cfg(feature = "automerge")]
            Self::AutomergeMerge { .. } => "AutomergeMerge",
            #[cfg(feature = "automerge")]
            Self::AutomergeReceiveSyncMessage { .. } => "AutomergeReceiveSyncMessage",
            #[cfg(feature = "automerge")]
            Self::AutomergeSave { .. } => "AutomergeSave",
            Self::BarrierEnter { .. } => "BarrierEnter",
            Self::BarrierLeave { .. } => "BarrierLeave",
            Self::BarrierStatus { .. } => "BarrierStatus",
            Self::BatchRead { .. } => "BatchRead",
            Self::BatchWrite { .. } => "BatchWrite",
            Self::BlobReplicatePull { .. } => "BlobReplicatePull",
            Self::CacheDownload { .. } => "CacheDownload",
            #[cfg(feature = "ci")]
            Self::CacheMigrationCancel => "CacheMigrationCancel",
            #[cfg(feature = "ci")]
            Self::CacheMigrationStart { .. } => "CacheMigrationStart",
            #[cfg(feature = "ci")]
            Self::CacheMigrationStatus => "CacheMigrationStatus",
            #[cfg(feature = "ci")]
            Self::CacheMigrationValidate { .. } => "CacheMigrationValidate",
            Self::CacheQuery { .. } => "CacheQuery",
            Self::CacheStats => "CacheStats",
            Self::ChangeMembership { .. } => "ChangeMembership",
            Self::CheckpointWal => "CheckpointWal",
            Self::CiCancelRun { .. } => "CiCancelRun",
            Self::CiGetArtifact { .. } => "CiGetArtifact",
            Self::CiGetJobLogs { .. } => "CiGetJobLogs",
            Self::CiGetJobOutput { .. } => "CiGetJobOutput",
            Self::CiGetStatus { .. } => "CiGetStatus",
            Self::CiListArtifacts { .. } => "CiListArtifacts",
            Self::CiListRuns { .. } => "CiListRuns",
            Self::CiSubscribeLogs { .. } => "CiSubscribeLogs",
            Self::CiTriggerPipeline { .. } => "CiTriggerPipeline",
            Self::CiUnwatchRepo { .. } => "CiUnwatchRepo",
            Self::CiWatchRepo { .. } => "CiWatchRepo",
            Self::CompareAndDeleteKey { .. } => "CompareAndDeleteKey",
            Self::CompareAndSwapKey { .. } => "CompareAndSwapKey",
            Self::ConditionalBatchWrite { .. } => "ConditionalBatchWrite",
            Self::CounterAdd { .. } => "CounterAdd",
            Self::CounterCompareAndSet { .. } => "CounterCompareAndSet",
            Self::CounterDecrement { .. } => "CounterDecrement",
            Self::CounterGet { .. } => "CounterGet",
            Self::CounterIncrement { .. } => "CounterIncrement",
            Self::CounterSet { .. } => "CounterSet",
            Self::CounterSubtract { .. } => "CounterSubtract",
            Self::DeleteBlob { .. } => "DeleteBlob",
            Self::DeleteKey { .. } => "DeleteKey",
            Self::DocsDelete { .. } => "DocsDelete",
            Self::DocsGet { .. } => "DocsGet",
            Self::DocsList { .. } => "DocsList",
            Self::DocsSet { .. } => "DocsSet",
            Self::DocsStatus => "DocsStatus",
            Self::DownloadBlob { .. } => "DownloadBlob",
            Self::DownloadBlobByHash { .. } => "DownloadBlobByHash",
            Self::DownloadBlobByProvider { .. } => "DownloadBlobByProvider",
            Self::ExecuteSql { .. } => "ExecuteSql",
            Self::FederateRepository { .. } => "FederateRepository",
            Self::ForgeApprovePatch { .. } => "ForgeApprovePatch",
            Self::ForgeCasRef { .. } => "ForgeCasRef",
            Self::ForgeCloseIssue { .. } => "ForgeCloseIssue",
            Self::ForgeClosePatch { .. } => "ForgeClosePatch",
            Self::ForgeCommentIssue { .. } => "ForgeCommentIssue",
            Self::ForgeCommit { .. } => "ForgeCommit",
            Self::ForgeCreateIssue { .. } => "ForgeCreateIssue",
            Self::ForgeCreatePatch { .. } => "ForgeCreatePatch",
            Self::ForgeCreateRepo { .. } => "ForgeCreateRepo",
            Self::ForgeCreateTree { .. } => "ForgeCreateTree",
            Self::ForgeDeleteRef { .. } => "ForgeDeleteRef",
            Self::ForgeFetchFederated { .. } => "ForgeFetchFederated",
            Self::ForgeGetBlob { .. } => "ForgeGetBlob",
            Self::ForgeGetCommit { .. } => "ForgeGetCommit",
            Self::ForgeGetDelegateKey { .. } => "ForgeGetDelegateKey",
            Self::ForgeGetIssue { .. } => "ForgeGetIssue",
            Self::ForgeGetPatch { .. } => "ForgeGetPatch",
            Self::ForgeGetRef { .. } => "ForgeGetRef",
            Self::ForgeGetRepo { .. } => "ForgeGetRepo",
            Self::ForgeGetTree { .. } => "ForgeGetTree",
            Self::ForgeListBranches { .. } => "ForgeListBranches",
            Self::ForgeListIssues { .. } => "ForgeListIssues",
            Self::ForgeListPatches { .. } => "ForgeListPatches",
            Self::ForgeListRepos { .. } => "ForgeListRepos",
            Self::ForgeListTags { .. } => "ForgeListTags",
            Self::ForgeLog { .. } => "ForgeLog",
            Self::ForgeMergePatch { .. } => "ForgeMergePatch",
            Self::ForgeReopenIssue { .. } => "ForgeReopenIssue",
            Self::ForgeSetRef { .. } => "ForgeSetRef",
            Self::ForgeStoreBlob { .. } => "ForgeStoreBlob",
            Self::ForgeUpdatePatch { .. } => "ForgeUpdatePatch",
            Self::GetBlob { .. } => "GetBlob",
            Self::GetBlobReplicationStatus { .. } => "GetBlobReplicationStatus",
            Self::GetBlobStatus { .. } => "GetBlobStatus",
            Self::GetBlobTicket { .. } => "GetBlobTicket",
            Self::GetClientTicket { .. } => "GetClientTicket",
            Self::GetClusterState => "GetClusterState",
            Self::GetClusterTicket => "GetClusterTicket",
            Self::GetClusterTicketCombined { .. } => "GetClusterTicketCombined",
            Self::GetDiscoveredCluster { .. } => "GetDiscoveredCluster",
            Self::GetDocsTicket { .. } => "GetDocsTicket",
            Self::GetFederationStatus => "GetFederationStatus",
            Self::GetHealth => "GetHealth",
            Self::GetKeyOrigin { .. } => "GetKeyOrigin",
            Self::GetLeader => "GetLeader",
            Self::GetMetrics => "GetMetrics",
            Self::GetNodeInfo => "GetNodeInfo",
            Self::GetPeerClusterStatus { .. } => "GetPeerClusterStatus",
            Self::GetRaftMetrics => "GetRaftMetrics",
            Self::GetTopology { .. } => "GetTopology",
            Self::GetVaultKeys { .. } => "GetVaultKeys",
            Self::GitBridgeFetch { .. } => "GitBridgeFetch",
            Self::GitBridgeListRefs { .. } => "GitBridgeListRefs",
            Self::GitBridgePush { .. } => "GitBridgePush",
            Self::GitBridgePushChunk { .. } => "GitBridgePushChunk",
            Self::GitBridgePushComplete { .. } => "GitBridgePushComplete",
            Self::GitBridgeProbeObjects { .. } => "GitBridgeProbeObjects",
            Self::GitBridgePushStart { .. } => "GitBridgePushStart",
            Self::HasBlob { .. } => "HasBlob",
            Self::HookGetMetrics { .. } => "HookGetMetrics",
            Self::HookList => "HookList",
            Self::HookTrigger { .. } => "HookTrigger",
            Self::InitCluster => "InitCluster",
            Self::JobCancel { .. } => "JobCancel",
            Self::JobGet { .. } => "JobGet",
            Self::JobList { .. } => "JobList",
            Self::JobQueueStats => "JobQueueStats",
            Self::JobSubmit { .. } => "JobSubmit",
            Self::JobUpdateProgress { .. } => "JobUpdateProgress",
            Self::LeaseGrant { .. } => "LeaseGrant",
            Self::LeaseKeepalive { .. } => "LeaseKeepalive",
            Self::LeaseList => "LeaseList",
            Self::LeaseRevoke { .. } => "LeaseRevoke",
            Self::LeaseTimeToLive { .. } => "LeaseTimeToLive",
            Self::ListBlobs { .. } => "ListBlobs",
            Self::ListDiscoveredClusters => "ListDiscoveredClusters",
            Self::ListFederatedRepositories => "ListFederatedRepositories",
            Self::ListPeerClusters => "ListPeerClusters",
            Self::ListVaults => "ListVaults",
            Self::LockAcquire { .. } => "LockAcquire",
            Self::LockRelease { .. } => "LockRelease",
            Self::LockRenew { .. } => "LockRenew",
            Self::LockTryAcquire { .. } => "LockTryAcquire",
            Self::Ping => "Ping",
            Self::PromoteLearner { .. } => "PromoteLearner",
            Self::ProtectBlob { .. } => "ProtectBlob",
            Self::QueueAck { .. } => "QueueAck",
            Self::QueueCreate { .. } => "QueueCreate",
            Self::QueueDelete { .. } => "QueueDelete",
            Self::QueueDequeue { .. } => "QueueDequeue",
            Self::QueueDequeueWait { .. } => "QueueDequeueWait",
            Self::QueueEnqueue { .. } => "QueueEnqueue",
            Self::QueueEnqueueBatch { .. } => "QueueEnqueueBatch",
            Self::QueueExtendVisibility { .. } => "QueueExtendVisibility",
            Self::QueueGetDLQ { .. } => "QueueGetDLQ",
            Self::QueueNack { .. } => "QueueNack",
            Self::QueuePeek { .. } => "QueuePeek",
            Self::QueueRedriveDLQ { .. } => "QueueRedriveDLQ",
            Self::QueueStatus { .. } => "QueueStatus",
            Self::RateLimiterAcquire { .. } => "RateLimiterAcquire",
            Self::RateLimiterAvailable { .. } => "RateLimiterAvailable",
            Self::RateLimiterReset { .. } => "RateLimiterReset",
            Self::RateLimiterTryAcquire { .. } => "RateLimiterTryAcquire",
            Self::ReadKey { .. } => "ReadKey",
            Self::RemovePeerCluster { .. } => "RemovePeerCluster",
            Self::RunBlobRepairCycle => "RunBlobRepairCycle",
            Self::RWLockAcquireRead { .. } => "RWLockAcquireRead",
            Self::RWLockAcquireWrite { .. } => "RWLockAcquireWrite",
            Self::RWLockDowngrade { .. } => "RWLockDowngrade",
            Self::RWLockReleaseRead { .. } => "RWLockReleaseRead",
            Self::RWLockReleaseWrite { .. } => "RWLockReleaseWrite",
            Self::RWLockStatus { .. } => "RWLockStatus",
            Self::RWLockTryAcquireRead { .. } => "RWLockTryAcquireRead",
            Self::RWLockTryAcquireWrite { .. } => "RWLockTryAcquireWrite",
            Self::ScanKeys { .. } => "ScanKeys",
            Self::SecretsKvDelete { .. } => "SecretsKvDelete",
            Self::SecretsKvDeleteMetadata { .. } => "SecretsKvDeleteMetadata",
            Self::SecretsKvDestroy { .. } => "SecretsKvDestroy",
            Self::SecretsKvList { .. } => "SecretsKvList",
            Self::SecretsKvMetadata { .. } => "SecretsKvMetadata",
            Self::SecretsKvRead { .. } => "SecretsKvRead",
            Self::SecretsKvUndelete { .. } => "SecretsKvUndelete",
            Self::SecretsKvUpdateMetadata { .. } => "SecretsKvUpdateMetadata",
            Self::SecretsKvWrite { .. } => "SecretsKvWrite",
            Self::SecretsNixCacheCreateKey { .. } => "SecretsNixCacheCreateKey",
            Self::SecretsNixCacheDeleteKey { .. } => "SecretsNixCacheDeleteKey",
            Self::SecretsNixCacheGetPublicKey { .. } => "SecretsNixCacheGetPublicKey",
            Self::SecretsNixCacheListKeys { .. } => "SecretsNixCacheListKeys",
            Self::SecretsNixCacheRotateKey { .. } => "SecretsNixCacheRotateKey",
            Self::SecretsPkiCreateRole { .. } => "SecretsPkiCreateRole",
            Self::SecretsPkiGenerateIntermediate { .. } => "SecretsPkiGenerateIntermediate",
            Self::SecretsPkiGenerateRoot { .. } => "SecretsPkiGenerateRoot",
            Self::SecretsPkiGetCrl { .. } => "SecretsPkiGetCrl",
            Self::SecretsPkiGetRole { .. } => "SecretsPkiGetRole",
            Self::SecretsPkiIssue { .. } => "SecretsPkiIssue",
            Self::SecretsPkiListCerts { .. } => "SecretsPkiListCerts",
            Self::SecretsPkiListRoles { .. } => "SecretsPkiListRoles",
            Self::SecretsPkiRevoke { .. } => "SecretsPkiRevoke",
            Self::SecretsPkiSetSignedIntermediate { .. } => "SecretsPkiSetSignedIntermediate",
            Self::SecretsTransitCreateKey { .. } => "SecretsTransitCreateKey",
            Self::SecretsTransitDatakey { .. } => "SecretsTransitDatakey",
            Self::SecretsTransitDecrypt { .. } => "SecretsTransitDecrypt",
            Self::SecretsTransitEncrypt { .. } => "SecretsTransitEncrypt",
            Self::SecretsTransitListKeys { .. } => "SecretsTransitListKeys",
            Self::SecretsTransitRewrap { .. } => "SecretsTransitRewrap",
            Self::SecretsTransitRotateKey { .. } => "SecretsTransitRotateKey",
            Self::SecretsTransitSign { .. } => "SecretsTransitSign",
            Self::SecretsTransitVerify { .. } => "SecretsTransitVerify",
            Self::SemaphoreAcquire { .. } => "SemaphoreAcquire",
            Self::SemaphoreRelease { .. } => "SemaphoreRelease",
            Self::SemaphoreStatus { .. } => "SemaphoreStatus",
            Self::SemaphoreTryAcquire { .. } => "SemaphoreTryAcquire",
            Self::SequenceCurrent { .. } => "SequenceCurrent",
            Self::SequenceNext { .. } => "SequenceNext",
            Self::SequenceReserve { .. } => "SequenceReserve",
            Self::ServiceDeregister { .. } => "ServiceDeregister",
            Self::ServiceDiscover { .. } => "ServiceDiscover",
            Self::ServiceGetInstance { .. } => "ServiceGetInstance",
            Self::ServiceHeartbeat { .. } => "ServiceHeartbeat",
            Self::ServiceList { .. } => "ServiceList",
            Self::ServiceRegister { .. } => "ServiceRegister",
            Self::ServiceUpdateHealth { .. } => "ServiceUpdateHealth",
            Self::ServiceUpdateMetadata { .. } => "ServiceUpdateMetadata",
            Self::SetPeerClusterEnabled { .. } => "SetPeerClusterEnabled",
            Self::SignedCounterAdd { .. } => "SignedCounterAdd",
            Self::SignedCounterGet { .. } => "SignedCounterGet",
            Self::SnixDirectoryGet { .. } => "SnixDirectoryGet",
            Self::SnixDirectoryPut { .. } => "SnixDirectoryPut",
            Self::SnixPathInfoGet { .. } => "SnixPathInfoGet",
            Self::SnixPathInfoPut { .. } => "SnixPathInfoPut",
            Self::TriggerBlobReplication { .. } => "TriggerBlobReplication",
            Self::TriggerSnapshot => "TriggerSnapshot",
            Self::TrustCluster { .. } => "TrustCluster",
            Self::UnprotectBlob { .. } => "UnprotectBlob",
            Self::UntrustCluster { .. } => "UntrustCluster",
            Self::UpdatePeerClusterFilter { .. } => "UpdatePeerClusterFilter",
            Self::UpdatePeerClusterPriority { .. } => "UpdatePeerClusterPriority",
            Self::WatchCancel { .. } => "WatchCancel",
            Self::WatchCreate { .. } => "WatchCreate",
            Self::WatchStatus { .. } => "WatchStatus",
            Self::WorkerCompleteJob { .. } => "WorkerCompleteJob",
            Self::WorkerDeregister { .. } => "WorkerDeregister",
            Self::WorkerHeartbeat { .. } => "WorkerHeartbeat",
            Self::WorkerPollJobs { .. } => "WorkerPollJobs",
            Self::WorkerRegister { .. } => "WorkerRegister",
            Self::WorkerStatus => "WorkerStatus",
            Self::WriteKey { .. } => "WriteKey",
            Self::WriteKeyWithLease { .. } => "WriteKeyWithLease",
            Self::TraceIngest { .. } => "TraceIngest",
            Self::TraceList { .. } => "TraceList",
            Self::TraceGet { .. } => "TraceGet",
            Self::TraceSearch { .. } => "TraceSearch",
            Self::MetricIngest { .. } => "MetricIngest",
            Self::MetricList { .. } => "MetricList",
            Self::MetricQuery { .. } => "MetricQuery",
            Self::AlertCreate { .. } => "AlertCreate",
            Self::AlertDelete { .. } => "AlertDelete",
            Self::AlertList => "AlertList",
            Self::AlertGet { .. } => "AlertGet",
            Self::AlertEvaluate { .. } => "AlertEvaluate",
            Self::IndexCreate { .. } => "IndexCreate",
            Self::IndexDrop { .. } => "IndexDrop",
            Self::IndexScan { .. } => "IndexScan",
            Self::IndexList => "IndexList",
            Self::PluginReload { .. } => "PluginReload",
        }
    }
}

impl ClientRpcRequest {
    /// Returns the app ID required to handle this request, or None for core requests.
    ///
    /// This is a pure function used by the handler registry for capability-aware dispatch.
    pub fn required_app(&self) -> Option<&'static str> {
        match self {
            // Core operations - always available
            Self::GetHealth
            | Self::GetRaftMetrics
            | Self::GetLeader
            | Self::GetNodeInfo
            | Self::GetClusterTicket
            | Self::InitCluster
            | Self::ReadKey { .. }
            | Self::WriteKey { .. }
            | Self::CompareAndSwapKey { .. }
            | Self::CompareAndDeleteKey { .. }
            | Self::TriggerSnapshot
            | Self::AddLearner { .. }
            | Self::ChangeMembership { .. }
            | Self::Ping
            | Self::GetClusterState
            | Self::DeleteKey { .. }
            | Self::ScanKeys { .. }
            | Self::GetMetrics
            | Self::PromoteLearner { .. }
            | Self::CheckpointWal
            | Self::ListVaults
            | Self::GetVaultKeys { .. }
            | Self::AddPeer { .. }
            | Self::GetClusterTicketCombined { .. }
            | Self::GetClientTicket { .. }
            | Self::GetDocsTicket { .. }
            | Self::GetTopology { .. } => None,

            // KV batch operations
            Self::BatchRead { .. } | Self::BatchWrite { .. } | Self::ConditionalBatchWrite { .. } => None,

            // Watch operations
            Self::WatchCreate { .. } | Self::WatchCancel { .. } | Self::WatchStatus { .. } => None,

            // Lease operations
            Self::LeaseGrant { .. }
            | Self::LeaseRevoke { .. }
            | Self::LeaseKeepalive { .. }
            | Self::LeaseTimeToLive { .. }
            | Self::LeaseList
            | Self::WriteKeyWithLease { .. }
            | Self::TraceIngest { .. }
            | Self::TraceList { .. }
            | Self::TraceGet { .. }
            | Self::TraceSearch { .. }
            | Self::MetricIngest { .. }
            | Self::MetricList { .. }
            | Self::MetricQuery { .. }
            | Self::AlertCreate { .. }
            | Self::AlertDelete { .. }
            | Self::AlertList
            | Self::AlertGet { .. }
            | Self::AlertEvaluate { .. }
            | Self::IndexCreate { .. }
            | Self::IndexDrop { .. }
            | Self::IndexScan { .. }
            | Self::IndexList
            | Self::PluginReload { .. } => None,

            // Coordination primitives
            Self::LockAcquire { .. }
            | Self::LockTryAcquire { .. }
            | Self::LockRelease { .. }
            | Self::LockRenew { .. }
            | Self::CounterGet { .. }
            | Self::CounterIncrement { .. }
            | Self::CounterDecrement { .. }
            | Self::CounterAdd { .. }
            | Self::CounterSubtract { .. }
            | Self::CounterSet { .. }
            | Self::CounterCompareAndSet { .. }
            | Self::SignedCounterGet { .. }
            | Self::SignedCounterAdd { .. }
            | Self::SequenceNext { .. }
            | Self::SequenceReserve { .. }
            | Self::SequenceCurrent { .. }
            | Self::RateLimiterTryAcquire { .. }
            | Self::RateLimiterAcquire { .. }
            | Self::RateLimiterAvailable { .. }
            | Self::RateLimiterReset { .. }
            | Self::BarrierEnter { .. }
            | Self::BarrierLeave { .. }
            | Self::BarrierStatus { .. }
            | Self::SemaphoreAcquire { .. }
            | Self::SemaphoreTryAcquire { .. }
            | Self::SemaphoreRelease { .. }
            | Self::SemaphoreStatus { .. }
            | Self::RWLockAcquireRead { .. }
            | Self::RWLockTryAcquireRead { .. }
            | Self::RWLockAcquireWrite { .. }
            | Self::RWLockTryAcquireWrite { .. }
            | Self::RWLockReleaseRead { .. }
            | Self::RWLockReleaseWrite { .. }
            | Self::RWLockDowngrade { .. }
            | Self::RWLockStatus { .. }
            | Self::QueueCreate { .. }
            | Self::QueueDelete { .. }
            | Self::QueueEnqueue { .. }
            | Self::QueueEnqueueBatch { .. }
            | Self::QueueDequeue { .. }
            | Self::QueueDequeueWait { .. }
            | Self::QueuePeek { .. }
            | Self::QueueAck { .. }
            | Self::QueueNack { .. }
            | Self::QueueExtendVisibility { .. }
            | Self::QueueStatus { .. }
            | Self::QueueGetDLQ { .. }
            | Self::QueueRedriveDLQ { .. } => None,

            // Service Registry
            Self::ServiceRegister { .. }
            | Self::ServiceDeregister { .. }
            | Self::ServiceDiscover { .. }
            | Self::ServiceList { .. }
            | Self::ServiceGetInstance { .. }
            | Self::ServiceHeartbeat { .. }
            | Self::ServiceUpdateHealth { .. }
            | Self::ServiceUpdateMetadata { .. } => None,

            // Blob operations
            Self::AddBlob { .. }
            | Self::GetBlob { .. }
            | Self::HasBlob { .. }
            | Self::GetBlobTicket { .. }
            | Self::ListBlobs { .. }
            | Self::ProtectBlob { .. }
            | Self::UnprotectBlob { .. }
            | Self::DeleteBlob { .. }
            | Self::DownloadBlob { .. }
            | Self::DownloadBlobByHash { .. }
            | Self::DownloadBlobByProvider { .. }
            | Self::GetBlobStatus { .. }
            | Self::BlobReplicatePull { .. }
            | Self::GetBlobReplicationStatus { .. }
            | Self::TriggerBlobReplication { .. }
            | Self::RunBlobRepairCycle => None,

            // Docs operations
            Self::DocsSet { .. }
            | Self::DocsGet { .. }
            | Self::DocsDelete { .. }
            | Self::DocsList { .. }
            | Self::DocsStatus => None,

            // Peer cluster operations
            Self::AddPeerCluster { .. }
            | Self::RemovePeerCluster { .. }
            | Self::ListPeerClusters
            | Self::GetPeerClusterStatus { .. }
            | Self::UpdatePeerClusterFilter { .. }
            | Self::UpdatePeerClusterPriority { .. }
            | Self::SetPeerClusterEnabled { .. }
            | Self::GetKeyOrigin { .. } => None,

            // Forge operations
            Self::ForgeCreateRepo { .. }
            | Self::ForgeGetRepo { .. }
            | Self::ForgeListRepos { .. }
            | Self::ForgeStoreBlob { .. }
            | Self::ForgeGetBlob { .. }
            | Self::ForgeCreateTree { .. }
            | Self::ForgeGetTree { .. }
            | Self::ForgeCommit { .. }
            | Self::ForgeGetCommit { .. }
            | Self::ForgeLog { .. }
            | Self::ForgeGetRef { .. }
            | Self::ForgeSetRef { .. }
            | Self::ForgeDeleteRef { .. }
            | Self::ForgeCasRef { .. }
            | Self::ForgeListBranches { .. }
            | Self::ForgeListTags { .. }
            | Self::ForgeCreateIssue { .. }
            | Self::ForgeListIssues { .. }
            | Self::ForgeGetIssue { .. }
            | Self::ForgeCommentIssue { .. }
            | Self::ForgeCloseIssue { .. }
            | Self::ForgeReopenIssue { .. }
            | Self::ForgeCreatePatch { .. }
            | Self::ForgeListPatches { .. }
            | Self::ForgeGetPatch { .. }
            | Self::ForgeUpdatePatch { .. }
            | Self::ForgeApprovePatch { .. }
            | Self::ForgeMergePatch { .. }
            | Self::ForgeClosePatch { .. }
            | Self::ForgeGetDelegateKey { .. } => Some("forge"),

            // Federation operations
            Self::GetFederationStatus
            | Self::ListDiscoveredClusters
            | Self::GetDiscoveredCluster { .. }
            | Self::TrustCluster { .. }
            | Self::UntrustCluster { .. }
            | Self::FederateRepository { .. }
            | Self::ListFederatedRepositories
            | Self::ForgeFetchFederated { .. } => Some("forge"),

            // Git Bridge operations
            Self::GitBridgeListRefs { .. }
            | Self::GitBridgeFetch { .. }
            | Self::GitBridgePush { .. }
            | Self::GitBridgePushStart { .. }
            | Self::GitBridgePushChunk { .. }
            | Self::GitBridgePushComplete { .. }
            | Self::GitBridgeProbeObjects { .. } => Some("forge"),

            // SQL
            Self::ExecuteSql { .. } => Some("sql"),

            // Job operations
            Self::JobSubmit { .. }
            | Self::JobGet { .. }
            | Self::JobList { .. }
            | Self::JobCancel { .. }
            | Self::JobUpdateProgress { .. }
            | Self::JobQueueStats
            | Self::WorkerStatus
            | Self::WorkerRegister { .. }
            | Self::WorkerHeartbeat { .. }
            | Self::WorkerDeregister { .. }
            | Self::WorkerPollJobs { .. }
            | Self::WorkerCompleteJob { .. } => Some("jobs"),

            // Hook operations
            Self::HookList | Self::HookGetMetrics { .. } | Self::HookTrigger { .. } => Some("hooks"),

            // CI operations
            Self::CiTriggerPipeline { .. }
            | Self::CiGetStatus { .. }
            | Self::CiListRuns { .. }
            | Self::CiCancelRun { .. }
            | Self::CiWatchRepo { .. }
            | Self::CiUnwatchRepo { .. }
            | Self::CiListArtifacts { .. }
            | Self::CiGetArtifact { .. }
            | Self::CiGetJobLogs { .. }
            | Self::CiSubscribeLogs { .. }
            | Self::CiGetJobOutput { .. } => Some("ci"),

            // Secrets operations
            Self::SecretsKvRead { .. }
            | Self::SecretsKvWrite { .. }
            | Self::SecretsKvDelete { .. }
            | Self::SecretsKvDestroy { .. }
            | Self::SecretsKvUndelete { .. }
            | Self::SecretsKvList { .. }
            | Self::SecretsKvMetadata { .. }
            | Self::SecretsKvUpdateMetadata { .. }
            | Self::SecretsKvDeleteMetadata { .. }
            | Self::SecretsTransitCreateKey { .. }
            | Self::SecretsTransitEncrypt { .. }
            | Self::SecretsTransitDecrypt { .. }
            | Self::SecretsTransitSign { .. }
            | Self::SecretsTransitVerify { .. }
            | Self::SecretsTransitRotateKey { .. }
            | Self::SecretsTransitListKeys { .. }
            | Self::SecretsTransitRewrap { .. }
            | Self::SecretsTransitDatakey { .. }
            | Self::SecretsPkiGenerateRoot { .. }
            | Self::SecretsPkiGenerateIntermediate { .. }
            | Self::SecretsPkiSetSignedIntermediate { .. }
            | Self::SecretsPkiCreateRole { .. }
            | Self::SecretsPkiIssue { .. }
            | Self::SecretsPkiRevoke { .. }
            | Self::SecretsPkiGetCrl { .. }
            | Self::SecretsPkiListCerts { .. }
            | Self::SecretsPkiGetRole { .. }
            | Self::SecretsPkiListRoles { .. }
            | Self::SecretsNixCacheCreateKey { .. }
            | Self::SecretsNixCacheGetPublicKey { .. }
            | Self::SecretsNixCacheRotateKey { .. }
            | Self::SecretsNixCacheDeleteKey { .. }
            | Self::SecretsNixCacheListKeys { .. } => Some("secrets"),

            // SNIX and Cache operations
            Self::CacheQuery { .. }
            | Self::CacheStats
            | Self::CacheDownload { .. }
            | Self::SnixDirectoryGet { .. }
            | Self::SnixDirectoryPut { .. }
            | Self::SnixPathInfoGet { .. }
            | Self::SnixPathInfoPut { .. } => Some("snix"),

            // Feature-gated: Cache Migration
            #[cfg(feature = "ci")]
            Self::CacheMigrationStart { .. }
            | Self::CacheMigrationStatus
            | Self::CacheMigrationCancel
            | Self::CacheMigrationValidate { .. } => Some("snix"),

            // Feature-gated: Automerge
            #[cfg(feature = "automerge")]
            Self::AutomergeCreate { .. }
            | Self::AutomergeGet { .. }
            | Self::AutomergeSave { .. }
            | Self::AutomergeDelete { .. }
            | Self::AutomergeApplyChanges { .. }
            | Self::AutomergeMerge { .. }
            | Self::AutomergeList { .. }
            | Self::AutomergeGetMetadata { .. }
            | Self::AutomergeExists { .. }
            | Self::AutomergeGenerateSyncMessage { .. }
            | Self::AutomergeReceiveSyncMessage { .. } => Some("automerge"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClientRpcResponse {
    /// Health status response.
    Health(HealthResponse),

    /// Raft metrics response.
    RaftMetrics(RaftMetricsResponse),

    /// Current leader response.
    Leader(Option<u64>),

    /// Node info response.
    NodeInfo(NodeInfoResponse),

    /// Cluster ticket response.
    ClusterTicket(ClusterTicketResponse),

    /// Cluster init response.
    InitResult(InitResultResponse),

    /// Read key response.
    ReadResult(ReadResultResponse),

    /// Write key response.
    WriteResult(WriteResultResponse),

    /// Compare-and-swap result response.
    CompareAndSwapResult(CompareAndSwapResultResponse),

    /// Snapshot trigger response.
    SnapshotResult(SnapshotResultResponse),

    /// Add learner response.
    AddLearnerResult(AddLearnerResultResponse),

    /// Change membership response.
    ChangeMembershipResult(ChangeMembershipResultResponse),

    /// Pong response for ping.
    Pong,

    /// Cluster state response.
    ClusterState(ClusterStateResponse),

    /// Error response for any request.
    Error(ErrorResponse),

    // =========================================================================
    // New responses (migrated from HTTP API)
    // =========================================================================
    /// Delete key response.
    DeleteResult(DeleteResultResponse),

    /// Scan keys response.
    ScanResult(ScanResultResponse),

    /// Prometheus metrics response.
    Metrics(MetricsResponse),

    /// Promote learner response.
    PromoteLearnerResult(PromoteLearnerResultResponse),

    /// Checkpoint WAL response.
    CheckpointWalResult(CheckpointWalResultResponse),

    /// List vaults response.
    VaultList(VaultListResponse),

    /// Vault keys response.
    VaultKeys(VaultKeysResponse),

    /// Add peer response.
    AddPeerResult(AddPeerResultResponse),

    /// Client ticket response for overlay subscription.
    ClientTicket(ClientTicketResponse),

    /// Docs ticket response for iroh-docs subscription.
    DocsTicket(DocsTicketResponse),

    // =========================================================================
    // Blob operation responses
    // =========================================================================
    /// Add blob result.
    AddBlobResult(AddBlobResultResponse),

    /// Get blob result.
    GetBlobResult(GetBlobResultResponse),

    /// Has blob result.
    HasBlobResult(HasBlobResultResponse),

    /// Get blob ticket result.
    GetBlobTicketResult(GetBlobTicketResultResponse),

    /// List blobs result.
    ListBlobsResult(ListBlobsResultResponse),

    /// Protect blob result.
    ProtectBlobResult(ProtectBlobResultResponse),

    /// Unprotect blob result.
    UnprotectBlobResult(UnprotectBlobResultResponse),

    /// Delete blob result.
    DeleteBlobResult(DeleteBlobResultResponse),

    /// Download blob result.
    DownloadBlobResult(DownloadBlobResultResponse),

    /// Download blob by hash result.
    DownloadBlobByHashResult(DownloadBlobResultResponse),

    /// Download blob by provider result.
    DownloadBlobByProviderResult(DownloadBlobResultResponse),

    /// Get blob status result.
    GetBlobStatusResult(GetBlobStatusResultResponse),

    /// Blob replicate pull result.
    BlobReplicatePullResult(BlobReplicatePullResultResponse),

    /// Get blob replication status result.
    GetBlobReplicationStatusResult(GetBlobReplicationStatusResultResponse),

    /// Trigger blob replication result.
    TriggerBlobReplicationResult(TriggerBlobReplicationResultResponse),

    /// Run blob repair cycle result.
    RunBlobRepairCycleResult(RunBlobRepairCycleResultResponse),

    // =========================================================================
    // Docs operation responses
    // =========================================================================
    /// Docs set result.
    DocsSetResult(DocsSetResultResponse),

    /// Docs get result.
    DocsGetResult(DocsGetResultResponse),

    /// Docs delete result.
    DocsDeleteResult(DocsDeleteResultResponse),

    /// Docs list result.
    DocsListResult(DocsListResultResponse),

    /// Docs status result.
    DocsStatusResult(DocsStatusResultResponse),

    // =========================================================================
    // Peer cluster operation responses
    // =========================================================================
    /// Add peer cluster result.
    AddPeerClusterResult(AddPeerClusterResultResponse),

    /// Remove peer cluster result.
    RemovePeerClusterResult(RemovePeerClusterResultResponse),

    /// List peer clusters result.
    ListPeerClustersResult(ListPeerClustersResultResponse),

    /// Get peer cluster status result.
    PeerClusterStatus(PeerClusterStatusResponse),

    /// Update peer cluster filter result.
    UpdatePeerClusterFilterResult(UpdatePeerClusterFilterResultResponse),

    /// Update peer cluster priority result.
    UpdatePeerClusterPriorityResult(UpdatePeerClusterPriorityResultResponse),

    /// Set peer cluster enabled result.
    SetPeerClusterEnabledResult(SetPeerClusterEnabledResultResponse),

    /// Key origin lookup result.
    KeyOriginResult(KeyOriginResultResponse),

    // =========================================================================
    // SQL query response
    // =========================================================================
    /// SQL query result.
    SqlResult(SqlResultResponse),

    // =========================================================================
    // Coordination primitive responses
    // =========================================================================
    /// Lock operation result (acquire, try_acquire, release, renew).
    LockResult(LockResultResponse),

    /// Atomic counter operation result.
    CounterResult(CounterResultResponse),

    /// Signed atomic counter operation result.
    SignedCounterResult(SignedCounterResultResponse),

    /// Sequence generator operation result.
    SequenceResult(SequenceResultResponse),

    /// Rate limiter operation result.
    RateLimiterResult(RateLimiterResultResponse),

    // =========================================================================
    // Batch operation responses
    // =========================================================================
    /// Batch read result.
    BatchReadResult(BatchReadResultResponse),

    /// Batch write result.
    BatchWriteResult(BatchWriteResultResponse),

    /// Conditional batch write result.
    ConditionalBatchWriteResult(ConditionalBatchWriteResultResponse),

    // =========================================================================
    // Watch operation responses
    // =========================================================================
    /// Watch creation result.
    WatchCreateResult(WatchCreateResultResponse),

    /// Watch cancellation result.
    WatchCancelResult(WatchCancelResultResponse),

    /// Watch status result.
    WatchStatusResult(WatchStatusResultResponse),

    /// Streaming watch event (delivered asynchronously after watch creation).
    /// NOTE: This is used for the streaming protocol, not request-response.
    WatchEvent(WatchEventResponse),

    // =========================================================================
    // Lease operation responses
    // =========================================================================
    /// Lease grant result.
    LeaseGrantResult(LeaseGrantResultResponse),

    /// Lease revoke result.
    LeaseRevokeResult(LeaseRevokeResultResponse),

    /// Lease keepalive result.
    LeaseKeepaliveResult(LeaseKeepaliveResultResponse),

    /// Lease time-to-live result.
    LeaseTimeToLiveResult(LeaseTimeToLiveResultResponse),

    /// Lease list result.
    LeaseListResult(LeaseListResultResponse),

    /// Barrier enter result.
    BarrierEnterResult(BarrierResultResponse),

    /// Barrier leave result.
    BarrierLeaveResult(BarrierResultResponse),

    /// Barrier status result.
    BarrierStatusResult(BarrierResultResponse),

    /// Semaphore acquire result.
    SemaphoreAcquireResult(SemaphoreResultResponse),

    /// Semaphore try-acquire result.
    SemaphoreTryAcquireResult(SemaphoreResultResponse),

    /// Semaphore release result.
    SemaphoreReleaseResult(SemaphoreResultResponse),

    /// Semaphore status result.
    SemaphoreStatusResult(SemaphoreResultResponse),

    // =========================================================================
    // Read-Write Lock responses
    // =========================================================================
    /// RWLock acquire read result.
    RWLockAcquireReadResult(RWLockResultResponse),

    /// RWLock try-acquire read result.
    RWLockTryAcquireReadResult(RWLockResultResponse),

    /// RWLock acquire write result.
    RWLockAcquireWriteResult(RWLockResultResponse),

    /// RWLock try-acquire write result.
    RWLockTryAcquireWriteResult(RWLockResultResponse),

    /// RWLock release read result.
    RWLockReleaseReadResult(RWLockResultResponse),

    /// RWLock release write result.
    RWLockReleaseWriteResult(RWLockResultResponse),

    /// RWLock downgrade result.
    RWLockDowngradeResult(RWLockResultResponse),

    /// RWLock status result.
    RWLockStatusResult(RWLockResultResponse),

    // =========================================================================
    // Queue responses
    // =========================================================================
    /// Queue create result.
    QueueCreateResult(QueueCreateResultResponse),

    /// Queue delete result.
    QueueDeleteResult(QueueDeleteResultResponse),

    /// Queue enqueue result.
    QueueEnqueueResult(QueueEnqueueResultResponse),

    /// Queue enqueue batch result.
    QueueEnqueueBatchResult(QueueEnqueueBatchResultResponse),

    /// Queue dequeue result.
    QueueDequeueResult(QueueDequeueResultResponse),

    /// Queue peek result.
    QueuePeekResult(QueuePeekResultResponse),

    /// Queue ack result.
    QueueAckResult(QueueAckResultResponse),

    /// Queue nack result.
    QueueNackResult(QueueNackResultResponse),

    /// Queue extend visibility result.
    QueueExtendVisibilityResult(QueueExtendVisibilityResultResponse),

    /// Queue status result.
    QueueStatusResult(QueueStatusResultResponse),

    /// Queue get DLQ result.
    QueueGetDLQResult(QueueGetDLQResultResponse),

    /// Queue redrive DLQ result.
    QueueRedriveDLQResult(QueueRedriveDLQResultResponse),

    // =========================================================================
    // Service Registry responses
    // =========================================================================
    /// Service register result.
    ServiceRegisterResult(ServiceRegisterResultResponse),

    /// Service deregister result.
    ServiceDeregisterResult(ServiceDeregisterResultResponse),

    /// Service discover result.
    ServiceDiscoverResult(ServiceDiscoverResultResponse),

    /// Service list result.
    ServiceListResult(ServiceListResultResponse),

    /// Service get instance result.
    ServiceGetInstanceResult(ServiceGetInstanceResultResponse),

    /// Service heartbeat result.
    ServiceHeartbeatResult(ServiceHeartbeatResultResponse),

    /// Service update health result.
    ServiceUpdateHealthResult(ServiceUpdateHealthResultResponse),

    /// Service update metadata result.
    ServiceUpdateMetadataResult(ServiceUpdateMetadataResultResponse),

    // =========================================================================
    // Sharding responses
    // =========================================================================
    /// Get topology result.
    TopologyResult(TopologyResultResponse),

    // =========================================================================
    // Forge responses (decentralized git)
    // =========================================================================
    /// Repository operation result.
    ForgeRepoResult(ForgeRepoResultResponse),

    /// Repository list result.
    ForgeRepoListResult(ForgeRepoListResultResponse),

    /// Blob operation result.
    ForgeBlobResult(ForgeBlobResultResponse),

    /// Tree operation result.
    ForgeTreeResult(ForgeTreeResultResponse),

    /// Commit operation result.
    ForgeCommitResult(ForgeCommitResultResponse),

    /// Commit log result.
    ForgeLogResult(ForgeLogResultResponse),

    /// Ref operation result.
    ForgeRefResult(ForgeRefResultResponse),

    /// Ref list result (branches or tags).
    ForgeRefListResult(ForgeRefListResultResponse),

    /// Issue operation result.
    ForgeIssueResult(ForgeIssueResultResponse),

    /// Issue list result.
    ForgeIssueListResult(ForgeIssueListResultResponse),

    /// Patch operation result.
    ForgePatchResult(ForgePatchResultResponse),

    /// Patch list result.
    ForgePatchListResult(ForgePatchListResultResponse),

    /// Generic forge operation success/error.
    ForgeOperationResult(ForgeOperationResultResponse),

    /// Delegate key result.
    ForgeKeyResult(ForgeKeyResultResponse),

    // =========================================================================
    // Federation operation responses
    // =========================================================================
    /// Federation status.
    FederationStatus(FederationStatusResponse),

    /// List of discovered clusters.
    DiscoveredClusters(DiscoveredClustersResponse),

    /// Single discovered cluster details.
    DiscoveredCluster(DiscoveredClusterResponse),

    /// Trust cluster result.
    TrustClusterResult(TrustClusterResultResponse),

    /// Untrust cluster result.
    UntrustClusterResult(UntrustClusterResultResponse),

    /// Federate repository result.
    FederateRepositoryResult(FederateRepositoryResultResponse),

    /// List of federated repositories.
    FederatedRepositories(FederatedRepositoriesResponse),

    /// Forge fetch federated result.
    ForgeFetchResult(ForgeFetchFederatedResultResponse),

    // =========================================================================
    // Git Bridge responses (for git-remote-aspen)
    // =========================================================================
    /// Git bridge list refs result.
    GitBridgeListRefs(GitBridgeListRefsResponse),

    /// Git bridge fetch result.
    GitBridgeFetch(GitBridgeFetchResponse),

    /// Git bridge push result.
    GitBridgePush(GitBridgePushResponse),

    /// Git bridge push start result (chunked transfer).
    GitBridgePushStart(GitBridgePushStartResponse),

    /// Git bridge push chunk result (chunked transfer).
    GitBridgePushChunk(GitBridgePushChunkResponse),

    /// Git bridge push complete result (chunked transfer).
    GitBridgePushComplete(GitBridgePushCompleteResponse),

    /// Git bridge probe objects result (incremental push).
    GitBridgeProbeObjects(GitBridgeProbeObjectsResponse),

    // =========================================================================
    // Job operation responses
    // =========================================================================
    /// Job submit result.
    JobSubmitResult(JobSubmitResultResponse),
    /// Job get result.
    JobGetResult(JobGetResultResponse),
    /// Job list result.
    JobListResult(JobListResultResponse),
    /// Job cancel result.
    JobCancelResult(JobCancelResultResponse),
    /// Job update progress result.
    JobUpdateProgressResult(JobUpdateProgressResultResponse),
    /// Job queue statistics result.
    JobQueueStatsResult(JobQueueStatsResultResponse),
    /// Worker status result.
    WorkerStatusResult(WorkerStatusResultResponse),
    /// Worker register result.
    WorkerRegisterResult(WorkerRegisterResultResponse),
    /// Worker heartbeat result.
    WorkerHeartbeatResult(WorkerHeartbeatResultResponse),
    /// Worker deregister result.
    WorkerDeregisterResult(WorkerDeregisterResultResponse),

    // =========================================================================
    // Hook operation responses
    // =========================================================================
    /// Hook list result.
    HookListResult(HookListResultResponse),
    /// Hook metrics result.
    HookMetricsResult(HookMetricsResultResponse),
    /// Hook trigger result.
    HookTriggerResult(HookTriggerResultResponse),

    // =========================================================================
    // CI/CD operation responses
    // =========================================================================
    /// CI trigger pipeline result.
    CiTriggerPipelineResult(CiTriggerPipelineResponse),
    /// CI get status result.
    CiGetStatusResult(CiGetStatusResponse),
    /// CI list runs result.
    CiListRunsResult(CiListRunsResponse),
    /// CI cancel run result.
    CiCancelRunResult(CiCancelRunResponse),
    /// CI watch repo result.
    CiWatchRepoResult(CiWatchRepoResponse),
    /// CI unwatch repo result.
    CiUnwatchRepoResult(CiUnwatchRepoResponse),
    /// CI list artifacts result.
    CiListArtifactsResult(CiListArtifactsResponse),
    /// CI get artifact result.
    CiGetArtifactResult(CiGetArtifactResponse),
    /// CI get job logs result.
    CiGetJobLogsResult(CiGetJobLogsResponse),
    /// CI subscribe logs result.
    CiSubscribeLogsResult(CiSubscribeLogsResponse),
    /// CI get job output result.
    CiGetJobOutputResult(CiGetJobOutputResponse),

    // =========================================================================
    // Nix Binary Cache responses
    // =========================================================================
    /// Cache query result.
    CacheQueryResult(CacheQueryResultResponse),
    /// Cache statistics result.
    CacheStatsResult(CacheStatsResultResponse),
    /// Cache download result (blob ticket).
    CacheDownloadResult(CacheDownloadResultResponse),

    // =========================================================================
    // Cache Migration responses (SNIX storage migration)
    // =========================================================================
    /// Cache migration started result.
    #[cfg(feature = "ci")]
    CacheMigrationStartResult(CacheMigrationStartResultResponse),
    /// Cache migration status result.
    #[cfg(feature = "ci")]
    CacheMigrationStatusResult(CacheMigrationStatusResultResponse),
    /// Cache migration cancel result.
    #[cfg(feature = "ci")]
    CacheMigrationCancelResult(CacheMigrationCancelResultResponse),
    /// Cache migration validation result.
    #[cfg(feature = "ci")]
    CacheMigrationValidateResult(CacheMigrationValidateResultResponse),

    // =========================================================================
    // Secrets operation responses
    // =========================================================================
    /// Secrets KV read result.
    SecretsKvReadResult(SecretsKvReadResultResponse),
    /// Secrets KV write result.
    SecretsKvWriteResult(SecretsKvWriteResultResponse),
    /// Secrets KV delete/destroy/undelete result.
    SecretsKvDeleteResult(SecretsKvDeleteResultResponse),
    /// Secrets KV list result.
    SecretsKvListResult(SecretsKvListResultResponse),
    /// Secrets KV metadata result.
    SecretsKvMetadataResult(SecretsKvMetadataResultResponse),
    /// Secrets Transit encrypt result.
    SecretsTransitEncryptResult(SecretsTransitEncryptResultResponse),
    /// Secrets Transit decrypt result.
    SecretsTransitDecryptResult(SecretsTransitDecryptResultResponse),
    /// Secrets Transit sign result.
    SecretsTransitSignResult(SecretsTransitSignResultResponse),
    /// Secrets Transit verify result.
    SecretsTransitVerifyResult(SecretsTransitVerifyResultResponse),
    /// Secrets Transit datakey result.
    SecretsTransitDatakeyResult(SecretsTransitDatakeyResultResponse),
    /// Secrets Transit key operation result (create, rotate).
    SecretsTransitKeyResult(SecretsTransitKeyResultResponse),
    /// Secrets Transit list keys result.
    SecretsTransitListResult(SecretsTransitListResultResponse),
    /// Secrets PKI certificate result (generate root, intermediate, issue).
    SecretsPkiCertificateResult(SecretsPkiCertificateResultResponse),
    /// Secrets PKI revoke result.
    SecretsPkiRevokeResult(SecretsPkiRevokeResultResponse),
    /// Secrets PKI CRL result.
    SecretsPkiCrlResult(SecretsPkiCrlResultResponse),
    /// Secrets PKI list result (certs, roles).
    SecretsPkiListResult(SecretsPkiListResultResponse),
    /// Secrets PKI role result.
    SecretsPkiRoleResult(SecretsPkiRoleResultResponse),

    /// Nix cache signing key result.
    SecretsNixCacheKeyResult(SecretsNixCacheKeyResultResponse),
    /// Nix cache key delete result.
    SecretsNixCacheDeleteResult(SecretsNixCacheDeleteResultResponse),
    /// Nix cache keys list result.
    SecretsNixCacheListResult(SecretsNixCacheListResultResponse),

    // =========================================================================
    // SNIX operation responses (for remote workers)
    // =========================================================================
    /// SNIX directory get result.
    SnixDirectoryGetResult(SnixDirectoryGetResultResponse),
    /// SNIX directory put result.
    SnixDirectoryPutResult(SnixDirectoryPutResultResponse),
    /// SNIX path info get result.
    SnixPathInfoGetResult(SnixPathInfoGetResultResponse),
    /// SNIX path info put result.
    SnixPathInfoPutResult(SnixPathInfoPutResultResponse),

    // -------------------------------------------------------------------------
    // Worker Job Coordination responses
    // -------------------------------------------------------------------------
    /// Worker job polling result.
    WorkerPollJobsResult(WorkerPollJobsResultResponse),
    /// Worker job completion result.
    WorkerCompleteJobResult(WorkerCompleteJobResultResponse),

    // -------------------------------------------------------------------------
    // Observability responses
    // -------------------------------------------------------------------------
    /// Trace ingest result.
    TraceIngestResult(TraceIngestResultResponse),
    /// Trace list result.
    TraceListResult(TraceListResultResponse),
    /// Trace get result.
    TraceGetResult(TraceGetResultResponse),
    /// Trace search result.
    TraceSearchResult(TraceSearchResultResponse),

    // -------------------------------------------------------------------------
    // Metrics responses
    // -------------------------------------------------------------------------
    /// Metric ingest result.
    MetricIngestResult(MetricIngestResultResponse),
    /// Metric list result.
    MetricListResult(MetricListResultResponse),
    /// Metric query result.
    MetricQueryResult(MetricQueryResultResponse),

    // -------------------------------------------------------------------------
    // Alert responses
    // -------------------------------------------------------------------------
    /// Alert create/update result.
    AlertCreateResult(AlertRuleResultResponse),
    /// Alert delete result.
    AlertDeleteResult(AlertRuleResultResponse),
    /// Alert list result.
    AlertListResult(AlertListResultResponse),
    /// Alert get result.
    AlertGetResult(AlertGetResultResponse),
    /// Alert evaluate result.
    AlertEvaluateResult(AlertEvaluateResultResponse),

    // -------------------------------------------------------------------------
    // Index responses
    // -------------------------------------------------------------------------
    /// Index create result.
    IndexCreateResult(IndexCreateResultResponse),
    /// Index drop result.
    IndexDropResult(IndexDropResultResponse),
    /// Index scan result.
    IndexScanResult(IndexScanResultResponse),
    /// Index list result.
    IndexListResult(IndexListResultResponse),

    /// Capability unavailable response.
    ///
    /// Returned when a request requires an app that is not loaded on this cluster.
    /// Contains the required app name and hints about clusters that can serve it.
    CapabilityUnavailable(CapabilityUnavailableResponse),

    /// Plugin reload result.
    ///
    /// NOTE: This variant MUST stay before the feature-gated section. If it were
    /// after `#[cfg(feature)]` variants, its postcard discriminant would shift
    /// when features are toggled, breaking wire compatibility.
    PluginReloadResult(PluginReloadResultResponse),

    // =========================================================================
    // FEATURE-GATED VARIANTS (must be at end for postcard discriminant stability)
    //
    // NO non-gated variants may appear after this line. The test
    // `test_no_ungated_variants_after_feature_gated_section` enforces this.
    // =========================================================================

    // -------------------------------------------------------------------------
    // Automerge responses
    // -------------------------------------------------------------------------
    /// Automerge create document result.
    #[cfg(feature = "automerge")]
    AutomergeCreateResult(AutomergeCreateResultResponse),

    /// Automerge get document result.
    #[cfg(feature = "automerge")]
    AutomergeGetResult(AutomergeGetResultResponse),

    /// Automerge save document result.
    #[cfg(feature = "automerge")]
    AutomergeSaveResult(AutomergeSaveResultResponse),

    /// Automerge delete document result.
    #[cfg(feature = "automerge")]
    AutomergeDeleteResult(AutomergeDeleteResultResponse),

    /// Automerge apply changes result.
    #[cfg(feature = "automerge")]
    AutomergeApplyChangesResult(AutomergeApplyChangesResultResponse),

    /// Automerge merge documents result.
    #[cfg(feature = "automerge")]
    AutomergeMergeResult(AutomergeMergeResultResponse),

    /// Automerge list documents result.
    #[cfg(feature = "automerge")]
    AutomergeListResult(AutomergeListResultResponse),

    /// Automerge get metadata result.
    #[cfg(feature = "automerge")]
    AutomergeGetMetadataResult(AutomergeGetMetadataResultResponse),

    /// Automerge exists check result.
    #[cfg(feature = "automerge")]
    AutomergeExistsResult(AutomergeExistsResultResponse),

    /// Automerge generate sync message result.
    #[cfg(feature = "automerge")]
    AutomergeGenerateSyncMessageResult(AutomergeGenerateSyncMessageResultResponse),

    /// Automerge receive sync message result.
    #[cfg(feature = "automerge")]
    AutomergeReceiveSyncMessageResult(AutomergeReceiveSyncMessageResultResponse),
}

/// Result of a plugin reload operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginReloadResultResponse {
    /// Whether the reload succeeded.
    pub is_success: bool,
    /// Number of plugins loaded after reload.
    pub plugin_count: u32,
    /// Error message if reload failed.
    pub error: Option<String>,
    /// Human-readable message.
    pub message: String,
}

impl ClientRpcResponse {
    /// Create an error response.
    pub fn error(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Error(ErrorResponse {
            code: code.into(),
            message: message.into(),
        })
    }
}

//! Client RPC protocol for Aspen communication over Iroh.
//!
//! This module defines the RPC protocol used by clients (including aspen-tui) to
//! communicate with aspen-node over Iroh P2P connections. It is separate from the
//! Raft RPC protocol which is used for cluster-internal consensus communication.
//!
//! # Architecture
//!
//! The Client RPC uses a distinct ALPN (`aspen-tui`) to distinguish it from Raft RPC.
//! This allows clients to connect directly to nodes without needing HTTP.
//!
//! # Tiger Style
//!
//! - Explicit request/response pairs
//! - Bounded message sizes
//! - Fail-fast on invalid requests

use serde::{Deserialize, Serialize};

// Re-export ALPN constant from canonical location
pub use crate::protocol_handlers::CLIENT_ALPN;

/// Maximum Client RPC message size (1 MB).
///
/// Tiger Style: Bounded to prevent memory exhaustion attacks.
pub const MAX_CLIENT_MESSAGE_SIZE: usize = 1024 * 1024;

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
    ReadKey { key: String },

    /// Write a key-value pair to the store.
    WriteKey { key: String, value: Vec<u8> },

    /// Compare-and-swap: atomically update value if current value matches expected.
    ///
    /// - `expected: None` means the key must NOT exist (create-if-absent)
    /// - `expected: Some(val)` means the key must exist with exactly that value
    CompareAndSwapKey {
        key: String,
        expected: Option<Vec<u8>>,
        new_value: Vec<u8>,
    },

    /// Compare-and-delete: atomically delete key if current value matches expected.
    CompareAndDeleteKey { key: String, expected: Vec<u8> },

    /// Trigger a snapshot.
    TriggerSnapshot,

    /// Add a learner node to the cluster.
    AddLearner { node_id: u64, addr: String },

    /// Change cluster membership.
    ChangeMembership { members: Vec<u64> },

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
    DeleteKey { key: String },

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
        force: bool,
    },

    /// Manually checkpoint SQLite WAL file.
    CheckpointWal,

    /// List all vaults (key namespaces).
    ListVaults,

    /// Get keys in a specific vault.
    GetVaultKeys { vault_name: String },

    /// Add a peer to the network factory.
    AddPeer {
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
        enabled: bool,
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
        /// Maximum bucket capacity.
        capacity: u64,
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
        /// Maximum bucket capacity.
        capacity: u64,
        /// Token refill rate per second.
        refill_rate: f64,
        /// Timeout in milliseconds.
        timeout_ms: u64,
    },

    /// Check available tokens in a rate limiter without consuming.
    RateLimiterAvailable {
        /// Rate limiter key.
        key: String,
        /// Maximum bucket capacity.
        capacity: u64,
        /// Token refill rate per second.
        refill_rate: f64,
    },

    /// Reset a rate limiter to full capacity.
    RateLimiterReset {
        /// Rate limiter key.
        key: String,
        /// Maximum bucket capacity.
        capacity: u64,
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
}

/// Client RPC response protocol.
///
/// Response types matching the request variants.
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
}

/// Health status response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    /// Overall status: "healthy", "degraded", or "unhealthy".
    pub status: String,
    /// Node identifier.
    pub node_id: u64,
    /// Raft node ID (may differ from node_id).
    pub raft_node_id: Option<u64>,
    /// Uptime in seconds.
    pub uptime_seconds: u64,
}

/// Raft metrics response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaftMetricsResponse {
    /// Node identifier.
    pub node_id: u64,
    /// Current Raft state (Leader, Follower, Candidate).
    pub state: String,
    /// Current leader node ID, if known.
    pub current_leader: Option<u64>,
    /// Current Raft term.
    pub current_term: u64,
    /// Last log index.
    pub last_log_index: Option<u64>,
    /// Last applied log index.
    pub last_applied_index: Option<u64>,
    /// Snapshot log index.
    pub snapshot_index: Option<u64>,
}

/// Node information response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInfoResponse {
    /// Node identifier.
    pub node_id: u64,
    /// Iroh endpoint address (serialized).
    pub endpoint_addr: String,
}

/// Cluster ticket response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterTicketResponse {
    /// Serialized cluster ticket.
    pub ticket: String,
    /// Gossip topic ID (debug format).
    pub topic_id: String,
    /// Cluster identifier (from cookie).
    pub cluster_id: String,
    /// This node's endpoint ID.
    pub endpoint_id: String,
    /// Number of bootstrap peers in ticket (for combined tickets).
    pub bootstrap_peers: Option<usize>,
}

/// Init cluster result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitResultResponse {
    /// Whether initialization succeeded.
    pub success: bool,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Read key result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadResultResponse {
    /// The value if found.
    pub value: Option<Vec<u8>>,
    /// Whether the key was found.
    pub found: bool,
    /// Optional error message when read fails (e.g., not leader).
    pub error: Option<String>,
}

/// Write key result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WriteResultResponse {
    /// Whether write succeeded.
    pub success: bool,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Compare-and-swap result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompareAndSwapResultResponse {
    /// Whether the CAS operation succeeded.
    ///
    /// True if the condition matched and the value was updated/deleted.
    /// False if the condition did not match.
    pub success: bool,
    /// The actual value of the key when CAS failed.
    ///
    /// This allows clients to retry with the correct expected value.
    /// None means the key did not exist.
    pub actual_value: Option<Vec<u8>>,
    /// Error message if operation failed due to internal error (not CAS condition).
    pub error: Option<String>,
}

/// Snapshot trigger result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotResultResponse {
    /// Whether snapshot was triggered.
    pub success: bool,
    /// Snapshot log index if successful.
    pub snapshot_index: Option<u64>,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Add learner result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddLearnerResultResponse {
    /// Whether adding learner succeeded.
    pub success: bool,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Change membership result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeMembershipResultResponse {
    /// Whether membership change succeeded.
    pub success: bool,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Error response for failed requests.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    /// Error code.
    pub code: String,
    /// Error message.
    pub message: String,
}

/// Cluster state response containing all known nodes.
///
/// Tiger Style: Bounded to MAX_CLUSTER_NODES (16) to prevent unbounded growth.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterStateResponse {
    /// All known nodes in the cluster.
    pub nodes: Vec<NodeDescriptor>,
    /// Current leader node ID, if known.
    pub leader_id: Option<u64>,
    /// This node's ID.
    pub this_node_id: u64,
}

/// Descriptor for a node in the cluster.
///
/// Contains all information needed to connect to and identify a node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeDescriptor {
    /// Node identifier.
    pub node_id: u64,
    /// Iroh endpoint address (serialized).
    pub endpoint_addr: String,
    /// Whether this node is a voter in Raft consensus.
    pub is_voter: bool,
    /// Whether this node is a learner (non-voting replica).
    pub is_learner: bool,
    /// Whether this node is the current leader.
    pub is_leader: bool,
}

/// Maximum number of nodes in cluster state response.
///
/// Tiger Style: Bounded limit to prevent memory exhaustion.
pub const MAX_CLUSTER_NODES: usize = 16;

// =============================================================================
// New response types (migrated from HTTP API)
// =============================================================================

/// Delete key result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteResultResponse {
    /// The key that was targeted.
    pub key: String,
    /// True if key existed and was deleted, false if not found.
    pub deleted: bool,
    /// Error message if operation failed.
    pub error: Option<String>,
}

/// Scan keys result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanResultResponse {
    /// Matching key-value pairs.
    pub entries: Vec<ScanEntry>,
    /// Number of entries returned.
    pub count: u32,
    /// True if more results available.
    pub is_truncated: bool,
    /// Token for next page (if truncated).
    pub continuation_token: Option<String>,
    /// Error message if operation failed.
    pub error: Option<String>,
}

/// Single entry from scan operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanEntry {
    /// Key name.
    pub key: String,
    /// Value (as UTF-8 string).
    pub value: String,
}

/// Prometheus metrics response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsResponse {
    /// Prometheus text format metrics.
    pub prometheus_text: String,
}

/// Promote learner result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromoteLearnerResultResponse {
    /// Whether promotion succeeded.
    pub success: bool,
    /// ID of promoted learner.
    pub learner_id: u64,
    /// Voters before the change.
    pub previous_voters: Vec<u64>,
    /// Voters after the change.
    pub new_voters: Vec<u64>,
    /// Status message.
    pub message: String,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Checkpoint WAL result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointWalResultResponse {
    /// Whether checkpoint succeeded.
    pub success: bool,
    /// Number of pages checkpointed.
    pub pages_checkpointed: Option<u32>,
    /// WAL file size before checkpoint (bytes).
    pub wal_size_before_bytes: Option<u64>,
    /// WAL file size after checkpoint (bytes).
    pub wal_size_after_bytes: Option<u64>,
    /// Error message if failed.
    pub error: Option<String>,
}

/// List vaults response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultListResponse {
    /// All vaults.
    pub vaults: Vec<VaultInfo>,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Information about a vault.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultInfo {
    /// Vault name.
    pub name: String,
    /// Number of keys in vault.
    pub key_count: u64,
}

/// Vault keys response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultKeysResponse {
    /// Vault name.
    pub vault: String,
    /// Keys in the vault.
    pub keys: Vec<VaultKeyValue>,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Key-value pair within a vault.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultKeyValue {
    /// Key name (without vault prefix).
    pub key: String,
    /// Value.
    pub value: String,
}

/// Add peer result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddPeerResultResponse {
    /// Whether add peer succeeded.
    pub success: bool,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Client ticket response for overlay subscription.
///
/// Used by clients to connect to a cluster as part of a priority-based
/// overlay system (similar to Nix binary cache substituters).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientTicketResponse {
    /// Serialized AspenClientTicket.
    pub ticket: String,
    /// Cluster identifier.
    pub cluster_id: String,
    /// Access level: "read" or "write".
    pub access: String,
    /// Priority level (0 = highest).
    pub priority: u32,
    /// This node's endpoint ID.
    pub endpoint_id: String,
    /// Error message if generation failed.
    pub error: Option<String>,
}

/// Docs ticket response for iroh-docs subscription.
///
/// Used by clients to subscribe to a cluster's iroh-docs namespace
/// for real-time state synchronization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocsTicketResponse {
    /// Serialized AspenDocsTicket.
    pub ticket: String,
    /// Cluster identifier.
    pub cluster_id: String,
    /// Namespace ID (derived from cluster cookie).
    pub namespace_id: String,
    /// Whether client has write access.
    pub read_write: bool,
    /// Priority level for this subscription.
    pub priority: u8,
    /// This node's endpoint ID.
    pub endpoint_id: String,
    /// Error message if generation failed.
    pub error: Option<String>,
}

// =============================================================================
// Blob operation response types
// =============================================================================

/// Add blob result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddBlobResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// BLAKE3 hash of the stored blob (hex-encoded).
    pub hash: Option<String>,
    /// Size of the blob in bytes.
    pub size: Option<u64>,
    /// Whether the blob was new (not already in store).
    pub was_new: Option<bool>,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Get blob result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetBlobResultResponse {
    /// Whether the blob was found.
    pub found: bool,
    /// Blob data if found.
    pub data: Option<Vec<u8>>,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Has blob result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HasBlobResultResponse {
    /// Whether the blob exists in the store.
    pub exists: bool,
    /// Error message if check failed.
    pub error: Option<String>,
}

/// Get blob ticket result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetBlobTicketResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// Serialized BlobTicket.
    pub ticket: Option<String>,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Blob list entry for listing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlobListEntry {
    /// BLAKE3 hash (hex-encoded).
    pub hash: String,
    /// Size in bytes.
    pub size: u64,
}

/// List blobs result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListBlobsResultResponse {
    /// List of blobs.
    pub blobs: Vec<BlobListEntry>,
    /// Total count returned.
    pub count: u32,
    /// Whether more blobs are available.
    pub has_more: bool,
    /// Continuation token for next page.
    pub continuation_token: Option<String>,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Protect blob result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtectBlobResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Unprotect blob result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnprotectBlobResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// Error message if failed.
    pub error: Option<String>,
}

// =============================================================================
// Peer cluster operation response types
// =============================================================================

/// Add peer cluster result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddPeerClusterResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// Cluster ID of the added peer.
    pub cluster_id: Option<String>,
    /// Priority assigned to this peer.
    pub priority: Option<u32>,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Remove peer cluster result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemovePeerClusterResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// Cluster ID of the removed peer.
    pub cluster_id: String,
    /// Error message if failed.
    pub error: Option<String>,
}

/// List peer clusters result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListPeerClustersResultResponse {
    /// List of peer cluster information.
    pub peers: Vec<PeerClusterInfo>,
    /// Total number of peer clusters.
    pub count: u32,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Information about a peer cluster subscription.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerClusterInfo {
    /// Cluster ID of the peer.
    pub cluster_id: String,
    /// Human-readable name of the peer.
    pub name: String,
    /// Connection state: "disconnected", "connecting", "connected", "failed".
    pub state: String,
    /// Priority for conflict resolution (0 = highest).
    pub priority: u32,
    /// Whether sync is enabled.
    pub enabled: bool,
    /// Number of completed sync sessions.
    pub sync_count: u64,
    /// Number of connection failures.
    pub failure_count: u64,
}

/// Peer cluster status response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerClusterStatusResponse {
    /// Whether the peer was found.
    pub found: bool,
    /// Cluster ID of the peer.
    pub cluster_id: String,
    /// Connection state: "disconnected", "connecting", "connected", "failed".
    pub state: String,
    /// Whether sync is currently in progress.
    pub syncing: bool,
    /// Entries received in current/last sync.
    pub entries_received: u64,
    /// Entries imported in current/last sync.
    pub entries_imported: u64,
    /// Entries skipped due to priority.
    pub entries_skipped: u64,
    /// Entries skipped due to filter.
    pub entries_filtered: u64,
    /// Error message if lookup failed.
    pub error: Option<String>,
}

/// Update peer cluster filter result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdatePeerClusterFilterResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// Cluster ID of the peer.
    pub cluster_id: String,
    /// New filter type: "full", "include", or "exclude".
    pub filter_type: Option<String>,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Update peer cluster priority result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdatePeerClusterPriorityResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// Cluster ID of the peer.
    pub cluster_id: String,
    /// Previous priority value.
    pub previous_priority: Option<u32>,
    /// New priority value.
    pub new_priority: Option<u32>,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Set peer cluster enabled result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetPeerClusterEnabledResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// Cluster ID of the peer.
    pub cluster_id: String,
    /// Whether the peer is now enabled.
    pub enabled: Option<bool>,
    /// Error message if failed.
    pub error: Option<String>,
}

// =============================================================================
// Coordination primitive response types
// =============================================================================

/// Lock operation result response.
///
/// Used for distributed lock acquire, try_acquire, release, and renew operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockResultResponse {
    /// Whether the lock operation succeeded.
    pub success: bool,
    /// Fencing token for the lock (if acquired).
    pub fencing_token: Option<u64>,
    /// Current holder ID (useful when lock is already held).
    pub holder_id: Option<String>,
    /// Lock deadline in Unix milliseconds (when lock expires).
    pub deadline_ms: Option<u64>,
    /// Error message if operation failed.
    pub error: Option<String>,
}

/// Counter operation result response.
///
/// Used for atomic counter get, increment, decrement, add, subtract, set operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CounterResultResponse {
    /// Whether the counter operation succeeded.
    pub success: bool,
    /// Current counter value after operation.
    pub value: Option<u64>,
    /// Error message if operation failed.
    pub error: Option<String>,
}

/// Signed counter operation result response.
///
/// Used for signed atomic counter operations that can go negative.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedCounterResultResponse {
    /// Whether the counter operation succeeded.
    pub success: bool,
    /// Current counter value after operation (can be negative).
    pub value: Option<i64>,
    /// Error message if operation failed.
    pub error: Option<String>,
}

/// Sequence generator result response.
///
/// Used for sequence next, reserve, and current operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SequenceResultResponse {
    /// Whether the sequence operation succeeded.
    pub success: bool,
    /// Sequence value (next ID or start of reserved range).
    pub value: Option<u64>,
    /// Error message if operation failed.
    pub error: Option<String>,
}

/// Rate limiter result response.
///
/// Used for rate limiter try_acquire, acquire, available, and reset operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimiterResultResponse {
    /// Whether the rate limit operation succeeded (tokens acquired).
    pub success: bool,
    /// Remaining tokens after operation.
    pub tokens_remaining: Option<u64>,
    /// Milliseconds to wait before retrying (when rate limited).
    pub retry_after_ms: Option<u64>,
    /// Error message if operation failed.
    pub error: Option<String>,
}

// =============================================================================
// SQL query response types
// =============================================================================

/// SQL query result response.
///
/// Contains the result of a read-only SQL query execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SqlResultResponse {
    /// Whether the query succeeded.
    pub success: bool,
    /// Column names (JSON array).
    pub columns: Option<Vec<String>>,
    /// Result rows (JSON array of arrays).
    /// Each inner array contains the values for one row.
    pub rows: Option<Vec<Vec<serde_json::Value>>>,
    /// Number of rows returned.
    pub row_count: Option<u32>,
    /// True if more rows exist but were not returned due to limit.
    pub is_truncated: Option<bool>,
    /// Query execution time in milliseconds.
    pub execution_time_ms: Option<u64>,
    /// Error message if query failed.
    pub error: Option<String>,
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

// =============================================================================
// Batch operation types
// =============================================================================

/// A single operation within a batch write.
///
/// Supports Set and Delete operations that can be mixed freely.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum BatchWriteOperation {
    /// Set a key to a value.
    Set {
        /// Key to set.
        key: String,
        /// Value to set (as bytes for RPC transport).
        value: Vec<u8>,
    },
    /// Delete a key.
    Delete {
        /// Key to delete.
        key: String,
    },
}

/// A condition for conditional batch writes.
///
/// All conditions must be satisfied for the batch to execute.
/// Similar to etcd's transaction compare operations.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum BatchCondition {
    /// Key must have exactly this value.
    ValueEquals {
        /// Key to check.
        key: String,
        /// Expected value (as bytes).
        expected: Vec<u8>,
    },
    /// Key must exist (any value).
    KeyExists {
        /// Key to check.
        key: String,
    },
    /// Key must not exist.
    KeyNotExists {
        /// Key to check.
        key: String,
    },
}

/// Batch read result response.
///
/// Contains values for all requested keys in order.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchReadResultResponse {
    /// Whether the batch read succeeded.
    pub success: bool,
    /// Values for each key in request order.
    /// None for keys that don't exist.
    pub values: Option<Vec<Option<Vec<u8>>>>,
    /// Error message if operation failed.
    pub error: Option<String>,
}

/// Batch write result response.
///
/// Reports success/failure for the entire atomic batch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchWriteResultResponse {
    /// Whether the batch write succeeded.
    pub success: bool,
    /// Number of operations applied (all or none).
    pub operations_applied: Option<u32>,
    /// Error message if operation failed.
    pub error: Option<String>,
}

/// Conditional batch write result response.
///
/// Reports whether conditions passed and operations were applied.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConditionalBatchWriteResultResponse {
    /// Whether the batch executed (all conditions passed).
    pub success: bool,
    /// Whether all conditions were satisfied.
    pub conditions_met: bool,
    /// Number of operations applied (0 if conditions failed).
    pub operations_applied: Option<u32>,
    /// Index of first failed condition (if any).
    pub failed_condition_index: Option<u32>,
    /// Details about why condition failed (e.g., actual value).
    pub failed_condition_reason: Option<String>,
    /// Error message if operation failed due to error (not condition).
    pub error: Option<String>,
}

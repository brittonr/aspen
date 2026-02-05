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

use serde::Deserialize;
use serde::Serialize;

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
pub const DEFAULT_GIT_CHUNK_SIZE: usize = 1024 * 1024;

/// Maximum chunk size for git bridge chunked transfers (4 MB).
///
/// Tiger Style: Upper bound to prevent abuse while supporting large objects.
pub const MAX_GIT_CHUNK_SIZE: usize = 4 * 1024 * 1024;

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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticatedRequest {
    /// The actual RPC request.
    pub request: ClientRpcRequest,
    /// Capability token for authorization (optional during migration).
    pub token: Option<aspen_auth::CapabilityToken>,
}

impl AuthenticatedRequest {
    /// Create an authenticated request with a token.
    pub fn new(request: ClientRpcRequest, token: aspen_auth::CapabilityToken) -> Self {
        Self {
            request,
            token: Some(token),
        }
    }

    /// Create an unauthenticated request (legacy compatibility).
    pub fn unauthenticated(request: ClientRpcRequest) -> Self {
        Self { request, token: None }
    }
}

impl From<ClientRpcRequest> for AuthenticatedRequest {
    fn from(request: ClientRpcRequest) -> Self {
        Self::unauthenticated(request)
    }
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
        force: bool,
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
        force: bool,
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
        size: u64,
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
        enabled: bool,
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
        include_prev_value: bool,
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
        include_keys: bool,
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
        /// Maximum permits (semaphore capacity).
        capacity: u32,
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
        /// Maximum permits (semaphore capacity).
        capacity: u32,
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
    // DNS operations - Record and zone management
    // =========================================================================
    /// Set a DNS record.
    ///
    /// Creates or updates a DNS record. The record data is JSON-encoded.
    DnsSetRecord {
        /// Domain name (e.g., "api.example.com").
        domain: String,
        /// Record type (A, AAAA, CNAME, MX, TXT, SRV, NS, SOA, PTR, CAA).
        record_type: String,
        /// TTL in seconds.
        ttl_seconds: u32,
        /// JSON-encoded record data (format depends on record type).
        data_json: String,
    },

    /// Get a DNS record.
    ///
    /// Returns the record for the specified domain and type.
    DnsGetRecord {
        /// Domain name.
        domain: String,
        /// Record type (A, AAAA, CNAME, MX, TXT, SRV, NS, SOA, PTR, CAA).
        record_type: String,
    },

    /// Get all DNS records for a domain.
    ///
    /// Returns all record types for the specified domain.
    DnsGetRecords {
        /// Domain name.
        domain: String,
    },

    /// Delete a DNS record.
    ///
    /// Removes the record for the specified domain and type.
    DnsDeleteRecord {
        /// Domain name.
        domain: String,
        /// Record type (A, AAAA, CNAME, MX, TXT, SRV, NS, SOA, PTR, CAA).
        record_type: String,
    },

    /// Resolve a domain (with wildcard matching).
    ///
    /// Looks up DNS records using wildcard fallback.
    DnsResolve {
        /// Domain name to resolve.
        domain: String,
        /// Record type (A, AAAA, CNAME, MX, TXT, SRV, NS, SOA, PTR, CAA).
        record_type: String,
    },

    /// Scan DNS records by prefix.
    ///
    /// Returns all records matching the domain prefix.
    DnsScanRecords {
        /// Domain prefix to match (empty matches all).
        prefix: String,
        /// Maximum results to return.
        limit: u32,
    },

    /// Create or update a DNS zone.
    ///
    /// Zones provide organizational grouping for records.
    DnsSetZone {
        /// Zone name (e.g., "example.com").
        name: String,
        /// Whether the zone is enabled.
        enabled: bool,
        /// Default TTL for records in this zone.
        default_ttl: u32,
        /// Optional description.
        description: Option<String>,
    },

    /// Get a DNS zone.
    DnsGetZone {
        /// Zone name.
        name: String,
    },

    /// List all DNS zones.
    DnsListZones,

    /// Delete a DNS zone.
    DnsDeleteZone {
        /// Zone name.
        name: String,
        /// Whether to also delete all records in the zone.
        delete_records: bool,
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
        /// Worker capacity (concurrent jobs).
        capacity: u32,
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
    // KV v2 Secrets Engine
    /// Read a secret from the KV v2 secrets engine.
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

    /// Write a secret to the KV v2 secrets engine.
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

    /// Soft-delete secret versions from KV v2.
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

    /// Permanently destroy secret versions from KV v2.
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
    // Pijul operations - Patch-based version control
    // -------------------------------------------------------------------------
    /// Initialize a new Pijul repository.
    #[cfg(feature = "pijul")]
    PijulRepoInit {
        /// Repository name.
        name: String,
        /// Optional description.
        description: Option<String>,
        /// Default channel name.
        default_channel: String,
    },

    /// List Pijul repositories.
    #[cfg(feature = "pijul")]
    PijulRepoList {
        /// Maximum results.
        limit: u32,
    },

    /// Get Pijul repository info.
    #[cfg(feature = "pijul")]
    PijulRepoInfo {
        /// Repository ID (hex-encoded).
        repo_id: String,
    },

    /// List channels in a Pijul repository.
    #[cfg(feature = "pijul")]
    PijulChannelList {
        /// Repository ID.
        repo_id: String,
    },

    /// Create a new channel.
    #[cfg(feature = "pijul")]
    PijulChannelCreate {
        /// Repository ID.
        repo_id: String,
        /// Channel name.
        name: String,
    },

    /// Delete a channel.
    #[cfg(feature = "pijul")]
    PijulChannelDelete {
        /// Repository ID.
        repo_id: String,
        /// Channel name.
        name: String,
    },

    /// Fork a channel.
    #[cfg(feature = "pijul")]
    PijulChannelFork {
        /// Repository ID.
        repo_id: String,
        /// Source channel.
        source: String,
        /// Target channel name.
        target: String,
    },

    /// Get channel info.
    #[cfg(feature = "pijul")]
    PijulChannelInfo {
        /// Repository ID.
        repo_id: String,
        /// Channel name.
        name: String,
    },

    /// Record changes from working directory.
    #[cfg(feature = "pijul")]
    PijulRecord {
        /// Repository ID.
        repo_id: String,
        /// Channel name.
        channel: String,
        /// Working directory path.
        working_dir: String,
        /// Change message.
        message: String,
        /// Author name.
        author_name: Option<String>,
        /// Author email.
        author_email: Option<String>,
    },

    /// Apply a change to a channel.
    #[cfg(feature = "pijul")]
    PijulApply {
        /// Repository ID.
        repo_id: String,
        /// Channel name.
        channel: String,
        /// Change hash (hex-encoded BLAKE3).
        change_hash: String,
    },

    /// Unrecord (remove) a change from a channel.
    #[cfg(feature = "pijul")]
    PijulUnrecord {
        /// Repository ID.
        repo_id: String,
        /// Channel name.
        channel: String,
        /// Change hash (hex-encoded BLAKE3).
        change_hash: String,
    },

    /// Get change log for a channel.
    #[cfg(feature = "pijul")]
    PijulLog {
        /// Repository ID.
        repo_id: String,
        /// Channel name.
        channel: String,
        /// Maximum entries.
        limit: u32,
    },

    /// Checkout pristine state to working directory.
    #[cfg(feature = "pijul")]
    PijulCheckout {
        /// Repository ID.
        repo_id: String,
        /// Channel name.
        channel: String,
        /// Output directory path.
        output_dir: String,
    },

    /// Show details of a specific change.
    #[cfg(feature = "pijul")]
    PijulShow {
        /// Repository ID.
        repo_id: String,
        /// Change hash (full or partial, hex-encoded).
        change_hash: String,
    },

    /// Get blame/attribution for a file.
    ///
    /// Returns a list of changes that have contributed to the current
    /// state of the specified file.
    #[cfg(feature = "pijul")]
    PijulBlame {
        /// Repository ID.
        repo_id: String,
        /// Channel name.
        channel: String,
        /// File path to get blame for.
        path: String,
    },

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
        success: bool,
        /// Error message if job failed.
        error_message: Option<String>,
        /// Job output data (logs, artifacts, etc.).
        output_data: Option<Vec<u8>>,
        /// Processing time in milliseconds.
        processing_time_ms: u64,
    },
}

impl ClientRpcRequest {
    /// Convert the request to an authorization operation.
    ///
    /// Returns None for operations that don't require authorization.
    pub fn to_operation(&self) -> Option<aspen_auth::Operation> {
        use aspen_auth::Operation;

        match self {
            // Cluster operations
            Self::InitCluster
            | Self::AddLearner { .. }
            | Self::ChangeMembership { .. }
            | Self::TriggerSnapshot
            | Self::PromoteLearner { .. }
            | Self::AddPeer { .. }
            | Self::CheckpointWal => Some(Operation::ClusterAdmin {
                action: "cluster_operation".to_string(),
            }),

            // Read-only operations
            Self::Ping
            | Self::GetHealth
            | Self::GetNodeInfo
            | Self::GetRaftMetrics
            | Self::GetLeader
            | Self::GetClusterTicket
            | Self::GetClusterState
            | Self::GetClusterTicketCombined { .. }
            | Self::GetMetrics
            | Self::ListVaults
            | Self::GetFederationStatus
            | Self::ListDiscoveredClusters
            | Self::GetDiscoveredCluster { .. }
            | Self::ListFederatedRepositories => None,

            // Key-value read operations
            Self::ReadKey { key } | Self::ScanKeys { prefix: key, .. } | Self::GetVaultKeys { vault_name: key } => {
                Some(Operation::Read { key: key.clone() })
            }

            // Key-value write operations
            Self::WriteKey { key, value } | Self::WriteKeyWithLease { key, value, .. } => Some(Operation::Write {
                key: key.clone(),
                value: value.clone(),
            }),
            Self::DeleteKey { key } | Self::CompareAndSwapKey { key, .. } | Self::CompareAndDeleteKey { key, .. } => {
                Some(Operation::Write {
                    key: key.clone(),
                    value: vec![],
                })
            }

            // Batch operations
            Self::BatchRead { keys } => keys.first().map(|key| Operation::Read { key: key.clone() }),
            Self::BatchWrite { operations } | Self::ConditionalBatchWrite { operations, .. } => {
                operations.first().map(|op| match op {
                    BatchWriteOperation::Set { key, value } => Operation::Write {
                        key: key.clone(),
                        value: value.clone(),
                    },
                    BatchWriteOperation::Delete { key } => Operation::Write {
                        key: key.clone(),
                        value: vec![],
                    },
                })
            }

            // Job operations
            Self::JobSubmit { .. }
            | Self::JobCancel { .. }
            | Self::JobUpdateProgress { .. }
            | Self::WorkerRegister { .. }
            | Self::WorkerHeartbeat { .. }
            | Self::WorkerDeregister { .. } => Some(Operation::Write {
                key: "_jobs:".to_string(),
                value: vec![],
            }),
            Self::JobGet { .. } | Self::JobList { .. } | Self::JobQueueStats | Self::WorkerStatus => {
                Some(Operation::Read {
                    key: "_jobs:".to_string(),
                })
            }

            // Blob operations
            Self::AddBlob { .. }
            | Self::ProtectBlob { .. }
            | Self::UnprotectBlob { .. }
            | Self::DeleteBlob { .. }
            | Self::DownloadBlob { .. }
            | Self::DownloadBlobByHash { .. }
            | Self::DownloadBlobByProvider { .. } => Some(Operation::Write {
                key: "_blob:".to_string(),
                value: vec![],
            }),
            Self::GetBlob { hash }
            | Self::HasBlob { hash }
            | Self::GetBlobTicket { hash }
            | Self::GetBlobStatus { hash }
            | Self::GetBlobReplicationStatus { hash } => Some(Operation::Read {
                key: format!("_blob:{hash}"),
            }),
            Self::ListBlobs { .. } => Some(Operation::Read {
                key: "_blob:".to_string(),
            }),
            // Blob replication operations (cluster-internal)
            Self::BlobReplicatePull { hash, .. } => Some(Operation::Write {
                key: format!("_blob:replica:{hash}"),
                value: vec![],
            }),
            Self::TriggerBlobReplication { hash, .. } => Some(Operation::Write {
                key: format!("_blob:replica:{hash}"),
                value: vec![],
            }),
            // Blob repair cycle is a cluster admin operation
            Self::RunBlobRepairCycle => Some(Operation::ClusterAdmin {
                action: "blob_repair_cycle".to_string(),
            }),

            // Docs operations
            Self::DocsSet { key, value } => Some(Operation::Write {
                key: format!("_docs:{key}"),
                value: value.clone(),
            }),
            Self::DocsGet { key } | Self::DocsDelete { key } => Some(Operation::Read {
                key: format!("_docs:{key}"),
            }),
            Self::DocsList { .. } | Self::DocsStatus => Some(Operation::Read {
                key: "_docs:".to_string(),
            }),

            // Peer cluster operations
            Self::AddPeerCluster { .. }
            | Self::RemovePeerCluster { .. }
            | Self::UpdatePeerClusterFilter { .. }
            | Self::UpdatePeerClusterPriority { .. }
            | Self::SetPeerClusterEnabled { .. } => Some(Operation::ClusterAdmin {
                action: "peer_cluster_operation".to_string(),
            }),
            Self::ListPeerClusters
            | Self::GetPeerClusterStatus { .. }
            | Self::GetKeyOrigin { .. }
            | Self::GetClientTicket { .. }
            | Self::GetDocsTicket { .. } => None,

            // SQL queries
            Self::ExecuteSql { .. } => Some(Operation::Read {
                key: "_sql:".to_string(),
            }),

            // Lock operations
            Self::LockAcquire { key, .. }
            | Self::LockTryAcquire { key, .. }
            | Self::LockRelease { key, .. }
            | Self::LockRenew { key, .. } => Some(Operation::Write {
                key: format!("_lock:{key}"),
                value: vec![],
            }),

            // Counter operations
            Self::CounterGet { key } | Self::SignedCounterGet { key } | Self::SequenceCurrent { key } => {
                Some(Operation::Read {
                    key: format!("_counter:{key}"),
                })
            }
            Self::CounterIncrement { key }
            | Self::CounterDecrement { key }
            | Self::CounterAdd { key, .. }
            | Self::CounterSubtract { key, .. }
            | Self::CounterSet { key, .. }
            | Self::CounterCompareAndSet { key, .. }
            | Self::SignedCounterAdd { key, .. }
            | Self::SequenceNext { key }
            | Self::SequenceReserve { key, .. } => Some(Operation::Write {
                key: format!("_counter:{key}"),
                value: vec![],
            }),

            // Rate limiter operations
            Self::RateLimiterTryAcquire { key, .. }
            | Self::RateLimiterAcquire { key, .. }
            | Self::RateLimiterReset { key, .. } => Some(Operation::Write {
                key: format!("_ratelimit:{key}"),
                value: vec![],
            }),
            Self::RateLimiterAvailable { key, .. } => Some(Operation::Read {
                key: format!("_ratelimit:{key}"),
            }),

            // Watch operations
            Self::WatchCreate { prefix, .. } => Some(Operation::Read { key: prefix.clone() }),
            Self::WatchCancel { .. } | Self::WatchStatus { .. } => None,

            // Lease operations
            Self::LeaseGrant { .. } | Self::LeaseRevoke { .. } | Self::LeaseKeepalive { .. } => {
                Some(Operation::Write {
                    key: "_lease:".to_string(),
                    value: vec![],
                })
            }
            Self::LeaseTimeToLive { .. } | Self::LeaseList => Some(Operation::Read {
                key: "_lease:".to_string(),
            }),

            // Barrier operations
            Self::BarrierEnter { name, .. } | Self::BarrierLeave { name, .. } => Some(Operation::Write {
                key: format!("_barrier:{name}"),
                value: vec![],
            }),
            Self::BarrierStatus { name } => Some(Operation::Read {
                key: format!("_barrier:{name}"),
            }),

            // Semaphore operations
            Self::SemaphoreAcquire { name, .. }
            | Self::SemaphoreTryAcquire { name, .. }
            | Self::SemaphoreRelease { name, .. } => Some(Operation::Write {
                key: format!("_semaphore:{name}"),
                value: vec![],
            }),
            Self::SemaphoreStatus { name } => Some(Operation::Read {
                key: format!("_semaphore:{name}"),
            }),

            // RWLock operations
            Self::RWLockAcquireRead { name, .. }
            | Self::RWLockTryAcquireRead { name, .. }
            | Self::RWLockAcquireWrite { name, .. }
            | Self::RWLockTryAcquireWrite { name, .. }
            | Self::RWLockReleaseRead { name, .. }
            | Self::RWLockReleaseWrite { name, .. }
            | Self::RWLockDowngrade { name, .. } => Some(Operation::Write {
                key: format!("_rwlock:{name}"),
                value: vec![],
            }),
            Self::RWLockStatus { name } => Some(Operation::Read {
                key: format!("_rwlock:{name}"),
            }),

            // Queue operations
            Self::QueueCreate { queue_name, .. }
            | Self::QueueDelete { queue_name }
            | Self::QueueEnqueue { queue_name, .. }
            | Self::QueueEnqueueBatch { queue_name, .. }
            | Self::QueueDequeue { queue_name, .. }
            | Self::QueueDequeueWait { queue_name, .. }
            | Self::QueueAck { queue_name, .. }
            | Self::QueueNack { queue_name, .. }
            | Self::QueueExtendVisibility { queue_name, .. }
            | Self::QueueRedriveDLQ { queue_name, .. } => Some(Operation::Write {
                key: format!("_queue:{queue_name}"),
                value: vec![],
            }),
            Self::QueuePeek { queue_name, .. }
            | Self::QueueStatus { queue_name }
            | Self::QueueGetDLQ { queue_name, .. } => Some(Operation::Read {
                key: format!("_queue:{queue_name}"),
            }),

            // Service registry operations
            Self::ServiceRegister { service_name, .. }
            | Self::ServiceDeregister { service_name, .. }
            | Self::ServiceHeartbeat { service_name, .. }
            | Self::ServiceUpdateHealth { service_name, .. }
            | Self::ServiceUpdateMetadata { service_name, .. } => Some(Operation::Write {
                key: format!("_service:{service_name}"),
                value: vec![],
            }),
            Self::ServiceDiscover { service_name, .. } | Self::ServiceGetInstance { service_name, .. } => {
                Some(Operation::Read {
                    key: format!("_service:{service_name}"),
                })
            }
            Self::ServiceList { prefix, .. } => Some(Operation::Read {
                key: format!("_service:{prefix}"),
            }),

            // DNS operations
            Self::DnsSetRecord { domain, .. }
            | Self::DnsDeleteRecord { domain, .. }
            | Self::DnsSetZone { name: domain, .. }
            | Self::DnsDeleteZone { name: domain, .. } => Some(Operation::Write {
                key: format!("_dns:{domain}"),
                value: vec![],
            }),
            Self::DnsGetRecord { domain, .. }
            | Self::DnsGetRecords { domain }
            | Self::DnsResolve { domain, .. }
            | Self::DnsGetZone { name: domain } => Some(Operation::Read {
                key: format!("_dns:{domain}"),
            }),
            Self::DnsScanRecords { prefix, .. } => Some(Operation::Read {
                key: format!("_dns:{prefix}"),
            }),
            Self::DnsListZones => Some(Operation::Read {
                key: "_dns:".to_string(),
            }),

            // Sharding operations
            Self::GetTopology { .. } => None,

            // Forge operations
            Self::ForgeCreateRepo { .. }
            | Self::ForgeStoreBlob { .. }
            | Self::ForgeCreateTree { .. }
            | Self::ForgeCommit { .. }
            | Self::ForgeSetRef { .. }
            | Self::ForgeDeleteRef { .. }
            | Self::ForgeCasRef { .. }
            | Self::ForgeCreateIssue { .. }
            | Self::ForgeCommentIssue { .. }
            | Self::ForgeCloseIssue { .. }
            | Self::ForgeReopenIssue { .. }
            | Self::ForgeCreatePatch { .. }
            | Self::ForgeUpdatePatch { .. }
            | Self::ForgeApprovePatch { .. }
            | Self::ForgeMergePatch { .. }
            | Self::ForgeClosePatch { .. }
            | Self::TrustCluster { .. }
            | Self::UntrustCluster { .. }
            | Self::FederateRepository { .. }
            | Self::ForgeFetchFederated { .. }
            | Self::GitBridgePush { .. }
            | Self::GitBridgePushStart { .. }
            | Self::GitBridgePushChunk { .. }
            | Self::GitBridgePushComplete { .. } => Some(Operation::Write {
                key: "_forge:".to_string(),
                value: vec![],
            }),
            Self::ForgeGetRepo { .. }
            | Self::ForgeListRepos { .. }
            | Self::ForgeGetBlob { .. }
            | Self::ForgeGetTree { .. }
            | Self::ForgeGetCommit { .. }
            | Self::ForgeLog { .. }
            | Self::ForgeGetRef { .. }
            | Self::ForgeListBranches { .. }
            | Self::ForgeListTags { .. }
            | Self::ForgeListIssues { .. }
            | Self::ForgeGetIssue { .. }
            | Self::ForgeListPatches { .. }
            | Self::ForgeGetPatch { .. }
            | Self::ForgeGetDelegateKey { .. }
            | Self::GitBridgeListRefs { .. }
            | Self::GitBridgeFetch { .. } => Some(Operation::Read {
                key: "_forge:".to_string(),
            }),

            // Pijul operations
            #[cfg(feature = "pijul")]
            Self::PijulRepoInit { .. }
            | Self::PijulChannelCreate { .. }
            | Self::PijulChannelDelete { .. }
            | Self::PijulChannelFork { .. }
            | Self::PijulRecord { .. }
            | Self::PijulApply { .. }
            | Self::PijulUnrecord { .. } => Some(Operation::Write {
                key: "_pijul:".to_string(),
                value: vec![],
            }),
            #[cfg(feature = "pijul")]
            Self::PijulRepoList { .. }
            | Self::PijulRepoInfo { .. }
            | Self::PijulChannelList { .. }
            | Self::PijulChannelInfo { .. }
            | Self::PijulLog { .. }
            | Self::PijulCheckout { .. }
            | Self::PijulShow { .. }
            | Self::PijulBlame { .. } => Some(Operation::Read {
                key: "_pijul:".to_string(),
            }),

            // Automerge operations
            #[cfg(feature = "automerge")]
            Self::AutomergeCreate { .. }
            | Self::AutomergeSave { .. }
            | Self::AutomergeDelete { .. }
            | Self::AutomergeApplyChanges { .. }
            | Self::AutomergeMerge { .. }
            | Self::AutomergeReceiveSyncMessage { .. } => Some(Operation::Write {
                key: "_automerge:".to_string(),
                value: vec![],
            }),
            #[cfg(feature = "automerge")]
            Self::AutomergeGet { .. }
            | Self::AutomergeList { .. }
            | Self::AutomergeGetMetadata { .. }
            | Self::AutomergeExists { .. }
            | Self::AutomergeGenerateSyncMessage { .. } => Some(Operation::Read {
                key: "_automerge:".to_string(),
            }),

            // Hook operations (read-only metadata access)
            Self::HookList | Self::HookGetMetrics { .. } => Some(Operation::Read {
                key: "_hooks:".to_string(),
            }),
            Self::HookTrigger { .. } => Some(Operation::Write {
                key: "_hooks:".to_string(),
                value: vec![],
            }),

            // CI/CD operations
            Self::CiGetStatus { run_id } => Some(Operation::Read {
                key: format!("_ci:runs:{}", run_id),
            }),
            Self::CiListRuns { repo_id, .. } => Some(Operation::Read {
                key: format!("_ci:runs:{}", repo_id.as_deref().unwrap_or("")),
            }),
            Self::CiTriggerPipeline { repo_id, .. }
            | Self::CiWatchRepo { repo_id }
            | Self::CiUnwatchRepo { repo_id } => Some(Operation::Write {
                key: format!("_ci:repos:{}", repo_id),
                value: vec![],
            }),
            Self::CiCancelRun { run_id, .. } => Some(Operation::Write {
                key: format!("_ci:runs:{}", run_id),
                value: vec![],
            }),
            Self::CiListArtifacts { job_id, run_id } => Some(Operation::Read {
                key: format!("_ci:artifacts:{}:{}", job_id, run_id.as_deref().unwrap_or("")),
            }),
            Self::CiGetArtifact { blob_hash } => Some(Operation::Read {
                key: format!("_ci:artifacts:{}", blob_hash),
            }),
            Self::CiGetJobLogs { run_id, job_id, .. } => Some(Operation::Read {
                key: format!("_ci:logs:{}:{}", run_id, job_id),
            }),
            Self::CiSubscribeLogs { run_id, job_id, .. } => Some(Operation::Read {
                key: format!("_ci:logs:{}:{}", run_id, job_id),
            }),
            Self::CiGetJobOutput { run_id, job_id } => Some(Operation::Read {
                key: format!("_ci:runs:{}:{}", run_id, job_id),
            }),

            // Secrets KV v2 operations
            Self::SecretsKvRead { mount, path, .. }
            | Self::SecretsKvList { mount, path }
            | Self::SecretsKvMetadata { mount, path } => Some(Operation::Read {
                key: format!("_secrets:{}:{}", mount, path),
            }),
            Self::SecretsKvWrite { mount, path, .. }
            | Self::SecretsKvDelete { mount, path, .. }
            | Self::SecretsKvDestroy { mount, path, .. }
            | Self::SecretsKvUndelete { mount, path, .. }
            | Self::SecretsKvUpdateMetadata { mount, path, .. }
            | Self::SecretsKvDeleteMetadata { mount, path } => Some(Operation::Write {
                key: format!("_secrets:{}:{}", mount, path),
                value: vec![],
            }),

            // Secrets Transit operations
            Self::SecretsTransitListKeys { mount } => Some(Operation::Read {
                key: format!("_secrets:{}:", mount),
            }),
            Self::SecretsTransitCreateKey { mount, name, .. } | Self::SecretsTransitRotateKey { mount, name } => {
                Some(Operation::Write {
                    key: format!("_secrets:{}:{}", mount, name),
                    value: vec![],
                })
            }
            Self::SecretsTransitEncrypt { mount, name, .. }
            | Self::SecretsTransitDecrypt { mount, name, .. }
            | Self::SecretsTransitSign { mount, name, .. }
            | Self::SecretsTransitVerify { mount, name, .. }
            | Self::SecretsTransitRewrap { mount, name, .. }
            | Self::SecretsTransitDatakey { mount, name, .. } => Some(Operation::Read {
                key: format!("_secrets:{}:{}", mount, name),
            }),

            // Secrets PKI operations
            Self::SecretsPkiGetCrl { mount }
            | Self::SecretsPkiListCerts { mount }
            | Self::SecretsPkiListRoles { mount } => Some(Operation::Read {
                key: format!("_secrets:{}:", mount),
            }),
            Self::SecretsPkiGetRole { mount, name } => Some(Operation::Read {
                key: format!("_secrets:{}:{}", mount, name),
            }),
            Self::SecretsPkiGenerateRoot { mount, .. }
            | Self::SecretsPkiGenerateIntermediate { mount, .. }
            | Self::SecretsPkiSetSignedIntermediate { mount, .. }
            | Self::SecretsPkiCreateRole { mount, .. }
            | Self::SecretsPkiIssue { mount, .. }
            | Self::SecretsPkiRevoke { mount, .. } => Some(Operation::Write {
                key: format!("_secrets:{}:", mount),
                value: vec![],
            }),

            // Nix cache signing key operations
            Self::SecretsNixCacheGetPublicKey { mount, .. } => Some(Operation::Read {
                key: format!("_secrets:{}:", mount),
            }),
            Self::SecretsNixCacheListKeys { mount } => Some(Operation::Read {
                key: format!("_secrets:{}:", mount),
            }),
            Self::SecretsNixCacheCreateKey { mount, .. }
            | Self::SecretsNixCacheRotateKey { mount, .. }
            | Self::SecretsNixCacheDeleteKey { mount, .. } => Some(Operation::Write {
                key: format!("_secrets:{}:", mount),
                value: vec![],
            }),

            // Cache operations (read-only, no writes from client)
            Self::CacheQuery { store_hash } | Self::CacheDownload { store_hash } => Some(Operation::Read {
                key: format!("_cache:narinfo:{store_hash}"),
            }),
            Self::CacheStats => Some(Operation::Read {
                key: "_cache:stats".to_string(),
            }),

            // Cache migration operations (admin only)
            #[cfg(feature = "ci")]
            Self::CacheMigrationStart { .. } | Self::CacheMigrationCancel => Some(Operation::ClusterAdmin {
                action: "cache_migration".to_string(),
            }),
            #[cfg(feature = "ci")]
            Self::CacheMigrationStatus | Self::CacheMigrationValidate { .. } => Some(Operation::Read {
                key: "_cache:migration:".to_string(),
            }),

            // SNIX operations (for remote workers)
            Self::SnixDirectoryGet { digest } => Some(Operation::Read {
                key: format!("snix:dir:{digest}"),
            }),
            Self::SnixDirectoryPut { .. } => Some(Operation::Write {
                key: "snix:dir:".to_string(),
                value: vec![],
            }),
            Self::SnixPathInfoGet { digest } => Some(Operation::Read {
                key: format!("snix:pathinfo:{digest}"),
            }),
            Self::SnixPathInfoPut { .. } => Some(Operation::Write {
                key: "snix:pathinfo:".to_string(),
                value: vec![],
            }),

            // Worker job coordination operations
            Self::WorkerPollJobs { worker_id, .. } => Some(Operation::Read {
                key: format!("__worker:{worker_id}:jobs"),
            }),
            Self::WorkerCompleteJob { worker_id, .. } => Some(Operation::Write {
                key: format!("__worker:{worker_id}:complete"),
                value: vec![],
            }),
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
    // DNS responses
    // =========================================================================
    /// DNS set record result.
    DnsSetRecordResult(DnsRecordResultResponse),

    /// DNS get record result.
    DnsGetRecordResult(DnsRecordResultResponse),

    /// DNS get records result.
    DnsGetRecordsResult(DnsRecordsResultResponse),

    /// DNS delete record result.
    DnsDeleteRecordResult(DnsDeleteRecordResultResponse),

    /// DNS resolve result.
    DnsResolveResult(DnsRecordsResultResponse),

    /// DNS scan records result.
    DnsScanRecordsResult(DnsRecordsResultResponse),

    /// DNS set zone result.
    DnsSetZoneResult(DnsZoneResultResponse),

    /// DNS get zone result.
    DnsGetZoneResult(DnsZoneResultResponse),

    /// DNS list zones result.
    DnsListZonesResult(DnsZonesResultResponse),

    /// DNS delete zone result.
    DnsDeleteZoneResult(DnsDeleteZoneResultResponse),

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

    // =========================================================================
    // FEATURE-GATED VARIANTS (must be at end for postcard discriminant stability)
    // =========================================================================

    // -------------------------------------------------------------------------
    // Pijul responses
    // -------------------------------------------------------------------------
    /// Pijul repository result.
    #[cfg(feature = "pijul")]
    PijulRepoResult(PijulRepoResponse),

    /// Pijul repository list result.
    #[cfg(feature = "pijul")]
    PijulRepoListResult(PijulRepoListResponse),

    /// Pijul channel result.
    #[cfg(feature = "pijul")]
    PijulChannelResult(PijulChannelResponse),

    /// Pijul channel list result.
    #[cfg(feature = "pijul")]
    PijulChannelListResult(PijulChannelListResponse),

    /// Pijul record result.
    #[cfg(feature = "pijul")]
    PijulRecordResult(PijulRecordResponse),

    /// Pijul apply result.
    #[cfg(feature = "pijul")]
    PijulApplyResult(PijulApplyResponse),

    /// Pijul unrecord result.
    #[cfg(feature = "pijul")]
    PijulUnrecordResult(PijulUnrecordResponse),

    /// Pijul log result.
    #[cfg(feature = "pijul")]
    PijulLogResult(PijulLogResponse),

    /// Pijul checkout result.
    #[cfg(feature = "pijul")]
    PijulCheckoutResult(PijulCheckoutResponse),

    /// Pijul show result.
    #[cfg(feature = "pijul")]
    PijulShowResult(PijulShowResponse),

    /// Pijul blame result.
    #[cfg(feature = "pijul")]
    PijulBlameResult(PijulBlameResponse),

    /// Pijul success (no payload).
    #[cfg(feature = "pijul")]
    PijulSuccess,

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
    /// Whether the node is initialized and ready to process non-bootstrap operations.
    /// A node becomes initialized when it receives Raft membership through replication.
    #[serde(default)]
    pub is_initialized: bool,
    /// Number of nodes in the current membership configuration.
    /// None if not yet initialized.
    #[serde(default)]
    pub membership_node_count: Option<u32>,
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
    /// Replication state for each node (only populated when this node is leader).
    ///
    /// Maps node_id -> matched_log_index. The matched index indicates how far
    /// each follower has replicated. A `None` value means the node's progress
    /// is unknown (e.g., newly added learner).
    pub replication: Option<Vec<ReplicationProgress>>,
}

/// Replication progress for a single node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicationProgress {
    /// Node identifier.
    pub node_id: u64,
    /// The highest log index known to be replicated on this node.
    /// `None` means replication progress is unknown.
    pub matched_index: Option<u64>,
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

/// Single entry from scan operation with revision metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanEntry {
    /// Key name.
    pub key: String,
    /// Value (as UTF-8 string).
    pub value: String,
    /// Per-key version counter (1, 2, 3...). Reset to 1 on delete+recreate.
    #[serde(default)]
    pub version: u64,
    /// Raft log index when key was first created.
    #[serde(default)]
    pub create_revision: u64,
    /// Raft log index of last modification.
    #[serde(default)]
    pub mod_revision: u64,
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

/// Delete blob result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteBlobResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Download blob result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadBlobResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// BLAKE3 hash of the downloaded blob (hex-encoded).
    pub hash: Option<String>,
    /// Size of the downloaded blob in bytes.
    pub size: Option<u64>,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Get blob status result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetBlobStatusResultResponse {
    /// Whether the blob exists.
    pub found: bool,
    /// BLAKE3 hash of the blob (hex-encoded).
    pub hash: Option<String>,
    /// Size of the blob in bytes.
    pub size: Option<u64>,
    /// Whether the blob is complete (all chunks present).
    pub complete: Option<bool>,
    /// List of protection tags.
    pub tags: Option<Vec<String>>,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Blob replicate pull result response.
///
/// Returned when a target node attempts to download a blob from a provider
/// as part of the replication system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlobReplicatePullResultResponse {
    /// Whether the replication succeeded.
    pub success: bool,
    /// BLAKE3 hash of the replicated blob (hex-encoded).
    pub hash: Option<String>,
    /// Size of the replicated blob in bytes.
    pub size: Option<u64>,
    /// Time taken to download in milliseconds.
    pub duration_ms: Option<u64>,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Get blob replication status result response.
///
/// Returns the current replication state of a blob including which nodes
/// have replicas, the policy, and health status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetBlobReplicationStatusResultResponse {
    /// Whether the blob has replication metadata.
    pub found: bool,
    /// BLAKE3 hash of the blob (hex-encoded).
    pub hash: Option<String>,
    /// Size of the blob in bytes.
    pub size: Option<u64>,
    /// Node IDs that have confirmed replicas.
    pub replica_nodes: Option<Vec<u64>>,
    /// Target replication factor from policy.
    pub replication_factor: Option<u32>,
    /// Minimum replicas required for durability.
    pub min_replicas: Option<u32>,
    /// Current replication status: "critical", "under_replicated", "degraded", "healthy",
    /// "over_replicated".
    pub status: Option<String>,
    /// Number of additional replicas needed to reach target.
    pub replicas_needed: Option<u32>,
    /// Timestamp when replication metadata was last updated (ISO 8601).
    pub updated_at: Option<String>,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Trigger blob replication result response.
///
/// Returned when manually triggering replication of a blob.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerBlobReplicationResultResponse {
    /// Whether the replication was triggered successfully.
    pub success: bool,
    /// BLAKE3 hash of the blob (hex-encoded).
    pub hash: Option<String>,
    /// Node IDs that successfully received the blob.
    pub successful_nodes: Option<Vec<u64>>,
    /// Node IDs that failed to receive the blob with error messages.
    pub failed_nodes: Option<Vec<(u64, String)>>,
    /// Total replication time in milliseconds.
    pub duration_ms: Option<u64>,
    /// Error message if operation failed entirely.
    pub error: Option<String>,
}

/// Run blob repair cycle result response.
///
/// Returned when manually triggering a full blob repair cycle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunBlobRepairCycleResultResponse {
    /// Whether the repair cycle was initiated successfully.
    pub success: bool,
    /// Error message if initiation failed.
    pub error: Option<String>,
}

// =============================================================================
// Docs operation response types
// =============================================================================

/// Docs set result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocsSetResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// The key that was set.
    pub key: Option<String>,
    /// Size of the value in bytes.
    pub size: Option<u64>,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Docs get result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocsGetResultResponse {
    /// Whether the key was found.
    pub found: bool,
    /// The value if found.
    pub value: Option<Vec<u8>>,
    /// Size of the value in bytes.
    pub size: Option<u64>,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Docs delete result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocsDeleteResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Docs list entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocsListEntry {
    /// The key.
    pub key: String,
    /// Size of the value in bytes.
    pub size: u64,
    /// Content hash (hex-encoded).
    pub hash: String,
}

/// Docs list result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocsListResultResponse {
    /// List of entries.
    pub entries: Vec<DocsListEntry>,
    /// Total count returned.
    pub count: u32,
    /// Whether more entries are available.
    pub has_more: bool,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Docs status result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocsStatusResultResponse {
    /// Whether docs is enabled.
    pub enabled: bool,
    /// Namespace ID (hex-encoded).
    pub namespace_id: Option<String>,
    /// Author ID (hex-encoded).
    pub author_id: Option<String>,
    /// Number of entries in the namespace.
    pub entry_count: Option<u64>,
    /// Whether the replica is open.
    pub replica_open: Option<bool>,
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

/// Key origin lookup result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyOriginResultResponse {
    /// Whether the key has origin metadata.
    pub found: bool,
    /// The key that was looked up.
    pub key: String,
    /// Cluster ID that wrote the key (if found).
    pub cluster_id: Option<String>,
    /// Priority of the origin cluster (if found). Lower = higher priority.
    pub priority: Option<u32>,
    /// Unix timestamp when the key was last updated (if found).
    pub timestamp_secs: Option<u64>,
    /// Whether this is a local cluster origin (priority 0).
    pub is_local: Option<bool>,
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

/// SQL cell value for RPC transport.
///
/// PostCard-compatible representation of SQL values. Unlike `serde_json::Value`,
/// this enum uses explicit variants that PostCard can serialize without
/// self-describing serialization (`serialize_any()`).
///
/// Maps directly to SQLite's type affinity system.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SqlCellValue {
    /// SQL NULL value.
    Null,
    /// 64-bit signed integer (SQLite INTEGER).
    Integer(i64),
    /// 64-bit floating point (SQLite REAL).
    Real(f64),
    /// UTF-8 text string (SQLite TEXT).
    Text(String),
    /// Binary data as base64-encoded string (SQLite BLOB).
    /// Base64 encoding ensures safe text transport.
    Blob(String),
}

impl SqlCellValue {
    /// Convert to display string for TUI rendering.
    pub fn to_display_string(&self) -> String {
        match self {
            SqlCellValue::Null => "(null)".to_string(),
            SqlCellValue::Integer(i) => i.to_string(),
            SqlCellValue::Real(f) => f.to_string(),
            SqlCellValue::Text(s) => s.clone(),
            SqlCellValue::Blob(b64) => format!("[blob: {}]", b64),
        }
    }
}

/// SQL query result response.
///
/// Contains the result of a read-only SQL query execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SqlResultResponse {
    /// Whether the query succeeded.
    pub success: bool,
    /// Column names.
    pub columns: Option<Vec<String>>,
    /// Result rows. Each inner vec contains values for one row in column order.
    /// Uses `SqlCellValue` instead of `serde_json::Value` for PostCard compatibility.
    pub rows: Option<Vec<Vec<SqlCellValue>>>,
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

// =============================================================================
// Watch operation response types
// =============================================================================

/// Watch creation result response.
///
/// Returns watch ID on success for use in cancel/status operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchCreateResultResponse {
    /// Whether watch creation succeeded.
    pub success: bool,
    /// Unique watch ID for this subscription.
    pub watch_id: Option<u64>,
    /// Current committed log index at watch creation time.
    /// Useful for understanding the starting point.
    pub current_index: Option<u64>,
    /// Error message if watch creation failed.
    pub error: Option<String>,
}

/// Watch cancellation result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchCancelResultResponse {
    /// Whether cancellation succeeded.
    pub success: bool,
    /// Watch ID that was cancelled.
    pub watch_id: u64,
    /// Error message if cancellation failed (e.g., watch not found).
    pub error: Option<String>,
}

/// Watch status result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchStatusResultResponse {
    /// Whether status query succeeded.
    pub success: bool,
    /// List of watch statuses.
    pub watches: Option<Vec<WatchInfo>>,
    /// Error message if query failed.
    pub error: Option<String>,
}

/// Information about an active watch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchInfo {
    /// Unique watch ID.
    pub watch_id: u64,
    /// Key prefix being watched.
    pub prefix: String,
    /// Last sent log index.
    pub last_sent_index: u64,
    /// Number of events sent.
    pub events_sent: u64,
    /// Watch creation timestamp (ms since epoch).
    pub created_at_ms: u64,
    /// Whether the watch includes previous values.
    pub include_prev_value: bool,
}

/// Streaming watch event response.
///
/// Delivered asynchronously to clients with active watches.
/// Similar to etcd's WatchResponse.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchEventResponse {
    /// Watch ID this event belongs to.
    pub watch_id: u64,
    /// Log index of this event.
    pub index: u64,
    /// Raft term when the operation was committed.
    pub term: u64,
    /// Timestamp when committed (ms since epoch).
    pub committed_at_ms: u64,
    /// The key-value events in this batch.
    pub events: Vec<WatchKeyEvent>,
}

/// A single key change event within a watch response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchKeyEvent {
    /// Type of event.
    pub event_type: WatchEventType,
    /// Key that changed.
    pub key: String,
    /// New value (for Put events).
    pub value: Option<Vec<u8>>,
    /// Previous value (if include_prev_value was set).
    pub prev_value: Option<Vec<u8>>,
}

/// Type of watch event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WatchEventType {
    /// Key was created or updated.
    Put,
    /// Key was deleted.
    Delete,
}

// =============================================================================
// Lease operation response types
// =============================================================================

/// Lease grant result response.
///
/// Returned when a new lease is granted.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaseGrantResultResponse {
    /// Whether the lease was granted.
    pub success: bool,
    /// Unique lease ID (client-provided or server-generated).
    pub lease_id: Option<u64>,
    /// Granted TTL in seconds.
    pub ttl_seconds: Option<u32>,
    /// Error message if grant failed.
    pub error: Option<String>,
}

/// Lease revoke result response.
///
/// Returned when a lease is revoked.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaseRevokeResultResponse {
    /// Whether the lease was revoked.
    pub success: bool,
    /// Number of keys deleted with the lease.
    pub keys_deleted: Option<u32>,
    /// Error message if revoke failed (e.g., lease not found).
    pub error: Option<String>,
}

/// Lease keepalive result response.
///
/// Returned when a lease is refreshed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaseKeepaliveResultResponse {
    /// Whether the keepalive succeeded.
    pub success: bool,
    /// Lease ID that was refreshed.
    pub lease_id: Option<u64>,
    /// New TTL in seconds (reset to original TTL).
    pub ttl_seconds: Option<u32>,
    /// Error message if keepalive failed (e.g., lease not found or expired).
    pub error: Option<String>,
}

/// Lease time-to-live result response.
///
/// Returns lease metadata including remaining TTL and attached keys.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaseTimeToLiveResultResponse {
    /// Whether the query succeeded.
    pub success: bool,
    /// Lease ID queried.
    pub lease_id: Option<u64>,
    /// Original TTL in seconds.
    pub granted_ttl_seconds: Option<u32>,
    /// Remaining TTL in seconds (0 if expired).
    pub remaining_ttl_seconds: Option<u32>,
    /// Keys attached to the lease (if include_keys was true).
    pub keys: Option<Vec<String>>,
    /// Error message if query failed (e.g., lease not found).
    pub error: Option<String>,
}

/// Lease list result response.
///
/// Returns all active leases.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaseListResultResponse {
    /// Whether the query succeeded.
    pub success: bool,
    /// List of active leases.
    pub leases: Option<Vec<LeaseInfo>>,
    /// Error message if query failed.
    pub error: Option<String>,
}

/// Information about an active lease.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaseInfo {
    /// Unique lease ID.
    pub lease_id: u64,
    /// Original TTL in seconds.
    pub granted_ttl_seconds: u32,
    /// Remaining TTL in seconds.
    pub remaining_ttl_seconds: u32,
    /// Number of keys attached to this lease.
    pub attached_keys: u32,
}

// ============================================================================
// Barrier types
// ============================================================================

/// Barrier operation result response.
///
/// Used for BarrierEnter, BarrierLeave, and BarrierStatus operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BarrierResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// Current number of participants at the barrier.
    pub current_count: Option<u32>,
    /// Required number of participants to release the barrier.
    pub required_count: Option<u32>,
    /// Current barrier phase: "waiting", "ready", or "leaving".
    pub phase: Option<String>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

// ============================================================================
// Semaphore types
// ============================================================================

/// Semaphore operation result response.
///
/// Used for SemaphoreAcquire, SemaphoreTryAcquire, SemaphoreRelease, and SemaphoreStatus.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemaphoreResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// Number of permits acquired (for acquire operations).
    pub permits_acquired: Option<u32>,
    /// Number of permits currently available.
    pub available: Option<u32>,
    /// Total capacity of the semaphore.
    pub capacity: Option<u32>,
    /// Suggested retry delay in milliseconds (if acquire failed due to no permits).
    pub retry_after_ms: Option<u64>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

// ============================================================================
// Read-Write Lock types
// ============================================================================

/// Read-write lock operation result response.
///
/// Used for RWLockAcquireRead, RWLockAcquireWrite, RWLockRelease, RWLockDowngrade, and
/// RWLockStatus.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RWLockResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// Current lock mode: "free", "read", or "write".
    pub mode: Option<String>,
    /// Fencing token (for write locks and downgrade).
    pub fencing_token: Option<u64>,
    /// Lock deadline in milliseconds since epoch.
    pub deadline_ms: Option<u64>,
    /// Number of active readers.
    pub reader_count: Option<u32>,
    /// Writer holder ID (if mode == "write").
    pub writer_holder: Option<String>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

// ============================================================================
// Queue types
// ============================================================================

/// Item to enqueue in a batch operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueEnqueueItem {
    /// Item payload.
    pub payload: Vec<u8>,
    /// Optional TTL in milliseconds.
    pub ttl_ms: Option<u64>,
    /// Optional message group ID for FIFO ordering.
    pub message_group_id: Option<String>,
    /// Optional deduplication ID.
    pub deduplication_id: Option<String>,
}

/// Queue create operation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueCreateResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// True if queue was created, false if it already existed.
    pub created: bool,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Queue delete operation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueDeleteResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// Number of items deleted.
    pub items_deleted: Option<u64>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Queue enqueue operation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueEnqueueResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// Item ID assigned to the enqueued item.
    pub item_id: Option<u64>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Queue batch enqueue operation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueEnqueueBatchResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// Item IDs assigned to the enqueued items.
    pub item_ids: Vec<u64>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// A dequeued item with receipt handle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueDequeuedItemResponse {
    /// Item ID.
    pub item_id: u64,
    /// Item payload.
    pub payload: Vec<u8>,
    /// Receipt handle for ack/nack.
    pub receipt_handle: String,
    /// Number of delivery attempts (including this one).
    pub delivery_attempts: u32,
    /// Original enqueue time (Unix ms).
    pub enqueued_at_ms: u64,
    /// Visibility deadline (Unix ms).
    pub visibility_deadline_ms: u64,
}

/// Queue dequeue operation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueDequeueResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// Dequeued items.
    pub items: Vec<QueueDequeuedItemResponse>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// A queue item for peek response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueItemResponse {
    /// Item ID.
    pub item_id: u64,
    /// Item payload.
    pub payload: Vec<u8>,
    /// Original enqueue time (Unix ms).
    pub enqueued_at_ms: u64,
    /// Expiration time (Unix ms), 0 = no expiration.
    pub expires_at_ms: u64,
    /// Number of delivery attempts.
    pub delivery_attempts: u32,
}

/// Queue peek operation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueuePeekResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// Peeked items.
    pub items: Vec<QueueItemResponse>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Queue ack operation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueAckResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Queue nack operation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueNackResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Queue extend visibility operation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueExtendVisibilityResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// New visibility deadline (Unix ms).
    pub new_deadline_ms: Option<u64>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Queue status result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueStatusResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// Whether the queue exists.
    pub exists: bool,
    /// Approximate number of visible items.
    pub visible_count: Option<u64>,
    /// Approximate number of pending items.
    pub pending_count: Option<u64>,
    /// Approximate number of DLQ items.
    pub dlq_count: Option<u64>,
    /// Total items enqueued.
    pub total_enqueued: Option<u64>,
    /// Total items acked.
    pub total_acked: Option<u64>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// A DLQ item response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueDLQItemResponse {
    /// Item ID.
    pub item_id: u64,
    /// Item payload.
    pub payload: Vec<u8>,
    /// Original enqueue time (Unix ms).
    pub enqueued_at_ms: u64,
    /// Delivery attempts before moving to DLQ.
    pub delivery_attempts: u32,
    /// Reason for moving to DLQ.
    pub reason: String,
    /// Time moved to DLQ (Unix ms).
    pub moved_at_ms: u64,
    /// Last error message (if any).
    pub last_error: Option<String>,
}

/// Queue get DLQ operation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueGetDLQResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// DLQ items.
    pub items: Vec<QueueDLQItemResponse>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Queue redrive DLQ operation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueRedriveDLQResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

// ============================================================================
// Service Registry types
// ============================================================================

/// Service register operation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceRegisterResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// Fencing token for this registration.
    pub fencing_token: Option<u64>,
    /// Registration deadline (Unix ms).
    pub deadline_ms: Option<u64>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Service deregister operation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceDeregisterResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// Whether the instance was registered before removal.
    pub was_registered: bool,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// A service instance in discovery results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceInstanceResponse {
    /// Unique instance identifier.
    pub instance_id: String,
    /// Service name.
    pub service_name: String,
    /// Network address (host:port).
    pub address: String,
    /// Health status: "healthy", "unhealthy", "unknown".
    pub health_status: String,
    /// Version string.
    pub version: String,
    /// Tags for filtering.
    pub tags: Vec<String>,
    /// Load balancing weight.
    pub weight: u32,
    /// Custom metadata (JSON object).
    pub custom_metadata: String,
    /// Registration time (Unix ms).
    pub registered_at_ms: u64,
    /// Last heartbeat time (Unix ms).
    pub last_heartbeat_ms: u64,
    /// TTL deadline (Unix ms).
    pub deadline_ms: u64,
    /// Associated lease ID (if any).
    pub lease_id: Option<u64>,
    /// Fencing token.
    pub fencing_token: u64,
}

/// Service discover operation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceDiscoverResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// List of matching instances.
    pub instances: Vec<ServiceInstanceResponse>,
    /// Number of instances returned.
    pub count: u32,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Service list operation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceListResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// List of service names.
    pub services: Vec<String>,
    /// Number of services returned.
    pub count: u32,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Service get instance operation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceGetInstanceResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// Whether the instance was found.
    pub found: bool,
    /// The instance (if found).
    pub instance: Option<ServiceInstanceResponse>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Service heartbeat operation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceHeartbeatResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// New deadline (Unix ms).
    pub new_deadline_ms: Option<u64>,
    /// Current health status.
    pub health_status: Option<String>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Service update health operation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceUpdateHealthResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Service update metadata operation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceUpdateMetadataResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

// =============================================================================
// DNS response types
// =============================================================================

/// DNS record response structure.
///
/// Contains a single DNS record in JSON format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsRecordResponse {
    /// Domain name.
    pub domain: String,
    /// Record type (A, AAAA, CNAME, MX, TXT, SRV, NS, SOA, PTR, CAA).
    pub record_type: String,
    /// TTL in seconds.
    pub ttl_seconds: u32,
    /// Record data as JSON string (format depends on type).
    pub data_json: String,
    /// Unix timestamp when record was last updated (milliseconds).
    pub updated_at_ms: u64,
}

/// DNS record operation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsRecordResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// Whether a record was found (for get operations).
    pub found: bool,
    /// The record (if found or created).
    pub record: Option<DnsRecordResponse>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// DNS records operation result (multiple records).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsRecordsResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// List of records.
    pub records: Vec<DnsRecordResponse>,
    /// Number of records returned.
    pub count: u32,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// DNS delete record result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsDeleteRecordResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// Whether the record existed and was deleted.
    pub deleted: bool,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// DNS zone response structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsZoneResponse {
    /// Zone name (e.g., "example.com").
    pub name: String,
    /// Whether the zone is enabled.
    pub enabled: bool,
    /// Default TTL for records in this zone.
    pub default_ttl: u32,
    /// SOA serial number.
    pub serial: u32,
    /// Unix timestamp of last modification (milliseconds).
    pub last_modified_ms: u64,
    /// Optional description.
    pub description: Option<String>,
}

/// DNS zone operation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsZoneResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// Whether a zone was found (for get operations).
    pub found: bool,
    /// The zone (if found or created).
    pub zone: Option<DnsZoneResponse>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// DNS zones list result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsZonesResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// List of zones.
    pub zones: Vec<DnsZoneResponse>,
    /// Number of zones returned.
    pub count: u32,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// DNS delete zone result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsDeleteZoneResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// Whether the zone existed and was deleted.
    pub deleted: bool,
    /// Number of records deleted (if delete_records was true).
    pub records_deleted: u32,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

// =============================================================================
// Sharding response types
// =============================================================================

/// Shard topology result for GetTopology RPC.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopologyResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// Current topology version.
    pub version: u64,
    /// Whether the topology was updated (false if client version matches).
    pub updated: bool,
    /// Serialized ShardTopology (JSON) if updated is true.
    pub topology_data: Option<String>,
    /// Number of shards in the topology.
    pub shard_count: u32,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

// =============================================================================
// Forge response types (decentralized git)
// =============================================================================

/// Repository information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgeRepoInfo {
    /// Repository ID (hex-encoded BLAKE3 hash).
    pub id: String,
    /// Repository name.
    pub name: String,
    /// Optional description.
    pub description: Option<String>,
    /// Default branch name.
    pub default_branch: String,
    /// Delegate public keys (hex-encoded).
    pub delegates: Vec<String>,
    /// Signature threshold.
    pub threshold: u32,
    /// Creation timestamp (ms since epoch).
    pub created_at_ms: u64,
}

/// Repository operation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgeRepoResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// Repository info (if found/created).
    pub repo: Option<ForgeRepoInfo>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Repository list result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgeRepoListResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// List of repositories.
    pub repos: Vec<ForgeRepoInfo>,
    /// Total count.
    pub count: u32,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Blob operation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgeBlobResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// Blob hash (hex-encoded BLAKE3).
    pub hash: Option<String>,
    /// Blob content (for get operations).
    pub content: Option<Vec<u8>>,
    /// Blob size in bytes.
    pub size: Option<u64>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Tree entry information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgeTreeEntry {
    /// File mode (e.g., 0o100644 for regular file).
    pub mode: u32,
    /// Entry name.
    pub name: String,
    /// Entry hash (hex-encoded BLAKE3).
    pub hash: String,
}

/// Tree operation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgeTreeResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// Tree hash (hex-encoded BLAKE3).
    pub hash: Option<String>,
    /// Tree entries (for get operations).
    pub entries: Option<Vec<ForgeTreeEntry>>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Commit information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgeCommitInfo {
    /// Commit hash (hex-encoded BLAKE3).
    pub hash: String,
    /// Tree hash.
    pub tree: String,
    /// Parent commit hashes.
    pub parents: Vec<String>,
    /// Author name.
    pub author_name: String,
    /// Author email.
    pub author_email: Option<String>,
    /// Author public key (hex-encoded).
    pub author_key: Option<String>,
    /// Commit message.
    pub message: String,
    /// Timestamp (ms since epoch).
    pub timestamp_ms: u64,
}

/// Commit operation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgeCommitResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// Commit info (if found/created).
    pub commit: Option<ForgeCommitInfo>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Commit log result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgeLogResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// List of commits.
    pub commits: Vec<ForgeCommitInfo>,
    /// Total commits returned.
    pub count: u32,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Ref information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgeRefInfo {
    /// Ref name (e.g., "heads/main", "tags/v1.0").
    pub name: String,
    /// Target hash (hex-encoded BLAKE3).
    pub hash: String,
}

/// Ref operation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgeRefResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// Whether the ref was found (for get/delete).
    pub found: bool,
    /// Ref info (if found).
    pub ref_info: Option<ForgeRefInfo>,
    /// Previous hash (for CAS operations).
    pub previous_hash: Option<String>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Ref list result (branches or tags).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgeRefListResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// List of refs.
    pub refs: Vec<ForgeRefInfo>,
    /// Total count.
    pub count: u32,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Comment information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgeCommentInfo {
    /// Comment hash (change ID).
    pub hash: String,
    /// Author public key (hex-encoded).
    pub author: String,
    /// Comment body.
    pub body: String,
    /// Timestamp (ms since epoch).
    pub timestamp_ms: u64,
}

/// Issue information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgeIssueInfo {
    /// Issue ID (hex-encoded).
    pub id: String,
    /// Issue title.
    pub title: String,
    /// Issue body.
    pub body: String,
    /// State: "open" or "closed".
    pub state: String,
    /// Labels.
    pub labels: Vec<String>,
    /// Number of comments.
    pub comment_count: u32,
    /// Assignee public keys (hex-encoded).
    pub assignees: Vec<String>,
    /// Creation timestamp (ms since epoch).
    pub created_at_ms: u64,
    /// Last update timestamp (ms since epoch).
    pub updated_at_ms: u64,
}

/// Issue operation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgeIssueResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// Issue info (if found/created).
    pub issue: Option<ForgeIssueInfo>,
    /// Comments (for detailed get).
    pub comments: Option<Vec<ForgeCommentInfo>>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Issue list result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgeIssueListResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// List of issues.
    pub issues: Vec<ForgeIssueInfo>,
    /// Total count.
    pub count: u32,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Patch revision information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgePatchRevision {
    /// Revision hash.
    pub hash: String,
    /// Head commit hash.
    pub head: String,
    /// Optional revision message.
    pub message: Option<String>,
    /// Author public key (hex-encoded).
    pub author: String,
    /// Timestamp (ms since epoch).
    pub timestamp_ms: u64,
}

/// Patch approval information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgePatchApproval {
    /// Approver public key (hex-encoded).
    pub author: String,
    /// Approved commit hash.
    pub commit: String,
    /// Optional approval message.
    pub message: Option<String>,
    /// Timestamp (ms since epoch).
    pub timestamp_ms: u64,
}

/// Patch information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgePatchInfo {
    /// Patch ID (hex-encoded).
    pub id: String,
    /// Patch title.
    pub title: String,
    /// Patch description.
    pub description: String,
    /// State: "open", "merged", or "closed".
    pub state: String,
    /// Base commit hash.
    pub base: String,
    /// Current head commit hash.
    pub head: String,
    /// Labels.
    pub labels: Vec<String>,
    /// Number of revisions.
    pub revision_count: u32,
    /// Number of approvals.
    pub approval_count: u32,
    /// Assignee/reviewer public keys (hex-encoded).
    pub assignees: Vec<String>,
    /// Creation timestamp (ms since epoch).
    pub created_at_ms: u64,
    /// Last update timestamp (ms since epoch).
    pub updated_at_ms: u64,
}

/// Patch operation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgePatchResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// Patch info (if found/created).
    pub patch: Option<ForgePatchInfo>,
    /// Comments (for detailed get).
    pub comments: Option<Vec<ForgeCommentInfo>>,
    /// Revisions (for detailed get).
    pub revisions: Option<Vec<ForgePatchRevision>>,
    /// Approvals (for detailed get).
    pub approvals: Option<Vec<ForgePatchApproval>>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Patch list result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgePatchListResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// List of patches.
    pub patches: Vec<ForgePatchInfo>,
    /// Total count.
    pub count: u32,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Generic forge operation result (for simple success/error responses).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgeOperationResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Delegate key result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgeKeyResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// Public key (hex-encoded).
    pub public_key: Option<String>,
    /// Secret key (hex-encoded). Only returned for authorized local requests.
    pub secret_key: Option<String>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

// =============================================================================
// Federation Response Structs
// =============================================================================

/// Federation status response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationStatusResponse {
    /// Whether federation is enabled.
    pub enabled: bool,
    /// Cluster name.
    pub cluster_name: String,
    /// Cluster public key (base32).
    pub cluster_key: String,
    /// Whether DHT discovery is enabled.
    pub dht_enabled: bool,
    /// Whether gossip is enabled.
    pub gossip_enabled: bool,
    /// Number of discovered clusters.
    pub discovered_clusters: u32,
    /// Number of federated repositories.
    pub federated_repos: u32,
    /// Error message if status retrieval failed.
    pub error: Option<String>,
}

/// Discovered cluster info.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredClusterInfo {
    /// Cluster public key.
    pub cluster_key: String,
    /// Cluster name.
    pub name: String,
    /// Number of nodes.
    pub node_count: u32,
    /// Capabilities.
    pub capabilities: Vec<String>,
    /// When discovered.
    pub discovered_at: String,
}

/// List of discovered clusters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredClustersResponse {
    /// List of discovered clusters.
    pub clusters: Vec<DiscoveredClusterInfo>,
    /// Total count.
    pub count: u32,
    /// Error message if retrieval failed.
    pub error: Option<String>,
}

/// Single discovered cluster details.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredClusterResponse {
    /// Whether the cluster was found.
    pub found: bool,
    /// Cluster public key.
    pub cluster_key: Option<String>,
    /// Cluster name.
    pub name: Option<String>,
    /// Number of nodes.
    pub node_count: Option<u32>,
    /// Capabilities.
    pub capabilities: Option<Vec<String>>,
    /// Relay URLs.
    pub relay_urls: Option<Vec<String>>,
    /// When discovered.
    pub discovered_at: Option<String>,
}

/// Trust cluster result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustClusterResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Untrust cluster result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UntrustClusterResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Federate repository result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederateRepositoryResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// Federated ID (if successful).
    pub fed_id: Option<String>,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Federated repository info.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederatedRepoInfo {
    /// Repository ID.
    pub repo_id: String,
    /// Federation mode.
    pub mode: String,
    /// Federated ID.
    pub fed_id: String,
}

/// List of federated repositories.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederatedRepositoriesResponse {
    /// List of federated repositories.
    pub repositories: Vec<FederatedRepoInfo>,
    /// Total count.
    pub count: u32,
    /// Error message if retrieval failed.
    pub error: Option<String>,
}

/// Forge fetch federated result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgeFetchFederatedResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// Remote cluster name.
    pub remote_cluster: Option<String>,
    /// Number of objects fetched.
    pub fetched: usize,
    /// Number of objects already present locally.
    pub already_present: usize,
    /// Errors encountered during fetch.
    pub errors: Vec<String>,
    /// Error message if operation failed.
    pub error: Option<String>,
}

// =============================================================================
// Git Bridge types (for git-remote-aspen)
// =============================================================================

/// Git object for import/export.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitBridgeObject {
    /// SHA-1 hash (hex-encoded, 40 characters).
    pub sha1: String,
    /// Object type: "blob", "tree", "commit", or "tag".
    pub object_type: String,
    /// Raw git object content (without header).
    pub data: Vec<u8>,
}

/// Ref update for git push.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitBridgeRefUpdate {
    /// Ref name (e.g., "refs/heads/main").
    pub ref_name: String,
    /// Old SHA-1 hash (for CAS), empty string if creating.
    pub old_sha1: String,
    /// New SHA-1 hash.
    pub new_sha1: String,
    /// Force update (bypass fast-forward check).
    pub force: bool,
}

/// Ref info for git list.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitBridgeRefInfo {
    /// Ref name (e.g., "refs/heads/main").
    pub ref_name: String,
    /// SHA-1 hash (hex-encoded, 40 characters).
    pub sha1: String,
}

/// Git bridge list refs response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitBridgeListRefsResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// List of refs with their SHA-1 hashes.
    pub refs: Vec<GitBridgeRefInfo>,
    /// HEAD symref target (e.g., "refs/heads/main"), if any.
    pub head: Option<String>,
    /// Error message if operation failed.
    pub error: Option<String>,
}

/// Git bridge fetch response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitBridgeFetchResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// Objects in dependency order (dependencies before dependents).
    pub objects: Vec<GitBridgeObject>,
    /// Number of objects skipped (already in have list).
    pub skipped: usize,
    /// Error message if operation failed.
    pub error: Option<String>,
}

/// Additional metadata for chunked git push operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitBridgePushMetadata {
    /// Optional commit message for the push.
    pub commit_message: Option<String>,
    /// Optional author information.
    pub author: Option<String>,
    /// Optional committer information.
    pub committer: Option<String>,
    /// Optional timestamp.
    pub timestamp: Option<u64>,
    /// Optional additional metadata as key-value pairs.
    pub additional: Option<std::collections::HashMap<String, String>>,
}

/// Git bridge push response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitBridgePushResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// Number of objects imported.
    pub objects_imported: usize,
    /// Number of objects skipped (already existed).
    pub objects_skipped: usize,
    /// Results for each ref update.
    pub ref_results: Vec<GitBridgeRefResult>,
    /// Error message if operation failed.
    pub error: Option<String>,
}

/// Result of a single ref update.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitBridgeRefResult {
    /// Ref name.
    pub ref_name: String,
    /// Whether the update succeeded.
    pub success: bool,
    /// Error message if update failed.
    pub error: Option<String>,
}

/// Response to GitBridgePushStart - provides session ID for chunked transfer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitBridgePushStartResponse {
    /// Unique session ID for this chunked push operation.
    pub session_id: String,
    /// Maximum chunk size in bytes that the server will accept.
    pub max_chunk_size: usize,
    /// Success indicator.
    pub success: bool,
    /// Error message if operation failed.
    pub error: Option<String>,
}

/// Response to GitBridgePushChunk - confirms chunk receipt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitBridgePushChunkResponse {
    /// Session ID being processed.
    pub session_id: String,
    /// Chunk ID that was processed.
    pub chunk_id: u64,
    /// Whether this chunk was received successfully.
    pub success: bool,
    /// Error message if chunk processing failed.
    pub error: Option<String>,
}

/// Response to GitBridgePushComplete - final push result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitBridgePushCompleteResponse {
    /// Session ID that was completed.
    pub session_id: String,
    /// Whether the entire operation succeeded.
    pub success: bool,
    /// Number of objects imported.
    pub objects_imported: usize,
    /// Number of objects skipped (already existed).
    pub objects_skipped: usize,
    /// Results for each ref update.
    pub ref_results: Vec<GitBridgeRefResult>,
    /// Error message if operation failed.
    pub error: Option<String>,
}

// =============================================================================
// Pijul Response Types
// =============================================================================

/// Pijul repository response.
#[cfg(feature = "pijul")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PijulRepoResponse {
    /// Repository ID (hex-encoded).
    pub id: String,
    /// Repository name.
    pub name: String,
    /// Optional description.
    pub description: Option<String>,
    /// Default channel name.
    pub default_channel: String,
    /// Number of channels.
    pub channel_count: u32,
    /// Created timestamp (ms since epoch).
    pub created_at_ms: u64,
}

/// Pijul repository list response.
#[cfg(feature = "pijul")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PijulRepoListResponse {
    /// Repositories.
    pub repos: Vec<PijulRepoResponse>,
    /// Total count.
    pub count: u32,
}

/// Pijul channel response.
#[cfg(feature = "pijul")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PijulChannelResponse {
    /// Channel name.
    pub name: String,
    /// Head change hash (hex-encoded, None if empty).
    pub head: Option<String>,
    /// Last updated timestamp (ms since epoch).
    pub updated_at_ms: u64,
}

/// Pijul channel list response.
#[cfg(feature = "pijul")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PijulChannelListResponse {
    /// Channels.
    pub channels: Vec<PijulChannelResponse>,
    /// Total count.
    pub count: u32,
}

/// Pijul recorded change info.
#[cfg(feature = "pijul")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PijulRecordedChange {
    /// Change hash (hex-encoded BLAKE3).
    pub hash: String,
    /// Change message.
    pub message: String,
    /// Number of hunks.
    pub hunks: u32,
    /// Size in bytes.
    pub size_bytes: u64,
}

/// Pijul record response.
#[cfg(feature = "pijul")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PijulRecordResponse {
    /// Recorded change (None if no changes).
    pub change: Option<PijulRecordedChange>,
}

/// Pijul apply response.
#[cfg(feature = "pijul")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PijulApplyResponse {
    /// Number of operations applied.
    pub operations: u64,
}

/// Pijul unrecord response.
#[cfg(feature = "pijul")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PijulUnrecordResponse {
    /// Whether the change was in the channel and was unrecorded.
    pub unrecorded: bool,
}

/// Pijul log entry.
#[cfg(feature = "pijul")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PijulLogEntry {
    /// Change hash (hex-encoded).
    pub change_hash: String,
    /// Change message.
    pub message: String,
    /// Author (name or key).
    pub author: Option<String>,
    /// Timestamp (ms since epoch).
    pub timestamp_ms: u64,
}

/// Pijul log response.
#[cfg(feature = "pijul")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PijulLogResponse {
    /// Log entries.
    pub entries: Vec<PijulLogEntry>,
    /// Total count.
    pub count: u32,
}

/// Pijul checkout response.
#[cfg(feature = "pijul")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PijulCheckoutResponse {
    /// Number of files written.
    pub files_written: u32,
    /// Number of conflicts.
    pub conflicts: u32,
}

/// Pijul show response - details of a specific change.
#[cfg(feature = "pijul")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PijulShowResponse {
    /// Full change hash (hex-encoded BLAKE3).
    pub change_hash: String,
    /// Repository ID.
    pub repo_id: String,
    /// Channel the change was recorded to.
    pub channel: String,
    /// Change message/description.
    pub message: String,
    /// Authors of the change.
    pub authors: Vec<PijulAuthorInfo>,
    /// Hashes of changes this change depends on.
    pub dependencies: Vec<String>,
    /// Size of the change in bytes.
    pub size_bytes: u64,
    /// Timestamp when recorded (milliseconds since epoch).
    pub recorded_at_ms: u64,
}

/// Author information for a Pijul change.
#[cfg(feature = "pijul")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PijulAuthorInfo {
    /// Author name.
    pub name: String,
    /// Author email (optional).
    pub email: Option<String>,
}

/// Pijul blame response - attribution information for a file.
#[cfg(feature = "pijul")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PijulBlameResponse {
    /// Path of the file being blamed.
    pub path: String,
    /// Channel the blame was performed on.
    pub channel: String,
    /// Repository ID.
    pub repo_id: String,
    /// List of changes that contributed to this file.
    pub attributions: Vec<PijulBlameEntry>,
    /// Whether the file currently exists in the channel.
    pub file_exists: bool,
}

/// A single attribution entry in blame output.
#[cfg(feature = "pijul")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PijulBlameEntry {
    /// Hash of the change (hex-encoded).
    pub change_hash: String,
    /// Author name.
    pub author: Option<String>,
    /// Author email.
    pub author_email: Option<String>,
    /// Change message (first line).
    pub message: String,
    /// Timestamp when recorded (milliseconds since epoch).
    pub recorded_at_ms: u64,
    /// Type of change: "add", "modify", "delete", "rename", "unknown".
    pub change_type: String,
}

// ============================================================================
// Automerge response types
// ============================================================================

/// Automerge create document result response.
#[cfg(feature = "automerge")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomergeCreateResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// Created document ID.
    pub document_id: Option<String>,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Automerge get document result response.
#[cfg(feature = "automerge")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomergeGetResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// Whether the document was found.
    pub found: bool,
    /// Document ID.
    pub document_id: Option<String>,
    /// Serialized Automerge document bytes (base64-encoded).
    pub document_bytes: Option<String>,
    /// Document metadata.
    pub metadata: Option<AutomergeDocumentMetadata>,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Automerge save document result response.
#[cfg(feature = "automerge")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomergeSaveResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// Document size in bytes.
    pub size_bytes: Option<u64>,
    /// Number of changes in document.
    pub change_count: Option<u64>,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Automerge delete document result response.
#[cfg(feature = "automerge")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomergeDeleteResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// Whether the document existed before deletion.
    pub existed: bool,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Automerge apply changes result response.
#[cfg(feature = "automerge")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomergeApplyChangesResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// Whether any changes were applied.
    pub changes_applied: bool,
    /// Number of changes applied.
    pub change_count: Option<u64>,
    /// New document heads (hex-encoded change hashes).
    pub new_heads: Vec<String>,
    /// New document size in bytes.
    pub new_size: Option<u64>,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Automerge merge documents result response.
#[cfg(feature = "automerge")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomergeMergeResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// Whether any changes were applied from merge.
    pub changes_applied: bool,
    /// Number of changes applied.
    pub change_count: Option<u64>,
    /// New document heads (hex-encoded change hashes).
    pub new_heads: Vec<String>,
    /// New document size in bytes.
    pub new_size: Option<u64>,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Automerge list documents result response.
#[cfg(feature = "automerge")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomergeListResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// List of document metadata.
    pub documents: Vec<AutomergeDocumentMetadata>,
    /// Whether there are more results.
    pub has_more: bool,
    /// Continuation token for fetching next page.
    pub continuation_token: Option<String>,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Automerge get metadata result response.
#[cfg(feature = "automerge")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomergeGetMetadataResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// Whether the document was found.
    pub found: bool,
    /// Document metadata.
    pub metadata: Option<AutomergeDocumentMetadata>,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Automerge exists check result response.
#[cfg(feature = "automerge")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomergeExistsResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// Whether the document exists.
    pub exists: bool,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Automerge generate sync message result response.
#[cfg(feature = "automerge")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomergeGenerateSyncMessageResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// Whether a sync message was generated (None means peer is up-to-date).
    pub has_message: bool,
    /// Sync message bytes (base64-encoded), if generated.
    pub message: Option<String>,
    /// Updated sync state (base64-encoded) for persistence.
    pub sync_state: Option<String>,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Automerge receive sync message result response.
#[cfg(feature = "automerge")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomergeReceiveSyncMessageResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// Whether any changes were applied from the sync message.
    pub changes_applied: bool,
    /// Updated sync state (base64-encoded) for persistence.
    pub sync_state: Option<String>,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Automerge document metadata.
#[cfg(feature = "automerge")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomergeDocumentMetadata {
    /// Document ID.
    pub document_id: String,
    /// Optional namespace.
    pub namespace: Option<String>,
    /// Optional title.
    pub title: Option<String>,
    /// Optional description.
    pub description: Option<String>,
    /// Creation timestamp (milliseconds since epoch).
    pub created_at_ms: u64,
    /// Last update timestamp (milliseconds since epoch).
    pub updated_at_ms: u64,
    /// Document size in bytes.
    pub size_bytes: u64,
    /// Number of changes in document history.
    pub change_count: u64,
    /// Current document heads (hex-encoded change hashes).
    pub heads: Vec<String>,
    /// Creator actor ID (hex-encoded).
    pub creator_actor_id: Option<String>,
    /// Tags for categorization.
    pub tags: Vec<String>,
}

// ============================================================================
// Job types
// ============================================================================

/// Job submit result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobSubmitResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// Job ID assigned to the submitted job.
    pub job_id: Option<String>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Job details for get/list operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobDetails {
    /// Job ID.
    pub job_id: String,
    /// Job type.
    pub job_type: String,
    /// Job status.
    pub status: String,
    /// Priority level.
    pub priority: u8,
    /// Progress percentage (0-100).
    pub progress: u8,
    /// Progress message.
    pub progress_message: Option<String>,
    /// Job payload (JSON-encoded string).
    pub payload: String,
    /// Tags associated with the job.
    pub tags: Vec<String>,
    /// Submission time (ISO 8601).
    pub submitted_at: String,
    /// Start time (ISO 8601).
    pub started_at: Option<String>,
    /// Completion time (ISO 8601).
    pub completed_at: Option<String>,
    /// Worker ID processing this job.
    pub worker_id: Option<String>,
    /// Number of retry attempts.
    pub attempts: u32,
    /// Job result (if completed, JSON-encoded string).
    pub result: Option<String>,
    /// Error message (if failed).
    pub error_message: Option<String>,
}

/// Job get result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobGetResultResponse {
    /// Whether the job was found.
    pub found: bool,
    /// Job details if found.
    pub job: Option<JobDetails>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Job list result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobListResultResponse {
    /// List of jobs matching the filter.
    pub jobs: Vec<JobDetails>,
    /// Total count of matching jobs.
    pub total_count: u32,
    /// Continuation token for pagination.
    pub continuation_token: Option<String>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Job cancel result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobCancelResultResponse {
    /// Whether the cancellation succeeded.
    pub success: bool,
    /// Previous status of the job.
    pub previous_status: Option<String>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Job update progress result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobUpdateProgressResultResponse {
    /// Whether the update succeeded.
    pub success: bool,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Job queue statistics response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobQueueStatsResultResponse {
    /// Number of pending jobs.
    pub pending_count: u64,
    /// Number of scheduled jobs.
    pub scheduled_count: u64,
    /// Number of running jobs.
    pub running_count: u64,
    /// Number of completed jobs (recent).
    pub completed_count: u64,
    /// Number of failed jobs (recent).
    pub failed_count: u64,
    /// Number of cancelled jobs (recent).
    pub cancelled_count: u64,
    /// Jobs per priority level.
    pub priority_counts: Vec<PriorityCount>,
    /// Jobs per type.
    pub type_counts: Vec<TypeCount>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Priority level job count.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriorityCount {
    /// Priority level (0=Low, 1=Normal, 2=High, 3=Critical).
    pub priority: u8,
    /// Number of jobs at this priority.
    pub count: u64,
}

/// Job type count.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeCount {
    /// Job type name.
    pub job_type: String,
    /// Number of jobs of this type.
    pub count: u64,
}

/// Worker information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerInfo {
    /// Worker ID.
    pub worker_id: String,
    /// Worker status: idle, busy, offline.
    pub status: String,
    /// Job types this worker can handle.
    pub capabilities: Vec<String>,
    /// Maximum concurrent jobs.
    pub capacity: u32,
    /// Currently active job count.
    pub active_jobs: u32,
    /// Job IDs currently being processed.
    pub active_job_ids: Vec<String>,
    /// Last heartbeat time (ISO 8601).
    pub last_heartbeat: String,
    /// Total jobs processed.
    pub total_processed: u64,
    /// Total jobs failed.
    pub total_failed: u64,
}

/// Worker status result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerStatusResultResponse {
    /// List of registered workers.
    pub workers: Vec<WorkerInfo>,
    /// Total worker count.
    pub total_workers: u32,
    /// Number of idle workers.
    pub idle_workers: u32,
    /// Number of busy workers.
    pub busy_workers: u32,
    /// Number of offline workers.
    pub offline_workers: u32,
    /// Total capacity across all workers.
    pub total_capacity: u32,
    /// Currently used capacity.
    pub used_capacity: u32,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Worker register result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerRegisterResultResponse {
    /// Whether registration succeeded.
    pub success: bool,
    /// Assigned worker token for authentication.
    pub worker_token: Option<String>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Worker heartbeat result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerHeartbeatResultResponse {
    /// Whether heartbeat was accepted.
    pub success: bool,
    /// Jobs to dequeue (job IDs).
    pub jobs_to_process: Vec<String>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Worker deregister result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerDeregisterResultResponse {
    /// Whether deregistration succeeded.
    pub success: bool,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

// ============================================================================
// Hook Response Types
// ============================================================================

/// Information about a configured hook handler.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookHandlerInfo {
    /// Handler name (unique identifier).
    pub name: String,
    /// Topic pattern this handler subscribes to (NATS-style wildcards).
    pub pattern: String,
    /// Handler type: "in_process", "shell", or "forward".
    pub handler_type: String,
    /// Execution mode: "direct" or "job".
    pub execution_mode: String,
    /// Whether the handler is enabled.
    pub enabled: bool,
    /// Timeout in milliseconds.
    pub timeout_ms: u64,
    /// Number of retries on failure.
    pub retry_count: u32,
}

/// Hook list result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookListResultResponse {
    /// Whether the hook service is enabled.
    pub enabled: bool,
    /// List of configured handlers.
    pub handlers: Vec<HookHandlerInfo>,
}

/// Metrics for a single hook handler.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookHandlerMetrics {
    /// Handler name.
    pub name: String,
    /// Total successful executions.
    pub success_count: u64,
    /// Total failed executions.
    pub failure_count: u64,
    /// Total dropped events (due to concurrency limit).
    pub dropped_count: u64,
    /// Total jobs submitted (for job mode handlers).
    pub jobs_submitted: u64,
    /// Average execution duration in microseconds.
    pub avg_duration_us: u64,
    /// Maximum execution duration in microseconds.
    pub max_duration_us: u64,
}

/// Hook metrics result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookMetricsResultResponse {
    /// Whether the hook service is enabled.
    pub enabled: bool,
    /// Global metrics (all handlers).
    pub total_events_processed: u64,
    /// Per-handler metrics.
    pub handlers: Vec<HookHandlerMetrics>,
}

/// Hook trigger result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookTriggerResultResponse {
    /// Whether the trigger was successful.
    pub success: bool,
    /// Number of handlers that matched and were dispatched to.
    pub dispatched_count: usize,
    /// Error message if the operation failed.
    pub error: Option<String>,
    /// Any handler failures (handler_name -> error message).
    pub handler_failures: Vec<(String, String)>,
}

// =============================================================================
// CI/CD Response Types
// =============================================================================

/// CI trigger pipeline response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CiTriggerPipelineResponse {
    /// Whether the trigger was successful.
    pub success: bool,
    /// Pipeline run ID (if successful).
    pub run_id: Option<String>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// CI pipeline stage information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CiStageInfo {
    /// Stage name.
    pub name: String,
    /// Stage status: pending, running, succeeded, failed, cancelled.
    pub status: String,
    /// Jobs in this stage.
    pub jobs: Vec<CiJobInfo>,
}

/// CI pipeline job information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CiJobInfo {
    /// Job ID.
    pub id: String,
    /// Job name.
    pub name: String,
    /// Job status: pending, running, succeeded, failed, cancelled.
    pub status: String,
    /// Job start time (Unix timestamp in milliseconds).
    pub started_at_ms: Option<u64>,
    /// Job end time (Unix timestamp in milliseconds).
    pub ended_at_ms: Option<u64>,
    /// Error message if job failed.
    pub error: Option<String>,
}

/// CI get status response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CiGetStatusResponse {
    /// Whether the pipeline run was found.
    pub found: bool,
    /// Pipeline run ID.
    pub run_id: Option<String>,
    /// Repository ID.
    pub repo_id: Option<String>,
    /// Git reference.
    pub ref_name: Option<String>,
    /// Commit hash.
    pub commit_hash: Option<String>,
    /// Pipeline status: pending, running, succeeded, failed, cancelled.
    pub status: Option<String>,
    /// Stage information.
    pub stages: Vec<CiStageInfo>,
    /// Creation time (Unix timestamp in milliseconds).
    pub created_at_ms: Option<u64>,
    /// Completion time (Unix timestamp in milliseconds).
    pub completed_at_ms: Option<u64>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// CI pipeline run summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CiRunInfo {
    /// Pipeline run ID.
    pub run_id: String,
    /// Repository ID.
    pub repo_id: String,
    /// Git reference.
    pub ref_name: String,
    /// Pipeline status.
    pub status: String,
    /// Creation time (Unix timestamp in milliseconds).
    pub created_at_ms: u64,
}

/// CI list runs response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CiListRunsResponse {
    /// Pipeline runs.
    pub runs: Vec<CiRunInfo>,
}

/// CI cancel run response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CiCancelRunResponse {
    /// Whether the cancel was successful.
    pub success: bool,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// CI watch repo response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CiWatchRepoResponse {
    /// Whether the watch was successful.
    pub success: bool,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// CI unwatch repo response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CiUnwatchRepoResponse {
    /// Whether the unwatch was successful.
    pub success: bool,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Information about a CI artifact.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CiArtifactInfo {
    /// Blob hash in the distributed store.
    pub blob_hash: String,
    /// Artifact name (e.g., store path for Nix builds).
    pub name: String,
    /// Size in bytes.
    pub size_bytes: u64,
    /// Content type (e.g., "application/x-nix-nar").
    pub content_type: String,
    /// When the artifact was created.
    pub created_at: String,
    /// Additional metadata.
    pub metadata: std::collections::HashMap<String, String>,
}

/// CI list artifacts response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CiListArtifactsResponse {
    /// Whether the operation was successful.
    pub success: bool,
    /// List of artifacts.
    pub artifacts: Vec<CiArtifactInfo>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// CI get artifact response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CiGetArtifactResponse {
    /// Whether the operation was successful.
    pub success: bool,
    /// Artifact metadata.
    pub artifact: Option<CiArtifactInfo>,
    /// Blob ticket for downloading (base32 encoded).
    pub blob_ticket: Option<String>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// A single CI log chunk from the KV store.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CiLogChunkInfo {
    /// Chunk index within the job's log stream.
    pub index: u32,
    /// Log content (may contain multiple lines with stream prefixes).
    pub content: String,
    /// Timestamp when this chunk was written (ms since epoch).
    pub timestamp_ms: u64,
}

/// CI get job logs response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CiGetJobLogsResponse {
    /// Whether the job was found.
    pub found: bool,
    /// Log chunks in order.
    pub chunks: Vec<CiLogChunkInfo>,
    /// Index of the last chunk returned.
    pub last_index: u32,
    /// Whether there are more chunks available.
    pub has_more: bool,
    /// Whether the log stream is complete (job finished).
    pub is_complete: bool,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// CI subscribe logs response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CiSubscribeLogsResponse {
    /// Whether the job was found.
    pub found: bool,
    /// KV prefix to watch via LOG_SUBSCRIBER_ALPN.
    ///
    /// Format: `_ci:logs:{run_id}:{job_id}:`
    pub watch_prefix: String,
    /// Current log index (for catch-up before subscribing).
    pub current_index: u64,
    /// Whether the job is still running (stream may have more data).
    pub is_running: bool,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// CI get job output response.
///
/// Returns the full stdout/stderr content for a completed job.
/// For large outputs stored in blobs, the server resolves the blob
/// references and returns the actual content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CiGetJobOutputResponse {
    /// Whether the job was found.
    pub found: bool,
    /// Full stdout content (resolved from blob if needed).
    pub stdout: Option<String>,
    /// Full stderr content (resolved from blob if needed).
    pub stderr: Option<String>,
    /// Whether stdout was stored as a blob.
    pub stdout_was_blob: bool,
    /// Whether stderr was stored as a blob.
    pub stderr_was_blob: bool,
    /// Total stdout size in bytes.
    pub stdout_size: u64,
    /// Total stderr size in bytes.
    pub stderr_size: u64,
    /// Error message if retrieval failed.
    pub error: Option<String>,
}

// =============================================================================
// Secrets Response Types
// =============================================================================

/// Version metadata for a KV secret.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretsKvVersionMetadata {
    /// Version number.
    pub version: u64,
    /// Creation time (Unix timestamp in milliseconds).
    pub created_time_unix_ms: u64,
    /// Deletion time if soft-deleted (Unix timestamp in milliseconds).
    pub deletion_time_unix_ms: Option<u64>,
    /// Whether this version has been permanently destroyed.
    pub destroyed: bool,
}

/// Secrets KV read result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretsKvReadResultResponse {
    /// Whether the read was successful.
    pub success: bool,
    /// Secret data (key-value pairs).
    pub data: Option<std::collections::HashMap<String, String>>,
    /// Version metadata.
    pub metadata: Option<SecretsKvVersionMetadata>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Secrets KV write result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretsKvWriteResultResponse {
    /// Whether the write was successful.
    pub success: bool,
    /// Version number of the written secret.
    pub version: Option<u64>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Secrets KV delete/destroy/undelete result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretsKvDeleteResultResponse {
    /// Whether the operation was successful.
    pub success: bool,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Secrets KV list result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretsKvListResultResponse {
    /// Whether the list was successful.
    pub success: bool,
    /// Secret keys (names only, not values).
    pub keys: Vec<String>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Version info in metadata response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretsKvVersionInfo {
    /// Version number.
    pub version: u64,
    /// Creation time (Unix timestamp in milliseconds).
    pub created_time_unix_ms: u64,
    /// Whether this version is deleted.
    pub deleted: bool,
    /// Whether this version is destroyed.
    pub destroyed: bool,
}

/// Secrets KV metadata result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretsKvMetadataResultResponse {
    /// Whether the read was successful.
    pub success: bool,
    /// Current version number.
    pub current_version: Option<u64>,
    /// Maximum versions to retain.
    pub max_versions: Option<u32>,
    /// Whether CAS is required for writes.
    pub cas_required: Option<bool>,
    /// Creation time (Unix timestamp in milliseconds).
    pub created_time_unix_ms: Option<u64>,
    /// Last update time (Unix timestamp in milliseconds).
    pub updated_time_unix_ms: Option<u64>,
    /// All version info.
    pub versions: Vec<SecretsKvVersionInfo>,
    /// Custom metadata.
    pub custom_metadata: Option<std::collections::HashMap<String, String>>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Secrets Transit encrypt result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretsTransitEncryptResultResponse {
    /// Whether the encryption was successful.
    pub success: bool,
    /// Ciphertext (prefixed with key version).
    pub ciphertext: Option<String>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Secrets Transit decrypt result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretsTransitDecryptResultResponse {
    /// Whether the decryption was successful.
    pub success: bool,
    /// Decrypted plaintext.
    pub plaintext: Option<Vec<u8>>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Secrets Transit sign result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretsTransitSignResultResponse {
    /// Whether the signing was successful.
    pub success: bool,
    /// Signature (prefixed with key version).
    pub signature: Option<String>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Secrets Transit verify result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretsTransitVerifyResultResponse {
    /// Whether the verification request was successful.
    pub success: bool,
    /// Whether the signature is valid.
    pub valid: Option<bool>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Secrets Transit datakey result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretsTransitDatakeyResultResponse {
    /// Whether the operation was successful.
    pub success: bool,
    /// Plaintext data key (if requested).
    pub plaintext: Option<Vec<u8>>,
    /// Encrypted/wrapped data key.
    pub ciphertext: Option<String>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Secrets Transit key operation result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretsTransitKeyResultResponse {
    /// Whether the operation was successful.
    pub success: bool,
    /// Key name.
    pub name: Option<String>,
    /// Current key version.
    pub version: Option<u64>,
    /// Key type.
    pub key_type: Option<String>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Secrets Transit list keys result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretsTransitListResultResponse {
    /// Whether the list was successful.
    pub success: bool,
    /// Key names.
    pub keys: Vec<String>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Secrets PKI certificate result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretsPkiCertificateResultResponse {
    /// Whether the operation was successful.
    pub success: bool,
    /// Certificate in PEM format.
    pub certificate: Option<String>,
    /// Private key in PEM format (only for issued certs, not for get operations).
    pub private_key: Option<String>,
    /// Certificate serial number.
    pub serial: Option<String>,
    /// Certificate signing request (for intermediate CA).
    pub csr: Option<String>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Secrets PKI revoke result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretsPkiRevokeResultResponse {
    /// Whether the revocation was successful.
    pub success: bool,
    /// Revoked serial number.
    pub serial: Option<String>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Secrets PKI CRL result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretsPkiCrlResultResponse {
    /// Whether the request was successful.
    pub success: bool,
    /// CRL in PEM format.
    pub crl: Option<String>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Secrets PKI list result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretsPkiListResultResponse {
    /// Whether the list was successful.
    pub success: bool,
    /// Serial numbers (for certs) or role names (for roles).
    pub items: Vec<String>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Secrets PKI role configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretsPkiRoleConfig {
    /// Role name.
    pub name: String,
    /// Allowed domain patterns.
    pub allowed_domains: Vec<String>,
    /// Maximum TTL in days.
    pub max_ttl_days: u32,
    /// Allow bare domains.
    pub allow_bare_domains: bool,
    /// Allow wildcard certificates.
    pub allow_wildcards: bool,
}

/// Secrets PKI role result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretsPkiRoleResultResponse {
    /// Whether the request was successful.
    pub success: bool,
    /// Role configuration.
    pub role: Option<SecretsPkiRoleConfig>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

// =============================================================================
// Nix Binary Cache Response Types
// =============================================================================

/// Cache entry returned by query.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntryResponse {
    /// Full store path (e.g., /nix/store/abc...-name).
    pub store_path: String,
    /// Store path hash (the abc... part).
    pub store_hash: String,
    /// BLAKE3 hash of the NAR in blob store.
    pub blob_hash: String,
    /// Size of NAR archive in bytes.
    pub nar_size: u64,
    /// SHA256 hash of NAR (for Nix verification).
    pub nar_hash: String,
    /// Original file size in bytes.
    pub file_size: Option<u64>,
    /// Store path references (dependencies).
    pub references: Vec<String>,
    /// Deriver store path.
    pub deriver: Option<String>,
    /// Creation time (Unix timestamp in milliseconds).
    pub created_at_ms: u64,
    /// Node ID that built this.
    pub created_by_node: u64,
    /// CI job ID that created this.
    pub ci_job_id: Option<String>,
    /// CI run ID that created this.
    pub ci_run_id: Option<String>,
}

/// Cache query result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheQueryResultResponse {
    /// Whether the store path was found in cache.
    pub found: bool,
    /// Cache entry if found.
    pub entry: Option<CacheEntryResponse>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Cache statistics response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStatsResultResponse {
    /// Total number of entries in cache.
    pub total_entries: u64,
    /// Total NAR bytes stored.
    pub total_nar_bytes: u64,
    /// Total query hits.
    pub query_hits: u64,
    /// Total query misses.
    pub query_misses: u64,
    /// Node ID.
    pub node_id: u64,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Cache download result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheDownloadResultResponse {
    /// Whether the store path was found.
    pub found: bool,
    /// Blob ticket for downloading the NAR (base64-encoded).
    pub blob_ticket: Option<String>,
    /// BLAKE3 hash of the NAR.
    pub blob_hash: Option<String>,
    /// Size of NAR archive in bytes.
    pub nar_size: Option<u64>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

// =============================================================================
// SNIX Response Types (for remote workers)
// =============================================================================

/// SNIX directory get result response.
///
/// Returns a directory from the cluster's DirectoryService.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnixDirectoryGetResultResponse {
    /// Whether the directory was found.
    pub found: bool,
    /// Protobuf-encoded directory (base64-encoded).
    /// Present only when `found` is true.
    pub directory_bytes: Option<String>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// SNIX directory put result response.
///
/// Returns the BLAKE3 digest of the stored directory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnixDirectoryPutResultResponse {
    /// Whether the directory was stored successfully.
    pub success: bool,
    /// BLAKE3 digest of the stored directory (hex-encoded, 64 chars).
    pub digest: Option<String>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// SNIX path info get result response.
///
/// Returns path info from the cluster's PathInfoService.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnixPathInfoGetResultResponse {
    /// Whether the path info was found.
    pub found: bool,
    /// Protobuf-encoded PathInfo (base64-encoded).
    /// Present only when `found` is true.
    pub pathinfo_bytes: Option<String>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// SNIX path info put result response.
///
/// Returns the path info that was stored.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnixPathInfoPutResultResponse {
    /// Whether the path info was stored successfully.
    pub success: bool,
    /// Store path that was registered (e.g., /nix/store/abc...-name).
    pub store_path: Option<String>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

// =============================================================================
// Worker Job Coordination Response Types
// =============================================================================

/// Job information returned by worker polling.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerJobInfo {
    /// Job identifier.
    pub job_id: String,
    /// Job type.
    pub job_type: String,
    /// Job specification data (JSON-encoded JobSpec).
    pub job_spec_json: String,
    /// Job priority.
    pub priority: String,
    /// When the job was created (Unix timestamp ms).
    pub created_at_ms: u64,
    /// Visibility timeout for this job (Unix timestamp ms).
    pub visibility_timeout_ms: u64,
    /// Receipt handle for acknowledging job completion (from queue).
    pub receipt_handle: String,
    /// Execution token for job completion (from job manager).
    pub execution_token: String,
}

/// Worker job polling result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerPollJobsResultResponse {
    /// Whether polling was successful.
    pub success: bool,
    /// Worker ID that polled for jobs.
    pub worker_id: String,
    /// Jobs assigned to this worker.
    pub jobs: Vec<WorkerJobInfo>,
    /// Error message if polling failed.
    pub error: Option<String>,
}

/// Worker job completion result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerCompleteJobResultResponse {
    /// Whether the job completion was recorded successfully.
    pub success: bool,
    /// Worker ID that completed the job.
    pub worker_id: String,
    /// Job ID that was completed.
    pub job_id: String,
    /// Error message if completion recording failed.
    pub error: Option<String>,
}

// =============================================================================
// Cache Migration Response Types
// =============================================================================

/// Cache migration start result response.
#[cfg(feature = "ci")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheMigrationStartResultResponse {
    /// Whether migration was started successfully.
    pub started: bool,
    /// Migration status if available.
    pub status: Option<CacheMigrationProgressResponse>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Cache migration status result response.
#[cfg(feature = "ci")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheMigrationStatusResultResponse {
    /// Whether migration is currently running.
    pub running: bool,
    /// Migration progress details.
    pub progress: Option<CacheMigrationProgressResponse>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Cache migration progress details.
#[cfg(feature = "ci")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheMigrationProgressResponse {
    /// Total legacy entries discovered.
    pub total_entries: u64,
    /// Entries successfully migrated.
    pub migrated_count: u64,
    /// Entries that failed migration.
    pub failed_count: u64,
    /// Entries skipped (already migrated or invalid).
    pub skipped_count: u64,
    /// Unix timestamp when migration started.
    pub started_at: u64,
    /// Unix timestamp of last update.
    pub last_updated: u64,
    /// Last processed store hash (for resumption).
    pub last_processed_hash: Option<String>,
    /// Whether migration is complete.
    pub is_complete: bool,
    /// Progress percentage (0.0 - 100.0).
    pub progress_percent: f64,
    /// Error message if migration encountered an issue.
    pub error_message: Option<String>,
}

/// Cache migration cancel result response.
#[cfg(feature = "ci")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheMigrationCancelResultResponse {
    /// Whether cancellation was successful.
    pub cancelled: bool,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Cache migration validation result response.
#[cfg(feature = "ci")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheMigrationValidateResultResponse {
    /// Whether all entries are migrated.
    pub complete: bool,
    /// Number of entries validated.
    pub validated_count: u64,
    /// Number of entries missing from SNIX storage.
    pub missing_count: u64,
    /// Sample of missing entry hashes (limited by max_report).
    pub missing_hashes: Vec<String>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

// =============================================================================
// Nix Cache Signing Key Response Types
// =============================================================================

/// Nix cache signing key operation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretsNixCacheKeyResultResponse {
    /// Whether the operation was successful.
    pub success: bool,
    /// Public key in Nix format ("{cache_name}:{base64_key}").
    pub public_key: Option<String>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Nix cache signing key deletion result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretsNixCacheDeleteResultResponse {
    /// Whether the deletion was successful.
    pub success: bool,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Nix cache signing keys list result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretsNixCacheListResultResponse {
    /// Whether the operation was successful.
    pub success: bool,
    /// List of cache names that have signing keys.
    pub cache_names: Option<Vec<String>>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

// =============================================================================
// Tests for postcard serialization roundtrip (discriminant stability)
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Test that Hook responses can roundtrip through postcard serialization.
    /// This verifies that feature-gated enum variants don't shift discriminants.
    #[test]
    fn test_hook_list_response_roundtrip() {
        let response = ClientRpcResponse::HookListResult(HookListResultResponse {
            enabled: true,
            handlers: vec![],
        });
        let bytes = postcard::to_stdvec(&response).expect("serialize");
        let decoded: ClientRpcResponse = postcard::from_bytes(&bytes).expect("deserialize");
        assert!(matches!(decoded, ClientRpcResponse::HookListResult(_)));
    }

    #[test]
    fn test_hook_metrics_response_roundtrip() {
        let response = ClientRpcResponse::HookMetricsResult(HookMetricsResultResponse {
            enabled: true,
            total_events_processed: 0,
            handlers: vec![],
        });
        let bytes = postcard::to_stdvec(&response).expect("serialize");
        let decoded: ClientRpcResponse = postcard::from_bytes(&bytes).expect("deserialize");
        assert!(matches!(decoded, ClientRpcResponse::HookMetricsResult(_)));
    }

    #[test]
    fn test_hook_trigger_response_roundtrip() {
        let response = ClientRpcResponse::HookTriggerResult(HookTriggerResultResponse {
            success: true,
            dispatched_count: 0,
            error: None,
            handler_failures: vec![],
        });
        let bytes = postcard::to_stdvec(&response).expect("serialize");
        let decoded: ClientRpcResponse = postcard::from_bytes(&bytes).expect("deserialize");
        assert!(matches!(decoded, ClientRpcResponse::HookTriggerResult(_)));
    }

    #[test]
    fn test_error_response_roundtrip() {
        let response = ClientRpcResponse::Error(ErrorResponse {
            code: "TEST_ERROR".to_string(),
            message: "test error message".to_string(),
        });
        let bytes = postcard::to_stdvec(&response).expect("serialize");
        let decoded: ClientRpcResponse = postcard::from_bytes(&bytes).expect("deserialize");
        assert!(matches!(decoded, ClientRpcResponse::Error(_)));
    }

    #[test]
    fn test_health_response_roundtrip() {
        let response = ClientRpcResponse::Health(HealthResponse {
            status: "healthy".to_string(),
            node_id: 1,
            raft_node_id: Some(1),
            uptime_seconds: 100,
            is_initialized: true,
            membership_node_count: Some(3),
        });
        let bytes = postcard::to_stdvec(&response).expect("serialize");
        let decoded: ClientRpcResponse = postcard::from_bytes(&bytes).expect("deserialize");
        assert!(matches!(decoded, ClientRpcResponse::Health(_)));
    }

    #[test]
    fn test_secrets_kv_read_response_roundtrip() {
        let response = ClientRpcResponse::SecretsKvReadResult(SecretsKvReadResultResponse {
            success: true,
            data: None,
            metadata: None,
            error: None,
        });
        let bytes = postcard::to_stdvec(&response).expect("serialize");
        let decoded: ClientRpcResponse = postcard::from_bytes(&bytes).expect("deserialize");
        assert!(matches!(decoded, ClientRpcResponse::SecretsKvReadResult(_)));
    }

    /// Verify that Hook request variants can roundtrip through postcard.
    #[test]
    fn test_hook_list_request_roundtrip() {
        let request = ClientRpcRequest::HookList;
        let bytes = postcard::to_stdvec(&request).expect("serialize");
        let decoded: ClientRpcRequest = postcard::from_bytes(&bytes).expect("deserialize");
        assert!(matches!(decoded, ClientRpcRequest::HookList));
    }

    #[test]
    fn test_hook_get_metrics_request_roundtrip() {
        let request = ClientRpcRequest::HookGetMetrics {
            handler_name: Some("test_handler".to_string()),
        };
        let bytes = postcard::to_stdvec(&request).expect("serialize");
        let decoded: ClientRpcRequest = postcard::from_bytes(&bytes).expect("deserialize");
        assert!(matches!(decoded, ClientRpcRequest::HookGetMetrics { handler_name: Some(_) }));
    }

    #[test]
    fn test_hook_trigger_request_roundtrip() {
        let request = ClientRpcRequest::HookTrigger {
            event_type: "write_committed".to_string(),
            payload_json: r#"{"key":"test"}"#.to_string(),
        };
        let bytes = postcard::to_stdvec(&request).expect("serialize");
        let decoded: ClientRpcRequest = postcard::from_bytes(&bytes).expect("deserialize");
        assert!(matches!(decoded, ClientRpcRequest::HookTrigger { .. }));
    }
}

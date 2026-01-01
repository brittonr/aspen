//! Client RPC protocol types for Aspen communication over Iroh.
//!
//! This module defines the RPC protocol used by clients to communicate with
//! aspen-node over Iroh P2P connections. It is separate from the Raft RPC
//! protocol which is used for cluster-internal consensus communication.
//!
//! # Architecture
//!
//! The Client RPC uses a distinct ALPN (`aspen-client`) to distinguish it from
//! Raft RPC. This allows clients to connect directly to nodes without needing HTTP.
//!
//! # Tiger Style
//!
//! - Explicit request/response pairs
//! - Bounded message sizes
//! - Fail-fast on invalid requests

use serde::Deserialize;
use serde::Serialize;

// BatchWriteOperation is included from rpc_types.rs at the end of this file

// ============================================================================
// Request Types
// ============================================================================

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
    GetClusterState,

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
    GetClientTicket {
        /// Access level: "read" or "write".
        access: String,
        /// Priority level (0 = highest).
        priority: u32,
    },

    /// Get a docs ticket for iroh-docs subscription.
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
    AddBlob {
        /// Blob data to store.
        data: Vec<u8>,
        /// Optional tag to protect the blob from GC.
        tag: Option<String>,
    },

    /// Get a blob by hash.
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
    DeleteBlob {
        /// BLAKE3 hash of the blob (hex-encoded).
        hash: String,
        /// Force deletion even if protected.
        force: bool,
    },

    /// Download a blob from a remote peer using a ticket.
    DownloadBlob {
        /// Serialized BlobTicket from the remote peer.
        ticket: String,
        /// Optional tag to protect the downloaded blob from GC.
        tag: Option<String>,
    },

    /// Download a blob by hash using DHT discovery.
    DownloadBlobByHash {
        /// BLAKE3 hash of the blob to download (hex-encoded).
        hash: String,
        /// Optional tag to protect the downloaded blob from GC.
        tag: Option<String>,
    },

    /// Download a blob from a specific provider using DHT mutable item lookup.
    DownloadBlobByProvider {
        /// BLAKE3 hash of the blob to download (hex-encoded).
        hash: String,
        /// Public key of the provider node (hex-encoded or base32).
        provider: String,
        /// Optional tag to protect the downloaded blob from GC.
        tag: Option<String>,
    },

    /// Get detailed status information about a blob.
    GetBlobStatus {
        /// BLAKE3 hash of the blob (hex-encoded).
        hash: String,
    },

    // =========================================================================
    // Docs operations (iroh-docs CRDT replication)
    // =========================================================================
    /// Set a key-value pair in the docs namespace.
    DocsSet {
        /// The key to set.
        key: String,
        /// The value to set.
        value: Vec<u8>,
    },

    /// Get a value from the docs namespace.
    DocsGet {
        /// The key to get.
        key: String,
    },

    /// Delete a key from the docs namespace.
    DocsDelete {
        /// The key to delete.
        key: String,
    },

    /// List entries in the docs namespace.
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
    GetKeyOrigin {
        /// The key to look up origin for.
        key: String,
    },

    // =========================================================================
    // SQL query operations
    // =========================================================================
    /// Execute a read-only SQL query against the state machine.
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
    LockTryAcquire {
        /// Lock key.
        key: String,
        /// Holder ID.
        holder_id: String,
        /// Lock TTL in milliseconds.
        ttl_ms: u64,
    },

    /// Release a distributed lock.
    LockRelease {
        /// Lock key.
        key: String,
        /// Holder ID that acquired the lock.
        holder_id: String,
        /// Fencing token from acquire operation.
        fencing_token: u64,
    },

    /// Renew a distributed lock's TTL.
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
    CounterGet { key: String },

    /// Increment an atomic counter by 1.
    CounterIncrement { key: String },

    /// Decrement an atomic counter by 1 (saturates at 0).
    CounterDecrement { key: String },

    /// Add an amount to an atomic counter.
    CounterAdd { key: String, amount: u64 },

    /// Subtract an amount from an atomic counter (saturates at 0).
    CounterSubtract { key: String, amount: u64 },

    /// Set an atomic counter to a specific value.
    CounterSet { key: String, value: u64 },

    /// Compare-and-set an atomic counter.
    CounterCompareAndSet {
        key: String,
        expected: u64,
        new_value: u64,
    },

    // =========================================================================
    // Coordination primitives - Signed Counter
    // =========================================================================
    /// Get the current value of a signed atomic counter.
    SignedCounterGet { key: String },

    /// Add an amount to a signed atomic counter (can be negative).
    SignedCounterAdd { key: String, amount: i64 },

    // =========================================================================
    // Coordination primitives - Sequence Generator
    // =========================================================================
    /// Get the next unique ID from a sequence.
    SequenceNext { key: String },

    /// Reserve a range of IDs from a sequence.
    SequenceReserve { key: String, count: u64 },

    /// Get the current (next available) value of a sequence without consuming it.
    SequenceCurrent { key: String },

    // =========================================================================
    // Coordination primitives - Rate Limiter
    // =========================================================================
    /// Try to acquire tokens from a rate limiter without blocking.
    RateLimiterTryAcquire {
        key: String,
        tokens: u64,
        capacity: u64,
        refill_rate: f64,
    },

    /// Acquire tokens from a rate limiter with timeout.
    RateLimiterAcquire {
        key: String,
        tokens: u64,
        capacity: u64,
        refill_rate: f64,
        timeout_ms: u64,
    },

    /// Check available tokens in a rate limiter without consuming.
    RateLimiterAvailable {
        key: String,
        capacity: u64,
        refill_rate: f64,
    },

    /// Reset a rate limiter to full capacity.
    RateLimiterReset {
        key: String,
        capacity: u64,
        refill_rate: f64,
    },

    // =========================================================================
    // Batch operations - Atomic multi-key operations
    // =========================================================================
    /// Read multiple keys atomically.
    BatchRead { keys: Vec<String> },

    /// Write multiple operations atomically.
    BatchWrite { operations: Vec<BatchWriteOperation> },

    /// Conditional batch write (etcd-style transaction).
    ConditionalBatchWrite {
        conditions: Vec<BatchCondition>,
        operations: Vec<BatchWriteOperation>,
    },

    // =========================================================================
    // Watch operations - Real-time key change notifications
    // =========================================================================
    /// Create a watch on keys matching a prefix.
    WatchCreate {
        prefix: String,
        start_index: u64,
        include_prev_value: bool,
    },

    /// Cancel an active watch.
    WatchCancel { watch_id: u64 },

    /// Get current watch status and statistics.
    WatchStatus { watch_id: Option<u64> },

    // =========================================================================
    // Lease operations - Time-based resource management
    // =========================================================================
    /// Grant a new lease with specified TTL.
    LeaseGrant {
        ttl_seconds: u32,
        lease_id: Option<u64>,
    },

    /// Revoke a lease and delete all attached keys.
    LeaseRevoke { lease_id: u64 },

    /// Refresh a lease's TTL (keepalive).
    LeaseKeepalive { lease_id: u64 },

    /// Get lease information including TTL and attached keys.
    LeaseTimeToLive { lease_id: u64, include_keys: bool },

    /// List all active leases.
    LeaseList,

    /// Write a key attached to a lease.
    WriteKeyWithLease {
        key: String,
        value: Vec<u8>,
        lease_id: u64,
    },

    // =========================================================================
    // Distributed Barrier operations
    // =========================================================================
    /// Enter a barrier, waiting until all participants arrive.
    BarrierEnter {
        name: String,
        participant_id: String,
        required_count: u32,
        timeout_ms: u64,
    },

    /// Leave a barrier after work is complete.
    BarrierLeave {
        name: String,
        participant_id: String,
        timeout_ms: u64,
    },

    /// Query barrier status without blocking.
    BarrierStatus { name: String },

    // =========================================================================
    // Distributed Semaphore operations
    // =========================================================================
    /// Acquire permits from a semaphore, blocking until available.
    SemaphoreAcquire {
        name: String,
        holder_id: String,
        permits: u32,
        capacity: u32,
        ttl_ms: u64,
        timeout_ms: u64,
    },

    /// Try to acquire permits without blocking.
    SemaphoreTryAcquire {
        name: String,
        holder_id: String,
        permits: u32,
        capacity: u32,
        ttl_ms: u64,
    },

    /// Release permits back to a semaphore.
    SemaphoreRelease {
        name: String,
        holder_id: String,
        permits: u32,
    },

    /// Query semaphore status.
    SemaphoreStatus { name: String },

    // =========================================================================
    // Read-Write Lock operations
    // =========================================================================
    /// Acquire read lock (blocking until available or timeout).
    RWLockAcquireRead {
        name: String,
        holder_id: String,
        ttl_ms: u64,
        timeout_ms: u64,
    },

    /// Try to acquire read lock (non-blocking).
    RWLockTryAcquireRead {
        name: String,
        holder_id: String,
        ttl_ms: u64,
    },

    /// Acquire write lock (blocking until available or timeout).
    RWLockAcquireWrite {
        name: String,
        holder_id: String,
        ttl_ms: u64,
        timeout_ms: u64,
    },

    /// Try to acquire write lock (non-blocking).
    RWLockTryAcquireWrite {
        name: String,
        holder_id: String,
        ttl_ms: u64,
    },

    /// Release read lock.
    RWLockReleaseRead { name: String, holder_id: String },

    /// Release write lock.
    RWLockReleaseWrite {
        name: String,
        holder_id: String,
        fencing_token: u64,
    },

    /// Downgrade write lock to read lock.
    RWLockDowngrade {
        name: String,
        holder_id: String,
        fencing_token: u64,
        ttl_ms: u64,
    },

    /// Query RWLock status.
    RWLockStatus { name: String },

    // =========================================================================
    // Queue operations
    // =========================================================================
    /// Create a distributed queue.
    QueueCreate {
        queue_name: String,
        default_visibility_timeout_ms: Option<u64>,
        default_ttl_ms: Option<u64>,
        max_delivery_attempts: Option<u32>,
    },

    /// Delete a queue and all its items.
    QueueDelete { queue_name: String },

    /// Enqueue an item to a distributed queue.
    QueueEnqueue {
        queue_name: String,
        payload: Vec<u8>,
        ttl_ms: Option<u64>,
        message_group_id: Option<String>,
        deduplication_id: Option<String>,
    },

    /// Enqueue multiple items in a batch.
    QueueEnqueueBatch {
        queue_name: String,
        items: Vec<QueueEnqueueItem>,
    },

    /// Dequeue items from a queue with visibility timeout (non-blocking).
    QueueDequeue {
        queue_name: String,
        consumer_id: String,
        max_items: u32,
        visibility_timeout_ms: u64,
    },

    /// Dequeue items with blocking wait.
    QueueDequeueWait {
        queue_name: String,
        consumer_id: String,
        max_items: u32,
        visibility_timeout_ms: u64,
        wait_timeout_ms: u64,
    },

    /// Peek at items without removing them.
    QueuePeek { queue_name: String, max_items: u32 },

    /// Acknowledge successful processing of an item.
    QueueAck {
        queue_name: String,
        receipt_handle: String,
    },

    /// Negative acknowledge - return to queue or move to DLQ.
    QueueNack {
        queue_name: String,
        receipt_handle: String,
        move_to_dlq: bool,
        error_message: Option<String>,
    },

    /// Extend visibility timeout for a pending item.
    QueueExtendVisibility {
        queue_name: String,
        receipt_handle: String,
        additional_timeout_ms: u64,
    },

    /// Get queue status.
    QueueStatus { queue_name: String },

    /// Get items from dead letter queue.
    QueueGetDLQ { queue_name: String, max_items: u32 },

    /// Move DLQ item back to main queue.
    QueueRedriveDLQ { queue_name: String, item_id: u64 },

    // =========================================================================
    // Service Registry operations
    // =========================================================================
    /// Register a service instance.
    ServiceRegister {
        service_name: String,
        instance_id: String,
        address: String,
        version: String,
        tags: String,
        weight: u32,
        custom_metadata: String,
        ttl_ms: u64,
        lease_id: Option<u64>,
    },

    /// Deregister a service instance.
    ServiceDeregister {
        service_name: String,
        instance_id: String,
        fencing_token: u64,
    },

    /// Discover service instances.
    ServiceDiscover {
        service_name: String,
        healthy_only: bool,
        tags: String,
        version_prefix: Option<String>,
        limit: Option<u32>,
    },

    /// Discover services by name prefix.
    ServiceList { prefix: String, limit: u32 },

    /// Get a specific service instance.
    ServiceGetInstance {
        service_name: String,
        instance_id: String,
    },

    /// Send heartbeat to renew TTL.
    ServiceHeartbeat {
        service_name: String,
        instance_id: String,
        fencing_token: u64,
    },

    /// Update instance health status.
    ServiceUpdateHealth {
        service_name: String,
        instance_id: String,
        fencing_token: u64,
        status: String,
    },

    /// Update instance metadata.
    ServiceUpdateMetadata {
        service_name: String,
        instance_id: String,
        fencing_token: u64,
        version: Option<String>,
        tags: Option<String>,
        weight: Option<u32>,
        custom_metadata: Option<String>,
    },

    // =========================================================================
    // DNS operations - Record and zone management
    // =========================================================================
    /// Set a DNS record.
    DnsSetRecord {
        domain: String,
        record_type: String,
        ttl_seconds: u32,
        data_json: String,
    },

    /// Get a DNS record.
    DnsGetRecord { domain: String, record_type: String },

    /// Get all DNS records for a domain.
    DnsGetRecords { domain: String },

    /// Delete a DNS record.
    DnsDeleteRecord { domain: String, record_type: String },

    /// Resolve a domain (with wildcard matching).
    DnsResolve { domain: String, record_type: String },

    /// Scan DNS records by prefix.
    DnsScanRecords { prefix: String, limit: u32 },

    /// Create or update a DNS zone.
    DnsSetZone {
        name: String,
        enabled: bool,
        default_ttl: u32,
        description: Option<String>,
    },

    /// Get a DNS zone.
    DnsGetZone { name: String },

    /// List all DNS zones.
    DnsListZones,

    /// Delete a DNS zone.
    DnsDeleteZone { name: String, delete_records: bool },

    // =========================================================================
    // Sharding operations - Topology management
    // =========================================================================
    /// Get the current shard topology.
    GetTopology { client_version: Option<u64> },

    // =========================================================================
    // Forge operations - Decentralized Git
    // =========================================================================
    /// Create a new repository.
    ForgeCreateRepo {
        name: String,
        description: Option<String>,
        default_branch: Option<String>,
    },

    /// Get repository information by ID.
    ForgeGetRepo { repo_id: String },

    /// List repositories.
    ForgeListRepos {
        limit: Option<u32>,
        offset: Option<u32>,
    },

    /// Store a blob (file content).
    ForgeStoreBlob { repo_id: String, content: Vec<u8> },

    /// Get a blob by hash.
    ForgeGetBlob { hash: String },

    /// Create a tree (directory).
    ForgeCreateTree { repo_id: String, entries_json: String },

    /// Get a tree by hash.
    ForgeGetTree { hash: String },

    /// Create a commit.
    ForgeCommit {
        repo_id: String,
        tree: String,
        parents: Vec<String>,
        message: String,
    },

    /// Get a commit by hash.
    ForgeGetCommit { hash: String },

    /// Get commit history from a ref.
    ForgeLog {
        repo_id: String,
        ref_name: Option<String>,
        limit: Option<u32>,
    },

    /// Get a ref value.
    ForgeGetRef { repo_id: String, ref_name: String },

    /// Set a ref value.
    ForgeSetRef {
        repo_id: String,
        ref_name: String,
        hash: String,
        signer: Option<String>,
        signature: Option<String>,
        timestamp_ms: Option<u64>,
    },

    /// Delete a ref.
    ForgeDeleteRef { repo_id: String, ref_name: String },

    /// Compare-and-set a ref (for safe concurrent updates).
    ForgeCasRef {
        repo_id: String,
        ref_name: String,
        expected: Option<String>,
        new_hash: String,
        signer: Option<String>,
        signature: Option<String>,
        timestamp_ms: Option<u64>,
    },

    /// List branches in a repository.
    ForgeListBranches { repo_id: String },

    /// List tags in a repository.
    ForgeListTags { repo_id: String },

    /// Create an issue.
    ForgeCreateIssue {
        repo_id: String,
        title: String,
        body: String,
        labels: Vec<String>,
    },

    /// List issues in a repository.
    ForgeListIssues {
        repo_id: String,
        state: Option<String>,
        limit: Option<u32>,
    },

    /// Get issue details.
    ForgeGetIssue { repo_id: String, issue_id: String },

    /// Add a comment to an issue.
    ForgeCommentIssue {
        repo_id: String,
        issue_id: String,
        body: String,
    },

    /// Close an issue.
    ForgeCloseIssue {
        repo_id: String,
        issue_id: String,
        reason: Option<String>,
    },

    /// Reopen an issue.
    ForgeReopenIssue { repo_id: String, issue_id: String },

    /// Create a patch (pull request equivalent).
    ForgeCreatePatch {
        repo_id: String,
        title: String,
        description: String,
        base: String,
        head: String,
    },

    /// List patches in a repository.
    ForgeListPatches {
        repo_id: String,
        state: Option<String>,
        limit: Option<u32>,
    },

    /// Get patch details.
    ForgeGetPatch { repo_id: String, patch_id: String },

    /// Update patch head (push new commits).
    ForgeUpdatePatch {
        repo_id: String,
        patch_id: String,
        head: String,
        message: Option<String>,
    },

    /// Approve a patch.
    ForgeApprovePatch {
        repo_id: String,
        patch_id: String,
        commit: String,
        message: Option<String>,
    },

    /// Merge a patch.
    ForgeMergePatch {
        repo_id: String,
        patch_id: String,
        merge_commit: String,
    },

    /// Close a patch without merging.
    ForgeClosePatch {
        repo_id: String,
        patch_id: String,
        reason: Option<String>,
    },

    /// Get the delegate key for a repository.
    ForgeGetDelegateKey { repo_id: String },

    // =========================================================================
    // Federation operations - Cross-cluster discovery and sync
    // =========================================================================
    /// Get federation status.
    GetFederationStatus,

    /// List discovered clusters.
    ListDiscoveredClusters,

    /// Get details about a discovered cluster.
    GetDiscoveredCluster { cluster_key: String },

    /// Trust a cluster.
    TrustCluster { cluster_key: String },

    /// Untrust a cluster.
    UntrustCluster { cluster_key: String },

    /// Federate a repository.
    FederateRepository { repo_id: String, mode: String },

    /// List federated repositories.
    ListFederatedRepositories,

    /// Fetch a federated repository from a remote cluster.
    ForgeFetchFederated {
        federated_id: String,
        remote_cluster: String,
    },

    // =========================================================================
    // Git Bridge operations (for git-remote-aspen)
    // =========================================================================
    /// List refs with their SHA-1 hashes (for git remote helper "list" command).
    GitBridgeListRefs { repo_id: String },

    /// Fetch objects for a ref (for git remote helper "fetch" command).
    GitBridgeFetch {
        repo_id: String,
        want: Vec<String>,
        have: Vec<String>,
    },

    /// Push objects and update refs (for git remote helper "push" command).
    GitBridgePush {
        repo_id: String,
        objects: Vec<GitBridgeObject>,
        refs: Vec<GitBridgeRefUpdate>,
    },

    // =========================================================================
    // Pijul operations - Patch-based version control
    // =========================================================================
    /// Initialize a new Pijul repository.
    #[cfg(feature = "pijul")]
    PijulRepoInit {
        name: String,
        description: Option<String>,
        default_channel: String,
    },

    /// List Pijul repositories.
    #[cfg(feature = "pijul")]
    PijulRepoList { limit: u32 },

    /// Get Pijul repository info.
    #[cfg(feature = "pijul")]
    PijulRepoInfo { repo_id: String },

    /// List channels in a Pijul repository.
    #[cfg(feature = "pijul")]
    PijulChannelList { repo_id: String },

    /// Create a new channel.
    #[cfg(feature = "pijul")]
    PijulChannelCreate { repo_id: String, name: String },

    /// Delete a channel.
    #[cfg(feature = "pijul")]
    PijulChannelDelete { repo_id: String, name: String },

    /// Fork a channel.
    #[cfg(feature = "pijul")]
    PijulChannelFork {
        repo_id: String,
        source: String,
        target: String,
    },

    /// Get channel info.
    #[cfg(feature = "pijul")]
    PijulChannelInfo { repo_id: String, name: String },

    /// Record changes from working directory.
    #[cfg(feature = "pijul")]
    PijulRecord {
        repo_id: String,
        channel: String,
        working_dir: String,
        message: String,
        author_name: Option<String>,
        author_email: Option<String>,
    },

    /// Apply a change to a channel.
    #[cfg(feature = "pijul")]
    PijulApply {
        repo_id: String,
        channel: String,
        change_hash: String,
    },

    /// Unrecord (remove) a change from a channel.
    #[cfg(feature = "pijul")]
    PijulUnrecord {
        repo_id: String,
        channel: String,
        change_hash: String,
    },

    /// Get change log for a channel.
    #[cfg(feature = "pijul")]
    PijulLog {
        repo_id: String,
        channel: String,
        limit: u32,
    },

    /// Checkout pristine state to working directory.
    #[cfg(feature = "pijul")]
    PijulCheckout {
        repo_id: String,
        channel: String,
        output_dir: String,
    },
}

impl ClientRpcRequest {
    /// Convert this request to an authorization Operation for capability checking.
    ///
    /// Returns `None` for requests that don't require authorization.
    pub fn to_operation(&self) -> Option<aspen_auth::Operation> {
        use aspen_auth::Operation;

        match self {
            // Read operations
            Self::ReadKey { key } => Some(Operation::Read { key: key.clone() }),
            Self::ScanKeys { prefix, .. } => Some(Operation::Read { key: prefix.clone() }),

            // Write operations (Write requires both key and value)
            Self::WriteKey { key, value } => Some(Operation::Write {
                key: key.clone(),
                value: value.clone()
            }),
            Self::CompareAndSwapKey { key, new_value, .. } => Some(Operation::Write {
                key: key.clone(),
                value: new_value.clone()
            }),
            Self::CompareAndDeleteKey { key, .. } => Some(Operation::Delete { key: key.clone() }),
            Self::DeleteKey { key } => Some(Operation::Delete { key: key.clone() }),

            // For operations with multiple keys, check against common prefix
            Self::BatchWrite { operations } => {
                // Calculate common prefix of all keys in operations
                let keys: Vec<String> = operations.iter()
                    .filter_map(|op| match op {
                        BatchWriteOperation::Set { key, .. } => Some(key.clone()),
                        BatchWriteOperation::Delete { key } => Some(key.clone()),
                    })
                    .collect();
                let prefix = common_prefix(&keys);
                Some(Operation::Write { key: prefix, value: Vec::new() })
            }

            // Admin operations
            Self::InitCluster => Some(Operation::ClusterAdmin { action: "init".to_string() }),
            Self::AddLearner { .. } => Some(Operation::ClusterAdmin { action: "add_learner".to_string() }),
            Self::ChangeMembership { .. } => Some(Operation::ClusterAdmin { action: "change_membership".to_string() }),
            Self::TriggerSnapshot => Some(Operation::ClusterAdmin { action: "trigger_snapshot".to_string() }),
            Self::AddPeer { .. } => Some(Operation::ClusterAdmin { action: "add_peer".to_string() }),

            // Public/unauthenticated operations
            Self::GetHealth | Self::GetNodeInfo | Self::GetClusterTicket |
            Self::GetTopology { .. } => None,

            // Everything else requires read access
            _ => Some(Operation::Read { key: String::new() }),
        }
    }
}

/// Calculate the common prefix of a list of strings.
///
/// Returns the longest string that is a prefix of all input strings.
fn common_prefix(strings: &[String]) -> String {
    if strings.is_empty() {
        return String::new();
    }

    if strings.len() == 1 {
        return strings[0].clone();
    }

    let mut prefix = String::new();
    let first = &strings[0];

    'outer: for (i, ch) in first.char_indices() {
        for s in &strings[1..] {
            if let Some(other_ch) = s.chars().nth(i) {
                if ch != other_ch {
                    break 'outer;
                }
            } else {
                break 'outer;
            }
        }
        prefix.push(ch);
    }

    prefix
}

// ============================================================================
// Response Types
// ============================================================================

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

    // Blob operation responses
    AddBlobResult(AddBlobResultResponse),
    GetBlobResult(GetBlobResultResponse),
    HasBlobResult(HasBlobResultResponse),
    GetBlobTicketResult(GetBlobTicketResultResponse),
    ListBlobsResult(ListBlobsResultResponse),
    ProtectBlobResult(ProtectBlobResultResponse),
    UnprotectBlobResult(UnprotectBlobResultResponse),
    DeleteBlobResult(DeleteBlobResultResponse),
    DownloadBlobResult(DownloadBlobResultResponse),
    DownloadBlobByHashResult(DownloadBlobResultResponse),
    DownloadBlobByProviderResult(DownloadBlobResultResponse),
    GetBlobStatusResult(GetBlobStatusResultResponse),

    // Docs operation responses
    DocsSetResult(DocsSetResultResponse),
    DocsGetResult(DocsGetResultResponse),
    DocsDeleteResult(DocsDeleteResultResponse),
    DocsListResult(DocsListResultResponse),
    DocsStatusResult(DocsStatusResultResponse),

    // Peer cluster operation responses
    AddPeerClusterResult(AddPeerClusterResultResponse),
    RemovePeerClusterResult(RemovePeerClusterResultResponse),
    ListPeerClustersResult(ListPeerClustersResultResponse),
    PeerClusterStatus(PeerClusterStatusResponse),
    UpdatePeerClusterFilterResult(UpdatePeerClusterFilterResultResponse),
    UpdatePeerClusterPriorityResult(UpdatePeerClusterPriorityResultResponse),
    SetPeerClusterEnabledResult(SetPeerClusterEnabledResultResponse),
    KeyOriginResult(KeyOriginResultResponse),

    // SQL query response
    SqlResult(SqlResultResponse),

    // Coordination primitive responses
    LockResult(LockResultResponse),
    CounterResult(CounterResultResponse),
    SignedCounterResult(SignedCounterResultResponse),
    SequenceResult(SequenceResultResponse),
    RateLimiterResult(RateLimiterResultResponse),

    // Batch operation responses
    BatchReadResult(BatchReadResultResponse),
    BatchWriteResult(BatchWriteResultResponse),
    ConditionalBatchWriteResult(ConditionalBatchWriteResultResponse),

    // Watch operation responses
    WatchCreateResult(WatchCreateResultResponse),
    WatchCancelResult(WatchCancelResultResponse),
    WatchStatusResult(WatchStatusResultResponse),
    WatchEvent(WatchEventResponse),

    // Lease operation responses
    LeaseGrantResult(LeaseGrantResultResponse),
    LeaseRevokeResult(LeaseRevokeResultResponse),
    LeaseKeepaliveResult(LeaseKeepaliveResultResponse),
    LeaseTimeToLiveResult(LeaseTimeToLiveResultResponse),
    LeaseListResult(LeaseListResultResponse),

    // Barrier responses
    BarrierEnterResult(BarrierResultResponse),
    BarrierLeaveResult(BarrierResultResponse),
    BarrierStatusResult(BarrierResultResponse),

    // Semaphore responses
    SemaphoreAcquireResult(SemaphoreResultResponse),
    SemaphoreTryAcquireResult(SemaphoreResultResponse),
    SemaphoreReleaseResult(SemaphoreResultResponse),
    SemaphoreStatusResult(SemaphoreResultResponse),

    // Read-Write Lock responses
    RWLockAcquireReadResult(RWLockResultResponse),
    RWLockTryAcquireReadResult(RWLockResultResponse),
    RWLockAcquireWriteResult(RWLockResultResponse),
    RWLockTryAcquireWriteResult(RWLockResultResponse),
    RWLockReleaseReadResult(RWLockResultResponse),
    RWLockReleaseWriteResult(RWLockResultResponse),
    RWLockDowngradeResult(RWLockResultResponse),
    RWLockStatusResult(RWLockResultResponse),

    // Queue responses
    QueueCreateResult(QueueCreateResultResponse),
    QueueDeleteResult(QueueDeleteResultResponse),
    QueueEnqueueResult(QueueEnqueueResultResponse),
    QueueEnqueueBatchResult(QueueEnqueueBatchResultResponse),
    QueueDequeueResult(QueueDequeueResultResponse),
    QueuePeekResult(QueuePeekResultResponse),
    QueueAckResult(QueueAckResultResponse),
    QueueNackResult(QueueNackResultResponse),
    QueueExtendVisibilityResult(QueueExtendVisibilityResultResponse),
    QueueStatusResult(QueueStatusResultResponse),
    QueueGetDLQResult(QueueGetDLQResultResponse),
    QueueRedriveDLQResult(QueueRedriveDLQResultResponse),

    // Service Registry responses
    ServiceRegisterResult(ServiceRegisterResultResponse),
    ServiceDeregisterResult(ServiceDeregisterResultResponse),
    ServiceDiscoverResult(ServiceDiscoverResultResponse),
    ServiceListResult(ServiceListResultResponse),
    ServiceGetInstanceResult(ServiceGetInstanceResultResponse),
    ServiceHeartbeatResult(ServiceHeartbeatResultResponse),
    ServiceUpdateHealthResult(ServiceUpdateHealthResultResponse),
    ServiceUpdateMetadataResult(ServiceUpdateMetadataResultResponse),

    // DNS responses
    DnsSetRecordResult(DnsRecordResultResponse),
    DnsGetRecordResult(DnsRecordResultResponse),
    DnsGetRecordsResult(DnsRecordsResultResponse),
    DnsDeleteRecordResult(DnsDeleteRecordResultResponse),
    DnsResolveResult(DnsRecordsResultResponse),
    DnsScanRecordsResult(DnsRecordsResultResponse),
    DnsSetZoneResult(DnsZoneResultResponse),
    DnsGetZoneResult(DnsZoneResultResponse),
    DnsListZonesResult(DnsZonesResultResponse),
    DnsDeleteZoneResult(DnsDeleteZoneResultResponse),

    // Sharding responses
    TopologyResult(TopologyResultResponse),

    // Forge responses (decentralized git)
    ForgeRepoResult(ForgeRepoResultResponse),
    ForgeRepoListResult(ForgeRepoListResultResponse),
    ForgeBlobResult(ForgeBlobResultResponse),
    ForgeTreeResult(ForgeTreeResultResponse),
    ForgeCommitResult(ForgeCommitResultResponse),
    ForgeLogResult(ForgeLogResultResponse),
    ForgeRefResult(ForgeRefResultResponse),
    ForgeRefListResult(ForgeRefListResultResponse),
    ForgeIssueResult(ForgeIssueResultResponse),
    ForgeIssueListResult(ForgeIssueListResultResponse),
    ForgePatchResult(ForgePatchResultResponse),
    ForgePatchListResult(ForgePatchListResultResponse),
    ForgeOperationResult(ForgeOperationResultResponse),
    ForgeKeyResult(ForgeKeyResultResponse),

    // Federation operation responses
    FederationStatus(FederationStatusResponse),
    DiscoveredClusters(DiscoveredClustersResponse),
    DiscoveredCluster(DiscoveredClusterResponse),
    TrustClusterResult(TrustClusterResultResponse),
    UntrustClusterResult(UntrustClusterResultResponse),
    FederateRepositoryResult(FederateRepositoryResultResponse),
    FederatedRepositories(FederatedRepositoriesResponse),
    ForgeFetchResult(ForgeFetchFederatedResultResponse),

    // Git Bridge responses (for git-remote-aspen)
    GitBridgeListRefs(GitBridgeListRefsResponse),
    GitBridgeFetch(GitBridgeFetchResponse),
    GitBridgePush(GitBridgePushResponse),

    // Pijul responses
    #[cfg(feature = "pijul")]
    PijulRepoResult(PijulRepoResponse),
    #[cfg(feature = "pijul")]
    PijulRepoListResult(PijulRepoListResponse),
    #[cfg(feature = "pijul")]
    PijulChannelResult(PijulChannelResponse),
    #[cfg(feature = "pijul")]
    PijulChannelListResult(PijulChannelListResponse),
    #[cfg(feature = "pijul")]
    PijulRecordResult(PijulRecordResponse),
    #[cfg(feature = "pijul")]
    PijulApplyResult(PijulApplyResponse),
    #[cfg(feature = "pijul")]
    PijulUnrecordResult(PijulUnrecordResponse),
    #[cfg(feature = "pijul")]
    PijulLogResult(PijulLogResponse),
    #[cfg(feature = "pijul")]
    PijulCheckoutResult(PijulCheckoutResponse),
    #[cfg(feature = "pijul")]
    PijulSuccess,
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

// ============================================================================
// Helper Structs
// ============================================================================

// Include all response helper types
include!("rpc_types.rs");

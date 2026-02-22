// Response and helper types for Client RPC protocol.
// This file is included in rpc.rs.
//
// Note: Most types in this file are unused in the current implementation
// but are kept for API completeness and future features.

/// Health status response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub node_id: u64,
    pub raft_node_id: Option<u64>,
    pub uptime_seconds: u64,
}

/// Raft metrics response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaftMetricsResponse {
    pub node_id: u64,
    pub state: String,
    pub current_leader: Option<u64>,
    pub current_term: u64,
    pub last_log_index: Option<u64>,
    pub last_applied_index: Option<u64>,
    pub snapshot_index: Option<u64>,
    pub replication: Option<Vec<ReplicationProgress>>,
}

/// Replication progress for a single node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicationProgress {
    pub node_id: u64,
    pub matched_index: Option<u64>,
}

/// Node information response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInfoResponse {
    pub node_id: u64,
    pub endpoint_addr: String,
}

/// Cluster ticket response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterTicketResponse {
    pub ticket: String,
    pub topic_id: String,
    pub cluster_id: String,
    pub endpoint_id: String,
    pub bootstrap_peers: Option<u32>,
}

/// Init cluster result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitResultResponse {
    pub success: bool,
    pub error: Option<String>,
}

/// Read key result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadResultResponse {
    pub value: Option<Vec<u8>>,
    pub found: bool,
    pub error: Option<String>,
}

/// Write key result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WriteResultResponse {
    pub success: bool,
    pub error: Option<String>,
}

/// Compare-and-swap result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompareAndSwapResultResponse {
    pub success: bool,
    pub actual_value: Option<Vec<u8>>,
    pub error: Option<String>,
}

/// Snapshot trigger result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotResultResponse {
    pub success: bool,
    pub snapshot_index: Option<u64>,
    pub error: Option<String>,
}

/// Add learner result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddLearnerResultResponse {
    pub success: bool,
    pub error: Option<String>,
}

/// Change membership result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeMembershipResultResponse {
    pub success: bool,
    pub error: Option<String>,
}

/// Error response for failed requests.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub code: String,
    pub message: String,
}

/// Cluster state response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterStateResponse {
    pub nodes: Vec<NodeDescriptor>,
    pub leader_id: Option<u64>,
    pub this_node_id: u64,
}

/// Descriptor for a node in the cluster.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeDescriptor {
    pub node_id: u64,
    pub endpoint_addr: String,
    pub is_voter: bool,
    pub is_learner: bool,
    pub is_leader: bool,
}

/// Delete key result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteResultResponse {
    pub key: String,
    pub deleted: bool,
    pub error: Option<String>,
}

/// Scan keys result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanResultResponse {
    pub entries: Vec<ScanEntry>,
    pub count: u32,
    pub is_truncated: bool,
    pub continuation_token: Option<String>,
    pub error: Option<String>,
}

/// Single entry from scan operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanEntry {
    pub key: String,
    pub value: String,
    #[serde(default)]
    pub version: u64,
    #[serde(default)]
    pub create_revision: u64,
    #[serde(default)]
    pub mod_revision: u64,
}

/// Prometheus metrics response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsResponse {
    pub prometheus_text: String,
}

/// Promote learner result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromoteLearnerResultResponse {
    pub success: bool,
    pub learner_id: u64,
    pub previous_voters: Vec<u64>,
    pub new_voters: Vec<u64>,
    pub message: String,
    pub error: Option<String>,
}

/// Checkpoint WAL result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointWalResultResponse {
    pub success: bool,
    pub pages_checkpointed: Option<u32>,
    pub wal_size_before_bytes: Option<u64>,
    pub wal_size_after_bytes: Option<u64>,
    pub error: Option<String>,
}

/// List vaults response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultListResponse {
    pub vaults: Vec<VaultInfo>,
    pub error: Option<String>,
}

/// Information about a vault.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultInfo {
    pub name: String,
    pub key_count: u64,
}

/// Vault keys response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultKeysResponse {
    pub vault: String,
    pub keys: Vec<VaultKeyValue>,
    pub error: Option<String>,
}

/// Key-value pair within a vault.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultKeyValue {
    pub key: String,
    pub value: String,
}

/// Add peer result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddPeerResultResponse {
    pub success: bool,
    pub error: Option<String>,
}

/// Client ticket response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientTicketResponse {
    pub ticket: String,
    pub cluster_id: String,
    pub access: String,
    pub priority: u32,
    pub endpoint_id: String,
    pub error: Option<String>,
}

/// Docs ticket response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocsTicketResponse {
    pub ticket: String,
    pub cluster_id: String,
    pub namespace_id: String,
    pub read_write: bool,
    pub priority: u8,
    pub endpoint_id: String,
    pub error: Option<String>,
}

// Blob operation response types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddBlobResultResponse {
    pub success: bool,
    pub hash: Option<String>,
    #[serde(rename = "size")]
    pub size_bytes: Option<u64>,
    pub was_new: Option<bool>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetBlobResultResponse {
    pub found: bool,
    pub data: Option<Vec<u8>>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HasBlobResultResponse {
    pub exists: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetBlobTicketResultResponse {
    pub success: bool,
    pub ticket: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlobListEntry {
    pub hash: String,
    #[serde(rename = "size")]
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListBlobsResultResponse {
    pub blobs: Vec<BlobListEntry>,
    pub count: u32,
    pub has_more: bool,
    pub continuation_token: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtectBlobResultResponse {
    pub success: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnprotectBlobResultResponse {
    pub success: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteBlobResultResponse {
    pub success: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadBlobResultResponse {
    pub success: bool,
    pub hash: Option<String>,
    #[serde(rename = "size")]
    pub size_bytes: Option<u64>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetBlobStatusResultResponse {
    pub found: bool,
    pub hash: Option<String>,
    #[serde(rename = "size")]
    pub size_bytes: Option<u64>,
    pub complete: Option<bool>,
    pub tags: Option<Vec<String>>,
    pub error: Option<String>,
}

// Docs operation response types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocsSetResultResponse {
    pub success: bool,
    pub key: Option<String>,
    #[serde(rename = "size")]
    pub size_bytes: Option<u64>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocsGetResultResponse {
    pub found: bool,
    pub value: Option<Vec<u8>>,
    #[serde(rename = "size")]
    pub size_bytes: Option<u64>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocsDeleteResultResponse {
    pub success: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocsListEntry {
    pub key: String,
    #[serde(rename = "size")]
    pub size_bytes: u64,
    pub hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocsListResultResponse {
    pub entries: Vec<DocsListEntry>,
    pub count: u32,
    pub has_more: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocsStatusResultResponse {
    #[serde(rename = "enabled")]
    pub is_enabled: bool,
    pub namespace_id: Option<String>,
    pub author_id: Option<String>,
    pub entry_count: Option<u64>,
    #[serde(rename = "replica_open")]
    pub is_replica_open: Option<bool>,
    pub error: Option<String>,
}

// Peer cluster operation response types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddPeerClusterResultResponse {
    pub success: bool,
    pub cluster_id: Option<String>,
    pub priority: Option<u32>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemovePeerClusterResultResponse {
    pub success: bool,
    pub cluster_id: String,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListPeerClustersResultResponse {
    pub peers: Vec<PeerClusterInfo>,
    pub count: u32,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerClusterInfo {
    pub cluster_id: String,
    pub name: String,
    pub state: String,
    pub priority: u32,
    #[serde(rename = "enabled")]
    pub is_enabled: bool,
    pub sync_count: u64,
    pub failure_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerClusterStatusResponse {
    pub found: bool,
    pub cluster_id: String,
    pub state: String,
    #[serde(rename = "syncing")]
    pub is_syncing: bool,
    pub entries_received: u64,
    pub entries_imported: u64,
    pub entries_skipped: u64,
    pub entries_filtered: u64,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdatePeerClusterFilterResultResponse {
    pub success: bool,
    pub cluster_id: String,
    pub filter_type: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdatePeerClusterPriorityResultResponse {
    pub success: bool,
    pub cluster_id: String,
    pub previous_priority: Option<u32>,
    pub new_priority: Option<u32>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetPeerClusterEnabledResultResponse {
    pub success: bool,
    pub cluster_id: String,
    pub enabled: Option<bool>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyOriginResultResponse {
    pub found: bool,
    pub key: String,
    pub cluster_id: Option<String>,
    pub priority: Option<u32>,
    pub timestamp_secs: Option<u64>,
    pub is_local: Option<bool>,
}

// SQL query response types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SqlCellValue {
    Null,
    Integer(i64),
    Real(f64),
    Text(String),
    Blob(String),
}

impl SqlCellValue {
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SqlResultResponse {
    pub success: bool,
    pub columns: Option<Vec<String>>,
    pub rows: Option<Vec<Vec<SqlCellValue>>>,
    pub row_count: Option<u32>,
    pub is_truncated: Option<bool>,
    pub execution_time_ms: Option<u64>,
    pub error: Option<String>,
}

// Coordination primitive response types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockResultResponse {
    pub success: bool,
    pub fencing_token: Option<u64>,
    pub holder_id: Option<String>,
    pub deadline_ms: Option<u64>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CounterResultResponse {
    pub success: bool,
    pub value: Option<u64>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedCounterResultResponse {
    pub success: bool,
    pub value: Option<i64>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SequenceResultResponse {
    pub success: bool,
    pub value: Option<u64>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimiterResultResponse {
    pub success: bool,
    pub tokens_remaining: Option<u64>,
    pub retry_after_ms: Option<u64>,
    pub error: Option<String>,
}

// Batch operation types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum BatchWriteOperation {
    Set { key: String, value: Vec<u8> },
    Delete { key: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum BatchCondition {
    ValueEquals { key: String, expected: Vec<u8> },
    KeyExists { key: String },
    KeyNotExists { key: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchReadResultResponse {
    pub success: bool,
    pub values: Option<Vec<Option<Vec<u8>>>>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchWriteResultResponse {
    pub success: bool,
    pub operations_applied: Option<u32>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConditionalBatchWriteResultResponse {
    pub success: bool,
    pub conditions_met: bool,
    pub operations_applied: Option<u32>,
    pub failed_condition_index: Option<u32>,
    pub failed_condition_reason: Option<String>,
    pub error: Option<String>,
}

// Watch operation response types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchCreateResultResponse {
    pub success: bool,
    pub watch_id: Option<u64>,
    pub current_index: Option<u64>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchCancelResultResponse {
    pub success: bool,
    pub watch_id: u64,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchStatusResultResponse {
    pub success: bool,
    pub watches: Option<Vec<WatchInfo>>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchInfo {
    pub watch_id: u64,
    pub prefix: String,
    pub last_sent_index: u64,
    pub events_sent: u64,
    pub created_at_ms: u64,
    pub should_include_prev_value: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchEventResponse {
    pub watch_id: u64,
    pub index: u64,
    pub term: u64,
    pub committed_at_ms: u64,
    pub events: Vec<WatchKeyEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchKeyEvent {
    pub event_type: WatchEventType,
    pub key: String,
    pub value: Option<Vec<u8>>,
    pub prev_value: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WatchEventType {
    Put,
    Delete,
}

// Lease operation response types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaseGrantResultResponse {
    pub success: bool,
    pub lease_id: Option<u64>,
    pub ttl_seconds: Option<u32>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaseRevokeResultResponse {
    pub success: bool,
    pub keys_deleted: Option<u32>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaseKeepaliveResultResponse {
    pub success: bool,
    pub lease_id: Option<u64>,
    pub ttl_seconds: Option<u32>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaseTimeToLiveResultResponse {
    pub success: bool,
    pub lease_id: Option<u64>,
    pub granted_ttl_seconds: Option<u32>,
    pub remaining_ttl_seconds: Option<u32>,
    pub keys: Option<Vec<String>>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaseListResultResponse {
    pub success: bool,
    pub leases: Option<Vec<LeaseInfo>>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaseInfo {
    pub lease_id: u64,
    pub granted_ttl_seconds: u32,
    pub remaining_ttl_seconds: u32,
    pub attached_keys: u32,
}

// Barrier types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BarrierResultResponse {
    pub success: bool,
    pub current_count: Option<u32>,
    pub required_count: Option<u32>,
    pub phase: Option<String>,
    pub error: Option<String>,
}

// Semaphore types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemaphoreResultResponse {
    pub success: bool,
    pub permits_acquired: Option<u32>,
    pub available: Option<u32>,
    pub capacity_permits: Option<u32>,
    pub retry_after_ms: Option<u64>,
    pub error: Option<String>,
}

// Read-Write Lock types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RWLockResultResponse {
    pub success: bool,
    pub mode: Option<String>,
    pub fencing_token: Option<u64>,
    pub deadline_ms: Option<u64>,
    pub reader_count: Option<u32>,
    pub writer_holder: Option<String>,
    pub error: Option<String>,
}

// Queue types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueEnqueueItem {
    pub payload: Vec<u8>,
    pub ttl_ms: Option<u64>,
    pub message_group_id: Option<String>,
    pub deduplication_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueCreateResultResponse {
    pub success: bool,
    pub created: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueDeleteResultResponse {
    pub success: bool,
    pub items_deleted: Option<u64>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueEnqueueResultResponse {
    pub success: bool,
    pub item_id: Option<u64>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueEnqueueBatchResultResponse {
    pub success: bool,
    pub item_ids: Vec<u64>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueDequeuedItemResponse {
    pub item_id: u64,
    pub payload: Vec<u8>,
    pub receipt_handle: String,
    pub delivery_attempts: u32,
    pub enqueued_at_ms: u64,
    pub visibility_deadline_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueDequeueResultResponse {
    pub success: bool,
    pub items: Vec<QueueDequeuedItemResponse>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueItemResponse {
    pub item_id: u64,
    pub payload: Vec<u8>,
    pub enqueued_at_ms: u64,
    pub expires_at_ms: u64,
    pub delivery_attempts: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueuePeekResultResponse {
    pub success: bool,
    pub items: Vec<QueueItemResponse>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueAckResultResponse {
    pub success: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueNackResultResponse {
    pub success: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueExtendVisibilityResultResponse {
    pub success: bool,
    pub new_deadline_ms: Option<u64>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueStatusResultResponse {
    pub success: bool,
    pub exists: bool,
    pub visible_count: Option<u64>,
    pub pending_count: Option<u64>,
    pub dlq_count: Option<u64>,
    pub total_enqueued: Option<u64>,
    pub total_acked: Option<u64>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueDLQItemResponse {
    pub item_id: u64,
    pub payload: Vec<u8>,
    pub enqueued_at_ms: u64,
    pub delivery_attempts: u32,
    pub reason: String,
    pub moved_at_ms: u64,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueGetDLQResultResponse {
    pub success: bool,
    pub items: Vec<QueueDLQItemResponse>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueRedriveDLQResultResponse {
    pub success: bool,
    pub error: Option<String>,
}

// Service Registry types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceRegisterResultResponse {
    pub success: bool,
    pub fencing_token: Option<u64>,
    pub deadline_ms: Option<u64>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceDeregisterResultResponse {
    pub success: bool,
    pub was_registered: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceInstanceResponse {
    pub instance_id: String,
    pub service_name: String,
    pub address: String,
    pub health_status: String,
    pub version: String,
    pub tags: Vec<String>,
    pub weight: u32,
    pub custom_metadata: String,
    pub registered_at_ms: u64,
    pub last_heartbeat_ms: u64,
    pub deadline_ms: u64,
    pub lease_id: Option<u64>,
    pub fencing_token: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceDiscoverResultResponse {
    pub success: bool,
    pub instances: Vec<ServiceInstanceResponse>,
    pub count: u32,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceListResultResponse {
    pub success: bool,
    pub services: Vec<String>,
    pub count: u32,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceGetInstanceResultResponse {
    pub success: bool,
    pub found: bool,
    pub instance: Option<ServiceInstanceResponse>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceHeartbeatResultResponse {
    pub success: bool,
    pub new_deadline_ms: Option<u64>,
    pub health_status: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceUpdateHealthResultResponse {
    pub success: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceUpdateMetadataResultResponse {
    pub success: bool,
    pub error: Option<String>,
}

// DNS response types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsRecordResponse {
    pub domain: String,
    pub record_type: String,
    pub ttl_seconds: u32,
    pub data_json: String,
    pub updated_at_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsRecordResultResponse {
    pub success: bool,
    pub found: bool,
    pub record: Option<DnsRecordResponse>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsRecordsResultResponse {
    pub success: bool,
    pub records: Vec<DnsRecordResponse>,
    pub count: u32,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsDeleteRecordResultResponse {
    pub success: bool,
    pub deleted: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsZoneResponse {
    pub name: String,
    #[serde(rename = "enabled")]
    pub is_enabled: bool,
    pub default_ttl: u32,
    pub serial: u32,
    pub last_modified_ms: u64,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsZoneResultResponse {
    pub success: bool,
    pub found: bool,
    pub zone: Option<DnsZoneResponse>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsZonesResultResponse {
    pub success: bool,
    pub zones: Vec<DnsZoneResponse>,
    pub count: u32,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsDeleteZoneResultResponse {
    pub success: bool,
    pub deleted: bool,
    pub records_deleted: u32,
    pub error: Option<String>,
}

// Sharding response types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopologyResultResponse {
    pub success: bool,
    pub version: u64,
    pub updated: bool,
    pub topology_data: Option<String>,
    pub shard_count: u32,
    pub error: Option<String>,
}

// Forge response types (decentralized git)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgeRepoInfo {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub default_branch: String,
    pub delegates: Vec<String>,
    pub threshold: u32,
    pub created_at_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgeRepoResultResponse {
    pub success: bool,
    pub repo: Option<ForgeRepoInfo>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgeRepoListResultResponse {
    pub success: bool,
    pub repos: Vec<ForgeRepoInfo>,
    pub count: u32,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgeBlobResultResponse {
    pub success: bool,
    pub hash: Option<String>,
    pub content: Option<Vec<u8>>,
    pub size: Option<u64>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgeTreeEntry {
    pub mode: u32,
    pub name: String,
    pub hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgeTreeResultResponse {
    pub success: bool,
    pub hash: Option<String>,
    pub entries: Option<Vec<ForgeTreeEntry>>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgeCommitInfo {
    pub hash: String,
    pub tree: String,
    pub parents: Vec<String>,
    pub author_name: String,
    pub author_email: Option<String>,
    pub author_key: Option<String>,
    pub message: String,
    pub timestamp_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgeCommitResultResponse {
    pub success: bool,
    pub commit: Option<ForgeCommitInfo>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgeLogResultResponse {
    pub success: bool,
    pub commits: Vec<ForgeCommitInfo>,
    pub count: u32,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgeRefInfo {
    pub name: String,
    pub hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgeRefResultResponse {
    pub success: bool,
    pub found: bool,
    pub ref_info: Option<ForgeRefInfo>,
    pub previous_hash: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgeRefListResultResponse {
    pub success: bool,
    pub refs: Vec<ForgeRefInfo>,
    pub count: u32,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgeCommentInfo {
    pub hash: String,
    pub author: String,
    pub body: String,
    pub timestamp_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgeIssueInfo {
    pub id: String,
    pub title: String,
    pub body: String,
    pub state: String,
    pub labels: Vec<String>,
    pub comment_count: u32,
    pub assignees: Vec<String>,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgeIssueResultResponse {
    pub success: bool,
    pub issue: Option<ForgeIssueInfo>,
    pub comments: Option<Vec<ForgeCommentInfo>>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgeIssueListResultResponse {
    pub success: bool,
    pub issues: Vec<ForgeIssueInfo>,
    pub count: u32,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgePatchRevision {
    pub hash: String,
    pub head: String,
    pub message: Option<String>,
    pub author: String,
    pub timestamp_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgePatchApproval {
    pub author: String,
    pub commit: String,
    pub message: Option<String>,
    pub timestamp_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgePatchInfo {
    pub id: String,
    pub title: String,
    pub description: String,
    pub state: String,
    pub base: String,
    pub head: String,
    pub labels: Vec<String>,
    pub revision_count: u32,
    pub approval_count: u32,
    pub assignees: Vec<String>,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgePatchResultResponse {
    pub success: bool,
    pub patch: Option<ForgePatchInfo>,
    pub comments: Option<Vec<ForgeCommentInfo>>,
    pub revisions: Option<Vec<ForgePatchRevision>>,
    pub approvals: Option<Vec<ForgePatchApproval>>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgePatchListResultResponse {
    pub success: bool,
    pub patches: Vec<ForgePatchInfo>,
    pub count: u32,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgeOperationResultResponse {
    pub success: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgeKeyResultResponse {
    pub success: bool,
    pub public_key: Option<String>,
    pub secret_key: Option<String>,
    pub error: Option<String>,
}

// Federation Response Structs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationStatusResponse {
    #[serde(rename = "enabled")]
    pub is_enabled: bool,
    pub cluster_name: String,
    pub cluster_key: String,
    #[serde(rename = "dht_enabled")]
    pub is_dht_enabled: bool,
    #[serde(rename = "gossip_enabled")]
    pub is_gossip_enabled: bool,
    pub discovered_clusters: u32,
    pub federated_repos: u32,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredClusterInfo {
    pub cluster_key: String,
    pub name: String,
    pub node_count: u32,
    pub capabilities: Vec<String>,
    pub discovered_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredClustersResponse {
    pub clusters: Vec<DiscoveredClusterInfo>,
    pub count: u32,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredClusterResponse {
    pub found: bool,
    pub cluster_key: Option<String>,
    pub name: Option<String>,
    pub node_count: Option<u32>,
    pub capabilities: Option<Vec<String>>,
    pub relay_urls: Option<Vec<String>>,
    pub discovered_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustClusterResultResponse {
    pub success: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UntrustClusterResultResponse {
    pub success: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederateRepositoryResultResponse {
    pub success: bool,
    pub fed_id: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederatedRepoInfo {
    pub repo_id: String,
    pub mode: String,
    pub fed_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederatedRepositoriesResponse {
    pub repositories: Vec<FederatedRepoInfo>,
    pub count: u32,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgeFetchFederatedResultResponse {
    pub success: bool,
    pub remote_cluster: Option<String>,
    pub fetched: u32,
    pub already_present: u32,
    pub errors: Vec<String>,
    pub error: Option<String>,
}

// Git Bridge types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitBridgeObject {
    pub sha1: String,
    pub object_type: String,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitBridgeRefUpdate {
    pub ref_name: String,
    pub old_sha1: String,
    pub new_sha1: String,
    pub force: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitBridgeRefInfo {
    pub ref_name: String,
    pub sha1: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitBridgeListRefsResponse {
    pub success: bool,
    pub refs: Vec<GitBridgeRefInfo>,
    pub head: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitBridgeFetchResponse {
    pub success: bool,
    pub objects: Vec<GitBridgeObject>,
    pub skipped: u32,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitBridgePushResponse {
    pub success: bool,
    pub objects_imported: u32,
    pub objects_skipped: u32,
    pub ref_results: Vec<GitBridgeRefResult>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitBridgeRefResult {
    pub ref_name: String,
    pub success: bool,
    pub error: Option<String>,
}

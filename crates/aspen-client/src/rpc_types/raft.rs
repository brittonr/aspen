// Raft consensus response types.

use serde::Deserialize;
use serde::Serialize;

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

/// Snapshot trigger result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotResultResponse {
    pub success: bool,
    pub snapshot_index: Option<u64>,
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

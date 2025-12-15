//! Raft consensus implementation with direct async APIs.
//!
//! This module provides a production-ready Raft consensus engine built on openraft,
//! using direct async method calls instead of actor message passing for simplicity.
//!
//! # Architecture
//!
//! - **RaftNode**: Direct wrapper around OpenRaft implementing ClusterController and KeyValueStore
//! - **Storage**: Hybrid storage backend (redb for log, SQLite for state machine)
//! - **Network**: IRPC-based network transport over Iroh P2P connections
//! - **Supervisor**: Lightweight task supervision with restart logic
//!
//! # Key Components
//!
//! - `RaftNode`: Direct async API for Raft operations (recommended)
//! - `SqliteStateMachine`: ACID state machine storage with snapshot support
//! - `IrpcRaftNetwork`: Network layer implementing RaftNetworkV2 over IRPC/Iroh
//! - `NodeFailureDetector`: Distinguishes transient errors from node-level failures
//!
//! # Tiger Style Compliance
//!
//! - Bounded resources: MAX_BATCH_SIZE (1000), MAX_SNAPSHOT_SIZE (1GB)
//! - Explicit error handling: snafu-based error types with context
//! - Fixed limits: Connection pools, semaphores for concurrent operations
//!
//! # Usage
//!
//! ```rust,ignore
//! use aspen::cluster::bootstrap::bootstrap_node;
//!
//! // Bootstrap node with direct async architecture
//! let handle = bootstrap_node(config).await?;
//!
//! // Use RaftNode directly for operations
//! handle.raft_node.init(init_request).await?;
//! handle.raft_node.write(write_request).await?;
//! ```

pub mod clock_drift_detection;
pub mod connection_pool;
pub mod constants;
pub mod learner_promotion;
pub mod madsim_network;
pub mod network;
pub mod node;
pub mod node_failure_detection;
pub mod rpc;
pub mod server;
pub mod storage;
pub mod storage_sqlite;
pub mod storage_validation;
pub mod supervisor;
// supervision module removed - was legacy from actor-based architecture
pub mod types;

use std::sync::Arc;

use crate::api::{
    DEFAULT_SCAN_LIMIT, KeyValueStoreError, MAX_SCAN_RESULTS, ScanEntry, ScanRequest, ScanResult,
};
use crate::raft::constants::MAX_BATCH_SIZE;
use crate::raft::storage::InMemoryStateMachine;
use crate::raft::storage_sqlite::SqliteStateMachine;

/// State machine variant that can hold either in-memory or sqlite-backed storage.
///
/// This enum allows the RaftActor to read from the same state machine that
/// receives writes through the Raft core, fixing the NotFound bug where reads
/// queried a placeholder state machine.
#[derive(Clone, Debug)]
pub enum StateMachineVariant {
    InMemory(Arc<InMemoryStateMachine>),
    Sqlite(Arc<SqliteStateMachine>),
}

impl StateMachineVariant {
    /// Read a value from the state machine.
    ///
    /// Returns:
    /// - `Ok(Some(value))` if key exists
    /// - `Ok(None)` if key not found
    /// - `Err(...)` if storage I/O error occurred
    pub async fn get(&self, key: &str) -> Result<Option<String>, KeyValueStoreError> {
        match self {
            Self::InMemory(sm) => Ok(sm.get(key).await),
            Self::Sqlite(sm) => sm.get(key).await.map_err(|err| KeyValueStoreError::Failed {
                reason: format!("sqlite storage read error: {}", err),
            }),
        }
    }

    /// Scan keys matching a prefix with pagination support.
    ///
    /// Returns keys in sorted order (lexicographic).
    /// Tiger Style: Bounded results prevent unbounded memory usage.
    pub async fn scan(&self, request: &ScanRequest) -> Result<ScanResult, KeyValueStoreError> {
        let limit = request
            .limit
            .unwrap_or(DEFAULT_SCAN_LIMIT)
            .min(MAX_SCAN_RESULTS)
            .min(MAX_BATCH_SIZE) as usize;
        let fetch_limit = limit.saturating_add(1);

        // Decode continuation token (format: base64(last_key))
        let start_after = request.continuation_token.as_ref().and_then(|token| {
            base64::Engine::decode(&base64::engine::general_purpose::STANDARD, token)
                .ok()
                .and_then(|bytes| String::from_utf8(bytes).ok())
        });

        match self {
            Self::InMemory(sm) => {
                let kv_pairs = sm.scan_kv_with_prefix_async(&request.prefix).await;
                Self::build_scan_result(kv_pairs, &start_after, limit)
            }
            Self::Sqlite(sm) => {
                let kv_pairs = sm
                    .scan(&request.prefix, start_after.as_deref(), Some(fetch_limit))
                    .await
                    .map_err(|err| KeyValueStoreError::Failed {
                        reason: format!("sqlite storage scan error: {}", err),
                    })?;
                Self::build_scan_result(kv_pairs, &start_after, limit)
            }
        }
    }

    /// Build a ScanResult from key-value pairs with pagination.
    fn build_scan_result(
        mut kv_pairs: Vec<(String, String)>,
        start_after: &Option<String>,
        limit: usize,
    ) -> Result<ScanResult, KeyValueStoreError> {
        // Sort by key for consistent ordering
        kv_pairs.sort_by(|a, b| a.0.cmp(&b.0));

        // Filter to entries after continuation token
        let filtered: Vec<_> = if let Some(after) = start_after {
            kv_pairs
                .into_iter()
                .filter(|(k, _)| k.as_str() > after.as_str())
                .collect()
        } else {
            kv_pairs
        };

        // Check if truncated and build entries
        let is_truncated = filtered.len() > limit;
        let entries: Vec<ScanEntry> = filtered
            .into_iter()
            .take(limit)
            .map(|(key, value)| ScanEntry { key, value })
            .collect();

        // Generate continuation token if truncated
        let continuation_token = if is_truncated {
            entries
                .last()
                .map(|e| base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &e.key))
        } else {
            None
        };

        let count = entries.len() as u32;

        Ok(ScanResult {
            entries,
            count,
            is_truncated,
            continuation_token,
        })
    }
}

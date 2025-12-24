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

/// Authentication and authorization for Raft operations.
pub mod auth;
/// Clock drift detection for monitoring time synchronization between nodes.
pub mod clock_drift_detection;
/// Connection pooling for efficient peer communication.
pub mod connection_pool;
/// Tiger Style resource limits and timeouts.
pub mod constants;
/// Storage integrity checks and validation.
pub mod integrity;
/// Learner to voter promotion logic.
pub mod learner_promotion;
/// Background task for cleaning up expired leases.
pub mod lease_cleanup;
/// Log subscription for observing Raft log changes.
pub mod log_subscriber;
/// Deterministic simulation network for madsim testing.
pub mod madsim_network;
/// Membership watcher for TrustedPeersRegistry synchronization.
pub mod membership_watcher;
/// Raft network layer implementation over Iroh.
pub mod network;
/// RaftNode implementation with direct async APIs.
pub mod node;
/// Node failure detection and health monitoring.
pub mod node_failure_detection;
/// Pure functions for Raft operations (no side effects).
pub mod pure;
/// IRPC service definitions for Raft RPC over Iroh.
pub mod rpc;
/// Raft server protocol handlers.
pub mod server;
/// Storage implementations (in-memory and redb).
pub mod storage;
/// Single-fsync Redb storage (shared log + state machine).
pub mod storage_shared;
/// SQLite-based state machine implementation.
pub mod storage_sqlite;
/// Offline storage validation and integrity checks.
pub mod storage_validation;
/// Task supervision with restart logic.
pub mod supervisor;
/// Background task for cleaning up expired TTL keys.
pub mod ttl_cleanup;
/// Type definitions and configurations for OpenRaft.
pub mod types;
/// Write batching for amortized fsync costs.
pub mod write_batcher;

// Re-export key types for convenience
use std::sync::Arc;

pub use write_batcher::BatchConfig;

use crate::api::DEFAULT_SCAN_LIMIT;
use crate::api::KeyValueStoreError;
use crate::api::KeyValueWithRevision;
use crate::api::MAX_SCAN_RESULTS;
use crate::api::ScanRequest;
use crate::api::ScanResult;
use crate::raft::constants::MAX_BATCH_SIZE;
use crate::raft::storage::InMemoryStateMachine;
use crate::raft::storage_shared::SharedRedbStorage;
use crate::raft::storage_sqlite::SqliteStateMachine;

/// State machine variant that can hold either in-memory, sqlite-backed, or redb-backed storage.
///
/// This enum allows the RaftActor to read from the same state machine that
/// receives writes through the Raft core, fixing the NotFound bug where reads
/// queried a placeholder state machine.
#[derive(Clone, Debug)]
pub enum StateMachineVariant {
    /// In-memory state machine (for testing).
    InMemory(Arc<InMemoryStateMachine>),
    /// SQLite-backed state machine (for production).
    Sqlite(Arc<SqliteStateMachine>),
    /// Redb-backed state machine (single-fsync, optimized for low latency).
    Redb(Arc<SharedRedbStorage>),
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
            Self::Redb(sm) => sm.get(key).map(|opt| opt.map(|e| e.value)).map_err(|err| KeyValueStoreError::Failed {
                reason: format!("redb storage read error: {}", err),
            }),
        }
    }

    /// Scan keys matching a prefix with pagination support.
    ///
    /// Returns keys in sorted order (lexicographic).
    /// Tiger Style: Bounded results prevent unbounded memory usage.
    pub async fn scan(&self, request: &ScanRequest) -> Result<ScanResult, KeyValueStoreError> {
        let limit = request.limit.unwrap_or(DEFAULT_SCAN_LIMIT).min(MAX_SCAN_RESULTS).min(MAX_BATCH_SIZE) as usize;
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
                let kv_pairs =
                    sm.scan(&request.prefix, start_after.as_deref(), Some(fetch_limit)).await.map_err(|err| {
                        KeyValueStoreError::Failed {
                            reason: format!("sqlite storage scan error: {}", err),
                        }
                    })?;
                Self::build_scan_result(kv_pairs, &start_after, limit)
            }
            Self::Redb(sm) => {
                let entries = sm.scan(&request.prefix, start_after.as_deref(), Some(fetch_limit)).map_err(|err| {
                    KeyValueStoreError::Failed {
                        reason: format!("redb storage scan error: {}", err),
                    }
                })?;
                // Convert KeyValueWithRevision to (String, String) pairs
                let kv_pairs: Vec<(String, String)> = entries.into_iter().map(|e| (e.key, e.value)).collect();
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
            kv_pairs.into_iter().filter(|(k, _)| k.as_str() > after.as_str()).collect()
        } else {
            kv_pairs
        };

        // Check if truncated and build entries
        let is_truncated = filtered.len() > limit;
        let entries: Vec<KeyValueWithRevision> = filtered
            .into_iter()
            .take(limit)
            .map(|(key, value)| KeyValueWithRevision {
                key,
                value,
                version: 1,         // In-memory doesn't track versions
                create_revision: 0, // In-memory doesn't track revisions
                mod_revision: 0,
            })
            .collect();

        // Generate continuation token if truncated
        let continuation_token = if is_truncated {
            entries.last().map(|e| base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &e.key))
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

    // =========================================================================
    // Lease Query Methods
    // =========================================================================

    /// Get lease information by ID.
    ///
    /// Returns (granted_ttl_seconds, remaining_ttl_seconds) if the lease exists.
    /// Returns None if the lease doesn't exist, has expired, or storage backend doesn't support
    /// leases.
    pub fn get_lease(&self, lease_id: u64) -> Option<(u32, u32)> {
        match self {
            Self::InMemory(_) => None, // In-memory doesn't track leases
            Self::Sqlite(sm) => sm.get_lease(lease_id).ok().flatten(),
            Self::Redb(_) => None, // TODO: Implement lease support for Redb
        }
    }

    /// Get all keys attached to a lease.
    ///
    /// Returns an empty list if the lease doesn't exist or storage doesn't support leases.
    pub fn get_lease_keys(&self, lease_id: u64) -> Vec<String> {
        match self {
            Self::InMemory(_) => vec![], // In-memory doesn't track leases
            Self::Sqlite(sm) => sm.get_lease_keys(lease_id).unwrap_or_default(),
            Self::Redb(_) => vec![], // TODO: Implement lease support for Redb
        }
    }

    /// List all active (non-expired) leases.
    ///
    /// Returns a list of (lease_id, granted_ttl_seconds, remaining_ttl_seconds).
    pub fn list_leases(&self) -> Vec<(u64, u32, u32)> {
        match self {
            Self::InMemory(_) => vec![], // In-memory doesn't track leases
            Self::Sqlite(sm) => sm.list_leases().unwrap_or_default(),
            Self::Redb(_) => vec![], // TODO: Implement lease support for Redb
        }
    }
}

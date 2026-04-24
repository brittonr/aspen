//! Reusable Redb Raft KV facade.
//!
//! This crate exposes the configuration and trait surface needed by a reusable
//! Redb-backed OpenRaft KV node without depending on Aspen binary configuration,
//! handler registries, dogfood defaults, trust, secrets, SQL, coordination, or a
//! concrete iroh endpoint adapter.

#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

pub use aspen_kv_types::DeleteRequest;
pub use aspen_kv_types::DeleteResult;
pub use aspen_kv_types::KeyValueStoreError;
pub use aspen_kv_types::ReadRequest;
pub use aspen_kv_types::ReadResult;
pub use aspen_kv_types::ScanRequest;
pub use aspen_kv_types::ScanResult;
pub use aspen_kv_types::WriteRequest;
pub use aspen_kv_types::WriteResult;
use aspen_raft_kv_types::NodeId;
use aspen_raft_kv_types::RaftKvMemberInfo;
pub use aspen_raft_kv_types::RaftKvRequest;
pub use aspen_raft_kv_types::RaftKvResponse;
pub use aspen_raft_kv_types::RaftKvStorageError;
pub use aspen_raft_kv_types::RaftKvTypeConfig;
pub use aspen_traits::ClusterController;
pub use aspen_traits::KeyValueStore;
use serde::Deserialize;
use serde::Serialize;
use thiserror::Error;

const DEFAULT_MAX_BATCH_ENTRIES: u32 = 1_000;
const DEFAULT_MAX_SCAN_RESULTS: u32 = 10_000;
const DEFAULT_MAX_KEY_SIZE_BYTES: u32 = 1_024;
const DEFAULT_MAX_VALUE_SIZE_BYTES: u32 = 1_048_576;
const DEFAULT_OPERATION_TIMEOUT_MS: u64 = 10_000;
const DEFAULT_CONNECT_TIMEOUT_MS: u64 = 5_000;
const MIN_RESOURCE_LIMIT: u32 = 1;
const MIN_TIMEOUT_MS: u64 = 1;

/// Bounded resource settings for a reusable Raft KV node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RaftKvResourceLimits {
    /// Maximum operations accepted in one batch request.
    pub max_batch_entries: u32,
    /// Maximum rows returned by one scan request.
    pub max_scan_results: u32,
    /// Maximum key size accepted by the facade, in bytes.
    pub max_key_size_bytes: u32,
    /// Maximum value size accepted by the facade, in bytes.
    pub max_value_size_bytes: u32,
    /// Default operation timeout, in milliseconds.
    pub operation_timeout_ms: u64,
    /// Default peer connection timeout, in milliseconds.
    pub connect_timeout_ms: u64,
}

impl Default for RaftKvResourceLimits {
    fn default() -> Self {
        Self {
            max_batch_entries: DEFAULT_MAX_BATCH_ENTRIES,
            max_scan_results: DEFAULT_MAX_SCAN_RESULTS,
            max_key_size_bytes: DEFAULT_MAX_KEY_SIZE_BYTES,
            max_value_size_bytes: DEFAULT_MAX_VALUE_SIZE_BYTES,
            operation_timeout_ms: DEFAULT_OPERATION_TIMEOUT_MS,
            connect_timeout_ms: DEFAULT_CONNECT_TIMEOUT_MS,
        }
    }
}

/// Storage path configuration for the Redb backend.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RaftKvStorageConfig {
    /// Filesystem path for the Redb database.
    pub path: PathBuf,
    /// Create database if it does not already exist.
    pub should_create: bool,
    /// Sync writes at the storage boundary.
    pub should_sync_writes: bool,
}

impl RaftKvStorageConfig {
    /// Build a Redb storage config with conservative defaults.
    #[must_use]
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            should_create: true,
            should_sync_writes: true,
        }
    }
}

/// Membership configuration without concrete transport endpoint construction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RaftKvMembershipConfig {
    /// Local node identifier.
    pub local_node_id: NodeId,
    /// Known members keyed by node identifier.
    pub members: BTreeMap<NodeId, RaftKvMemberInfo>,
}

impl RaftKvMembershipConfig {
    /// Create membership config from explicit members.
    #[must_use]
    pub fn new(local_node_id: NodeId, members: BTreeMap<NodeId, RaftKvMemberInfo>) -> Self {
        Self { local_node_id, members }
    }

    /// Return true when local node appears in membership metadata.
    #[must_use]
    pub fn has_local_member(&self) -> bool {
        self.members.contains_key(&self.local_node_id)
    }

    /// Return configured member count.
    #[must_use]
    pub fn member_count(&self) -> u32 {
        u32::try_from(self.members.len()).unwrap_or(u32::MAX)
    }
}

/// Complete reusable Raft KV node configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RaftKvNodeConfig {
    /// Storage settings.
    pub storage: RaftKvStorageConfig,
    /// Membership settings.
    pub membership: RaftKvMembershipConfig,
    /// Bounded resource settings.
    pub resources: RaftKvResourceLimits,
}

impl RaftKvNodeConfig {
    /// Validate this node config.
    pub fn validate(&self) -> Result<(), RaftKvConfigError> {
        validate_node_config(self)
    }
}

/// Declarative node specification returned by the builder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RaftKvNodeSpec {
    /// Validated configuration.
    pub config: RaftKvNodeConfig,
}

/// Builder for reusable Raft KV node specifications.
#[derive(Debug, Clone)]
pub struct RaftKvNodeBuilder {
    config: RaftKvNodeConfig,
}

impl RaftKvNodeBuilder {
    /// Create a new builder from required storage and membership settings.
    #[must_use]
    pub fn new(storage: RaftKvStorageConfig, membership: RaftKvMembershipConfig) -> Self {
        Self {
            config: RaftKvNodeConfig {
                storage,
                membership,
                resources: RaftKvResourceLimits::default(),
            },
        }
    }

    /// Replace resource limits.
    #[must_use]
    pub fn with_resource_limits(mut self, resources: RaftKvResourceLimits) -> Self {
        self.config.resources = resources;
        self
    }

    /// Validate and return a declarative node spec.
    pub fn build(self) -> Result<RaftKvNodeSpec, RaftKvConfigError> {
        validate_node_config(&self.config)?;
        Ok(RaftKvNodeSpec { config: self.config })
    }
}

/// Bundled operation surfaces for callers that already own concrete services.
#[derive(Clone)]
pub struct RaftKvServices {
    kv_store: Arc<dyn KeyValueStore>,
    cluster_controller: Arc<dyn ClusterController>,
}

impl RaftKvServices {
    /// Wrap concrete service implementations behind reusable traits.
    #[must_use]
    pub fn new(kv_store: Arc<dyn KeyValueStore>, cluster_controller: Arc<dyn ClusterController>) -> Self {
        Self {
            kv_store,
            cluster_controller,
        }
    }

    /// Get key-value operation surface.
    #[must_use]
    pub fn kv_store(&self) -> Arc<dyn KeyValueStore> {
        Arc::clone(&self.kv_store)
    }

    /// Get cluster-control operation surface.
    #[must_use]
    pub fn cluster_controller(&self) -> Arc<dyn ClusterController> {
        Arc::clone(&self.cluster_controller)
    }
}

/// Config validation error.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum RaftKvConfigError {
    /// Storage path is empty.
    #[error("storage path is empty")]
    EmptyStoragePath,
    /// Local node is absent from membership map.
    #[error("local node {local_node_id} is absent from membership config")]
    MissingLocalMember { local_node_id: NodeId },
    /// No cluster members were configured.
    #[error("membership config has no members")]
    EmptyMembership,
    /// Resource limit is zero.
    #[error("resource limit {name} must be at least {min}, got {actual}")]
    ResourceLimitTooSmall { name: &'static str, min: u32, actual: u32 },
    /// Timeout is zero.
    #[error("timeout {name} must be at least {min_ms}ms, got {actual_ms}ms")]
    TimeoutTooSmall {
        name: &'static str,
        min_ms: u64,
        actual_ms: u64,
    },
}

fn validate_node_config(config: &RaftKvNodeConfig) -> Result<(), RaftKvConfigError> {
    validate_storage_config(&config.storage)?;
    validate_membership_config(&config.membership)?;
    validate_resource_limits(&config.resources)?;
    Ok(())
}

fn validate_storage_config(config: &RaftKvStorageConfig) -> Result<(), RaftKvConfigError> {
    if path_is_empty(&config.path) {
        return Err(RaftKvConfigError::EmptyStoragePath);
    }
    Ok(())
}

fn path_is_empty(path: &Path) -> bool {
    path.as_os_str().is_empty()
}

fn validate_membership_config(config: &RaftKvMembershipConfig) -> Result<(), RaftKvConfigError> {
    if config.members.is_empty() {
        return Err(RaftKvConfigError::EmptyMembership);
    }
    if !config.has_local_member() {
        return Err(RaftKvConfigError::MissingLocalMember {
            local_node_id: config.local_node_id,
        });
    }
    Ok(())
}

fn validate_resource_limits(limits: &RaftKvResourceLimits) -> Result<(), RaftKvConfigError> {
    validate_nonzero_u32("max_batch_entries", limits.max_batch_entries)?;
    validate_nonzero_u32("max_scan_results", limits.max_scan_results)?;
    validate_nonzero_u32("max_key_size_bytes", limits.max_key_size_bytes)?;
    validate_nonzero_u32("max_value_size_bytes", limits.max_value_size_bytes)?;
    validate_nonzero_timeout_ms("operation_timeout_ms", limits.operation_timeout_ms)?;
    validate_nonzero_timeout_ms("connect_timeout_ms", limits.connect_timeout_ms)?;
    Ok(())
}

fn validate_nonzero_u32(name: &'static str, actual: u32) -> Result<(), RaftKvConfigError> {
    if actual < MIN_RESOURCE_LIMIT {
        return Err(RaftKvConfigError::ResourceLimitTooSmall {
            name,
            min: MIN_RESOURCE_LIMIT,
            actual,
        });
    }
    Ok(())
}

fn validate_nonzero_timeout_ms(name: &'static str, actual_ms: u64) -> Result<(), RaftKvConfigError> {
    if actual_ms < MIN_TIMEOUT_MS {
        return Err(RaftKvConfigError::TimeoutTooSmall {
            name,
            min_ms: MIN_TIMEOUT_MS,
            actual_ms,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const LOCAL_NODE_ID: NodeId = 1;
    const REMOTE_NODE_ID: NodeId = 2;
    const ENDPOINT_ID: &str = "node-a";
    const STORAGE_FILE: &str = "raft-kv.redb";

    fn member_info() -> RaftKvMemberInfo {
        RaftKvMemberInfo::new(ENDPOINT_ID.to_string(), Vec::new())
    }

    fn valid_membership() -> RaftKvMembershipConfig {
        let mut members = BTreeMap::new();
        members.insert(LOCAL_NODE_ID, member_info());
        RaftKvMembershipConfig::new(LOCAL_NODE_ID, members)
    }

    fn valid_storage() -> RaftKvStorageConfig {
        RaftKvStorageConfig::new(PathBuf::from(STORAGE_FILE))
    }

    #[test]
    fn builder_accepts_valid_reusable_config() {
        let spec = RaftKvNodeBuilder::new(valid_storage(), valid_membership()).build();
        assert!(spec.is_ok());
        let spec = spec.expect("valid config should build");
        assert_eq!(spec.config.membership.local_node_id, LOCAL_NODE_ID);
        assert_eq!(spec.config.membership.member_count(), 1);
        assert_eq!(spec.config.resources.max_batch_entries, DEFAULT_MAX_BATCH_ENTRIES);
    }

    #[test]
    fn builder_rejects_empty_storage_path() {
        let storage = RaftKvStorageConfig::new(PathBuf::new());
        let result = RaftKvNodeBuilder::new(storage, valid_membership()).build();
        assert_eq!(result, Err(RaftKvConfigError::EmptyStoragePath));
    }

    #[test]
    fn builder_rejects_missing_local_member() {
        let mut members = BTreeMap::new();
        members.insert(REMOTE_NODE_ID, member_info());
        let membership = RaftKvMembershipConfig::new(LOCAL_NODE_ID, members);
        let result = RaftKvNodeBuilder::new(valid_storage(), membership).build();
        assert_eq!(
            result,
            Err(RaftKvConfigError::MissingLocalMember {
                local_node_id: LOCAL_NODE_ID,
            })
        );
    }

    #[test]
    fn builder_rejects_zero_resource_limit() {
        let resources = RaftKvResourceLimits {
            max_batch_entries: 0,
            ..RaftKvResourceLimits::default()
        };
        let result =
            RaftKvNodeBuilder::new(valid_storage(), valid_membership()).with_resource_limits(resources).build();
        assert_eq!(
            result,
            Err(RaftKvConfigError::ResourceLimitTooSmall {
                name: "max_batch_entries",
                min: MIN_RESOURCE_LIMIT,
                actual: 0,
            })
        );
    }

    #[test]
    fn public_operation_traits_remain_reexported() {
        static_assertions::assert_obj_safe!(KeyValueStore);
        static_assertions::assert_obj_safe!(ClusterController);
    }
}

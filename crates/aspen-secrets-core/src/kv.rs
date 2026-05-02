//! KV secrets engine types.
//!
//! Data structures for versioned secret storage, following HashiCorp Vault patterns.

use std::collections::HashMap;

use serde::Deserialize;
use serde::Serialize;

/// Secret data stored in KV.
///
/// This is the actual secret content stored at each version.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretData {
    /// Key-value pairs making up the secret.
    pub data: HashMap<String, String>,
}

impl SecretData {
    /// Create new secret data from a map.
    pub fn new(data: HashMap<String, String>) -> Self {
        Self { data }
    }

    /// Create empty secret data.
    pub fn empty() -> Self {
        Self { data: HashMap::new() }
    }

    /// Get a value by key.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.data.get(key).map(|s| s.as_str())
    }

    /// Insert a key-value pair.
    pub fn insert(&mut self, key: String, value: String) {
        self.data.insert(key, value);
    }

    /// Merge another secret data into this one (for PATCH operations).
    pub fn merge(&mut self, other: &SecretData) {
        for (k, v) in &other.data {
            self.data.insert(k.clone(), v.clone());
        }
    }

    /// Get the total size in bytes (approximate).
    pub fn size_bytes(&self) -> usize {
        self.data.iter().map(|(k, v)| k.len() + v.len()).sum()
    }
}

/// Metadata for a specific version of a secret.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionMetadata {
    /// Version number (1-indexed).
    pub version: u64,
    /// Unix timestamp when this version was created.
    pub created_time_unix_ms: u64,
    /// Unix timestamp when this version was soft-deleted (None if not deleted).
    pub deletion_time_unix_ms: Option<u64>,
    /// Whether this version has been permanently destroyed.
    pub destroyed: bool,
}

impl VersionMetadata {
    /// Create metadata for a new version.
    pub fn new(version: u64, created_time_unix_ms: u64) -> Self {
        Self {
            version,
            created_time_unix_ms,
            deletion_time_unix_ms: None,
            destroyed: false,
        }
    }

    /// Check if this version is accessible (not deleted or destroyed).
    pub fn is_accessible(&self) -> bool {
        !self.destroyed && self.deletion_time_unix_ms.is_none()
    }

    /// Check if this version is soft-deleted (can be undeleted).
    pub fn is_soft_deleted(&self) -> bool {
        self.deletion_time_unix_ms.is_some() && !self.destroyed
    }
}

/// Metadata for a secret path (across all versions).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretMetadata {
    /// Current version number (next write will be current_version + 1).
    pub current_version: u64,
    /// Unix timestamp when this secret was first created.
    pub created_time_unix_ms: u64,
    /// Unix timestamp when this secret was last updated.
    pub updated_time_unix_ms: u64,
    /// Maximum number of versions to keep (0 = use engine default).
    pub max_versions: u32,
    /// Whether CAS is required for writes to this secret.
    pub cas_required: bool,
    /// Time after which versions are automatically deleted (0 = never).
    pub delete_version_after_secs: u64,
    /// Custom metadata (user-defined key-value pairs).
    pub custom_metadata: HashMap<String, String>,
    /// Version metadata for each version.
    pub versions: HashMap<u64, VersionMetadata>,
}

impl SecretMetadata {
    /// Create new metadata for a secret.
    pub fn new(created_time_unix_ms: u64) -> Self {
        Self {
            current_version: 0,
            created_time_unix_ms,
            updated_time_unix_ms: created_time_unix_ms,
            max_versions: 0,
            cas_required: false,
            delete_version_after_secs: 0,
            custom_metadata: HashMap::new(),
            versions: HashMap::new(),
        }
    }

    /// Increment version and return the new version number.
    pub fn next_version(&mut self, created_time_unix_ms: u64) -> u64 {
        self.current_version += 1;
        self.updated_time_unix_ms = created_time_unix_ms;
        self.versions
            .insert(self.current_version, VersionMetadata::new(self.current_version, created_time_unix_ms));
        self.current_version
    }

    /// Get the oldest version number.
    pub fn oldest_version(&self) -> Option<u64> {
        self.versions.keys().min().copied()
    }

    /// Get the newest (current) version metadata.
    pub fn current_version_metadata(&self) -> Option<&VersionMetadata> {
        self.versions.get(&self.current_version)
    }

    /// Get metadata for a specific version.
    pub fn version(&self, version: u64) -> Option<&VersionMetadata> {
        self.versions.get(&version)
    }

    /// Get mutable metadata for a specific version.
    pub fn version_mut(&mut self, version: u64) -> Option<&mut VersionMetadata> {
        self.versions.get_mut(&version)
    }

    /// Remove old versions to stay within max_versions limit.
    pub fn prune_versions(&mut self, max_versions: u32) -> Vec<u64> {
        if max_versions == 0 || self.versions.len() as u32 <= max_versions {
            return vec![];
        }

        let mut versions: Vec<u64> = self.versions.keys().copied().collect();
        versions.sort();

        let to_remove = versions.len() - max_versions as usize;
        let removed: Vec<u64> = versions.into_iter().take(to_remove).collect();

        for v in &removed {
            self.versions.remove(v);
        }

        removed
    }

    /// Get versions that should be auto-deleted based on delete_version_after_secs.
    pub fn expired_versions(&self, now_unix_ms: u64) -> Vec<u64> {
        if self.delete_version_after_secs == 0 {
            return vec![];
        }

        let ttl_ms = self.delete_version_after_secs * 1000;
        self.versions
            .iter()
            .filter(|(_, meta)| meta.is_accessible() && now_unix_ms.saturating_sub(meta.created_time_unix_ms) > ttl_ms)
            .map(|(v, _)| *v)
            .collect()
    }
}

/// KV engine configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KvConfig {
    /// Maximum number of versions to keep per secret.
    pub max_versions: u32,
    /// Whether CAS is required by default for all secrets.
    pub cas_required: bool,
    /// Time in seconds after which versions are automatically deleted.
    pub delete_version_after_secs: u64,
}

impl Default for KvConfig {
    fn default() -> Self {
        Self {
            max_versions: crate::constants::DEFAULT_MAX_VERSIONS,
            cas_required: false,
            delete_version_after_secs: 0,
        }
    }
}

/// Request to read a secret.
#[derive(Debug, Clone)]
pub struct ReadSecretRequest {
    /// Secret path.
    pub path: String,
    /// Specific version to read (None = current).
    pub version: Option<u64>,
}

impl ReadSecretRequest {
    /// Create a new read request for the current version.
    pub fn new(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            version: None,
        }
    }

    /// Create a new read request for a specific version.
    pub fn with_version(path: impl Into<String>, version: u64) -> Self {
        Self {
            path: path.into(),
            version: Some(version),
        }
    }
}

/// Response from reading a secret.
#[derive(Debug, Clone)]
pub struct ReadSecretResponse {
    /// The secret data.
    pub data: SecretData,
    /// Metadata for this version.
    pub metadata: VersionMetadata,
}

/// Request to write a secret.
#[derive(Debug, Clone)]
pub struct WriteSecretRequest {
    /// Secret path.
    pub path: String,
    /// Secret data to write.
    pub data: SecretData,
    /// Check-and-set version (None = no CAS).
    pub cas: Option<u64>,
}

impl WriteSecretRequest {
    /// Create a new write request without CAS.
    pub fn new(path: impl Into<String>, data: SecretData) -> Self {
        Self {
            path: path.into(),
            data,
            cas: None,
        }
    }

    /// Create a new write request with CAS.
    pub fn with_cas(path: impl Into<String>, data: SecretData, expected_version: u64) -> Self {
        Self {
            path: path.into(),
            data,
            cas: Some(expected_version),
        }
    }
}

/// Response from writing a secret.
#[derive(Debug, Clone)]
pub struct WriteSecretResponse {
    /// Version number of the written secret.
    pub version: u64,
    /// Metadata for this version.
    pub metadata: VersionMetadata,
}

/// Request to delete secret versions.
#[derive(Debug, Clone)]
pub struct DeleteSecretRequest {
    /// Secret path.
    pub path: String,
    /// Versions to delete (empty = delete current version).
    pub versions: Vec<u64>,
}

impl DeleteSecretRequest {
    /// Create a request to soft-delete the current version.
    pub fn current(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            versions: vec![],
        }
    }

    /// Create a request to soft-delete specific versions.
    pub fn versions(path: impl Into<String>, versions: Vec<u64>) -> Self {
        Self {
            path: path.into(),
            versions,
        }
    }
}

/// Request to permanently destroy secret versions.
#[derive(Debug, Clone)]
pub struct DestroySecretRequest {
    /// Secret path.
    pub path: String,
    /// Versions to destroy.
    pub versions: Vec<u64>,
}

impl DestroySecretRequest {
    /// Create a request to destroy specific versions.
    pub fn new(path: impl Into<String>, versions: Vec<u64>) -> Self {
        Self {
            path: path.into(),
            versions,
        }
    }
}

/// Request to undelete soft-deleted versions.
#[derive(Debug, Clone)]
pub struct UndeleteSecretRequest {
    /// Secret path.
    pub path: String,
    /// Versions to undelete.
    pub versions: Vec<u64>,
}

impl UndeleteSecretRequest {
    /// Create a request to undelete specific versions.
    pub fn new(path: impl Into<String>, versions: Vec<u64>) -> Self {
        Self {
            path: path.into(),
            versions,
        }
    }
}

/// Request to read secret metadata.
#[derive(Debug, Clone)]
pub struct ReadMetadataRequest {
    /// Secret path.
    pub path: String,
}

impl ReadMetadataRequest {
    /// Create a new metadata read request.
    pub fn new(path: impl Into<String>) -> Self {
        Self { path: path.into() }
    }
}

/// Request to update secret metadata.
#[derive(Debug, Clone)]
pub struct UpdateMetadataRequest {
    /// Secret path.
    pub path: String,
    /// Maximum versions to keep (None = don't change).
    pub max_versions: Option<u32>,
    /// Whether CAS is required (None = don't change).
    pub cas_required: Option<bool>,
    /// Delete versions after this many seconds (None = don't change).
    pub delete_version_after_secs: Option<u64>,
    /// Custom metadata to set (merged with existing).
    pub custom_metadata: Option<HashMap<String, String>>,
}

impl UpdateMetadataRequest {
    /// Create a new metadata update request.
    pub fn new(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            max_versions: None,
            cas_required: None,
            delete_version_after_secs: None,
            custom_metadata: None,
        }
    }

    /// Set max_versions.
    pub fn with_max_versions(mut self, max_versions: u32) -> Self {
        self.max_versions = Some(max_versions);
        self
    }

    /// Set cas_required.
    pub fn with_cas_required(mut self, cas_required: bool) -> Self {
        self.cas_required = Some(cas_required);
        self
    }
}

/// Request to list secrets.
#[derive(Debug, Clone)]
pub struct ListSecretsRequest {
    /// Path prefix to list.
    pub path: String,
}

impl ListSecretsRequest {
    /// Create a new list request.
    pub fn new(path: impl Into<String>) -> Self {
        Self { path: path.into() }
    }
}

/// Response from listing secrets.
#[derive(Debug, Clone)]
pub struct ListSecretsResponse {
    /// Keys found under the path.
    /// Keys ending in "/" represent sub-paths.
    pub keys: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // SecretData
    // =========================================================================

    #[test]
    fn test_secret_data_empty() {
        let sd = SecretData::empty();
        assert!(sd.data.is_empty());
        assert_eq!(sd.size_bytes(), 0);
    }

    #[test]
    fn test_secret_data_new_and_get() {
        let mut data = HashMap::new();
        data.insert("password".into(), "s3cret".into());
        data.insert("username".into(), "admin".into());
        let sd = SecretData::new(data);
        assert_eq!(sd.get("password"), Some("s3cret"));
        assert_eq!(sd.get("username"), Some("admin"));
        assert_eq!(sd.get("missing"), None);
    }

    #[test]
    fn test_secret_data_insert() {
        let mut sd = SecretData::empty();
        sd.insert("key".into(), "value".into());
        assert_eq!(sd.get("key"), Some("value"));
    }

    #[test]
    fn test_secret_data_merge_overwrites() {
        let mut base = SecretData::new(HashMap::from([("a".into(), "1".into()), ("b".into(), "2".into())]));
        let patch = SecretData::new(HashMap::from([("b".into(), "updated".into()), ("c".into(), "3".into())]));
        base.merge(&patch);
        assert_eq!(base.get("a"), Some("1"));
        assert_eq!(base.get("b"), Some("updated"));
        assert_eq!(base.get("c"), Some("3"));
    }

    #[test]
    fn test_secret_data_size_bytes() {
        let sd = SecretData::new(HashMap::from([
            ("key".into(), "val".into()), // 3 + 3 = 6
            ("ab".into(), "cdef".into()), // 2 + 4 = 6
        ]));
        assert_eq!(sd.size_bytes(), 12);
    }

    #[test]
    fn test_secret_data_serde_roundtrip() {
        let sd = SecretData::new(HashMap::from([("password".into(), "hunter2".into())]));
        let json = serde_json::to_string(&sd).expect("serialize");
        let back: SecretData = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.get("password"), Some("hunter2"));
    }

    // =========================================================================
    // VersionMetadata
    // =========================================================================

    #[test]
    fn test_version_metadata_new_is_accessible() {
        let vm = VersionMetadata::new(1, 1000);
        assert_eq!(vm.version, 1);
        assert_eq!(vm.created_time_unix_ms, 1000);
        assert!(vm.is_accessible());
        assert!(!vm.is_soft_deleted());
        assert!(!vm.destroyed);
    }

    #[test]
    fn test_version_metadata_soft_delete() {
        let mut vm = VersionMetadata::new(1, 1000);
        vm.deletion_time_unix_ms = Some(2000);
        assert!(!vm.is_accessible());
        assert!(vm.is_soft_deleted());
    }

    #[test]
    fn test_version_metadata_destroyed() {
        let mut vm = VersionMetadata::new(1, 1000);
        vm.destroyed = true;
        assert!(!vm.is_accessible());
        assert!(!vm.is_soft_deleted()); // destroyed overrides soft-delete
    }

    #[test]
    fn test_version_metadata_destroyed_and_deleted() {
        let mut vm = VersionMetadata::new(1, 1000);
        vm.deletion_time_unix_ms = Some(2000);
        vm.destroyed = true;
        assert!(!vm.is_accessible());
        assert!(!vm.is_soft_deleted()); // destroyed takes priority
    }

    // =========================================================================
    // SecretMetadata
    // =========================================================================

    #[test]
    fn test_secret_metadata_new() {
        let sm = SecretMetadata::new(5000);
        assert_eq!(sm.current_version, 0);
        assert_eq!(sm.created_time_unix_ms, 5000);
        assert_eq!(sm.updated_time_unix_ms, 5000);
        assert!(!sm.cas_required);
        assert_eq!(sm.max_versions, 0);
        assert!(sm.versions.is_empty());
    }

    #[test]
    fn test_secret_metadata_next_version() {
        let mut sm = SecretMetadata::new(1000);
        let v1 = sm.next_version(2000);
        assert_eq!(v1, 1);
        assert_eq!(sm.current_version, 1);
        assert_eq!(sm.updated_time_unix_ms, 2000);
        assert!(sm.versions.contains_key(&1));

        let v2 = sm.next_version(3000);
        assert_eq!(v2, 2);
        assert_eq!(sm.versions.len(), 2);
    }

    #[test]
    fn test_secret_metadata_oldest_version() {
        let mut sm = SecretMetadata::new(0);
        assert_eq!(sm.oldest_version(), None);
        sm.next_version(100);
        sm.next_version(200);
        sm.next_version(300);
        assert_eq!(sm.oldest_version(), Some(1));
    }

    #[test]
    fn test_secret_metadata_current_version_metadata() {
        let mut sm = SecretMetadata::new(0);
        assert!(sm.current_version_metadata().is_none());
        sm.next_version(1000);
        let meta = sm.current_version_metadata().expect("should have v1");
        assert_eq!(meta.version, 1);
        assert_eq!(meta.created_time_unix_ms, 1000);
    }

    #[test]
    fn test_secret_metadata_version_access() {
        let mut sm = SecretMetadata::new(0);
        sm.next_version(100);
        sm.next_version(200);
        assert!(sm.version(1).is_some());
        assert!(sm.version(2).is_some());
        assert!(sm.version(3).is_none());
    }

    #[test]
    fn test_secret_metadata_version_mut() {
        let mut sm = SecretMetadata::new(0);
        sm.next_version(100);
        let v = sm.version_mut(1).expect("v1 exists");
        v.deletion_time_unix_ms = Some(500);
        assert!(sm.version(1).expect("v1").is_soft_deleted());
    }

    // =========================================================================
    // prune_versions
    // =========================================================================

    #[test]
    fn test_prune_versions_no_op_when_under_limit() {
        let mut sm = SecretMetadata::new(0);
        sm.next_version(100);
        sm.next_version(200);
        let removed = sm.prune_versions(5);
        assert!(removed.is_empty());
        assert_eq!(sm.versions.len(), 2);
    }

    #[test]
    fn test_prune_versions_zero_means_unlimited() {
        let mut sm = SecretMetadata::new(0);
        for i in 1..=10 {
            sm.next_version(i * 100);
        }
        let removed = sm.prune_versions(0);
        assert!(removed.is_empty());
        assert_eq!(sm.versions.len(), 10);
    }

    #[test]
    fn test_prune_versions_removes_oldest() {
        let mut sm = SecretMetadata::new(0);
        for i in 1..=5 {
            sm.next_version(i * 100);
        }
        let removed = sm.prune_versions(3);
        assert_eq!(removed, vec![1, 2]);
        assert_eq!(sm.versions.len(), 3);
        assert!(sm.version(1).is_none());
        assert!(sm.version(2).is_none());
        assert!(sm.version(3).is_some());
        assert!(sm.version(5).is_some());
    }

    // =========================================================================
    // expired_versions
    // =========================================================================

    #[test]
    fn test_expired_versions_disabled_when_zero() {
        let mut sm = SecretMetadata::new(0);
        sm.next_version(100);
        sm.delete_version_after_secs = 0;
        let expired = sm.expired_versions(999_999);
        assert!(expired.is_empty());
    }

    #[test]
    fn test_expired_versions_returns_old_versions() {
        let mut sm = SecretMetadata::new(0);
        sm.next_version(1000); // created at 1000ms
        sm.next_version(5000); // created at 5000ms
        sm.delete_version_after_secs = 3; // 3 seconds = 3000ms

        // At time 5000ms: v1 is 4000ms old (>3000ms), v2 is 0ms old
        let expired = sm.expired_versions(5000);
        assert_eq!(expired, vec![1]);
    }

    #[test]
    fn test_expired_versions_skips_deleted() {
        let mut sm = SecretMetadata::new(0);
        sm.next_version(1000);
        sm.next_version(2000);
        sm.delete_version_after_secs = 1; // 1 second

        // Soft-delete v1
        sm.version_mut(1).expect("v1").deletion_time_unix_ms = Some(1500);

        // At time 10000ms: v1 is soft-deleted (not accessible), v2 is expired
        let expired = sm.expired_versions(10000);
        assert!(!expired.contains(&1)); // not accessible
        assert!(expired.contains(&2));
    }

    // =========================================================================
    // KvConfig
    // =========================================================================

    #[test]
    fn test_kv_config_default() {
        let cfg = KvConfig::default();
        assert_eq!(cfg.max_versions, crate::constants::DEFAULT_MAX_VERSIONS);
        assert!(!cfg.cas_required);
        assert_eq!(cfg.delete_version_after_secs, 0);
    }

    // =========================================================================
    // Request constructors
    // =========================================================================

    #[test]
    fn test_read_secret_request_current() {
        let req = ReadSecretRequest::new("secret/myapp");
        assert_eq!(req.path, "secret/myapp");
        assert!(req.version.is_none());
    }

    #[test]
    fn test_read_secret_request_versioned() {
        let req = ReadSecretRequest::with_version("secret/myapp", 3);
        assert_eq!(req.path, "secret/myapp");
        assert_eq!(req.version, Some(3));
    }

    #[test]
    fn test_write_secret_request_without_cas() {
        let data = SecretData::new(HashMap::from([("key".into(), "val".into())]));
        let req = WriteSecretRequest::new("path", data);
        assert_eq!(req.path, "path");
        assert!(req.cas.is_none());
    }

    #[test]
    fn test_write_secret_request_with_cas() {
        let data = SecretData::empty();
        let req = WriteSecretRequest::with_cas("path", data, 5);
        assert_eq!(req.cas, Some(5));
    }

    #[test]
    fn test_delete_secret_request_current() {
        let req = DeleteSecretRequest::current("secret/db");
        assert_eq!(req.path, "secret/db");
        assert!(req.versions.is_empty());
    }

    #[test]
    fn test_delete_secret_request_versions() {
        let req = DeleteSecretRequest::versions("secret/db", vec![1, 3, 5]);
        assert_eq!(req.versions, vec![1, 3, 5]);
    }

    #[test]
    fn test_destroy_secret_request() {
        let req = DestroySecretRequest::new("path", vec![2, 4]);
        assert_eq!(req.path, "path");
        assert_eq!(req.versions, vec![2, 4]);
    }

    #[test]
    fn test_undelete_secret_request() {
        let req = UndeleteSecretRequest::new("path", vec![1]);
        assert_eq!(req.path, "path");
        assert_eq!(req.versions, vec![1]);
    }

    #[test]
    fn test_read_metadata_request() {
        let req = ReadMetadataRequest::new("secret/app");
        assert_eq!(req.path, "secret/app");
    }

    #[test]
    fn test_update_metadata_request_builder() {
        let req = UpdateMetadataRequest::new("path").with_max_versions(20).with_cas_required(true);
        assert_eq!(req.path, "path");
        assert_eq!(req.max_versions, Some(20));
        assert_eq!(req.cas_required, Some(true));
        assert!(req.delete_version_after_secs.is_none());
        assert!(req.custom_metadata.is_none());
    }

    #[test]
    fn test_list_secrets_request() {
        let req = ListSecretsRequest::new("secret/");
        assert_eq!(req.path, "secret/");
    }
}

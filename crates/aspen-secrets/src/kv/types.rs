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

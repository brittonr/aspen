//! Nix binary cache metadata types.
//!
//! This module defines the data structures for storing Nix store path metadata
//! in the Aspen distributed cache. The design follows the Nix narinfo format
//! but stores data as JSON in the Raft KV store for easy querying.

use serde::Deserialize;
use serde::Serialize;

use crate::error::CacheError;
use crate::error::Result;

// Tiger Style: All limits explicit and bounded
/// Maximum number of references per cache entry.
pub const MAX_REFERENCES: usize = 1000;
/// Maximum length of deriver path.
pub const MAX_DERIVER_LENGTH: usize = 1024;
/// Maximum length of store path.
pub const MAX_STORE_PATH_LENGTH: usize = 512;
/// Store hash length (Nix uses 32 bytes = 52 base32 chars, but we also support base16).
pub const STORE_HASH_LENGTH: usize = 32;

/// KV key prefix for cache entries.
pub const CACHE_KEY_PREFIX: &str = "_cache:narinfo:";
/// KV key for cache statistics.
pub const CACHE_STATS_KEY: &str = "_cache:stats";

/// A cache entry representing a Nix store path in the distributed cache.
///
/// This contains all metadata needed to retrieve and verify a NAR archive
/// from the blob store.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CacheEntry {
    /// Full Nix store path (e.g., /nix/store/abc...-name).
    pub store_path: String,

    /// The hash portion of the store path (e.g., abc...).
    /// This is used as the lookup key.
    pub store_hash: String,

    /// BLAKE3 hash of the NAR archive in the blob store.
    /// This is used to retrieve the blob via iroh-blobs.
    pub blob_hash: String,

    /// Size of the NAR archive in bytes.
    pub nar_size: u64,

    /// SHA256 hash of the NAR archive (Nix's native format).
    /// Format: "sha256:base16_or_base32_hash"
    pub nar_hash: String,

    /// Original file size (may differ from NAR size due to NAR overhead).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file_size: Option<u64>,

    /// Store paths this entry references (dependencies).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub references: Vec<String>,

    /// Deriver store path (the .drv that built this).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deriver: Option<String>,

    /// Unix timestamp when this entry was created.
    pub created_at: u64,

    /// Node ID that built and uploaded this entry.
    pub created_by_node: u64,

    /// CI job ID that created this entry (if from CI build).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ci_job_id: Option<String>,

    /// CI pipeline run ID (if from CI build).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ci_run_id: Option<String>,
}

impl CacheEntry {
    /// Create a new cache entry with required fields.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        store_path: String,
        store_hash: String,
        blob_hash: String,
        nar_size: u64,
        nar_hash: String,
        created_at: u64,
        created_by_node: u64,
    ) -> Self {
        Self {
            store_path,
            store_hash,
            blob_hash,
            nar_size,
            nar_hash,
            file_size: None,
            references: Vec::new(),
            deriver: None,
            created_at,
            created_by_node,
            ci_job_id: None,
            ci_run_id: None,
        }
    }

    /// Set the references for this entry.
    pub fn with_references(mut self, references: Vec<String>) -> Result<Self> {
        if references.len() > MAX_REFERENCES {
            return Err(CacheError::TooManyReferences {
                count: references.len() as u32,
                max: MAX_REFERENCES as u32,
            });
        }
        self.references = references;
        Ok(self)
    }

    /// Set the deriver for this entry.
    pub fn with_deriver(mut self, deriver: Option<String>) -> Result<Self> {
        if let Some(ref d) = deriver
            && d.len() > MAX_DERIVER_LENGTH
        {
            return Err(CacheError::DeriverTooLong {
                length_bytes: d.len() as u64,
                max_bytes: MAX_DERIVER_LENGTH as u64,
            });
        }
        self.deriver = deriver;
        Ok(self)
    }

    /// Set CI metadata for this entry.
    pub fn with_ci_metadata(mut self, job_id: Option<String>, run_id: Option<String>) -> Self {
        self.ci_job_id = job_id;
        self.ci_run_id = run_id;
        self
    }

    /// Generate the KV key for this entry.
    pub fn kv_key(&self) -> String {
        format!("{}{}", CACHE_KEY_PREFIX, self.store_hash)
    }

    /// Serialize this entry to JSON bytes for KV storage.
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        serde_json::to_vec(self).map_err(|e| CacheError::Serialization { message: e.to_string() })
    }

    /// Deserialize an entry from JSON bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        serde_json::from_slice(bytes).map_err(|e| CacheError::Deserialization { message: e.to_string() })
    }
}

/// Cache statistics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CacheStats {
    /// Total number of cache entries.
    pub total_entries: u64,

    /// Total size of all NAR archives in bytes.
    pub total_nar_bytes: u64,

    /// Number of cache queries.
    pub query_count: u64,

    /// Number of cache hits.
    pub hit_count: u64,

    /// Number of cache misses.
    pub miss_count: u64,

    /// Unix timestamp of last update.
    pub last_updated: u64,
}

impl CacheStats {
    /// Create new empty stats.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a cache hit.
    pub fn record_hit(&mut self) {
        self.query_count += 1;
        self.hit_count += 1;
    }

    /// Record a cache miss.
    pub fn record_miss(&mut self) {
        self.query_count += 1;
        self.miss_count += 1;
    }

    /// Record a new entry being added.
    pub fn record_entry_added(&mut self, nar_size: u64) {
        self.total_entries += 1;
        self.total_nar_bytes += nar_size;
    }

    /// Calculate hit rate as a percentage.
    pub fn hit_rate_percent(&self) -> f64 {
        if self.query_count == 0 {
            0.0
        } else {
            (self.hit_count as f64 / self.query_count as f64) * 100.0
        }
    }

    /// Serialize to JSON bytes.
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        serde_json::to_vec(self).map_err(|e| CacheError::Serialization { message: e.to_string() })
    }

    /// Deserialize from JSON bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        serde_json::from_slice(bytes).map_err(|e| CacheError::Deserialization { message: e.to_string() })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_entry_roundtrip() {
        let entry = CacheEntry::new(
            "/nix/store/abc123-hello".to_string(),
            "abc123".to_string(),
            "blake3hash".to_string(),
            1024,
            "sha256:deadbeef".to_string(),
            1234567890,
            1,
        );

        let bytes = entry.to_bytes().unwrap();
        let decoded = CacheEntry::from_bytes(&bytes).unwrap();
        assert_eq!(entry, decoded);
    }

    #[test]
    fn test_cache_entry_kv_key() {
        let entry = CacheEntry::new(
            "/nix/store/abc123-hello".to_string(),
            "abc123".to_string(),
            "blake3hash".to_string(),
            1024,
            "sha256:deadbeef".to_string(),
            1234567890,
            1,
        );

        assert_eq!(entry.kv_key(), "_cache:narinfo:abc123");
    }

    #[test]
    fn test_cache_entry_too_many_references() {
        let entry = CacheEntry::new(
            "/nix/store/abc123-hello".to_string(),
            "abc123".to_string(),
            "blake3hash".to_string(),
            1024,
            "sha256:deadbeef".to_string(),
            1234567890,
            1,
        );

        let too_many: Vec<String> = (0..MAX_REFERENCES + 1).map(|i| format!("ref{i}")).collect();
        let result = entry.with_references(too_many);
        assert!(result.is_err());
    }

    #[test]
    fn test_cache_stats() {
        let mut stats = CacheStats::new();
        assert_eq!(stats.hit_rate_percent(), 0.0);

        stats.record_hit();
        stats.record_hit();
        stats.record_miss();

        assert_eq!(stats.query_count, 3);
        assert_eq!(stats.hit_count, 2);
        assert_eq!(stats.miss_count, 1);
        assert!((stats.hit_rate_percent() - 66.666).abs() < 0.01);
    }
}

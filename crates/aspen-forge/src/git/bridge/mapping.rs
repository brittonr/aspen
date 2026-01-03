//! Bidirectional hash mapping between BLAKE3 and SHA-1.
//!
//! This module provides persistent storage for mapping between Aspen Forge's
//! BLAKE3 hashes and standard Git's SHA-1 hashes. Mappings are stored in
//! Raft KV for cluster-wide consistency.

use std::num::NonZeroUsize;
use std::sync::Arc;

use base64::Engine;
use lru::LruCache;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use crate::identity::RepoId;
use aspen_core::{KeyValueStore, ReadRequest, WriteCommand, WriteRequest};

use super::constants::{KV_PREFIX_B3_TO_SHA1, KV_PREFIX_SHA1_TO_B3, MAX_HASH_CACHE_SIZE, MAX_HASH_MAPPING_BATCH_SIZE};
use super::error::{BridgeError, BridgeResult};
use super::sha1::Sha1Hash;

/// Git object type for validation during conversion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GitObjectType {
    /// Blob (file content).
    Blob,
    /// Tree (directory listing).
    Tree,
    /// Commit.
    Commit,
    /// Annotated tag.
    Tag,
}

impl GitObjectType {
    /// Convert to git object type string.
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Blob => "blob",
            Self::Tree => "tree",
            Self::Commit => "commit",
            Self::Tag => "tag",
        }
    }

    /// Parse from git object type string.
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "blob" => Some(Self::Blob),
            "tree" => Some(Self::Tree),
            "commit" => Some(Self::Commit),
            "tag" => Some(Self::Tag),
            _ => None,
        }
    }
}

/// A bidirectional hash mapping entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HashMapping {
    /// BLAKE3 hash (Forge).
    pub blake3: [u8; 32],
    /// SHA-1 hash (Git).
    pub sha1: [u8; 20],
    /// Object type.
    pub object_type: GitObjectType,
    /// Timestamp when mapping was created (milliseconds since epoch).
    pub created_at_ms: u64,
}

impl HashMapping {
    /// Create a new hash mapping.
    pub fn new(blake3: blake3::Hash, sha1: Sha1Hash, object_type: GitObjectType) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        Self {
            blake3: *blake3.as_bytes(),
            sha1: *sha1.as_bytes(),
            object_type,
            created_at_ms: now,
        }
    }

    /// Get the BLAKE3 hash.
    pub fn blake3_hash(&self) -> blake3::Hash {
        blake3::Hash::from_bytes(self.blake3)
    }

    /// Get the SHA-1 hash.
    pub fn sha1_hash(&self) -> Sha1Hash {
        Sha1Hash::from_bytes(self.sha1)
    }

    /// Serialize to base64 string for KV storage.
    fn to_base64(&self) -> BridgeResult<String> {
        let bytes = postcard::to_allocvec(self)?;
        Ok(base64::engine::general_purpose::STANDARD.encode(&bytes))
    }

    /// Deserialize from base64 string.
    fn from_base64(s: &str) -> BridgeResult<Self> {
        let bytes = base64::engine::general_purpose::STANDARD.decode(s).map_err(|e| BridgeError::Serialization {
            message: format!("invalid base64: {e}"),
        })?;
        Ok(postcard::from_bytes(&bytes)?)
    }
}

/// Cache entry for quick lookups.
#[derive(Debug, Clone, Copy)]
struct CacheEntry {
    sha1: Sha1Hash,
    object_type: GitObjectType,
}

/// Bidirectional hash mapping store.
///
/// Stores mappings between BLAKE3 (Forge) and SHA-1 (Git) hashes.
/// Uses an LRU cache for frequently accessed mappings and Raft KV
/// for persistent storage.
pub struct HashMappingStore<K: KeyValueStore + ?Sized> {
    /// KV store for persistent mappings.
    kv: Arc<K>,
    /// LRU cache for BLAKE3 -> SHA-1 lookups.
    cache: RwLock<LruCache<[u8; 32], CacheEntry>>,
}

impl<K: KeyValueStore + ?Sized> HashMappingStore<K> {
    /// Create a new hash mapping store.
    pub fn new(kv: Arc<K>) -> Self {
        let cache_size = NonZeroUsize::new(MAX_HASH_CACHE_SIZE).unwrap();
        Self {
            kv,
            cache: RwLock::new(LruCache::new(cache_size)),
        }
    }

    /// Build the KV key for BLAKE3 -> SHA-1 lookup.
    fn b3_to_sha1_key(repo_id: &RepoId, blake3: &blake3::Hash) -> String {
        format!("{}{}:{}", KV_PREFIX_B3_TO_SHA1, repo_id.to_hex(), hex::encode(blake3.as_bytes()))
    }

    /// Build the KV key for SHA-1 -> BLAKE3 lookup.
    fn sha1_to_b3_key(repo_id: &RepoId, sha1: &Sha1Hash) -> String {
        format!("{}{}:{}", KV_PREFIX_SHA1_TO_B3, repo_id.to_hex(), sha1.to_hex())
    }

    /// Look up SHA-1 hash from BLAKE3 hash.
    ///
    /// Checks cache first, then falls back to KV store.
    pub async fn get_sha1(
        &self,
        repo_id: &RepoId,
        blake3: &blake3::Hash,
    ) -> BridgeResult<Option<(Sha1Hash, GitObjectType)>> {
        // Check cache first
        {
            let mut cache = self.cache.write();
            if let Some(entry) = cache.get(blake3.as_bytes()) {
                return Ok(Some((entry.sha1, entry.object_type)));
            }
        }

        // Fetch from KV
        let key = Self::b3_to_sha1_key(repo_id, blake3);
        let request = ReadRequest::new(key);
        let result = match self.kv.read(request).await {
            Ok(r) => r,
            Err(aspen_core::KeyValueStoreError::NotFound { .. }) => {
                return Ok(None);
            }
            Err(e) => {
                return Err(BridgeError::KvStorage { message: e.to_string() });
            }
        };

        match result.kv {
            Some(kv) => {
                let mapping = HashMapping::from_base64(&kv.value)?;
                let sha1 = mapping.sha1_hash();
                let object_type = mapping.object_type;

                // Update cache
                {
                    let mut cache = self.cache.write();
                    cache.put(*blake3.as_bytes(), CacheEntry { sha1, object_type });
                }

                Ok(Some((sha1, object_type)))
            }
            None => Ok(None),
        }
    }

    /// Look up BLAKE3 hash from SHA-1 hash.
    pub async fn get_blake3(
        &self,
        repo_id: &RepoId,
        sha1: &Sha1Hash,
    ) -> BridgeResult<Option<(blake3::Hash, GitObjectType)>> {
        let key = Self::sha1_to_b3_key(repo_id, sha1);
        let request = ReadRequest::new(key);
        let result = match self.kv.read(request).await {
            Ok(r) => r,
            Err(aspen_core::KeyValueStoreError::NotFound { .. }) => {
                return Ok(None);
            }
            Err(e) => {
                return Err(BridgeError::KvStorage { message: e.to_string() });
            }
        };

        match result.kv {
            Some(kv) => {
                let mapping = HashMapping::from_base64(&kv.value)?;
                Ok(Some((mapping.blake3_hash(), mapping.object_type)))
            }
            None => Ok(None),
        }
    }

    /// Store a bidirectional hash mapping.
    ///
    /// Writes to both BLAKE3->SHA-1 and SHA-1->BLAKE3 indices.
    pub async fn store(
        &self,
        repo_id: &RepoId,
        blake3: blake3::Hash,
        sha1: Sha1Hash,
        object_type: GitObjectType,
    ) -> BridgeResult<()> {
        let mapping = HashMapping::new(blake3, sha1, object_type);
        let value = mapping.to_base64()?;

        // Write BLAKE3 -> SHA-1 mapping
        let b3_key = Self::b3_to_sha1_key(repo_id, &blake3);
        let request = WriteRequest {
            command: WriteCommand::Set {
                key: b3_key,
                value: value.clone(),
            },
        };
        self.kv.write(request).await.map_err(|e| BridgeError::KvStorage { message: e.to_string() })?;

        // Write SHA-1 -> BLAKE3 mapping
        let sha1_key = Self::sha1_to_b3_key(repo_id, &sha1);
        let request = WriteRequest {
            command: WriteCommand::Set { key: sha1_key, value },
        };
        self.kv.write(request).await.map_err(|e| BridgeError::KvStorage { message: e.to_string() })?;

        // Update cache
        {
            let mut cache = self.cache.write();
            cache.put(*blake3.as_bytes(), CacheEntry { sha1, object_type });
        }

        Ok(())
    }

    /// Store multiple hash mappings in a batch.
    ///
    /// More efficient than individual stores for bulk imports.
    /// Respects `MAX_HASH_MAPPING_BATCH_SIZE` limit.
    pub async fn store_batch(
        &self,
        repo_id: &RepoId,
        mappings: &[(blake3::Hash, Sha1Hash, GitObjectType)],
    ) -> BridgeResult<()> {
        if mappings.len() > MAX_HASH_MAPPING_BATCH_SIZE {
            return Err(BridgeError::ImportBatchExceeded {
                count: mappings.len(),
                max: MAX_HASH_MAPPING_BATCH_SIZE,
            });
        }

        // Store each mapping (in production, this would use batch writes)
        for (blake3, sha1, object_type) in mappings {
            self.store(repo_id, *blake3, *sha1, *object_type).await?;
        }

        Ok(())
    }

    /// Check if a BLAKE3 hash has a mapping.
    pub async fn has_blake3(&self, repo_id: &RepoId, blake3: &blake3::Hash) -> BridgeResult<bool> {
        // Check cache first
        {
            let cache = self.cache.read();
            if cache.peek(blake3.as_bytes()).is_some() {
                return Ok(true);
            }
        }

        // Check KV
        let key = Self::b3_to_sha1_key(repo_id, blake3);
        let request = ReadRequest::new(key);
        let result = match self.kv.read(request).await {
            Ok(r) => r,
            Err(aspen_core::KeyValueStoreError::NotFound { .. }) => {
                return Ok(false);
            }
            Err(e) => {
                return Err(BridgeError::KvStorage { message: e.to_string() });
            }
        };

        Ok(result.kv.is_some())
    }

    /// Check if a SHA-1 hash has a mapping.
    pub async fn has_sha1(&self, repo_id: &RepoId, sha1: &Sha1Hash) -> BridgeResult<bool> {
        let key = Self::sha1_to_b3_key(repo_id, sha1);
        let request = ReadRequest::new(key);
        let result = match self.kv.read(request).await {
            Ok(r) => r,
            Err(aspen_core::KeyValueStoreError::NotFound { .. }) => {
                return Ok(false);
            }
            Err(e) => {
                return Err(BridgeError::KvStorage { message: e.to_string() });
            }
        };

        Ok(result.kv.is_some())
    }

    /// Clear the in-memory cache.
    ///
    /// Useful after bulk operations to free memory.
    pub fn clear_cache(&self) {
        let mut cache = self.cache.write();
        cache.clear();
    }

    /// Get cache statistics for monitoring.
    pub fn cache_stats(&self) -> (usize, usize) {
        let cache = self.cache.read();
        (cache.len(), cache.cap().get())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_git_object_type_roundtrip() {
        for obj_type in [
            GitObjectType::Blob,
            GitObjectType::Tree,
            GitObjectType::Commit,
            GitObjectType::Tag,
        ] {
            let s = obj_type.as_str();
            let parsed = GitObjectType::from_str(s).unwrap();
            assert_eq!(parsed, obj_type);
        }
    }

    #[test]
    fn test_hash_mapping_new() {
        let blake3 = blake3::hash(b"test content");
        let sha1 = Sha1Hash::from_bytes([0xab; 20]);
        let mapping = HashMapping::new(blake3, sha1, GitObjectType::Blob);

        assert_eq!(mapping.blake3_hash(), blake3);
        assert_eq!(mapping.sha1_hash(), sha1);
        assert_eq!(mapping.object_type, GitObjectType::Blob);
        assert!(mapping.created_at_ms > 0);
    }

    #[test]
    fn test_hash_mapping_serialization() {
        let blake3 = blake3::hash(b"test content");
        let sha1 = Sha1Hash::from_bytes([0xab; 20]);
        let mapping = HashMapping::new(blake3, sha1, GitObjectType::Commit);

        let bytes = postcard::to_allocvec(&mapping).unwrap();
        let decoded: HashMapping = postcard::from_bytes(&bytes).unwrap();

        assert_eq!(decoded.blake3, mapping.blake3);
        assert_eq!(decoded.sha1, mapping.sha1);
        assert_eq!(decoded.object_type, mapping.object_type);
    }

    #[test]
    fn test_hash_mapping_base64_roundtrip() {
        let blake3 = blake3::hash(b"test content");
        let sha1 = Sha1Hash::from_bytes([0xab; 20]);
        let mapping = HashMapping::new(blake3, sha1, GitObjectType::Tree);

        let encoded = mapping.to_base64().unwrap();
        let decoded = HashMapping::from_base64(&encoded).unwrap();

        assert_eq!(decoded.blake3, mapping.blake3);
        assert_eq!(decoded.sha1, mapping.sha1);
        assert_eq!(decoded.object_type, mapping.object_type);
    }
}

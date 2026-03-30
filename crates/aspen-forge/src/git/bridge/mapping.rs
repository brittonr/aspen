//! Bidirectional hash mapping between BLAKE3 and SHA-1.
//!
//! This module provides persistent storage for mapping between Aspen Forge's
//! BLAKE3 hashes and standard Git's SHA-1 hashes. Mappings are stored in
//! Raft KV for cluster-wide consistency.

use std::num::NonZeroUsize;
use std::sync::Arc;

use aspen_core::KeyValueStore;
use aspen_core::ReadRequest;
use aspen_core::WriteCommand;
use aspen_core::WriteRequest;
use base64::Engine;
use lru::LruCache;
use parking_lot::RwLock;
use serde::Deserialize;
use serde::Serialize;

use super::constants::KV_PREFIX_B3_TO_SHA1;
use super::constants::KV_PREFIX_REMAP;
use super::constants::KV_PREFIX_SHA1_TO_B3;
use super::constants::MAX_HASH_CACHE_SIZE;
use super::constants::MAX_HASH_MAPPING_BATCH_SIZE;
use super::error::BridgeError;
use super::error::BridgeResult;
use super::sha1::Sha1Hash;
use crate::identity::RepoId;

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
    pub fn parse(s: &str) -> Option<Self> {
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
        // MAX_HASH_CACHE_SIZE is a compile-time constant > 0 (asserted in constants.rs).
        // NonZeroUsize::new() is not const-stable; use MIN as a safe fallback.
        let cache_size = NonZeroUsize::new(MAX_HASH_CACHE_SIZE).unwrap_or(NonZeroUsize::MIN);
        Self {
            kv,
            cache: RwLock::new(LruCache::new(cache_size)),
        }
    }

    /// Get a reference to the underlying KV store.
    pub fn kv(&self) -> &Arc<K> {
        &self.kv
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

    /// Store multiple hash mappings using batched KV writes.
    ///
    /// Each mapping produces 2 KV pairs (b3→sha1 and sha1→b3). Pairs are
    /// chunked into `SetMulti` writes of up to `MAX_SETMULTI_KEYS` pairs
    /// each, reducing Raft round-trips from 2×N to ceil(2×N / 100).
    ///
    /// Respects `MAX_HASH_MAPPING_BATCH_SIZE` limit.
    pub async fn store_batch(
        &self,
        repo_id: &RepoId,
        mappings: &[(blake3::Hash, Sha1Hash, GitObjectType)],
    ) -> BridgeResult<()> {
        if mappings.len() > MAX_HASH_MAPPING_BATCH_SIZE {
            return Err(BridgeError::ImportBatchExceeded {
                count: mappings.len() as u32,
                max: MAX_HASH_MAPPING_BATCH_SIZE as u32,
            });
        }

        if mappings.is_empty() {
            return Ok(());
        }

        // Build all KV pairs upfront: 2 per mapping (b3→sha1 + sha1→b3).
        let mut pairs: Vec<(String, String)> = Vec::with_capacity(mappings.len().saturating_mul(2));

        for (blake3, sha1, object_type) in mappings {
            let mapping = HashMapping::new(*blake3, *sha1, *object_type);
            let value = mapping.to_base64()?;

            let b3_key = Self::b3_to_sha1_key(repo_id, blake3);
            let sha1_key = Self::sha1_to_b3_key(repo_id, sha1);

            pairs.push((b3_key, value.clone()));
            pairs.push((sha1_key, value));
        }

        // Chunk into SetMulti writes respecting MAX_SETMULTI_KEYS (100).
        let max_pairs = aspen_core::MAX_SETMULTI_KEYS as usize;
        for chunk in pairs.chunks(max_pairs) {
            let request = WriteRequest {
                command: WriteCommand::SetMulti { pairs: chunk.to_vec() },
            };
            self.kv.write(request).await.map_err(|e| BridgeError::KvStorage {
                message: format!("failed to store batch mappings: {}", e),
            })?;
        }

        // Update cache for all mappings.
        {
            let mut cache = self.cache.write();
            for (blake3, sha1, object_type) in mappings {
                cache.put(*blake3.as_bytes(), CacheEntry {
                    sha1: *sha1,
                    object_type: *object_type,
                });
            }
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

    // ========================================================================
    // Origin→Mirror BLAKE3 remap (federation)
    // ========================================================================

    /// Build the KV key for an origin→mirror BLAKE3 remap entry.
    fn remap_key(repo_id: &RepoId, origin_blake3: &blake3::Hash) -> String {
        format!("{}{}:{}", KV_PREFIX_REMAP, repo_id.to_hex(), hex::encode(origin_blake3.as_bytes()))
    }

    /// Store an origin-BLAKE3 → mirror-BLAKE3 remap entry.
    ///
    /// Written during federation import so the mirror's DAG exporter can
    /// translate embedded origin BLAKE3 references to mirror hashes.
    pub async fn write_remap(
        &self,
        repo_id: &RepoId,
        origin_blake3: blake3::Hash,
        mirror_blake3: blake3::Hash,
    ) -> BridgeResult<()> {
        let key = Self::remap_key(repo_id, &origin_blake3);
        let value = hex::encode(mirror_blake3.as_bytes());
        let request = WriteRequest {
            command: WriteCommand::Set { key, value },
        };
        self.kv.write(request).await.map_err(|e| BridgeError::KvStorage {
            message: format!("failed to write remap entry: {e}"),
        })?;
        Ok(())
    }

    /// Store multiple origin→mirror BLAKE3 remap entries in batched KV writes.
    pub async fn write_remap_batch(
        &self,
        repo_id: &RepoId,
        remaps: &[(blake3::Hash, blake3::Hash)],
    ) -> BridgeResult<()> {
        if remaps.is_empty() {
            return Ok(());
        }

        let pairs: Vec<(String, String)> = remaps
            .iter()
            .map(|(origin, mirror)| {
                let key = Self::remap_key(repo_id, origin);
                let value = hex::encode(mirror.as_bytes());
                (key, value)
            })
            .collect();

        let max_pairs = aspen_core::MAX_SETMULTI_KEYS as usize;
        for chunk in pairs.chunks(max_pairs) {
            let request = WriteRequest {
                command: WriteCommand::SetMulti { pairs: chunk.to_vec() },
            };
            self.kv.write(request).await.map_err(|e| BridgeError::KvStorage {
                message: format!("failed to write remap batch: {e}"),
            })?;
        }

        Ok(())
    }

    /// Look up a mirror BLAKE3 hash from an origin BLAKE3 hash.
    ///
    /// Returns `None` if no remap entry exists (non-federated objects
    /// or objects whose `SyncObject.envelope_hash` was absent).
    pub async fn get_remap(
        &self,
        repo_id: &RepoId,
        origin_blake3: &blake3::Hash,
    ) -> BridgeResult<Option<blake3::Hash>> {
        let key = Self::remap_key(repo_id, origin_blake3);
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
                let bytes = hex::decode(&kv.value).map_err(|e| BridgeError::Serialization {
                    message: format!("invalid remap hex: {e}"),
                })?;
                if bytes.len() != 32 {
                    return Err(BridgeError::Serialization {
                        message: format!("remap value has wrong length: {} (expected 32)", bytes.len()),
                    });
                }
                let mut arr = [0u8; 32];
                arr.copy_from_slice(&bytes);
                Ok(Some(blake3::Hash::from_bytes(arr)))
            }
            None => Ok(None),
        }
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
            let parsed = GitObjectType::parse(s).unwrap();
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

    #[tokio::test]
    async fn test_remap_write_and_get_roundtrip() {
        let kv = aspen_testing_core::DeterministicKeyValueStore::new();
        let store = HashMappingStore::new(kv);
        let repo_id = RepoId::from_hash(blake3::Hash::from_bytes([0x01; 32]));
        let origin_b3 = blake3::hash(b"origin object bytes");
        let mirror_b3 = blake3::hash(b"mirror object bytes");

        store.write_remap(&repo_id, origin_b3, mirror_b3).await.unwrap();
        let got = store.get_remap(&repo_id, &origin_b3).await.unwrap();
        assert_eq!(got, Some(mirror_b3));
    }

    #[tokio::test]
    async fn test_remap_get_nonexistent_returns_none() {
        let kv = aspen_testing_core::DeterministicKeyValueStore::new();
        let store = HashMappingStore::new(kv);
        let repo_id = RepoId::from_hash(blake3::Hash::from_bytes([0x02; 32]));
        let missing_b3 = blake3::hash(b"never stored");

        let got = store.get_remap(&repo_id, &missing_b3).await.unwrap();
        assert_eq!(got, None);
    }
}

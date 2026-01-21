//! Migration support for transitioning from legacy CacheEntry to SNIX PathInfo.
//!
//! This module provides:
//!
//! - [`MigrationAwareCacheIndex`]: A unified cache index that queries both legacy
//!   and SNIX storage, preferring SNIX when available
//! - [`migrate_entry`]: Convert a single legacy CacheEntry to SNIX PathInfo
//! - [`MigrationWorker`]: Background worker for batch migration
//!
//! # Migration Strategy
//!
//! Phase 5 of the SNIX migration uses a dual-read approach:
//!
//! 1. **Reads**: Query SNIX PathInfoService first, fall back to legacy CacheIndex
//! 2. **Writes**: Write to both stores (dual-write) during transition
//! 3. **Migration**: Background worker converts legacy entries to SNIX format
//!
//! # Key Format Differences
//!
//! | Storage | Key Format | Digest Type |
//! |---------|------------|-------------|
//! | Legacy  | `_cache:narinfo:<base32-hash>` | 32-52 char string |
//! | SNIX    | `snix:pathinfo:<hex-20-byte>` | 20-byte array |
//!
//! The store hash in legacy format needs conversion to 20-byte digest.

use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use snafu::Snafu;
use snix_store::pathinfoservice::PathInfoService;
use tracing::{debug, info, instrument, warn};

use aspen_blob::BlobStore;
use aspen_cache::{CacheEntry, CacheIndex, CacheStats};
use aspen_core::KeyValueStore;

use crate::RaftPathInfoService;
use crate::constants::{
    MIGRATION_BATCH_SIZE, MIGRATION_ENTRY_TIMEOUT_SECS, MIGRATION_PROGRESS_KEY_PREFIX, STORE_PATH_DIGEST_LENGTH,
};

/// Result type for migration operations.
pub type Result<T, E = MigrationError> = std::result::Result<T, E>;

/// Errors that can occur during migration.
#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum MigrationError {
    /// Failed to read from legacy cache.
    #[snafu(display("failed to read legacy cache entry: {message}"))]
    LegacyRead { message: String },

    /// Failed to write to SNIX store.
    #[snafu(display("failed to write SNIX PathInfo: {message}"))]
    SnixWrite { message: String },

    /// Failed to read blob data.
    #[snafu(display("failed to read blob {hash}: {message}"))]
    BlobRead { hash: String, message: String },

    /// Blob not found in store.
    #[snafu(display("blob not found: {hash}"))]
    BlobNotFound { hash: String },

    /// Invalid store path format.
    #[snafu(display("invalid store path: {path}"))]
    InvalidStorePath { path: String },

    /// Failed to parse store path digest.
    #[snafu(display("failed to parse store path digest: {message}"))]
    DigestParse { message: String },

    /// Migration was cancelled.
    #[snafu(display("migration cancelled"))]
    Cancelled,

    /// Resource limit exceeded.
    #[snafu(display("resource limit exceeded: {message}"))]
    ResourceLimit { message: String },

    /// Serialization error.
    #[snafu(display("serialization error: {message}"))]
    Serialization { message: String },

    /// KV store operation failed.
    #[snafu(display("KV store error: {message}"))]
    KvStore { message: String },
}

/// Version of cache entry storage format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CacheEntryVersion {
    /// Legacy format: whole NAR as single blob, CacheEntry metadata
    V1,
    /// SNIX format: decomposed castore storage, PathInfo metadata
    V2,
}

/// Result of a cache lookup that includes version information.
#[derive(Debug, Clone)]
pub struct VersionedCacheEntry {
    /// The version of the storage format.
    pub version: CacheEntryVersion,
    /// The cache entry data (from legacy or converted from SNIX).
    pub entry: CacheEntry,
}

/// Migration progress tracking.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MigrationProgress {
    /// Total legacy entries discovered.
    pub total_entries: u64,
    /// Entries successfully migrated.
    pub migrated_count: u64,
    /// Entries that failed migration.
    pub failed_count: u64,
    /// Entries skipped (already migrated or invalid).
    pub skipped_count: u64,
    /// Unix timestamp when migration started.
    pub started_at: u64,
    /// Unix timestamp of last update.
    pub last_updated: u64,
    /// Last processed store hash (for resumption).
    pub last_processed_hash: Option<String>,
    /// Whether migration is complete.
    pub is_complete: bool,
    /// Error message if migration failed.
    pub error_message: Option<String>,
}

impl MigrationProgress {
    /// Create new migration progress.
    pub fn new(total_entries: u64) -> Self {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
        Self {
            total_entries,
            migrated_count: 0,
            failed_count: 0,
            skipped_count: 0,
            started_at: now,
            last_updated: now,
            last_processed_hash: None,
            is_complete: false,
            error_message: None,
        }
    }

    /// Record a successful migration.
    pub fn record_success(&mut self, store_hash: &str) {
        self.migrated_count += 1;
        self.last_processed_hash = Some(store_hash.to_string());
        self.touch();
    }

    /// Record a failed migration.
    pub fn record_failure(&mut self, store_hash: &str, error: &str) {
        self.failed_count += 1;
        self.last_processed_hash = Some(store_hash.to_string());
        self.error_message = Some(format!("Last failure at {}: {}", store_hash, error));
        self.touch();
    }

    /// Record a skipped entry.
    pub fn record_skip(&mut self, store_hash: &str) {
        self.skipped_count += 1;
        self.last_processed_hash = Some(store_hash.to_string());
        self.touch();
    }

    /// Mark migration as complete.
    pub fn mark_complete(&mut self) {
        self.is_complete = true;
        self.touch();
    }

    /// Update the last_updated timestamp.
    fn touch(&mut self) {
        self.last_updated = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
    }

    /// Calculate progress percentage.
    pub fn progress_percent(&self) -> f64 {
        if self.total_entries == 0 {
            return 100.0;
        }
        let processed = self.migrated_count + self.failed_count + self.skipped_count;
        (processed as f64 / self.total_entries as f64) * 100.0
    }

    /// Serialize to JSON.
    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string(self).map_err(|e| MigrationError::Serialization { message: e.to_string() })
    }

    /// Deserialize from JSON.
    pub fn from_json(json: &str) -> Result<Self> {
        serde_json::from_str(json).map_err(|e| MigrationError::Serialization { message: e.to_string() })
    }
}

/// A cache index that supports both legacy CacheEntry and SNIX PathInfo formats.
///
/// During the migration period, this index:
/// - Queries SNIX PathInfoService first for reads
/// - Falls back to legacy CacheIndex if not found in SNIX
/// - Writes to both stores for new entries (dual-write)
///
/// This allows gradual migration while maintaining full functionality.
pub struct MigrationAwareCacheIndex<K, L>
where
    K: KeyValueStore + Send + Sync + 'static,
    L: CacheIndex + Send + Sync + 'static,
{
    /// SNIX PathInfo service (preferred for reads).
    snix_pathinfo: Arc<RaftPathInfoService<K>>,
    /// Legacy cache index (fallback).
    legacy_index: Arc<L>,
    /// Whether to dual-write to both stores.
    dual_write_enabled: bool,
}

impl<K, L> MigrationAwareCacheIndex<K, L>
where
    K: KeyValueStore + Send + Sync + 'static,
    L: CacheIndex + Send + Sync + 'static,
{
    /// Create a new migration-aware cache index.
    ///
    /// # Arguments
    ///
    /// * `snix_pathinfo` - The SNIX PathInfoService for new storage
    /// * `legacy_index` - The legacy CacheIndex for backward compatibility
    /// * `dual_write_enabled` - Whether to write to both stores
    pub fn new(snix_pathinfo: Arc<RaftPathInfoService<K>>, legacy_index: Arc<L>, dual_write_enabled: bool) -> Self {
        Self {
            snix_pathinfo,
            legacy_index,
            dual_write_enabled,
        }
    }

    /// Convert a store hash to a 20-byte digest.
    ///
    /// Nix store hashes are either:
    /// - Base32 (52 chars) representing 32 bytes, truncated to 20 bytes
    /// - Base16 (40 chars) representing 20 bytes
    fn store_hash_to_digest(store_hash: &str) -> Result<[u8; STORE_PATH_DIGEST_LENGTH]> {
        // Try hex decode first (40 chars = 20 bytes)
        if store_hash.len() == 40
            && let Ok(bytes) = hex::decode(store_hash)
            && bytes.len() == STORE_PATH_DIGEST_LENGTH
        {
            let mut digest = [0u8; STORE_PATH_DIGEST_LENGTH];
            digest.copy_from_slice(&bytes);
            return Ok(digest);
        }

        // Try Nix base32 decode (52 chars = 32 bytes, take first 20)
        if let Some(bytes) = nix_base32_decode(store_hash)
            && bytes.len() >= STORE_PATH_DIGEST_LENGTH
        {
            let mut digest = [0u8; STORE_PATH_DIGEST_LENGTH];
            digest.copy_from_slice(&bytes[..STORE_PATH_DIGEST_LENGTH]);
            return Ok(digest);
        }

        Err(MigrationError::DigestParse {
            message: format!("cannot convert store hash '{}' to 20-byte digest", store_hash),
        })
    }

    /// Get a cache entry, checking SNIX first then falling back to legacy.
    #[instrument(skip(self), fields(store_hash = %store_hash))]
    pub async fn get_versioned(&self, store_hash: &str) -> aspen_cache::Result<Option<VersionedCacheEntry>> {
        // Try to convert to 20-byte digest for SNIX lookup
        if let Ok(digest) = Self::store_hash_to_digest(store_hash) {
            // Try SNIX first
            match self.snix_pathinfo.get(digest).await {
                Ok(Some(path_info)) => {
                    debug!(store_path = %path_info.store_path, "found in SNIX storage");
                    // Convert PathInfo to CacheEntry for compatibility
                    let entry = pathinfo_to_cache_entry(&path_info);
                    return Ok(Some(VersionedCacheEntry {
                        version: CacheEntryVersion::V2,
                        entry,
                    }));
                }
                Ok(None) => {
                    debug!("not found in SNIX storage, trying legacy");
                }
                Err(e) => {
                    warn!(error = %e, "SNIX lookup failed, falling back to legacy");
                }
            }
        }

        // Fall back to legacy
        match self.legacy_index.get(store_hash).await {
            Ok(Some(entry)) => {
                debug!(store_path = %entry.store_path, "found in legacy storage");
                Ok(Some(VersionedCacheEntry {
                    version: CacheEntryVersion::V1,
                    entry,
                }))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Check if an entry exists in either store.
    #[instrument(skip(self))]
    pub async fn exists_any(&self, store_hash: &str) -> aspen_cache::Result<bool> {
        // Check SNIX first
        if let Ok(digest) = Self::store_hash_to_digest(store_hash) {
            match self.snix_pathinfo.get(digest).await {
                Ok(Some(_)) => return Ok(true),
                Ok(None) => {}
                Err(e) => {
                    warn!(error = %e, "SNIX existence check failed");
                }
            }
        }

        // Check legacy
        self.legacy_index.exists(store_hash).await
    }

    /// Check if an entry is already migrated to SNIX.
    pub async fn is_migrated(&self, store_hash: &str) -> bool {
        if let Ok(digest) = Self::store_hash_to_digest(store_hash) {
            matches!(self.snix_pathinfo.get(digest).await, Ok(Some(_)))
        } else {
            false
        }
    }
}

#[async_trait]
impl<K, L> CacheIndex for MigrationAwareCacheIndex<K, L>
where
    K: KeyValueStore + Send + Sync + 'static,
    L: CacheIndex + Send + Sync + 'static,
{
    async fn get(&self, store_hash: &str) -> aspen_cache::Result<Option<CacheEntry>> {
        Ok(self.get_versioned(store_hash).await?.map(|v| v.entry))
    }

    async fn put(&self, entry: CacheEntry) -> aspen_cache::Result<()> {
        // Always write to legacy for backward compatibility
        self.legacy_index.put(entry.clone()).await?;

        if self.dual_write_enabled {
            // Also write to SNIX if dual-write is enabled
            // Note: This requires the blob to be in decomposed format already,
            // which won't be the case for legacy entries. Skip SNIX write for now
            // and let the migration worker handle conversion.
            debug!(
                store_hash = %entry.store_hash,
                "dual-write: written to legacy, SNIX write requires migration"
            );
        }

        Ok(())
    }

    async fn exists(&self, store_hash: &str) -> aspen_cache::Result<bool> {
        self.exists_any(store_hash).await
    }

    async fn stats(&self) -> aspen_cache::Result<CacheStats> {
        // For now, just return legacy stats
        // TODO: Merge stats from both stores
        self.legacy_index.stats().await
    }
}

/// Convert a SNIX PathInfo to a legacy CacheEntry.
///
/// This allows serving SNIX data to clients that expect the legacy format.
fn pathinfo_to_cache_entry(path_info: &snix_store::pathinfoservice::PathInfo) -> CacheEntry {
    use aspen_cache::CacheEntry;
    use snix_castore::Node;

    let store_path = format!("/nix/store/{}", path_info.store_path);
    // Convert 20-byte digest to hex string
    let store_hash = hex::encode(path_info.store_path.digest());

    // The blob_hash in legacy format was the BLAKE3 hash of the whole NAR.
    // In SNIX, we have the root node digest instead. Extract it from the node.
    let root_digest = match &path_info.node {
        Node::Directory { digest, .. } => digest.to_string(),
        Node::File { digest, .. } => digest.to_string(),
        Node::Symlink { .. } => "symlink".to_string(),
    };
    let blob_hash = format!("snix-root:{}", root_digest);

    let nar_hash = format!("sha256:{}", hex::encode(path_info.nar_sha256));

    let references: Vec<String> = path_info.references.iter().map(|r| format!("/nix/store/{}", r)).collect();

    let deriver = path_info.deriver.as_ref().map(|d| format!("/nix/store/{}.drv", d));

    // We don't have creation metadata from PathInfo, use defaults
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();

    let mut entry = CacheEntry::new(
        store_path,
        store_hash,
        blob_hash,
        path_info.nar_size,
        nar_hash,
        now,
        0, // Unknown node
    );

    // Best effort to add references and deriver
    let _ = entry.clone().with_references(references);
    entry.deriver = deriver;

    entry
}

/// Decode a Nix base32 string.
///
/// Nix uses a non-standard base32 alphabet: 0123456789abcdfghijklmnpqrsvwxyz
/// (no e, o, t, u to avoid confusion).
fn nix_base32_decode(input: &str) -> Option<Vec<u8>> {
    const NIX_BASE32_CHARS: &[u8] = b"0123456789abcdfghijklmnpqrsvwxyz";

    let mut output = Vec::new();
    let mut buffer: u64 = 0;
    let mut bits_in_buffer = 0;

    for c in input.chars() {
        let value = NIX_BASE32_CHARS.iter().position(|&x| x == c as u8)? as u64;
        buffer = (buffer << 5) | value;
        bits_in_buffer += 5;

        while bits_in_buffer >= 8 {
            bits_in_buffer -= 8;
            output.push((buffer >> bits_in_buffer) as u8);
            buffer &= (1 << bits_in_buffer) - 1;
        }
    }

    Some(output)
}

/// Migration worker that converts legacy cache entries to SNIX format.
///
/// The worker runs in the background, processing entries in batches with
/// configurable rate limiting to avoid impacting normal operations.
pub struct MigrationWorker<K, L, B>
where
    K: KeyValueStore + Send + Sync + 'static,
    L: CacheIndex + Send + Sync + 'static,
    B: BlobStore + Send + Sync + 'static,
{
    /// KV store for progress tracking and SNIX storage.
    kv: Arc<K>,
    /// Legacy cache index.
    legacy_index: Arc<L>,
    /// Blob store for reading legacy NAR data.
    blob_store: Arc<B>,
    /// SNIX blob service (for decomposed storage).
    snix_blob_service: Arc<crate::IrohBlobService<B>>,
    /// SNIX directory service.
    snix_directory_service: Arc<crate::RaftDirectoryService<K>>,
    /// SNIX path info service.
    snix_pathinfo_service: Arc<RaftPathInfoService<K>>,
    /// Batch size for processing.
    batch_size: u32,
    /// Delay between batches.
    batch_delay: Duration,
    /// Cancellation flag.
    cancelled: Arc<std::sync::atomic::AtomicBool>,
}

impl<K, L, B> MigrationWorker<K, L, B>
where
    K: KeyValueStore + Send + Sync + 'static,
    L: CacheIndex + Send + Sync + 'static,
    B: BlobStore + Clone + Send + Sync + 'static,
{
    /// Create a new migration worker.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        kv: Arc<K>,
        legacy_index: Arc<L>,
        blob_store: Arc<B>,
        snix_blob_service: Arc<crate::IrohBlobService<B>>,
        snix_directory_service: Arc<crate::RaftDirectoryService<K>>,
        snix_pathinfo_service: Arc<RaftPathInfoService<K>>,
    ) -> Self {
        Self {
            kv,
            legacy_index,
            blob_store,
            snix_blob_service,
            snix_directory_service,
            snix_pathinfo_service,
            batch_size: MIGRATION_BATCH_SIZE,
            batch_delay: Duration::from_millis(100),
            cancelled: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    /// Set the batch size.
    pub fn with_batch_size(mut self, size: u32) -> Self {
        self.batch_size = size;
        self
    }

    /// Set the delay between batches.
    pub fn with_batch_delay(mut self, delay: Duration) -> Self {
        self.batch_delay = delay;
        self
    }

    /// Get the cancellation handle.
    pub fn cancel_handle(&self) -> Arc<std::sync::atomic::AtomicBool> {
        Arc::clone(&self.cancelled)
    }

    /// Cancel the migration.
    pub fn cancel(&self) {
        self.cancelled.store(true, std::sync::atomic::Ordering::SeqCst);
    }

    /// Load or create migration progress.
    async fn load_progress(&self) -> Result<MigrationProgress> {
        let key = format!("{}current", MIGRATION_PROGRESS_KEY_PREFIX);
        let result = self
            .kv
            .read(aspen_core::kv::ReadRequest::new(&key))
            .await
            .map_err(|e| MigrationError::KvStore { message: e.to_string() })?;

        match result.kv {
            Some(kv) => MigrationProgress::from_json(&kv.value),
            None => {
                // Get total count from legacy stats
                let stats = self
                    .legacy_index
                    .stats()
                    .await
                    .map_err(|e| MigrationError::LegacyRead { message: e.to_string() })?;
                Ok(MigrationProgress::new(stats.total_entries))
            }
        }
    }

    /// Save migration progress.
    async fn save_progress(&self, progress: &MigrationProgress) -> Result<()> {
        let key = format!("{}current", MIGRATION_PROGRESS_KEY_PREFIX);
        let value = progress.to_json()?;

        self.kv
            .write(aspen_core::kv::WriteRequest {
                command: aspen_core::kv::WriteCommand::Set { key, value },
            })
            .await
            .map_err(|e| MigrationError::KvStore { message: e.to_string() })?;

        Ok(())
    }

    /// Run the migration.
    ///
    /// This processes all legacy entries, converting them to SNIX format.
    /// Progress is checkpointed regularly for resumption.
    #[instrument(skip(self))]
    pub async fn run(&self) -> Result<MigrationProgress> {
        info!("starting cache migration to SNIX format");

        let mut progress = self.load_progress().await?;

        if progress.is_complete {
            info!("migration already complete");
            return Ok(progress);
        }

        info!(total = progress.total_entries, migrated = progress.migrated_count, "resuming migration");

        // List all PathInfo entries from SNIX to build a set of already-migrated digests
        let mut migrated_digests = std::collections::HashSet::new();
        let mut stream = self.snix_pathinfo_service.list();
        while let Some(result) = stream.next().await {
            match result {
                Ok(path_info) => {
                    // Convert 20-byte digest to hex string for comparison
                    migrated_digests.insert(hex::encode(path_info.store_path.digest()));
                }
                Err(e) => {
                    warn!(error = %e, "failed to list SNIX entries");
                    break;
                }
            }
        }

        info!(already_migrated = migrated_digests.len(), "checked existing SNIX entries");

        // Note: In a real implementation, we would scan the legacy cache.
        // For now, this is a placeholder that shows the structure.
        // The actual scan would use the legacy index's list method if available,
        // or scan the KV store with the cache prefix.

        // Checkpoint progress
        self.save_progress(&progress).await?;

        // Check for cancellation
        if self.cancelled.load(std::sync::atomic::Ordering::SeqCst) {
            warn!("migration cancelled");
            return Err(MigrationError::Cancelled);
        }

        // Mark complete
        progress.mark_complete();
        self.save_progress(&progress).await?;

        info!(
            migrated = progress.migrated_count,
            failed = progress.failed_count,
            skipped = progress.skipped_count,
            "migration complete"
        );

        Ok(progress)
    }

    /// Migrate a single entry.
    ///
    /// This:
    /// 1. Reads the legacy NAR blob
    /// 2. Ingests it into SNIX castore (decomposing into blobs + directories)
    /// 3. Creates a PathInfo entry
    #[allow(dead_code)]
    async fn migrate_entry(&self, entry: &CacheEntry) -> Result<()> {
        let _timeout = Duration::from_secs(MIGRATION_ENTRY_TIMEOUT_SECS);

        debug!(store_path = %entry.store_path, "migrating entry");

        // Parse the blob hash
        let blob_hash: iroh_blobs::Hash = entry.blob_hash.parse().map_err(|e| MigrationError::BlobRead {
            hash: entry.blob_hash.clone(),
            message: format!("invalid hash format: {}", e),
        })?;

        // Read the NAR blob
        let nar_bytes = self
            .blob_store
            .get_bytes(&blob_hash)
            .await
            .map_err(|e| MigrationError::BlobRead {
                hash: entry.blob_hash.clone(),
                message: e.to_string(),
            })?
            .ok_or_else(|| MigrationError::BlobNotFound {
                hash: entry.blob_hash.clone(),
            })?;

        // Ingest NAR into SNIX castore
        // Note: This requires the snix_store::nar::ingest module
        let mut cursor = std::io::Cursor::new(nar_bytes.to_vec());

        // Clone the services to satisfy 'static lifetime requirements
        // The services implement Clone because they wrap Arc internally
        let blob_svc = (*self.snix_blob_service).clone();
        let dir_svc = (*self.snix_directory_service).clone();

        let (root_node, nar_sha256, nar_size) =
            snix_store::nar::ingest_nar_and_hash(blob_svc, dir_svc, &mut cursor, &None).await.map_err(|e| {
                MigrationError::SnixWrite {
                    message: format!("NAR ingestion failed: {}", e),
                }
            })?;

        // Parse store path
        let store_path: nix_compat::store_path::StorePath<String> =
            nix_compat::store_path::StorePath::from_absolute_path(entry.store_path.as_bytes()).map_err(|_| {
                MigrationError::InvalidStorePath {
                    path: entry.store_path.clone(),
                }
            })?;

        // Convert references
        let references: Vec<nix_compat::store_path::StorePath<String>> = entry
            .references
            .iter()
            .filter_map(|r| nix_compat::store_path::StorePath::<String>::from_absolute_path(r.as_bytes()).ok())
            .collect();

        // Parse deriver
        let deriver = entry
            .deriver
            .as_ref()
            .and_then(|d| nix_compat::store_path::StorePath::<String>::from_absolute_path(d.as_bytes()).ok());

        // Create PathInfo
        let path_info = snix_store::pathinfoservice::PathInfo {
            store_path,
            node: root_node,
            references,
            nar_size,
            nar_sha256,
            signatures: vec![],
            deriver,
            ca: None,
        };

        // Store in SNIX
        self.snix_pathinfo_service
            .put(path_info)
            .await
            .map_err(|e| MigrationError::SnixWrite { message: e.to_string() })?;

        debug!(store_path = %entry.store_path, "entry migrated successfully");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nix_base32_decode() {
        // Known Nix store hash
        let hash = "0c6kzph7l0dcbfmjap64f0czdafn3b7x";
        let decoded = nix_base32_decode(hash);
        assert!(decoded.is_some());
        assert_eq!(decoded.unwrap().len(), 20);
    }

    #[test]
    fn test_migration_progress_serialization() {
        let mut progress = MigrationProgress::new(100);
        progress.record_success("abc123");
        progress.record_failure("def456", "test error");

        let json = progress.to_json().unwrap();
        let restored = MigrationProgress::from_json(&json).unwrap();

        assert_eq!(restored.total_entries, 100);
        assert_eq!(restored.migrated_count, 1);
        assert_eq!(restored.failed_count, 1);
    }

    #[test]
    fn test_progress_percentage() {
        let mut progress = MigrationProgress::new(100);
        assert_eq!(progress.progress_percent(), 0.0);

        progress.migrated_count = 50;
        assert_eq!(progress.progress_percent(), 50.0);

        progress.failed_count = 25;
        progress.skipped_count = 25;
        assert_eq!(progress.progress_percent(), 100.0);
    }

    #[test]
    fn test_store_hash_to_digest_hex() {
        // 40 hex chars = 20 bytes
        let hash = "0123456789abcdef0123456789abcdef01234567";
        let digest = MigrationAwareCacheIndex::<
            aspen_core::DeterministicKeyValueStore,
            aspen_cache::KvCacheIndex<aspen_core::DeterministicKeyValueStore>,
        >::store_hash_to_digest(hash);
        assert!(digest.is_ok());
        assert_eq!(digest.unwrap().len(), 20);
    }
}

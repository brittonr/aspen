//! Git importer service.
//!
//! Imports objects from standard Git repositories into Aspen Forge.
//! Handles packfile parsing, object conversion, and hash mapping.
//!
//! # Processing Order
//!
//! Objects are imported using wave-based parallelism. The topological sort
//! groups objects into "waves" where all objects in a wave have no dependencies
//! on each other (only on objects in earlier waves). This allows concurrent
//! import within each wave while maintaining correct dependency order.
//!
//! Wave 0: Objects with no in-set dependencies (typically blobs)
//! Wave 1: Objects depending only on wave 0 objects
//! Wave N: Objects depending only on objects in waves < N
//!
//! Each wave is processed with `buffer_unordered(MAX_IMPORT_CONCURRENCY)`,
//! and we wait for all objects in a wave to complete before starting the next.

use std::sync::Arc;

use aspen_blob::prelude::*;
use aspen_core::KeyValueStore;
use aspen_core::hlc::HLC;
use futures::StreamExt;

use super::constants::MAX_IMPORT_BATCH_SIZE;
use super::constants::MAX_IMPORT_CONCURRENCY;
use super::converter::GitObjectConverter;
use super::error::BridgeError;
use super::error::BridgeResult;
use super::mapping::GitObjectType;
use super::mapping::HashMappingStore;
use super::sha1::Sha1Hash;
use super::topological::ObjectCollector;
use super::topological::PendingObject;
use super::topological::extract_commit_dependencies;
use super::topological::extract_tag_dependencies;
use super::topological::extract_tree_dependencies;
use crate::identity::RepoId;
use crate::refs::RefStore;

/// Result of an import operation.
#[derive(Debug)]
pub struct ImportResult {
    /// Number of objects imported.
    pub objects_imported: u32,
    /// Number of objects skipped (already had mappings).
    pub objects_skipped: u32,
    /// Refs that were updated.
    pub refs_updated: Vec<(String, blake3::Hash)>,
}

/// Service for importing Git objects into Forge.
pub struct GitImporter<K: KeyValueStore + ?Sized, B> {
    /// Hash mapping store.
    mapping: Arc<HashMappingStore<K>>,
    /// Object converter.
    converter: GitObjectConverter<K>,
    /// Blob store for storing imported objects.
    blobs: Arc<B>,
    /// Ref store for updating refs.
    refs: Arc<RefStore<K>>,
}

impl<K: KeyValueStore + ?Sized, B: BlobStore> GitImporter<K, B> {
    /// Create a new git importer.
    pub fn new(
        mapping: Arc<HashMappingStore<K>>,
        blobs: Arc<B>,
        refs: Arc<RefStore<K>>,
        secret_key: iroh::SecretKey,
        hlc: HLC,
    ) -> Self {
        let converter = GitObjectConverter::new(Arc::clone(&mapping), secret_key, hlc);
        Self {
            mapping,
            converter,
            blobs,
            refs,
        }
    }

    /// Import a single git object.
    ///
    /// The object should be in raw git format (type + size + NUL + content).
    /// Returns the BLAKE3 hash of the imported object.
    ///
    /// Note: For objects with dependencies (trees, commits, tags), the
    /// dependencies must already be imported and have hash mappings.
    pub async fn import_object(&self, repo_id: &RepoId, git_bytes: &[u8]) -> BridgeResult<blake3::Hash> {
        let (obj_type, content) = GitObjectConverter::<K>::split_git_object(git_bytes)?;

        let (signed, sha1, blake3) = match obj_type {
            GitObjectType::Blob => self.converter.import_blob(content)?,
            GitObjectType::Tree => self.converter.import_tree(repo_id, content).await?,
            GitObjectType::Commit => self.converter.import_commit(repo_id, content).await?,
            GitObjectType::Tag => self.converter.import_tag(repo_id, content).await?,
        };

        // Store the object
        let bytes = signed.to_bytes();
        self.blobs
            .add_bytes(&bytes)
            .await
            .map_err(|e| BridgeError::BlobStorage { message: e.to_string() })?;

        // Store the hash mapping
        self.mapping.store(repo_id, blake3, sha1, obj_type).await.map_err(|e| BridgeError::KvStorage {
            message: format!("failed to store hash mapping for {}: {}", sha1.to_hex(), e),
        })?;

        Ok(blake3)
    }

    /// Import multiple objects using wave-based parallelism.
    ///
    /// Objects are sorted into waves where all objects in a wave can be
    /// processed concurrently (they have no dependencies on each other).
    /// Waves are processed sequentially to ensure dependencies are satisfied.
    ///
    /// This provides significant speedup over sequential import while
    /// maintaining correctness: a typical Git push with many blobs will
    /// have most blobs in wave 0, processed in parallel.
    ///
    /// Returns import statistics.
    pub async fn import_objects(
        &self,
        repo_id: &RepoId,
        objects: Vec<(Sha1Hash, GitObjectType, Vec<u8>)>,
    ) -> BridgeResult<ImportResult> {
        if objects.len() > MAX_IMPORT_BATCH_SIZE {
            return Err(BridgeError::ImportBatchExceeded {
                count: objects.len() as u32,
                max: MAX_IMPORT_BATCH_SIZE as u32,
            });
        }

        // Build pending objects with dependencies
        let mut collector = ObjectCollector::new();

        for (sha1, obj_type, data) in objects {
            // Check if already imported
            if self.mapping.has_sha1(repo_id, &sha1).await? {
                collector.mark_existing(sha1);
                continue;
            }

            // Extract dependencies based on object type
            let (_, content) = GitObjectConverter::<K>::split_git_object(&data)?;
            let deps = match obj_type {
                GitObjectType::Blob => Vec::new(),
                GitObjectType::Tree => extract_tree_dependencies(content)?,
                GitObjectType::Commit => {
                    let content_str = std::str::from_utf8(content)?;
                    extract_commit_dependencies(content_str)?
                }
                GitObjectType::Tag => {
                    let content_str = std::str::from_utf8(content)?;
                    extract_tag_dependencies(content_str)?
                }
            };

            collector.add(PendingObject::new(sha1, obj_type, data, deps))?;
        }

        // Sort into waves for parallel processing
        let waves = collector.finish_waves()?;
        let skipped_count = waves.skipped.len() as u32;
        let total_objects: u32 = waves.waves.iter().map(|w| w.len() as u32).sum();
        let wave_count = waves.waves.len();

        // Import objects wave by wave with parallelism within each wave.
        //
        // Objects in the same wave have no dependencies on each other,
        // so they can be imported concurrently. We must wait for all
        // objects in a wave to complete before starting the next wave
        // because later waves depend on mappings created in earlier waves.
        let repo_id = *repo_id;
        let mut imported = 0u32;

        for (wave_idx, wave) in waves.waves.into_iter().enumerate() {
            let wave_size = wave.len();

            // Process all objects in this wave concurrently
            let results: Vec<BridgeResult<blake3::Hash>> = futures::stream::iter(wave)
                .map(|obj| async move { self.import_object(&repo_id, &obj.data).await })
                .buffer_unordered(MAX_IMPORT_CONCURRENCY)
                .collect()
                .await;

            // Check for errors and count successful imports
            for result in results {
                result?;
                imported += 1;
            }

            tracing::trace!(wave = wave_idx, objects = wave_size, "wave import complete");
        }

        tracing::debug!(
            total = total_objects,
            imported = imported,
            skipped = skipped_count,
            waves = wave_count,
            "wave-based parallel import complete"
        );

        Ok(ImportResult {
            objects_imported: imported,
            objects_skipped: skipped_count,
            refs_updated: Vec::new(),
        })
    }

    /// Import objects and update a ref.
    ///
    /// Imports all objects reachable from the given commit, then updates
    /// the specified ref to point to the imported commit.
    pub async fn import_ref(
        &self,
        repo_id: &RepoId,
        ref_name: &str,
        commit_sha1: Sha1Hash,
        objects: Vec<(Sha1Hash, GitObjectType, Vec<u8>)>,
    ) -> BridgeResult<ImportResult> {
        // Import all objects
        let mut result = self.import_objects(repo_id, objects).await.map_err(|e| BridgeError::ImportFailed {
            message: format!("failed to import objects for ref {}: {}", ref_name, e),
        })?;

        // Get the BLAKE3 hash for the commit
        let (commit_blake3, _) =
            self.mapping.get_blake3(repo_id, &commit_sha1).await?.ok_or_else(|| BridgeError::MappingNotFound {
                hash: commit_sha1.to_hex(),
            })?;

        // Update the ref
        self.refs
            .set(repo_id, ref_name, commit_blake3)
            .await
            .map_err(|e| BridgeError::KvStorage { message: e.to_string() })?;

        result.refs_updated.push((ref_name.to_string(), commit_blake3));

        Ok(result)
    }

    /// Check if a SHA-1 hash already has a mapping.
    pub async fn has_object(&self, repo_id: &RepoId, sha1: &Sha1Hash) -> BridgeResult<bool> {
        self.mapping.has_sha1(repo_id, sha1).await
    }

    /// Get the BLAKE3 hash for a SHA-1 hash.
    pub async fn get_blake3(&self, repo_id: &RepoId, sha1: &Sha1Hash) -> BridgeResult<Option<blake3::Hash>> {
        Ok(self.mapping.get_blake3(repo_id, sha1).await?.map(|(h, _)| h))
    }

    /// Import a single object from raw content (without git header).
    ///
    /// This is a convenience method for the RPC handler where objects arrive
    /// with separate type and content fields.
    ///
    /// Returns the BLAKE3 hash and whether the object already existed.
    pub async fn import_object_raw(
        &self,
        repo_id: &RepoId,
        sha1: Sha1Hash,
        object_type: &str,
        content: &[u8],
    ) -> BridgeResult<SingleImportResult> {
        // Check if already imported
        if self.mapping.has_sha1(repo_id, &sha1).await? {
            let (blake3, _) = self
                .mapping
                .get_blake3(repo_id, &sha1)
                .await?
                .ok_or_else(|| BridgeError::MappingNotFound { hash: sha1.to_hex() })?;
            return Ok(SingleImportResult {
                blake3,
                already_existed: true,
            });
        }

        // Build full git object bytes
        let header = format!("{} {}\0", object_type, content.len());
        let mut git_bytes = Vec::with_capacity(header.len() + content.len());
        git_bytes.extend_from_slice(header.as_bytes());
        git_bytes.extend_from_slice(content);

        // Import
        let blake3 = self.import_object(repo_id, &git_bytes).await?;

        Ok(SingleImportResult {
            blake3,
            already_existed: false,
        })
    }

    /// Update a ref to point to a new SHA-1 hash.
    ///
    /// The SHA-1 must already have a BLAKE3 mapping (i.e., the object must be imported).
    pub async fn update_ref(&self, repo_id: &RepoId, ref_name: &str, sha1: Sha1Hash) -> BridgeResult<blake3::Hash> {
        // Get the BLAKE3 hash for the SHA-1
        let (blake3, _) = self
            .mapping
            .get_blake3(repo_id, &sha1)
            .await?
            .ok_or_else(|| BridgeError::MappingNotFound { hash: sha1.to_hex() })?;

        // Update the ref
        self.refs
            .set(repo_id, ref_name, blake3)
            .await
            .map_err(|e| BridgeError::KvStorage { message: e.to_string() })?;

        Ok(blake3)
    }
}

/// Result of importing a single object.
#[derive(Debug)]
pub struct SingleImportResult {
    /// BLAKE3 hash of the imported object.
    pub blake3: blake3::Hash,
    /// Whether the object already existed (skipped).
    pub already_existed: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    // Integration tests would require mocking the blob store and KV store
    // Unit tests for the public API would go here
}

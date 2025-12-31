//! Git importer service.
//!
//! Imports objects from standard Git repositories into Aspen Forge.
//! Handles packfile parsing, object conversion, and hash mapping.

use std::sync::Arc;

use aspen_core::KeyValueStore;
use aspen_blob::BlobStore;
use crate::identity::RepoId;
use crate::refs::RefStore;

use super::constants::MAX_IMPORT_BATCH_SIZE;
use super::converter::GitObjectConverter;
use super::error::{BridgeError, BridgeResult};
use super::mapping::{GitObjectType, HashMappingStore};
use super::sha1::Sha1Hash;
use super::topological::{
    extract_commit_dependencies, extract_tag_dependencies, extract_tree_dependencies,
    ObjectCollector, PendingObject,
};

/// Result of an import operation.
#[derive(Debug)]
pub struct ImportResult {
    /// Number of objects imported.
    pub objects_imported: usize,
    /// Number of objects skipped (already had mappings).
    pub objects_skipped: usize,
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
    ) -> Self {
        let converter = GitObjectConverter::new(Arc::clone(&mapping), secret_key);
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
    pub async fn import_object(
        &self,
        repo_id: &RepoId,
        git_bytes: &[u8],
    ) -> BridgeResult<blake3::Hash> {
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
            .map_err(|e| BridgeError::BlobStorage {
                message: e.to_string(),
            })?;

        // Store the hash mapping
        self.mapping.store(repo_id, blake3, sha1, obj_type).await?;

        Ok(blake3)
    }

    /// Import multiple objects in topological order.
    ///
    /// Objects are automatically sorted so dependencies are processed first.
    /// Returns import statistics.
    pub async fn import_objects(
        &self,
        repo_id: &RepoId,
        objects: Vec<(Sha1Hash, GitObjectType, Vec<u8>)>,
    ) -> BridgeResult<ImportResult> {
        if objects.len() > MAX_IMPORT_BATCH_SIZE {
            return Err(BridgeError::ImportBatchExceeded {
                count: objects.len(),
                max: MAX_IMPORT_BATCH_SIZE,
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

        // Sort topologically
        let order = collector.finish()?;

        // Import in order
        let mut imported = 0;
        for obj in order.objects {
            self.import_object(repo_id, &obj.data).await?;
            imported += 1;
        }

        Ok(ImportResult {
            objects_imported: imported,
            objects_skipped: order.skipped.len(),
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
        let mut result = self.import_objects(repo_id, objects).await?;

        // Get the BLAKE3 hash for the commit
        let (commit_blake3, _) = self
            .mapping
            .get_blake3(repo_id, &commit_sha1)
            .await?
            .ok_or_else(|| BridgeError::MappingNotFound {
                hash: commit_sha1.to_hex(),
            })?;

        // Update the ref
        self.refs
            .set(repo_id, ref_name, commit_blake3)
            .await
            .map_err(|e| BridgeError::KvStorage {
                message: e.to_string(),
            })?;

        result
            .refs_updated
            .push((ref_name.to_string(), commit_blake3));

        Ok(result)
    }

    /// Check if a SHA-1 hash already has a mapping.
    pub async fn has_object(&self, repo_id: &RepoId, sha1: &Sha1Hash) -> BridgeResult<bool> {
        self.mapping.has_sha1(repo_id, sha1).await
    }

    /// Get the BLAKE3 hash for a SHA-1 hash.
    pub async fn get_blake3(
        &self,
        repo_id: &RepoId,
        sha1: &Sha1Hash,
    ) -> BridgeResult<Option<blake3::Hash>> {
        Ok(self
            .mapping
            .get_blake3(repo_id, sha1)
            .await?
            .map(|(h, _)| h))
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
                .ok_or_else(|| BridgeError::MappingNotFound {
                    hash: sha1.to_hex(),
                })?;
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
    pub async fn update_ref(
        &self,
        repo_id: &RepoId,
        ref_name: &str,
        sha1: Sha1Hash,
    ) -> BridgeResult<blake3::Hash> {
        // Get the BLAKE3 hash for the SHA-1
        let (blake3, _) = self
            .mapping
            .get_blake3(repo_id, &sha1)
            .await?
            .ok_or_else(|| BridgeError::MappingNotFound {
                hash: sha1.to_hex(),
            })?;

        // Update the ref
        self.refs
            .set(repo_id, ref_name, blake3)
            .await
            .map_err(|e| BridgeError::KvStorage {
                message: e.to_string(),
            })?;

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

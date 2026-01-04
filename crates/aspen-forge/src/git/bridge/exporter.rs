//! Git exporter service.
//!
//! Exports objects from Aspen Forge to standard Git format.
//! Handles DAG traversal, object conversion, and packfile generation.

use std::collections::HashSet;
use std::collections::VecDeque;
use std::sync::Arc;

use aspen_blob::BlobStore;
use aspen_core::KeyValueStore;

use super::constants::MAX_DAG_TRAVERSAL_DEPTH;
use super::constants::MAX_PUSH_OBJECTS;
use super::converter::GitObjectConverter;
use super::error::BridgeError;
use super::error::BridgeResult;
use super::mapping::GitObjectType;
use super::mapping::HashMappingStore;
use super::sha1::Sha1Hash;
use crate::git::object::GitObject;
use crate::identity::RepoId;
use crate::refs::RefStore;
use crate::types::SignedObject;

/// A git object ready for export.
#[derive(Debug)]
pub struct ExportedObject {
    /// SHA-1 hash of the git object.
    pub sha1: Sha1Hash,
    /// Object type.
    pub object_type: GitObjectType,
    /// Git object content (without header).
    pub content: Vec<u8>,
}

impl ExportedObject {
    /// Get the full git object bytes (with header).
    pub fn to_git_bytes(&self) -> Vec<u8> {
        let type_str = self.object_type.as_str();
        let header = format!("{} {}\0", type_str, self.content.len());
        let mut bytes = Vec::with_capacity(header.len() + self.content.len());
        bytes.extend_from_slice(header.as_bytes());
        bytes.extend_from_slice(&self.content);
        bytes
    }
}

/// Result of an export operation.
#[derive(Debug)]
pub struct ExportResult {
    /// Objects exported (in dependency order).
    pub objects: Vec<ExportedObject>,
    /// Number of objects that were already known to remote.
    pub objects_skipped: usize,
    /// Refs being pushed.
    pub refs: Vec<(String, Sha1Hash)>,
}

/// Service for exporting Forge objects to Git format.
pub struct GitExporter<K: KeyValueStore + ?Sized, B> {
    /// Hash mapping store.
    mapping: Arc<HashMappingStore<K>>,
    /// Object converter.
    converter: GitObjectConverter<K>,
    /// Blob store for reading objects.
    blobs: Arc<B>,
    /// Ref store for reading refs.
    refs: Arc<RefStore<K>>,
}

impl<K: KeyValueStore + ?Sized, B: BlobStore> GitExporter<K, B> {
    /// Create a new git exporter.
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

    /// Export a single object to git format.
    ///
    /// Returns the git content and SHA-1 hash.
    /// Dependencies must already have hash mappings.
    pub async fn export_object(&self, repo_id: &RepoId, blake3: blake3::Hash) -> BridgeResult<ExportedObject> {
        // Fetch the object from blob store
        let iroh_hash = iroh_blobs::Hash::from_bytes(*blake3.as_bytes());
        let bytes = self
            .blobs
            .get_bytes(&iroh_hash)
            .await
            .map_err(|e| BridgeError::BlobStorage { message: e.to_string() })?
            .ok_or_else(|| BridgeError::ObjectNotFound {
                hash: hex::encode(blake3.as_bytes()),
            })?;

        // Deserialize
        let signed: SignedObject<GitObject> = SignedObject::from_bytes(&bytes)?;

        // Determine object type
        let object_type = match &signed.payload {
            GitObject::Blob(_) => GitObjectType::Blob,
            GitObject::Tree(_) => GitObjectType::Tree,
            GitObject::Commit(_) => GitObjectType::Commit,
            GitObject::Tag(_) => GitObjectType::Tag,
        };

        // Convert to git format
        let (content, sha1) = self.converter.export_object(repo_id, &signed.payload).await?;

        // Store the mapping if not already present
        if !self.mapping.has_blake3(repo_id, &blake3).await? {
            self.mapping.store(repo_id, blake3, sha1, object_type).await?;
        }

        Ok(ExportedObject {
            sha1,
            object_type,
            content,
        })
    }

    /// Export all objects reachable from a commit.
    ///
    /// Walks the DAG and exports objects in dependency order.
    /// Optionally skip objects that are already known to the remote.
    pub async fn export_commit_dag(
        &self,
        repo_id: &RepoId,
        commit_blake3: blake3::Hash,
        known_to_remote: &HashSet<Sha1Hash>,
    ) -> BridgeResult<ExportResult> {
        // Phase 1: Walk the DAG and collect all objects (without exporting)
        let mut to_export: Vec<(blake3::Hash, SignedObject<GitObject>)> = Vec::new();
        let mut visited: HashSet<blake3::Hash> = HashSet::new();
        let mut queue: VecDeque<blake3::Hash> = VecDeque::new();
        let mut skipped = 0;
        let mut depth = 0;

        queue.push_back(commit_blake3);

        while let Some(blake3) = queue.pop_front() {
            if visited.contains(&blake3) {
                continue;
            }

            if to_export.len() >= MAX_PUSH_OBJECTS {
                return Err(BridgeError::PushTooLarge {
                    count: to_export.len(),
                    max: MAX_PUSH_OBJECTS,
                });
            }

            depth += 1;
            if depth > MAX_DAG_TRAVERSAL_DEPTH {
                return Err(BridgeError::DepthExceeded {
                    depth,
                    max: MAX_DAG_TRAVERSAL_DEPTH,
                });
            }

            visited.insert(blake3);

            // Check if remote already has this object
            if let Some((sha1, _)) = self.mapping.get_sha1(repo_id, &blake3).await? {
                if known_to_remote.contains(&sha1) {
                    skipped += 1;
                    continue;
                }
            }

            // Fetch the object (but don't export yet)
            let iroh_hash = iroh_blobs::Hash::from_bytes(*blake3.as_bytes());
            let bytes = self
                .blobs
                .get_bytes(&iroh_hash)
                .await
                .map_err(|e| BridgeError::BlobStorage { message: e.to_string() })?
                .ok_or_else(|| BridgeError::ObjectNotFound {
                    hash: hex::encode(blake3.as_bytes()),
                })?;

            let signed: SignedObject<GitObject> = SignedObject::from_bytes(&bytes)?;

            // Queue dependencies for processing
            match &signed.payload {
                GitObject::Commit(commit) => {
                    queue.push_back(commit.tree());
                    for parent in commit.parents() {
                        queue.push_back(parent);
                    }
                }
                GitObject::Tree(tree) => {
                    for entry in &tree.entries {
                        queue.push_back(entry.hash());
                    }
                }
                GitObject::Tag(tag) => {
                    queue.push_back(tag.target());
                }
                GitObject::Blob(_) => {}
            }

            to_export.push((blake3, signed));
        }

        // Phase 2: Reverse to get dependency order (blobs first, then trees, then commits)
        to_export.reverse();

        // Phase 3: Export each object in dependency order
        let mut objects = Vec::with_capacity(to_export.len());
        for (blake3, _signed) in to_export {
            let exported = self.export_object(repo_id, blake3).await?;
            objects.push(exported);
        }

        Ok(ExportResult {
            objects,
            objects_skipped: skipped,
            refs: Vec::new(),
        })
    }

    /// Export a ref and all reachable objects.
    pub async fn export_ref(
        &self,
        repo_id: &RepoId,
        ref_name: &str,
        known_to_remote: &HashSet<Sha1Hash>,
    ) -> BridgeResult<ExportResult> {
        // Get the ref value
        let blake3 = self
            .refs
            .get(repo_id, ref_name)
            .await
            .map_err(|e| BridgeError::KvStorage { message: e.to_string() })?
            .ok_or_else(|| BridgeError::RefNotFound {
                ref_name: ref_name.to_string(),
            })?;

        // Export the DAG
        let mut result = self.export_commit_dag(repo_id, blake3, known_to_remote).await?;

        // Get SHA-1 for the ref
        let (sha1, _) = self.mapping.get_sha1(repo_id, &blake3).await?.ok_or_else(|| BridgeError::MappingNotFound {
            hash: hex::encode(blake3.as_bytes()),
        })?;

        result.refs.push((ref_name.to_string(), sha1));

        Ok(result)
    }

    /// List refs with their SHA-1 hashes.
    ///
    /// For the git remote helper's "list" command.
    /// Exports the entire commit DAG on-demand to generate SHA-1 mappings if needed.
    pub async fn list_refs(&self, repo_id: &RepoId) -> BridgeResult<Vec<(String, Option<Sha1Hash>)>> {
        use std::collections::HashSet;

        let forge_refs =
            self.refs.list(repo_id).await.map_err(|e| BridgeError::KvStorage { message: e.to_string() })?;

        let mut result = Vec::with_capacity(forge_refs.len());

        for (name, blake3) in forge_refs {
            // Try to get existing mapping first
            let sha1 = match self.mapping.get_sha1(repo_id, &blake3).await? {
                Some((h, _)) => Some(h),
                None => {
                    // No mapping exists - export the entire commit DAG to create mappings
                    // This walks the DAG in dependency order (blobs -> trees -> commits)
                    match self.export_commit_dag(repo_id, blake3, &HashSet::new()).await {
                        Ok(export_result) => {
                            // Find the SHA-1 for the commit we just exported
                            // After export_commit_dag, the commit is last (dependencies reversed)
                            export_result
                                .objects
                                .iter()
                                .rev()
                                .find(|obj| matches!(obj.object_type, GitObjectType::Commit))
                                .map(|obj| obj.sha1)
                        }
                        Err(e) => {
                            // Log but don't fail - ref will be listed without SHA-1
                            tracing::warn!(
                                ref_name = %name,
                                blake3 = %hex::encode(blake3.as_bytes()),
                                error = %e,
                                "failed to export DAG for ref"
                            );
                            None
                        }
                    }
                }
            };
            result.push((name, sha1));
        }

        Ok(result)
    }

    /// Get the SHA-1 hash for a BLAKE3 hash.
    pub async fn get_sha1(&self, repo_id: &RepoId, blake3: &blake3::Hash) -> BridgeResult<Option<Sha1Hash>> {
        Ok(self.mapping.get_sha1(repo_id, blake3).await?.map(|(h, _)| h))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exported_object_to_git_bytes() {
        let obj = ExportedObject {
            sha1: Sha1Hash::from_bytes([0xab; 20]),
            object_type: GitObjectType::Blob,
            content: b"hello\n".to_vec(),
        };

        let bytes = obj.to_git_bytes();
        assert!(bytes.starts_with(b"blob 6\0"));
        assert!(bytes.ends_with(b"hello\n"));
    }
}

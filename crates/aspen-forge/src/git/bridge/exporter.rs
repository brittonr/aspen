//! Git exporter service.
//!
//! Exports objects from Aspen Forge to standard Git format.
//! Handles DAG traversal, object conversion, and packfile generation.

use std::collections::HashSet;
use std::collections::VecDeque;
use std::sync::Arc;

use aspen_blob::prelude::*;
use aspen_core::KeyValueStore;
use aspen_core::hlc::HLC;

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
    pub objects_skipped: u32,
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

    /// Read serialized SignedObject bytes for a BLAKE3 hash.
    ///
    /// Tries KV first (`forge:obj:{repo}:{b3}`) for reliable reads,
    /// falls back to iroh-blobs `get_bytes` (may fail with bao encode
    /// error on some filesystems).
    async fn read_object_bytes(&self, repo_id: &RepoId, blake3: blake3::Hash) -> BridgeResult<bytes::Bytes> {
        // Try KV first (reliable — stored in redb via Raft)
        let obj_key =
            format!("{}{}:{}", super::constants::KV_PREFIX_OBJ, repo_id.to_hex(), hex::encode(blake3.as_bytes()));
        let obj_key_for_log = obj_key.clone();
        let read_req = aspen_core::ReadRequest::new(obj_key.clone());
        match self.mapping.kv().read(read_req).await {
            Ok(result) => {
                if let Some(kv) = result.kv {
                    // Check for chunked storage manifest
                    if let Some(rest) = kv.value.strip_prefix("chunks:") {
                        if let Ok(num_chunks) = rest.parse::<usize>() {
                            match self.read_chunked_object(&obj_key, num_chunks).await {
                                Ok(decoded) => {
                                    tracing::debug!(
                                        blake3 = %hex::encode(blake3.as_bytes()),
                                        size = decoded.len(),
                                        chunks = num_chunks,
                                        "read chunked object bytes from KV"
                                    );
                                    return Ok(bytes::Bytes::from(decoded));
                                }
                                Err(e) => {
                                    tracing::warn!(
                                        blake3 = %hex::encode(blake3.as_bytes()),
                                        error = %e,
                                        "failed to read chunked object, falling back to iroh-blobs"
                                    );
                                }
                            }
                        }
                    } else {
                        use base64::Engine;
                        if let Ok(decoded) = base64::engine::general_purpose::STANDARD.decode(&kv.value) {
                            tracing::debug!(
                                blake3 = %hex::encode(blake3.as_bytes()),
                                size = decoded.len(),
                                "read object bytes from KV"
                            );
                            return Ok(bytes::Bytes::from(decoded));
                        }
                        tracing::warn!(blake3 = %hex::encode(blake3.as_bytes()), "KV object bytes base64 decode failed");
                    }
                } else {
                    tracing::debug!(blake3 = %hex::encode(blake3.as_bytes()), key = %obj_key_for_log, "KV object key not found, falling back to iroh-blobs");
                }
            }
            Err(e) => {
                tracing::debug!(blake3 = %hex::encode(blake3.as_bytes()), error = %e, "KV read failed, falling back to iroh-blobs");
            }
        }

        // Fall back to iroh-blobs (may fail with bao encode error)
        let iroh_hash = iroh_blobs::Hash::from_bytes(*blake3.as_bytes());
        match self.blobs.get_bytes(&iroh_hash).await {
            Ok(Some(bytes)) => Ok(bytes),
            Ok(None) => {
                // Wait for replication
                let available = self
                    .blobs
                    .wait_available(&iroh_hash, aspen_blob::BLOB_READ_WAIT_TIMEOUT)
                    .await
                    .map_err(|e| BridgeError::BlobStorage { message: e.to_string() })?;
                if !available {
                    return Err(BridgeError::ObjectNotFound {
                        hash: hex::encode(blake3.as_bytes()),
                    });
                }
                self.blobs
                    .get_bytes(&iroh_hash)
                    .await
                    .map_err(|e| BridgeError::BlobStorage { message: e.to_string() })?
                    .ok_or_else(|| BridgeError::ObjectNotFound {
                        hash: hex::encode(blake3.as_bytes()),
                    })
            }
            Err(e) => {
                tracing::warn!(
                    blake3 = %hex::encode(blake3.as_bytes()),
                    error = %e,
                    "iroh-blobs get_bytes failed and KV had no object bytes"
                );
                Err(BridgeError::BlobStorage {
                    message: format!("object {} not in KV, iroh-blobs failed: {}", hex::encode(blake3.as_bytes()), e),
                })
            }
        }
    }

    /// Read a chunked object from multiple KV entries.
    ///
    /// Large objects (> OBJ_CHUNK_THRESHOLD) are stored as:
    ///   main key → "chunks:N"
    ///   main key + ":c:0" → base64 chunk 0
    ///   main key + ":c:1" → base64 chunk 1
    ///   ...
    async fn read_chunked_object(&self, obj_key: &str, num_chunks: usize) -> Result<Vec<u8>, String> {
        use base64::Engine;

        if num_chunks == 0 || num_chunks > super::constants::OBJ_MAX_CHUNKS {
            return Err(format!("invalid chunk count: {num_chunks}"));
        }

        let mut combined_b64 = String::new();
        for i in 0..num_chunks {
            let chunk_key = format!("{obj_key}:c:{i}");
            let read_req = aspen_core::ReadRequest::new(chunk_key.clone());
            match self.mapping.kv().read(read_req).await {
                Ok(result) => {
                    if let Some(kv) = result.kv {
                        combined_b64.push_str(&kv.value);
                    } else {
                        return Err(format!("chunk {i}/{num_chunks} not found: {chunk_key}"));
                    }
                }
                Err(e) => {
                    return Err(format!("chunk {i}/{num_chunks} read error: {e}"));
                }
            }
        }

        base64::engine::general_purpose::STANDARD
            .decode(&combined_b64)
            .map_err(|e| format!("chunked base64 decode failed: {e}"))
    }

    /// Export a single object to git format.
    ///
    /// Returns the git content and SHA-1 hash.
    /// Dependencies must already have hash mappings.
    ///
    /// Tiger Style: Wait for blob availability before reading to handle
    /// eventual consistency in multi-node clusters.
    /// Optimization: Try reading locally first before waiting for distributed availability.
    pub async fn export_object(&self, repo_id: &RepoId, blake3: blake3::Hash) -> BridgeResult<ExportedObject> {
        let bytes = self.read_object_bytes(repo_id, blake3).await?;

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
            self.mapping.store(repo_id, blake3, sha1, object_type).await.map_err(|e| BridgeError::KvStorage {
                message: format!("failed to store hash mapping during export: {}", e),
            })?;
        }

        Ok(ExportedObject {
            sha1,
            object_type,
            content,
        })
    }

    /// Queue dependencies of a git object for DAG traversal.
    fn export_commit_dag_queue_deps(queue: &mut VecDeque<blake3::Hash>, object: &GitObject) {
        match object {
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
    }

    /// Walk the DAG and collect objects to export (Phase 1).
    async fn export_commit_dag_collect(
        &self,
        repo_id: &RepoId,
        commit_blake3: blake3::Hash,
        known_to_remote: &HashSet<Sha1Hash>,
    ) -> BridgeResult<(Vec<(blake3::Hash, SignedObject<GitObject>)>, usize)> {
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
                    count: to_export.len() as u32,
                    max: MAX_PUSH_OBJECTS as u32,
                });
            }

            depth += 1;
            if depth > MAX_DAG_TRAVERSAL_DEPTH {
                return Err(BridgeError::DepthExceeded {
                    depth: depth as u32,
                    max: MAX_DAG_TRAVERSAL_DEPTH as u32,
                });
            }

            visited.insert(blake3);

            // Check if remote already has this object
            if let Some((sha1, _)) =
                self.mapping.get_sha1(repo_id, &blake3).await.map_err(|e| BridgeError::KvStorage {
                    message: format!("failed to lookup SHA1 mapping for {}: {}", hex::encode(blake3.as_bytes()), e),
                })?
                && known_to_remote.contains(&sha1)
            {
                skipped += 1;
                continue;
            }

            let bytes = self.read_object_bytes(repo_id, blake3).await?;
            let signed: SignedObject<GitObject> = SignedObject::from_bytes(&bytes)?;
            Self::export_commit_dag_queue_deps(&mut queue, &signed.payload);
            to_export.push((blake3, signed));
        }

        Ok((to_export, skipped))
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
        let (mut to_export, skipped) = self.export_commit_dag_collect(repo_id, commit_blake3, known_to_remote).await?;

        // Phase 2: Reverse to get dependency order (blobs first, then trees, then commits)
        to_export.reverse();

        // Phase 3: Export each object in dependency order
        let mut objects = Vec::with_capacity(to_export.len());
        for (blake3, _signed) in to_export {
            let exported = self.export_object(repo_id, blake3).await.map_err(|e| BridgeError::ConversionFailed {
                message: format!("failed to export object {}: {}", hex::encode(blake3.as_bytes()), e),
            })?;
            objects.push(exported);
        }

        Ok(ExportResult {
            objects,
            objects_skipped: skipped as u32,
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
        let mut result = self.export_commit_dag(repo_id, blake3, known_to_remote).await.map_err(|e| {
            BridgeError::ConversionFailed {
                message: format!("failed to export commit DAG for ref {}: {}", ref_name, e),
            }
        })?;

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

    /// Walk the DAG and collect objects, using BLAKE3 hashes for dedup.
    ///
    /// Like `export_commit_dag_collect` but accepts BLAKE3 hashes in
    /// `known_blake3` instead of SHA1, skipping the SHA1 mapping lookup.
    /// Used by federation sync where the remote sends BLAKE3 have_hashes.
    ///
    /// Returns `(objects_to_export, skipped_count)`. When `limit` is
    /// reached, stops collecting and returns what it has — caller checks
    /// whether the full DAG was traversed.
    async fn collect_dag_blake3(
        &self,
        repo_id: &RepoId,
        commit_blake3: blake3::Hash,
        known_blake3: &HashSet<[u8; 32]>,
        limit: usize,
    ) -> BridgeResult<(Vec<(blake3::Hash, SignedObject<GitObject>)>, usize, bool)> {
        let mut to_export: Vec<(blake3::Hash, SignedObject<GitObject>)> = Vec::new();
        let mut visited: HashSet<blake3::Hash> = HashSet::new();
        let mut queue: VecDeque<blake3::Hash> = VecDeque::new();
        let mut skipped = 0usize;
        let mut depth = 0usize;
        let mut hit_limit = false;

        queue.push_back(commit_blake3);

        while let Some(blake3) = queue.pop_front() {
            if visited.contains(&blake3) {
                continue;
            }

            depth += 1;
            if depth > MAX_DAG_TRAVERSAL_DEPTH {
                hit_limit = true;
                break;
            }

            visited.insert(blake3);

            // Check if remote already has this object (by BLAKE3 directly)
            if known_blake3.contains(blake3.as_bytes()) {
                skipped += 1;
                continue;
            }

            let bytes = self.read_object_bytes(repo_id, blake3).await?;
            let signed: SignedObject<GitObject> = SignedObject::from_bytes(&bytes)?;
            Self::export_commit_dag_queue_deps(&mut queue, &signed.payload);
            to_export.push((blake3, signed));

            if to_export.len() >= limit {
                hit_limit = true;
                break;
            }
        }

        Ok((to_export, skipped, hit_limit))
    }

    /// Export reachable objects from a commit using BLAKE3-based dedup.
    ///
    /// For federation sync: the remote peer sends BLAKE3 hashes of objects
    /// it already has. Returns raw git object bytes (with header) keyed by
    /// BLAKE3 hash and object type. Stops at `limit` objects and reports
    /// `has_more` when the DAG was not fully traversed.
    pub async fn export_commit_dag_blake3(
        &self,
        repo_id: &RepoId,
        commit_blake3: blake3::Hash,
        known_blake3: &HashSet<[u8; 32]>,
        limit: usize,
    ) -> BridgeResult<FederationExportResult> {
        let (mut to_export, skipped, has_more) =
            self.collect_dag_blake3(repo_id, commit_blake3, known_blake3, limit).await?;

        // Reverse to get dependency order (blobs first, then trees, then commits)
        to_export.reverse();

        let mut objects = Vec::with_capacity(to_export.len());
        for (b3, signed) in &to_export {
            let object_type = match &signed.payload {
                GitObject::Blob(_) => GitObjectType::Blob,
                GitObject::Tree(_) => GitObjectType::Tree,
                GitObject::Commit(_) => GitObjectType::Commit,
                GitObject::Tag(_) => GitObjectType::Tag,
            };

            // Convert to git bytes (without header) for the receiver to import
            let (content, sha1) = self.converter.export_object(repo_id, &signed.payload).await?;

            objects.push(FederationExportedObject {
                blake3: *b3,
                object_type,
                content,
                sha1,
            });
        }

        Ok(FederationExportResult {
            objects,
            objects_skipped: skipped as u32,
            has_more,
        })
    }
}

/// A git object exported for federation.
#[derive(Debug)]
pub struct FederationExportedObject {
    /// BLAKE3 hash of the stored blob (envelope hash).
    pub blake3: blake3::Hash,
    /// Object type (commit, tree, blob, tag).
    pub object_type: GitObjectType,
    /// Git SHA-1 hash, computed from the exported content.
    pub sha1: super::sha1::Sha1Hash,
    /// Git object content (without header).
    pub content: Vec<u8>,
}

/// Result of a federation DAG export.
#[derive(Debug)]
pub struct FederationExportResult {
    /// Objects exported in dependency order.
    pub objects: Vec<FederationExportedObject>,
    /// Number of objects skipped (already known to remote).
    pub objects_skipped: u32,
    /// Whether there are more objects to fetch (DAG not fully traversed).
    pub has_more: bool,
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

    /// Import a blob via GitImporter, then immediately export it via GitExporter.
    /// Uses InMemoryBlobStore — if this fails, the issue is in the hash mapping
    /// or SignedObject serialization, not iroh-blobs.
    #[tokio::test]
    async fn test_import_then_export_roundtrip_memory() {
        use aspen_blob::InMemoryBlobStore;
        use aspen_core::hlc::create_hlc;

        use crate::git::bridge::HashMappingStore;
        use crate::identity::RepoId;
        use crate::refs::RefStore;

        let kv: Arc<dyn aspen_core::KeyValueStore> = Arc::new(aspen_testing::DeterministicKeyValueStore::new());
        let blobs = Arc::new(InMemoryBlobStore::new());
        let mapping = Arc::new(HashMappingStore::new(Arc::clone(&kv)));
        let refs = Arc::new(RefStore::new(Arc::clone(&kv)));
        let secret_key = iroh::SecretKey::generate(&mut rand::rng());

        let importer = crate::git::bridge::GitImporter::new(
            Arc::clone(&mapping),
            Arc::clone(&blobs),
            Arc::clone(&refs),
            secret_key.clone(),
            create_hlc("test-import"),
        );

        let exporter = GitExporter::new(
            Arc::clone(&mapping),
            Arc::clone(&blobs),
            Arc::clone(&refs),
            secret_key.clone(),
            create_hlc("test-export"),
        );

        let repo_id = RepoId::from_hash(blake3::hash(b"roundtrip-test"));

        // Import a blob
        let content = b"hello from roundtrip test\n";
        let git_bytes = {
            let header = format!("blob {}\0", content.len());
            let mut bytes = Vec::with_capacity(header.len() + content.len());
            bytes.extend_from_slice(header.as_bytes());
            bytes.extend_from_slice(content);
            bytes
        };

        let blake3_hash = importer.import_object(&repo_id, &git_bytes).await.expect("import should succeed");

        // Export it back
        let exported = exporter.export_object(&repo_id, blake3_hash).await.expect("export should succeed");

        assert_eq!(exported.object_type, GitObjectType::Blob);
        assert_eq!(exported.content, content);
    }
}

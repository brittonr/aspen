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
use n0_future::BufferedStreamExt;
use n0_future::StreamExt;

use super::constants::MAX_HASH_MAPPING_BATCH_SIZE;
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
    /// Per-object hash mappings: (SHA-1, BLAKE3) for every object processed
    /// (both newly imported and already-present). Callers that need to correlate
    /// git SHA-1 hashes with Forge BLAKE3 hashes (e.g., federation ref translation)
    /// can use this without extra KV lookups.
    pub mappings: Vec<(Sha1Hash, blake3::Hash)>,
    /// Objects that failed to import. Each entry is (SHA-1, error description).
    /// Non-empty when partial-success mode is used (e.g., federation convergent
    /// retry). The caller can retry these objects in a subsequent pass.
    pub failures: Vec<(Sha1Hash, String)>,
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
        let (blake3, sha1, obj_type) = self.import_object_store_blob(repo_id, git_bytes).await?;

        // Store the hash mapping individually (for single-object imports).
        // Bulk imports use import_objects() which batches mappings per wave.
        self.mapping.store(repo_id, blake3, sha1, obj_type).await.map_err(|e| BridgeError::KvStorage {
            message: format!("failed to store hash mapping for {}: {}", sha1.to_hex(), e),
        })?;

        Ok(blake3)
    }

    /// Convert and store the blob for a single git object, returning
    /// mapping info without writing it to KV.
    ///
    /// Used by `import_objects()` to separate blob storage (concurrent)
    /// from mapping storage (batched per wave).
    async fn import_object_store_blob(
        &self,
        repo_id: &RepoId,
        git_bytes: &[u8],
    ) -> BridgeResult<(blake3::Hash, Sha1Hash, GitObjectType)> {
        let (obj_type, content) = GitObjectConverter::<K>::split_git_object(git_bytes)?;

        let (signed, sha1, blake3) = match obj_type {
            GitObjectType::Blob => self.converter.import_blob(content)?,
            GitObjectType::Tree => self.converter.import_tree(repo_id, content).await?,
            GitObjectType::Commit => self.converter.import_commit(repo_id, content).await?,
            GitObjectType::Tag => self.converter.import_tag(repo_id, content).await?,
        };

        // Store the object blob only — mapping is deferred.
        let bytes = signed.to_bytes();
        self.blobs
            .add_bytes(&bytes)
            .await
            .map_err(|e| BridgeError::BlobStorage { message: e.to_string() })?;

        // Also store in KV for reliable reads (iroh-blobs FsStore bao can be unreliable).
        // Key: forge:obj:{repo_id}:{blake3_hex}, Value: base64-encoded serialized bytes.
        // Large objects (> CHUNK_THRESHOLD base64) are split across multiple KV entries.
        let obj_key =
            format!("{}{}:{}", super::constants::KV_PREFIX_OBJ, repo_id.to_hex(), hex::encode(blake3.as_bytes()));
        use base64::Engine;
        let obj_value = base64::engine::general_purpose::STANDARD.encode(&bytes);
        let obj_key_debug = obj_key.clone();

        if obj_value.len() <= super::constants::OBJ_CHUNK_THRESHOLD {
            // Small object: single KV entry
            let write_req = aspen_core::WriteRequest {
                command: aspen_core::WriteCommand::Set {
                    key: obj_key,
                    value: obj_value,
                },
            };
            // Best-effort: log but don't fail import if KV write fails
            match self.mapping.kv().write(write_req).await {
                Ok(_) => {
                    tracing::debug!(
                        blake3 = %hex::encode(blake3.as_bytes()),
                        key = %obj_key_debug,
                        size = bytes.len(),
                        "stored object bytes in KV"
                    );
                }
                Err(e) => {
                    tracing::warn!(
                        blake3 = %hex::encode(blake3.as_bytes()),
                        key = %obj_key_debug,
                        error = %e,
                        "failed to store object bytes in KV (non-fatal)"
                    );
                }
            }
        } else {
            // Large object: split across multiple KV entries
            let chunk_size = super::constants::OBJ_CHUNK_SIZE;
            let chunks: Vec<&str> = obj_value
                .as_bytes()
                .chunks(chunk_size)
                .map(|c| {
                    // SAFETY: base64 output is always valid UTF-8
                    std::str::from_utf8(c).unwrap_or("")
                })
                .collect();
            let num_chunks = chunks.len().min(super::constants::OBJ_MAX_CHUNKS);

            // Write chunk data first, then the manifest
            let mut chunk_ok = true;
            for (i, chunk_data) in chunks.iter().take(num_chunks).enumerate() {
                let chunk_key = format!("{}:c:{}", obj_key, i);
                let write_req = aspen_core::WriteRequest {
                    command: aspen_core::WriteCommand::Set {
                        key: chunk_key,
                        value: (*chunk_data).to_string(),
                    },
                };
                if let Err(e) = self.mapping.kv().write(write_req).await {
                    tracing::warn!(
                        blake3 = %hex::encode(blake3.as_bytes()),
                        chunk = i,
                        error = %e,
                        "failed to store object chunk in KV (non-fatal)"
                    );
                    chunk_ok = false;
                    break;
                }
            }

            if chunk_ok {
                // Write manifest: main key stores "chunks:N"
                let manifest = format!("chunks:{}", num_chunks);
                let write_req = aspen_core::WriteRequest {
                    command: aspen_core::WriteCommand::Set {
                        key: obj_key,
                        value: manifest,
                    },
                };
                match self.mapping.kv().write(write_req).await {
                    Ok(_) => {
                        tracing::debug!(
                            blake3 = %hex::encode(blake3.as_bytes()),
                            key = %obj_key_debug,
                            size = bytes.len(),
                            chunks = num_chunks,
                            "stored large object bytes in KV (chunked)"
                        );
                    }
                    Err(e) => {
                        tracing::warn!(
                            blake3 = %hex::encode(blake3.as_bytes()),
                            key = %obj_key_debug,
                            error = %e,
                            "failed to store object manifest in KV (non-fatal)"
                        );
                    }
                }
            }
        }

        Ok((blake3, sha1, obj_type))
    }

    /// Import multiple objects using wave-based parallelism with partial-success semantics.
    ///
    /// Objects are sorted into waves where all objects in a wave can be
    /// processed concurrently (they have no dependencies on each other).
    /// Waves are processed sequentially to ensure dependencies are satisfied.
    ///
    /// Individual object failures within a wave are collected in
    /// `ImportResult::failures` rather than aborting the entire import.
    /// This enables callers (e.g., federation convergent retry) to make
    /// maximum progress per pass and retry only the failed objects.
    ///
    /// Returns import statistics including per-object mappings and failures.
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
        //
        // Blob storage is concurrent per-object within the wave. Hash
        // mappings are collected and written in batched SetMulti calls
        // after each wave, reducing Raft round-trips from 2×N individual
        // writes to ceil(2×N / MAX_SETMULTI_KEYS) batched writes.
        let repo_id = *repo_id;
        let mut imported = 0u32;
        let mut all_mappings: Vec<(Sha1Hash, blake3::Hash)> =
            Vec::with_capacity(total_objects as usize + skipped_count as usize);
        let mut all_failures: Vec<(Sha1Hash, String)> = Vec::new();

        // Include skipped objects (already had mappings) in the result.
        for sha1 in &waves.skipped {
            if let Ok(Some((blake3, _))) = self.mapping.get_blake3(&repo_id, sha1).await {
                all_mappings.push((*sha1, blake3));
            }
        }

        for (wave_idx, wave) in waves.waves.into_iter().enumerate() {
            let wave_size = wave.len();

            // Store blobs concurrently, collect mapping info for c2e.
            // Carry the SHA-1 through so failures can be attributed.
            #[allow(clippy::type_complexity)]
            let results: Vec<(Sha1Hash, BridgeResult<(blake3::Hash, Sha1Hash, GitObjectType)>)> =
                n0_future::stream::iter(wave)
                    .map(|obj| {
                        let sha1 = obj.sha1;
                        async move { (sha1, self.import_object_store_blob(&repo_id, &obj.data).await) }
                    })
                    .buffered_unordered(MAX_IMPORT_CONCURRENCY)
                    .collect()
                    .await;

            // Collect successful mappings; record per-object failures and continue.
            let mut wave_mappings: Vec<(blake3::Hash, Sha1Hash, GitObjectType)> = Vec::with_capacity(wave_size);
            // c2e: (sha1, envelope_blake3) — keyed by SHA-1 for deterministic lookup
            let mut wave_c2e: Vec<(Sha1Hash, [u8; 32])> = Vec::with_capacity(wave_size);
            let mut wave_failures = 0u32;
            for (obj_sha1, result) in results {
                match result {
                    Ok((blake3, sha1, obj_type)) => {
                        all_mappings.push((sha1, blake3));
                        wave_mappings.push((blake3, sha1, obj_type));
                        wave_c2e.push((sha1, *blake3.as_bytes()));
                        imported += 1;
                    }
                    Err(e) => {
                        all_failures.push((obj_sha1, e.to_string()));
                        wave_failures += 1;
                    }
                }
            }

            // Batch-write all hash mappings for this wave.
            // store_batch chunks internally to respect MAX_SETMULTI_KEYS.
            for chunk in wave_mappings.chunks(MAX_HASH_MAPPING_BATCH_SIZE) {
                self.mapping.store_batch(&repo_id, chunk).await?;
            }

            // Batch-write c2e (SHA-1 → envelope BLAKE3) index for federation DAG dedup.
            // Keyed by SHA-1 hex because SHA-1 is the canonical git object identifier,
            // guaranteed identical on import and export paths regardless of internal
            // serialization differences (tree sort order, commit normalization).
            {
                let mut c2e_pairs: Vec<(String, String)> = Vec::with_capacity(wave_c2e.len());
                for (sha1, env_hash) in &wave_c2e {
                    c2e_pairs
                        .push((format!("forge:c2e:{}:{}", repo_id.to_hex(), sha1.to_hex()), hex::encode(env_hash)));
                }
                for chunk in c2e_pairs.chunks(500) {
                    let _ = self
                        .mapping
                        .kv()
                        .write(aspen_core::WriteRequest {
                            command: aspen_core::WriteCommand::SetMulti { pairs: chunk.to_vec() },
                        })
                        .await;
                }
            }

            if wave_failures > 0 {
                tracing::debug!(
                    wave = wave_idx,
                    imported = wave_size as u32 - wave_failures,
                    failed = wave_failures,
                    "wave import partial — some objects failed"
                );
            } else {
                tracing::trace!(wave = wave_idx, objects = wave_size, "wave import complete");
            }
        }

        tracing::info!(
            total = total_objects,
            imported = imported,
            skipped = skipped_count,
            failed = all_failures.len(),
            waves = wave_count,
            "wave-based parallel import complete"
        );

        Ok(ImportResult {
            objects_imported: imported,
            objects_skipped: skipped_count,
            refs_updated: Vec::new(),
            mappings: all_mappings,
            failures: all_failures,
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

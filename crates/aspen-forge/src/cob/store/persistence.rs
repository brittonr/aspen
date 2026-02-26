//! Persistence operations for `CobStore` (store/get changes, manage heads).

use std::collections::HashMap;
use std::collections::HashSet;
use std::collections::VecDeque;

use aspen_blob::prelude::*;
use aspen_core::KeyValueStore;
use aspen_core::ReadConsistency;

use super::super::change::CobChange;
use super::super::change::CobType;
use super::CobStore;
use super::types::CobUpdateEvent;
use crate::constants::KV_PREFIX_COB_HEADS;
use crate::constants::MAX_COB_CHANGE_SIZE_BYTES;
use crate::constants::MAX_COB_CHANGES_TO_RESOLVE;
use crate::constants::MAX_COB_PARENTS;
use crate::error::ForgeError;
use crate::error::ForgeResult;
use crate::identity::RepoId;
use crate::types::SignedObject;

impl<B: BlobStore, K: KeyValueStore + ?Sized> CobStore<B, K> {
    /// Store a change and update heads.
    ///
    /// After successful storage, emits a `CobUpdateEvent` for gossip broadcast.
    pub(super) async fn store_change(&self, repo_id: &RepoId, change: CobChange) -> ForgeResult<blake3::Hash> {
        // Validate parents count
        if change.parents.len() as u32 > MAX_COB_PARENTS {
            return Err(ForgeError::TooManyParents {
                count: change.parents.len() as u32,
                max: MAX_COB_PARENTS,
            });
        }

        // Sign and store with HLC timestamp
        let signed = SignedObject::new(change.clone(), &self.secret_key, &self.hlc)?;
        let hash = signed.hash();
        let bytes = signed.to_bytes();

        // Check size limit
        if bytes.len() as u64 > MAX_COB_CHANGE_SIZE_BYTES {
            return Err(ForgeError::ObjectTooLarge {
                size: bytes.len() as u64,
                max: MAX_COB_CHANGE_SIZE_BYTES,
            });
        }

        // Store in blobs
        self.blobs.add_bytes(&bytes).await.map_err(|e| ForgeError::BlobStorage { message: e.to_string() })?;

        // Update heads: remove parents, add new hash
        self.update_heads(repo_id, &change.cob_type, &change.cob_id(), &change.parents(), hash).await?;

        // Emit event (ignore send errors if no subscribers)
        let _ = self.event_tx.send(CobUpdateEvent {
            repo_id: *repo_id,
            cob_type: change.cob_type,
            cob_id: change.cob_id(),
            change_hash: hash,
            author: signed.author,
            hlc_timestamp: signed.hlc_timestamp.clone(),
        });

        Ok(hash)
    }

    /// Get the current heads for a COB.
    pub(super) async fn get_heads(
        &self,
        repo_id: &RepoId,
        cob_type: CobType,
        cob_id: &blake3::Hash,
    ) -> ForgeResult<Vec<blake3::Hash>> {
        let key = format!(
            "{}{}:{}:{}",
            KV_PREFIX_COB_HEADS,
            repo_id.to_hex(),
            cob_type.as_str(),
            hex::encode(cob_id.as_bytes())
        );

        let result = match self
            .kv
            .read(aspen_core::ReadRequest {
                key: key.clone(),
                consistency: ReadConsistency::Linearizable,
            })
            .await
        {
            Ok(r) => r,
            Err(aspen_core::KeyValueStoreError::NotFound { .. }) => {
                // Key not found means no heads yet
                return Ok(vec![]);
            }
            Err(e) => return Err(ForgeError::from(e)),
        };

        match result.kv.map(|kv| kv.value) {
            Some(value) => {
                // Parse heads from stored format (newline-separated hex hashes)
                let heads: Vec<blake3::Hash> = value
                    .lines()
                    .filter(|s| !s.is_empty())
                    .filter_map(|s| {
                        let bytes = hex::decode(s).ok()?;
                        if bytes.len() == 32 {
                            let mut arr = [0u8; 32];
                            arr.copy_from_slice(&bytes);
                            Some(blake3::Hash::from_bytes(arr))
                        } else {
                            None
                        }
                    })
                    .collect();
                Ok(heads)
            }
            None => Ok(vec![]),
        }
    }

    /// Update heads after storing a change.
    async fn update_heads(
        &self,
        repo_id: &RepoId,
        cob_type: &CobType,
        cob_id: &blake3::Hash,
        parents: &[blake3::Hash],
        new_head: blake3::Hash,
    ) -> ForgeResult<()> {
        let key = format!(
            "{}{}:{}:{}",
            KV_PREFIX_COB_HEADS,
            repo_id.to_hex(),
            cob_type.as_str(),
            hex::encode(cob_id.as_bytes())
        );

        // Get current heads
        let mut heads: HashSet<blake3::Hash> = self.get_heads(repo_id, *cob_type, cob_id).await?.into_iter().collect();

        // Remove parents (they're no longer heads)
        for parent in parents {
            heads.remove(parent);
        }

        // Add new head
        heads.insert(new_head);

        // Serialize heads
        let value = heads.iter().map(|h| hex::encode(h.as_bytes())).collect::<Vec<_>>().join("\n");

        // Store
        self.kv
            .write(aspen_core::WriteRequest {
                command: aspen_core::WriteCommand::Set { key, value },
            })
            .await?;

        Ok(())
    }

    /// Collect all changes reachable from the given heads.
    pub(super) async fn collect_changes(
        &self,
        heads: Vec<blake3::Hash>,
    ) -> ForgeResult<HashMap<blake3::Hash, SignedObject<CobChange>>> {
        let mut changes = HashMap::new();
        let mut visited = HashSet::new();
        let mut queue = VecDeque::from(heads);

        while let Some(hash) = queue.pop_front() {
            if changes.len() as u32 >= MAX_COB_CHANGES_TO_RESOLVE {
                return Err(ForgeError::TooManyChanges {
                    count: changes.len() as u32,
                    max: MAX_COB_CHANGES_TO_RESOLVE,
                });
            }

            if visited.insert(hash) {
                let signed = self.get_change(&hash).await?;

                // Queue parents
                for parent in signed.payload.parents() {
                    if !visited.contains(&parent) {
                        queue.push_back(parent);
                    }
                }

                changes.insert(hash, signed);
            }
        }

        Ok(changes)
    }

    /// Get a change by hash.
    ///
    /// Tiger Style: Wait for blob availability before reading to handle
    /// eventual consistency in multi-node clusters.
    /// Optimization: Try reading locally first before waiting for distributed availability.
    pub(super) async fn get_change(&self, hash: &blake3::Hash) -> ForgeResult<SignedObject<CobChange>> {
        let iroh_hash = iroh_blobs::Hash::from_bytes(*hash.as_bytes());

        // Optimization: Try reading locally first (fast path for locally available blobs)
        // This avoids the 30-second wait when the blob is already present
        if let Some(bytes) = self
            .blobs
            .get_bytes(&iroh_hash)
            .await
            .map_err(|e| ForgeError::BlobStorage { message: e.to_string() })?
        {
            let signed: SignedObject<CobChange> = SignedObject::from_bytes(&bytes)?;
            signed.verify()?;
            return Ok(signed);
        }

        // Blob not available locally — wait briefly for replication.
        // Uses wait_available (single blob) with a shorter read timeout so
        // reads fail fast in degraded clusters.
        let available = self
            .blobs
            .wait_available(&iroh_hash, aspen_blob::BLOB_READ_WAIT_TIMEOUT)
            .await
            .map_err(|e| ForgeError::BlobStorage { message: e.to_string() })?;

        if !available {
            return Err(ForgeError::ObjectNotFound {
                hash: hex::encode(hash.as_bytes()),
            });
        }

        // Now read the blob (should be available after wait)
        let bytes = self
            .blobs
            .get_bytes(&iroh_hash)
            .await
            .map_err(|e| ForgeError::BlobStorage { message: e.to_string() })?
            .ok_or_else(|| ForgeError::ObjectNotFound {
                hash: hex::encode(hash.as_bytes()),
            })?;

        let signed: SignedObject<CobChange> = SignedObject::from_bytes(&bytes)?;
        signed.verify()?;

        Ok(signed)
    }

    /// Topologically sort changes (parents before children).
    pub(super) fn topological_sort(
        &self,
        changes: &HashMap<blake3::Hash, SignedObject<CobChange>>,
    ) -> ForgeResult<Vec<(blake3::Hash, SignedObject<CobChange>)>> {
        let mut result = Vec::with_capacity(changes.len());
        let mut visited = HashSet::new();
        let mut temp_mark = HashSet::new();

        fn visit(
            hash: blake3::Hash,
            changes: &HashMap<blake3::Hash, SignedObject<CobChange>>,
            visited: &mut HashSet<blake3::Hash>,
            temp_mark: &mut HashSet<blake3::Hash>,
            result: &mut Vec<(blake3::Hash, SignedObject<CobChange>)>,
        ) -> ForgeResult<()> {
            if visited.contains(&hash) {
                return Ok(());
            }

            if temp_mark.contains(&hash) {
                return Err(ForgeError::CobResolutionFailed {
                    message: "cycle detected in change graph".to_string(),
                });
            }

            temp_mark.insert(hash);

            if let Some(signed) = changes.get(&hash) {
                for parent in signed.payload.parents() {
                    visit(parent, changes, visited, temp_mark, result)?;
                }

                visited.insert(hash);
                temp_mark.remove(&hash);
                result.push((hash, signed.clone()));
            }

            Ok(())
        }

        for hash in changes.keys() {
            visit(*hash, changes, &mut visited, &mut temp_mark, &mut result)?;
        }

        Ok(result)
    }
}

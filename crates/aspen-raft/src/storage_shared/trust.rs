//! Trust share and digest storage operations.
//!
//! Stores Shamir secret sharing data in dedicated redb tables, isolated
//! from application KV operations.

use std::collections::BTreeMap;

use aspen_raft_types::TrustInitializePayload;
use aspen_trust::shamir::SECRET_SIZE;
use aspen_trust::shamir::Share;
use aspen_trust::shamir::ShareDigest;
use snafu::ResultExt;

use super::AppResponse;
use super::BeginReadSnafu;
use super::BeginWriteSnafu;
use super::CommitSnafu;
use super::GetSnafu;
use super::OpenTableSnafu;
use super::RangeSnafu;
use super::SharedRedbStorage;
use super::SharedStorageError;
use super::TRUST_DIGESTS_TABLE;
use super::TRUST_EXPUNGED_TABLE;
use super::TRUST_SHARES_TABLE;

impl SharedRedbStorage {
    /// Store a share for the given epoch.
    ///
    /// Overwrites any existing share at this epoch.
    pub fn store_share(&self, epoch: u64, share: &Share) -> Result<(), SharedStorageError> {
        let bytes = share.to_bytes();
        let write_txn = self.db.begin_write().context(BeginWriteSnafu)?;
        {
            let mut table = write_txn.open_table(TRUST_SHARES_TABLE).context(OpenTableSnafu)?;
            table.insert(epoch, bytes.as_slice()).context(super::InsertSnafu)?;
        }
        write_txn.commit().context(CommitSnafu)?;
        Ok(())
    }

    /// Load a share for the given epoch.
    ///
    /// Returns `None` if no share exists for this epoch.
    pub fn load_share(&self, epoch: u64) -> Result<Option<Share>, SharedStorageError> {
        let read_txn = self.db.begin_read().context(BeginReadSnafu)?;
        let table = read_txn.open_table(TRUST_SHARES_TABLE).context(OpenTableSnafu)?;
        let entry = table.get(epoch).context(GetSnafu)?;

        match entry {
            Some(value) => {
                let bytes = value.value();
                if bytes.len() != SECRET_SIZE + 1 {
                    return Ok(None); // Corrupted entry
                }
                let mut arr = [0u8; SECRET_SIZE + 1];
                arr.copy_from_slice(bytes);
                match Share::from_bytes(&arr) {
                    Ok(share) => Ok(Some(share)),
                    Err(_) => Ok(None), // Invalid share (e.g., zero x-coordinate)
                }
            }
            None => Ok(None),
        }
    }

    /// Store digests for all nodes at the given epoch.
    ///
    /// Keys are formatted as `"{epoch}:{node_id}"` to enable range queries by epoch.
    pub fn store_digests(&self, epoch: u64, digests: &BTreeMap<u64, ShareDigest>) -> Result<(), SharedStorageError> {
        let write_txn = self.db.begin_write().context(BeginWriteSnafu)?;
        {
            let mut table = write_txn.open_table(TRUST_DIGESTS_TABLE).context(OpenTableSnafu)?;
            for (&node_id, digest) in digests {
                let key = format!("{epoch}:{node_id}");
                table.insert(key.as_str(), digest.as_slice()).context(super::InsertSnafu)?;
            }
        }
        write_txn.commit().context(CommitSnafu)?;
        Ok(())
    }

    /// Check if this node has been expunged.
    pub fn is_expunged(&self) -> Result<bool, SharedStorageError> {
        let read_txn = self.db.begin_read().context(BeginReadSnafu)?;
        let table = read_txn.open_table(TRUST_EXPUNGED_TABLE).context(OpenTableSnafu)?;
        let entry = table.get(0u64).context(GetSnafu)?;
        Ok(entry.is_some())
    }

    /// Load the expungement metadata, if this node has been expunged.
    pub fn load_expunged(&self) -> Result<Option<aspen_cluster_types::ExpungedMetadata>, SharedStorageError> {
        let read_txn = self.db.begin_read().context(BeginReadSnafu)?;
        let table = read_txn.open_table(TRUST_EXPUNGED_TABLE).context(OpenTableSnafu)?;
        let entry = table.get(0u64).context(GetSnafu)?;

        match entry {
            Some(value) => {
                let bytes = value.value();
                let metadata: aspen_cluster_types::ExpungedMetadata =
                    serde_json::from_slice(bytes).map_err(|e| SharedStorageError::Internal {
                        reason: format!("failed to deserialize ExpungedMetadata: {e}"),
                    })?;
                Ok(Some(metadata))
            }
            None => Ok(None),
        }
    }

    /// Mark this node as permanently expunged.
    ///
    /// This also zeroizes all trust shares to prevent secret reconstruction.
    pub fn mark_expunged(&self, metadata: aspen_cluster_types::ExpungedMetadata) -> Result<(), SharedStorageError> {
        let write_txn = self.db.begin_write().context(BeginWriteSnafu)?;
        {
            // Write the expungement marker
            let mut expunged_table = write_txn.open_table(TRUST_EXPUNGED_TABLE).context(OpenTableSnafu)?;
            let bytes = serde_json::to_vec(&metadata).map_err(|e| SharedStorageError::Internal {
                reason: format!("failed to serialize ExpungedMetadata: {e}"),
            })?;
            expunged_table.insert(0u64, bytes.as_slice()).context(super::InsertSnafu)?;

            // Zeroize all shares: overwrite with zeros then remove
            let mut shares_table = write_txn.open_table(TRUST_SHARES_TABLE).context(OpenTableSnafu)?;
            let zero_share = [0u8; SECRET_SIZE + 1];
            let epochs: Vec<u64> = {
                use redb::ReadableTable;
                let iter = shares_table.iter().context(RangeSnafu)?;
                iter.filter_map(|e| {
                    e.ok().map(|(k, _v): (redb::AccessGuard<'_, u64>, redb::AccessGuard<'_, &[u8]>)| k.value())
                })
                .collect()
            };
            for epoch in &epochs {
                // Overwrite with zeros
                shares_table.insert(*epoch, zero_share.as_slice()).context(super::InsertSnafu)?;
            }
            // Then delete
            for epoch in &epochs {
                shares_table.remove(*epoch).context(super::RemoveSnafu)?;
            }
        }
        write_txn.commit().context(CommitSnafu)?;

        tracing::warn!(
            epoch = metadata.epoch,
            removed_by = metadata.removed_by,
            "Node has been permanently expunged from cluster"
        );

        Ok(())
    }

    /// Apply a committed trust initialization request to the local storage tables.
    pub(crate) fn apply_trust_initialize_in_txn(
        &self,
        shares_table: &mut redb::Table<u64, &[u8]>,
        digests_table: &mut redb::Table<&str, &[u8]>,
        payload: &TrustInitializePayload,
    ) -> Result<AppResponse, SharedStorageError> {
        let local_node_id = self.local_node_id.ok_or_else(|| SharedStorageError::Internal {
            reason: "trust initialization requires a numeric local node id".to_string(),
        })?;

        let share_bytes = payload
            .shares
            .iter()
            .find_map(|(node_id, bytes)| if *node_id == local_node_id { Some(bytes) } else { None })
            .ok_or_else(|| SharedStorageError::Internal {
                reason: format!("no trust share assigned for local node {local_node_id}"),
            })?;

        if share_bytes.len() != SECRET_SIZE + 1 {
            return Err(SharedStorageError::Internal {
                reason: format!("invalid trust share length {}, expected {}", share_bytes.len(), SECRET_SIZE + 1),
            });
        }

        let mut share_array = [0u8; SECRET_SIZE + 1];
        share_array.copy_from_slice(share_bytes);
        let share = Share::from_bytes(&share_array).map_err(|e| SharedStorageError::Internal {
            reason: format!("failed to decode trust share: {e}"),
        })?;

        shares_table.insert(payload.epoch, share.to_bytes().as_slice()).context(super::InsertSnafu)?;

        for (node_id, digest) in &payload.digests {
            let key = format!("{}:{}", payload.epoch, node_id);
            digests_table.insert(key.as_str(), digest.as_slice()).context(super::InsertSnafu)?;
        }

        Ok(AppResponse::default())
    }

    /// Load all digests for the given epoch.
    ///
    /// Returns a map from node_id to SHA3-256 digest.
    pub fn load_digests(&self, epoch: u64) -> Result<BTreeMap<u64, ShareDigest>, SharedStorageError> {
        let read_txn = self.db.begin_read().context(BeginReadSnafu)?;
        let table = read_txn.open_table(TRUST_DIGESTS_TABLE).context(OpenTableSnafu)?;

        let prefix = format!("{epoch}:");
        // Use range to scan all keys starting with "epoch:"
        // The range end is "epoch;" (';' is the char after ':' in ASCII)
        let range_end = format!("{epoch};");
        let mut digests = BTreeMap::new();

        for entry in table.range::<&str>(prefix.as_str()..range_end.as_str()).context(RangeSnafu)? {
            let (key_guard, value_guard) = entry.context(GetSnafu)?;
            let key_str: &str = key_guard.value();
            if let Some(node_id_str) = key_str.strip_prefix(&prefix)
                && let Ok(node_id) = node_id_str.parse::<u64>()
            {
                let bytes: &[u8] = value_guard.value();
                if bytes.len() == 32 {
                    let mut digest = [0u8; 32];
                    digest.copy_from_slice(bytes);
                    digests.insert(node_id, digest);
                }
            }
        }

        Ok(digests)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use tempfile::TempDir;

    use super::*;

    #[test]
    fn test_apply_trust_initialize_stores_only_local_share() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("trust-init.redb");
        let storage = SharedRedbStorage::new(&db_path, "2").unwrap();

        let secret = [9u8; SECRET_SIZE];
        let mut rng = rand::rng();
        let shares = aspen_trust::shamir::split_secret(&secret, 2, 3, &mut rng).unwrap();
        let payload = TrustInitializePayload {
            epoch: 1,
            shares: vec![
                (1, shares[0].to_bytes().to_vec()),
                (2, shares[1].to_bytes().to_vec()),
                (3, shares[2].to_bytes().to_vec()),
            ],
            digests: vec![
                (1, aspen_trust::shamir::share_digest(&shares[0])),
                (2, aspen_trust::shamir::share_digest(&shares[1])),
                (3, aspen_trust::shamir::share_digest(&shares[2])),
            ],
        };

        let write_txn = storage.db.begin_write().unwrap();
        {
            let mut shares_table = write_txn.open_table(TRUST_SHARES_TABLE).unwrap();
            let mut digests_table = write_txn.open_table(TRUST_DIGESTS_TABLE).unwrap();
            storage.apply_trust_initialize_in_txn(&mut shares_table, &mut digests_table, &payload).unwrap();
        }
        write_txn.commit().unwrap();

        let stored_share = storage.load_share(1).unwrap().unwrap();
        assert_eq!(stored_share, shares[1]);

        let digests = storage.load_digests(1).unwrap();
        let expected: BTreeMap<u64, ShareDigest> = payload.digests.iter().copied().collect();
        assert_eq!(digests, expected);
    }
}

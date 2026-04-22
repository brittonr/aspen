//! Trust share and digest storage operations.
//!
//! Stores Shamir secret sharing data in dedicated redb tables, isolated
//! from application KV operations.

use std::collections::BTreeMap;

use aspen_cluster_types::NodeAddress;
use aspen_raft_types::TrustInitializePayload;
use aspen_raft_types::TrustReconfigurationPayload;
use aspen_trust::chain::EncryptedSecretChain;
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
use super::RemoveSnafu;
use super::SharedRedbStorage;
use super::SharedStorageError;
use super::TRUST_CHAINS_TABLE;
use super::TRUST_DIGESTS_TABLE;
use super::TRUST_EXPUNGED_TABLE;
use super::TRUST_MEMBERS_TABLE;
use super::TRUST_NONCE_COUNTER_TABLE;
use super::TRUST_REENCRYPTION_PROGRESS_TABLE;
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

    /// Load the current trust epoch, if one has been committed.
    pub fn load_current_trust_epoch(&self) -> Result<Option<u64>, SharedStorageError> {
        self.read_sm_meta("trust_current_epoch")
    }

    /// Load the explicit threshold override for the current trust configuration.
    pub fn load_trust_threshold_override(&self) -> Result<Option<u8>, SharedStorageError> {
        self.read_sm_meta("trust_threshold_override")
    }

    /// Load membership addresses for the given trust epoch.
    pub fn load_members(&self, epoch: u64) -> Result<BTreeMap<u64, NodeAddress>, SharedStorageError> {
        let read_txn = self.db.begin_read().context(BeginReadSnafu)?;
        let table = read_txn.open_table(TRUST_MEMBERS_TABLE).context(OpenTableSnafu)?;

        let prefix = format!("{epoch}:");
        let range_end = format!("{epoch};");
        let mut members = BTreeMap::new();

        for entry in table.range::<&str>(prefix.as_str()..range_end.as_str()).context(RangeSnafu)? {
            let (key_guard, value_guard) = entry.context(GetSnafu)?;
            let key_str: &str = key_guard.value();
            if let Some(node_id_str) = key_str.strip_prefix(&prefix)
                && let Ok(node_id) = node_id_str.parse::<u64>()
            {
                let endpoint = bincode::deserialize(value_guard.value()).map_err(|e| SharedStorageError::Internal {
                    reason: format!("failed to deserialize NodeAddress: {e}"),
                })?;
                members.insert(node_id, endpoint);
            }
        }

        Ok(members)
    }

    /// Load the encrypted secret chain stored for the given epoch.
    pub fn load_encrypted_chain(&self, epoch: u64) -> Result<Option<EncryptedSecretChain>, SharedStorageError> {
        let read_txn = self.db.begin_read().context(BeginReadSnafu)?;
        let table = read_txn.open_table(TRUST_CHAINS_TABLE).context(OpenTableSnafu)?;
        let entry = table.get(epoch).context(GetSnafu)?;

        match entry {
            Some(value) => {
                let chain = bincode::deserialize(value.value()).map_err(|e| SharedStorageError::Internal {
                    reason: format!("failed to deserialize EncryptedSecretChain: {e}"),
                })?;
                Ok(Some(chain))
            }
            None => Ok(None),
        }
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
    ///
    /// Nodes that join after epoch 1 still replay the historical `TrustInitialize`
    /// entry during replication. Those nodes were not assigned an epoch-1 share,
    /// so they persist the cluster-wide trust metadata without storing a local share.
    /// If the payload says this node was an initial member, however, the share
    /// must still be present; otherwise the committed entry is malformed.
    pub(crate) fn apply_trust_initialize_in_txn(
        &self,
        shares_table: &mut redb::Table<u64, &[u8]>,
        digests_table: &mut redb::Table<&str, &[u8]>,
        members_table: &mut redb::Table<&str, &[u8]>,
        sm_meta_table: &mut redb::Table<&str, &[u8]>,
        payload: &TrustInitializePayload,
    ) -> Result<AppResponse, SharedStorageError> {
        let local_node_id = self.local_node_id.ok_or_else(|| SharedStorageError::Internal {
            reason: "trust operations require a numeric local node id".to_string(),
        })?;
        let local_is_initial_member = payload.members.iter().any(|(node_id, _)| *node_id == local_node_id);
        let local_share = decode_optional_local_share(Some(local_node_id), &payload.shares)?;
        if local_is_initial_member {
            let share = local_share.ok_or_else(|| SharedStorageError::Internal {
                reason: format!("no trust share assigned for initial member {local_node_id}"),
            })?;
            shares_table.insert(payload.epoch, share.to_bytes().as_slice()).context(super::InsertSnafu)?;
        } else if local_share.is_some() {
            return Err(SharedStorageError::Internal {
                reason: format!("trust share assigned for non-member {local_node_id}"),
            });
        }
        store_epoch_digests_in_txn(digests_table, payload.epoch, &payload.digests)?;
        store_epoch_members_in_txn(members_table, payload.epoch, &payload.members)?;
        store_trust_threshold_override_in_txn(sm_meta_table, payload.threshold_override)?;
        Ok(AppResponse::default())
    }

    /// Apply a committed trust reconfiguration request to the local storage tables.
    pub(crate) fn apply_trust_reconfiguration_in_txn(
        &self,
        shares_table: &mut redb::Table<u64, &[u8]>,
        digests_table: &mut redb::Table<&str, &[u8]>,
        chains_table: &mut redb::Table<u64, &[u8]>,
        members_table: &mut redb::Table<&str, &[u8]>,
        sm_meta_table: &mut redb::Table<&str, &[u8]>,
        payload: &TrustReconfigurationPayload,
    ) -> Result<AppResponse, SharedStorageError> {
        if let Some(share) = decode_optional_local_share(self.local_node_id, &payload.shares)? {
            shares_table.insert(payload.epoch, share.to_bytes().as_slice()).context(super::InsertSnafu)?;
        }
        store_epoch_digests_in_txn(digests_table, payload.epoch, &payload.digests)?;
        store_epoch_members_in_txn(members_table, payload.epoch, &payload.members)?;

        let chain_bytes = bincode::serialize(&payload.encrypted_chain).map_err(|e| SharedStorageError::Internal {
            reason: format!("failed to serialize EncryptedSecretChain: {e}"),
        })?;
        chains_table.insert(payload.epoch, chain_bytes.as_slice()).context(super::InsertSnafu)?;

        let epoch_bytes = bincode::serialize(&payload.epoch).map_err(|e| SharedStorageError::Internal {
            reason: format!("failed to serialize trust epoch: {e}"),
        })?;
        sm_meta_table.insert("trust_current_epoch", epoch_bytes.as_slice()).context(super::InsertSnafu)?;
        store_trust_threshold_override_in_txn(sm_meta_table, payload.threshold_override)?;

        Ok(AppResponse::default())
    }

    /// Persist the nonce counter for a given node.
    ///
    /// Called by the encryption layer after each `wrap_write` to ensure
    /// nonce counters survive restarts and prevent nonce reuse.
    pub fn store_nonce_counter(&self, node_id: u64, counter: u64) -> Result<(), SharedStorageError> {
        let write_txn = self.db.begin_write().context(BeginWriteSnafu)?;
        {
            let mut table = write_txn.open_table(TRUST_NONCE_COUNTER_TABLE).context(OpenTableSnafu)?;
            table.insert(node_id, counter).context(super::InsertSnafu)?;
        }
        write_txn.commit().context(CommitSnafu)?;
        Ok(())
    }

    /// Load the persisted nonce counter for a given node.
    ///
    /// Returns `None` if no counter has been persisted yet (fresh node).
    pub fn load_nonce_counter(&self, node_id: u64) -> Result<Option<u64>, SharedStorageError> {
        let read_txn = self.db.begin_read().context(BeginReadSnafu)?;
        let table = read_txn.open_table(TRUST_NONCE_COUNTER_TABLE).context(OpenTableSnafu)?;
        let entry = table.get(node_id).context(GetSnafu)?;
        Ok(entry.map(|v| v.value()))
    }

    /// Save re-encryption checkpoint progress for a table prefix.
    pub fn save_reencryption_checkpoint(&self, table_name: &str, last_key: &str) -> Result<(), SharedStorageError> {
        let write_txn = self.db.begin_write().context(BeginWriteSnafu)?;
        {
            let mut table = write_txn.open_table(TRUST_REENCRYPTION_PROGRESS_TABLE).context(OpenTableSnafu)?;
            table.insert(table_name, last_key.as_bytes()).context(super::InsertSnafu)?;
        }
        write_txn.commit().context(CommitSnafu)?;
        Ok(())
    }

    /// Load re-encryption checkpoint progress for a table prefix.
    pub fn load_reencryption_checkpoint(&self, table_name: &str) -> Result<Option<String>, SharedStorageError> {
        let read_txn = self.db.begin_read().context(BeginReadSnafu)?;
        let table = read_txn.open_table(TRUST_REENCRYPTION_PROGRESS_TABLE).context(OpenTableSnafu)?;
        let entry = table.get(table_name).context(GetSnafu)?;
        entry
            .map(|value| {
                let bytes: &[u8] = value.value();
                String::from_utf8(bytes.to_vec()).map_err(|error| SharedStorageError::Internal {
                    reason: format!("invalid reencryption checkpoint UTF-8: {error}"),
                })
            })
            .transpose()
    }

    /// Clear re-encryption checkpoint progress for a table prefix.
    pub fn clear_reencryption_checkpoint(&self, table_name: &str) -> Result<(), SharedStorageError> {
        let write_txn = self.db.begin_write().context(BeginWriteSnafu)?;
        {
            let mut table = write_txn.open_table(TRUST_REENCRYPTION_PROGRESS_TABLE).context(OpenTableSnafu)?;
            table.remove(table_name).context(RemoveSnafu)?;
        }
        write_txn.commit().context(CommitSnafu)?;
        Ok(())
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

fn decode_optional_local_share(
    local_node_id: Option<u64>,
    shares: &[(u64, Vec<u8>)],
) -> Result<Option<Share>, SharedStorageError> {
    let local_node_id = local_node_id.ok_or_else(|| SharedStorageError::Internal {
        reason: "trust operations require a numeric local node id".to_string(),
    })?;

    let Some(share_bytes) = shares
        .iter()
        .find_map(|(node_id, bytes)| if *node_id == local_node_id { Some(bytes) } else { None })
    else {
        return Ok(None);
    };

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
    Ok(Some(share))
}

fn store_epoch_digests_in_txn(
    digests_table: &mut redb::Table<&str, &[u8]>,
    epoch: u64,
    digests: &[(u64, [u8; 32])],
) -> Result<(), SharedStorageError> {
    for (node_id, digest) in digests {
        let key = format!("{epoch}:{node_id}");
        digests_table.insert(key.as_str(), digest.as_slice()).context(super::InsertSnafu)?;
    }
    Ok(())
}

fn store_epoch_members_in_txn(
    members_table: &mut redb::Table<&str, &[u8]>,
    epoch: u64,
    members: &[(u64, NodeAddress)],
) -> Result<(), SharedStorageError> {
    for (node_id, node_addr) in members {
        let key = format!("{epoch}:{node_id}");
        let node_addr_bytes = bincode::serialize(node_addr).map_err(|e| SharedStorageError::Internal {
            reason: format!("failed to serialize NodeAddress: {e}"),
        })?;
        members_table.insert(key.as_str(), node_addr_bytes.as_slice()).context(super::InsertSnafu)?;
    }
    Ok(())
}

fn store_trust_threshold_override_in_txn(
    sm_meta_table: &mut redb::Table<&str, &[u8]>,
    threshold_override: Option<u8>,
) -> Result<(), SharedStorageError> {
    match threshold_override {
        Some(value) => {
            let bytes = bincode::serialize(&value).map_err(|e| SharedStorageError::Internal {
                reason: format!("failed to serialize trust threshold override: {e}"),
            })?;
            sm_meta_table.insert("trust_threshold_override", bytes.as_slice()).context(super::InsertSnafu)?;
        }
        None => {
            sm_meta_table.remove("trust_threshold_override").context(super::RemoveSnafu)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use iroh::EndpointAddr;
    use iroh::SecretKey;
    use tempfile::TempDir;

    use super::*;

    fn node_addr() -> NodeAddress {
        let key = SecretKey::generate(&mut rand::rng());
        NodeAddress::new(EndpointAddr::new(key.public()))
    }

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
            threshold_override: Some(2),
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
            members: vec![(1, node_addr()), (2, node_addr()), (3, node_addr())],
        };

        let write_txn = storage.db.begin_write().unwrap();
        {
            let mut shares_table = write_txn.open_table(TRUST_SHARES_TABLE).unwrap();
            let mut digests_table = write_txn.open_table(TRUST_DIGESTS_TABLE).unwrap();
            let mut members_table = write_txn.open_table(TRUST_MEMBERS_TABLE).unwrap();
            let mut sm_meta_table = write_txn.open_table(super::super::SM_META_TABLE).unwrap();
            storage
                .apply_trust_initialize_in_txn(
                    &mut shares_table,
                    &mut digests_table,
                    &mut members_table,
                    &mut sm_meta_table,
                    &payload,
                )
                .unwrap();
        }
        write_txn.commit().unwrap();

        let stored_share = storage.load_share(1).unwrap().unwrap();
        assert_eq!(stored_share, shares[1]);

        let digests = storage.load_digests(1).unwrap();
        let expected: BTreeMap<u64, ShareDigest> = payload.digests.iter().copied().collect();
        assert_eq!(digests, expected);
        assert_eq!(storage.load_trust_threshold_override().unwrap(), Some(2));
        assert_eq!(storage.load_members(1).unwrap().len(), 3);
    }

    #[test]
    fn test_apply_trust_initialize_allows_later_joiner_without_share() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("trust-init-late-joiner.redb");
        let storage = SharedRedbStorage::new(&db_path, "4").unwrap();

        let secret = [5u8; SECRET_SIZE];
        let mut rng = rand::rng();
        let shares = aspen_trust::shamir::split_secret(&secret, 2, 3, &mut rng).unwrap();
        let payload = TrustInitializePayload {
            epoch: 1,
            threshold_override: Some(2),
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
            members: vec![(1, node_addr()), (2, node_addr()), (3, node_addr())],
        };

        let write_txn = storage.db.begin_write().unwrap();
        {
            let mut shares_table = write_txn.open_table(TRUST_SHARES_TABLE).unwrap();
            let mut digests_table = write_txn.open_table(TRUST_DIGESTS_TABLE).unwrap();
            let mut members_table = write_txn.open_table(TRUST_MEMBERS_TABLE).unwrap();
            let mut sm_meta_table = write_txn.open_table(super::super::SM_META_TABLE).unwrap();
            storage
                .apply_trust_initialize_in_txn(
                    &mut shares_table,
                    &mut digests_table,
                    &mut members_table,
                    &mut sm_meta_table,
                    &payload,
                )
                .unwrap();
        }
        write_txn.commit().unwrap();

        assert_eq!(storage.load_share(1).unwrap(), None);
        assert_eq!(storage.load_digests(1).unwrap().len(), 3);
        assert_eq!(storage.load_trust_threshold_override().unwrap(), Some(2));
        assert_eq!(storage.load_members(1).unwrap().len(), 3);
    }

    #[test]
    fn test_apply_trust_initialize_rejects_share_for_non_member() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("trust-init-non-member-share.redb");
        let storage = SharedRedbStorage::new(&db_path, "4").unwrap();

        let secret = [4u8; SECRET_SIZE];
        let mut rng = rand::rng();
        let shares = aspen_trust::shamir::split_secret(&secret, 2, 4, &mut rng).unwrap();
        let payload = TrustInitializePayload {
            epoch: 1,
            threshold_override: Some(2),
            shares: vec![
                (1, shares[0].to_bytes().to_vec()),
                (2, shares[1].to_bytes().to_vec()),
                (3, shares[2].to_bytes().to_vec()),
                (4, shares[3].to_bytes().to_vec()),
            ],
            digests: vec![
                (1, aspen_trust::shamir::share_digest(&shares[0])),
                (2, aspen_trust::shamir::share_digest(&shares[1])),
                (3, aspen_trust::shamir::share_digest(&shares[2])),
                (4, aspen_trust::shamir::share_digest(&shares[3])),
            ],
            members: vec![(1, node_addr()), (2, node_addr()), (3, node_addr())],
        };

        let write_txn = storage.db.begin_write().unwrap();
        let err = {
            let mut shares_table = write_txn.open_table(TRUST_SHARES_TABLE).unwrap();
            let mut digests_table = write_txn.open_table(TRUST_DIGESTS_TABLE).unwrap();
            let mut members_table = write_txn.open_table(TRUST_MEMBERS_TABLE).unwrap();
            let mut sm_meta_table = write_txn.open_table(super::super::SM_META_TABLE).unwrap();
            storage
                .apply_trust_initialize_in_txn(
                    &mut shares_table,
                    &mut digests_table,
                    &mut members_table,
                    &mut sm_meta_table,
                    &payload,
                )
                .unwrap_err()
        };

        assert!(matches!(err, SharedStorageError::Internal { .. }));
        assert!(err.to_string().contains("trust share assigned for non-member 4"));
    }

    #[test]
    fn test_apply_trust_initialize_requires_share_for_initial_member() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("trust-init-missing-local-share.redb");
        let storage = SharedRedbStorage::new(&db_path, "2").unwrap();

        let secret = [6u8; SECRET_SIZE];
        let mut rng = rand::rng();
        let shares = aspen_trust::shamir::split_secret(&secret, 2, 3, &mut rng).unwrap();
        let payload = TrustInitializePayload {
            epoch: 1,
            threshold_override: Some(2),
            shares: vec![(1, shares[0].to_bytes().to_vec()), (3, shares[2].to_bytes().to_vec())],
            digests: vec![
                (1, aspen_trust::shamir::share_digest(&shares[0])),
                (2, aspen_trust::shamir::share_digest(&shares[1])),
                (3, aspen_trust::shamir::share_digest(&shares[2])),
            ],
            members: vec![(1, node_addr()), (2, node_addr()), (3, node_addr())],
        };

        let write_txn = storage.db.begin_write().unwrap();
        let err = {
            let mut shares_table = write_txn.open_table(TRUST_SHARES_TABLE).unwrap();
            let mut digests_table = write_txn.open_table(TRUST_DIGESTS_TABLE).unwrap();
            let mut members_table = write_txn.open_table(TRUST_MEMBERS_TABLE).unwrap();
            let mut sm_meta_table = write_txn.open_table(super::super::SM_META_TABLE).unwrap();
            storage
                .apply_trust_initialize_in_txn(
                    &mut shares_table,
                    &mut digests_table,
                    &mut members_table,
                    &mut sm_meta_table,
                    &payload,
                )
                .unwrap_err()
        };

        assert!(matches!(err, SharedStorageError::Internal { .. }));
        assert!(err.to_string().contains("no trust share assigned for initial member 2"));
    }

    #[test]
    fn test_apply_trust_reconfiguration_updates_epoch_and_chain() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("trust-reconfig.redb");
        let storage = SharedRedbStorage::new(&db_path, "2").unwrap();

        let secret = [7u8; SECRET_SIZE];
        let mut rng = rand::rng();
        let shares = aspen_trust::shamir::split_secret(&secret, 2, 3, &mut rng).unwrap();
        let payload = TrustReconfigurationPayload {
            epoch: 8,
            threshold_override: Some(2),
            shares: vec![
                (1, shares[0].to_bytes().to_vec()),
                (2, shares[1].to_bytes().to_vec()),
                (4, shares[2].to_bytes().to_vec()),
            ],
            digests: vec![
                (1, aspen_trust::shamir::share_digest(&shares[0])),
                (2, aspen_trust::shamir::share_digest(&shares[1])),
                (4, aspen_trust::shamir::share_digest(&shares[2])),
            ],
            members: vec![(1, node_addr()), (2, node_addr()), (4, node_addr())],
            encrypted_chain: EncryptedSecretChain {
                salt: [4; 32],
                data: vec![9, 8, 7],
                epoch: 8,
                prior_count: 3,
            },
        };

        let write_txn = storage.db.begin_write().unwrap();
        {
            let mut shares_table = write_txn.open_table(TRUST_SHARES_TABLE).unwrap();
            let mut digests_table = write_txn.open_table(TRUST_DIGESTS_TABLE).unwrap();
            let mut chains_table = write_txn.open_table(TRUST_CHAINS_TABLE).unwrap();
            let mut members_table = write_txn.open_table(TRUST_MEMBERS_TABLE).unwrap();
            let mut sm_meta_table = write_txn.open_table(super::super::SM_META_TABLE).unwrap();
            storage
                .apply_trust_reconfiguration_in_txn(
                    &mut shares_table,
                    &mut digests_table,
                    &mut chains_table,
                    &mut members_table,
                    &mut sm_meta_table,
                    &payload,
                )
                .unwrap();
        }
        write_txn.commit().unwrap();

        assert_eq!(storage.load_share(8).unwrap().unwrap(), shares[1]);
        assert_eq!(storage.load_digests(8).unwrap().len(), 3);
        assert_eq!(storage.load_current_trust_epoch().unwrap(), Some(8));
        assert_eq!(storage.load_trust_threshold_override().unwrap(), Some(2));
        assert_eq!(storage.load_members(8).unwrap().len(), 3);
        assert_eq!(storage.load_encrypted_chain(8).unwrap(), Some(payload.encrypted_chain));
    }

    #[test]
    fn test_apply_trust_reconfiguration_allows_removed_local_node() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("trust-reconfig-removed.redb");
        let storage = SharedRedbStorage::new(&db_path, "3").unwrap();

        let secret = [7u8; SECRET_SIZE];
        let mut rng = rand::rng();
        let shares = aspen_trust::shamir::split_secret(&secret, 2, 3, &mut rng).unwrap();
        let payload = TrustReconfigurationPayload {
            epoch: 8,
            threshold_override: Some(2),
            shares: vec![
                (1, shares[0].to_bytes().to_vec()),
                (2, shares[1].to_bytes().to_vec()),
                (4, shares[2].to_bytes().to_vec()),
            ],
            digests: vec![
                (1, aspen_trust::shamir::share_digest(&shares[0])),
                (2, aspen_trust::shamir::share_digest(&shares[1])),
                (4, aspen_trust::shamir::share_digest(&shares[2])),
            ],
            members: vec![(1, node_addr()), (2, node_addr()), (4, node_addr())],
            encrypted_chain: EncryptedSecretChain {
                salt: [4; 32],
                data: vec![9, 8, 7],
                epoch: 8,
                prior_count: 3,
            },
        };

        let write_txn = storage.db.begin_write().unwrap();
        {
            let mut shares_table = write_txn.open_table(TRUST_SHARES_TABLE).unwrap();
            let mut digests_table = write_txn.open_table(TRUST_DIGESTS_TABLE).unwrap();
            let mut chains_table = write_txn.open_table(TRUST_CHAINS_TABLE).unwrap();
            let mut members_table = write_txn.open_table(TRUST_MEMBERS_TABLE).unwrap();
            let mut sm_meta_table = write_txn.open_table(super::super::SM_META_TABLE).unwrap();
            storage
                .apply_trust_reconfiguration_in_txn(
                    &mut shares_table,
                    &mut digests_table,
                    &mut chains_table,
                    &mut members_table,
                    &mut sm_meta_table,
                    &payload,
                )
                .unwrap();
        }
        write_txn.commit().unwrap();

        assert_eq!(storage.load_share(8).unwrap(), None);
        assert_eq!(storage.load_current_trust_epoch().unwrap(), Some(8));
        assert_eq!(storage.load_members(8).unwrap().len(), 3);
        assert_eq!(storage.load_encrypted_chain(8).unwrap(), Some(payload.encrypted_chain));
    }

    #[test]
    fn test_mark_expunged_sets_flag_and_zeroizes_shares() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("trust-expunge.redb");
        let storage = SharedRedbStorage::new(&db_path, "2").unwrap();

        // Store shares for two epochs
        let secret = [8u8; SECRET_SIZE];
        let mut rng = rand::rng();
        let shares = aspen_trust::shamir::split_secret(&secret, 2, 3, &mut rng).unwrap();
        storage.store_share(1, &shares[0]).unwrap();
        storage.store_share(2, &shares[1]).unwrap();

        // Verify shares exist
        assert!(storage.load_share(1).unwrap().is_some());
        assert!(storage.load_share(2).unwrap().is_some());
        assert!(!storage.is_expunged().unwrap());

        // Expunge the node
        let metadata = aspen_cluster_types::ExpungedMetadata {
            epoch: 5,
            removed_by: 1,
            timestamp_ms: 1234567890,
        };
        storage.mark_expunged(metadata.clone()).unwrap();

        // Verify expungement
        assert!(storage.is_expunged().unwrap());
        let loaded = storage.load_expunged().unwrap().unwrap();
        assert_eq!(loaded.epoch, 5);
        assert_eq!(loaded.removed_by, 1);

        // Verify all shares are gone
        assert_eq!(storage.load_share(1).unwrap(), None);
        assert_eq!(storage.load_share(2).unwrap(), None);
    }

    #[test]
    fn test_expunged_node_rejects_share_load() {
        // After expungement, load_share returns None for all epochs,
        // and is_expunged() returns true.
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("trust-expunge-rejects.redb");
        let storage = SharedRedbStorage::new(&db_path, "2").unwrap();

        let secret = [2u8; SECRET_SIZE];
        let mut rng = rand::rng();
        let shares = aspen_trust::shamir::split_secret(&secret, 2, 3, &mut rng).unwrap();
        storage.store_share(1, &shares[0]).unwrap();
        storage.store_share(5, &shares[1]).unwrap();
        storage.store_share(10, &shares[2]).unwrap();

        // All shares accessible before expungement
        assert!(storage.load_share(1).unwrap().is_some());
        assert!(storage.load_share(5).unwrap().is_some());
        assert!(storage.load_share(10).unwrap().is_some());

        storage
            .mark_expunged(aspen_cluster_types::ExpungedMetadata {
                epoch: 11,
                removed_by: 1,
                timestamp_ms: 0,
            })
            .unwrap();

        // All shares gone after expungement
        assert!(storage.is_expunged().unwrap());
        assert_eq!(storage.load_share(1).unwrap(), None);
        assert_eq!(storage.load_share(5).unwrap(), None);
        assert_eq!(storage.load_share(10).unwrap(), None);
    }

    #[test]
    fn test_expungement_survives_reload() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("trust-expunge-reload.redb");

        // Phase 1: mark expunged
        {
            let storage = SharedRedbStorage::new(&db_path, "2").unwrap();
            let secret = [1u8; SECRET_SIZE];
            let mut rng = rand::rng();
            let shares = aspen_trust::shamir::split_secret(&secret, 2, 3, &mut rng).unwrap();
            storage.store_share(1, &shares[0]).unwrap();

            storage
                .mark_expunged(aspen_cluster_types::ExpungedMetadata {
                    epoch: 3,
                    removed_by: 1,
                    timestamp_ms: 999,
                })
                .unwrap();
        }

        // Phase 2: reopen and check
        {
            let storage = SharedRedbStorage::new(&db_path, "2").unwrap();
            assert!(storage.is_expunged().unwrap());
            let metadata = storage.load_expunged().unwrap().unwrap();
            assert_eq!(metadata.epoch, 3);
            assert_eq!(metadata.removed_by, 1);
            assert_eq!(storage.load_share(1).unwrap(), None);
        }
    }
}

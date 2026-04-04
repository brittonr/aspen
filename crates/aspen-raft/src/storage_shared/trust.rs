//! Trust share and digest storage operations.
//!
//! Stores Shamir secret sharing data in dedicated redb tables, isolated
//! from application KV operations.

use std::collections::BTreeMap;

use aspen_trust::shamir::SECRET_SIZE;
use aspen_trust::shamir::Share;
use aspen_trust::shamir::ShareDigest;
use snafu::ResultExt;

use super::BeginReadSnafu;
use super::BeginWriteSnafu;
use super::CommitSnafu;
use super::GetSnafu;
use super::OpenTableSnafu;
use super::RangeSnafu;
use super::SharedRedbStorage;
use super::SharedStorageError;
use super::TRUST_DIGESTS_TABLE;
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

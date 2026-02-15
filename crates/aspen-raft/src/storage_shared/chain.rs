//! Chain integrity: chain hash verification and integrity checking operations.

use snafu::ResultExt;

use super::*;

impl SharedRedbStorage {
    /// Get the current chain tip for verification.
    pub fn chain_tip_for_verification(&self) -> Result<(u64, ChainHash), SharedStorageError> {
        let chain_tip = self.chain_tip.read().map_err(|_| SharedStorageError::LockPoisoned {
            context: "reading chain_tip for verification".into(),
        })?;
        Ok((chain_tip.index, chain_tip.hash))
    }

    /// Read chain hash at a specific log index.
    pub(super) fn read_chain_hash_at(&self, index: u64) -> Result<Option<ChainHash>, SharedStorageError> {
        let read_txn = self.db.begin_read().context(BeginReadSnafu)?;
        let table = read_txn.open_table(CHAIN_HASH_TABLE).context(OpenTableSnafu)?;

        match table.get(index).context(GetSnafu)? {
            Some(value) => {
                let bytes = value.value();
                // Tiger Style: chain hash must be exactly 32 bytes if present
                debug_assert!(
                    bytes.len() == 32 || bytes.is_empty(),
                    "CHAIN: hash at index {index} has unexpected size {}",
                    bytes.len()
                );
                if bytes.len() == 32 {
                    let mut hash = [0u8; 32];
                    hash.copy_from_slice(bytes);
                    Ok(Some(hash))
                } else {
                    Ok(None)
                }
            }
            None => Ok(None),
        }
    }
}

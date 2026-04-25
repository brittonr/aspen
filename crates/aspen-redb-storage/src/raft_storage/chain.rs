//! Chain integrity: chain hash verification and integrity checking operations.

use snafu::ResultExt;

use super::BeginReadSnafu;
use super::GetSnafu;
use super::OpenTableSnafu;
use super::RedbKvStorage;
use super::SharedStorageError;
use super::CHAIN_HASH_TABLE;

use crate::ChainHash;

impl RedbKvStorage {
    pub fn chain_tip_for_verification(&self) -> Result<(u64, ChainHash), SharedStorageError> {
        let chain_tip = self.chain_tip.read().map_err(|_| SharedStorageError::LockPoisoned {
            context: "reading chain_tip for verification".into(),
        })?;
        Ok((chain_tip.index, chain_tip.hash))
    }

    pub(super) fn read_chain_hash_at(&self, index: u64) -> Result<Option<ChainHash>, SharedStorageError> {
        let read_txn = self.db.begin_read().context(BeginReadSnafu)?;
        let table = read_txn.open_table(CHAIN_HASH_TABLE).context(OpenTableSnafu)?;

        match table.get(index).context(GetSnafu)? {
            Some(value) => {
                let bytes = value.value();
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

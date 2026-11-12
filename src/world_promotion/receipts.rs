use std::path::Path;

use molten_node_host::node_state::DirectoryRoot;
use molten_node_host::node_state::NamespaceKind;
use molten_node_host::node_state::RelativePath;
use molten_node_host::node_state::Scope;

use super::CanonicalWorldPromotionRecord;
use super::WorldPromotionReceiptPort;
use crate::error::Failure;
use crate::error::Result;

const RECEIPT_DIRECTORY: &str = "world-promotion/receipts";
const MAX_RECEIPT_BYTES: u64 = 1 << 20;

/// Content-addressed receipt storage under the node storage namespace.
pub struct LocalPromotionReceipts {
    storage: Scope,
}

impl LocalPromotionReceipts {
    pub fn open(state_root: &Path) -> Result<Self> {
        let root = DirectoryRoot::open(state_root)?;
        let storage = root.namespace(NamespaceKind::Storage)?;
        Ok(Self { storage })
    }
}

impl WorldPromotionReceiptPort for LocalPromotionReceipts {
    fn publish_promotion_receipt(&mut self, receipt: &CanonicalWorldPromotionRecord) -> Result<()> {
        let hash = blake3::hash(&receipt.bytes).to_hex().to_string();
        let path = RelativePath::parse(&format!("{RECEIPT_DIRECTORY}/{hash}.preserves"))?;
        if self.storage.try_exists(&path)? {
            let existing = self.storage.read(&path, MAX_RECEIPT_BYTES)?;
            return if existing == receipt.bytes {
                Ok(())
            } else {
                Err(Failure::invalid_harness("stored promotion receipt does not match its content address"))
            };
        }
        self.storage.create_dir_all(&RelativePath::parse(RECEIPT_DIRECTORY)?)?;
        self.storage.write(&path, &receipt.bytes)?;
        Ok(())
    }
}

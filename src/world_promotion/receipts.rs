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
#[cfg(test)]
mod tests {
    use molten_node_host::node_state::NamespaceKind;

    use super::*;

    fn receipt(bytes: &[u8]) -> CanonicalWorldPromotionRecord {
        CanonicalWorldPromotionRecord {
            record_ref: "blake3:0000000000000000000000000000000000000000000000000000000000000000".to_string(),
            value: preserves::IOValue::new(0),
            bytes: bytes.to_vec(),
        }
    }

    #[test]
    fn receipts_are_content_addressed_idempotent_and_tamper_evident() {
        let temporary = cap_tempfile::tempdir(cap_std::ambient_authority()).expect("temporary state root");
        let root = DirectoryRoot::from_dir(temporary.try_clone().expect("clone root"));
        root.create_layout().expect("state layout");
        let mut receipts = LocalPromotionReceipts {
            storage: root.namespace(NamespaceKind::Storage).expect("storage"),
        };
        let record = receipt(b"canonical promotion receipt");
        receipts.publish_promotion_receipt(&record).expect("first publication");
        receipts.publish_promotion_receipt(&record).expect("idempotent publication");
        let hash = blake3::hash(&record.bytes).to_hex().to_string();
        let path = RelativePath::parse(&format!("{RECEIPT_DIRECTORY}/{hash}.preserves")).expect("receipt path");
        receipts.storage.write(&path, b"tampered").expect("tamper");
        assert!(receipts.publish_promotion_receipt(&record).is_err());
    }
}

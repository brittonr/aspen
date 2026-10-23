use std::collections::BTreeMap;
use std::collections::BTreeSet;

use super::*;

/// The index and catalog rebuilt from the objects a reopened heap holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveredIndex {
    /// Digest to object mappings, in digest order.
    pub entries: Vec<(Digest, u64)>,
    /// The recovered catalog, or the empty catalog of a fresh store.
    pub catalog: MarbleCatalog,
    /// Extra objects that repeat a digest with an identical payload. The
    /// lowest identifier stays the logical object; these are never served.
    pub redundant_object_ids: Vec<u64>,
}

/// Rebuild the digest index from a reopened heap, one object at a time.
///
/// Marble recovers only batches whose trailer it can verify, so the objects
/// observed here are exactly the atomically recovered batches: an interrupted
/// batch contributes no object and therefore no mapping. Each object's own
/// checksum is marble's; this core checks the frame, the identifier range,
/// and that one digest never names two different payloads.
// r[impl aspen.marble_store.identity]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexRecovery {
    layout: ObjectIdLayout,
    max_object_bytes: u32,
    entries: BTreeMap<Digest, (u64, blake3::Hash)>,
    redundant: BTreeSet<u64>,
    catalog: Option<MarbleCatalog>,
    issues: BTreeSet<MarbleStoreIssue>,
}

impl IndexRecovery {
    pub const fn new(layout: ObjectIdLayout, max_object_bytes: u32) -> Self {
        Self {
            layout,
            max_object_bytes,
            entries: BTreeMap::new(),
            redundant: BTreeSet::new(),
            catalog: None,
            issues: BTreeSet::new(),
        }
    }

    /// Observe one live object the heap returned for `object_id`.
    pub fn observe(&mut self, object_id: u64, object: &[u8]) {
        match self.layout.kind_of(object_id) {
            None => {
                self.issues.insert(MarbleStoreIssue::ForeignObjectId(object_id));
            }
            Some(ObjectKind::Catalog) => self.observe_catalog(object_id, object),
            Some(ObjectKind::Block) => self.observe_block(object_id, object),
        }
    }

    fn observe_catalog(&mut self, object_id: u64, object: &[u8]) {
        if object_id != self.layout.catalog_id() {
            self.issues.insert(MarbleStoreIssue::ForeignObjectId(object_id));
            return;
        }
        match decode_catalog(object) {
            Ok(catalog) => self.catalog = Some(catalog),
            Err(issue) => {
                self.issues.insert(issue);
            }
        }
    }

    fn observe_block(&mut self, object_id: u64, object: &[u8]) {
        let frame = match decode_block_frame(object, self.max_object_bytes) {
            Ok(frame) => frame,
            Err(issue) => {
                self.issues.insert(issue);
                return;
            }
        };
        let payload_hash = blake3::hash(frame.payload);
        let Some((kept_id, kept_hash)) = self.entries.get(&frame.digest).copied() else {
            self.entries.insert(frame.digest, (object_id, payload_hash));
            return;
        };
        if kept_hash != payload_hash {
            self.issues.insert(MarbleStoreIssue::RecoveredDigestConflict);
            return;
        }
        let (lower, higher) = if object_id < kept_id {
            (object_id, kept_id)
        } else {
            (kept_id, object_id)
        };
        self.entries.insert(frame.digest, (lower, payload_hash));
        self.redundant.insert(higher);
    }

    /// Finish recovery: the rebuilt index, or every issue that refuses it.
    pub fn finish(self) -> Result<RecoveredIndex, Vec<MarbleStoreIssue>> {
        if !self.issues.is_empty() {
            return Err(self.issues.into_iter().collect());
        }
        Ok(RecoveredIndex {
            entries: self.entries.into_iter().map(|(digest, (object_id, _))| (digest, object_id)).collect(),
            catalog: self.catalog.unwrap_or_default(),
            redundant_object_ids: self.redundant.into_iter().collect(),
        })
    }
}

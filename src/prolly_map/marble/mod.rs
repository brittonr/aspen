//! The marble-plus-art Prolly block store spike.
//!
//! An optional, configuration-selected physical backend behind the Prolly
//! block-store and retention ports. It is measured against the Redb store and
//! is never the default path. Every blocking heap operation runs on one bounded
//! executor worker, which also owns the maintenance schedule. Marble opens its
//! heap by filesystem path, so the caller supplies the heap directory; the
//! capability-rooted node-state namespaces do not reach this backend.

use molten_core::marble_store::*;
use molten_core::prolly_map::*;

use super::storesession::read_session_error;
use super::*;

mod backend;
mod executor;
mod retention;

pub use retention::MarbleStoreStatistics;

/// A Prolly block store over a marble heap and an art digest index.
pub struct MarbleArtBlockStore {
    executor: executor::StoreExecutor<backend::MarbleBackend>,
}

impl MarbleArtBlockStore {
    /// Open the store at `heap_directory`, recovering its index on the worker.
    // r[impl aspen.marble_store.spike]
    pub fn open(heap_directory: &std::path::Path, profile: &MarbleStoreProfile) -> ProllyPortResult<Self> {
        let directory = heap_directory.to_path_buf();
        let profile = *profile;
        let executor = executor::StoreExecutor::start(profile.executor_queue_depth, move || {
            backend::MarbleBackend::open(directory, profile)
        })?;
        Ok(Self { executor })
    }

    /// Defragment the heap now, outside the backend's own schedule.
    pub fn maintain(&mut self) -> ProllyPortResult<u64> {
        self.executor.run(backend::MarbleBackend::maintain)?
    }

    pub fn statistics(&self) -> ProllyPortResult<MarbleStoreStatistics> {
        self.executor.run(|state: &mut backend::MarbleBackend| state.statistics())?
    }
}

impl ProllyBlockStorePort for MarbleArtBlockStore {
    fn read_block(&self, node_ref: &NodeRef) -> ProllyPortResult<Option<Vec<u8>>> {
        let node_ref = node_ref.clone();
        self.executor.run(move |state: &mut backend::MarbleBackend| state.read_block(&node_ref))?
    }

    fn open_read_session(&self, bounds: ReadSessionBounds) -> ProllyPortResult<Box<dyn ProllyReadSession + '_>> {
        Ok(Box::new(MarbleReadSession {
            store: Some(self),
            bounds,
            blocks_read: 0,
            bytes_copied: 0,
            is_exhausted: false,
        }))
    }

    fn stage_blocks(&mut self, blocks: &[EncodedBlock]) -> ProllyPortResult<()> {
        let blocks = blocks.to_vec();
        self.executor.run(move |state: &mut backend::MarbleBackend| state.stage_blocks(&blocks))?
    }

    fn read_root(&self, map_id: &str) -> ProllyPortResult<Option<PublishedProllyRoot>> {
        let map_id = map_id.to_owned();
        self.executor.run(move |state: &mut backend::MarbleBackend| state.read_root(&map_id))?
    }

    fn compare_and_advance(
        &mut self,
        map_id: &str,
        expected: &ExpectedProllyRoot,
        next: &PublishedProllyRoot,
    ) -> ProllyPortResult<ProllyPublicationObservation> {
        let (map_id, expected, next) = (map_id.to_owned(), expected.clone(), next.clone());
        self.executor
            .run(move |state: &mut backend::MarbleBackend| state.compare_and_advance(map_id, &expected, &next))?
    }
}

impl ProllyRetentionPort for MarbleArtBlockStore {
    fn pin_blocks(&mut self, node_refs: &[NodeRef]) -> ProllyPortResult<()> {
        let node_refs = node_refs.to_vec();
        self.executor.run(move |state: &mut backend::MarbleBackend| state.pin_blocks(&node_refs))?
    }

    fn release_pins(&mut self, node_refs: &[NodeRef]) -> ProllyPortResult<()> {
        let node_refs = node_refs.to_vec();
        self.executor.run(move |state: &mut backend::MarbleBackend| state.release_pins(&node_refs))?
    }

    fn retention_inventory(&self) -> ProllyPortResult<ProllyRetentionInventory> {
        self.executor.run(|state: &mut backend::MarbleBackend| state.retention_inventory())?
    }

    fn compare_retention_inventory_and_delete(
        &mut self,
        expected: &ProllyRetentionInventory,
        candidates: &[NodeRef],
    ) -> ProllyPortResult<GcDeletionOutcome> {
        let (expected, candidates) = (expected.clone(), candidates.to_vec());
        self.executor.run(move |state: &mut backend::MarbleBackend| {
            state.compare_retention_inventory_and_delete(&expected, candidates)
        })?
    }
}

/// A bounded read session over the marble store.
///
/// Marble has no read transaction; the session's borrow of the store keeps
/// every mutation out while it is open.
struct MarbleReadSession<'a> {
    store: Option<&'a MarbleArtBlockStore>,
    bounds: ReadSessionBounds,
    blocks_read: u32,
    bytes_copied: u64,
    is_exhausted: bool,
}

impl ProllyReadSession for MarbleReadSession<'_> {
    fn read_block(&mut self, node_ref: &NodeRef) -> ProllyPortResult<Option<Vec<u8>>> {
        if self.is_exhausted {
            return Err(read_session_error("prolly-read-session-exhausted", "the read session bounds are exhausted"));
        }
        let Some(store) = self.store else {
            return Err(read_session_error("prolly-read-session-closed", "the read session is closed"));
        };
        if self.blocks_read >= self.bounds.max_blocks {
            self.is_exhausted = true;
            return Err(read_session_error(
                "prolly-read-session-block-bound",
                "the read session block bound is reached",
            ));
        }
        let bytes = store.read_block(node_ref)?;
        let length = bytes.as_ref().map_or(Some(0), |bytes| u64::try_from(bytes.len()).ok());
        let copied = length.and_then(|length| self.bytes_copied.checked_add(length));
        let Some(copied) = copied.filter(|copied| *copied <= self.bounds.max_bytes) else {
            self.is_exhausted = true;
            return Err(read_session_error("prolly-read-session-byte-bound", "the read session byte bound is reached"));
        };
        self.blocks_read = self.blocks_read.saturating_add(1);
        self.bytes_copied = copied;
        Ok(bytes)
    }

    fn close(&mut self) -> ReadSessionObservation {
        self.store = None;
        ReadSessionObservation {
            blocks_read: self.blocks_read,
            bytes_copied: self.bytes_copied,
            transactions_opened: 0,
            end: if self.is_exhausted {
                ReadSessionEnd::BoundExhausted
            } else {
                ReadSessionEnd::Closed
            },
        }
    }
}

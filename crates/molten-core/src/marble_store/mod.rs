//! Pure decisions of the marble-plus-art object store spike.
//!
//! The spike stores immutable content-addressed Prolly blocks in a marble heap
//! and indexes each fixed 32-byte BLAKE3 digest to a marble `ObjectId` with an
//! adaptive radix trie. BLAKE3 stays the only content identity: an `ObjectId`
//! is a physical handle private to the backend and never names content. This
//! core owns object-id allocation by kind, the block frame and catalog
//! encodings, in-flight batch visibility, index recovery after a crash, the
//! maintenance schedule, a bounded read cache, and the claim boundary of the
//! spike decision record. Files, threads, clocks, and the marble and art
//! crates stay in the shell.

#![allow(
    tigerstyle::concrete_construction_in_core,
    reason = "the pure core constructs bounded plans, caches, and local collections, never infrastructure adapters"
)]

mod allocation;
mod batch;
mod cache;
mod catalog;
mod decision;
mod frame;
mod recovery;

pub use allocation::*;
pub use batch::*;
pub use cache::*;
pub use catalog::*;
pub use decision::*;
pub use frame::*;
pub use recovery::*;

#[cfg(test)]
mod tests;

/// A BLAKE3 content digest: the only content identity of a stored block.
pub type Digest = [u8; 32];

/// The byte length of a digest, for bounded arithmetic.
const DIGEST_BYTES: usize = 32;

/// The kinds of object the backend allocates identifiers for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ObjectKind {
    /// The one catalog object: published heads, pins, and the retention generation.
    Catalog,
    /// An immutable content-addressed block.
    Block,
}

/// Why a store profile, plan, frame, or recovery is rejected.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum MarbleStoreIssue {
    EmptyRange(ObjectKind),
    OverlappingRanges,
    /// The block range has no identifier left for the batch: fail closed.
    ObjectIdExhausted,
    /// An identifier outside every declared range was observed.
    ForeignObjectId(u64),
    ZeroMaintenanceInterval,
    ExecutorQueueOutOfBounds,
    ReadCacheOutOfBounds,
    ObjectBoundOutOfRange,
    /// A node reference is not `blake3:` followed by 64 lowercase hex digits.
    InvalidNodeRef,
    /// One request stages two different payloads under one digest.
    DigestCollision,
    PayloadTooLarge,
    TruncatedFrame,
    UnknownFrameVersion(u8),
    InvalidCatalog(&'static str),
    /// Two recovered objects carry one digest with different payloads.
    RecoveredDigestConflict,
}

/// The most entries the executor queue may hold.
pub const MAX_EXECUTOR_QUEUE_DEPTH: u32 = 1_024;
/// The most entries the bounded read cache may hold.
pub const MAX_READ_CACHE_ENTRIES: u32 = 65_536;
/// The largest object the spike stores: one frame header plus one block.
pub const MAX_OBJECT_BYTES: u32 = 16 * 1_024 * 1_024;

/// When the backend runs marble maintenance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MaintenanceSchedule {
    /// Run maintenance after this many committed write batches.
    pub every_committed_batches: u32,
}

impl MaintenanceSchedule {
    /// Whether maintenance is due after `committed_since_last` batches.
    pub const fn is_due(&self, committed_since_last: u32) -> bool {
        self.every_committed_batches != 0 && committed_since_last >= self.every_committed_batches
    }
}

/// The reviewed configuration of one marble-plus-art store.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MarbleStoreProfile {
    pub layout: ObjectIdLayout,
    pub maintenance: MaintenanceSchedule,
    /// Requests the bounded executor queues before it refuses more.
    pub executor_queue_depth: u32,
    pub read_cache_entries: u32,
    /// The largest frame the store writes or accepts on read.
    pub max_object_bytes: u32,
}

/// Validate a store profile before any storage is opened.
// r[impl aspen.marble_store.spike]
pub fn validate_store_profile(profile: &MarbleStoreProfile) -> Vec<MarbleStoreIssue> {
    let mut issues = validate_layout(&profile.layout);
    if profile.maintenance.every_committed_batches == 0 {
        issues.push(MarbleStoreIssue::ZeroMaintenanceInterval);
    }
    if profile.executor_queue_depth == 0 || profile.executor_queue_depth > MAX_EXECUTOR_QUEUE_DEPTH {
        issues.push(MarbleStoreIssue::ExecutorQueueOutOfBounds);
    }
    if profile.read_cache_entries > MAX_READ_CACHE_ENTRIES {
        issues.push(MarbleStoreIssue::ReadCacheOutOfBounds);
    }
    if profile.max_object_bytes <= FRAME_HEADER_BYTES || profile.max_object_bytes > MAX_OBJECT_BYTES {
        issues.push(MarbleStoreIssue::ObjectBoundOutOfRange);
    }
    issues
}

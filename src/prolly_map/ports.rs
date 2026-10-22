use molten_core::prolly_map::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublishedProllyRoot {
    pub root: ProllyRoot,
    pub generation: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpectedProllyRoot {
    pub root_ref: Option<RootRef>,
    pub generation: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProllyPublicationObservation {
    Applied,
    AlreadyApplied,
    Stale,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProllyPortError {
    pub code: &'static str,
    pub detail: String,
    pub outcome_unknown: bool,
}

impl ProllyPortError {
    pub fn new(code: &'static str, detail: impl Into<String>, outcome_unknown: bool) -> Self {
        Self {
            code,
            detail: detail.into(),
            outcome_unknown,
        }
    }
}

pub type ProllyPortResult<T> = std::result::Result<T, ProllyPortError>;

/// The retention inventory a deletion admission was built against.
///
/// The inventory is the durable state the admission observed: the published
/// roots, the pinned blocks, and the retention generation. A deletion that
/// finds a different inventory is refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProllyRetentionInventory {
    pub roots: Vec<NodeRef>,
    pub pins: Vec<NodeRef>,
    pub retention_generation: u64,
}

/// The outcome of a compare-and-delete operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GcDeletionOutcome {
    /// The inventory matched and the candidates were deleted.
    Deleted { deleted: Vec<NodeRef> },
    /// The inventory did not match, so nothing was deleted.
    InventoryMismatch { reason: &'static str },
}

impl GcDeletionOutcome {
    pub fn is_deleted(&self) -> bool {
        matches!(self, Self::Deleted { .. })
    }
}

// r[impl molten.prolly_map.storage_boundary]
// r[impl molten.prolly.gc.currentness]
pub trait ProllyBlockStorePort {
    fn read_block(&self, node_ref: &NodeRef) -> ProllyPortResult<Option<Vec<u8>>>;

    fn stage_blocks(&mut self, blocks: &[EncodedBlock]) -> ProllyPortResult<()>;

    fn read_root(&self, map_id: &str) -> ProllyPortResult<Option<PublishedProllyRoot>>;

    fn compare_and_advance(
        &mut self,
        map_id: &str,
        expected: &ExpectedProllyRoot,
        next: &PublishedProllyRoot,
    ) -> ProllyPortResult<ProllyPublicationObservation>;
}

// r[impl molten.prolly.gc.currentness]
pub trait ProllyRetentionPort {
    /// Pin blocks so a garbage collection cannot delete them.
    fn pin_blocks(&mut self, node_refs: &[NodeRef]) -> ProllyPortResult<()>;

    /// Release pins the caller no longer needs.
    fn release_pins(&mut self, node_refs: &[NodeRef]) -> ProllyPortResult<()>;

    /// The current retention inventory of the store.
    fn retention_inventory(&self) -> ProllyPortResult<ProllyRetentionInventory>;

    /// Compare the current retention inventory with the admission inventory
    /// and delete the candidates in one indivisible step.
    fn compare_retention_inventory_and_delete(
        &mut self,
        expected: &ProllyRetentionInventory,
        candidates: &[NodeRef],
    ) -> ProllyPortResult<GcDeletionOutcome>;
}

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

// r[impl molten.prolly_map.storage_boundary]
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

    fn delete_blocks(&mut self, node_refs: &[NodeRef]) -> ProllyPortResult<()>;
}

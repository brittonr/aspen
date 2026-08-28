use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MapEdit {
    Insert(SemanticEntry),
    Update(SemanticEntry),
    Delete(Vec<u8>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditPlan {
    pub profile_ref: ProfileRef,
    pub prior_root_ref: RootRef,
    pub next: MapBuild,
    pub staged_blocks: Vec<EncodedBlock>,
    pub reused_block_count: u32,
    pub edit_count: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffKind {
    Added,
    Removed,
    Modified,
}

impl DiffKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Added => "added",
            Self::Removed => "removed",
            Self::Modified => "modified",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffRecord {
    pub kind: DiffKind,
    pub key: Vec<u8>,
    pub before: Option<Vec<u8>>,
    pub after: Option<Vec<u8>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapDiff {
    pub left_root_ref: RootRef,
    pub right_root_ref: RootRef,
    pub records: Vec<DiffRecord>,
    pub skipped_equal_nodes: u32,
    pub complete: bool,
    pub selects_merge_winner: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapReadResult {
    pub root_ref: RootRef,
    pub entries: Vec<SemanticEntry>,
    pub closure: Vec<NodeRef>,
    pub graph_facts: Vec<GraphFact>,
    pub visited_nodes: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphFact {
    pub node_ref: NodeRef,
    pub children: Vec<NodeRef>,
    pub complete: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GcPlan {
    pub profile_ref: ProfileRef,
    pub roots: Vec<NodeRef>,
    pub pins: Vec<NodeRef>,
    pub reachable: Vec<NodeRef>,
    pub candidate_unreachable: Vec<NodeRef>,
    pub complete: bool,
    pub deletion_authorized: bool,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DifferentialDecision {
    Agreement,
    Divergence,
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProllyDifferentialEvidence {
    pub map_root_ref: RootRef,
    pub oracle_observation_ref: String,
    pub decision: DifferentialDecision,
    pub first_divergence: Option<String>,
    pub cross_format_root_equality_required: bool,
    pub proves_correctness: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProllyBenchmarkResult {
    pub profile_ref: ProfileRef,
    pub entry_count: u32,
    pub logical_bytes: u64,
    pub block_count: u32,
    pub block_bytes: u64,
    pub reused_blocks: u32,
    pub diff_records: u32,
    pub skipped_equal_nodes: u32,
    pub gc_candidates: u32,
    pub restart_verified: bool,
    pub timing_proves_correctness: bool,
}

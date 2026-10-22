use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MapEdit {
    Insert(SemanticEntry),
    Update(SemanticEntry),
    Delete(Vec<u8>),
}

/// Operation-level storage work counters.
///
/// The counters measure work an operation performed: blocks read, blocks
/// decoded, subtrees the operation did not traverse, read transactions opened,
/// and block bytes copied. They are behavioral facts, distinct from structural
/// sharing such as the count of node identities two snapshots share.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ProllyWorkCounters {
    pub nodes_read: u32,
    pub nodes_decoded: u32,
    pub subtrees_not_traversed: u32,
    pub transactions_opened: u32,
    pub bytes_copied: u64,
}

impl ProllyWorkCounters {
    /// The counters of two operations combined.
    pub const fn combined(self, other: Self) -> Self {
        Self {
            nodes_read: self.nodes_read.saturating_add(other.nodes_read),
            nodes_decoded: self.nodes_decoded.saturating_add(other.nodes_decoded),
            subtrees_not_traversed: self.subtrees_not_traversed.saturating_add(other.subtrees_not_traversed),
            transactions_opened: self.transactions_opened.saturating_add(other.transactions_opened),
            bytes_copied: self.bytes_copied.saturating_add(other.bytes_copied),
        }
    }
}

/// How an edit plan built its next map.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditStrategy {
    /// Every block of the next map was rebuilt from the full entry sequence.
    FullRebuild,
    /// Only the affected leaf region was rechunked; the unchanged leaves were
    /// reused and the internal levels were rebuilt over the leaf ranges.
    Incremental,
    /// The incremental planner reached its work bound and completed through
    /// the full rebuild.
    RebuildFallback,
}

impl EditStrategy {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::FullRebuild => "full-rebuild",
            Self::Incremental => "incremental",
            Self::RebuildFallback => "rebuild-fallback",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditPlan {
    pub profile_ref: ProfileRef,
    pub prior_root_ref: RootRef,
    pub next: MapBuild,
    pub staged_blocks: Vec<EncodedBlock>,
    pub reused_block_count: u32,
    pub edit_count: u32,
    /// How the plan built the next map.
    pub strategy: EditStrategy,
    /// The leaf blocks the plan encoded for the next map.
    pub leaves_encoded: u32,
    /// The work the plan performed, including the prior snapshot validation.
    pub work: ProllyWorkCounters,
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
    /// The node identities both validated closures share. A structural fact:
    /// it says nothing about the work the comparison avoided.
    pub shared_node_count: u32,
    /// The work of validating both snapshots under the existing contracts.
    pub validation: ProllyWorkCounters,
    /// The work of the comparison after validation.
    pub comparison: ProllyWorkCounters,
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
    /// The leaves of the snapshot in key order, as spans of `entries`.
    pub leaf_spans: Vec<LeafSpan>,
    /// The work the read performed.
    pub work: ProllyWorkCounters,
}

/// One leaf of a validated snapshot, as a span of its flattened entries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeafSpan {
    pub node_ref: NodeRef,
    pub start: usize,
    pub len: usize,
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
    pub shared_node_count: u32,
    pub diff_subtrees_not_traversed: u32,
    pub diff_comparison_nodes_decoded: u32,
    pub gc_candidates: u32,
    pub restart_verified: bool,
    pub timing_proves_correctness: bool,
}

/// The bounds of one block read session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReadSessionBounds {
    /// The most blocks the session may read.
    pub max_blocks: u32,
    /// The most block bytes the session may copy.
    pub max_bytes: u64,
}

/// How a read session ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadSessionEnd {
    /// The caller closed the session.
    Closed,
    /// A request exceeded the session bounds, and the session refused it and
    /// every later request.
    BoundExhausted,
}

/// The closing observation of a read session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReadSessionObservation {
    pub blocks_read: u32,
    pub bytes_copied: u64,
    pub transactions_opened: u32,
    pub end: ReadSessionEnd,
}

//! `Embedded`-aware bounded structural traversal.
//!
//! The walker visits a value tree in preorder under explicit node and depth budgets and reports the
//! first record label, symbol, string, byte string, or content reference a predicate selects.
//! Record labels, fields, sequence and set items, dictionary keys and values, and the payload of
//! every `Embedded` value are all children, so a wrapped marker or reference cannot evade a scan.
//! The walk keeps an explicit stack of open containers instead of recursing.
//!
//! The walker is generic over `TraversalNode`, implemented here for `&PackedValue` and by the
//! `molten` shell for `preserves::IOValue`, so both trees share one traversal law.

pub mod catalog;
mod packed;
mod walk;

pub use walk::find_match;

pub const DEFAULT_MAX_NODES: u64 = 8_192;
pub const DEFAULT_MAX_DEPTH: u64 = 128;

/// The class of token a predicate is asked about.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TokenKind {
    RecordLabel,
    Symbol,
    String,
    /// A byte string, presented as the content reference of its bytes.
    ByteString,
    /// A string that parses as a canonical content reference.
    ContentRef,
}

/// Which token classes a traversal offers to its predicate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TokenScope {
    pub record_labels: bool,
    pub symbols: bool,
    pub strings: bool,
    pub byte_strings: bool,
    pub content_refs: bool,
}

impl TokenScope {
    pub const STRUCTURAL_MARKERS: Self = Self {
        record_labels: true,
        symbols: true,
        strings: false,
        byte_strings: false,
        content_refs: false,
    };
    pub const CONTENT_REFS: Self = Self {
        record_labels: false,
        symbols: false,
        strings: false,
        byte_strings: false,
        content_refs: true,
    };
    pub const ALL: Self = Self {
        record_labels: true,
        symbols: true,
        strings: true,
        byte_strings: true,
        content_refs: true,
    };
}

/// Node and depth budgets; the root is at depth 1.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TraversalLimits {
    pub max_nodes: u64,
    pub max_depth: u64,
}

impl TraversalLimits {
    pub const DEFAULT: Self = Self {
        max_nodes: DEFAULT_MAX_NODES,
        max_depth: DEFAULT_MAX_DEPTH,
    };
}

/// One step of the path from the root to a match.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PathSegment {
    Root,
    Label,
    Field(u64),
    Item(u64),
    EntryKey(u64),
    EntryValue(u64),
    Embedded,
}

impl core::fmt::Display for PathSegment {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Root => formatter.write_str("$"),
            Self::Label => formatter.write_str("label"),
            Self::Field(index) => write!(formatter, "field[{index}]"),
            Self::Item(index) => write!(formatter, "item[{index}]"),
            Self::EntryKey(index) => write!(formatter, "entry[{index}].key"),
            Self::EntryValue(index) => write!(formatter, "entry[{index}].value"),
            Self::Embedded => formatter.write_str("embedded"),
        }
    }
}

/// The first token a traversal selected, with its path from the root.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TokenMatch {
    pub kind: TokenKind,
    pub token: alloc::string::String,
    pub path: alloc::vec::Vec<PathSegment>,
}

/// A traversal budget was exhausted before the traversal finished.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TraversalError {
    NodeCountOverflow,
    NodesExceeded { limit: u64 },
    DepthExceeded { limit: u64 },
}

impl core::fmt::Display for TraversalError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NodeCountOverflow => formatter.write_str("structural Preserves scan node count overflow"),
            Self::NodesExceeded { limit } => write!(formatter, "structural Preserves scan exceeded {limit} nodes"),
            Self::DepthExceeded { limit } => write!(formatter, "structural Preserves scan exceeded depth {limit}"),
        }
    }
}

/// What the walker needs to know about one node.
pub enum NodeShape<'a> {
    /// A record, with its label when the label is a symbol.
    Record {
        symbol_label: Option<alloc::borrow::Cow<'a, str>>,
    },
    Symbol(alloc::borrow::Cow<'a, str>),
    String(alloc::borrow::Cow<'a, str>),
    ByteString(alloc::borrow::Cow<'a, [u8]>),
    /// Any other value; its children (if any) still come from `TraversalNode::children`.
    Other,
}

/// A value tree the walker can traverse.
pub trait TraversalNode: Sized {
    fn shape(&self) -> NodeShape<'_>;

    /// The node's children in preorder, at most `budget` of them: a record's label then fields;
    /// sequence or set items; each dictionary key followed by its value; an `Embedded` payload.
    fn children(&self, budget: u64) -> alloc::vec::Vec<(PathSegment, Self)>;
}

/// Collects at most `budget` items without materializing the rest.
pub fn within_budget<I: Iterator>(items: I, budget: u64) -> alloc::vec::Vec<I::Item> {
    items.zip(0..budget).map(|(item, _)| item).collect()
}

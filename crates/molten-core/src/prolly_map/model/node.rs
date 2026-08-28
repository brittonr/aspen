use super::ProfileRef;

pub const PROLLY_LEAF_SCHEMA: &str = "molten.prolly-leaf-node.v1";
pub const PROLLY_INTERNAL_SCHEMA: &str = "molten.prolly-internal-node.v1";
pub const PROLLY_ROOT_SCHEMA: &str = "molten.prolly-root.v1";

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct NodeRef(String);

impl NodeRef {
    pub fn new(value: String) -> Self {
        Self(value)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RootRef(String);

impl RootRef {
    pub fn new(value: String) -> Self {
        Self(value)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SemanticEntry {
    pub key: Vec<u8>,
    pub value: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeafNode {
    pub schema: String,
    pub profile_ref: ProfileRef,
    pub entries: Vec<SemanticEntry>,
    pub encoded_len: u32,
    pub node_ref: NodeRef,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChildRange {
    pub min_key: Vec<u8>,
    pub max_key: Vec<u8>,
    pub node_ref: NodeRef,
    pub encoded_len: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InternalNode {
    pub schema: String,
    pub profile_ref: ProfileRef,
    pub children: Vec<ChildRange>,
    pub encoded_len: u32,
    pub node_ref: NodeRef,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProllyNode {
    Leaf(LeafNode),
    Internal(InternalNode),
}

impl ProllyNode {
    pub fn node_ref(&self) -> &NodeRef {
        match self {
            Self::Leaf(node) => &node.node_ref,
            Self::Internal(node) => &node.node_ref,
        }
    }

    pub fn encoded_len(&self) -> u32 {
        match self {
            Self::Leaf(node) => node.encoded_len,
            Self::Internal(node) => node.encoded_len,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncodedBlock {
    pub node_ref: NodeRef,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProllyRoot {
    pub schema: String,
    pub profile_ref: ProfileRef,
    pub top_node_ref: NodeRef,
    pub height: u16,
    pub entry_count: u32,
    pub root_ref: RootRef,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapSnapshot {
    pub root: ProllyRoot,
    pub blocks: Vec<EncodedBlock>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapBuild {
    pub snapshot: MapSnapshot,
    pub logical_bytes: u64,
    pub block_bytes: u64,
}

pub const PROLLY_PROFILE_SCHEMA: &str = "molten.prolly-map-profile.v1";
pub const PROLLY_MAP_FORMAT: &str = "prolly-semantic-map-v1";
pub const PROLLY_KEY_CODEC: &str = "canonical-bytes-v1";
pub const PROLLY_VALUE_CODEC: &str = "canonical-bytes-v1";
pub const PROLLY_COMPARISON: &str = "unsigned-lexicographic-v1";
pub const PROLLY_NODE_CODEC: &str = "molten-prolly-node-binary-v1";
pub const PROLLY_SIZE_ACCOUNTING: &str = "exact-encoded-bytes-v1";
pub const PROLLY_PROFILE_DOMAIN: &str = "molten-prolly-profile:v1";
pub const PROLLY_LEAF_DOMAIN: &str = "molten-prolly-leaf:v1";
pub const PROLLY_INTERNAL_DOMAIN: &str = "molten-prolly-internal:v1";
pub const PROLLY_ROOT_DOMAIN: &str = "molten-prolly-root:v1";
pub const PROLLY_BOUNDARY_DOMAIN: &str = "molten-prolly-boundary:v1";
pub const PROLLY_BOUNDARY_SEED_REF: &str = "blake3:4da8a4af528826c160a956f28ad0b8cce119c2636ea8a94b48b99e6596805261";
pub const PROLLY_FORMAT_VERSION: u32 = 1;

pub const MIN_NODE_BYTES: u32 = 256;
pub const TARGET_NODE_BYTES: u32 = 1_024;
pub const MAX_NODE_BYTES: u32 = 4_096;
pub const MIN_FANOUT: u16 = 2;
pub const TARGET_FANOUT: u16 = 4;
pub const MAX_FANOUT: u16 = 8;
pub const MAX_KEY_BYTES: u32 = 64;
pub const MAX_VALUE_BYTES: u32 = 1_024;
pub const MAX_ENTRIES: u32 = 4_096;
pub const MAX_TREE_HEIGHT: u16 = 16;
pub const MAX_DIFF_RECORDS: u32 = 8_192;
pub const MAX_GRAPH_FACTS: u32 = 16_384;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ProfileRef(String);

impl ProfileRef {
    pub fn new(value: String) -> Self {
        Self(value)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProllyLimits {
    pub max_key_bytes: u32,
    pub max_value_bytes: u32,
    pub max_entries: u32,
    pub max_tree_height: u16,
    pub max_diff_records: u32,
    pub max_graph_facts: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProllyProfile {
    pub schema: String,
    pub format: String,
    pub format_version: u32,
    pub key_codec: String,
    pub value_codec: String,
    pub comparison: String,
    pub node_codec: String,
    pub boundary_domain: String,
    pub boundary_seed_ref: String,
    pub size_accounting: String,
    pub min_node_bytes: u32,
    pub target_node_bytes: u32,
    pub max_node_bytes: u32,
    pub min_fanout: u16,
    pub target_fanout: u16,
    pub max_fanout: u16,
    pub profile_domain: String,
    pub leaf_domain: String,
    pub internal_domain: String,
    pub root_domain: String,
    pub limits: ProllyLimits,
    pub profile_ref: ProfileRef,
}

use super::*;

pub const MAX_DAG_NODES: usize = 4_096;
pub const MAX_DAG_EDGES: usize = 16_384;
pub const MAX_DAG_ROOTS: usize = 64;
pub const MAX_DAG_DEPTH: usize = 256;
pub const MAX_DAG_BYTES: u64 = 1_073_741_824;
pub const MAX_DAG_STEPS: usize = 65_536;
pub const MAX_DAG_PEERS: usize = 64;
pub const MAX_DAG_DOMAIN_BYTES: usize = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DagEdgeKind {
    Child,
    Dependency,
    Reference,
}

impl DagEdgeKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Child => "child",
            Self::Dependency => "dependency",
            Self::Reference => "reference",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct DagEdge {
    pub kind: DagEdgeKind,
    pub target: DagNodeRef,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DagNode {
    pub node_ref: DagNodeRef,
    pub schema_ref: DagSchemaRef,
    pub payload_ref: Option<DagContentRef>,
    pub encoded_bytes: u64,
    pub edges: Vec<DagEdge>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DagRoot {
    pub root_ref: DagRootRef,
    pub domain: String,
    pub node_ref: DagNodeRef,
    pub schema_ref: DagSchemaRef,
}

impl DagRoot {
    pub fn validate_domain(&self) -> Result<(), DagReferenceError> {
        if self.domain.is_empty()
            || self.domain.len() > MAX_DAG_DOMAIN_BYTES
            || self.domain.chars().any(char::is_control)
        {
            return Err(DagReferenceError::InvalidDomain);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DagGraph {
    pub roots: Vec<DagRoot>,
    pub nodes: Vec<DagNode>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DagBounds {
    pub max_nodes: usize,
    pub max_edges: usize,
    pub max_roots: usize,
    pub max_depth: usize,
    pub max_bytes: u64,
    pub max_steps: usize,
    pub max_peers: usize,
}

impl Default for DagBounds {
    fn default() -> Self {
        Self {
            max_nodes: MAX_DAG_NODES,
            max_edges: MAX_DAG_EDGES,
            max_roots: MAX_DAG_ROOTS,
            max_depth: MAX_DAG_DEPTH,
            max_bytes: MAX_DAG_BYTES,
            max_steps: MAX_DAG_STEPS,
            max_peers: MAX_DAG_PEERS,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DagSyncStrategy {
    Full,
    StemFirst,
    LeafOnly,
    Resumable,
    PeerPartitioned,
}

impl DagSyncStrategy {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Full => "full",
            Self::StemFirst => "stem-first",
            Self::LeafOnly => "leaf-only",
            Self::Resumable => "resumable",
            Self::PeerPartitioned => "peer-partitioned",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum DagObjectRef {
    Node(DagNodeRef),
    Content(DagContentRef),
}

impl DagObjectRef {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Node(reference) => reference.as_str(),
            Self::Content(reference) => reference.as_str(),
        }
    }

    pub const fn kind(&self) -> &'static str {
        match self {
            Self::Node(_) => "node",
            Self::Content(_) => "content",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DagInventory {
    pub available: Vec<DagObjectRef>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DagSyncProgress {
    pub epoch_ref: DagEpochRef,
    pub generation: u64,
    pub strategy: DagSyncStrategy,
    pub policy_ref: DagPolicyRef,
    pub root_refs: Vec<DagRootRef>,
    pub schema_refs: Vec<DagSchemaRef>,
    pub peers: Vec<DagPeerId>,
    pub verified: Vec<DagObjectRef>,
    pub steps_completed: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DagSyncRequest {
    pub root_refs: Vec<DagRootRef>,
    pub strategy: DagSyncStrategy,
    pub inventory: DagInventory,
    pub progress: Option<DagSyncProgress>,
    pub peers: Vec<DagPeerId>,
    pub epoch_ref: DagEpochRef,
    pub generation: u64,
    pub policy_ref: DagPolicyRef,
    pub bounds: DagBounds,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DagFetchRequest {
    pub object_ref: DagObjectRef,
    pub assigned_peer: Option<DagPeerId>,
    pub sequence: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DagSyncPlan {
    pub plan_ref: DagPlanRef,
    pub epoch_ref: DagEpochRef,
    pub generation: u64,
    pub strategy: DagSyncStrategy,
    pub roots: Vec<DagRootRef>,
    pub schema_refs: Vec<DagSchemaRef>,
    pub peers: Vec<DagPeerId>,
    pub topological_nodes: Vec<DagNodeRef>,
    pub missing: Vec<DagObjectRef>,
    pub requests: Vec<DagFetchRequest>,
    pub complete: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DagResponseObservation {
    pub epoch_ref: DagEpochRef,
    pub generation: u64,
    pub object_ref: DagObjectRef,
    pub assigned_peer: Option<DagPeerId>,
    pub identity_verified: bool,
    pub authorization_admitted: bool,
    pub encoded_bytes: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DagSyncDecision {
    Complete,
    Partial,
    Denied,
}

impl DagSyncDecision {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Complete => "complete",
            Self::Partial => "partial",
            Self::Denied => "denied",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DagSyncReceipt {
    pub decision: DagSyncDecision,
    pub plan_ref: Option<DagPlanRef>,
    pub epoch_ref: DagEpochRef,
    pub generation: u64,
    pub strategy: DagSyncStrategy,
    pub requested: usize,
    pub verified: usize,
    pub missing: Vec<DagObjectRef>,
    pub issues: Vec<DagSyncIssue>,
    pub non_claims: Vec<String>,
}

pub const DAG_SYNC_NON_CLAIMS: &[&str] = &[
    "verified receipt does not grant install or execution authority",
    "graph completion does not grant merge or publication authority",
    "content receipt does not prove provenance or application correctness",
    "peer assignment does not prove peer trust or availability",
    "local completion does not prove global convergence",
    "DAG synchronization does not own domain conflict semantics",
];

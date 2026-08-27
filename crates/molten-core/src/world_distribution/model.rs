use crate::dag_sync::DagObjectRef;
use crate::world_commit::RootKind;
use crate::world_commit::WorldCommitRef;
use crate::world_commit::WorldRootRef;

mod claims;
mod closure;
mod retention;

pub use claims::*;
pub use closure::*;
pub use retention::*;

pub const WORLD_DISTRIBUTION_PROFILE: &str = "molten-world-distribution-v1";
pub const WORLD_DAG_DOMAIN: &str = "molten-world-commit";
pub const WORLD_DISTRIBUTION_NON_CLAIMS: &[&str] = &[
    "closure completion does not grant activation authority",
    "claim authentication does not grant local branch authority",
    "replication does not select a winning head",
    "reachability does not grant retention or deletion authority",
    "remote lease evidence does not prove peer availability",
    "local completion does not prove permanent durability or global convergence",
];

pub const MAX_WORLD_DISTRIBUTION_OBJECTS: usize = crate::content_replication::MAX_CONTENTS;
pub const MAX_WORLD_DISTRIBUTION_BYTES: u64 = crate::content_replication::MAX_REPLICATION_BYTES;
pub const MAX_WORLD_DISTRIBUTION_CLAIMS: usize = crate::dag_sync::MAX_DAG_PEERS;
pub const MAX_WORLD_DISTRIBUTION_CLAIM_BYTES: u64 = crate::world_commit::MAX_WORLD_COMMIT_CANONICAL_BYTES as u64;
pub const MAX_WORLD_RETENTION_CLASSES: usize = 16;
pub const MAX_WORLD_RETENTION_ROOTS_PER_CLASS: usize = crate::content_replication::MAX_CONTENTS;
pub const MAX_WORLD_DISTRIBUTION_DIAGNOSTICS: usize = crate::content_replication::MAX_DIAGNOSTICS;
pub const MAX_WORLD_DISTRIBUTION_EVIDENCE_REFS: usize = crate::dag_sync::MAX_DAG_NODES;
pub const WORLD_RETENTION_BINDING_ROOT_CLASS: &str = "world-retention";
pub const WORLD_BINDING_IDENTIFIER_BYTES: usize = 256;
pub const MAX_WORLD_BINDING_EDGES: usize = crate::dag_sync::MAX_DAG_EDGES;
pub const MAX_WORLD_BINDING_PATH_NODES: usize = MAX_WORLD_DISTRIBUTION_OBJECTS;
pub const MAX_WORLD_BINDING_ROOT_CLASSES: usize = 1;
pub const MAX_WORLD_BINDING_ISSUES: usize = MAX_WORLD_DISTRIBUTION_DIAGNOSTICS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum WorldObjectDomain {
    Commit,
    Root(RootKind),
}

impl WorldObjectDomain {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Commit => "world-commit",
            Self::Root(kind) => kind.as_str(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum WorldObjectRef {
    Commit(WorldCommitRef),
    Root(WorldRootRef),
}

impl WorldObjectRef {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Commit(reference) => reference.as_str(),
            Self::Root(reference) => reference.as_str(),
        }
    }

    pub const fn domain(&self) -> WorldObjectDomain {
        match self {
            Self::Commit(_) => WorldObjectDomain::Commit,
            Self::Root(reference) => WorldObjectDomain::Root(reference.kind()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum WorldDistributionIssue {
    InvalidBounds(&'static str),
    ObjectLimitExceeded,
    ByteLimitExceeded,
    DuplicateCommit(String),
    DuplicateRoot(String),
    MissingRequestedCommit,
    MissingParent(String),
    MissingRoot(String),
    CommitIdentityMismatch(String),
    CommitCoreInvalid(String),
    NonCanonicalCommitCore(String),
    EmptyRootObject(String),
    DagReferenceInvalid(String),
    DagPlanningDenied(String),
    InventoryObjectUnknown(String),
    ReplicationManifestDrift,
    ReplicationPlanningDenied(String),
    ReplicationObjectUnsolicited(String),
    ClaimLimitExceeded,
    ClaimEnvelopeInvalid(String),
    ClaimConflictInvalid(String),
    RetentionClassLimitExceeded,
    DuplicateRetentionClass(String),
    MissingRetentionClass(String),
    RetentionRootLimitExceeded(String),
    RetentionRootUnknown(String),
    RetentionOwnerInvalid(String),
    RemoteLeaseInvalid(String),
    RetentionBindingDenied(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldActivationFacts {
    pub closure_complete: bool,
    pub domains_verified: bool,
    pub schemas_admitted: bool,
    pub current_policy_admitted: bool,
    pub current_authority_admitted: bool,
    pub claim_admitted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldActivationDecision {
    pub admitted: bool,
    pub diagnostics: Vec<String>,
    pub non_claims: Vec<String>,
}

pub fn distribution_non_claims() -> Vec<String> {
    WORLD_DISTRIBUTION_NON_CLAIMS.iter().map(ToString::to_string).collect()
}

pub fn dag_object_to_world(object: &DagObjectRef, descriptors: &[WorldObjectDescriptor]) -> Option<WorldObjectRef> {
    descriptors
        .iter()
        .find(|descriptor| descriptor.object_ref.as_str() == object.as_str())
        .map(|descriptor| descriptor.object_ref.clone())
}

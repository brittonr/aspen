use super::super::WorldObjectDomain;
use super::super::WorldObjectRef;
use crate::content_replication::Inventory;
use crate::content_replication::Manifest;
use crate::content_replication::Peer;
use crate::content_replication::Plan;
use crate::content_replication::PriorOperation;
use crate::dag_sync::DagBounds;
use crate::dag_sync::DagEpochRef;
use crate::dag_sync::DagGraph;
use crate::dag_sync::DagPeerId;
use crate::dag_sync::DagPolicyRef;
use crate::dag_sync::DagSchemaRef;
use crate::dag_sync::DagSyncPlan;
use crate::dag_sync::DagSyncProgress;
use crate::dag_sync::DagSyncRequest;
use crate::dag_sync::DagSyncStrategy;
use crate::world_commit::WorldCommitBounds;
use crate::world_commit::WorldCommitCore;
use crate::world_commit::WorldCommitRef;
use crate::world_commit::WorldRootRef;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldCommitObject {
    pub commit_ref: WorldCommitRef,
    pub core: WorldCommitCore,
    pub canonical_bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldRootObject {
    pub root: WorldRootRef,
    pub schema_ref: DagSchemaRef,
    pub encoded_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldObjectDescriptor {
    pub object_ref: WorldObjectRef,
    pub domain: WorldObjectDomain,
    pub schema_ref: DagSchemaRef,
    pub encoded_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldDagProjectionInput {
    pub requested: WorldCommitRef,
    pub commits: Vec<WorldCommitObject>,
    pub roots: Vec<WorldRootObject>,
    pub bounds: WorldCommitBounds,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldDagProjection {
    pub requested: WorldCommitRef,
    pub graph: DagGraph,
    pub objects: Vec<WorldObjectDescriptor>,
    pub total_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldSyncContext {
    pub inventory: Vec<WorldObjectRef>,
    pub progress: Option<DagSyncProgress>,
    pub peers: Vec<DagPeerId>,
    pub epoch_ref: DagEpochRef,
    pub generation: u64,
    pub policy_ref: DagPolicyRef,
    pub strategy: DagSyncStrategy,
    pub bounds: DagBounds,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldClosurePlan {
    pub projection: WorldDagProjection,
    pub request: DagSyncRequest,
    pub shared_plan: DagSyncPlan,
    pub missing: Vec<WorldObjectRef>,
    pub complete: bool,
    pub activation_authorized: bool,
    pub non_claims: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldReplicationProfile {
    pub service_id: String,
    pub generation: u64,
    pub membership_epoch: u64,
    pub placement_epoch: u64,
    pub authority_ref: String,
    pub identity_ref: String,
    pub content_profile_ref: String,
    pub transport_profile_ref: String,
    pub retention_policy_ref: String,
    pub evidence_profile_ref: String,
    pub desired_replicas: usize,
    pub minimum_verified_replicas: usize,
    pub minimum_fault_domains: usize,
    pub max_attempts: u32,
    pub max_concurrent_transfers: usize,
    pub max_transfer_bytes: u64,
    pub max_queue_depth: usize,
    pub max_timers: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldReplicationPlanRequest {
    pub profile: WorldReplicationProfile,
    pub inventory: Inventory,
    pub peers: Vec<Peer>,
    pub history: Vec<PriorOperation>,
    pub observed_tick: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldReplicationPlan {
    pub closure_ref: WorldCommitRef,
    pub manifest: Manifest,
    pub shared_plan: Plan,
    pub activation_authorized: bool,
    pub non_claims: Vec<String>,
}

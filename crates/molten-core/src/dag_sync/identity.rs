#![allow(
    tigerstyle::non_trait_imports,
    reason = "canonical identity uses ordered graph collections at each explicit framing boundary"
)]

use std::collections::BTreeMap;
use std::collections::BTreeSet;

use super::*;

const PLAN_IDENTITY_CONTEXT: &str = "onixresearch.molten.dag-sync.plan.identity.v1";

pub(super) fn assigned_peer(
    strategy: DagSyncStrategy,
    object_ref: &DagObjectRef,
    peers: &BTreeSet<DagPeerId>,
) -> Option<DagPeerId> {
    if strategy != DagSyncStrategy::PeerPartitioned || peers.is_empty() {
        return None;
    }
    let digest = blake3::hash(object_ref.as_str().as_bytes());
    let mut prefix = [0_u8; std::mem::size_of::<u64>()];
    prefix.copy_from_slice(&digest.as_bytes()[..std::mem::size_of::<u64>()]);
    let peer_count = u64::try_from(peers.len()).ok()?;
    assert!(peer_count != 0, "nonempty peer set must have a nonzero length");
    let index = usize::try_from(u64::from_be_bytes(prefix) % peer_count).ok()?;
    peers.iter().nth(index).cloned()
}

pub(super) fn progress_schema_refs(
    root_refs: &[DagRootRef],
    roots: &BTreeMap<DagRootRef, &DagRoot>,
    reachable: &BTreeSet<DagNodeRef>,
    nodes: &BTreeMap<DagNodeRef, &DagNode>,
) -> Result<Vec<DagSchemaRef>, DagSyncIssue> {
    let mut schema_refs = BTreeSet::new();
    for root_ref in root_refs {
        let root = roots.get(root_ref).ok_or(DagSyncIssue::UnknownRoot)?;
        schema_refs.insert(root.schema_ref.clone());
    }
    for node_ref in reachable {
        let node = nodes.get(node_ref).ok_or(DagSyncIssue::UnknownEdgeTarget)?;
        schema_refs.insert(node.schema_ref.clone());
    }
    Ok(schema_refs.into_iter().collect())
}

pub(super) struct PlanIdentityInput<'a> {
    pub request: &'a DagSyncRequest,
    pub roots: &'a [DagRootRef],
    pub schemas: &'a [DagSchemaRef],
    pub peers: &'a [DagPeerId],
    pub nodes: &'a [DagNodeRef],
    pub requests: &'a [DagFetchRequest],
}

pub(super) fn identify_plan(input: &PlanIdentityInput<'_>) -> Result<DagPlanRef, DagSyncIssue> {
    let mut hasher = blake3::Hasher::new_derive_key(PLAN_IDENTITY_CONTEXT);
    update(&mut hasher, input.request.epoch_ref.as_str())?;
    update(&mut hasher, &input.request.generation.to_string())?;
    update(&mut hasher, input.request.strategy.as_str())?;
    update(&mut hasher, input.request.policy_ref.as_str())?;
    for root in input.roots {
        update(&mut hasher, root.as_str())?;
    }
    for schema in input.schemas {
        update(&mut hasher, schema.as_str())?;
    }
    for peer in input.peers {
        update(&mut hasher, peer.as_str())?;
    }
    for node in input.nodes {
        update(&mut hasher, node.as_str())?;
    }
    for fetch in input.requests {
        update(&mut hasher, fetch.object_ref.kind())?;
        update(&mut hasher, fetch.object_ref.as_str())?;
        update(&mut hasher, fetch.assigned_peer.as_ref().map_or("", DagPeerId::as_str))?;
    }
    Ok(DagPlanRef::generated(hasher.finalize()))
}

fn update(hasher: &mut blake3::Hasher, value: &str) -> Result<(), DagSyncIssue> {
    let length = u64::try_from(value.len()).map_err(|_| DagSyncIssue::StepBoundExceeded)?;
    hasher.update(&length.to_be_bytes());
    hasher.update(value.as_bytes());
    Ok(())
}

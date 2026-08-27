#![allow(
    tigerstyle::borrowed_argument_types,
    reason = "local validators append to one preallocated bounded typed issue sink"
)]

use std::collections::BTreeSet;

use artifact_binding_core::ArtifactId;
use artifact_binding_core::GraphEdge;
use artifact_binding_core::GraphRoot;
use artifact_binding_core::ReachabilityLimits;
use artifact_binding_core::RetirementLimits;
use artifact_binding_core::RootClassId;
use artifact_binding_core::RootId;

use super::super::*;
use crate::world_commit::WorldCommitRef;

const WORLD_RETENTION_ROOT_CONTEXT: &str = "onixresearch.molten.world-distribution.retention-root.v1";

pub(super) fn validate_class_observation(
    observation: &WorldRetentionClassObservation,
    known: &BTreeSet<WorldObjectRef>,
    issues: &mut Vec<WorldDistributionIssue>,
) {
    if validate_content_ref(&observation.owner_ref, "owner").is_err() {
        issues.push(WorldDistributionIssue::RetentionOwnerInvalid(observation.owner_ref.clone()));
    }
    if observation.roots.len() > MAX_WORLD_RETENTION_ROOTS_PER_CLASS {
        issues.push(WorldDistributionIssue::RetentionRootLimitExceeded(observation.class.as_str().to_string()));
    }
    for root in &observation.roots {
        if !known.contains(root) {
            issues.push(WorldDistributionIssue::RetentionRootUnknown(root.as_str().to_string()));
        }
    }
    for evidence in &observation.evidence_refs {
        if validate_content_ref(evidence, "evidence").is_err() {
            issues.push(WorldDistributionIssue::RetentionOwnerInvalid(evidence.clone()));
        }
    }
}

pub(super) fn validate_remote_lease(
    lease: &WorldRemoteLeaseObservation,
    known: &BTreeSet<WorldObjectRef>,
    issues: &mut Vec<WorldDistributionIssue>,
) {
    let references = [
        lease.lease_ref.as_str(),
        lease.peer_ref.as_str(),
        lease.validity_basis_ref.as_str(),
        lease.evidence_ref.as_str(),
    ];
    if lease.generation == 0
        || references.into_iter().any(|reference| validate_content_ref(reference, "lease").is_err())
    {
        issues.push(WorldDistributionIssue::RemoteLeaseInvalid(lease.lease_ref.clone()));
    }
    if lease.roots.len() > MAX_WORLD_RETENTION_ROOTS_PER_CLASS {
        issues.push(WorldDistributionIssue::RetentionRootLimitExceeded("remote-lease".to_string()));
    }
    for root in &lease.roots {
        if !known.contains(root) {
            issues.push(WorldDistributionIssue::RetentionRootUnknown(root.as_str().to_string()));
        }
    }
}

pub(super) fn binding_root(
    class: WorldRetentionClass,
    owner_ref: &str,
    object: &WorldObjectRef,
) -> Result<GraphRoot, Vec<WorldDistributionIssue>> {
    let root_id = retention_root_id(class, owner_ref, object)?;
    Ok(GraphRoot {
        class: binding_root_class()?,
        id: RootId::try_new(&root_id, WORLD_BINDING_IDENTIFIER_BYTES).map_err(|error| {
            vec![WorldDistributionIssue::RetentionBindingDenied(format!(
                "root:{error:?}"
            ))]
        })?,
        target: artifact(object.as_str())?,
        generation_scope: None,
    })
}

pub(super) fn binding_edges(projection: &WorldDagProjection) -> Result<Vec<GraphEdge>, Vec<WorldDistributionIssue>> {
    let edge_capacity = bounded_sum(projection.graph.nodes.iter().map(|node| node.edges.len()))?;
    if edge_capacity > MAX_WORLD_BINDING_EDGES {
        return Err(vec![WorldDistributionIssue::ObjectLimitExceeded]);
    }
    let mut edges = Vec::with_capacity(edge_capacity);
    for node in &projection.graph.nodes {
        for edge in &node.edges {
            edges.push(GraphEdge {
                from: artifact(node.node_ref.as_str())?,
                to: artifact(edge.target.as_str())?,
            });
        }
    }
    edges.sort_by(|left, right| {
        left.from.as_str().cmp(right.from.as_str()).then_with(|| left.to.as_str().cmp(right.to.as_str()))
    });
    edges.dedup();
    Ok(edges)
}

pub(super) fn binding_root_class() -> Result<RootClassId, Vec<WorldDistributionIssue>> {
    RootClassId::try_new(WORLD_RETENTION_BINDING_ROOT_CLASS, WORLD_BINDING_IDENTIFIER_BYTES).map_err(|error| {
        vec![WorldDistributionIssue::RetentionBindingDenied(format!(
            "class:{error:?}"
        ))]
    })
}

pub(super) const fn binding_limits() -> RetirementLimits {
    RetirementLimits {
        reachability: ReachabilityLimits {
            max_roots: MAX_WORLD_DISTRIBUTION_OBJECTS,
            max_edges: MAX_WORLD_BINDING_EDGES,
            max_nodes: MAX_WORLD_DISTRIBUTION_OBJECTS,
            max_path_nodes: MAX_WORLD_BINDING_PATH_NODES,
            max_diagnostics: MAX_WORLD_DISTRIBUTION_DIAGNOSTICS,
        },
        max_attributions: MAX_WORLD_DISTRIBUTION_OBJECTS,
        max_root_classes: MAX_WORLD_BINDING_ROOT_CLASSES,
        max_issues: MAX_WORLD_BINDING_ISSUES,
    }
}

pub(super) fn artifact(value: &str) -> Result<ArtifactId, Vec<WorldDistributionIssue>> {
    ArtifactId::try_new(value, WORLD_BINDING_IDENTIFIER_BYTES).map_err(|error| {
        vec![WorldDistributionIssue::RetentionBindingDenied(format!(
            "artifact:{error:?}"
        ))]
    })
}

fn retention_root_id(
    class: WorldRetentionClass,
    owner_ref: &str,
    object: &WorldObjectRef,
) -> Result<String, Vec<WorldDistributionIssue>> {
    let mut hasher = blake3::Hasher::new_derive_key(WORLD_RETENTION_ROOT_CONTEXT);
    update(&mut hasher, class.as_str())?;
    update(&mut hasher, owner_ref)?;
    update(&mut hasher, object.as_str())?;
    Ok(format!("blake3:{}", hasher.finalize().to_hex()))
}

fn update(hasher: &mut blake3::Hasher, value: &str) -> Result<(), Vec<WorldDistributionIssue>> {
    let length = u64::try_from(value.len()).map_err(|_| vec![WorldDistributionIssue::ByteLimitExceeded])?;
    hasher.update(&length.to_be_bytes());
    hasher.update(value.as_bytes());
    Ok(())
}

pub(super) fn validate_content_ref(value: &str, field: &str) -> Result<(), Vec<WorldDistributionIssue>> {
    WorldCommitRef::new(value.to_string()).map(|_| ()).map_err(|error| {
        vec![WorldDistributionIssue::RetentionOwnerInvalid(format!(
            "{field}:{error:?}"
        ))]
    })
}

pub(super) fn bounded_sum(mut values: impl Iterator<Item = usize>) -> Result<usize, Vec<WorldDistributionIssue>> {
    values.try_fold(0_usize, |total, value| {
        total.checked_add(value).ok_or_else(|| vec![WorldDistributionIssue::ObjectLimitExceeded])
    })
}

pub(super) fn normalize_issues(mut issues: Vec<WorldDistributionIssue>) -> Vec<WorldDistributionIssue> {
    issues.sort();
    issues.dedup();
    issues
}

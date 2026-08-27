#![allow(
    tigerstyle::borrowed_argument_types,
    reason = "local validation helpers append to one preallocated bounded typed issue sink"
)]

use std::collections::BTreeMap;

use super::super::*;
use crate::dag_sync::DagNodeRef;
use crate::dag_sync::DagObjectRef;
use crate::dag_sync::DagRootRef;
use crate::dag_sync::DagSchemaRef;
use crate::world_commit::WORLD_COMMIT_SCHEMA;
use crate::world_commit::WorldCommitRef;
use crate::world_commit::WorldRootRef;
use crate::world_commit::identify_world_commit;
use crate::world_commit::validate_and_normalize_core;

const WORLD_COMMIT_SCHEMA_CONTEXT: &str = "onixresearch.molten.world-distribution.commit-schema.v1";

pub(super) fn validate_projection_bounds(input: &WorldDagProjectionInput, issues: &mut Vec<WorldDistributionIssue>) {
    let object_count = input.commits.len().saturating_add(input.roots.len());
    if object_count == 0 || object_count > MAX_WORLD_DISTRIBUTION_OBJECTS {
        issues.push(WorldDistributionIssue::ObjectLimitExceeded);
    }
    if input.bounds.max_closure_objects == 0 || input.bounds.max_closure_objects > MAX_WORLD_DISTRIBUTION_OBJECTS {
        issues.push(WorldDistributionIssue::InvalidBounds("max-closure-objects"));
    }
    if object_count > input.bounds.max_closure_objects {
        issues.push(WorldDistributionIssue::ObjectLimitExceeded);
    }
}

pub(super) fn validate_commit(
    commit: &WorldCommitObject,
    bounds: &crate::world_commit::WorldCommitBounds,
    issues: &mut Vec<WorldDistributionIssue>,
) {
    match identify_world_commit(&commit.canonical_bytes) {
        Ok(identity) if identity == commit.commit_ref => {}
        _ => issues.push(WorldDistributionIssue::CommitIdentityMismatch(commit.commit_ref.as_str().to_string())),
    }
    match validate_and_normalize_core(&commit.core, bounds) {
        Ok(normalized) if normalized == commit.core => {}
        Ok(_) => issues.push(WorldDistributionIssue::NonCanonicalCommitCore(commit.commit_ref.as_str().to_string())),
        Err(core_issues) => {
            issues.push(WorldDistributionIssue::CommitCoreInvalid(format!("{}:{core_issues:?}", commit.commit_ref)))
        }
    }
}

pub(super) fn validate_dependencies(
    commits: &BTreeMap<WorldCommitRef, &WorldCommitObject>,
    roots: &BTreeMap<WorldRootRef, &WorldRootObject>,
    issues: &mut Vec<WorldDistributionIssue>,
) {
    for commit in commits.values() {
        for parent in &commit.core.parents {
            if !commits.contains_key(parent) {
                issues.push(WorldDistributionIssue::MissingParent(parent.as_str().to_string()));
            }
        }
        for root in &commit.core.roots {
            if !roots.contains_key(root) {
                issues.push(WorldDistributionIssue::MissingRoot(root.as_str().to_string()));
            }
        }
    }
}

pub(super) fn add_bytes(current: u64, additional: u64) -> Result<u64, Vec<WorldDistributionIssue>> {
    let total = current.checked_add(additional).ok_or_else(|| vec![WorldDistributionIssue::ByteLimitExceeded])?;
    if total > MAX_WORLD_DISTRIBUTION_BYTES {
        return Err(vec![WorldDistributionIssue::ByteLimitExceeded]);
    }
    Ok(total)
}

pub(super) fn world_object_to_dag(object: &WorldObjectRef) -> Result<DagObjectRef, WorldDistributionIssue> {
    DagNodeRef::new(object.as_str().to_string())
        .map(DagObjectRef::Node)
        .map_err(|issue| WorldDistributionIssue::DagReferenceInvalid(format!("node:{issue:?}")))
}

pub(super) fn node_ref(value: &str) -> Result<DagNodeRef, Vec<WorldDistributionIssue>> {
    DagNodeRef::new(value.to_string())
        .map_err(|issue| vec![WorldDistributionIssue::DagReferenceInvalid(format!("node:{issue:?}"))])
}

pub(super) fn root_ref(value: &str) -> Result<DagRootRef, Vec<WorldDistributionIssue>> {
    DagRootRef::new(value.to_string())
        .map_err(|issue| vec![WorldDistributionIssue::DagReferenceInvalid(format!("root:{issue:?}"))])
}

pub(super) fn commit_schema_ref() -> Result<DagSchemaRef, WorldDistributionIssue> {
    let mut hasher = blake3::Hasher::new_derive_key(WORLD_COMMIT_SCHEMA_CONTEXT);
    let length = u64::try_from(WORLD_COMMIT_SCHEMA.len())
        .map_err(|_| WorldDistributionIssue::DagReferenceInvalid("schema-length".to_string()))?;
    hasher.update(&length.to_be_bytes());
    hasher.update(WORLD_COMMIT_SCHEMA.as_bytes());
    DagSchemaRef::new(format!("blake3:{}", hasher.finalize().to_hex()))
        .map_err(|issue| WorldDistributionIssue::DagReferenceInvalid(format!("schema:{issue:?}")))
}

pub(super) fn normalize_issues(mut issues: Vec<WorldDistributionIssue>) -> Vec<WorldDistributionIssue> {
    issues.sort();
    issues.dedup();
    issues
}

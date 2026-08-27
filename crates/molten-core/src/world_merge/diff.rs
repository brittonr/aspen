use std::collections::BTreeSet;

use super::WorldDiffReport;
use super::WorldMergeIssue;
use super::WorldMergeRequest;
use super::WorldRootDiff;
use super::WorldRootDiffClass;

pub fn diff_world_roots(request: &WorldMergeRequest) -> Result<WorldDiffReport, Vec<WorldMergeIssue>> {
    let maximum =
        usize::try_from(request.bounds.max_roots).map_err(|_| vec![WorldMergeIssue::InvalidBounds("max_roots")])?;
    if request.roots.len() > maximum {
        return Err(vec![WorldMergeIssue::RootLimitExceeded]);
    }
    let unique = request.roots.iter().map(|input| input.kind).collect::<BTreeSet<_>>();
    if unique.len() != request.roots.len() {
        return Err(vec![WorldMergeIssue::DuplicateRoot]);
    }
    let mut roots = request
        .roots
        .iter()
        .map(|input| WorldRootDiff {
            kind: input.kind,
            class: classify_root(input, request.profile.root_modes.contains_key(&input.kind)),
        })
        .collect::<Vec<_>>();
    roots.sort_by_key(|root| root.kind);
    let mut source_heads = request.source_heads.clone();
    source_heads.sort();
    Ok(WorldDiffReport {
        base_head: request.base_head.clone(),
        source_heads,
        roots,
    })
}

fn classify_root(input: &super::WorldMergeRootInput, is_profile_included: bool) -> WorldRootDiffClass {
    if !is_profile_included {
        return WorldRootDiffClass::ProfileExcluded;
    }
    if !input.base.available || !input.left.available || !input.right.available {
        return WorldRootDiffClass::Unavailable;
    }
    if input.base.root.is_none() || input.left.root.is_none() || input.right.root.is_none() {
        return WorldRootDiffClass::Absent;
    }
    if !schemas_compatible(input) {
        return WorldRootDiffClass::Incompatible;
    }
    if input.left.root == input.right.root {
        WorldRootDiffClass::Equal
    } else {
        WorldRootDiffClass::Changed
    }
}

fn schemas_compatible(input: &super::WorldMergeRootInput) -> bool {
    input.base.schema_ref == input.left.schema_ref && input.left.schema_ref == input.right.schema_ref
}

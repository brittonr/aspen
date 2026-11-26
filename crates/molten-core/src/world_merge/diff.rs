use alloc::collections::BTreeSet;

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
    let present = [&input.base, &input.left, &input.right].into_iter().filter(|value| value.root.is_some());
    if present.clone().next().is_none() {
        return WorldRootDiffClass::Absent;
    }
    // Creation and deletion are changes: a root absent from base, left, or right is compared
    // with the present sides rather than reported as wholly absent, and a side missing its root
    // is never reported as equal to the other side.
    // r[impl molten.world_commit.merge_conservative_precedence_and_full_graph_closure]
    if !schemas_compatible(present) {
        return WorldRootDiffClass::Incompatible;
    }
    if input.left.root.is_some() && input.left.root == input.right.root {
        WorldRootDiffClass::Equal
    } else {
        WorldRootDiffClass::Changed
    }
}

fn schemas_compatible<'a>(mut present: impl Iterator<Item = &'a super::WorldMergeValue>) -> bool {
    let Some(first) = present.next() else {
        return true;
    };
    present.all(|value| value.schema_ref == first.schema_ref)
}

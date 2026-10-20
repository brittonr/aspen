use std::collections::BTreeSet;

use super::WorldMergeConflict;
use super::WorldMergeHandler;
use super::WorldMergeIssue;
use super::WorldMergeMode;
use super::WorldMergePlan;
use super::WorldMergeRequest;
use super::WorldMergeRootInput;
use super::WorldMergedRoot;
use super::merge_keyed_values;
use super::planning::PlanIdentityInput;
use super::planning::add_application_handler;
use super::planning::identify_plan;
use crate::world_commit::RootKind;

const MINIMUM_SOURCE_HEADS: usize = 2;

pub fn plan_world_merge(
    mut request: WorldMergeRequest,
    materializations: Vec<super::MaterializedWorldMigration>,
    handler: Option<&dyn WorldMergeHandler>,
) -> Result<WorldMergePlan, Vec<WorldMergeIssue>> {
    let (original_ref, migrations) = super::migration::prepare_values(&mut request, materializations)?;
    let maximum_roots =
        usize::try_from(request.bounds.max_roots).map_err(|_| vec![WorldMergeIssue::InvalidBounds("max_roots")])?;
    let maximum_conflicts = usize::try_from(request.bounds.max_conflicts)
        .map_err(|_| vec![WorldMergeIssue::InvalidBounds("max_conflicts")])?;
    let (mut outputs, mut conflicts, mut reduction_issues) =
        reduce_roots(&request, handler, maximum_roots, maximum_conflicts);
    if !reduction_issues.is_empty() {
        reduction_issues.sort();
        reduction_issues.dedup();
        return Err(reduction_issues);
    }
    conflicts.sort_by(|left, right| {
        left.kind.cmp(&right.kind).then(left.key.cmp(&right.key)).then(left.code.cmp(right.code))
    });
    let mut source_heads = std::mem::take(&mut request.source_heads);
    source_heads.sort();
    super::migration::restore_generated_outputs(&mut request, &migrations, &mut outputs);
    let plan_ref = identify_plan(PlanIdentityInput {
        original_request: &original_ref,
        base_head: &request.base_head,
        source_heads: &source_heads,
        outputs: &outputs,
        conflicts: &conflicts,
        migrations: &migrations,
    })
    .map_err(|issue| vec![issue])?;
    Ok(WorldMergePlan {
        plan_ref,
        base_head: request.base_head,
        source_heads,
        outputs,
        conflicts,
        migrations,
        original_request_ref: original_ref,
    })
}

fn reduce_roots(
    request: &WorldMergeRequest,
    handler: Option<&dyn WorldMergeHandler>,
    maximum_roots: usize,
    maximum_conflicts: usize,
) -> (Vec<WorldMergedRoot>, Vec<WorldMergeConflict>, Vec<WorldMergeIssue>) {
    let mut roots = request.roots.iter().collect::<Vec<_>>();
    roots.sort_by_key(|root| root.kind);
    let mut outputs = Vec::with_capacity(maximum_roots.min(roots.len()));
    let mut conflicts = Vec::with_capacity(maximum_conflicts.min(roots.len()));
    let mut issues = Vec::with_capacity(roots.len());
    for root in &roots {
        let mut reduction = MergeReduction {
            outputs: &mut outputs,
            conflicts: &mut conflicts,
            issues: &mut issues,
        };
        reduce_root(request, root, handler, &mut reduction);
        if outputs.len() > maximum_roots {
            issues.push(WorldMergeIssue::RootLimitExceeded);
            break;
        }
        if conflicts.len() > maximum_conflicts {
            issues.push(WorldMergeIssue::ConflictLimitExceeded);
            break;
        }
    }
    (outputs, conflicts, issues)
}

struct MergeReduction<'a> {
    outputs: &'a mut Vec<WorldMergedRoot>,
    conflicts: &'a mut Vec<WorldMergeConflict>,
    issues: &'a mut Vec<WorldMergeIssue>,
}

fn reduce_root(
    request: &WorldMergeRequest,
    root: &WorldMergeRootInput,
    handler: Option<&dyn WorldMergeHandler>,
    reduction: &mut MergeReduction<'_>,
) {
    if root.left.root == root.right.root && root.left.schema_ref == root.right.schema_ref {
        reduction.outputs.push(selected_output(root, root.left.root.clone()));
        return;
    }
    if is_runtime_sensitive(root.kind) {
        reduction.issues.push(WorldMergeIssue::RuntimeSensitiveRoot(root.kind));
        return;
    }
    if let Err(issue) = validate_schema_admission(request, root) {
        reduction.issues.push(issue);
        return;
    }
    let Some(mode) = request.profile.root_modes.get(&root.kind).copied() else {
        reduction.issues.push(WorldMergeIssue::ModeNotDeclared(root.kind));
        return;
    };
    match mode {
        WorldMergeMode::IdenticalOnly => {
            add_identical(root, reduction.outputs, reduction.conflicts);
        }
        WorldMergeMode::AncestorReplacement => {
            add_ancestor_replacement(root, reduction.outputs, reduction.conflicts);
        }
        WorldMergeMode::KeyedDurableValues => {
            add_keyed(request, root, reduction.outputs, reduction.conflicts, reduction.issues);
        }
        WorldMergeMode::ApplicationHandler => {
            add_application_handler(request, root, handler, reduction.outputs, reduction.issues);
        }
    }
}

fn add_keyed(
    request: &WorldMergeRequest,
    root: &WorldMergeRootInput,
    outputs: &mut Vec<WorldMergedRoot>,
    conflicts: &mut Vec<WorldMergeConflict>,
    issues: &mut Vec<WorldMergeIssue>,
) {
    if root.kind != RootKind::DurableState {
        issues.push(WorldMergeIssue::RuntimeSensitiveRoot(root.kind));
        return;
    }
    match merge_keyed_values(
        root.kind,
        &root.base.keyed_values,
        &root.left.keyed_values,
        &root.right.keyed_values,
        &request.bounds,
    ) {
        Ok(mut result) => {
            if result.conflicts.is_empty() {
                outputs.push(WorldMergedRoot {
                    kind: root.kind,
                    selected_root: None,
                    generated_values: result.values,
                    generated_bytes: None,
                    output_schema: root.left.schema_ref.clone(),
                });
            }
            conflicts.append(&mut result.conflicts);
        }
        Err(issue) => issues.push(issue),
    }
}

pub(super) fn validate_request(request: &WorldMergeRequest) -> Vec<WorldMergeIssue> {
    let mut issues = Vec::with_capacity(8);
    if request.bounds.max_roots == 0 {
        issues.push(WorldMergeIssue::InvalidBounds("max_roots"));
    }
    if request.bounds.max_keys == 0 {
        issues.push(WorldMergeIssue::InvalidBounds("max_keys"));
    }
    if request.bounds.max_value_bytes == 0 {
        issues.push(WorldMergeIssue::InvalidBounds("max_value_bytes"));
    }
    if request.bounds.max_conflicts == 0 {
        issues.push(WorldMergeIssue::InvalidBounds("max_conflicts"));
    }
    if !request.common_ancestor_verified {
        issues.push(WorldMergeIssue::MissingBase);
    }
    if request.common_ancestor_ambiguous {
        issues.push(WorldMergeIssue::AmbiguousBase);
    }
    if request.source_heads.len() < MINIMUM_SOURCE_HEADS {
        issues.push(WorldMergeIssue::SourceCountInvalid);
    }
    let unique_sources = request.source_heads.iter().collect::<BTreeSet<_>>();
    if unique_sources.len() != request.source_heads.len() {
        issues.push(WorldMergeIssue::DuplicateSource);
    }
    let unique_roots = request.roots.iter().map(|root| root.kind).collect::<BTreeSet<_>>();
    if unique_roots.len() != request.roots.len() {
        issues.push(WorldMergeIssue::DuplicateRoot);
    }
    match usize::try_from(request.bounds.max_roots) {
        Ok(maximum_roots) if request.roots.len() > maximum_roots => {
            issues.push(WorldMergeIssue::RootLimitExceeded);
        }
        Ok(_) => {}
        Err(_) => issues.push(WorldMergeIssue::InvalidBounds("max_roots")),
    }
    issues
}

fn validate_schema_admission(_request: &WorldMergeRequest, root: &WorldMergeRootInput) -> Result<(), WorldMergeIssue> {
    let mut schema = None;
    for value in [&root.base, &root.left, &root.right] {
        if value.root.is_none() {
            continue;
        }
        if !value.available {
            return Err(WorldMergeIssue::UnavailableRoot(root.kind));
        }
        if let Some(expected) = schema {
            if value.schema_ref.as_ref() != expected {
                return Err(WorldMergeIssue::MigrationRequired(root.kind));
            }
        } else {
            schema = Some(value.schema_ref.as_ref());
        }
    }
    Ok(())
}

fn add_identical(
    root: &WorldMergeRootInput,
    outputs: &mut Vec<WorldMergedRoot>,
    conflicts: &mut Vec<WorldMergeConflict>,
) {
    if root.left.root == root.right.root {
        outputs.push(selected_output(root, root.left.root.clone()));
    } else {
        conflicts.push(root_conflict(root.kind, "identical-only-mismatch"));
    }
}

fn add_ancestor_replacement(
    root: &WorldMergeRootInput,
    outputs: &mut Vec<WorldMergedRoot>,
    conflicts: &mut Vec<WorldMergeConflict>,
) {
    if root.left.root == root.base.root {
        outputs.push(selected_output(root, root.right.root.clone()));
    } else if root.right.root == root.base.root {
        outputs.push(selected_output(root, root.left.root.clone()));
    } else {
        conflicts.push(root_conflict(root.kind, "concurrent-root-change"));
    }
}

fn selected_output(
    root: &WorldMergeRootInput,
    selected_root: Option<crate::world_commit::WorldRootRef>,
) -> WorldMergedRoot {
    let selected = [&root.left, &root.right, &root.base].into_iter().find(|value| value.root == selected_root);
    WorldMergedRoot {
        kind: root.kind,
        selected_root,
        generated_values: std::collections::BTreeMap::new(),
        generated_bytes: None,
        output_schema: selected.and_then(|value| value.schema_ref.clone()),
    }
}

fn root_conflict(kind: RootKind, code: &'static str) -> WorldMergeConflict {
    WorldMergeConflict { kind, key: None, code }
}

pub(super) fn is_runtime_sensitive(kind: RootKind) -> bool {
    matches!(
        kind,
        RootKind::Tasks
            | RootKind::Scheduler
            | RootKind::Effects
            | RootKind::Time
            | RootKind::Entropy
            | RootKind::AuthorityObservation
            | RootKind::OpaqueMachineSnapshot
    )
}

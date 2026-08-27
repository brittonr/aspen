use std::collections::BTreeMap;
use std::collections::BTreeSet;

use super::ClosureIssue;
use super::ClosureReport;
use super::ClosureRequest;
use super::RestoreIssue;
use super::RestorePlan;
use super::RestoreStep;
use super::RestoreStepKind;
use super::RootKind;
use super::RootReplayClassification;
use super::WorldCommitCore;
use super::WorldCommitRef;
use super::root_for_kind;

const TERMINAL_RESTORE_STEP_COUNT: usize = 2;
const WALK_EVENT_FACTOR: usize = 2;

// r[impl molten.world_commit.restore]
pub fn validate_closure(request: &ClosureRequest) -> ClosureReport {
    let mut issues = Vec::new();
    if !validate_closure_bound(request, &mut issues) {
        issues.sort();
        issues.dedup();
        return ClosureReport {
            commit_ref: request.commit_ref.clone(),
            complete: false,
            first_missing_root: None,
            issues,
        };
    }
    let core = match super::validate_and_normalize_core(&request.core, &request.bounds) {
        Ok(core) => core,
        Err(core_issues) => {
            issues.push(ClosureIssue::InvalidCore(format!("{core_issues:?}")));
            request.core.clone()
        }
    };
    let first_missing_root = validate_root_objects(request, &core, &mut issues);
    validate_parent_graph(request, &core, &mut issues);
    issues.sort();
    issues.dedup();
    ClosureReport {
        commit_ref: request.commit_ref.clone(),
        complete: issues.is_empty(),
        first_missing_root,
        issues,
    }
}

// r[impl molten.world_commit.restore]
pub fn plan_restore(
    commit_ref: &WorldCommitRef,
    core: &WorldCommitCore,
    closure: &ClosureReport,
) -> Result<RestorePlan, RestoreIssue> {
    if closure.commit_ref != *commit_ref {
        return Err(RestoreIssue::ClosureCommitMismatch);
    }
    if !closure.complete {
        return Err(RestoreIssue::IncompleteClosure(closure.issues.clone()));
    }
    let core =
        super::validate_and_normalize_core(core, &super::protocol_bounds()).map_err(|_| RestoreIssue::InvalidCore)?;
    let mut steps = Vec::with_capacity(core.roots.len().saturating_add(TERMINAL_RESTORE_STEP_COUNT));
    push_root_step(&core, RootKind::Schema, RestoreStepKind::VerifySchema, &mut steps)?;
    push_root_step(&core, RootKind::Artifact, RestoreStepKind::MaterializeArtifacts, &mut steps)?;
    push_root_step(&core, RootKind::Policy, RestoreStepKind::AdmitPolicy, &mut steps)?;
    push_root_step(&core, RootKind::RuntimeProfile, RestoreStepKind::AdmitRuntimeProfile, &mut steps)?;
    push_optional_root_step(&core, RootKind::DurableState, RestoreStepKind::RestoreDurableState, &mut steps);
    push_optional_root_step(&core, RootKind::History, RestoreStepKind::RestoreHistory, &mut steps);
    push_optional_root_step(&core, RootKind::Tasks, RestoreStepKind::RestoreTasks, &mut steps);
    push_optional_root_step(&core, RootKind::Scheduler, RestoreStepKind::RestoreScheduler, &mut steps);
    push_optional_root_step(&core, RootKind::Time, RestoreStepKind::RestoreTime, &mut steps);
    push_optional_root_step(&core, RootKind::Entropy, RestoreStepKind::RestoreEntropy, &mut steps);
    push_optional_root_step(&core, RootKind::Effects, RestoreStepKind::RestoreEffects, &mut steps);
    push_optional_root_step(
        &core,
        RootKind::AuthorityObservation,
        RestoreStepKind::RecordAuthorityObservation,
        &mut steps,
    );
    push_optional_root_step(
        &core,
        RootKind::OpaqueMachineSnapshot,
        RestoreStepKind::RestoreOpaqueMachineSnapshot,
        &mut steps,
    );
    steps.push(RestoreStep {
        kind: RestoreStepKind::RecheckCurrentAdmission,
        root: None,
    });
    steps.push(RestoreStep {
        kind: RestoreStepKind::ActivateRuntime,
        root: None,
    });
    let replay = core
        .roots
        .iter()
        .map(|root| RootReplayClassification {
            root_kind: root.kind(),
            class: super::replay_class(root.kind()),
        })
        .collect();
    Ok(RestorePlan {
        commit_ref: commit_ref.clone(),
        steps,
        replay,
        current_admission_required: true,
    })
}

fn validate_closure_bound(request: &ClosureRequest, issues: &mut Vec<ClosureIssue>) -> bool {
    let bound_issues = super::validate_bounds(&request.bounds);
    if !bound_issues.is_empty() {
        issues.push(ClosureIssue::InvalidCore(format!("{bound_issues:?}")));
        return false;
    }
    let direct_objects = request.roots.len().saturating_add(request.parent_graph.len());
    if direct_objects > request.bounds.max_closure_objects {
        issues.push(ClosureIssue::BoundExceeded {
            field: "closure-objects",
            actual: direct_objects,
            maximum: request.bounds.max_closure_objects,
        });
        return false;
    }
    let actual_parent_max = request
        .parent_graph
        .iter()
        .map(|observation| observation.parents.len())
        .chain(std::iter::once(request.core.parents.len()))
        .max()
        .unwrap_or(0);
    if actual_parent_max > request.bounds.max_parents {
        issues.push(ClosureIssue::BoundExceeded {
            field: "parent-edges-per-commit",
            actual: actual_parent_max,
            maximum: request.bounds.max_parents,
        });
        return false;
    }
    let parent_edges = request
        .parent_graph
        .iter()
        .map(|observation| observation.parents.len())
        .fold(request.core.parents.len(), usize::saturating_add);
    let observed = direct_objects.saturating_add(parent_edges);
    if observed > request.bounds.max_closure_objects {
        issues.push(ClosureIssue::BoundExceeded {
            field: "closure-objects-and-edges",
            actual: observed,
            maximum: request.bounds.max_closure_objects,
        });
        return false;
    }
    true
}

fn validate_root_objects(
    request: &ClosureRequest,
    core: &WorldCommitCore,
    issues: &mut Vec<ClosureIssue>,
) -> Option<RootKind> {
    let mut observations = BTreeMap::new();
    for observation in &request.roots {
        let kind = observation.root.kind();
        if observations.insert(kind, observation).is_some() {
            issues.push(ClosureIssue::DuplicateRootObservation(kind));
        }
        if root_for_kind(&core.roots, kind).is_none() {
            issues.push(ClosureIssue::UnexpectedRootObservation(kind));
        }
    }
    let mut first_missing = None;
    for root in &core.roots {
        let kind = root.kind();
        let Some(observation) = observations.get(&kind) else {
            issues.push(ClosureIssue::MissingRootObject(kind));
            first_missing.get_or_insert(kind);
            continue;
        };
        if !observation.object_present {
            issues.push(ClosureIssue::MissingRootObject(kind));
            first_missing.get_or_insert(kind);
        }
        if observation.root != *root || !observation.identity_matches {
            issues.push(ClosureIssue::RootIdentityMismatch(kind));
        }
        if !observation.schema_matches {
            issues.push(ClosureIssue::RootSchemaMismatch(kind));
        }
    }
    first_missing
}

fn validate_parent_graph(request: &ClosureRequest, core: &WorldCommitCore, issues: &mut Vec<ClosureIssue>) {
    let mut graph = BTreeMap::<WorldCommitRef, Vec<WorldCommitRef>>::new();
    graph.insert(request.commit_ref.clone(), core.parents.clone());
    for observation in &request.parent_graph {
        super::validate_parent_edges(&observation.commit_ref, &observation.parents, &request.bounds, issues);
        if graph.insert(observation.commit_ref.clone(), observation.parents.clone()).is_some() {
            issues.push(ClosureIssue::DuplicateParentObservation(observation.commit_ref.as_str().to_string()));
        }
        if !observation.object_present {
            issues.push(ClosureIssue::MissingParentObject(observation.commit_ref.as_str().to_string()));
        }
    }
    super::validate_parent_edges(&request.commit_ref, &core.parents, &request.bounds, issues);
    for (commit_ref, parents) in &graph {
        for parent in parents {
            if !graph.contains_key(parent) {
                issues.push(ClosureIssue::MissingParentObservation(format!(
                    "{} -> {}",
                    commit_ref.as_str(),
                    parent.as_str()
                )));
            }
        }
    }
    if let Some(cycle) = first_cycle(&request.commit_ref, &graph) {
        issues.push(ClosureIssue::ParentCycle(cycle.as_str().to_string()));
    }
}

fn first_cycle(root: &WorldCommitRef, graph: &BTreeMap<WorldCommitRef, Vec<WorldCommitRef>>) -> Option<WorldCommitRef> {
    let capacity = graph.len().saturating_mul(WALK_EVENT_FACTOR);
    let mut stack = Vec::with_capacity(capacity);
    let mut visiting = BTreeSet::new();
    let mut visited = BTreeSet::new();
    stack.push((root.clone(), false));
    while let Some((node, is_exit)) = stack.pop() {
        if is_exit {
            visiting.remove(&node);
            visited.insert(node);
            continue;
        }
        if visited.contains(&node) {
            continue;
        }
        if !visiting.insert(node.clone()) {
            return Some(node);
        }
        stack.push((node.clone(), true));
        if let Some(parents) = graph.get(&node) {
            for parent in parents.iter().rev() {
                stack.push((parent.clone(), false));
            }
        }
    }
    None
}

fn push_root_step(
    core: &WorldCommitCore,
    kind: RootKind,
    step: RestoreStepKind,
    steps: &mut Vec<RestoreStep>,
) -> Result<(), RestoreIssue> {
    let root = root_for_kind(&core.roots, kind).cloned().ok_or(RestoreIssue::RootUnavailable(kind))?;
    steps.push(RestoreStep {
        kind: step,
        root: Some(root),
    });
    Ok(())
}

fn push_optional_root_step(
    core: &WorldCommitCore,
    kind: RootKind,
    step: RestoreStepKind,
    steps: &mut Vec<RestoreStep>,
) {
    if let Some(root) = root_for_kind(&core.roots, kind) {
        steps.push(RestoreStep {
            kind: step,
            root: Some(root.clone()),
        });
    }
}

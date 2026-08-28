use std::collections::BTreeMap;
use std::collections::BTreeSet;

use super::super::*;
use super::bounded_sorted_issues;

pub fn required_world_replay_closure(
    trace: &WorldTransitionTrace,
    commits: &[WorldReplayCommitClosure],
    additional: &[WorldReplayClosureRequirement],
) -> Vec<WorldReplayClosureRequirement> {
    let mut requirements = BTreeSet::new();
    requirements.insert(WorldReplayClosureRequirement {
        object_ref: trace.trace_ref.clone(),
        role: WorldReplayCapsuleMemberRole::Trace,
    });
    requirements.insert(WorldReplayClosureRequirement {
        object_ref: trace.profile.profile_ref.as_str().to_string(),
        role: WorldReplayCapsuleMemberRole::RuntimeProfile,
    });
    if let Some(cohort_ref) = &trace.profile.cohort_ref {
        requirements.insert(WorldReplayClosureRequirement {
            object_ref: cohort_ref.as_str().to_string(),
            role: WorldReplayCapsuleMemberRole::RuntimeCohort,
        });
    }
    if let Some(descriptor_ref) = &trace.profile.snapshot_descriptor_ref {
        requirements.insert(WorldReplayClosureRequirement {
            object_ref: descriptor_ref.clone(),
            role: WorldReplayCapsuleMemberRole::SnapshotDescriptor,
        });
    }
    for step in &trace.steps {
        requirements.insert(WorldReplayClosureRequirement {
            object_ref: step.input.input_ref.clone(),
            role: WorldReplayCapsuleMemberRole::TransitionInput,
        });
    }
    for commit in commits {
        requirements.insert(WorldReplayClosureRequirement {
            object_ref: commit.commit_ref.as_str().to_string(),
            role: WorldReplayCapsuleMemberRole::WorldCommit,
        });
        for root in &commit.roots {
            requirements.insert(WorldReplayClosureRequirement {
                object_ref: root.as_str().to_string(),
                role: WorldReplayCapsuleMemberRole::TypedRoot(root.kind()),
            });
            if let Some(role) = semantic_root_role(root.kind()) {
                requirements.insert(WorldReplayClosureRequirement {
                    object_ref: root.as_str().to_string(),
                    role,
                });
            }
        }
    }
    requirements.extend(additional.iter().cloned());
    requirements.into_iter().collect()
}

pub(crate) fn validate_commit_closure(
    trace: &WorldTransitionTrace,
    commits: &[WorldReplayCommitClosure],
) -> Vec<WorldReplayIssue> {
    let mut issues = Vec::with_capacity(MAX_WORLD_REPLAY_DIAGNOSTICS);
    let mut by_ref = BTreeMap::new();
    for commit in commits {
        let key = commit.commit_ref.as_str();
        if by_ref.insert(key, commit).is_some() {
            issues.push(WorldReplayIssue::DuplicateCommit(key.to_string()));
        }
        if !commit.canonical_identity_verified {
            issues.push(WorldReplayIssue::CommitIdentityUnverified(key.to_string()));
        }
        issues.extend(validate_commit_roots(commit));
    }
    issues.extend(validate_required_commits(trace, &by_ref));
    bounded_sorted_issues(issues, MAX_WORLD_REPLAY_DIAGNOSTICS)
}

pub(crate) fn validate_capsule_closure(
    capsule: &WorldReplayCapsule,
    requirements: &[WorldReplayClosureRequirement],
    maximum: usize,
) -> Vec<WorldReplayIssue> {
    let required = requirements.iter().cloned().collect::<BTreeSet<_>>();
    let provided = capsule
        .members
        .iter()
        .flat_map(|member| {
            member.roles.iter().map(|role| WorldReplayClosureRequirement {
                object_ref: member.object_ref.clone(),
                role: *role,
            })
        })
        .collect::<BTreeSet<_>>();
    let mut issues = Vec::with_capacity(maximum);
    for missing in required.difference(&provided) {
        issues.push(WorldReplayIssue::MissingClosureRole {
            object_ref: missing.object_ref.clone(),
            role: missing.role.label(),
        });
    }
    for extra in provided.difference(&required) {
        issues.push(WorldReplayIssue::UndeclaredClosureRole {
            object_ref: extra.object_ref.clone(),
            role: extra.role.label(),
        });
    }
    bounded_sorted_issues(issues, maximum)
}

fn semantic_root_role(kind: crate::world_commit::RootKind) -> Option<WorldReplayCapsuleMemberRole> {
    match kind {
        crate::world_commit::RootKind::Artifact => Some(WorldReplayCapsuleMemberRole::Artifact),
        crate::world_commit::RootKind::Schema => Some(WorldReplayCapsuleMemberRole::Schema),
        crate::world_commit::RootKind::Policy => Some(WorldReplayCapsuleMemberRole::Policy),
        crate::world_commit::RootKind::RuntimeProfile => Some(WorldReplayCapsuleMemberRole::RuntimeProfile),
        crate::world_commit::RootKind::DurableState
        | crate::world_commit::RootKind::Tasks
        | crate::world_commit::RootKind::History
        | crate::world_commit::RootKind::Effects
        | crate::world_commit::RootKind::Scheduler
        | crate::world_commit::RootKind::Time
        | crate::world_commit::RootKind::Entropy
        | crate::world_commit::RootKind::AuthorityObservation
        | crate::world_commit::RootKind::OpaqueMachineSnapshot => None,
    }
}

fn validate_commit_roots(commit: &WorldReplayCommitClosure) -> Vec<WorldReplayIssue> {
    let mut issues = Vec::with_capacity(MAX_WORLD_REPLAY_DIAGNOSTICS);
    let mut root_kinds = BTreeSet::new();
    for root in &commit.roots {
        if !root_kinds.insert(root.kind()) {
            issues.push(WorldReplayIssue::DuplicateCommitRoot {
                commit_ref: commit.commit_ref.as_str().to_string(),
                root_kind: root.kind(),
            });
        }
    }
    issues
}

fn validate_required_commits<'a>(
    trace: &WorldTransitionTrace,
    by_ref: &BTreeMap<&'a str, &'a WorldReplayCommitClosure>,
) -> Vec<WorldReplayIssue> {
    let mut issues = Vec::with_capacity(MAX_WORLD_REPLAY_DIAGNOSTICS);
    let mut required = BTreeSet::new();
    required.insert(trace.initial_commit.as_str());
    for step in &trace.steps {
        required.insert(step.expected_successor.as_str());
        let Some(successor) = by_ref.get(step.expected_successor.as_str()) else {
            issues.push(WorldReplayIssue::MissingCommit(step.expected_successor.as_str().to_string()));
            continue;
        };
        if !successor.parents.iter().any(|parent| parent == &step.expected_parent) {
            issues.push(WorldReplayIssue::CommitParentMismatch(step.expected_successor.as_str().to_string()));
        }
    }
    for required_ref in required {
        if !by_ref.contains_key(required_ref) {
            issues.push(WorldReplayIssue::MissingCommit(required_ref.to_string()));
        }
    }
    issues
}

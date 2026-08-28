use std::collections::BTreeMap;

use super::super::*;
use crate::world_commit::RootKind;
use crate::world_commit::WorldCommitRef;
use crate::world_commit::WorldRootRef;

pub(super) const MEMBER_BYTES: u64 = 64;
pub(super) const EXPECTED_STEP_COUNT: usize = 2;

pub(super) fn valid_request() -> WorldReplayPlanRequest {
    let trace = valid_trace();
    let commits = valid_commits(&trace);
    let additional_requirements = vec![
        WorldReplayClosureRequirement {
            object_ref: digest("content-manifest"),
            role: WorldReplayCapsuleMemberRole::ContentManifest,
        },
        WorldReplayClosureRequirement {
            object_ref: digest("sealed-reproduction-bundle"),
            role: WorldReplayCapsuleMemberRole::SealedReproductionBundle,
        },
    ];
    let capsule = valid_capsule(&trace, &commits, &additional_requirements);
    WorldReplayPlanRequest {
        supported_profile_refs: vec![trace.profile.profile_ref.as_str().to_string()],
        trace,
        capsule,
        commits,
        additional_requirements,
        bounds: WorldReplayBounds::default(),
    }
}

pub(super) fn matching_observations(request: &WorldReplayPlanRequest) -> Vec<WorldReplayTransitionObservation> {
    request
        .trace
        .steps
        .iter()
        .map(|step| {
            let closure = request
                .commits
                .iter()
                .find(|commit| commit.commit_ref == step.expected_successor)
                .expect("expected closure");
            WorldReplayTransitionObservation {
                position: step.position,
                observed_parent: step.expected_parent.clone(),
                actual: WorldReplayObservedCommit {
                    commit_ref: step.expected_successor.clone(),
                    roots: closure.roots.clone(),
                },
                field_differences: Vec::new(),
            }
        })
        .collect()
}

pub(super) fn replace_trace_member(capsule: &mut WorldReplayCapsule, new_trace_ref: &str) {
    let member = capsule
        .members
        .iter_mut()
        .find(|member| member.roles.contains(&WorldReplayCapsuleMemberRole::Trace))
        .expect("trace member");
    member.object_ref = new_trace_ref.to_string();
    capsule.members.sort_by(|left, right| left.object_ref.cmp(&right.object_ref));
}

pub(super) fn root(kind: RootKind, label: &str) -> WorldRootRef {
    WorldRootRef::parse(kind, digest(label)).expect("root ref")
}

pub(super) fn commit_ref(label: &str) -> WorldCommitRef {
    WorldCommitRef::new(digest(label)).expect("commit ref")
}

pub(super) fn digest(label: &str) -> String {
    format!("blake3:{}", blake3::hash(label.as_bytes()).to_hex())
}

fn valid_trace() -> WorldTransitionTrace {
    let profile = WorldReplayProfile {
        kind: WorldReplayProfileKind::Logical,
        profile_ref: profile_ref("logical-profile"),
        cohort_ref: None,
        snapshot_descriptor_ref: None,
    };
    let initial_commit = commit_ref("initial");
    let first_successor = commit_ref("successor-one");
    let second_successor = commit_ref("successor-two");
    let mut trace = WorldTransitionTrace {
        schema: WORLD_TRANSITION_TRACE_SCHEMA.to_string(),
        trace_ref: placeholder_ref(),
        initial_commit: initial_commit.clone(),
        profile: profile.clone(),
        steps: vec![
            WorldTransitionStep {
                position: 0,
                expected_parent: initial_commit,
                input: transition_input("input-one"),
                profile_ref: profile.profile_ref.clone(),
                expected_successor: first_successor.clone(),
            },
            WorldTransitionStep {
                position: 1,
                expected_parent: first_successor,
                input: transition_input("input-two"),
                profile_ref: profile.profile_ref.clone(),
                expected_successor: second_successor,
            },
        ],
    };
    trace.trace_ref = identify_world_transition_trace(&trace).expect("trace identity");
    trace
}

fn valid_commits(trace: &WorldTransitionTrace) -> Vec<WorldReplayCommitClosure> {
    vec![
        WorldReplayCommitClosure {
            commit_ref: trace.initial_commit.clone(),
            parents: Vec::new(),
            roots: vec![
                root(RootKind::Artifact, "artifact-initial"),
                root(RootKind::Schema, "schema"),
            ],
            canonical_identity_verified: true,
        },
        WorldReplayCommitClosure {
            commit_ref: trace.steps[0].expected_successor.clone(),
            parents: vec![trace.steps[0].expected_parent.clone()],
            roots: vec![
                root(RootKind::Artifact, "artifact-one"),
                root(RootKind::Schema, "schema"),
            ],
            canonical_identity_verified: true,
        },
        WorldReplayCommitClosure {
            commit_ref: trace.steps[1].expected_successor.clone(),
            parents: vec![trace.steps[1].expected_parent.clone()],
            roots: vec![
                root(RootKind::Artifact, "artifact-two"),
                root(RootKind::Schema, "schema"),
            ],
            canonical_identity_verified: true,
        },
    ]
}

fn valid_capsule(
    trace: &WorldTransitionTrace,
    commits: &[WorldReplayCommitClosure],
    additional: &[WorldReplayClosureRequirement],
) -> WorldReplayCapsule {
    let requirements = required_world_replay_closure(trace, commits, additional);
    let mut roles_by_ref = BTreeMap::<String, Vec<WorldReplayCapsuleMemberRole>>::new();
    for requirement in requirements {
        roles_by_ref.entry(requirement.object_ref).or_default().push(requirement.role);
    }
    let members = roles_by_ref
        .into_iter()
        .map(|(object_ref, mut roles)| {
            roles.sort();
            roles.dedup();
            let codec = if roles.contains(&WorldReplayCapsuleMemberRole::ContentManifest) {
                WorldReplayMemberCodec::ContentManifestV1
            } else if roles.contains(&WorldReplayCapsuleMemberRole::SealedReproductionBundle) {
                WorldReplayMemberCodec::SealedReproductionBundleV1
            } else {
                WorldReplayMemberCodec::CanonicalPreservesV1
            };
            WorldReplayCapsuleMember {
                object_ref,
                roles,
                codec,
                byte_length: MEMBER_BYTES,
                protection: WorldReplayMemberProtection::Public,
            }
        })
        .collect::<Vec<_>>();
    let mut capsule = WorldReplayCapsule {
        schema: WORLD_REPLAY_CAPSULE_SCHEMA.to_string(),
        capsule_ref: placeholder_ref(),
        trace_ref: trace.trace_ref.clone(),
        profile: trace.profile.clone(),
        members,
        non_claims: world_replay_non_claims(),
    };
    capsule.capsule_ref = identify_world_replay_capsule(&capsule).expect("capsule identity");
    capsule
}

fn transition_input(label: &str) -> WorldTransitionInput {
    WorldTransitionInput {
        kind: WorldTransitionInputKind::Command,
        input_ref: digest(label),
        schema_ref: digest(&format!("{label}-schema")),
        byte_length: MEMBER_BYTES,
    }
}

fn profile_ref(label: &str) -> crate::world_commit::SnapshotProfileRef {
    crate::world_commit::SnapshotProfileRef::new(digest(label)).expect("profile ref")
}

fn placeholder_ref() -> String {
    digest("placeholder")
}

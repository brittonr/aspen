use std::collections::BTreeMap;

use molten_core::world_commit::*;
use molten_core::world_replay::*;

use super::super::*;

pub(super) const MEMBER_BYTES: u64 = 8;
pub(super) const CHUNK_BYTES: u64 = 4;
pub(super) const EXPECTED_TRANSITIONS: usize = 2;
pub(super) const ADMISSION_GENERATION: u64 = 7;

pub(super) struct Fixture {
    pub request: WorldReplayPlanRequest,
    pub commits: Vec<crate::world_commit::CanonicalWorldCommit>,
}

pub(super) fn fixture(kind: WorldReplayProfileKind) -> Fixture {
    let profile_ref = SnapshotProfileRef::new(digest("replay-profile")).expect("profile ref");
    let cohort_ref = (kind == WorldReplayProfileKind::Opaque)
        .then(|| SnapshotCohortRef::new(digest("opaque-cohort")).expect("cohort ref"));
    let snapshot_kind = match kind {
        WorldReplayProfileKind::Logical => SnapshotProfileKind::Logical,
        WorldReplayProfileKind::Opaque => SnapshotProfileKind::Opaque,
    };
    let snapshot_profile = SnapshotProfile {
        kind: snapshot_kind,
        profile_ref: profile_ref.clone(),
        cohort_ref: cohort_ref.clone(),
    };
    let initial = canonical_commit(&snapshot_profile, Vec::new(), "initial");
    let successor_one = canonical_commit(&snapshot_profile, vec![initial.commit_ref.clone()], "successor-one");
    let successor_two = canonical_commit(&snapshot_profile, vec![successor_one.commit_ref.clone()], "successor-two");
    let replay_profile = WorldReplayProfile {
        kind,
        profile_ref: profile_ref.clone(),
        cohort_ref,
        snapshot_descriptor_ref: (kind == WorldReplayProfileKind::Opaque).then(|| digest("snapshot-descriptor")),
    };
    let mut trace = transition_trace(replay_profile.clone(), &initial, &successor_one, &successor_two);
    trace.trace_ref = identify_world_transition_trace(&trace).expect("trace identity");
    let commits = vec![initial, successor_one, successor_two];
    let closures = commits
        .iter()
        .map(|commit| WorldReplayCommitClosure {
            commit_ref: commit.commit_ref.clone(),
            parents: commit.core.parents.clone(),
            roots: commit.core.roots.clone(),
            canonical_identity_verified: true,
        })
        .collect::<Vec<_>>();
    let additional_requirements = additional_requirements();
    let requirements = required_world_replay_closure(&trace, &closures, &additional_requirements);
    let members = capsule_members(requirements);
    let mut capsule = WorldReplayCapsule {
        schema: WORLD_REPLAY_CAPSULE_SCHEMA.to_string(),
        capsule_ref: placeholder_ref(),
        trace_ref: trace.trace_ref.clone(),
        profile: replay_profile,
        members,
        non_claims: world_replay_non_claims(),
    };
    capsule.capsule_ref = identify_world_replay_capsule(&capsule).expect("capsule identity");
    Fixture {
        request: WorldReplayPlanRequest {
            supported_profile_refs: vec![capsule.profile.profile_ref.as_str().to_string()],
            trace,
            capsule,
            commits: closures,
            additional_requirements,
            bounds: WorldReplayBounds::default(),
        },
        commits,
    }
}

pub(super) fn payloads(capsule: &WorldReplayCapsule) -> Vec<WorldReplayMemberPayload> {
    let member_len = usize::try_from(MEMBER_BYTES).expect("fixture member length");
    capsule
        .members
        .iter()
        .map(|member| WorldReplayMemberPayload {
            object_ref: member.object_ref.clone(),
            bytes: vec![0; member_len],
        })
        .collect()
}

pub(super) fn dependency_refs() -> Vec<String> {
    vec![digest("molten"), digest("chaoscontrol"), digest("valence")]
}

pub(super) fn digest(label: &str) -> String {
    format!("blake3:{}", blake3::hash(label.as_bytes()).to_hex())
}

pub(super) fn placeholder_ref() -> String {
    digest("placeholder")
}

pub(super) fn temporary_root(label: &str) -> std::path::PathBuf {
    let path = std::env::temp_dir().join(format!("molten-{label}-{}", std::process::id()));
    if path.exists() {
        std::fs::remove_dir_all(&path).expect("remove stale fixture root");
    }
    path
}

fn canonical_commit(
    profile: &SnapshotProfile,
    parents: Vec<WorldCommitRef>,
    label: &str,
) -> crate::world_commit::CanonicalWorldCommit {
    let roots = profile
        .kind
        .required_roots()
        .iter()
        .map(|kind| WorldRootRef::parse(*kind, digest(&format!("{label}-{}", kind.as_str()))).expect("root"))
        .collect();
    crate::world_commit::canonical_world_commit(
        &WorldCommitCore {
            version: WorldCommitVersion::V1,
            profile: profile.clone(),
            parents,
            roots,
            completeness: CompletenessClaim::for_profile(profile.kind),
        },
        &WorldCommitBounds::default(),
    )
    .expect("canonical commit")
}

fn transition_trace(
    profile: WorldReplayProfile,
    initial: &crate::world_commit::CanonicalWorldCommit,
    first: &crate::world_commit::CanonicalWorldCommit,
    second: &crate::world_commit::CanonicalWorldCommit,
) -> WorldTransitionTrace {
    WorldTransitionTrace {
        schema: WORLD_TRANSITION_TRACE_SCHEMA.to_string(),
        trace_ref: placeholder_ref(),
        initial_commit: initial.commit_ref.clone(),
        profile: profile.clone(),
        steps: vec![
            WorldTransitionStep {
                position: 0,
                expected_parent: initial.commit_ref.clone(),
                input: transition_input("input-one"),
                profile_ref: profile.profile_ref.clone(),
                expected_successor: first.commit_ref.clone(),
            },
            WorldTransitionStep {
                position: 1,
                expected_parent: first.commit_ref.clone(),
                input: transition_input("input-two"),
                profile_ref: profile.profile_ref,
                expected_successor: second.commit_ref.clone(),
            },
        ],
    }
}

fn additional_requirements() -> Vec<WorldReplayClosureRequirement> {
    vec![
        WorldReplayClosureRequirement {
            object_ref: digest("content-manifest"),
            role: WorldReplayCapsuleMemberRole::ContentManifest,
        },
        WorldReplayClosureRequirement {
            object_ref: digest("sealed-bundle"),
            role: WorldReplayCapsuleMemberRole::SealedReproductionBundle,
        },
    ]
}

fn capsule_members(requirements: Vec<WorldReplayClosureRequirement>) -> Vec<WorldReplayCapsuleMember> {
    let mut roles = BTreeMap::<String, Vec<WorldReplayCapsuleMemberRole>>::new();
    for requirement in requirements {
        roles.entry(requirement.object_ref).or_default().push(requirement.role);
    }
    roles
        .into_iter()
        .map(|(object_ref, mut roles)| {
            roles.sort();
            roles.dedup();
            WorldReplayCapsuleMember {
                codec: member_codec(&roles),
                object_ref,
                roles,
                byte_length: MEMBER_BYTES,
                protection: WorldReplayMemberProtection::Public,
            }
        })
        .collect()
}

fn member_codec(roles: &[WorldReplayCapsuleMemberRole]) -> WorldReplayMemberCodec {
    if roles.contains(&WorldReplayCapsuleMemberRole::ContentManifest) {
        WorldReplayMemberCodec::ContentManifestV1
    } else if roles.contains(&WorldReplayCapsuleMemberRole::SealedReproductionBundle) {
        WorldReplayMemberCodec::SealedReproductionBundleV1
    } else {
        WorldReplayMemberCodec::CanonicalPreservesV1
    }
}

fn transition_input(label: &str) -> WorldTransitionInput {
    WorldTransitionInput {
        kind: WorldTransitionInputKind::Command,
        input_ref: digest(label),
        schema_ref: digest(&format!("{label}-schema")),
        byte_length: MEMBER_BYTES,
    }
}

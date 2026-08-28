use super::super::*;
use super::fixture::*;

#[test]
fn complete_logical_trace_plans_stable_ordered_replay() {
    // r[verify molten.world_replay.transition_chain]
    // r[verify molten.world_replay.capsule]
    let request = valid_request();
    let first = plan_world_replay(&request).expect("complete replay plan");
    let second = plan_world_replay(&request).expect("stable replay plan");

    assert_eq!(first, second);
    assert_eq!(first.plan_ref, second.plan_ref);
    assert!(first.current_admission_required);
    assert_eq!(first.non_claims, world_replay_non_claims());
    assert_eq!(
        first
            .operations
            .iter()
            .filter(|operation| operation.kind == WorldReplayOperationKind::ExecuteTransition)
            .count(),
        EXPECTED_STEP_COUNT
    );
    assert_eq!(
        first
            .operations
            .iter()
            .find(|operation| {
                matches!(
                    operation.kind,
                    WorldReplayOperationKind::RestoreLogicalProfile | WorldReplayOperationKind::RestoreOpaqueProfile
                )
            })
            .map(|operation| operation.kind),
        Some(WorldReplayOperationKind::RestoreLogicalProfile)
    );
}

#[test]
fn trace_rejects_wrong_parent_and_profile_drift() {
    // r[verify molten.world_replay.transition_chain]
    let mut request = valid_request();
    request.trace.steps[1].expected_parent = commit_ref("wrong-parent");
    request.trace.steps[1].profile_ref =
        crate::world_commit::SnapshotProfileRef::new(digest("wrong-profile")).expect("profile ref");
    request.trace.trace_ref = identify_world_transition_trace(&request.trace).expect("trace identity");
    request.capsule.trace_ref = request.trace.trace_ref.clone();
    replace_trace_member(&mut request.capsule, &request.trace.trace_ref);
    request.capsule.capsule_ref = identify_world_replay_capsule(&request.capsule).expect("capsule identity");

    let issues = plan_world_replay(&request).expect_err("drifted trace denied");
    assert!(issues.contains(&WorldReplayIssue::StepParentMismatch { position: 1 }));
    assert!(issues.contains(&WorldReplayIssue::StepProfileMismatch { position: 1 }));
}

#[test]
fn capsule_rejects_missing_and_extra_declared_closure() {
    // r[verify molten.world_replay.capsule]
    let mut missing = valid_request();
    let missing_ref = missing.trace.steps[0].input.input_ref.clone();
    missing.capsule.members.retain(|member| member.object_ref != missing_ref);
    missing.capsule.capsule_ref = identify_world_replay_capsule(&missing.capsule).expect("capsule identity");
    let missing_issues = plan_world_replay(&missing).expect_err("missing closure denied");
    assert!(missing_issues.iter().any(|issue| {
        matches!(
            issue,
            WorldReplayIssue::MissingClosureRole { object_ref, role }
                if object_ref == &missing_ref && role == "transition-input"
        )
    }));

    let mut extra = valid_request();
    extra.capsule.members.push(WorldReplayCapsuleMember {
        object_ref: digest("extra-member"),
        roles: vec![WorldReplayCapsuleMemberRole::Artifact],
        codec: WorldReplayMemberCodec::CanonicalPreservesV1,
        byte_length: MEMBER_BYTES,
        protection: WorldReplayMemberProtection::Public,
    });
    extra.capsule.members.sort_by(|left, right| left.object_ref.cmp(&right.object_ref));
    extra.capsule.capsule_ref = identify_world_replay_capsule(&extra.capsule).expect("capsule identity");
    let extra_issues = plan_world_replay(&extra).expect_err("extra closure denied");
    assert!(extra_issues.iter().any(|issue| {
        matches!(
            issue,
            WorldReplayIssue::UndeclaredClosureRole { object_ref, role }
                if object_ref == &digest("extra-member") && role == "artifact"
        )
    }));
}

#[test]
fn capsule_rejects_noncanonical_member_and_role_order() {
    let mut request = valid_request();
    request.capsule.members.reverse();
    let trace_member = request
        .capsule
        .members
        .iter_mut()
        .find(|member| member.roles.contains(&WorldReplayCapsuleMemberRole::Trace))
        .expect("trace member");
    trace_member.roles.push(WorldReplayCapsuleMemberRole::Artifact);
    trace_member.roles.sort();
    trace_member.roles.reverse();
    request.capsule.capsule_ref = identify_world_replay_capsule(&request.capsule).expect("capsule identity");

    let issues = validate_world_replay_capsule(&request.capsule, &request.bounds);
    assert!(issues.contains(&WorldReplayIssue::NonCanonicalMemberOrder));
    assert!(issues.iter().any(|issue| { matches!(issue, WorldReplayIssue::NonCanonicalMemberRoleOrder(_)) }));
}

#[test]
fn reordered_steps_stale_schema_and_unsupported_profile_are_denied() {
    // r[verify molten.world_replay.verification]
    let mut request = valid_request();
    request.trace.steps.swap(0, 1);
    request.trace.schema = "molten.world-replay.transition-trace.v0".to_string();
    request.trace.trace_ref = identify_world_transition_trace(&request.trace).expect("trace identity");
    request.capsule.trace_ref = request.trace.trace_ref.clone();
    replace_trace_member(&mut request.capsule, &request.trace.trace_ref);
    request.capsule.capsule_ref = identify_world_replay_capsule(&request.capsule).expect("capsule identity");
    request.supported_profile_refs.clear();

    let issues = plan_world_replay(&request).expect_err("stale reordered trace denied");
    assert!(issues.contains(&WorldReplayIssue::InvalidSchema("transition-trace")));
    assert!(issues.contains(&WorldReplayIssue::UnsupportedProfile));
    assert!(issues.iter().any(|issue| { matches!(issue, WorldReplayIssue::NonContiguousStep { .. }) }));
}

#[test]
fn opaque_profile_requires_exact_cohort_and_descriptor_without_fallback() {
    // r[verify molten.world_replay.execution_boundary]
    let mut request = valid_request();
    request.trace.profile.kind = WorldReplayProfileKind::Opaque;
    request.trace.profile.cohort_ref = None;
    request.trace.profile.snapshot_descriptor_ref = None;
    request.trace.trace_ref = identify_world_transition_trace(&request.trace).expect("trace identity");
    request.capsule.profile = request.trace.profile.clone();
    request.capsule.trace_ref = request.trace.trace_ref.clone();
    replace_trace_member(&mut request.capsule, &request.trace.trace_ref);
    request.capsule.capsule_ref = identify_world_replay_capsule(&request.capsule).expect("capsule identity");

    let issues = plan_world_replay(&request).expect_err("inexact opaque profile denied");
    assert!(issues.contains(&WorldReplayIssue::OpaqueCohortMissing));
    assert!(issues.contains(&WorldReplayIssue::OpaqueSnapshotDescriptorMissing));
}

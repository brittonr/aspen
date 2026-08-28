use molten_core::world_commit::SnapshotCohortRef;
use molten_core::world_commit::WorldCommitRef;
use molten_core::world_replay::*;
use preserves::IOValue;

use super::CanonicalWorldReplayRecord;
use super::WORLD_REPLAY_CAPSULE_RECORD;
use super::WORLD_REPLAY_DIVERGENCE_RECORD;
use super::WORLD_REPLAY_PLAN_RECORD;
use super::WORLD_TRANSITION_TRACE_RECORD;
use super::support::*;
use crate::error::MoltenError;
use crate::error::Result;

// r[impl molten.world_replay.transition_chain]
pub fn canonical_world_transition_trace(trace: &WorldTransitionTrace) -> Result<CanonicalWorldReplayRecord> {
    core_issues(validate_world_transition_trace(trace, &wire_bounds()))?;
    canonical(
        "transition-trace",
        WORLD_TRANSITION_TRACE_RECORD,
        record(WORLD_TRANSITION_TRACE_RECORD, vec![
            string(&trace.schema),
            field("trace-ref", string(&trace.trace_ref)),
            field("initial-commit", string(trace.initial_commit.as_str())),
            profile_value(&trace.profile),
            field("steps", sequence(trace.steps.iter().map(transition_step_value).collect())),
        ]),
    )
}

// r[impl molten.world_replay.capsule]
pub fn canonical_world_replay_capsule(capsule: &WorldReplayCapsule) -> Result<CanonicalWorldReplayRecord> {
    core_issues(validate_world_replay_capsule(capsule, &wire_bounds()))?;
    canonical(
        "capsule",
        WORLD_REPLAY_CAPSULE_RECORD,
        record(WORLD_REPLAY_CAPSULE_RECORD, vec![
            string(&capsule.schema),
            field("capsule-ref", string(&capsule.capsule_ref)),
            field("trace-ref", string(&capsule.trace_ref)),
            profile_value(&capsule.profile),
            field("members", sequence(capsule.members.iter().map(capsule_member_value).collect())),
            non_claims_value(&capsule.non_claims),
        ]),
    )
}

pub fn canonical_world_replay_plan(plan: &WorldReplayPlan) -> Result<CanonicalWorldReplayRecord> {
    core_issues(validate_world_replay_plan(plan))?;
    canonical(
        "plan",
        WORLD_REPLAY_PLAN_RECORD,
        record(WORLD_REPLAY_PLAN_RECORD, vec![
            string(&plan.schema),
            field("plan-ref", string(&plan.plan_ref)),
            field("trace-ref", string(&plan.trace_ref)),
            field("capsule-ref", string(&plan.capsule_ref)),
            profile_value(&plan.profile),
            field("operations", sequence(plan.operations.iter().map(operation_value).collect())),
            field("current-admission-required", boolean(plan.current_admission_required)),
            non_claims_value(&plan.non_claims),
        ]),
    )
}

// r[impl molten.world_replay.divergence]
pub fn canonical_world_replay_divergence(divergence: &WorldReplayDivergence) -> Result<CanonicalWorldReplayRecord> {
    let identity = identify_world_replay_divergence(divergence).map_err(core_issue)?;
    if divergence.schema != WORLD_REPLAY_DIVERGENCE_SCHEMA || divergence.divergence_ref != identity {
        return Err(MoltenError::invalid_harness("world replay divergence identity is invalid"));
    }
    canonical(
        "divergence",
        WORLD_REPLAY_DIVERGENCE_RECORD,
        record(WORLD_REPLAY_DIVERGENCE_RECORD, vec![
            string(&divergence.schema),
            field("divergence-ref", string(&divergence.divergence_ref)),
            field("kind", string(divergence.kind.as_str())),
            field("position", number(divergence.position)),
            field("expected-parent", string(divergence.expected_parent.as_str())),
            field("observed-parent", optional_ref(divergence.observed_parent.as_ref().map(WorldCommitRef::as_str))),
            field("expected-commit", string(divergence.expected_commit.as_str())),
            field("actual-commit", optional_ref(divergence.actual_commit.as_ref().map(WorldCommitRef::as_str))),
            field(
                "root-kind",
                divergence
                    .root_kind
                    .map_or_else(|| record("none", Vec::new()), |kind| record("some", vec![string(kind.as_str())])),
            ),
            field("field-path", sequence(divergence.field_path.iter().map(string).collect())),
        ]),
    )
}

fn wire_bounds() -> WorldReplayBounds {
    WorldReplayBounds {
        max_steps: MAX_WORLD_REPLAY_STEPS,
        max_members: MAX_WORLD_REPLAY_MEMBERS,
        max_member_bytes: MAX_WORLD_REPLAY_MEMBER_BYTES,
        max_total_bytes: MAX_WORLD_REPLAY_TOTAL_BYTES,
        max_field_path_segments: MAX_WORLD_REPLAY_FIELD_PATH_SEGMENTS,
        max_field_segment_bytes: MAX_WORLD_REPLAY_FIELD_SEGMENT_BYTES,
        max_diagnostics: MAX_WORLD_REPLAY_DIAGNOSTICS,
    }
}

fn transition_step_value(step: &WorldTransitionStep) -> IOValue {
    record("transition-step", vec![
        field("position", number(step.position)),
        field("expected-parent", string(step.expected_parent.as_str())),
        field("input-kind", string(step.input.kind.as_str())),
        field("input-ref", string(&step.input.input_ref)),
        field("input-schema-ref", string(&step.input.schema_ref)),
        field("input-byte-length", number(step.input.byte_length)),
        field("profile-ref", string(step.profile_ref.as_str())),
        field("expected-successor", string(step.expected_successor.as_str())),
    ])
}

pub(super) fn profile_value(profile: &WorldReplayProfile) -> IOValue {
    record("profile", vec![
        field("kind", string(profile.kind.as_str())),
        field("profile-ref", string(profile.profile_ref.as_str())),
        field("cohort-ref", optional_ref(profile.cohort_ref.as_ref().map(SnapshotCohortRef::as_str))),
        field("snapshot-descriptor-ref", optional_ref(profile.snapshot_descriptor_ref.as_deref())),
    ])
}

fn capsule_member_value(member: &WorldReplayCapsuleMember) -> IOValue {
    let protection = match &member.protection {
        WorldReplayMemberProtection::Public => record("public", Vec::new()),
        WorldReplayMemberProtection::Ciphertext { descriptor_ref } => {
            record("ciphertext", vec![string(descriptor_ref)])
        }
    };
    record("capsule-member", vec![
        field("object-ref", string(&member.object_ref)),
        field("roles", sequence(member.roles.iter().map(|role| string(role.label())).collect())),
        field("codec", string(member.codec.as_str())),
        field("byte-length", number(member.byte_length)),
        field("protection", protection),
    ])
}

fn operation_value(operation: &WorldReplayOperation) -> IOValue {
    record("replay-operation", vec![
        field("kind", string(operation.kind.as_str())),
        field(
            "position",
            operation
                .position
                .map_or_else(|| record("none", Vec::new()), |position| record("some", vec![number(position)])),
        ),
        field("subject-ref", string(&operation.subject_ref)),
    ])
}

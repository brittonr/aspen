use super::*;
use crate::world_commit::SnapshotCohortRef;
use crate::world_commit::WorldCommitRef;

pub fn identify_world_transition_trace(trace: &WorldTransitionTrace) -> Result<String, WorldReplayIssue> {
    let mut hasher = blake3::Hasher::new_derive_key(WORLD_TRANSITION_TRACE_IDENTITY_DOMAIN);
    update_text(&mut hasher, &trace.schema)?;
    update_text(&mut hasher, trace.initial_commit.as_str())?;
    update_profile(&mut hasher, &trace.profile)?;
    update_usize(&mut hasher, trace.steps.len())?;
    for step in &trace.steps {
        update_number(&mut hasher, step.position);
        update_text(&mut hasher, step.expected_parent.as_str())?;
        update_text(&mut hasher, step.input.kind.as_str())?;
        update_text(&mut hasher, &step.input.input_ref)?;
        update_text(&mut hasher, &step.input.schema_ref)?;
        update_number(&mut hasher, step.input.byte_length);
        update_text(&mut hasher, step.profile_ref.as_str())?;
        update_text(&mut hasher, step.expected_successor.as_str())?;
    }
    Ok(format!("blake3:{}", hasher.finalize().to_hex()))
}

pub fn identify_world_replay_capsule(capsule: &WorldReplayCapsule) -> Result<String, WorldReplayIssue> {
    let mut hasher = blake3::Hasher::new_derive_key(WORLD_REPLAY_CAPSULE_IDENTITY_DOMAIN);
    update_text(&mut hasher, &capsule.schema)?;
    update_text(&mut hasher, &capsule.trace_ref)?;
    update_profile(&mut hasher, &capsule.profile)?;
    update_usize(&mut hasher, capsule.members.len())?;
    for member in &capsule.members {
        update_text(&mut hasher, &member.object_ref)?;
        update_usize(&mut hasher, member.roles.len())?;
        for role in &member.roles {
            update_text(&mut hasher, &role.label())?;
        }
        update_text(&mut hasher, member.codec.as_str())?;
        update_number(&mut hasher, member.byte_length);
        update_text(&mut hasher, member.protection.as_str())?;
        match &member.protection {
            WorldReplayMemberProtection::Public => update_bool(&mut hasher, false),
            WorldReplayMemberProtection::Ciphertext { descriptor_ref } => {
                update_bool(&mut hasher, true);
                update_text(&mut hasher, descriptor_ref)?;
            }
        }
    }
    for non_claim in &capsule.non_claims {
        update_text(&mut hasher, non_claim)?;
    }
    Ok(format!("blake3:{}", hasher.finalize().to_hex()))
}

pub fn identify_world_replay_plan(plan: &WorldReplayPlan) -> Result<String, WorldReplayIssue> {
    let mut hasher = blake3::Hasher::new_derive_key(WORLD_REPLAY_PLAN_IDENTITY_DOMAIN);
    update_text(&mut hasher, &plan.schema)?;
    update_text(&mut hasher, &plan.trace_ref)?;
    update_text(&mut hasher, &plan.capsule_ref)?;
    update_profile(&mut hasher, &plan.profile)?;
    update_usize(&mut hasher, plan.operations.len())?;
    for operation in &plan.operations {
        update_text(&mut hasher, operation.kind.as_str())?;
        update_optional_number(&mut hasher, operation.position);
        update_text(&mut hasher, &operation.subject_ref)?;
    }
    update_bool(&mut hasher, plan.current_admission_required);
    for non_claim in &plan.non_claims {
        update_text(&mut hasher, non_claim)?;
    }
    Ok(format!("blake3:{}", hasher.finalize().to_hex()))
}

pub fn identify_world_replay_divergence(divergence: &WorldReplayDivergence) -> Result<String, WorldReplayIssue> {
    let mut hasher = blake3::Hasher::new_derive_key(WORLD_REPLAY_DIVERGENCE_IDENTITY_DOMAIN);
    update_text(&mut hasher, &divergence.schema)?;
    update_text(&mut hasher, divergence.kind.as_str())?;
    update_number(&mut hasher, divergence.position);
    update_text(&mut hasher, divergence.expected_parent.as_str())?;
    update_optional_text(&mut hasher, divergence.observed_parent.as_ref().map(WorldCommitRef::as_str))?;
    update_text(&mut hasher, divergence.expected_commit.as_str())?;
    update_optional_text(&mut hasher, divergence.actual_commit.as_ref().map(WorldCommitRef::as_str))?;
    match divergence.root_kind {
        Some(kind) => {
            update_bool(&mut hasher, true);
            update_text(&mut hasher, kind.as_str())?;
        }
        None => update_bool(&mut hasher, false),
    }
    update_usize(&mut hasher, divergence.field_path.len())?;
    for segment in &divergence.field_path {
        update_text(&mut hasher, segment)?;
    }
    Ok(format!("blake3:{}", hasher.finalize().to_hex()))
}

fn update_profile(hasher: &mut blake3::Hasher, profile: &WorldReplayProfile) -> Result<(), WorldReplayIssue> {
    update_text(hasher, profile.kind.as_str())?;
    update_text(hasher, profile.profile_ref.as_str())?;
    update_optional_text(hasher, profile.cohort_ref.as_ref().map(SnapshotCohortRef::as_str))?;
    update_optional_text(hasher, profile.snapshot_descriptor_ref.as_deref())
}

fn update_optional_text(hasher: &mut blake3::Hasher, value: Option<&str>) -> Result<(), WorldReplayIssue> {
    match value {
        Some(value) => {
            update_bool(hasher, true);
            update_text(hasher, value)
        }
        None => {
            update_bool(hasher, false);
            Ok(())
        }
    }
}

fn update_optional_number(hasher: &mut blake3::Hasher, value: Option<u64>) {
    match value {
        Some(value) => {
            update_bool(hasher, true);
            update_number(hasher, value);
        }
        None => update_bool(hasher, false),
    }
}

fn update_text(hasher: &mut blake3::Hasher, value: &str) -> Result<(), WorldReplayIssue> {
    let length = u64::try_from(value.len()).map_err(|_| WorldReplayIssue::InvalidText("identity-length"))?;
    update_number(hasher, length);
    hasher.update(value.as_bytes());
    Ok(())
}

fn update_usize(hasher: &mut blake3::Hasher, value: usize) -> Result<(), WorldReplayIssue> {
    let value = u64::try_from(value).map_err(|_| WorldReplayIssue::InvalidText("identity-count"))?;
    update_number(hasher, value);
    Ok(())
}

fn update_number(hasher: &mut blake3::Hasher, value: u64) {
    hasher.update(&value.to_be_bytes());
}

fn update_bool(hasher: &mut blake3::Hasher, value: bool) {
    hasher.update(&[u8::from(value)]);
}

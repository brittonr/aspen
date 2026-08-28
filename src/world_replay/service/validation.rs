use molten_core::world_commit::SnapshotCohortRef;
use molten_core::world_commit::SnapshotProfileKind;
use molten_core::world_replay::*;

use super::super::*;
use super::support::validate_ref;
use crate::error::MoltenError;
use crate::error::Result;

pub(super) fn validate_initial_profile(
    profile: &WorldReplayProfile,
    initial_commit: &crate::world_commit::CanonicalWorldCommit,
) -> Result<()> {
    let expected_kind = match profile.kind {
        WorldReplayProfileKind::Logical => SnapshotProfileKind::Logical,
        WorldReplayProfileKind::Opaque => SnapshotProfileKind::Opaque,
    };
    if initial_commit.core.profile.kind != expected_kind
        || initial_commit.core.profile.profile_ref != profile.profile_ref
        || initial_commit.core.profile.cohort_ref != profile.cohort_ref
    {
        return Err(MoltenError::invalid_harness(
            "world replay initial commit profile does not match the replay profile",
        ));
    }
    Ok(())
}

pub(super) fn validate_materialization(
    member: &WorldReplayCapsuleMember,
    observation: &WorldReplayMaterializationObservation,
) -> Result<()> {
    validate_ref(&observation.observation_ref, "world replay materialization")?;
    if observation.object_ref != member.object_ref || !observation.available || !observation.identity_verified {
        return Err(MoltenError::invalid_harness("world replay member materialization is unavailable or unverified"));
    }
    Ok(())
}

pub(super) fn validate_restore(
    profile: &WorldReplayProfile,
    observation: &WorldReplayRestoreObservation,
) -> Result<()> {
    validate_ref(&observation.observation_ref, "world replay restore")?;
    if observation.profile_ref != profile.profile_ref.as_str() {
        return Err(MoltenError::invalid_harness("world replay restore profile drifted"));
    }
    let expected_cohort = profile.cohort_ref.as_ref().map(SnapshotCohortRef::as_str);
    if observation.cohort_ref.as_deref() != expected_cohort {
        return Err(MoltenError::invalid_harness("world replay restore cohort drifted"));
    }
    if profile.kind == WorldReplayProfileKind::Opaque && observation.logical_fallback_used {
        return Err(MoltenError::invalid_harness("opaque world replay attempted a logical fallback"));
    }
    Ok(())
}

pub(super) fn validate_admission_binding(
    trace: &WorldTransitionTrace,
    capsule: &WorldReplayCapsule,
    observation: &WorldReplayAdmissionObservation,
) -> Result<()> {
    validate_ref(&observation.observation_ref, "world replay current admission")?;
    if observation.trace_ref != trace.trace_ref
        || observation.capsule_ref != capsule.capsule_ref
        || observation.profile_ref != trace.profile.profile_ref.as_str()
    {
        return Err(MoltenError::invalid_harness("world replay current admission is bound to different inputs"));
    }
    Ok(())
}

pub(super) fn validate_execution(
    step: &WorldTransitionStep,
    observation: &WorldReplayExecutionObservation,
) -> Result<()> {
    validate_ref(&observation.observation_ref, "world replay transition execution")?;
    if observation.position != step.position || observation.input_ref != step.input.input_ref {
        return Err(MoltenError::invalid_harness("world replay transition execution is bound to the wrong step"));
    }
    Ok(())
}

use super::*;
use crate::world_commit::SnapshotProfileRef;
use crate::world_commit::WorldCommitRef;
use crate::world_replay::WORLD_TRANSITION_TRACE_SCHEMA;
use crate::world_replay::WorldReplayBounds;
use crate::world_replay::WorldReplayProfile;
use crate::world_replay::WorldReplayProfileKind;
use crate::world_replay::WorldTransitionInput;
use crate::world_replay::WorldTransitionInputKind;
use crate::world_replay::WorldTransitionStep;
use crate::world_replay::WorldTransitionTrace;
use crate::world_replay::identify_world_transition_trace;
use crate::world_replay::validate_world_transition_trace;

const OBSERVATION_TRANSITION_POSITION: u64 = 0;
const PLACEHOLDER_DIGEST: &str = "blake3:0000000000000000000000000000000000000000000000000000000000000000";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldPromotionObservationCommitRequest {
    pub reservation: WorldReleaseReservation,
    pub attempt: WorldAttemptRecord,
    pub successor_commit: WorldCommitRef,
    pub logical_profile_ref: SnapshotProfileRef,
    pub observation_schema_ref: String,
    pub observation_byte_length: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldPromotionObservationCommitPlan {
    pub parent_commit: WorldCommitRef,
    pub successor_commit: WorldCommitRef,
    pub observation_ref: WorldReleaseObservationRef,
    pub trace: WorldTransitionTrace,
    pub mutates_promoted_commit: bool,
    pub grants_dispatch_authority: bool,
    pub external_completion_proven: bool,
    pub non_claims: Vec<String>,
}

// r[impl molten.world_promotion.observation_commit]
pub fn plan_world_promotion_observation_commit(
    request: &WorldPromotionObservationCommitRequest,
) -> Result<WorldPromotionObservationCommitPlan, Vec<WorldPromotionIssue>> {
    let observation_ref = validate_observation_request(request)?;
    let trace = build_observation_trace(request, &observation_ref)?;
    Ok(WorldPromotionObservationCommitPlan {
        parent_commit: request.reservation.candidate_head.clone(),
        successor_commit: request.successor_commit.clone(),
        observation_ref,
        trace,
        mutates_promoted_commit: false,
        grants_dispatch_authority: false,
        external_completion_proven: true,
        non_claims: observation_non_claims(),
    })
}

fn validate_observation_request(
    request: &WorldPromotionObservationCommitRequest,
) -> Result<WorldReleaseObservationRef, Vec<WorldPromotionIssue>> {
    let mut issues = Vec::with_capacity(MAX_WORLD_PROMOTION_DIAGNOSTICS);
    if request.attempt.reservation_ref != request.reservation.reservation_ref {
        issues.push(WorldPromotionIssue::ObservationReservationMismatch);
    }
    if !reservation_can_record_observation(request.reservation.state) {
        issues.push(WorldPromotionIssue::ReservationNotCommitted);
    }
    if request.attempt.state != WorldReleaseState::Acknowledged || !request.attempt.external_completion_proven {
        issues.push(WorldPromotionIssue::ObservationNotAcknowledged);
    }
    let observation_ref = request.attempt.observation_ref.clone();
    if observation_ref.is_none() {
        issues.push(WorldPromotionIssue::ObservationReferenceMissing);
    }
    if request.successor_commit == request.reservation.candidate_head {
        issues.push(WorldPromotionIssue::ObservationSuccessorUnchanged);
    }
    if !issues.is_empty() {
        issues.sort();
        issues.dedup();
        return Err(issues);
    }
    observation_ref.ok_or_else(|| vec![WorldPromotionIssue::ObservationReferenceMissing])
}

const fn reservation_can_record_observation(state: WorldReleaseState) -> bool {
    matches!(
        state,
        WorldReleaseState::Committed
            | WorldReleaseState::Claimed
            | WorldReleaseState::Attempting
            | WorldReleaseState::Observed
            | WorldReleaseState::Acknowledged
            | WorldReleaseState::Reconciled
    )
}

fn build_observation_trace(
    request: &WorldPromotionObservationCommitRequest,
    observation_ref: &WorldReleaseObservationRef,
) -> Result<WorldTransitionTrace, Vec<WorldPromotionIssue>> {
    let profile = WorldReplayProfile {
        kind: WorldReplayProfileKind::Logical,
        profile_ref: request.logical_profile_ref.clone(),
        cohort_ref: None,
        snapshot_descriptor_ref: None,
    };
    let mut trace = WorldTransitionTrace {
        schema: WORLD_TRANSITION_TRACE_SCHEMA.to_string(),
        trace_ref: PLACEHOLDER_DIGEST.to_string(),
        initial_commit: request.reservation.candidate_head.clone(),
        profile: profile.clone(),
        steps: vec![observation_step(request, observation_ref, profile.profile_ref)],
    };
    trace.trace_ref =
        identify_world_transition_trace(&trace).map_err(|_| vec![WorldPromotionIssue::ObservationTraceInvalid])?;
    if !validate_world_transition_trace(&trace, &WorldReplayBounds::default()).is_empty() {
        return Err(vec![WorldPromotionIssue::ObservationTraceInvalid]);
    }
    Ok(trace)
}

fn observation_step(
    request: &WorldPromotionObservationCommitRequest,
    observation_ref: &WorldReleaseObservationRef,
    profile_ref: SnapshotProfileRef,
) -> WorldTransitionStep {
    WorldTransitionStep {
        position: OBSERVATION_TRANSITION_POSITION,
        expected_parent: request.reservation.candidate_head.clone(),
        input: WorldTransitionInput {
            kind: WorldTransitionInputKind::RecordedEffect,
            input_ref: observation_ref.as_str().to_string(),
            schema_ref: request.observation_schema_ref.clone(),
            byte_length: request.observation_byte_length,
        },
        profile_ref,
        expected_successor: request.successor_commit.clone(),
    }
}

fn observation_non_claims() -> Vec<String> {
    let mut non_claims = promotion_non_claims();
    non_claims.push("an observation successor does not rewrite the promoted commit".to_string());
    non_claims.push("an observation successor does not grant dispatch authority".to_string());
    non_claims.push("a logical observation successor does not establish opaque replay equivalence".to_string());
    non_claims
}

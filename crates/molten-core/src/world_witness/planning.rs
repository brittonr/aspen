use super::bounds;
use super::domain;
use super::validation;
use crate::world_head;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StagedClaim {
    pub operation_id: String,
    pub transition: world_head::WorldHeadTransitionPlan,
    pub selected_profile: domain::AssuranceProfile,
    pub retained: domain::RetainedWitnessState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageInput {
    pub staged: StagedClaim,
    pub authenticated: bool,
    pub authority: world_head::WorldHeadAuthorityObservation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WitnessRequestPlan {
    pub operation_id: String,
    pub subject: domain::WitnessSubject,
}

// r[impl molten.world_witness.finalization]
pub fn plan_stage(input: &StageInput) -> Result<WitnessRequestPlan, domain::WitnessIssue> {
    bounds::validate_stage(&input.staged)?;
    let transition = &input.staged.transition;
    if input.staged.operation_id.is_empty() || !input.authenticated {
        return Err(domain::WitnessIssue::InvalidStage);
    }
    validate_transition(transition)?;
    validate_authority(&input.authority, transition)?;
    if matches!(input.staged.retained, domain::RetainedWitnessState::Missing) {
        return Err(domain::WitnessIssue::MissingHighestState);
    }
    if input.staged.selected_profile == domain::AssuranceProfile::LocalGenerationV1
        && matches!(input.staged.retained, domain::RetainedWitnessState::Present(_))
    {
        return Err(domain::WitnessIssue::ProfileDowngrade);
    }
    Ok(WitnessRequestPlan {
        operation_id: input.staged.operation_id.clone(),
        subject: subject(&input.staged),
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FinalizationInput {
    pub staged: StagedClaim,
    pub durable_stage: Option<StagedClaim>,
    pub current_head: Option<world_head::WorldHeadState>,
    pub authority: world_head::WorldHeadAuthorityObservation,
    pub currentness: domain::CurrentnessInput,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FinalizationPlan {
    pub transition: world_head::WorldHeadTransitionPlan,
    pub currentness: domain::CurrentnessAdmission,
    pub operation_id: String,
}

/// Must be evaluated from observations read inside the local mutation boundary.
// r[impl molten.world_witness.finalization]
pub fn plan_finalization(input: &FinalizationInput) -> Result<FinalizationPlan, domain::WitnessIssue> {
    bounds::validate_stage(&input.staged)?;
    if let Some(staged) = &input.durable_stage {
        bounds::validate_stage(staged)?;
    }
    bounds::validate(&input.currentness)?;
    if input.durable_stage.as_ref() != Some(&input.staged) {
        return Err(domain::WitnessIssue::InvalidStage);
    }
    if input.current_head != input.staged.transition.before {
        return Err(domain::WitnessIssue::LocalConflict);
    }
    validate_transition(&input.staged.transition)?;
    validate_authority(&input.authority, &input.staged.transition)?;
    if input.currentness.subject != subject(&input.staged)
        || input.currentness.operation_id != input.staged.operation_id
        || input.currentness.selected_profile != input.staged.selected_profile
        || input.currentness.retained != input.staged.retained
        || input.currentness.local_generation != input.current_head.as_ref().map_or(0, |head| head.generation)
    {
        return Err(domain::WitnessIssue::InvalidStage);
    }
    let currentness = validation::validate_currentness(&input.currentness)?;
    Ok(FinalizationPlan {
        transition: input.staged.transition.clone(),
        currentness,
        operation_id: input.staged.operation_id.clone(),
    })
}

fn validate_transition(transition: &world_head::WorldHeadTransitionPlan) -> Result<(), domain::WitnessIssue> {
    let generation = transition.before.as_ref().map_or(0, |before| before.generation);
    if generation.checked_add(1) != Some(transition.after.generation)
        || transition.before.as_ref().is_some_and(|before| {
            before.branch_id != transition.after.branch_id || before.branch_class != transition.after.branch_class
        })
    {
        return Err(domain::WitnessIssue::InvalidStage);
    }
    Ok(())
}

fn validate_authority(
    authority: &world_head::WorldHeadAuthorityObservation,
    transition: &world_head::WorldHeadTransitionPlan,
) -> Result<(), domain::WitnessIssue> {
    let generation = transition.before.as_ref().map_or(0, |before| before.generation);
    if !authority.admitted
        || authority.policy_ref != transition.after.policy_ref
        || authority.observed_generation != generation
    {
        return Err(domain::WitnessIssue::AuthorityDenied);
    }
    Ok(())
}

pub fn subject(staged: &StagedClaim) -> domain::WitnessSubject {
    domain::WitnessSubject {
        branch_id: staged.transition.after.branch_id.clone(),
        claim_ref: staged.transition.claim_ref.clone(),
        generation: staged.transition.after.generation,
    }
}

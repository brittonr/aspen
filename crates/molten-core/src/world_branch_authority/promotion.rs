use super::*;

const PROMOTION_ADMISSION_IDENTITY_DOMAIN: &str = "onixresearch.molten.world-branch-authority.promotion-admission.v1";

pub const WORLD_BRANCH_PROMOTION_ADMISSION_SCHEMA: &str = "molten.world-branch-promotion-admission.v1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldBranchPromotionReservationFacts {
    pub authority_plan_ref: String,
    pub promotion_plan_ref: String,
    pub reservation_ref: String,
    pub candidate_head_ref: String,
    pub capability_ref: String,
    pub reservation_committed: bool,
    pub complete_reservation_set: bool,
    pub reservation_matches_plan: bool,
    pub candidate_matches: bool,
    pub external_effects_completed: bool,
    pub dispatch_authorized: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldBranchPromotionReservationAdmission {
    pub schema: &'static str,
    pub admission_ref: String,
    pub authority_plan_ref: String,
    pub promotion_plan_ref: String,
    pub reservation_ref: String,
    pub candidate_head_ref: String,
    pub capability_ref: String,
    pub reservation_committed: bool,
    pub complete_reservation_set: bool,
    pub dispatch_authorized: bool,
}

// r[impl molten.world_branch_authority.activation]
pub fn plan_world_branch_promotion_admission(
    plan: &WorldBranchAuthorityPlan,
    facts: &WorldBranchPromotionReservationFacts,
) -> Result<WorldBranchPromotionReservationAdmission, WorldBranchAuthorityDiagnostic> {
    validate_promotion_facts(plan, facts)?;
    let mut admission = WorldBranchPromotionReservationAdmission {
        schema: WORLD_BRANCH_PROMOTION_ADMISSION_SCHEMA,
        admission_ref: String::new(),
        authority_plan_ref: facts.authority_plan_ref.clone(),
        promotion_plan_ref: facts.promotion_plan_ref.clone(),
        reservation_ref: facts.reservation_ref.clone(),
        candidate_head_ref: facts.candidate_head_ref.clone(),
        capability_ref: facts.capability_ref.clone(),
        reservation_committed: facts.reservation_committed,
        complete_reservation_set: facts.complete_reservation_set,
        dispatch_authorized: false,
    };
    admission.admission_ref = promotion_admission_identity(&admission)?;
    debug_assert_eq!(validate_world_branch_promotion_admission(plan, &admission), Ok(()));
    Ok(admission)
}

pub fn validate_world_branch_promotion_admission(
    plan: &WorldBranchAuthorityPlan,
    admission: &WorldBranchPromotionReservationAdmission,
) -> Result<(), WorldBranchAuthorityDiagnostic> {
    if plan.mode != Some(WorldBranchMode::PromotionGated) || !plan.allowed {
        return Err(WorldBranchAuthorityDiagnostic::NonBranchable);
    }
    if admission.schema != WORLD_BRANCH_PROMOTION_ADMISSION_SCHEMA
        || [
            admission.admission_ref.as_str(),
            admission.authority_plan_ref.as_str(),
            admission.promotion_plan_ref.as_str(),
            admission.reservation_ref.as_str(),
            admission.candidate_head_ref.as_str(),
            admission.capability_ref.as_str(),
        ]
        .iter()
        .any(|reference| !valid_content_ref(reference))
    {
        return Err(WorldBranchAuthorityDiagnostic::InvalidInput);
    }
    if admission.authority_plan_ref != plan.plan_ref || admission.capability_ref != plan.capability_ref {
        return Err(WorldBranchAuthorityDiagnostic::ObservationMismatch);
    }
    if admission.dispatch_authorized {
        return Err(WorldBranchAuthorityDiagnostic::PromotionDispatchOverclaim);
    }
    if !admission.reservation_committed || !admission.complete_reservation_set {
        return Err(WorldBranchAuthorityDiagnostic::PromotionReservationMissing);
    }
    if promotion_admission_identity(admission)? != admission.admission_ref {
        return Err(WorldBranchAuthorityDiagnostic::ObservationMismatch);
    }
    Ok(())
}

fn validate_promotion_facts(
    plan: &WorldBranchAuthorityPlan,
    facts: &WorldBranchPromotionReservationFacts,
) -> Result<(), WorldBranchAuthorityDiagnostic> {
    if plan.mode != Some(WorldBranchMode::PromotionGated) || !plan.allowed {
        return Err(WorldBranchAuthorityDiagnostic::NonBranchable);
    }
    if facts.authority_plan_ref != plan.plan_ref || facts.capability_ref != plan.capability_ref {
        return Err(WorldBranchAuthorityDiagnostic::ObservationMismatch);
    }
    if [
        facts.authority_plan_ref.as_str(),
        facts.promotion_plan_ref.as_str(),
        facts.reservation_ref.as_str(),
        facts.candidate_head_ref.as_str(),
        facts.capability_ref.as_str(),
    ]
    .iter()
    .any(|reference| !valid_content_ref(reference))
    {
        return Err(WorldBranchAuthorityDiagnostic::InvalidInput);
    }
    if facts.dispatch_authorized {
        return Err(WorldBranchAuthorityDiagnostic::PromotionDispatchOverclaim);
    }
    if !facts.reservation_committed
        || !facts.complete_reservation_set
        || !facts.reservation_matches_plan
        || !facts.candidate_matches
        || facts.external_effects_completed
    {
        return Err(WorldBranchAuthorityDiagnostic::PromotionReservationMissing);
    }
    Ok(())
}

fn promotion_admission_identity(
    admission: &WorldBranchPromotionReservationAdmission,
) -> Result<String, WorldBranchAuthorityDiagnostic> {
    let mut hasher = blake3::Hasher::new_derive_key(PROMOTION_ADMISSION_IDENTITY_DOMAIN);
    for value in [
        admission.schema,
        admission.authority_plan_ref.as_str(),
        admission.promotion_plan_ref.as_str(),
        admission.reservation_ref.as_str(),
        admission.candidate_head_ref.as_str(),
        admission.capability_ref.as_str(),
    ] {
        update_identity_text(&mut hasher, value)?;
    }
    for value in [
        admission.reservation_committed,
        admission.complete_reservation_set,
        admission.dispatch_authorized,
    ] {
        hasher.update(&[u8::from(value)]);
    }
    Ok(format!("blake3:{}", hasher.finalize().to_hex()))
}

fn update_identity_text(hasher: &mut blake3::Hasher, value: &str) -> Result<(), WorldBranchAuthorityDiagnostic> {
    let length = u64::try_from(value.len()).map_err(|_| WorldBranchAuthorityDiagnostic::InvalidInput)?;
    hasher.update(&length.to_le_bytes());
    hasher.update(value.as_bytes());
    Ok(())
}

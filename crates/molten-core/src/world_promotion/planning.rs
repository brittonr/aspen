use std::collections::BTreeSet;

use super::*;
use crate::world_head::WorldBranchClass;
use crate::world_head::WorldHeadState;

mod transaction;

pub(in crate::world_promotion) use transaction::identity;
pub(in crate::world_promotion) use transaction::transaction_current_facts;
use transaction::transactional_plan;

const PROMOTION_PLAN_CONTEXT: &str = "onixresearch.molten.world-promotion.plan.v1";
const RELEASE_RESERVATION_CONTEXT: &str = "onixresearch.molten.world-promotion.reservation.v1";
pub(super) const TRANSACTION_ATTEMPT_CONTEXT: &str = "onixresearch.molten.world-promotion.transaction-attempt.v1";

// r[impl molten.world_promotion.plan]
// r[impl molten.world_promotion.transaction]
pub fn plan_world_promotion(request: &WorldPromotionRequest) -> Result<WorldPromotionPlan, Vec<WorldPromotionIssue>> {
    let mut issues = validate_request(request);
    if !issues.is_empty() {
        issues.sort();
        issues.dedup();
        return Err(issues);
    }
    let successor_generation = request
        .expected_generation
        .checked_add(1)
        .ok_or_else(|| vec![WorldPromotionIssue::GenerationOverflow])?;
    let mut intents = request.intents.clone();
    intents.sort_by(|left, right| left.intent_ref.cmp(&right.intent_ref));
    let plan_ref = identify_promotion(request, &intents)?;
    let mut reservations = intents
        .iter()
        .filter(|intent| intent.release_class == Some(WorldIntentReleaseClass::Release))
        .map(|intent| release_reservation(request, &plan_ref, successor_generation, intent))
        .collect::<Result<Vec<_>, _>>()?;
    reservations.sort_by(|left, right| left.reservation_ref.cmp(&right.reservation_ref));
    if reservations.len() > request.bounds.max_reservations {
        return Err(vec![WorldPromotionIssue::ReservationLimitExceeded]);
    }
    let before = WorldHeadState {
        branch_id: request.branch_id.clone(),
        branch_class: request.branch_class,
        head: request.expected_head.clone(),
        generation: request.expected_generation,
        policy_ref: request.policy_ref.clone(),
    };
    let after = WorldHeadState {
        branch_id: request.branch_id.clone(),
        branch_class: WorldBranchClass::Release,
        head: request.candidate_head.clone(),
        generation: successor_generation,
        policy_ref: request.policy_ref.clone(),
    };
    let transaction = transactional_plan(request, &plan_ref, &reservations, successor_generation)?;
    Ok(WorldPromotionPlan {
        plan_ref,
        operation_ref: request.operation_ref.clone(),
        authority_ref: request.authority.authority_ref.clone(),
        before,
        after,
        intents,
        reservations,
        transaction,
        external_effects_completed: false,
        non_claims: promotion_non_claims(),
    })
}

pub fn validate_promotion_transaction(
    plan: &WorldPromotionPlan,
    facts: &WorldPromotionTransactionFacts,
) -> Result<(), Vec<WorldPromotionIssue>> {
    let mut issues = Vec::with_capacity(MAX_WORLD_PROMOTION_DIAGNOSTICS);
    if facts.observed_head.as_ref() != Some(&plan.before) {
        issues.push(WorldPromotionIssue::AuthorityGenerationMismatch);
    }
    if facts.authority_ref != plan.authority_ref || !facts.authority_admitted {
        issues.push(WorldPromotionIssue::AuthorityDenied);
    }
    if facts.authority_generation != plan.before.generation {
        issues.push(WorldPromotionIssue::AuthorityGenerationMismatch);
    }
    if facts.policy_ref != plan.after.policy_ref {
        issues.push(WorldPromotionIssue::AuthorityPolicyMismatch);
    }
    if !facts.intent_closure_complete {
        issues.push(WorldPromotionIssue::IntentClosureIncomplete);
    }
    if validate_reservation_set(plan, &facts.reservation_refs).is_err() {
        issues.push(WorldPromotionIssue::ReservationSetMismatch);
    }
    if issues.is_empty() {
        Ok(())
    } else {
        issues.sort();
        issues.dedup();
        Err(issues)
    }
}

pub fn validate_reservation_set(
    plan: &WorldPromotionPlan,
    observed: &[WorldReleaseReservationRef],
) -> Result<(), WorldPromotionIssue> {
    let mut expected =
        plan.reservations.iter().map(|reservation| reservation.reservation_ref.clone()).collect::<Vec<_>>();
    let mut observed = observed.to_vec();
    expected.sort();
    observed.sort();
    if expected == observed {
        Ok(())
    } else {
        Err(WorldPromotionIssue::ReservationSetMismatch)
    }
}

fn validate_request(request: &WorldPromotionRequest) -> Vec<WorldPromotionIssue> {
    let mut issues = Vec::with_capacity(MAX_WORLD_PROMOTION_DIAGNOSTICS);
    if request.bounds.max_intents == 0
        || request.bounds.max_intents > MAX_WORLD_PROMOTION_INTENTS
        || request.bounds.max_reservations > request.bounds.max_intents
    {
        issues.push(WorldPromotionIssue::InvalidBounds);
    }
    if request.intents.len() > request.bounds.max_intents {
        issues.push(WorldPromotionIssue::IntentLimitExceeded);
    }
    if !request.intent_closure_complete {
        issues.push(WorldPromotionIssue::IntentClosureIncomplete);
    }
    if request.simulation_only {
        issues.push(WorldPromotionIssue::SimulationBranchDenied);
    }
    if request.branch_class != WorldBranchClass::Candidate {
        issues.push(WorldPromotionIssue::BranchClassDenied);
    }
    if request.expected_head == request.candidate_head {
        issues.push(WorldPromotionIssue::CandidateEqualsActive);
    }
    if !request.authority.admitted {
        issues.push(WorldPromotionIssue::AuthorityDenied);
    }
    if request.authority.policy_ref != request.policy_ref {
        issues.push(WorldPromotionIssue::AuthorityPolicyMismatch);
    }
    if request.authority.observed_generation != request.expected_generation {
        issues.push(WorldPromotionIssue::AuthorityGenerationMismatch);
    }
    validate_intents(&request.intents, &mut issues);
    issues
}

#[allow(
    tigerstyle::borrowed_argument_types,
    reason = "the bounded validator appends typed issues to one preallocated local sink"
)]
fn validate_intents(intents: &[WorldEffectIntent], issues: &mut Vec<WorldPromotionIssue>) {
    let mut intent_refs = BTreeSet::new();
    let mut semantic_refs = BTreeSet::new();
    for intent in intents {
        if !intent_refs.insert(intent.intent_ref.clone()) {
            issues.push(WorldPromotionIssue::DuplicateIntent(intent.intent_ref.as_str().to_string()));
        }
        if !semantic_refs.insert(intent.semantic_ref.clone()) {
            issues.push(WorldPromotionIssue::DuplicateSemanticIntent(intent.semantic_ref.as_str().to_string()));
        }
        if intent.release_class.is_none() {
            issues.push(WorldPromotionIssue::IntentUnclassified(intent.intent_ref.as_str().to_string()));
        }
    }
}

fn identify_promotion(
    request: &WorldPromotionRequest,
    intents: &[WorldEffectIntent],
) -> Result<WorldPromotionPlanRef, Vec<WorldPromotionIssue>> {
    let mut hasher = blake3::Hasher::new_derive_key(PROMOTION_PLAN_CONTEXT);
    update(&mut hasher, request.operation_ref.as_str())?;
    update(&mut hasher, request.branch_id.as_str())?;
    update(&mut hasher, request.branch_class.as_str())?;
    update(&mut hasher, request.expected_head.as_str())?;
    update(&mut hasher, request.candidate_head.as_str())?;
    hasher.update(&request.expected_generation.to_be_bytes());
    update(&mut hasher, request.policy_ref.as_str())?;
    update(&mut hasher, request.authority.authority_ref.as_str())?;
    for intent in intents {
        update(&mut hasher, intent.intent_ref.as_str())?;
        update(&mut hasher, intent.semantic_ref.as_str())?;
        update(&mut hasher, intent.handler_ref.as_str())?;
        update(&mut hasher, intent.adapter_ref.as_str())?;
        let release_class = intent.release_class.ok_or_else(|| {
            vec![WorldPromotionIssue::IntentUnclassified(
                intent.intent_ref.as_str().to_string(),
            )]
        })?;
        update(&mut hasher, release_class.as_str())?;
    }
    WorldPromotionPlanRef::new(format!("blake3:{}", hasher.finalize().to_hex()))
        .map_err(|error| vec![WorldPromotionIssue::TransactionalMapping(format!("{error:?}"))])
}

fn release_reservation(
    request: &WorldPromotionRequest,
    plan_ref: &WorldPromotionPlanRef,
    generation: u64,
    intent: &WorldEffectIntent,
) -> Result<WorldReleaseReservation, Vec<WorldPromotionIssue>> {
    let reservation_ref = derived_reference(RELEASE_RESERVATION_CONTEXT, &[
        plan_ref.as_str(),
        request.operation_ref.as_str(),
        request.candidate_head.as_str(),
        intent.intent_ref.as_str(),
        intent.semantic_ref.as_str(),
    ])?;
    Ok(WorldReleaseReservation {
        reservation_ref: WorldReleaseReservationRef::new(reservation_ref)
            .map_err(|error| vec![WorldPromotionIssue::TransactionalMapping(format!("{error:?}"))])?,
        promotion_ref: plan_ref.clone(),
        operation_ref: request.operation_ref.clone(),
        candidate_head: request.candidate_head.clone(),
        intent_ref: intent.intent_ref.clone(),
        semantic_ref: intent.semantic_ref.clone(),
        handler_ref: intent.handler_ref.clone(),
        adapter_ref: intent.adapter_ref.clone(),
        generation,
        state: WorldReleaseState::Planned,
    })
}

pub(super) fn expected_attempt_ref(
    reservation: &WorldReleaseReservation,
) -> Result<WorldReleaseAttemptRef, Vec<WorldPromotionIssue>> {
    WorldReleaseAttemptRef::new(derived_reference(TRANSACTION_ATTEMPT_CONTEXT, &[
        reservation.reservation_ref.as_str(),
        reservation.operation_ref.as_str(),
    ])?)
    .map_err(|error| vec![WorldPromotionIssue::TransactionalMapping(format!("{error:?}"))])
}

fn derived_reference(context: &str, fields: &[&str]) -> Result<String, Vec<WorldPromotionIssue>> {
    let mut hasher = blake3::Hasher::new_derive_key(context);
    for field in fields {
        update(&mut hasher, field)?;
    }
    Ok(format!("blake3:{}", hasher.finalize().to_hex()))
}

fn update(hasher: &mut blake3::Hasher, value: &str) -> Result<(), Vec<WorldPromotionIssue>> {
    let length = u64::try_from(value.len())
        .map_err(|_| vec![WorldPromotionIssue::TransactionalMapping("frame-length".to_string())])?;
    hasher.update(&length.to_be_bytes());
    hasher.update(value.as_bytes());
    Ok(())
}

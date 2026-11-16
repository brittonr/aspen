use super::super::*;
use super::promotion_request;
use super::reference;
use crate::world_head::WorldBranchId;

fn operator_for(plan: &WorldPromotionPlan) -> WorldOperatorAuthority {
    WorldOperatorAuthority {
        branch_id: plan.after.branch_id.clone(),
        authority_ref: plan.authority_ref.clone(),
        policy_ref: plan.after.policy_ref.clone(),
        generation: plan.after.generation,
        admitted: true,
    }
}

fn transaction_facts(plan: &WorldPromotionPlan) -> WorldPromotionTransactionFacts {
    WorldPromotionTransactionFacts {
        observed_head: Some(plan.before.clone()),
        authority_ref: plan.authority_ref.clone(),
        authority_admitted: true,
        authority_generation: plan.before.generation,
        policy_ref: plan.after.policy_ref.clone(),
        intent_closure_complete: true,
        intent_material_available: true,
        reservation_refs: plan.reservations.iter().map(|reservation| reservation.reservation_ref.clone()).collect(),
    }
}

// r[verify molten.world_promotion.reconciliation]
#[test]
fn current_operator_authority_admits_deny_retry_and_abandon_for_the_original_plan() {
    let plan = plan_world_promotion(&promotion_request()).expect("plan");
    let reservation = &plan.reservations[0];
    let operator = operator_for(&plan);
    assert_eq!(validate_operator_authority(&operator, reservation, &plan), Ok(()));
}

// r[verify molten.world_promotion.reconciliation]
#[test]
fn stale_revoked_or_foreign_operator_authority_is_denied() {
    let plan = plan_world_promotion(&promotion_request()).expect("plan");
    let reservation = &plan.reservations[0];

    let mut revoked = operator_for(&plan);
    revoked.admitted = false;
    assert_eq!(
        validate_operator_authority(&revoked, reservation, &plan),
        Err(vec![WorldPromotionIssue::AuthorityDenied])
    );

    let mut stale = operator_for(&plan);
    stale.generation = plan.before.generation;
    assert_eq!(
        validate_operator_authority(&stale, reservation, &plan),
        Err(vec![WorldPromotionIssue::AuthorityGenerationMismatch])
    );

    let mut foreign = operator_for(&plan);
    foreign.authority_ref = WorldPromotionAuthorityRef::new(reference("other-authority")).expect("authority");
    assert_eq!(
        validate_operator_authority(&foreign, reservation, &plan),
        Err(vec![WorldPromotionIssue::AuthorityDenied])
    );

    let mut other_branch = operator_for(&plan);
    other_branch.branch_id = WorldBranchId::new("other").expect("branch");
    assert_eq!(
        validate_operator_authority(&other_branch, reservation, &plan),
        Err(vec![WorldPromotionIssue::AuthorityDenied])
    );

    let mut other_policy = operator_for(&plan);
    other_policy.policy_ref = crate::world_head::WorldHeadPolicyRef::new(reference("other-policy")).expect("policy");
    assert_eq!(
        validate_operator_authority(&other_policy, reservation, &plan),
        Err(vec![WorldPromotionIssue::AuthorityPolicyMismatch])
    );

    let mut other_plan_request = promotion_request();
    other_plan_request.operation_ref =
        WorldPromotionOperationRef::new(reference("other-operation")).expect("operation");
    let other_plan = plan_world_promotion(&other_plan_request).expect("other plan");
    assert_eq!(
        validate_operator_authority(&operator_for(&plan), reservation, &other_plan),
        Err(vec![WorldPromotionIssue::ReservationSetMismatch])
    );
}

// r[verify molten.world_promotion.transaction]
#[test]
fn intent_material_must_match_every_plan_intent_exactly() {
    let plan = plan_world_promotion(&promotion_request()).expect("plan");
    let mut reversed = plan.intents.clone();
    reversed.reverse();
    assert!(intent_material_matches(&plan, &reversed));

    assert!(!intent_material_matches(&plan, &plan.intents[..1]));
    let duplicated = vec![plan.intents[0].clone(), plan.intents[0].clone()];
    assert!(!intent_material_matches(&plan, &duplicated));
    let mut other_adapter = plan.intents.clone();
    other_adapter[0].adapter_ref = WorldPromotionAdapterRef::new(reference("other-adapter")).expect("adapter");
    assert!(!intent_material_matches(&plan, &other_adapter));
}

// r[verify molten.world_promotion.transaction]
#[test]
fn unavailable_intent_material_denies_the_transaction() {
    let plan = plan_world_promotion(&promotion_request()).expect("plan");
    assert_eq!(validate_promotion_transaction(&plan, &transaction_facts(&plan)), Ok(()));
    let mut facts = transaction_facts(&plan);
    facts.intent_material_available = false;
    assert_eq!(
        validate_promotion_transaction(&plan, &facts),
        Err(vec![WorldPromotionIssue::IntentMaterialUnavailable])
    );
}

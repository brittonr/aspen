use super::super::*;
use super::operator::EXPIRES_AT;
use super::operator::attempt_ref;
use super::operator::deny_target;
use super::operator::grant;
use super::operator::predecessor_live;
use super::operator::promote_facts;
use super::operator::promoted_live;
use super::operator::transaction_facts;
use super::promotion_request;
use super::reference;
use crate::world_commit::WorldCommitRef;

// r[verify molten.world_commit.promotion_authority_action_and_canonical_binding]
#[test]
fn deny_or_abandon_grant_cannot_authorize_promotion() {
    let plan = plan_world_promotion(&promotion_request()).expect("plan");
    let live = predecessor_live(&plan);
    assert_eq!(validate_promotion_transaction(&plan, &promote_facts(&plan), &live), Ok(()));
    let reservation_ref = plan.reservations[0].reservation_ref.clone();
    for target in [
        deny_target(&plan),
        WorldPromotionGrantTarget::PlanRetry {
            reservation_ref: reservation_ref.clone(),
            attempt_ref: attempt_ref("attempt"),
        },
        WorldPromotionGrantTarget::AbandonUncertain {
            reservation_ref: reservation_ref.clone(),
            attempt_ref: attempt_ref("attempt"),
        },
    ] {
        // Issued at the promotion's predecessor generation, so only the action differs.
        let facts = transaction_facts(&plan, grant(&plan, plan.before.generation, target));
        assert_eq!(
            validate_promotion_transaction(&plan, &facts, &live),
            Err(vec![WorldPromotionIssue::AuthorityActionMismatch])
        );
    }
    let other_candidate = WorldPromotionGrantTarget::Promote {
        candidate_head: WorldCommitRef::new(reference("other-candidate")).expect("candidate"),
    };
    let facts = transaction_facts(&plan, grant(&plan, plan.before.generation, other_candidate));
    assert_eq!(
        validate_promotion_transaction(&plan, &facts, &live),
        Err(vec![WorldPromotionIssue::AuthorityTargetMismatch])
    );
}

// r[verify molten.world_commit.promotion_authority_action_and_canonical_binding]
// r[verify molten.world_commit.promotion_atomic_dispatch_and_readback]
#[test]
fn promotion_grant_is_denied_when_expired_consumed_or_the_live_head_moved() {
    let plan = plan_world_promotion(&promotion_request()).expect("plan");
    let mut expired = promote_facts(&plan);
    expired.observed_unix_seconds = EXPIRES_AT;
    assert_eq!(
        validate_promotion_transaction(&plan, &expired, &predecessor_live(&plan)),
        Err(vec![WorldPromotionIssue::AuthorityExpired])
    );
    let consumed = WorldPromotionLiveFacts {
        grant_nonce_consumed: true,
        ..predecessor_live(&plan)
    };
    assert_eq!(
        validate_promotion_transaction(&plan, &promote_facts(&plan), &consumed),
        Err(vec![WorldPromotionIssue::AuthorityNonceConsumed])
    );
    // The live head read inside the write transaction, not an earlier observation, decides.
    assert_eq!(
        validate_promotion_transaction(&plan, &promote_facts(&plan), &promoted_live(&plan)),
        Err(vec![WorldPromotionIssue::AuthorityGenerationMismatch])
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
    let live = predecessor_live(&plan);
    assert_eq!(validate_promotion_transaction(&plan, &promote_facts(&plan), &live), Ok(()));
    let mut facts = promote_facts(&plan);
    facts.intent_material_available = false;
    assert_eq!(
        validate_promotion_transaction(&plan, &facts, &live),
        Err(vec![WorldPromotionIssue::IntentMaterialUnavailable])
    );
}

#[test]
fn grant_nonce_and_action_spellings_are_closed() {
    assert!(WorldPromotionGrantNonce::new("ab").is_err());
    assert!(WorldPromotionGrantNonce::new("G".repeat(64)).is_err());
    assert!(WorldPromotionGrantNonce::new("a".repeat(64)).is_ok());
    for action in [
        WorldPromotionGrantAction::Promote,
        WorldPromotionGrantAction::DenyReservation,
        WorldPromotionGrantAction::PlanRetry,
        WorldPromotionGrantAction::AbandonUncertain,
    ] {
        assert_eq!(WorldPromotionGrantAction::parse(action.as_str()), Some(action));
    }
    assert_eq!(WorldPromotionGrantAction::parse("deny"), None);
}

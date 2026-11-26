use super::*;
use crate::world_promotion::SignedIntentClosure;

pub const WORLD_PROMOTION_GRANT_SCHEMA: &str = "molten.world-promotion.authority-grant.v2";
pub const WORLD_PROMOTION_INTENT_CLOSURE_SCHEMA: &str = "molten.world-promotion.intent-closure.v2";
const GRANT_RECORD: &str = "world-promotion-authority-grant";
const CLOSURE_RECORD: &str = "world-promotion-intent-closure";

/// The canonical packed Preserves record a promotion or operator grant is signed over. Every
/// field is length-delimited by the encoding, and the target record's label is the grant's
/// action, so distinct grants never share signing bytes.
// r[impl molten.world_commit.promotion_authority_action_and_canonical_binding]
pub fn canonical_promotion_grant(grant: &WorldPromotionGrant) -> Result<CanonicalWorldPromotionRecord> {
    canonical(crate::preserves_rail::record(GRANT_RECORD, vec![
        crate::preserves_rail::string(WORLD_PROMOTION_GRANT_SCHEMA),
        named("branch-id", string(grant.branch_id.as_str())),
        named("authority-ref", string(grant.authority_ref.as_str())),
        named("policy-ref", string(grant.policy_ref.as_str())),
        named("generation", number(grant.generation)),
        named("admitted", boolean(grant.admitted)),
        named("target", grant_target(&grant.target)),
        named("nonce", string(grant.nonce.as_str())),
        named("expires-at", number(grant.expires_at)),
    ]))
}

/// The canonical packed Preserves record an intent closure is signed over.
// r[impl molten.world_commit.promotion_authority_action_and_canonical_binding]
pub fn canonical_intent_closure(closure: &SignedIntentClosure) -> Result<CanonicalWorldPromotionRecord> {
    canonical(crate::preserves_rail::record(CLOSURE_RECORD, vec![
        crate::preserves_rail::string(WORLD_PROMOTION_INTENT_CLOSURE_SCHEMA),
        named("operation-ref", string(&closure.operation_ref)),
        named("branch-id", string(&closure.branch_id)),
        named("candidate-head", string(&closure.candidate_head)),
        named("expected-generation", number(closure.expected_generation)),
        named("complete", boolean(closure.complete)),
        named("intent-refs", sequence(closure.intent_refs.iter().map(string).collect())),
    ]))
}

fn grant_target(target: &WorldPromotionGrantTarget) -> IOValue {
    let fields = match target {
        WorldPromotionGrantTarget::Promote { candidate_head } => {
            vec![named("candidate-head", string(candidate_head.as_str()))]
        }
        WorldPromotionGrantTarget::DenyReservation { reservation_ref } => {
            vec![named("reservation-ref", string(reservation_ref.as_str()))]
        }
        WorldPromotionGrantTarget::PlanRetry {
            reservation_ref,
            attempt_ref,
        }
        | WorldPromotionGrantTarget::AbandonUncertain {
            reservation_ref,
            attempt_ref,
        } => vec![
            named("reservation-ref", string(reservation_ref.as_str())),
            named("attempt-ref", string(attempt_ref.as_str())),
        ],
    };
    crate::preserves_rail::record(target.action().as_str(), fields)
}

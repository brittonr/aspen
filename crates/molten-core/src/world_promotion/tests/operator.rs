use super::super::*;
use super::promotion_request;
use super::reference;
use crate::world_commit::WorldCommitRef;
use crate::world_head::WorldBranchId;
use crate::world_head::WorldHeadPolicyRef;

const OBSERVED_UNIX_SECONDS: u64 = 1_000;
pub(super) const EXPIRES_AT: u64 = 2_000;

fn nonce(label: &str) -> WorldPromotionGrantNonce {
    WorldPromotionGrantNonce::new(blake3::hash(label.as_bytes()).to_hex().to_string()).expect("nonce")
}

pub(super) fn grant(
    plan: &WorldPromotionPlan,
    generation: u64,
    target: WorldPromotionGrantTarget,
) -> WorldPromotionGrant {
    WorldPromotionGrant {
        branch_id: plan.after.branch_id.clone(),
        authority_ref: plan.authority_ref.clone(),
        policy_ref: plan.after.policy_ref.clone(),
        generation,
        admitted: true,
        target,
        nonce: nonce("grant"),
        expires_at: EXPIRES_AT,
    }
}

pub(super) fn deny_target(plan: &WorldPromotionPlan) -> WorldPromotionGrantTarget {
    WorldPromotionGrantTarget::DenyReservation {
        reservation_ref: plan.reservations[0].reservation_ref.clone(),
    }
}

pub(super) fn attempt_ref(label: &str) -> WorldReleaseAttemptRef {
    WorldReleaseAttemptRef::new(reference(label)).expect("attempt")
}

fn promote_target(plan: &WorldPromotionPlan) -> WorldPromotionGrantTarget {
    WorldPromotionGrantTarget::Promote {
        candidate_head: plan.after.head.clone(),
    }
}

/// The live facts right after the plan committed: the head is the plan's successor.
pub(super) fn promoted_live(plan: &WorldPromotionPlan) -> WorldPromotionLiveFacts {
    WorldPromotionLiveFacts {
        head: Some(plan.after.clone()),
        grant_nonce_consumed: false,
    }
}

pub(super) fn predecessor_live(plan: &WorldPromotionPlan) -> WorldPromotionLiveFacts {
    WorldPromotionLiveFacts {
        head: Some(plan.before.clone()),
        grant_nonce_consumed: false,
    }
}

fn check(
    plan: &WorldPromotionPlan,
    grant: &WorldPromotionGrant,
    requested: &WorldPromotionGrantTarget,
    live: &WorldPromotionLiveFacts,
    observed_unix_seconds: u64,
) -> Result<(), Vec<WorldPromotionIssue>> {
    validate_operator_authority(WorldOperatorAuthorityInput {
        grant,
        requested,
        reservation: &plan.reservations[0],
        plan,
        live,
        observed_unix_seconds,
    })
}

pub(super) fn transaction_facts(
    plan: &WorldPromotionPlan,
    grant: WorldPromotionGrant,
) -> WorldPromotionTransactionFacts {
    WorldPromotionTransactionFacts {
        grant,
        observed_unix_seconds: OBSERVED_UNIX_SECONDS,
        intent_closure_complete: true,
        intent_material_available: true,
        reservation_refs: plan.reservations.iter().map(|reservation| reservation.reservation_ref.clone()).collect(),
    }
}

pub(super) fn promote_facts(plan: &WorldPromotionPlan) -> WorldPromotionTransactionFacts {
    transaction_facts(plan, grant(plan, plan.before.generation, promote_target(plan)))
}

// r[verify molten.world_promotion.reconciliation]
// r[verify molten.world_commit.promotion_authority_action_and_canonical_binding]
#[test]
fn current_operator_grant_admits_exactly_its_own_action_and_target() {
    let plan = plan_world_promotion(&promotion_request()).expect("plan");
    let live = promoted_live(&plan);
    let reservation_ref = plan.reservations[0].reservation_ref.clone();
    let targets = [
        deny_target(&plan),
        WorldPromotionGrantTarget::PlanRetry {
            reservation_ref: reservation_ref.clone(),
            attempt_ref: attempt_ref("attempt"),
        },
        WorldPromotionGrantTarget::AbandonUncertain {
            reservation_ref,
            attempt_ref: attempt_ref("attempt"),
        },
    ];
    for target in &targets {
        let operator = grant(&plan, plan.after.generation, target.clone());
        assert_eq!(check(&plan, &operator, target, &live, OBSERVED_UNIX_SECONDS), Ok(()));
        for other in targets.iter().filter(|other| *other != target) {
            assert_eq!(
                check(&plan, &operator, other, &live, OBSERVED_UNIX_SECONDS),
                Err(vec![WorldPromotionIssue::AuthorityActionMismatch]),
                "a {:?} grant must not authorize {:?}",
                target.action(),
                other.action()
            );
        }
    }
}

// r[verify molten.world_commit.promotion_authority_action_and_canonical_binding]
#[test]
fn operator_grant_for_another_reservation_or_attempt_is_a_target_mismatch() {
    let plan = plan_world_promotion(&promotion_request()).expect("plan");
    let live = promoted_live(&plan);
    let foreign_reservation = WorldReleaseReservationRef::new(reference("other-reservation")).expect("reservation");
    let deny_other = grant(&plan, plan.after.generation, WorldPromotionGrantTarget::DenyReservation {
        reservation_ref: foreign_reservation,
    });
    assert_eq!(
        check(&plan, &deny_other, &deny_target(&plan), &live, OBSERVED_UNIX_SECONDS),
        Err(vec![WorldPromotionIssue::AuthorityTargetMismatch])
    );
    let reservation_ref = plan.reservations[0].reservation_ref.clone();
    let abandon_other = grant(&plan, plan.after.generation, WorldPromotionGrantTarget::AbandonUncertain {
        reservation_ref: reservation_ref.clone(),
        attempt_ref: attempt_ref("other-attempt"),
    });
    let requested = WorldPromotionGrantTarget::AbandonUncertain {
        reservation_ref,
        attempt_ref: attempt_ref("attempt"),
    };
    assert_eq!(
        check(&plan, &abandon_other, &requested, &live, OBSERVED_UNIX_SECONDS),
        Err(vec![WorldPromotionIssue::AuthorityTargetMismatch])
    );
}

// r[verify molten.world_commit.promotion_authority_action_and_canonical_binding]
#[test]
fn expired_or_consumed_operator_grant_is_denied() {
    let plan = plan_world_promotion(&promotion_request()).expect("plan");
    let target = deny_target(&plan);
    let operator = grant(&plan, plan.after.generation, target.clone());
    let live = promoted_live(&plan);
    assert_eq!(check(&plan, &operator, &target, &live, EXPIRES_AT - 1), Ok(()));
    assert_eq!(
        check(&plan, &operator, &target, &live, EXPIRES_AT),
        Err(vec![WorldPromotionIssue::AuthorityExpired])
    );
    let consumed = WorldPromotionLiveFacts {
        grant_nonce_consumed: true,
        ..promoted_live(&plan)
    };
    assert_eq!(
        check(&plan, &operator, &target, &consumed, OBSERVED_UNIX_SECONDS),
        Err(vec![WorldPromotionIssue::AuthorityNonceConsumed])
    );
}

// r[verify molten.world_commit.promotion_authority_action_and_canonical_binding]
#[test]
fn operator_grant_must_match_the_live_branch_head() {
    let plan = plan_world_promotion(&promotion_request()).expect("plan");
    let target = deny_target(&plan);
    let operator = grant(&plan, plan.after.generation, target.clone());

    // A later promotion advanced the branch: the grant at the reservation's generation is stale.
    let mut advanced = plan.after.clone();
    advanced.generation = plan.after.generation + 1;
    advanced.head = WorldCommitRef::new(reference("later-candidate")).expect("head");
    let advanced_live = WorldPromotionLiveFacts {
        head: Some(advanced.clone()),
        grant_nonce_consumed: false,
    };
    assert_eq!(
        check(&plan, &operator, &target, &advanced_live, OBSERVED_UNIX_SECONDS),
        Err(vec![WorldPromotionIssue::AuthorityGenerationMismatch])
    );
    // A grant issued at the live generation is current for the older reservation.
    let current = grant(&plan, advanced.generation, target.clone());
    assert_eq!(check(&plan, &current, &target, &advanced_live, OBSERVED_UNIX_SECONDS), Ok(()));

    let missing = WorldPromotionLiveFacts {
        head: None,
        grant_nonce_consumed: false,
    };
    assert_eq!(
        check(&plan, &operator, &target, &missing, OBSERVED_UNIX_SECONDS),
        Err(vec![WorldPromotionIssue::AuthorityHeadStale])
    );

    let mut repoliced = plan.after.clone();
    repoliced.policy_ref = WorldHeadPolicyRef::new(reference("live-policy")).expect("policy");
    let repoliced_live = WorldPromotionLiveFacts {
        head: Some(repoliced),
        grant_nonce_consumed: false,
    };
    assert_eq!(
        check(&plan, &operator, &target, &repoliced_live, OBSERVED_UNIX_SECONDS),
        Err(vec![WorldPromotionIssue::AuthorityPolicyMismatch])
    );
}

// r[verify molten.world_promotion.reconciliation]
#[test]
fn stale_revoked_or_foreign_operator_authority_is_denied() {
    let plan = plan_world_promotion(&promotion_request()).expect("plan");
    let target = deny_target(&plan);
    let live = promoted_live(&plan);
    let operator = || grant(&plan, plan.after.generation, target.clone());

    let mut revoked = operator();
    revoked.admitted = false;
    assert_eq!(
        check(&plan, &revoked, &target, &live, OBSERVED_UNIX_SECONDS),
        Err(vec![WorldPromotionIssue::AuthorityDenied])
    );

    let mut stale = operator();
    stale.generation = plan.before.generation;
    assert_eq!(
        check(&plan, &stale, &target, &live, OBSERVED_UNIX_SECONDS),
        Err(vec![WorldPromotionIssue::AuthorityGenerationMismatch])
    );

    let mut foreign = operator();
    foreign.authority_ref = WorldPromotionAuthorityRef::new(reference("other-authority")).expect("authority");
    assert_eq!(
        check(&plan, &foreign, &target, &live, OBSERVED_UNIX_SECONDS),
        Err(vec![WorldPromotionIssue::AuthorityDenied])
    );

    let mut other_branch = operator();
    other_branch.branch_id = WorldBranchId::new("other").expect("branch");
    assert_eq!(
        check(&plan, &other_branch, &target, &live, OBSERVED_UNIX_SECONDS),
        Err(vec![WorldPromotionIssue::AuthorityDenied])
    );

    let mut other_policy = operator();
    other_policy.policy_ref = WorldHeadPolicyRef::new(reference("other-policy")).expect("policy");
    assert_eq!(
        check(&plan, &other_policy, &target, &live, OBSERVED_UNIX_SECONDS),
        Err(vec![WorldPromotionIssue::AuthorityPolicyMismatch])
    );

    let mut other_plan_request = promotion_request();
    other_plan_request.operation_ref =
        WorldPromotionOperationRef::new(reference("other-operation")).expect("operation");
    let other_plan = plan_world_promotion(&other_plan_request).expect("other plan");
    assert_eq!(
        validate_operator_authority(WorldOperatorAuthorityInput {
            grant: &operator(),
            requested: &target,
            reservation: &plan.reservations[0],
            plan: &other_plan,
            live: &live,
            observed_unix_seconds: OBSERVED_UNIX_SECONDS,
        }),
        Err(vec![WorldPromotionIssue::ReservationSetMismatch])
    );
}

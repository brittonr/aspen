use transactional_reconciliation_core::Blake3IdentityDeriver;
use transactional_reconciliation_core::Bound;
use transactional_reconciliation_core::CurrentFacts;
use transactional_reconciliation_core::Generation;
use transactional_reconciliation_core::Identity;
use transactional_reconciliation_core::Limits;
use transactional_reconciliation_core::OperationDraft;
use transactional_reconciliation_core::PersistenceBinding;
use transactional_reconciliation_core::PlanningInput;
use transactional_reconciliation_core::Prerequisite;
use transactional_reconciliation_core::Revision;
use transactional_reconciliation_core::build_plan;
use transactional_reconciliation_core::plan_publication;
use transactional_reconciliation_core::reserve_attempt;

use super::super::*;
use super::TRANSACTION_ATTEMPT_CONTEXT;
use super::derived_reference;

const TRANSACTION_SYNTHETIC_CONTEXT: &str = "onixresearch.molten.world-promotion.synthetic-operation.v1";
const HEX_PAIR_BYTES: usize = 2;
const IDENTITY_BYTES: usize = 32;

struct Deriver;

impl Blake3IdentityDeriver for Deriver {
    fn derive_identity(&self, framed_bytes: &[u8]) -> Result<Identity, transactional_reconciliation_core::CoreError> {
        Identity::new(*blake3::hash(framed_bytes).as_bytes())
    }
}

pub(super) fn transactional_plan(
    request: &WorldPromotionRequest,
    plan_ref: &WorldPromotionPlanRef,
    reservations: &[WorldReleaseReservation],
    generation: u64,
) -> Result<TransactionalPromotionPlan, Vec<WorldPromotionIssue>> {
    let deriver = Deriver;
    let desired = identity(plan_ref.as_str())?;
    let policy = identity(request.policy_ref.as_str())?;
    let predecessor = identity(request.expected_head.as_str())?;
    let authority = identity(request.authority.authority_ref.as_str())?;
    let prerequisites = vec![
        Prerequisite::new(predecessor, predecessor),
        Prerequisite::new(policy, authority),
    ];
    let drafts = transaction_drafts(request, reservations, generation)?;
    let operation_limit = Bound::new(MAX_WORLD_PROMOTION_TRANSACTION_OPERATIONS).map_err(transaction_error)?;
    let prerequisite_limit = Bound::new(MAX_WORLD_PROMOTION_PREREQUISITES).map_err(transaction_error)?;
    let shared_plan = build_plan(
        &deriver,
        Limits::new(operation_limit, prerequisite_limit),
        PlanningInput::new(
            Revision::observed(request.expected_generation),
            desired,
            policy,
            prerequisites.clone(),
            drafts,
        ),
    )
    .map_err(transaction_error)?;
    let current = CurrentFacts::new(Revision::observed(request.expected_generation), desired, policy, prerequisites);
    let publication = plan_publication(&shared_plan, &current, desired).map_err(transaction_error)?;
    let release_operations = transactional_release_operations(&shared_plan, reservations)?;
    let publication_operation = shared_plan.operations().first().ok_or_else(|| {
        vec![WorldPromotionIssue::TransactionalMapping(
            "shared-plan-empty".to_string(),
        )]
    })?;
    let publication_attempt_identity = identity(&derived_reference(TRANSACTION_ATTEMPT_CONTEXT, &[
        plan_ref.as_str(),
        request.operation_ref.as_str(),
    ])?)?;
    let publication_reservation =
        reserve_attempt(&shared_plan, publication_operation.idempotency_identity(), publication_attempt_identity)
            .map_err(transaction_error)?;
    let persistence_binding =
        PersistenceBinding::new(publication, publication_reservation, predecessor).map_err(transaction_error)?;
    Ok(TransactionalPromotionPlan {
        shared_plan,
        publication,
        publication_reservation,
        persistence_binding,
        release_operations,
    })
}

pub(in crate::world_promotion) fn transaction_current_facts(
    plan: &WorldPromotionPlan,
) -> Result<CurrentFacts, Vec<WorldPromotionIssue>> {
    let desired = identity(plan.plan_ref.as_str())?;
    let policy = identity(plan.after.policy_ref.as_str())?;
    let predecessor = identity(plan.before.head.as_str())?;
    let authority = identity(plan.authority_ref.as_str())?;
    Ok(CurrentFacts::new(Revision::observed(plan.before.generation), desired, policy, vec![
        Prerequisite::new(predecessor, predecessor),
        Prerequisite::new(policy, authority),
    ]))
}

pub(in crate::world_promotion) fn identity(reference: &str) -> Result<Identity, Vec<WorldPromotionIssue>> {
    let hex = reference
        .strip_prefix("blake3:")
        .ok_or_else(|| vec![WorldPromotionIssue::TransactionalMapping("identity-prefix".to_string())])?;
    if hex.len() != IDENTITY_BYTES.saturating_mul(HEX_PAIR_BYTES) {
        return Err(vec![WorldPromotionIssue::TransactionalMapping("identity-length".to_string())]);
    }
    let mut bytes = [0_u8; IDENTITY_BYTES];
    for (index, pair) in hex.as_bytes().chunks_exact(HEX_PAIR_BYTES).enumerate() {
        let high = hex_nibble(pair[0])?;
        let low = hex_nibble(pair[1])?;
        bytes[index] = (high << 4) | low;
    }
    Identity::new(bytes).map_err(transaction_error)
}

fn transaction_drafts(
    request: &WorldPromotionRequest,
    reservations: &[WorldReleaseReservation],
    generation: u64,
) -> Result<Vec<OperationDraft>, Vec<WorldPromotionIssue>> {
    if reservations.is_empty() {
        return Ok(vec![OperationDraft::new(
            identity(request.candidate_head.as_str())?,
            identity(request.operation_ref.as_str())?,
            Generation::observed(generation),
            identity(&derived_reference(TRANSACTION_SYNTHETIC_CONTEXT, &[request.policy_ref.as_str()])?)?,
        )]);
    }
    reservations
        .iter()
        .map(|reservation| {
            Ok(OperationDraft::new(
                identity(reservation.semantic_ref.as_str())?,
                identity(reservation.intent_ref.as_str())?,
                Generation::observed(generation),
                identity(reservation.handler_ref.as_str())?,
            ))
        })
        .collect()
}

fn transactional_release_operations(
    plan: &transactional_reconciliation_core::ImmutablePlan,
    reservations: &[WorldReleaseReservation],
) -> Result<Vec<TransactionalReleaseOperation>, Vec<WorldPromotionIssue>> {
    let mut operations = Vec::with_capacity(reservations.len());
    for reservation in reservations {
        let semantic = identity(reservation.semantic_ref.as_str())?;
        let intent = identity(reservation.intent_ref.as_str())?;
        let shared = plan
            .operations()
            .iter()
            .find(|operation| operation.subject() == semantic && operation.input() == intent)
            .ok_or_else(|| {
                vec![WorldPromotionIssue::TransactionalMapping(
                    "release-operation-missing".to_string(),
                )]
            })?;
        let attempt_identity = identity(&derived_reference(TRANSACTION_ATTEMPT_CONTEXT, &[
            reservation.reservation_ref.as_str(),
            reservation.operation_ref.as_str(),
        ])?)?;
        let initial_attempt =
            reserve_attempt(plan, shared.idempotency_identity(), attempt_identity).map_err(transaction_error)?;
        operations.push(TransactionalReleaseOperation {
            reservation_ref: reservation.reservation_ref.clone(),
            shared_operation_identity: shared.idempotency_identity(),
            initial_attempt,
        });
    }
    Ok(operations)
}

fn hex_nibble(byte: u8) -> Result<u8, Vec<WorldPromotionIssue>> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        _ => Err(vec![WorldPromotionIssue::TransactionalMapping("identity-hex".to_string())]),
    }
}

fn transaction_error(error: transactional_reconciliation_core::CoreError) -> Vec<WorldPromotionIssue> {
    vec![WorldPromotionIssue::TransactionalMapping(format!("{error:?}"))]
}

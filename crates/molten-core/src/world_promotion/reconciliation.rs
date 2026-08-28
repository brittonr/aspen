use transactional_reconciliation_core::CommitObservation;
use transactional_reconciliation_core::PersistenceState;
use transactional_reconciliation_core::ReadBackObservation;
use transactional_reconciliation_core::Revision;
use transactional_reconciliation_core::admit_published;
use transactional_reconciliation_core::classify_commit;
use transactional_reconciliation_core::reconcile_read_back;

use super::planning::identity;
use super::*;

// r[impl molten.world_promotion.reconciliation]
pub fn classify_promotion_commit(
    plan: &WorldPromotionPlan,
    observation: &WorldPromotionCommitObservation,
) -> Result<WorldPromotionPersistence, Vec<WorldPromotionIssue>> {
    let shared_observation = match observation {
        WorldPromotionCommitObservation::Applied => CommitObservation::Applied(plan.transaction.persistence_binding),
        WorldPromotionCommitObservation::NotApplied {
            current_head,
            current_generation,
        } => CommitObservation::NotApplied {
            current_revision: Revision::observed(*current_generation),
            current_content_identity: identity(current_head.as_str())?,
        },
        WorldPromotionCommitObservation::OutcomeUnknown => CommitObservation::OutcomeUnknown,
        WorldPromotionCommitObservation::RepairReported => CommitObservation::RepairReported,
        WorldPromotionCommitObservation::Corrupt => CommitObservation::Corrupt,
        WorldPromotionCommitObservation::Inconsistent => CommitObservation::Inconsistent,
    };
    Ok(persistence(plan, classify_commit(plan.transaction.persistence_binding, shared_observation)))
}

// r[impl molten.world_promotion.reconciliation]
pub fn reconcile_promotion_read_back(
    plan: &WorldPromotionPlan,
    prior: &WorldPromotionPersistence,
    observation: &WorldPromotionReadBackObservation,
) -> Result<WorldPromotionPersistence, Vec<WorldPromotionIssue>> {
    let shared_observation = match observation {
        WorldPromotionReadBackObservation::Prior { head, generation } => ReadBackObservation::Prior {
            revision: Revision::observed(*generation),
            content_identity: identity(head.as_str())?,
        },
        WorldPromotionReadBackObservation::Reservation => {
            ReadBackObservation::Reservation(plan.transaction.persistence_binding)
        }
        WorldPromotionReadBackObservation::Missing => ReadBackObservation::Missing,
        WorldPromotionReadBackObservation::Corrupt => ReadBackObservation::Corrupt,
    };
    let reconciled = reconcile_read_back(prior.shared, shared_observation).map_err(transaction_error)?;
    Ok(persistence(plan, reconciled))
}

fn persistence(
    plan: &WorldPromotionPlan,
    shared: transactional_reconciliation_core::PersistenceDecision,
) -> WorldPromotionPersistence {
    let is_dispatch_eligible = shared.state() == PersistenceState::Published
        && admit_published(shared, plan.transaction.publication_reservation).is_ok();
    WorldPromotionPersistence {
        shared,
        dispatch_eligible: is_dispatch_eligible,
        mutation_authorized_by_evidence: false,
        non_claims: promotion_non_claims(),
    }
}

fn transaction_error(error: transactional_reconciliation_core::CoreError) -> Vec<WorldPromotionIssue> {
    vec![WorldPromotionIssue::TransactionalMapping(format!("{error:?}"))]
}

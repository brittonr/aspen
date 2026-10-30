use transactional_reconciliation_core;

use super::bounds;
use super::domain;
use super::planning;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReconciliationInput {
    pub finalization: planning::FinalizationInput,
    pub publication_intent: transactional_reconciliation_core::PublicationIntent,
    pub publication_observation: transactional_reconciliation_core::PublicationObservation,
    pub reservation: transactional_reconciliation_core::AttemptReservation,
    pub operation: transactional_reconciliation_core::Operation,
    pub terminal: Option<transactional_reconciliation_core::TerminalObservation>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReconciliationDecision {
    Finalize(Box<planning::FinalizationPlan>),
    AlreadyComplete,
    Superseded,
    Conflict,
    Retryable,
    Denied(domain::WitnessIssue),
    ManualReview,
}

/// Uncertainty is not permission to dispatch a new append or local mutation.
// r[impl molten.world_witness.reconciliation]
pub fn reconcile(input: &ReconciliationInput) -> ReconciliationDecision {
    if let Err(issue) = bounds::validate_stage(&input.finalization.staged)
        .and_then(|()| input.finalization.durable_stage.as_ref().map_or(Ok(()), bounds::validate_stage))
        .and_then(|()| bounds::validate(&input.finalization.currentness))
    {
        return ReconciliationDecision::Denied(issue);
    }
    let staged = &input.finalization.staged;
    if binding_is_invalid(input) {
        return ReconciliationDecision::ManualReview;
    }
    let publication = transactional_reconciliation_core::classify_publication(
        &input.publication_intent,
        input.publication_observation,
    );
    let recovery = transactional_reconciliation_core::recover_attempt(input.reservation, input.terminal);
    match publication {
        transactional_reconciliation_core::PublicationClassification::Invalid => ReconciliationDecision::Conflict,
        transactional_reconciliation_core::PublicationClassification::Uncertain => ReconciliationDecision::ManualReview,
        transactional_reconciliation_core::PublicationClassification::Stale => ReconciliationDecision::Superseded,
        transactional_reconciliation_core::PublicationClassification::Published => {
            if input.finalization.current_head.as_ref() != Some(&staged.transition.after) {
                return ReconciliationDecision::Conflict;
            }
            match super::validation::validate_committed_currentness(&input.finalization.currentness) {
                Ok(_) => ReconciliationDecision::AlreadyComplete,
                Err(issue) => ReconciliationDecision::Denied(issue),
            }
        }
        transactional_reconciliation_core::PublicationClassification::NotPublished => match recovery {
            transactional_reconciliation_core::RecoveryDecision::PendingUnobserved => {
                ReconciliationDecision::ManualReview
            }
            transactional_reconciliation_core::RecoveryDecision::Terminal(
                transactional_reconciliation_core::EffectTerminal::Failed(_),
            ) => ReconciliationDecision::Retryable,
            transactional_reconciliation_core::RecoveryDecision::Terminal(
                transactional_reconciliation_core::EffectTerminal::Succeeded(_),
            ) => match planning::plan_finalization(&input.finalization) {
                Ok(plan) => ReconciliationDecision::Finalize(Box::new(plan)),
                Err(domain::WitnessIssue::LocalConflict) => ReconciliationDecision::Superseded,
                Err(issue) => ReconciliationDecision::Denied(issue),
            },
        },
    }
}

fn binding_is_invalid(input: &ReconciliationInput) -> bool {
    let staged = &input.finalization.staged;
    operation_identity(&staged.operation_id) != Some(input.operation.input())
        || operation_identity(staged.transition.claim_ref.as_str()) != Some(input.operation.subject())
        || input.operation.idempotency_identity() != input.reservation.operation_identity()
        || input.operation.generation()
            != transactional_reconciliation_core::Generation::observed(staged.transition.after.generation)
        || input.reservation.plan_identity() != input.publication_intent.plan_identity()
        || input.publication_intent.expected_revision()
            != transactional_reconciliation_core::Revision::observed(
                staged.transition.before.as_ref().map_or(0, |state| state.generation),
            )
        || input.publication_intent.successor_revision()
            != transactional_reconciliation_core::Revision::observed(staged.transition.after.generation)
        || operation_identity(staged.transition.after.head.as_str())
            != Some(input.publication_intent.successor_content_identity())
        || input.finalization.currentness.subject != planning::subject(staged)
        || input.finalization.currentness.operation_id != staged.operation_id
        || input.finalization.currentness.selected_profile != staged.selected_profile
        || input.finalization.currentness.retained != staged.retained
}

fn operation_identity(value: &str) -> Option<transactional_reconciliation_core::Identity> {
    let hex = value.strip_prefix("blake3:")?;
    if hex.len() != 64 {
        return None;
    }
    let mut bytes = [0; 32];
    for (index, pair) in hex.as_bytes().chunks_exact(2).enumerate() {
        let upper = char::from(pair[0]).to_digit(16)?;
        let lower = char::from(pair[1]).to_digit(16)?;
        bytes[index] = u8::try_from(upper * 16 + lower).ok()?;
    }
    transactional_reconciliation_core::Identity::new(bytes).ok()
}

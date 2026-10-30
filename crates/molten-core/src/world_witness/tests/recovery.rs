use transactional_reconciliation_core;

use super::super::domain;
use super::super::reconciliation;
use super::support;

struct Deriver;

impl transactional_reconciliation_core::Blake3IdentityDeriver for Deriver {
    fn derive_identity(
        &self,
        bytes: &[u8],
    ) -> Result<transactional_reconciliation_core::Identity, transactional_reconciliation_core::CoreError> {
        transactional_reconciliation_core::Identity::new(*blake3::hash(bytes).as_bytes())
    }
}

fn identity(label: &str) -> transactional_reconciliation_core::Identity {
    transactional_reconciliation_core::Identity::new(*blake3::hash(label.as_bytes()).as_bytes()).unwrap()
}

fn fixture() -> reconciliation::ReconciliationInput {
    let revision = transactional_reconciliation_core::Revision::observed(1);
    let desired = identity("after");
    let policy = identity("policy");
    let plan = transactional_reconciliation_core::build_plan(
        &Deriver,
        transactional_reconciliation_core::Limits::new(
            transactional_reconciliation_core::Bound::new(4).unwrap(),
            transactional_reconciliation_core::Bound::new(4).unwrap(),
        ),
        transactional_reconciliation_core::PlanningInput::new(revision, desired, policy, vec![], vec![
            transactional_reconciliation_core::OperationDraft::new(
                identity("claim"),
                identity("operation"),
                transactional_reconciliation_core::Generation::observed(2),
                identity("provider"),
            ),
        ]),
    )
    .unwrap();
    let reservation = transactional_reconciliation_core::reserve_attempt(
        &plan,
        plan.operations()[0].idempotency_identity(),
        identity("attempt"),
    )
    .unwrap();
    let finalization = support::finalization();
    let terminal = Some(transactional_reconciliation_core::TerminalObservation::new(
        reservation.plan_identity(),
        reservation.operation_identity(),
        reservation.attempt_identity(),
        transactional_reconciliation_core::EffectTerminal::Succeeded(identity("receipt")),
    ));
    reconciliation::ReconciliationInput {
        finalization,
        reservation,
        terminal,
        operation: plan.operations()[0],
        publication_intent: transactional_reconciliation_core::plan_publication(
            &plan,
            &transactional_reconciliation_core::CurrentFacts::new(revision, desired, policy, vec![]),
            desired,
        )
        .unwrap(),
        publication_observation: transactional_reconciliation_core::PublicationObservation::NotApplied {
            current_revision: revision,
        },
    }
}

// r[verify molten.world_witness.reconciliation]
#[test]
fn exact_successful_append_recovers_to_finalization_and_complete_publication() {
    let mut input = fixture();
    assert!(matches!(reconciliation::reconcile(&input), reconciliation::ReconciliationDecision::Finalize(_)));
    input.publication_observation = transactional_reconciliation_core::PublicationObservation::Applied {
        predecessor: input.publication_intent.expected_revision(),
        successor: input.publication_intent.successor_revision(),
        successor_content_identity: input.publication_intent.successor_content_identity(),
    };
    input.finalization.current_head = Some(input.finalization.staged.transition.after.clone());
    assert_eq!(reconciliation::reconcile(&input), reconciliation::ReconciliationDecision::AlreadyComplete);
}

// r[verify molten.world_witness.reconciliation]
#[test]
fn uncertain_append_and_local_commit_do_not_become_duplicate_dispatch() {
    let mut input = fixture();
    input.terminal = None;
    assert_eq!(reconciliation::reconcile(&input), reconciliation::ReconciliationDecision::ManualReview);
    input = fixture();
    input.publication_observation = transactional_reconciliation_core::PublicationObservation::OutcomeUnknown;
    assert_eq!(reconciliation::reconcile(&input), reconciliation::ReconciliationDecision::ManualReview);
    input = fixture();
    input.terminal = Some(transactional_reconciliation_core::TerminalObservation::new(
        input.reservation.plan_identity(),
        input.reservation.operation_identity(),
        identity("other-attempt"),
        transactional_reconciliation_core::EffectTerminal::Succeeded(identity("receipt")),
    ));
    assert_eq!(reconciliation::reconcile(&input), reconciliation::ReconciliationDecision::ManualReview);
}

// r[verify molten.world_witness.reconciliation]
#[test]
fn failed_append_is_retryable_and_superseded_or_mismatched_publications_are_not_success() {
    let mut input = fixture();
    input.terminal = Some(transactional_reconciliation_core::TerminalObservation::new(
        input.reservation.plan_identity(),
        input.reservation.operation_identity(),
        input.reservation.attempt_identity(),
        transactional_reconciliation_core::EffectTerminal::Failed(identity("failure")),
    ));
    assert_eq!(reconciliation::reconcile(&input), reconciliation::ReconciliationDecision::Retryable);
    input.publication_observation = transactional_reconciliation_core::PublicationObservation::NotApplied {
        current_revision: transactional_reconciliation_core::Revision::observed(4),
    };
    assert_eq!(reconciliation::reconcile(&input), reconciliation::ReconciliationDecision::Superseded);
    input.publication_observation = transactional_reconciliation_core::PublicationObservation::Applied {
        predecessor: input.publication_intent.expected_revision(),
        successor: input.publication_intent.successor_revision(),
        successor_content_identity: identity("other-content"),
    };
    assert_eq!(reconciliation::reconcile(&input), reconciliation::ReconciliationDecision::Conflict);
}

// r[verify molten.world_witness.provider_boundary]
#[test]
fn recovered_append_cannot_override_current_authority_denial() {
    let mut input = fixture();
    input.finalization.authority.admitted = false;
    assert_eq!(
        reconciliation::reconcile(&input),
        reconciliation::ReconciliationDecision::Denied(domain::WitnessIssue::AuthorityDenied)
    );
    input.finalization.authority.admitted = true;
    input.finalization.currentness.operation_id = support::reference("other-operation");
    assert_eq!(reconciliation::reconcile(&input), reconciliation::ReconciliationDecision::ManualReview);
}

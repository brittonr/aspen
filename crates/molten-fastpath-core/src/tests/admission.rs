//! Stable-view fast admission and session-keyed convergence.

fn fallback(reason: crate::quorate::FallbackReason) -> crate::quorate::StableViewDecision {
    crate::quorate::StableViewDecision::Fallback(reason)
}

// r[verify molten.consensus.fast_path_model.stable_view]
#[test]
fn same_view_superquorum_and_all_proposer_promises_fast_commit() {
    for node_count in [super::support::THREE_NODES, super::support::FIVE_NODES] {
        assert_eq!(
            crate::quorate::evaluate_stable_view(
                &super::support::attempt(node_count),
                super::support::quorums(node_count)
            ),
            crate::quorate::StableViewDecision::FastCommitted
        );
    }
}

// r[verify molten.consensus.fast_path_model.fallback_identity]
#[test]
fn mixed_views_missing_promises_and_identity_drift_fallback() {
    let quorums = super::support::quorums(super::support::THREE_NODES);
    let mut mixed = super::support::attempt(super::support::THREE_NODES);
    mixed.acknowledgements[0].base_view = super::support::NEXT_VIEW;
    assert_eq!(
        crate::quorate::evaluate_stable_view(&mixed, quorums),
        fallback(crate::quorate::FallbackReason::MixedView)
    );

    let mut missing = super::support::attempt(super::support::THREE_NODES);
    missing.promises.clear();
    assert_eq!(
        crate::quorate::evaluate_stable_view(&missing, quorums),
        fallback(crate::quorate::FallbackReason::MissingProposerPromise)
    );

    let mut drift = super::support::attempt(super::support::THREE_NODES);
    drift.original_operation.command_ref = "other-command".to_owned();
    assert_eq!(
        crate::quorate::evaluate_stable_view(&drift, quorums),
        fallback(crate::quorate::FallbackReason::IdentityMismatch)
    );
}

/// A five-replica superquorum (4 of 5) need not contain the proposer. Without the proposer's own
/// acknowledgement the original engine may decide a different command at that position, so the
/// promise alone must not admit the fast path.
// r[verify molten.consensus.fast_path_model.stable_view]
// r[verify molten.consensus.fast_path_model.fault_corpus]
#[test]
fn a_superquorum_without_the_active_proposer_acknowledgement_falls_back() {
    let quorums = super::support::quorums(super::support::FIVE_NODES);
    let mut without_proposer = super::support::attempt(super::support::FIVE_NODES);
    without_proposer
        .acknowledgements
        .retain(|acknowledgement| acknowledgement.replica_id != super::support::PROPOSER);
    assert_eq!(without_proposer.acknowledgements.len(), quorums.superquorum);
    assert_eq!(
        crate::quorate::evaluate_stable_view(&without_proposer, quorums),
        fallback(crate::quorate::FallbackReason::MissingProposerAcknowledgement)
    );
    without_proposer.original_path_available = false;
    assert_eq!(
        crate::quorate::evaluate_stable_view(&without_proposer, quorums),
        fallback(crate::quorate::FallbackReason::OriginalPathUnavailable)
    );

    // The same superquorum size with the proposer among the acknowledgers fast-commits.
    let mut with_proposer = super::support::attempt(super::support::FIVE_NODES);
    with_proposer.acknowledgements.truncate(quorums.superquorum);
    assert!(with_proposer.acknowledgements.iter().any(|row| row.replica_id == super::support::PROPOSER));
    assert_eq!(
        crate::quorate::evaluate_stable_view(&with_proposer, quorums),
        crate::quorate::StableViewDecision::FastCommitted
    );
}

// r[verify molten.consensus.fast_path_model.stable_view]
#[test]
fn an_active_proposer_outside_the_admitted_replicas_can_never_acknowledge() {
    let quorums = super::support::quorums(super::support::THREE_NODES);
    let mut outsider = super::support::attempt(super::support::THREE_NODES);
    outsider.active_proposers = super::support::strings(&["outsider"]);
    outsider.promises[0].proposer_id = "outsider".to_owned();
    let template = outsider.acknowledgements[0].clone();
    outsider.acknowledgements.push(crate::quorate::FastAcknowledgement {
        replica_id: "outsider".to_owned(),
        ..template
    });
    assert_eq!(
        crate::quorate::evaluate_stable_view(&outsider, quorums),
        fallback(crate::quorate::FallbackReason::MissingProposerAcknowledgement)
    );
}

// r[verify molten.consensus.fast_path_model.stable_view]
#[test]
fn acknowledgements_from_outside_the_admitted_replicas_never_count() {
    let quorums = super::support::quorums(super::support::THREE_NODES);
    let mut padded = super::support::attempt(super::support::THREE_NODES);
    padded.acknowledgements.truncate(quorums.superquorum - 1);
    let template = padded.acknowledgements[0].clone();
    for outsider in ["outsider-0", "outsider-1", "outsider-2"] {
        padded.acknowledgements.push(crate::quorate::FastAcknowledgement {
            replica_id: outsider.to_owned(),
            ..template.clone()
        });
    }
    assert_eq!(
        crate::quorate::evaluate_stable_view(&padded, quorums),
        fallback(crate::quorate::FallbackReason::InsufficientAcknowledgements)
    );

    // A non-member acknowledgement in another view is ignored, not a mixed view.
    let mut stray = super::support::attempt(super::support::THREE_NODES);
    stray.acknowledgements.push(crate::quorate::FastAcknowledgement {
        replica_id: "outsider".to_owned(),
        base_view: super::support::NEXT_VIEW,
        ..template
    });
    assert_eq!(
        crate::quorate::evaluate_stable_view(&stray, quorums),
        crate::quorate::StableViewDecision::FastCommitted
    );
}

// r[verify molten.consensus.fast_path_model.fallback_identity]
#[test]
fn convergence_suppresses_duplicate_application_and_reply() {
    let mut ledger = crate::quorate::ApplicationLedger::default();
    let operation = super::support::operation("command-a");
    let first = ledger.converge(&operation).expect("first session operation");
    let duplicate = ledger.converge(&operation).expect("identical retry");
    assert!(first.is_applied);
    assert!(first.is_replied);
    assert!(!first.is_duplicate_suppressed);
    assert!(!duplicate.is_applied);
    assert!(!duplicate.is_replied);
    assert!(duplicate.is_duplicate_suppressed);
    assert_eq!(ledger.applied_operations().collect::<Vec<_>>(), vec![&operation]);
}

// r[verify molten.consensus.fast_path_model.fallback_identity]
#[test]
fn equal_commands_from_distinct_sessions_both_apply_and_a_conflicting_retry_is_rejected() {
    let mut ledger = crate::quorate::ApplicationLedger::default();
    let first = super::support::operation("command-a");
    let mut other_session = first.clone();
    other_session.session_ref = "session-b".to_owned();
    assert!(ledger.converge(&first).expect("session A").is_applied);
    let distinct = ledger.converge(&other_session).expect("session B");
    assert!(distinct.is_applied, "session B's equal command is its own operation");
    assert!(distinct.is_replied, "session B's equal command is its own reply");
    assert_eq!(ledger.applied_operations().count(), 2);

    let mut conflicting = first.clone();
    conflicting.command_ref = "command-b".to_owned();
    assert_eq!(ledger.converge(&conflicting), Err(crate::quorate::LedgerIssue::ConflictingSessionRetry));
    assert_eq!(ledger.applied_operations().count(), 2, "a rejected retry applies nothing");
    assert!(ledger.converge(&first).expect("original retry").is_duplicate_suppressed);
}

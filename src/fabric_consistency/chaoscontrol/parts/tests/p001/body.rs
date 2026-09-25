
// r[verify molten.consensus.chaoscontrol_operation_identity]
#[test]
fn retry_with_changed_identity_is_an_invalid_idempotency_input() {
    let attempts = vec![
        ChaosControlProposalAttempt {
            operation: ChaosControlLogicalOperation {
                client_session: "client-session-a".to_string(),
                sequence: 3,
            },
            operation_ref: test_ref("operation-1"),
            outcome: ChaosControlProposalOutcome::Indefinite,
        },
        ChaosControlProposalAttempt {
            operation: ChaosControlLogicalOperation {
                client_session: "client-session-a".to_string(),
                sequence: 4,
            },
            operation_ref: test_ref("operation-1"),
            outcome: ChaosControlProposalOutcome::Indefinite,
        },
    ];
    let error = admit_proposal_attempts(&attempts).expect_err("changed identity is rejected");
    assert!(matches!(error, crate::error::MoltenError::InvalidHarness(_)));
}

// r[verify molten.consensus.chaoscontrol_operation_identity]
#[test]
fn indefinite_outcome_cannot_become_definite_rejection() {
    let operation = ChaosControlLogicalOperation {
        client_session: "client-session-a".to_string(),
        sequence: 3,
    };
    let attempts = vec![
        ChaosControlProposalAttempt {
            operation: operation.clone(),
            operation_ref: test_ref("operation-1"),
            outcome: ChaosControlProposalOutcome::Indefinite,
        },
        ChaosControlProposalAttempt {
            operation,
            operation_ref: test_ref("operation-1"),
            outcome: ChaosControlProposalOutcome::DefinitelyRejected,
        },
    ];
    let error = admit_proposal_attempts(&attempts).expect_err("timeout cannot become definite non-execution evidence");
    assert!(matches!(error, crate::error::MoltenError::InvalidHarness(_)));
}

// r[verify molten.consensus.chaoscontrol_operation_identity]
#[test]
fn acknowledged_retry_names_the_same_committed_index() {
    let operation = ChaosControlLogicalOperation {
        client_session: "client-session-a".to_string(),
        sequence: 3,
    };
    let attempts = vec![
        ChaosControlProposalAttempt {
            operation: operation.clone(),
            operation_ref: test_ref("operation-1"),
            outcome: ChaosControlProposalOutcome::Acknowledged { committed_index: 4 },
        },
        ChaosControlProposalAttempt {
            operation,
            operation_ref: test_ref("operation-1"),
            outcome: ChaosControlProposalOutcome::Acknowledged { committed_index: 5 },
        },
    ];
    let error = admit_proposal_attempts(&attempts).expect_err("one logical operation applies at most once");
    assert!(matches!(error, crate::error::MoltenError::InvalidHarness(_)));
}

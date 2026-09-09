use super::*;
use crate::error::MoltenError;

const TEST_PROFILE_REF: &str = "blake3:1111111111111111111111111111111111111111111111111111111111111111";
const TEST_INITIAL_STATE_REF: &str = "blake3:2222222222222222222222222222222222222222222222222222222222222222";
// Known-answer vectors over the exact archived ChaosControl smr-chain framing,
// computed independently of this implementation with b3sum.
const TEST_GENESIS_DIGEST: &str = "blake3:c0aef32e9df5fccb69e516d215173eafed309ae3b6c9a01993f8d06af732f8df";
const TEST_FIRST_TRANSITION_DIGEST: &str = "blake3:8547a0d8e82b1365308e1a322d10079b0badc88461aff743df4679e8ae16b680";
const TEST_COMMAND_BYTES: &[u8] = b"molten-command-bytes";

fn test_ref(label: &str) -> String {
    crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("chaoscontrol-conformance-test-ref", vec![
        crate::preserves_rail::string(label),
    ]))
    .expect("test ref")
}

fn lossless_profile() -> ChaosControlConformanceProfile {
    ChaosControlConformanceProfile {
        chaoscontrol_profile_ref: TEST_PROFILE_REF.to_string(),
        observation_mode: ChaosControlObservationMode::Lossless,
        max_command_bytes: 4_096,
        max_projected_observations: 64,
    }
}

fn committed_apply(index: u64) -> CommittedApplyObservation {
    CommittedApplyObservation {
        group_ref: test_ref("consistency-group"),
        replica_ref: test_ref("replica-a"),
        command_index: index,
        operation_ref: test_ref(&format!("operation-{index}")),
        command_ref: test_ref(&format!("command-{index}")),
        command_bytes: TEST_COMMAND_BYTES.to_vec(),
        application_state_ref: test_ref(&format!("application-state-{index}")),
        lifecycle_generation: 1,
        application_receipt_ref: test_ref(&format!("application-receipt-{index}")),
    }
}

fn projected_observations(count: u64) -> Vec<ChaosControlChainObservation> {
    let mut projector =
        ChaosControlChainProjector::bind(lossless_profile(), TEST_INITIAL_STATE_REF).expect("projector binds");
    (1..=count)
        .map(|index| {
            let mut apply = committed_apply(index);
            apply.command_bytes = format!("command-{index}").into_bytes();
            projector.project(&apply).expect("projection succeeds")
        })
        .collect()
}

// r[verify molten.consensus.chaoscontrol_chain_observation]
#[test]
fn genesis_and_first_transition_match_the_archived_chaoscontrol_framing() {
    let projector =
        ChaosControlChainProjector::bind(lossless_profile(), TEST_INITIAL_STATE_REF).expect("projector binds");
    assert_eq!(projector.genesis_digest(), TEST_GENESIS_DIGEST);
    let mut projector = projector;
    let observation = projector.project(&committed_apply(1)).expect("projection succeeds");
    assert_eq!(observation.next_digest, TEST_FIRST_TRANSITION_DIGEST);
    assert_eq!(observation.prior_digest, TEST_GENESIS_DIGEST);
}

// r[verify molten.consensus.chaoscontrol_chain_observation]
#[test]
fn deterministic_projection_repeats_identical_observations() {
    let mut first =
        ChaosControlChainProjector::bind(lossless_profile(), TEST_INITIAL_STATE_REF).expect("projector binds");
    let mut second =
        ChaosControlChainProjector::bind(lossless_profile(), TEST_INITIAL_STATE_REF).expect("projector binds");
    let first_observation = first.project(&committed_apply(1)).expect("projection succeeds");
    let second_observation = second.project(&committed_apply(1)).expect("projection succeeds");
    assert_eq!(first_observation, second_observation);
}

// r[verify molten.consensus.chaoscontrol_chain_observation]
#[test]
fn replicas_applying_the_same_command_share_digests_and_keep_own_refs() {
    let mut replica_a =
        ChaosControlChainProjector::bind(lossless_profile(), TEST_INITIAL_STATE_REF).expect("projector binds");
    let mut replica_b =
        ChaosControlChainProjector::bind(lossless_profile(), TEST_INITIAL_STATE_REF).expect("projector binds");
    let mut apply_b = committed_apply(1);
    apply_b.replica_ref = test_ref("replica-b");
    apply_b.application_state_ref = test_ref("application-state-b");
    let observation_a = replica_a.project(&committed_apply(1)).expect("projection succeeds");
    let observation_b = replica_b.project(&apply_b).expect("projection succeeds");
    assert_eq!(observation_a.next_digest, observation_b.next_digest);
    assert_ne!(observation_a.replica_ref, observation_b.replica_ref);
    assert_ne!(observation_a.application_state_ref, observation_b.application_state_ref);
}

// r[verify molten.consensus.chaoscontrol_chain_observation]
#[test]
fn projection_binds_every_required_observation_field() {
    let mut projector =
        ChaosControlChainProjector::bind(lossless_profile(), TEST_INITIAL_STATE_REF).expect("projector binds");
    let apply = committed_apply(1);
    let observation = projector.project(&apply).expect("projection succeeds");
    assert_eq!(observation.chaoscontrol_profile_ref, TEST_PROFILE_REF);
    assert_eq!(observation.group_ref, apply.group_ref);
    assert_eq!(observation.replica_ref, apply.replica_ref);
    assert_eq!(observation.command_index, 1);
    assert_eq!(observation.operation_ref, apply.operation_ref);
    assert_eq!(observation.command_ref, apply.command_ref);
    assert_eq!(observation.command_bytes, TEST_COMMAND_BYTES);
    assert_eq!(observation.application_state_ref, apply.application_state_ref);
    assert_eq!(observation.lifecycle_generation, apply.lifecycle_generation);
    assert_eq!(observation.application_receipt_ref, apply.application_receipt_ref);
}

// r[verify molten.consensus.chaoscontrol_chain_observation]
#[test]
fn projection_denies_an_observation_that_bypasses_the_committed_application_path() {
    let mut projector =
        ChaosControlChainProjector::bind(lossless_profile(), TEST_INITIAL_STATE_REF).expect("projector binds");
    let mut fabricated = committed_apply(1);
    fabricated.application_receipt_ref = "not-a-content-ref".to_string();
    let error = projector.project(&fabricated).expect_err("fabricated observation is denied");
    assert!(matches!(error, MoltenError::InvalidHarness(_)));
}

// r[verify molten.consensus.chaoscontrol_chain_observation]
#[test]
fn projection_denies_noncontiguous_committed_apply_order() {
    let mut projector =
        ChaosControlChainProjector::bind(lossless_profile(), TEST_INITIAL_STATE_REF).expect("projector binds");
    projector.project(&committed_apply(1)).expect("projection succeeds");
    let error = projector.project(&committed_apply(3)).expect_err("changed order is denied");
    assert!(matches!(error, MoltenError::InvalidHarness(_)));
    assert_eq!(projector.next_command_index(), 2);
}

// r[verify molten.consensus.chaoscontrol_chain_observation]
#[test]
fn projection_denies_duplicated_committed_apply() {
    let mut projector =
        ChaosControlChainProjector::bind(lossless_profile(), TEST_INITIAL_STATE_REF).expect("projector binds");
    projector.project(&committed_apply(1)).expect("projection succeeds");
    let error = projector.project(&committed_apply(1)).expect_err("duplicate apply is denied");
    assert!(matches!(error, MoltenError::InvalidHarness(_)));
    assert_eq!(projector.projected_count(), 1);
}

// r[verify molten.consensus.chaoscontrol_chain_observation]
#[test]
fn projection_denies_rollback_to_an_earlier_command_index() {
    let mut projector =
        ChaosControlChainProjector::bind(lossless_profile(), TEST_INITIAL_STATE_REF).expect("projector binds");
    projector.project(&committed_apply(1)).expect("projection succeeds");
    let mut rollback = committed_apply(1);
    rollback.command_bytes = b"rewritten-command".to_vec();
    let error = projector.project(&rollback).expect_err("rollback is denied");
    assert!(matches!(error, MoltenError::InvalidHarness(_)));
}

// r[verify molten.consensus.chaoscontrol_chain_observation]
#[test]
fn projection_denies_malformed_references() {
    let mut projector =
        ChaosControlChainProjector::bind(lossless_profile(), TEST_INITIAL_STATE_REF).expect("projector binds");
    let mut malformed = committed_apply(1);
    malformed.application_state_ref = "blake3:NOTHEX".to_string();
    let error = projector.project(&malformed).expect_err("malformed ref is denied");
    assert!(matches!(error, MoltenError::InvalidHarness(_)));
}

// r[verify molten.consensus.chaoscontrol_chain_observation]
#[test]
fn projection_denies_a_stale_lifecycle_generation() {
    let mut projector =
        ChaosControlChainProjector::bind(lossless_profile(), TEST_INITIAL_STATE_REF).expect("projector binds");
    projector.project(&committed_apply(1)).expect("projection succeeds");
    let mut stale = committed_apply(2);
    stale.lifecycle_generation = 0;
    let error = projector.project(&stale).expect_err("stale generation is denied");
    assert!(matches!(error, MoltenError::InvalidHarness(_)));
}

// r[verify molten.consensus.chaoscontrol_chain_observation]
#[test]
fn sampled_profile_cannot_support_conformance_projection() {
    let mut profile = lossless_profile();
    profile.observation_mode = ChaosControlObservationMode::Sampled;
    let mut projector = ChaosControlChainProjector::bind(profile, TEST_INITIAL_STATE_REF).expect("projector binds");
    let error = projector.project(&committed_apply(1)).expect_err("sampled projection cannot support conformance");
    assert!(matches!(error, MoltenError::InvalidHarness(_)));
}

// r[verify molten.consensus.chaoscontrol_chain_observation]
#[test]
fn lossless_ledger_reports_ready_when_every_observation_arrives() {
    let mut ledger = ChaosControlObservationLedger::new(ChaosControlObservationMode::Lossless);
    for observation in projected_observations(3) {
        let status = ledger.ingest(&observation).expect("ingest succeeds");
        assert_eq!(status, ChaosControlIngestStatus::Appended);
    }
    assert_eq!(ledger.conformance_verdict(), ChaosControlConformanceVerdict::Ready);
}

// r[verify molten.consensus.chaoscontrol_chain_observation]
#[test]
fn lossless_ledger_blocks_conformance_on_an_observation_gap_without_safety_rejection() {
    let mut ledger = ChaosControlObservationLedger::new(ChaosControlObservationMode::Lossless);
    let observations = projected_observations(3);
    ledger.ingest(&observations[0]).expect("ingest succeeds");
    ledger.ingest(&observations[2]).expect("ingest succeeds");
    assert_eq!(ledger.conformance_verdict(), ChaosControlConformanceVerdict::BlockedByObserverGap {
        dropped_events: 1
    });
    assert!(ledger.violations().is_empty());
}

// r[verify molten.consensus.chaoscontrol_chain_observation]
#[test]
fn ledger_suppresses_identical_duplicate_reports() {
    let mut ledger = ChaosControlObservationLedger::new(ChaosControlObservationMode::Lossless);
    let observations = projected_observations(2);
    ledger.ingest(&observations[0]).expect("ingest succeeds");
    let status = ledger.ingest(&observations[0]).expect("re-ingest succeeds");
    assert_eq!(status, ChaosControlIngestStatus::DuplicateSuppressed);
    ledger.ingest(&observations[1]).expect("ingest succeeds");
    assert_eq!(ledger.conformance_verdict(), ChaosControlConformanceVerdict::Ready);
}

// r[verify molten.consensus.chaoscontrol_chain_observation]
#[test]
fn ledger_records_rollback_as_a_persistent_safety_violation() {
    let mut ledger = ChaosControlObservationLedger::new(ChaosControlObservationMode::Lossless);
    let observations = projected_observations(3);
    ledger.ingest(&observations[0]).expect("ingest succeeds");
    let mut rewritten = observations[0].clone();
    rewritten.next_digest = test_ref("rewritten-digest");
    let status = ledger.ingest(&rewritten).expect("conflicting ingest is classified");
    assert_eq!(status, ChaosControlIngestStatus::ViolationRecorded);
    ledger.ingest(&observations[1]).expect("later matching observation is ingested");
    assert!(matches!(ledger.conformance_verdict(), ChaosControlConformanceVerdict::RejectedSafety { .. }));
}

// r[verify molten.consensus.chaoscontrol_operation_identity]
#[test]
fn transport_observations_map_to_bounded_proposal_outcomes() {
    assert_eq!(
        map_transport_observation(ChaosControlTransportObservation::AcknowledgementReceipt { committed_index: 7 }),
        ChaosControlProposalOutcome::Acknowledged { committed_index: 7 }
    );
    assert_eq!(
        map_transport_observation(ChaosControlTransportObservation::DefiniteRejectionReceipt),
        ChaosControlProposalOutcome::DefinitelyRejected
    );
    for uncertain in [
        ChaosControlTransportObservation::Timeout,
        ChaosControlTransportObservation::Disconnect,
        ChaosControlTransportObservation::ProcessLoss,
    ] {
        assert_eq!(map_transport_observation(uncertain), ChaosControlProposalOutcome::Indefinite);
    }
}

// r[verify molten.consensus.chaoscontrol_operation_identity]
#[test]
fn indefinite_outcome_resolves_through_committed_history_at_most_once() {
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
            operation: operation.clone(),
            operation_ref: test_ref("operation-1"),
            outcome: ChaosControlProposalOutcome::Indefinite,
        },
        ChaosControlProposalAttempt {
            operation,
            operation_ref: test_ref("operation-1"),
            outcome: ChaosControlProposalOutcome::Acknowledged { committed_index: 4 },
        },
    ];
    admit_proposal_attempts(&attempts).expect("retry keeps identity and resolves at most once");
}

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
    assert!(matches!(error, MoltenError::InvalidHarness(_)));
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
    assert!(matches!(error, MoltenError::InvalidHarness(_)));
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
    assert!(matches!(error, MoltenError::InvalidHarness(_)));
}

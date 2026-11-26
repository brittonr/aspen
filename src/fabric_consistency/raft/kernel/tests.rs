//! The shell keeps canonical Preserves identities and every refusal's failure class when it
//! drives the `molten-raft-core` kernel.

use super::*;

// r[verify molten.consensus.no_std_raft_and_reliability_kernels]
#[test]
fn refusals_keep_their_failure_class_and_text_at_the_shell_boundary() {
    let denied = replica_failure(ReplicaError::Denied("Raft event requires a running replica"));
    assert_eq!(denied, crate::error::Failure::invalid_harness("Raft event requires a running replica"));
    assert_eq!(denied.class(), None);

    let divergence = replica_failure(ReplicaError::RecoveryEvidence {
        snapshot_ref: tests::test_ref("diverged-snapshot"),
        index: 1,
        term: 2,
    });
    assert_eq!(divergence.class(), Some(crate::error::FailureClass::MalformedObservation));
    assert!(divergence.to_string().contains("Raft recovery evidence diverges from snapshot"), "{divergence}");

    let encoding = replica_failure(ReplicaError::Identity(molten_raft_core::denial::IdentityError::new("bad record")));
    assert_eq!(encoding, crate::error::Failure::Preserves("bad record".to_owned()));

    let labelled = replica_failure(ReplicaError::ContentRef {
        label: Some("Raft election timer ref"),
        issue: molten_raft_core::denial::refs::ContentRefIssue::MissingPrefix {
            value: "token".to_owned(),
        },
    });
    assert_eq!(
        labelled.to_string(),
        "invalid harness artifact: invalid Raft election timer ref: invalid harness artifact: content ref must start \
         with blake3:, got token"
    );
}

// r[verify molten.consensus.no_std_raft_and_reliability_kernels]
#[test]
fn live_transitions_name_timers_and_snapshots_by_canonical_preserves_identity() {
    let leader = tests::committed_leader();
    assert_eq!(
        leader.active_election_timer_ref,
        election_timer_ref(
            &leader.profile.group_binding_ref,
            &leader.node_id,
            leader.profile.service_generation,
            leader.current_term,
            leader.election_timer_sequence,
        )
        .expect("canonical timer ref")
    );
    let snapshot = apply_replica_event(&leader, ReplicaEvent::CreateSnapshot {
        application_state_ref: tests::test_ref("canonical-identity-application-state"),
    })
    .expect("snapshot step");
    let stored = snapshot.next.snapshot.clone().expect("stored snapshot");
    assert_eq!(stored.snapshot_ref, snapshot_ref(&stored).expect("canonical snapshot ref"));

    let mut tampered = stored.clone();
    tampered.snapshot_ref = tests::test_ref("not-the-canonical-snapshot");
    let mut state = snapshot.next.clone();
    state.snapshot = Some(tampered);
    let error =
        apply_replica_event(&state, ReplicaEvent::HeartbeatTimeout).expect_err("a non-canonical ref is refused");
    assert_eq!(error, crate::error::Failure::invalid_harness("Raft snapshot binding or identity mismatch"));
}

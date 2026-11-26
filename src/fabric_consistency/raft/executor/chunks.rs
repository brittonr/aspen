//! Effect-bounded commit through the live executor: a full message batch commits in chunks that
//! each fit the fenced effect bound.

use super::*;

/// The reliability service's admitted per-step effect bound (`reliability/service/recipe/node.rs`).
const RELIABILITY_EFFECTS_PER_STEP: usize = 128;
/// A full `AppendEntries` batch commits in two steps: the effect budget leaves room for this many
/// proposal outcomes beside the commit record, the application batch and both intent fences.
const FIRST_COMMIT_CHUNK: u64 = 124;

fn apply(state: &ReplicaState, event: ReplicaEvent) -> ReplicaTransition {
    apply_replica_event(state, event).expect("admitted transition")
}

fn propose(state: &ReplicaState, label: &str) -> ReplicaTransition {
    apply(state, ReplicaEvent::Propose {
        request_ref: super::tests::test_ref(label),
        command_ref: super::tests::test_ref(&format!("{label}-command")),
        command_schema_ref: super::tests::test_ref("liveness-schema"),
    })
}

// r[verify molten.consensus.raft_replication_and_recovery_liveness]
#[tokio::test]
async fn a_full_message_batch_commits_in_effect_bounded_chunks() {
    let (mut leader, mut follower) = super::tests::elect_node_a();
    leader.profile.max_effects_per_step = RELIABILITY_EFFECTS_PER_STEP;
    follower.profile.max_effects_per_step = RELIABILITY_EFFECTS_PER_STEP;
    let mut last_proposal = None;
    for write in 0..MAX_REPLICA_MESSAGE_ENTRIES {
        let proposal = propose(&leader, &format!("batch-write-{write}"));
        leader = proposal.next.clone();
        last_proposal = Some(proposal);
    }
    let last_proposal = last_proposal.expect("batch proposals");
    let append = super::tests::sent_envelope_to(&last_proposal, super::tests::NODE_B);
    assert!(
        matches!(&append.message, Message::AppendEntries { entries, .. } if entries.len() == MAX_REPLICA_MESSAGE_ENTRIES)
    );
    let replicated = apply(&follower, ReplicaEvent::Message { envelope: append });
    let response = super::tests::sent_envelope_to(&replicated, super::tests::NODE_A);

    let mut ports = super::executor_tests::RecordingPorts::default();
    let first = execute_replica_event(
        &leader,
        ReplicaEvent::Message {
            envelope: response.clone(),
        },
        &mut ports,
    )
    .await;
    let ReplicaExecutionOutcome::Applied(first) = first else {
        panic!("the first chunk must fit the fenced effect bound: {first:?}");
    };
    assert_eq!(first.next.commit_index, FIRST_COMMIT_CHUNK);
    assert!(first.observations.len() <= RELIABILITY_EFFECTS_PER_STEP);

    // The next successful response (here a heartbeat acknowledgement) commits the remainder.
    let second = execute_replica_event(&first.next, ReplicaEvent::Message { envelope: response }, &mut ports).await;
    let ReplicaExecutionOutcome::Applied(second) = second else {
        panic!("the remainder must commit: {second:?}");
    };
    let full_batch = u64::try_from(MAX_REPLICA_MESSAGE_ENTRIES).expect("bounded message batch");
    assert_eq!(second.next.commit_index, full_batch);
    assert_eq!(second.next.last_applied, second.next.commit_index);
}

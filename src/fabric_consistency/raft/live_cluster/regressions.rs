//! Live three-replica regressions for the Raft liveness defects: queued timers reaching a replica
//! whose role changed, a `ReadProbe` reaching a lagging follower, and a healed old leader
//! reconciling a retried request ref. Every turn runs the real executor against Redb durability and
//! Iroh transport. The test chooses the delivery order, so a frame it receives and drops models a
//! partition.

use super::*;

const NODE_A: &str = super::super::tests::NODE_A;
const NODE_B: &str = super::super::tests::NODE_B;
const NODE_C: &str = super::super::tests::NODE_C;

async fn three_nodes() -> (Node, Node, Node) {
    let group = super::super::tests::active_group();
    let listener_a = crate::fabric_transport::cross_process::tests::listener_with_secret(NODE_A_SECRET_BYTE).await;
    let listener_b = crate::fabric_transport::cross_process::tests::listener_with_secret(NODE_B_SECRET_BYTE).await;
    let listener_c = crate::fabric_transport::cross_process::tests::listener_with_secret(NODE_C_SECRET_BYTE).await;
    let endpoints = std::collections::BTreeMap::from([
        (NODE_A.to_string(), listener_a.handoff().clone()),
        (NODE_B.to_string(), listener_b.handoff().clone()),
        (NODE_C.to_string(), listener_c.handoff().clone()),
    ]);
    let node_a = setup::build_node(&group, NODE_A, listener_a, &endpoints).await.expect("node A");
    let node_b = setup::build_node(&group, NODE_B, listener_b, &endpoints).await.expect("node B");
    let node_c = setup::build_node(&group, NODE_C, listener_c, &endpoints).await.expect("node C");
    (node_a, node_b, node_c)
}

async fn close(node_a: Node, node_b: Node, node_c: Node) {
    setup::close_node(node_a).await;
    setup::close_node(node_b).await;
    setup::close_node(node_c).await;
}

/// Handle `event` at `node`, which sends nothing.
async fn quiet(node: &mut Node, event: ReplicaEvent) -> crate::error::Result<ExecutedReplicaTransition> {
    workflow::require_applied_transition(node.service.handle_event(event).await)
}

/// Handle `event` at `node` while `to` receives the one frame it sends.
async fn turn_to_one(
    node: &mut Node,
    event: ReplicaEvent,
    to: &mut Node,
) -> crate::error::Result<(ExecutedReplicaTransition, ReplicaEvent)> {
    let session = to.session_ref.clone();
    let (outcome, received) =
        tokio::join!(node.service.handle_event(event), receive_replica_event(&mut to.listener, &session, timeout()),);
    let executed = workflow::require_applied_transition(outcome)?;
    Ok((executed, received?.event))
}

/// Handle `event` at `node` while `first` and `second` each receive the frame it sends them.
async fn turn_to_two(
    node: &mut Node,
    event: ReplicaEvent,
    first: &mut Node,
    second: &mut Node,
) -> crate::error::Result<(ExecutedReplicaTransition, ReplicaEvent, ReplicaEvent)> {
    let first_session = first.session_ref.clone();
    let second_session = second.session_ref.clone();
    let (outcome, first_received, second_received) = tokio::join!(
        node.service.handle_event(event),
        receive_replica_event(&mut first.listener, &first_session, timeout()),
        receive_replica_event(&mut second.listener, &second_session, timeout()),
    );
    let executed = workflow::require_applied_transition(outcome)?;
    Ok((executed, first_received?.event, second_received?.event))
}

fn propose(label: &str) -> ReplicaEvent {
    ReplicaEvent::Propose {
        request_ref: super::super::tests::test_ref(label),
        command_ref: super::super::tests::test_ref(&format!("{label}-command")),
        command_schema_ref: super::super::tests::test_ref("live-cluster-command-schema"),
    }
}

fn campaign(node: &Node) -> ReplicaEvent {
    ReplicaEvent::ElectionTimeout {
        timer_ref: node.service.state().active_election_timer_ref.clone(),
    }
}

fn log_identities(node: &Node) -> Vec<(u64, u64, String)> {
    let log = &node.service.state().log;
    log.iter().map(|entry| (entry.index, entry.term, entry.request_ref.clone())).collect()
}

// r[verify molten.consensus.raft_timer_lifecycle_safety]
#[tokio::test]
async fn replicas_ignore_timers_queued_before_a_role_change_and_keep_serving() {
    let (mut node_a, mut node_b, mut node_c) = three_nodes().await;
    let startup_timer = node_a.service.state().active_election_timer_ref.clone();
    workflow::elect_node_a(&mut node_a, &mut node_b, &mut node_c).await.expect("live election");

    // The startup timer and the candidate's own timer can both still be queued when A leads.
    let leader = node_a.service.state().clone();
    let candidate_timer = leader.active_election_timer_ref.clone();
    for timer_ref in [startup_timer, candidate_timer] {
        let ignored = quiet(&mut node_a, ReplicaEvent::ElectionTimeout { timer_ref }).await;
        assert_eq!(ignored.expect("a queued election timeout at the leader is benign").next, leader);
    }
    let follower = node_b.service.state().clone();
    let ignored = quiet(&mut node_b, ReplicaEvent::HeartbeatTimeout).await;
    assert_eq!(ignored.expect("a heartbeat tick at a follower is benign").next, follower);
    let request_ref = super::super::tests::test_ref("live-after-stale-election-timeouts");
    workflow::replicate_request(&mut node_a, &mut node_b, &mut node_c, &request_ref)
        .await
        .expect("the leader keeps serving");
    assert_eq!(node_a.service.state().commit_index, INITIAL_LOG_INDEX);

    // B campaigns at a higher term; A steps down and grants, and its queued heartbeat tick is benign.
    let timeout_b = campaign(&node_b);
    let (_, vote_request_a, _) =
        turn_to_two(&mut node_b, timeout_b, &mut node_a, &mut node_c).await.expect("B campaigns");
    let (stepped_down, grant) = turn_to_one(&mut node_a, vote_request_a, &mut node_b).await.expect("A votes");
    assert_eq!(stepped_down.next.role, ReplicaRole::Follower);
    let stepped_down = stepped_down.next;
    let ignored = quiet(&mut node_a, ReplicaEvent::HeartbeatTimeout).await;
    assert_eq!(ignored.expect("a stepped-down leader ignores its pending tick").next, stepped_down);

    let (won, append_a, _) = turn_to_two(&mut node_b, grant, &mut node_a, &mut node_c).await.expect("B wins");
    assert_eq!(won.next.role, ReplicaRole::Leader);
    let (_, response) = turn_to_one(&mut node_a, append_a, &mut node_b).await.expect("A follows B");
    quiet(&mut node_b, response).await.expect("B records A's acknowledgement");
    let (_, append_a, _) = turn_to_two(&mut node_b, propose("live-after-step-down"), &mut node_a, &mut node_c)
        .await
        .expect("B proposes");
    let (_, response) = turn_to_one(&mut node_a, append_a, &mut node_b).await.expect("A replicates");
    let committed = quiet(&mut node_b, response).await.expect("B commits with A");
    assert_eq!(committed.next.commit_index, INITIAL_LOG_INDEX + 1);
    close(node_a, node_b, node_c).await;
}

// r[verify molten.consensus.raft_replication_and_recovery_liveness]
#[tokio::test]
async fn a_lagging_follower_acknowledges_a_read_probe_that_completes_a_quorum_read() {
    let (mut node_a, mut node_b, mut node_c) = three_nodes().await;
    workflow::elect_node_a(&mut node_a, &mut node_b, &mut node_c).await.expect("live election");
    // A commits index 1 with B; the append to C is dropped, so C lags the read index.
    let request_ref = super::super::tests::test_ref("live-lagging-follower-write");
    workflow::replicate_until_commit(&mut node_a, &mut node_b, &mut node_c, &request_ref)
        .await
        .expect("commit with B");
    assert_eq!(node_a.service.state().commit_index, INITIAL_LOG_INDEX);
    assert!(node_c.service.state().log.is_empty());

    let read = ReplicaEvent::Read {
        request_ref: super::super::tests::test_ref("live-lagging-follower-read"),
        mode: crate::fabric_consistency::ReadMode::Linearizable,
    };
    let (_, _probe_b, probe_c) = turn_to_two(&mut node_a, read, &mut node_b, &mut node_c).await.expect("probes");
    assert!(matches!(&probe_c, ReplicaEvent::Message {
        envelope: ReplicaMessageEnvelope {
            message: Message::ReadProbe {
                required_index: INITIAL_LOG_INDEX,
                ..
            },
            ..
        }
    }));
    let (acknowledged, acknowledgement) = turn_to_one(&mut node_c, probe_c, &mut node_a)
        .await
        .expect("a lagging follower acknowledges the probe");
    assert_eq!(acknowledged.next.current_term, LIVE_TERM);
    assert_eq!(acknowledged.next.leader_id.as_deref(), Some(NODE_A));
    assert!(acknowledged.next.log.is_empty(), "acknowledging needs no catch-up");

    let served = quiet(&mut node_a, acknowledgement).await.expect("the leader serves the read");
    let kinds = served.observations.iter().map(|observation| observation.kind).collect::<Vec<_>>();
    assert!(kinds.contains(&ReplicaEffectKind::ReadOutcome), "C's acknowledgement completes the read");
    assert!(node_a.service.state().pending_reads.is_empty());
    close(node_a, node_b, node_c).await;
}

// r[verify molten.consensus.raft_replication_and_recovery_liveness]
#[tokio::test]
async fn a_healed_old_leader_durably_truncates_its_uncommitted_suffix_for_the_committed_retry() {
    let (mut node_a, mut node_b, mut node_c) = three_nodes().await;
    workflow::elect_node_a(&mut node_a, &mut node_b, &mut node_c).await.expect("live election");
    let retried_ref = super::super::tests::test_ref("live-partition-retried-request");
    // A is partitioned: both appends are dropped, leaving (1, 1, R0) and (2, 1, R1) uncommitted.
    for label in ["live-partition-isolated-request", "live-partition-retried-request"] {
        turn_to_two(&mut node_a, propose(label), &mut node_b, &mut node_c).await.expect("isolated proposal");
    }
    assert_eq!(node_a.service.state().log[1].request_ref, retried_ref);

    // B leads term 2 with C's vote and commits the client's retry at index 1.
    let timeout_b = campaign(&node_b);
    let (_, vote_request_a, vote_request_c) =
        turn_to_two(&mut node_b, timeout_b, &mut node_a, &mut node_c).await.expect("B campaigns");
    let (_, grant) = turn_to_one(&mut node_c, vote_request_c, &mut node_b).await.expect("C votes");
    let (_, initial_a, initial_c) = turn_to_two(&mut node_b, grant, &mut node_a, &mut node_c).await.expect("B wins");
    let (_, response) = turn_to_one(&mut node_c, initial_c, &mut node_b).await.expect("C follows B");
    quiet(&mut node_b, response).await.expect("B records C");
    let (_, append_a, append_c) =
        turn_to_two(&mut node_b, propose("live-partition-retried-request"), &mut node_a, &mut node_c)
            .await
            .expect("retry at B");
    let (_, response) = turn_to_one(&mut node_c, append_c, &mut node_b).await.expect("C replicates");
    assert_eq!(quiet(&mut node_b, response).await.expect("B commits").next.commit_index, INITIAL_LOG_INDEX);

    // The partition heals: A sees B's term, then B's appends.
    let (_, refusal) = turn_to_one(&mut node_a, vote_request_a, &mut node_b).await.expect("A steps down");
    quiet(&mut node_b, refusal).await.expect("B ignores the late vote");
    let (_, response) = turn_to_one(&mut node_a, initial_a, &mut node_b).await.expect("A accepts B");
    quiet(&mut node_b, response).await.expect("B records A");
    let (_, response) = turn_to_one(&mut node_a, append_a, &mut node_b)
        .await
        .expect("A truncates its suffix and accepts the retry");
    assert_eq!(log_identities(&node_a), vec![(INITIAL_LOG_INDEX, LIVE_TERM + 1, retried_ref.clone())]);
    quiet(&mut node_b, response).await.expect("B records A's match");

    let (_, heartbeat_a, _) = turn_to_two(&mut node_b, ReplicaEvent::HeartbeatTimeout, &mut node_a, &mut node_c)
        .await
        .expect("tick");
    let (applied, response) = turn_to_one(&mut node_a, heartbeat_a, &mut node_b).await.expect("A applies");
    assert_eq!(applied.next.commit_index, INITIAL_LOG_INDEX);
    assert_eq!(node_a.service.ports().application.handler().applied_request_refs, vec![retried_ref]);
    quiet(&mut node_b, response).await.expect("B records A's commit");
    close(node_a, node_b, node_c).await;
}

use super::*;
#[tokio::test]
async fn candidate_can_win_with_one_unavailable_peer() {
    let group = super::tests::active_group();
    let state = super::tests::started_state(&group, super::tests::NODE_A);
    let follower = super::tests::started_state(&group, super::tests::NODE_C);
    let mut ports = RecordingPorts {
        unavailable_peer: Some(super::tests::NODE_B.to_string()),
        ..RecordingPorts::default()
    };
    let outcome = execute_replica_event(
        &state,
        ReplicaEvent::ElectionTimeout {
            timer_ref: state.active_election_timer_ref.clone(),
        },
        &mut ports,
    )
    .await;
    let ReplicaExecutionOutcome::Applied(candidate) = outcome else {
        panic!("message loss must not abort the election");
    };
    assert_eq!(candidate.next.role, ReplicaRole::Candidate);
    assert_eq!(candidate.next.lifecycle, ReplicaLifecycle::Running);
    assert_eq!(candidate.next.voted_for.as_deref(), Some(super::tests::NODE_A));
    assert_eq!(candidate.transport_failures.len(), 1);
    assert!(candidate.transport_failures[0].diagnostic.contains("peer unavailable"));
    let lost_sequence = candidate.transport_failures[0].sequence;
    assert!(
        !candidate.observations.iter().any(|item| item.sequence == lost_sequence),
        "loss must never produce success evidence"
    );
    assert!(candidate.observations.windows(2).all(|pair| pair[0].sequence < pair[1].sequence));
    assert!(ports.executed.contains(&ReplicaEffectKind::ArmElectionTimer));
    assert!(
        ports.hard_states.last().is_some_and(|state| !state.2),
        "message loss alone must not retain recovery intent"
    );

    let request = ports
        .sent
        .iter()
        .find(|message| message.to == follower.node_id)
        .expect("reachable peer request")
        .clone();
    let mut follower_ports = RecordingPorts::default();
    let vote = execute_replica_event(&follower, ReplicaEvent::Message { envelope: request }, &mut follower_ports).await;
    assert!(matches!(vote, ReplicaExecutionOutcome::Applied(_)));
    let response = follower_ports.sent.pop().expect("durable vote response");
    let won = execute_replica_event(&candidate.next, ReplicaEvent::Message { envelope: response }, &mut ports).await;
    let ReplicaExecutionOutcome::Applied(leader) = won else {
        panic!("the live majority must elect a leader");
    };
    assert_eq!(leader.next.role, ReplicaRole::Leader);
    assert_eq!(leader.next.lifecycle, ReplicaLifecycle::Running);
    assert_eq!(leader.next.current_term, candidate.next.current_term);
    assert_eq!(ports.executed.last(), Some(&ReplicaEffectKind::ArmHeartbeatTimer));
}

#[tokio::test]
async fn leader_keeps_heartbeats_and_commits_with_one_unavailable_peer() {
    let (leader, follower) = super::tests::elect_node_a();
    let mut ports = RecordingPorts {
        unavailable_peer: Some(super::tests::NODE_C.to_string()),
        ..RecordingPorts::default()
    };
    let heartbeat = execute_replica_event(&leader, ReplicaEvent::HeartbeatTimeout, &mut ports).await;
    let ReplicaExecutionOutcome::Applied(heartbeat) = heartbeat else {
        panic!("one lost heartbeat must not revoke leadership");
    };
    assert_eq!(heartbeat.next.role, ReplicaRole::Leader);
    assert_eq!(heartbeat.next.current_term, leader.current_term);
    assert_eq!(heartbeat.transport_failures.len(), 1);
    assert_eq!(ports.executed.last(), Some(&ReplicaEffectKind::ArmHeartbeatTimer));
    ports.sent.clear();
    let request_ref = super::tests::test_ref("unavailable-peer-proposal");
    let proposal = execute_replica_event(
        &heartbeat.next,
        ReplicaEvent::Propose {
            request_ref: request_ref.clone(),
            command_ref: super::tests::test_ref("unavailable-peer-command"),
            command_schema_ref: super::tests::test_ref("unavailable-peer-schema"),
        },
        &mut ports,
    )
    .await;
    let ReplicaExecutionOutcome::Applied(proposal) = proposal else {
        panic!("message loss must not discard the durable proposal");
    };
    let append = ports.sent.iter().find(|message| message.to == follower.node_id).expect("reachable append").clone();
    let mut follower_ports = RecordingPorts::default();
    let replicated =
        execute_replica_event(&follower, ReplicaEvent::Message { envelope: append }, &mut follower_ports).await;
    assert!(matches!(replicated, ReplicaExecutionOutcome::Applied(_)));
    let response = follower_ports.sent.pop().expect("durable append response");
    let committed =
        execute_replica_event(&proposal.next, ReplicaEvent::Message { envelope: response }, &mut ports).await;
    let ReplicaExecutionOutcome::Applied(committed) = committed else {
        panic!("the reachable majority must commit");
    };
    assert_eq!(committed.next.role, ReplicaRole::Leader);
    assert_eq!(committed.next.lifecycle, ReplicaLifecycle::Running);
    assert_eq!(committed.next.completed_requests.get(&request_ref), Some(&committed.next.commit_index));
    assert_eq!(committed.next.last_applied, committed.next.commit_index);
    assert_eq!(committed.next.commit_index, proposal.next.log.last().expect("proposal entry").index);
    assert!(ports.executed.contains(&ReplicaEffectKind::ApplyCommitted));
}

#[tokio::test]
async fn failed_vote_storage_never_sends_a_vote_or_campaigns() {
    let group = super::tests::active_group();
    let candidate = super::tests::started_state(&group, super::tests::NODE_A);
    let follower = super::tests::started_state(&group, super::tests::NODE_B);
    let election = apply_replica_event(&candidate, ReplicaEvent::ElectionTimeout {
        timer_ref: candidate.active_election_timer_ref.clone(),
    })
    .expect("election plan");
    let request = super::tests::sent_envelope_to(&election, super::tests::NODE_B);
    let mut ports = RecordingPorts {
        fail_on: Some(ReplicaEffectKind::PersistHardState),
        ..RecordingPorts::default()
    };
    let vote = execute_replica_event(
        &follower,
        ReplicaEvent::Message {
            envelope: request.clone(),
        },
        &mut ports,
    )
    .await;
    let ReplicaExecutionOutcome::Failed(failed) = vote else {
        panic!("uncertain vote storage must quarantine");
    };
    assert_eq!(failed.retained.lifecycle, ReplicaLifecycle::Stopped);
    assert_eq!(failed.failed_kind, ReplicaEffectKind::PersistHardState);
    assert!(ports.sent.is_empty());
    ports.fail_on = None;
    let retry = execute_replica_event(&failed.retained, ReplicaEvent::Message { envelope: request }, &mut ports).await;
    assert!(matches!(retry, ReplicaExecutionOutcome::Denied { .. }));
    let campaign = execute_replica_event(
        &failed.retained,
        ReplicaEvent::ElectionTimeout {
            timer_ref: failed.retained.active_election_timer_ref.clone(),
        },
        &mut ports,
    )
    .await;
    assert!(matches!(campaign, ReplicaExecutionOutcome::Denied { .. }));
    assert!(ports.sent.is_empty());
}

#[tokio::test]
async fn failed_timer_after_an_unavailable_peer_still_quarantines_and_retains_failure() {
    let (leader, _) = super::tests::elect_node_a();
    let mut ports = RecordingPorts {
        unavailable_peer: Some(super::tests::NODE_C.to_string()),
        fail_on: Some(ReplicaEffectKind::ArmHeartbeatTimer),
        ..RecordingPorts::default()
    };
    let outcome = execute_replica_event(&leader, ReplicaEvent::HeartbeatTimeout, &mut ports).await;
    let ReplicaExecutionOutcome::Failed(failed) = outcome else {
        panic!("only transport errors are message loss");
    };
    assert_eq!(failed.failed_kind, ReplicaEffectKind::ArmHeartbeatTimer);
    assert_eq!(failed.retained.lifecycle, ReplicaLifecycle::Stopped);
    assert!(matches!(failed.retained.rejoin, ReplicaRejoin::Recovering(_)));
    assert_eq!(failed.transport_failures.len(), 1);
    assert!(failed.transport_failures[0].diagnostic.contains("peer unavailable"));
    assert_eq!(failed.completed.iter().filter(|item| item.kind == ReplicaEffectKind::Send).count(), 1);
}

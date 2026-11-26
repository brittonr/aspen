
pub(super) fn elect_node_a() -> (ReplicaState, ReplicaState) {
    let group = active_group();
    let node_a = started_state(&group, NODE_A);
    let node_b = started_state(&group, NODE_B);
    let election = apply_replica_event(&node_a, ReplicaEvent::ElectionTimeout {
        timer_ref: node_a.active_election_timer_ref.clone(),
    })
    .expect("node A election");
    let vote_request = sent_envelope_to(&election, NODE_B);
    let vote =
        apply_replica_event(&node_b, ReplicaEvent::Message { envelope: vote_request }).expect("node B vote response");
    let vote_response = sent_envelope_to(&vote, NODE_A);
    let leader = apply_replica_event(&election.next, ReplicaEvent::Message {
        envelope: vote_response,
    })
    .expect("node A leadership");
    assert_eq!(leader.next.role, ReplicaRole::Leader);
    (leader.next, vote.next)
}

pub(super) fn committed_leader() -> ReplicaState {
    let (leader, follower) = elect_node_a();
    let proposal = apply_replica_event(&leader, ReplicaEvent::Propose {
        request_ref: test_ref("committed-helper-request"),
        command_ref: test_ref("committed-helper-command"),
        command_schema_ref: test_ref("committed-helper-schema"),
    })
    .expect("helper proposal");
    let append = sent_envelope_to(&proposal, NODE_B);
    let replicated =
        apply_replica_event(&follower, ReplicaEvent::Message { envelope: append }).expect("helper follower append");
    let response = sent_envelope_to(&replicated, NODE_A);
    apply_replica_event(&proposal.next, ReplicaEvent::Message { envelope: response })
        .expect("helper majority commit")
        .next
}

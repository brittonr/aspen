use super::*;

pub(super) async fn elect_node_a(node_a: &mut LiveNode, node_b: &mut LiveNode, node_c: &mut LiveNode) -> Result<()> {
    let timer_ref = node_a.service.state().active_election_timer_ref.clone();
    let session_b = node_b.session_ref.clone();
    let session_c = node_c.session_ref.clone();
    let election = node_a.service.handle_event(ReplicaEvent::ElectionTimeout { timer_ref });
    let receive_b = receive_replica_event(&mut node_b.listener, &session_b, live_timeout());
    let receive_c = receive_replica_event(&mut node_c.listener, &session_c, live_timeout());
    let (election, vote_request_b, vote_request_c) = tokio::join!(election, receive_b, receive_c);
    require_applied(election)?;
    let vote_request_b = vote_request_b?;
    let _vote_request_c = vote_request_c?;

    let session_a = node_a.session_ref.clone();
    let vote = node_b.service.handle_event(vote_request_b.event);
    let receive_a = receive_replica_event(&mut node_a.listener, &session_a, live_timeout());
    let (vote, vote_response) = tokio::join!(vote, receive_a);
    require_applied(vote)?;

    let session_b = node_b.session_ref.clone();
    let session_c = node_c.session_ref.clone();
    let leadership = node_a.service.handle_event(vote_response?.event);
    let receive_b = receive_replica_event(&mut node_b.listener, &session_b, live_timeout());
    let receive_c = receive_replica_event(&mut node_c.listener, &session_c, live_timeout());
    let (leadership, initial_append_b, initial_append_c) = tokio::join!(leadership, receive_b, receive_c);
    require_applied(leadership)?;
    let _initial_append_b = initial_append_b?;
    let _initial_append_c = initial_append_c?;
    Ok(())
}

pub(super) async fn replicate_request(
    node_a: &mut LiveNode,
    node_b: &mut LiveNode,
    node_c: &mut LiveNode,
    request_ref: &str,
) -> Result<()> {
    let session_b = node_b.session_ref.clone();
    let session_c = node_c.session_ref.clone();
    let proposal = node_a.service.handle_event(ReplicaEvent::Propose {
        request_ref: request_ref.to_string(),
        command_ref: test_ref("live-cluster-command"),
        command_schema_ref: test_ref("live-cluster-command-schema"),
    });
    let receive_b = receive_replica_event(&mut node_b.listener, &session_b, live_timeout());
    let receive_c = receive_replica_event(&mut node_c.listener, &session_c, live_timeout());
    let (proposal, append_b, append_c) = tokio::join!(proposal, receive_b, receive_c);
    require_applied(proposal)?;
    let append_b = append_b?;
    let _append_c = append_c?;

    let session_a = node_a.session_ref.clone();
    let follower_append = node_b.service.handle_event(append_b.event);
    let receive_a = receive_replica_event(&mut node_a.listener, &session_a, live_timeout());
    let (follower_append, append_response) = tokio::join!(follower_append, receive_a);
    require_applied(follower_append)?;
    require_applied(node_a.service.handle_event(append_response?.event).await)?;
    replicate_commit_notice(node_a, node_b, node_c).await
}

async fn replicate_commit_notice(node_a: &mut LiveNode, node_b: &mut LiveNode, node_c: &mut LiveNode) -> Result<()> {
    let session_b = node_b.session_ref.clone();
    let session_c = node_c.session_ref.clone();
    let heartbeat = node_a.service.handle_event(ReplicaEvent::HeartbeatTimeout);
    let receive_b = receive_replica_event(&mut node_b.listener, &session_b, live_timeout());
    let receive_c = receive_replica_event(&mut node_c.listener, &session_c, live_timeout());
    let (heartbeat, commit_b, commit_c) = tokio::join!(heartbeat, receive_b, receive_c);
    require_applied(heartbeat)?;
    let commit_b = commit_b?;
    let _commit_c = commit_c?;

    let session_a = node_a.session_ref.clone();
    let follower_commit = node_b.service.handle_event(commit_b.event);
    let receive_a = receive_replica_event(&mut node_a.listener, &session_a, live_timeout());
    let (follower_commit, response) = tokio::join!(follower_commit, receive_a);
    require_applied(follower_commit)?;
    require_applied(node_a.service.handle_event(response?.event).await)?;
    Ok(())
}

fn require_applied(outcome: ReplicaExecutionOutcome) -> Result<()> {
    match outcome {
        ReplicaExecutionOutcome::Applied(_) => Ok(()),
        ReplicaExecutionOutcome::Denied { diagnostic, .. } => {
            Err(crate::error::MoltenError::invalid_harness(format!("live cluster turn denied: {diagnostic}")))
        }
        ReplicaExecutionOutcome::Failed(failed) => Err(crate::error::MoltenError::invalid_harness(format!(
            "live cluster effect {} failed: {}",
            failed.failed_kind.as_str(),
            failed.diagnostic
        ))),
    }
}

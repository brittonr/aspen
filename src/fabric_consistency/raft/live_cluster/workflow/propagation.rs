/// Notify the followers of an applied commit and handle the first follower's response.
pub(super) async fn notify_followers(
    node_a: &mut super::super::Node,
    node_b: &mut super::super::Node,
    node_c: &mut super::super::Node,
) -> crate::error::Result<()> {
    let session_b = node_b.session_ref.clone();
    let session_c = node_c.session_ref.clone();
    let heartbeat = node_a.service.handle_event(crate::fabric_consistency::raft::model::ReplicaEvent::HeartbeatTimeout);
    let receive_b = crate::fabric_consistency::raft::iroh::receive_replica_event(
        &mut node_b.listener,
        &session_b,
        super::super::timeout(),
    );
    let receive_c = crate::fabric_consistency::raft::iroh::receive_replica_event(
        &mut node_c.listener,
        &session_c,
        super::super::timeout(),
    );
    let (heartbeat, commit_b, commit_c) = tokio::join!(heartbeat, receive_b, receive_c);
    super::require_applied(heartbeat)?;
    let commit_b = commit_b?;
    let _commit_c = commit_c?;

    let session_a = node_a.session_ref.clone();
    let follower_commit = node_b.service.handle_event(commit_b.event);
    let receive_a = crate::fabric_consistency::raft::iroh::receive_replica_event(
        &mut node_a.listener,
        &session_a,
        super::super::timeout(),
    );
    let (follower_commit, response) = tokio::join!(follower_commit, receive_a);
    super::require_applied(follower_commit)?;
    super::require_applied(node_a.service.handle_event(response?.event).await)?;
    Ok(())
}

//! The fresh generation: node A leads and commits one request with a quorum
//! acknowledgement, nodes B and C follow, and every replica publishes its
//! checkpoint receipt once the producer asks for it.

/// Node A: campaign on the start signal, commit one request with quorum while
/// node C withholds acknowledgement, then catch node C up.
pub(super) async fn leader(
    node: &mut super::Node,
    run_directory: &std::path::Path,
    deadline: &mut super::Deadline,
) -> crate::error::Result<()> {
    super::publish(run_directory, &node.node_id, "ready")?;
    super::await_value(&run_directory.join(super::super::receipt::START_FILE), deadline).await?;
    super::election_timeout(node).await?;
    let mut is_proposed = false;
    let mut is_catch_up_started = false;
    let mut is_done = false;
    for _turn in 0..super::PHASE_TURNS {
        deadline.require_before()?;
        checkpoint_if_requested(node, run_directory)?;
        let Some(event) = super::poll_event(node).await? else {
            continue;
        };
        super::require_applied(node.service.handle_event(event).await)?;
        let state = node.service.state();
        if state.role == crate::fabric_consistency::raft::ReplicaRole::Leader && !is_proposed {
            super::require_applied(
                node.service
                    .handle_event(crate::fabric_consistency::raft::ReplicaEvent::Propose {
                        request_ref: super::request_ref(),
                        command_ref: super::command_ref(),
                        command_schema_ref: super::super::fixture::command_schema_ref(),
                    })
                    .await,
            )?;
            is_proposed = true;
        }
        if node.service.state().commit_index >= crate::fabric_consistency::raft::INITIAL_LOG_INDEX
            && !is_catch_up_started
        {
            super::super::super::peer::write_value(
                &run_directory.join(super::super::receipt::CATCH_UP_FILE),
                &super::super::receipt::signal("catch-up"),
            )?;
            super::require_applied(
                node.service.handle_event(crate::fabric_consistency::raft::ReplicaEvent::HeartbeatTimeout).await,
            )?;
            is_catch_up_started = true;
        }
        if is_catch_up_started && !is_done && is_caught_up(node) {
            super::super::super::peer::write_value(
                &run_directory.join(super::super::receipt::LEADER_DONE_FILE),
                &super::super::receipt::signal("leader-done"),
            )?;
            is_done = true;
        }
    }
    Err(super::exceeded_bound())
}

/// Whether both followers acknowledged the committed entry.
fn is_caught_up(node: &super::Node) -> bool {
    let match_index = &node.service.state().match_index;
    [super::super::fixture::NODE_B, super::super::fixture::NODE_C].iter().all(|peer| {
        match_index
            .get(*peer)
            .is_some_and(|index| *index >= crate::fabric_consistency::raft::INITIAL_LOG_INDEX)
    })
}

/// Nodes B and C follow; node C withholds every entry-carrying or
/// commit-advancing frame until the leader has committed, so the commit quorum
/// is exactly the leader and node B.
pub(super) async fn follower(
    node: &mut super::Node,
    run_directory: &std::path::Path,
    deadline: &mut super::Deadline,
) -> crate::error::Result<()> {
    super::publish(run_directory, &node.node_id, "ready")?;
    let is_lagging = node.node_id == super::super::fixture::NODE_C;
    for _turn in 0..super::PHASE_TURNS {
        deadline.require_before()?;
        checkpoint_if_requested(node, run_directory)?;
        let Some(event) = super::poll_event(node).await? else {
            continue;
        };
        if is_lagging && !run_directory.join(super::super::receipt::CATCH_UP_FILE).is_file() && advances_commit(&event)
        {
            continue;
        }
        super::require_applied(node.service.handle_event(event).await)?;
    }
    Err(super::exceeded_bound())
}

fn advances_commit(event: &crate::fabric_consistency::raft::ReplicaEvent) -> bool {
    matches!(event, crate::fabric_consistency::raft::ReplicaEvent::Message {
        envelope: crate::fabric_consistency::raft::ReplicaMessageEnvelope {
            message: crate::fabric_consistency::raft::Message::AppendEntries { entries, leader_commit, .. },
            ..
        }
    } if !entries.is_empty() || *leader_commit > crate::fabric_consistency::raft::INITIAL_COMMIT_INDEX)
}

fn checkpoint_if_requested(node: &super::Node, run_directory: &std::path::Path) -> crate::error::Result<()> {
    let output = super::super::receipt::node_file(run_directory, &node.node_id, "checkpoint");
    let state = node.service.state();
    if !run_directory.join(super::super::receipt::CHECKPOINT_FILE).is_file()
        || output.is_file()
        || state.commit_index < crate::fabric_consistency::raft::INITIAL_LOG_INDEX
        || state.last_applied < crate::fabric_consistency::raft::INITIAL_LOG_INDEX
    {
        return Ok(());
    }
    super::super::super::peer::write_value(&output, &super::node_receipt(node)?.value())
}

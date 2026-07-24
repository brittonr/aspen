use super::*;
use crate::fabric_consistency::raft::live_cluster;

pub(super) async fn run(
    node_id: String,
    endpoint_identity: String,
    mut node: live_cluster::LiveNode,
    run_directory: &Path,
) -> Result<()> {
    let listener = node.listener.take().ok_or_else(|| MoltenError::invalid_harness("recovered listener is absent"))?;
    let mut ingress = IrohReplicaIngressPump::spawn(listener, IrohReplicaIngressConfig {
        session_ref: node.session_ref.clone(),
        accept_timeout: Duration::from_secs(CHILD_TIMEOUT_SECONDS),
        event_capacity: INGRESS_EVENT_CAPACITY,
        delivery_limit: INGRESS_DELIVERY_LIMIT,
    })?;
    write_signal(&run_directory.join(format!("{node_id}-ready.preserves")), "recovered-ready")?;
    if node_id == NODE_B {
        wait_for_file(&run_directory.join(RESTART_START_FILE))?;
        let timer_ref = node.service.state().active_election_timer_ref.clone();
        child::require_applied(node.service.handle_event(ReplicaEvent::ElectionTimeout { timer_ref }).await)?;
    }
    let mut stale_frame_sent = false;
    for _step in 0..EVENT_LOOP_LIMIT {
        if run_directory.join(STOP_FILE).is_file() {
            node.listener = Some(ingress.shutdown().await?);
            return child::finish_node(node_id, endpoint_identity, node, run_directory).await;
        }
        if node_id == NODE_A && !stale_frame_sent && run_directory.join(RECOVERED_LEADER_FILE).is_file() {
            send_stale_leader_frame(&mut node).await?;
            stale_frame_sent = true;
        }
        let Some(event) = child::poll_event(&mut ingress).await? else {
            continue;
        };
        let message_term = match &event.event {
            ReplicaEvent::Message { envelope } => Some(envelope.message.term()),
            _ => None,
        };
        child::require_applied(node.service.handle_event(event.event).await)?;
        if node_id == NODE_B && node.service.state().role == ReplicaRole::Leader {
            write_signal(&run_directory.join(RECOVERED_LEADER_FILE), "recovered-leader")?;
            if message_term == Some(STALE_LEADER_TERM) && node.service.state().current_term == RECOVERED_LEADER_TERM {
                write_signal(&run_directory.join(STALE_FENCED_FILE), "stale-fenced")?;
            }
        }
    }
    Err(MoltenError::invalid_harness("recovered replica exhausted its bounded event loop"))
}

async fn send_stale_leader_frame(node: &mut live_cluster::LiveNode) -> Result<()> {
    let state = node.service.state();
    let envelope = ReplicaMessageEnvelope {
        group_binding_ref: state.profile.group_binding_ref.clone(),
        service_generation: state.profile.service_generation,
        from: NODE_A.to_string(),
        to: NODE_B.to_string(),
        message: RaftMessage::AppendEntries {
            term: STALE_LEADER_TERM,
            leader_id: NODE_A.to_string(),
            prev_log_index: INITIAL_LOG_INDEX,
            prev_log_term: STALE_LEADER_TERM,
            entries: Vec::new(),
            leader_commit: INITIAL_LOG_INDEX,
            config_epoch: state.membership.config_epoch,
            fencing_epoch: state.profile.fencing_epoch,
        },
    };
    let _evidence_ref = node.service.ports_mut().transport.send(&envelope).await?;
    Ok(())
}

use super::*;

pub(super) async fn run(
    node_id: String,
    endpoint_identity: String,
    mut node: crate::fabric_consistency::raft::live_cluster::Node,
    run_directory: &std::path::Path,
) -> crate::error::Result<()> {
    if node.service.ports().durability.adapter().is_repair_observed()
        && !matches!(node.service.state().rejoin, ReplicaRejoin::Recovering(_))
    {
        return Err(crate::error::Failure::invalid_harness("repaired live node bypassed peer-query admission"));
    }
    let listener = node
        .listener
        .take()
        .ok_or_else(|| crate::error::Failure::invalid_harness("recovered listener is absent"))?;
    let mut ingress = ReplicaIngressPump::spawn(listener, ReplicaIngressConfig {
        session_ref: node.session_ref.clone(),
        accept_timeout: std::time::Duration::from_secs(CHILD_TIMEOUT_SECONDS),
        event_capacity: INGRESS_EVENT_CAPACITY,
        delivery_limit: INGRESS_DELIVERY_LIMIT,
    })?;
    write_signal(&run_directory.join(format!("{node_id}-ready.preserves")), "recovered-ready")?;
    wait_for_file(&run_directory.join(RESTART_START_FILE))?;
    if matches!(node.service.state().rejoin, ReplicaRejoin::Recovering(_)) {
        let timer_ref = node.service.state().active_election_timer_ref.clone();
        child::require_applied(node.service.handle_event(ReplicaEvent::ElectionTimeout { timer_ref }).await)?;
    }
    let mut is_campaign_started = false;
    let mut is_stale_frame_sent = false;
    for step in 0..EVENT_LOOP_LIMIT {
        is_campaign_started = drive_recovery(&mut node, &node_id, run_directory, step, is_campaign_started).await?;
        if run_directory.join(STOP_FILE).is_file() {
            if node.service.state().rejoin != ReplicaRejoin::Voter {
                return Err(crate::error::Failure::invalid_harness(
                    "recovered live node never regained voting admission",
                ));
            }
            node.listener = Some(ingress.shutdown().await?);
            return child::finish_node(node_id, endpoint_identity, node, run_directory).await;
        }
        if node_id == super::super::tests::NODE_A
            && !is_stale_frame_sent
            && run_directory.join(RECOVERED_LEADER_FILE).is_file()
        {
            send_stale_leader_frame(&mut node).await?;
            is_stale_frame_sent = true;
        }
        let Some(event) = child::poll_event(&mut ingress).await? else {
            continue;
        };
        let message_term = match &event.event {
            ReplicaEvent::Message { envelope } => Some(envelope.message.term()),
            _ => None,
        };
        child::require_applied(node.service.handle_event(event.event).await)?;
        if node_id == super::super::tests::NODE_B && node.service.state().role == ReplicaRole::Leader {
            write_signal(&run_directory.join(RECOVERED_LEADER_FILE), "recovered-leader")?;
            if message_term == Some(STALE_LEADER_TERM) && node.service.state().current_term > STALE_LEADER_TERM {
                write_signal(&run_directory.join(STALE_FENCED_FILE), "stale-fenced")?;
            }
        }
    }
    Err(crate::error::Failure::invalid_harness("recovered replica exhausted its bounded event loop"))
}

async fn send_stale_leader_frame(
    node: &mut crate::fabric_consistency::raft::live_cluster::Node,
) -> crate::error::Result<()> {
    let state = node.service.state();
    let envelope = ReplicaMessageEnvelope {
        group_binding_ref: state.profile.group_binding_ref.clone(),
        service_generation: state.profile.service_generation,
        from: super::super::tests::NODE_A.to_string(),
        to: super::super::tests::NODE_B.to_string(),
        message: Message::AppendEntries {
            term: STALE_LEADER_TERM,
            leader_id: super::super::tests::NODE_A.to_string(),
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

async fn drive_recovery(
    node: &mut crate::fabric_consistency::raft::live_cluster::Node,
    node_id: &str,
    run_directory: &std::path::Path,
    step: usize,
    mut is_campaign_started: bool,
) -> crate::error::Result<bool> {
    if !matches!(node.service.state().rejoin, ReplicaRejoin::Recovering(_)) {
        write_signal(&run_directory.join(format!("{node_id}-reconciled.preserves")), "reconciled")?;
    }
    if node_id == super::super::tests::NODE_B
        && !is_campaign_started
        && [
            super::super::tests::NODE_A,
            super::super::tests::NODE_B,
            super::super::tests::NODE_C,
        ]
        .iter()
        .all(|peer| run_directory.join(format!("{peer}-reconciled.preserves")).is_file())
    {
        let timer_ref = node.service.state().active_election_timer_ref.clone();
        child::require_applied(node.service.handle_event(ReplicaEvent::ElectionTimeout { timer_ref }).await)?;
        is_campaign_started = true;
    }
    if step > 0 && step.is_multiple_of(10) && matches!(node.service.state().rejoin, ReplicaRejoin::Recovering(_)) {
        let timer_ref = node.service.state().active_election_timer_ref.clone();
        child::require_applied(node.service.handle_event(ReplicaEvent::ElectionTimeout { timer_ref }).await)?;
    }
    Ok(is_campaign_started)
}

//! The recovered generation: every replica restarts from its durable store,
//! reconciles through the peer-query protocol, node B is re-elected, and each
//! process publishes its terminal receipt on the stop signal.

/// Idle event polls after which a recovering replica retries its peer-query
/// round, and after which the recovering campaigner re-campaigns.
const RECOVERY_RETRY_POLLS: u64 = 10;
const CAMPAIGN_RETRY_POLLS: u64 = 50;

/// A recovered replica: reconcile through the peer-query protocol, node B
/// campaigns once every replica is reconciled, and the recovered process of
/// the crashed leader presents stale frames to the re-elected leader.
pub(super) async fn recovered(
    mut node: super::Node,
    run_directory: &std::path::Path,
    deadline: &mut super::Deadline,
) -> crate::error::Result<()> {
    super::super::super::peer::write_value(
        &super::super::receipt::node_file(run_directory, &node.node_id, "recovered"),
        &super::node_receipt(&node)?.value(),
    )?;
    super::publish(run_directory, &node.node_id, "ready")?;
    super::await_value(&run_directory.join(super::super::receipt::RESTART_FILE), deadline).await?;
    if super::is_recovering(&node) {
        super::election_timeout(&mut node).await?;
    }
    let mut stale_term = None;
    let mut idle_polls = 0_u64;
    let mut campaign_polls = None;
    for _turn in 0..super::PHASE_TURNS {
        deadline.require_before()?;
        if !super::is_recovering(&node) {
            super::publish(run_directory, &node.node_id, "reconciled")?;
        }
        if run_directory.join(super::super::receipt::STOP_FILE).is_file() {
            return finish(node, run_directory).await;
        }
        if node.node_id == super::super::fixture::NODE_B {
            campaign_polls = drive_campaign(&mut node, run_directory, campaign_polls).await?;
        }
        if node.node_id == super::super::fixture::NODE_A
            && stale_term.is_none()
            && run_directory.join(super::super::receipt::RECOVERED_LEADER_FILE).is_file()
        {
            let term = super::super::receipt::parse_term(
                &super::await_value(&run_directory.join(super::super::receipt::STALE_LEADER_FILE), deadline).await?,
            )?;
            super::stale::send_probes(&mut node, term).await?;
            stale_term = Some(term);
        }
        let Some(event) = super::poll_event(&mut node).await? else {
            idle_polls += 1;
            if idle_polls.is_multiple_of(RECOVERY_RETRY_POLLS) && super::is_recovering(&node) {
                super::election_timeout(&mut node).await?;
            }
            continue;
        };
        if let Some(term) = stale_term {
            super::stale::record_rejection(run_directory, &event, term)?;
        }
        if node.node_id == super::super::fixture::NODE_B
            && let Some(probe) = super::stale::classify(&event)
        {
            super::stale::handle_probe(&mut node, run_directory, probe, event).await?;
            continue;
        }
        super::require_applied(node.service.handle_event(event).await)?;
        if node.node_id == super::super::fixture::NODE_B
            && node.service.state().role == crate::fabric_consistency::raft::ReplicaRole::Leader
        {
            super::publish_file(&run_directory.join(super::super::receipt::RECOVERED_LEADER_FILE), "recovered-leader")?;
        }
    }
    Err(super::exceeded_bound())
}

/// Node B campaigns once all three replicas reconciled, and campaigns again
/// (in a new term) if a bounded number of polls pass without leadership.
async fn drive_campaign(
    node: &mut super::Node,
    run_directory: &std::path::Path,
    campaign_polls: Option<u64>,
) -> crate::error::Result<Option<u64>> {
    let state = node.service.state();
    if state.role == crate::fabric_consistency::raft::ReplicaRole::Leader || super::is_recovering(node) {
        return Ok(campaign_polls);
    }
    let is_all_reconciled = super::super::fixture::NODES
        .iter()
        .all(|peer| super::super::receipt::node_file(run_directory, peer, "reconciled").is_file());
    match campaign_polls {
        None if is_all_reconciled => {
            super::election_timeout(node).await?;
            Ok(Some(0))
        }
        Some(polls) if polls >= CAMPAIGN_RETRY_POLLS => {
            super::election_timeout(node).await?;
            Ok(Some(0))
        }
        Some(polls) => Ok(Some(polls + 1)),
        None => Ok(None),
    }
}

async fn finish(mut node: super::Node, run_directory: &std::path::Path) -> crate::error::Result<()> {
    if !super::super::receipt::rejoin_is_voter(&node.service.state().rejoin) {
        return Err(crate::capability_qualification::invalid(
            "recovered live Raft replica never regained voting admission",
        ));
    }
    let terminal = super::node_receipt(&node)?;
    let ingress = node
        .ingress
        .take()
        .ok_or_else(|| crate::capability_qualification::invalid("live Raft ingress pump is absent"))?;
    let listener = ingress.shutdown().await?;
    drop(node);
    listener.drain_and_close(crate::fabric_transport::ListenerDrainReason::OperatorRequest).await?;
    super::super::super::peer::write_value(
        &super::super::receipt::node_file(run_directory, &terminal.node_id, "terminal"),
        &terminal.value(),
    )
}

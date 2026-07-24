use std::collections::BTreeMap;

use super::*;
use crate::fabric_consistency::raft::live_cluster;
use crate::fabric_transport::cross_process::tests::listener_with_secret;
use crate::fabric_transport::parse_canonical_cross_process_endpoint;

const NODE_A_SECRET_BYTE: u8 = 41;
const NODE_B_SECRET_BYTE: u8 = 43;
const NODE_C_SECRET_BYTE: u8 = 47;
const EVENT_POLL_MILLISECONDS: u64 = 100;
const EVENT_LOOP_LIMIT: usize = 1_200;
const INGRESS_EVENT_CAPACITY: usize = 32;
const INGRESS_DELIVERY_LIMIT: u64 = 1_024;

pub(super) async fn run(node_id: String, run_directory: PathBuf) -> Result<()> {
    let listener = listener_with_secret(secret_byte(&node_id)?).await;
    write_value(&endpoint_path(&run_directory, &node_id), &listener.handoff().value)?;
    wait_for_nodes(&run_directory, "endpoint")?;
    let endpoints = read_endpoints(&run_directory)?;
    let endpoint_identity = endpoints
        .get(&node_id)
        .ok_or_else(|| MoltenError::invalid_harness("child endpoint identity is absent"))?
        .descriptor
        .public_endpoint_identity
        .clone();
    let group = super::super::tests::active_group();
    let mut node = live_cluster::build_node(&group, &node_id, listener, &endpoints).await?;
    let listener = node.listener.take().ok_or_else(|| MoltenError::invalid_harness("child live listener is absent"))?;
    let mut ingress = IrohReplicaIngressPump::spawn(listener, IrohReplicaIngressConfig {
        session_ref: node.session_ref.clone(),
        accept_timeout: Duration::from_secs(CHILD_TIMEOUT_SECONDS),
        event_capacity: INGRESS_EVENT_CAPACITY,
        delivery_limit: INGRESS_DELIVERY_LIMIT,
    })?;
    write_signal(&run_directory.join(format!("{node_id}-ready.preserves")), "ready")?;
    if node_id == NODE_A {
        wait_for_file(&run_directory.join(START_FILE))?;
        run_leader(&mut node, &mut ingress, &run_directory).await?;
    } else {
        run_follower(&mut node, &mut ingress, &run_directory, node_id == NODE_C).await?;
    }
    node.listener = Some(ingress.shutdown().await?);
    let mut receipt = receipt_from_node(&node_id, &endpoint_identity, &node)?;
    live_cluster::close_node(node).await;
    receipt.clean_shutdown = true;
    write_value(&receipt_path(&run_directory, &node_id), &receipt_value(&receipt))
}

fn secret_byte(node_id: &str) -> Result<u8> {
    match node_id {
        NODE_A => Ok(NODE_A_SECRET_BYTE),
        NODE_B => Ok(NODE_B_SECRET_BYTE),
        NODE_C => Ok(NODE_C_SECRET_BYTE),
        _ => Err(MoltenError::invalid_harness("child secret requested outside static membership")),
    }
}

fn read_endpoints(run_directory: &Path) -> Result<BTreeMap<String, CanonicalCrossProcessEndpoint>> {
    [NODE_A, NODE_B, NODE_C]
        .into_iter()
        .map(|node_id| {
            let value = read_value(&endpoint_path(run_directory, node_id))?;
            Ok((node_id.to_string(), parse_canonical_cross_process_endpoint(&value)?))
        })
        .collect()
}

async fn run_leader(
    node: &mut live_cluster::LiveNode,
    ingress: &mut IrohReplicaIngressPump,
    run_directory: &Path,
) -> Result<()> {
    let timer_ref = node.service.state().active_election_timer_ref.clone();
    require_applied(node.service.handle_event(ReplicaEvent::ElectionTimeout { timer_ref }).await)?;
    let mut proposal_started = false;
    let mut read_started = false;
    let mut snapshot_started = false;
    let mut quorum_loss_started = false;
    for _step in 0..EVENT_LOOP_LIMIT {
        if run_directory.join(STOP_FILE).is_file() {
            return Ok(());
        }
        let Some(event) = poll_event(ingress).await? else {
            continue;
        };
        let outcome = node.service.handle_event(event.event).await;
        let read_completed = has_read_outcome(&outcome);
        require_applied(outcome)?;
        if node.service.state().role == ReplicaRole::Leader && !proposal_started {
            propose(node).await?;
            proposal_started = true;
        }
        if node.service.state().commit_index == INITIAL_LOG_INDEX && !read_started {
            begin_read(node).await?;
            read_started = true;
        }
        if read_completed && !snapshot_started {
            begin_snapshot_catch_up(node).await?;
            snapshot_started = true;
        }
        if snapshot_started
            && !quorum_loss_started
            && node.service.state().match_index.get(NODE_C) == Some(&INITIAL_LOG_INDEX)
        {
            begin_quorum_loss(node, run_directory).await?;
            quorum_loss_started = true;
            write_signal(&run_directory.join(LEADER_DONE_FILE), "leader-done")?;
        }
    }
    Err(MoltenError::invalid_harness("leader exhausted its bounded event loop"))
}

async fn propose(node: &mut live_cluster::LiveNode) -> Result<()> {
    require_applied(
        node.service
            .handle_event(ReplicaEvent::Propose {
                request_ref: super::super::tests::test_ref(REQUEST_LABEL),
                command_ref: super::super::tests::test_ref("distinct-process-command"),
                command_schema_ref: super::super::tests::test_ref("live-cluster-command-schema"),
            })
            .await,
    )
}

async fn begin_read(node: &mut live_cluster::LiveNode) -> Result<()> {
    require_applied(
        node.service
            .handle_event(ReplicaEvent::Read {
                request_ref: super::super::tests::test_ref("distinct-process-read"),
                mode: crate::fabric_consistency::ConsistencyReadMode::Linearizable,
            })
            .await,
    )
}

async fn begin_quorum_loss(node: &mut live_cluster::LiveNode, run_directory: &Path) -> Result<()> {
    write_signal(&run_directory.join(PARTITION_FILE), "partition")?;
    require_applied(
        node.service
            .handle_event(ReplicaEvent::Propose {
                request_ref: super::super::tests::test_ref(QUORUM_LOSS_REQUEST_LABEL),
                command_ref: super::super::tests::test_ref("distinct-process-quorum-loss-command"),
                command_schema_ref: super::super::tests::test_ref("live-cluster-command-schema"),
            })
            .await,
    )?;
    require_applied(
        node.service
            .handle_event(ReplicaEvent::Read {
                request_ref: super::super::tests::test_ref("distinct-process-quorum-loss-read"),
                mode: crate::fabric_consistency::ConsistencyReadMode::Linearizable,
            })
            .await,
    )
}

async fn begin_snapshot_catch_up(node: &mut live_cluster::LiveNode) -> Result<()> {
    require_applied(
        node.service
            .handle_event(ReplicaEvent::CreateSnapshot {
                application_state_ref: super::super::tests::test_ref(APPLICATION_STATE_LABEL),
            })
            .await,
    )?;
    require_applied(node.service.handle_event(ReplicaEvent::HeartbeatTimeout).await)
}

async fn run_follower(
    node: &mut live_cluster::LiveNode,
    ingress: &mut IrohReplicaIngressPump,
    run_directory: &Path,
    lag_until_snapshot: bool,
) -> Result<()> {
    for _step in 0..EVENT_LOOP_LIMIT {
        if run_directory.join(STOP_FILE).is_file() {
            return Ok(());
        }
        let Some(event) = poll_event(ingress).await? else {
            continue;
        };
        if run_directory.join(PARTITION_FILE).is_file() {
            continue;
        }
        if lag_until_snapshot && should_drop_before_snapshot(node.service.state(), &event.event) {
            continue;
        }
        require_applied(node.service.handle_event(event.event).await)?;
    }
    Err(MoltenError::invalid_harness("follower exhausted its bounded event loop"))
}

fn should_drop_before_snapshot(state: &ReplicaState, event: &ReplicaEvent) -> bool {
    if state.snapshot.is_some() {
        return false;
    }
    matches!(event, ReplicaEvent::Message {
        envelope: ReplicaMessageEnvelope {
            message: RaftMessage::AppendEntries { entries, leader_commit, .. },
            ..
        }
    } if !entries.is_empty() || *leader_commit > INITIAL_COMMIT_INDEX)
        || matches!(event, ReplicaEvent::Message {
            envelope: ReplicaMessageEnvelope {
                message: RaftMessage::ReadProbe { .. },
                ..
            }
        })
}

async fn poll_event(ingress: &mut IrohReplicaIngressPump) -> Result<Option<ReceivedReplicaEvent>> {
    tokio::select! {
        result = ingress.next() => result.map(Some),
        () = tokio::time::sleep(Duration::from_millis(EVENT_POLL_MILLISECONDS)) => Ok(None),
    }
}

fn has_read_outcome(outcome: &ReplicaExecutionOutcome) -> bool {
    matches!(outcome, ReplicaExecutionOutcome::Applied(applied)
        if applied.observations.iter().any(|observation| observation.kind == ReplicaEffectKind::ReadOutcome))
}

fn require_applied(outcome: ReplicaExecutionOutcome) -> Result<()> {
    match outcome {
        ReplicaExecutionOutcome::Applied(_) => Ok(()),
        ReplicaExecutionOutcome::Denied { diagnostic, .. } => {
            Err(MoltenError::invalid_harness(format!("distinct-process turn denied: {diagnostic}")))
        }
        ReplicaExecutionOutcome::Failed(failed) => Err(MoltenError::invalid_harness(format!(
            "distinct-process effect {} failed: {}",
            failed.failed_kind.as_str(),
            failed.diagnostic
        ))),
    }
}

fn receipt_from_node(node_id: &str, endpoint_identity: &str, node: &live_cluster::LiveNode) -> Result<ProcessReceipt> {
    let state = node.service.state();
    let application = node.service.ports().application.handler();
    let durability = node.service.ports().durability.adapter().state();
    Ok(ProcessReceipt {
        node_id: node_id.to_string(),
        process_id: u64::from(std::process::id()),
        endpoint_identity: endpoint_identity.to_string(),
        role: state.role,
        term: state.current_term,
        commit_index: state.commit_index,
        last_applied: state.last_applied,
        quorum_term: state.quorum_confirmed_term.unwrap_or(INITIAL_TERM),
        pending_read_count: u64::try_from(state.pending_reads.len())
            .map_err(|_| MoltenError::invalid_harness("pending read count overflow"))?,
        snapshot_ref: state.snapshot.as_ref().map_or_else(String::new, |snapshot| snapshot.snapshot_ref.clone()),
        request_completed: state.completed_requests.contains_key(&super::super::tests::test_ref(REQUEST_LABEL)),
        quorum_loss_request_uncommitted: !state
            .completed_requests
            .contains_key(&super::super::tests::test_ref(QUORUM_LOSS_REQUEST_LABEL)),
        application_applied: application.applied_request_refs.contains(&super::super::tests::test_ref(REQUEST_LABEL)),
        application_restored: application.restored_application_state_ref
            == Some(super::super::tests::test_ref(APPLICATION_STATE_LABEL)),
        durable_record_count: u64::try_from(durability.durable_log.len())
            .map_err(|_| MoltenError::invalid_harness("durable record count overflow"))?,
        durable_snapshot_count: u64::try_from(durability.snapshots.len())
            .map_err(|_| MoltenError::invalid_harness("durable snapshot count overflow"))?,
        clean_shutdown: false,
    })
}

fn receipt_value(receipt: &ProcessReceipt) -> IOValue {
    crate::preserves_rail::record(RECEIPT_SCHEMA, vec![
        crate::preserves_rail::string(&receipt.node_id),
        crate::preserves_rail::u64_value(receipt.process_id),
        crate::preserves_rail::string(&receipt.endpoint_identity),
        crate::preserves_rail::string(match receipt.role {
            ReplicaRole::Leader => "leader",
            _ => "follower",
        }),
        crate::preserves_rail::u64_value(receipt.term),
        crate::preserves_rail::u64_value(receipt.commit_index),
        crate::preserves_rail::u64_value(receipt.last_applied),
        crate::preserves_rail::u64_value(receipt.quorum_term),
        crate::preserves_rail::u64_value(receipt.pending_read_count),
        crate::preserves_rail::string(&receipt.snapshot_ref),
        crate::preserves_rail::bool_value(receipt.request_completed),
        crate::preserves_rail::bool_value(receipt.quorum_loss_request_uncommitted),
        crate::preserves_rail::bool_value(receipt.application_applied),
        crate::preserves_rail::bool_value(receipt.application_restored),
        crate::preserves_rail::u64_value(receipt.durable_record_count),
        crate::preserves_rail::u64_value(receipt.durable_snapshot_count),
        crate::preserves_rail::bool_value(receipt.clean_shutdown),
    ])
}

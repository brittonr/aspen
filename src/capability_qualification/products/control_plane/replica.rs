//! One live Raft replica peer process. It assembles the production scoped live
//! replica (capability-rooted Redb durable log, real Iroh transport and ingress
//! pump, Tokio time port, admitted application port, control channel) under the
//! fixed group, and drives only explicit, phase-signalled events so leader
//! identity, terms and the commit quorum do not depend on timer races.

use super::fixture;
use super::receipt;
use super::receipt::Probe;
use crate::fabric_consistency::raft::AdmittedReplicaPort;
use crate::fabric_consistency::raft::ChannelReplicaPort;
use crate::fabric_consistency::raft::Command;
use crate::fabric_consistency::raft::CommittedBatchHandler;
use crate::fabric_consistency::raft::ConcreteReplicaPortBundle;
use crate::fabric_consistency::raft::INITIAL_COMMIT_INDEX;
use crate::fabric_consistency::raft::INITIAL_LOG_INDEX;
use crate::fabric_consistency::raft::Message;
use crate::fabric_consistency::raft::RedbReplicaPort;
use crate::fabric_consistency::raft::ReplicaAdapterSet;
use crate::fabric_consistency::raft::ReplicaConfig;
use crate::fabric_consistency::raft::ReplicaEvent;
use crate::fabric_consistency::raft::ReplicaExecutionOutcome;
use crate::fabric_consistency::raft::ReplicaIngressConfig;
use crate::fabric_consistency::raft::ReplicaIngressPump;
use crate::fabric_consistency::raft::ReplicaKind;
use crate::fabric_consistency::raft::ReplicaMessageEnvelope;
use crate::fabric_consistency::raft::ReplicaObservation;
use crate::fabric_consistency::raft::ReplicaRejoin;
use crate::fabric_consistency::raft::ReplicaRole;
use crate::fabric_consistency::raft::ReplicaTransportEffects as _;
use crate::fabric_consistency::raft::ReplicaTransportPort;
use crate::fabric_consistency::raft::ReplicatedEntry;
use crate::fabric_consistency::raft::ScopedLiveReplica;
use crate::fabric_consistency::raft::ServiceReplicaConfig;
use crate::fabric_consistency::raft::SnapshotRestore;
use crate::fabric_consistency::raft::TokioReplicaConfig;
use crate::fabric_consistency::raft::TokioReplicaPort;

const EVENT_POLL: std::time::Duration = std::time::Duration::from_millis(100);
const SIGNAL_POLL: std::time::Duration = std::time::Duration::from_millis(10);
/// Bound of every peer phase; the producer kills a peer long before it.
const PHASE_BOUND: std::time::Duration = std::time::Duration::from_secs(120);
const TRANSPORT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);
const INGRESS_ACCEPT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);
const INGRESS_EVENT_CAPACITY: usize = 32;
const INGRESS_DELIVERY_LIMIT: u64 = 1_024;
const INGRESS_DELIVERY_WINDOW: std::time::Duration = std::time::Duration::from_secs(1);
const CONTROL_OBSERVATION_CAPACITY: usize = 32;
/// Idle event polls after which a recovering replica retries its peer-query
/// round, and after which the recovering campaigner re-campaigns.
const RECOVERY_RETRY_POLLS: u64 = 10;
const CAMPAIGN_RETRY_POLLS: u64 = 50;

type Ports = ConcreteReplicaPortBundle<crate::fabric_time::OperatingSystemEntropySource, AppliedCommands>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Mode {
    Fresh,
    Recover,
}

/// The admitted application: the committed request sequence it applied, in order.
#[derive(Debug, Default)]
pub(super) struct AppliedCommands {
    request_refs: Vec<String>,
}

impl AppliedCommands {
    fn state_ref(&self) -> crate::error::Result<String> {
        crate::preserves_rail::canonical_hash(&application_state_value(&self.request_refs))
    }
}

impl CommittedBatchHandler for AppliedCommands {
    fn restore_snapshot(&mut self, _snapshot: &SnapshotRestore) -> crate::error::Result<String> {
        // The qualification run never snapshots; a restore is outside its admitted history.
        Err(super::super::super::invalid("qualification live Raft application admits no snapshot restore"))
    }

    fn apply_batch(&mut self, commands: &[Command]) -> crate::error::Result<String> {
        self.request_refs.extend(commands.iter().map(|command| command.request_ref.clone()));
        self.state_ref()
    }
}

pub(super) fn application_state_value(request_refs: &[String]) -> preserves::IOValue {
    crate::preserves_rail::record("qualification-live-raft-application-state-v1", vec![
        crate::preserves_rail::sequence(request_refs.iter().map(crate::preserves_rail::string).collect()),
    ])
}

pub(super) fn committed_log_value(entries: &[ReplicatedEntry]) -> preserves::IOValue {
    crate::preserves_rail::record("qualification-live-raft-committed-log-v1", vec![crate::preserves_rail::sequence(
        entries
            .iter()
            .map(|entry| {
                crate::preserves_rail::record("entry", vec![
                    crate::preserves_rail::u64_value(entry.index),
                    crate::preserves_rail::u64_value(entry.term),
                    crate::preserves_rail::string(&entry.request_ref),
                    crate::preserves_rail::string(&entry.command_ref),
                    crate::preserves_rail::string(&entry.command_schema_ref),
                ])
            })
            .collect(),
    )])
}

pub(super) fn request_ref() -> String {
    fixture::reference("committed-request")
}

pub(super) fn command_ref() -> String {
    fixture::reference("committed-command")
}

struct Node {
    node_id: String,
    endpoint_identity: String,
    service: ScopedLiveReplica<Ports>,
    ingress: Option<ReplicaIngressPump>,
    recovery: Option<receipt::Recovery>,
    _control: tokio::sync::mpsc::Receiver<ReplicaObservation>,
}

/// Peer entry: one replica process in one mode.
pub(super) fn run(node_id: &str, mode: Mode, run_directory: &std::path::Path) -> crate::error::Result<()> {
    let runtime = tokio::runtime::Runtime::new().map_err(crate::error::Failure::from)?;
    runtime.block_on(async {
        let deadline = tokio::time::Instant::now() + PHASE_BOUND;
        let mut node = open(node_id, mode, run_directory, deadline).await?;
        match (mode, node_id) {
            (Mode::Fresh, fixture::NODE_A) => fresh_leader(&mut node, run_directory, deadline).await,
            (Mode::Fresh, _) => fresh_follower(&mut node, run_directory, deadline).await,
            (Mode::Recover, _) => recovered(node, run_directory, deadline).await,
        }
    })
}

async fn open(
    node_id: &str,
    mode: Mode,
    run_directory: &std::path::Path,
    deadline: tokio::time::Instant,
) -> crate::error::Result<Node> {
    let group = fixture::group()?;
    let listener = crate::fabric_transport::Listener::bind(fixture::listener_input(&group, node_id)?).await?;
    let endpoint_identity = listener.handoff().descriptor.public_endpoint_identity.clone();
    super::super::peer::write_value(
        &receipt::node_file(run_directory, node_id, "endpoint"),
        &listener.handoff().value,
    )?;
    let mut peers = std::collections::BTreeMap::new();
    for peer_id in fixture::NODES.into_iter().filter(|peer_id| *peer_id != node_id) {
        let value = await_value(&receipt::node_file(run_directory, peer_id, "endpoint"), deadline).await?;
        let endpoint = crate::fabric_transport::parse_endpoint(&value)?;
        peers.insert(peer_id.to_string(), fixture::client_input(&group, node_id, peer_id, endpoint)?);
    }
    let time = fixture::time_profile()?;
    let mut plan = fixture::start_plan(&group, node_id, &time.profile.profile_ref)?;
    let durability_root = run_directory.join(format!("{node_id}-durability"));
    std::fs::create_dir_all(&durability_root).map_err(crate::error::Failure::from)?;
    let adapter = crate::fabric_durability::RedbDurableStateAdapter::open(
        &durability_root,
        fixture::durable_profile()?,
        fixture::durable_namespace(node_id),
    )?;
    let durability = RedbReplicaPort::new(
        adapter,
        plan.state.profile.durable_log_ref.clone(),
        plan.state.profile.snapshot_store_ref.clone(),
    )?
    .reconcile_repaired_storage()?;
    let has_durable_state =
        !durability.adapter().state().durable_log.is_empty() || !durability.adapter().state().snapshots.is_empty();
    let recovery = match mode {
        Mode::Fresh if has_durable_state => {
            return Err(super::super::super::invalid("fresh live Raft replica found existing durable state"));
        }
        Mode::Fresh => None,
        Mode::Recover if !has_durable_state => {
            return Err(super::super::super::invalid("recovered live Raft replica found no durable state"));
        }
        Mode::Recover => {
            let recovered = durability.plan_recovery(plan)?;
            plan = recovered.start_plan;
            Some(receipt::Recovery {
                recovery_ref: recovered.recovery_ref,
                durable_commit_index: recovered.durable_commit_index,
                replay_entry_count: u64::try_from(recovered.replay_entry_count)
                    .map_err(|_| super::super::super::invalid("live Raft replay count exceeds u64"))?,
            })
        }
    };
    let capacity = usize::try_from(time.profile.max_scheduler_queue_depth)
        .ok()
        .filter(|capacity| *capacity > 0)
        .ok_or_else(|| super::super::super::invalid("live Raft inbox needs a positive admitted queue depth"))?;
    let (timer_sender, inbox) = tokio::sync::mpsc::channel(capacity);
    let time_port = TokioReplicaPort::new_operating_system(
        TokioReplicaConfig {
            generation: group.service_generation,
            service_id: group.service_id.clone(),
            capability_ref: fixture::reference(&format!("{node_id}/time-capability")),
            entropy_binding_ref: plan.state.profile.entropy_profile_ref.clone(),
            tick_duration: fixture::TICK_DURATION,
            heartbeat_ticks: plan.state.profile.heartbeat_ticks,
            election_min_ticks: plan.state.profile.election_min_ticks,
            election_max_ticks: plan.state.profile.election_max_ticks,
            profile: time.profile,
        },
        timer_sender,
    )?;
    let application = AdmittedReplicaPort::new(
        ServiceReplicaConfig {
            group_binding_ref: group.binding_ref.clone(),
            application_manifest_ref: group.application_manifest_ref.clone(),
            handler_ref: fixture::reference("application-handler"),
            command_schema_refs: std::collections::BTreeSet::from([fixture::command_schema_ref()]),
            initial_applied_index: INITIAL_COMMIT_INDEX,
        },
        AppliedCommands::default(),
    )?;
    let (control_sender, control) = tokio::sync::mpsc::channel(CONTROL_OBSERVATION_CAPACITY);
    let control_port = ChannelReplicaPort::new(
        ReplicaConfig {
            service_id: group.service_id.clone(),
            service_generation: group.service_generation,
            supervision_ref: plan.state.profile.supervision_ref.clone(),
        },
        control_sender,
    )?;
    let transport = ReplicaTransportPort::new(plan.state.profile.protocol_ref.clone(), peers, TRANSPORT_TIMEOUT)?;
    let ports = crate::fabric_consistency::raft::assemble_scoped_concrete_replica(
        fixture::runtime_identity(&plan),
        ReplicaAdapterSet {
            durability,
            transport,
            time: time_port,
            application,
            control: control_port,
        },
    )?;
    let service = ScopedLiveReplica::start(plan, ports, inbox).await?;
    let ingress = ReplicaIngressPump::spawn(listener, ReplicaIngressConfig {
        session_ref: fixture::session_ref(),
        accept_timeout: INGRESS_ACCEPT_TIMEOUT,
        event_capacity: INGRESS_EVENT_CAPACITY,
        delivery_limit: INGRESS_DELIVERY_LIMIT,
        delivery_window: INGRESS_DELIVERY_WINDOW,
    })?;
    Ok(Node {
        node_id: node_id.to_string(),
        endpoint_identity,
        service,
        ingress: Some(ingress),
        recovery,
        _control: control,
    })
}

/// Node A: campaign on the start signal, commit one request with quorum while
/// node C withholds acknowledgement, then catch node C up.
async fn fresh_leader(
    node: &mut Node,
    run_directory: &std::path::Path,
    deadline: tokio::time::Instant,
) -> crate::error::Result<()> {
    publish(run_directory, &node.node_id, "ready")?;
    await_value(&run_directory.join(receipt::START_FILE), deadline).await?;
    let timer_ref = node.service.state().active_election_timer_ref.clone();
    require_applied(node.service.handle_event(ReplicaEvent::ElectionTimeout { timer_ref }).await)?;
    let mut is_proposed = false;
    let mut is_catch_up_started = false;
    let mut is_done = false;
    loop {
        require_before(deadline)?;
        checkpoint_if_requested(node, run_directory)?;
        let Some(event) = poll_event(node).await? else {
            continue;
        };
        require_applied(node.service.handle_event(event).await)?;
        let state = node.service.state();
        if state.role == ReplicaRole::Leader && !is_proposed {
            require_applied(
                node.service
                    .handle_event(ReplicaEvent::Propose {
                        request_ref: request_ref(),
                        command_ref: command_ref(),
                        command_schema_ref: fixture::command_schema_ref(),
                    })
                    .await,
            )?;
            is_proposed = true;
        }
        if node.service.state().commit_index >= INITIAL_LOG_INDEX && !is_catch_up_started {
            super::super::peer::write_value(&run_directory.join(receipt::CATCH_UP_FILE), &receipt::signal("catch-up"))?;
            require_applied(node.service.handle_event(ReplicaEvent::HeartbeatTimeout).await)?;
            is_catch_up_started = true;
        }
        let match_index = &node.service.state().match_index;
        if is_catch_up_started
            && !is_done
            && [fixture::NODE_B, fixture::NODE_C]
                .iter()
                .all(|peer| match_index.get(*peer).is_some_and(|index| *index >= INITIAL_LOG_INDEX))
        {
            super::super::peer::write_value(
                &run_directory.join(receipt::LEADER_DONE_FILE),
                &receipt::signal("leader-done"),
            )?;
            is_done = true;
        }
    }
}

/// Nodes B and C follow; node C withholds every entry-carrying or
/// commit-advancing frame until the leader has committed, so the commit quorum
/// is exactly the leader and node B.
async fn fresh_follower(
    node: &mut Node,
    run_directory: &std::path::Path,
    deadline: tokio::time::Instant,
) -> crate::error::Result<()> {
    publish(run_directory, &node.node_id, "ready")?;
    let is_lagging = node.node_id == fixture::NODE_C;
    loop {
        require_before(deadline)?;
        checkpoint_if_requested(node, run_directory)?;
        let Some(event) = poll_event(node).await? else {
            continue;
        };
        if is_lagging && !run_directory.join(receipt::CATCH_UP_FILE).is_file() && advances_commit(&event) {
            continue;
        }
        require_applied(node.service.handle_event(event).await)?;
    }
}

fn advances_commit(event: &ReplicaEvent) -> bool {
    matches!(event, ReplicaEvent::Message {
        envelope: ReplicaMessageEnvelope {
            message: Message::AppendEntries { entries, leader_commit, .. },
            ..
        }
    } if !entries.is_empty() || *leader_commit > INITIAL_COMMIT_INDEX)
}

fn checkpoint_if_requested(node: &Node, run_directory: &std::path::Path) -> crate::error::Result<()> {
    let output = receipt::node_file(run_directory, &node.node_id, "checkpoint");
    let state = node.service.state();
    if !run_directory.join(receipt::CHECKPOINT_FILE).is_file()
        || output.is_file()
        || state.commit_index < INITIAL_LOG_INDEX
        || state.last_applied < INITIAL_LOG_INDEX
    {
        return Ok(());
    }
    super::super::peer::write_value(&output, &replica_receipt(node)?.value())
}

/// A recovered replica: reconcile through the peer-query protocol, node B
/// campaigns once every replica is reconciled, and the recovered process of
/// the crashed leader presents stale frames to the re-elected leader.
async fn recovered(
    mut node: Node,
    run_directory: &std::path::Path,
    deadline: tokio::time::Instant,
) -> crate::error::Result<()> {
    super::super::peer::write_value(
        &receipt::node_file(run_directory, &node.node_id, "recovered"),
        &replica_receipt(&node)?.value(),
    )?;
    publish(run_directory, &node.node_id, "ready")?;
    await_value(&run_directory.join(receipt::RESTART_FILE), deadline).await?;
    if is_recovering(&node) {
        election_timeout(&mut node).await?;
    }
    let mut stale_term = None;
    let mut idle_polls = 0_u64;
    let mut campaign_polls = None;
    loop {
        require_before(deadline)?;
        if !is_recovering(&node) {
            publish(run_directory, &node.node_id, "reconciled")?;
        }
        if run_directory.join(receipt::STOP_FILE).is_file() {
            return finish(node, run_directory).await;
        }
        if node.node_id == fixture::NODE_B {
            campaign_polls = drive_campaign(&mut node, run_directory, campaign_polls).await?;
        }
        if node.node_id == fixture::NODE_A
            && stale_term.is_none()
            && run_directory.join(receipt::RECOVERED_LEADER_FILE).is_file()
        {
            let term =
                receipt::parse_term(&await_value(&run_directory.join(receipt::STALE_LEADER_FILE), deadline).await?)?;
            send_stale_probes(&mut node, term).await?;
            stale_term = Some(term);
        }
        let Some(event) = poll_event(&mut node).await? else {
            idle_polls += 1;
            if idle_polls.is_multiple_of(RECOVERY_RETRY_POLLS) && is_recovering(&node) {
                election_timeout(&mut node).await?;
            }
            continue;
        };
        if let Some(term) = stale_term {
            record_rejection(run_directory, &event, term)?;
        }
        if node.node_id == fixture::NODE_B
            && let Some(probe) = stale_probe(&event)
        {
            handle_probe(&mut node, run_directory, probe, event).await?;
            continue;
        }
        require_applied(node.service.handle_event(event).await)?;
        if node.node_id == fixture::NODE_B && node.service.state().role == ReplicaRole::Leader {
            publish_file(&run_directory.join(receipt::RECOVERED_LEADER_FILE), "recovered-leader")?;
        }
    }
}

/// Node B campaigns once all three replicas reconciled, and campaigns again
/// (in a new term) if a bounded number of polls pass without leadership.
async fn drive_campaign(
    node: &mut Node,
    run_directory: &std::path::Path,
    campaign_polls: Option<u64>,
) -> crate::error::Result<Option<u64>> {
    let state = node.service.state();
    if state.role == ReplicaRole::Leader || is_recovering(node) {
        return Ok(campaign_polls);
    }
    let all_reconciled =
        fixture::NODES.iter().all(|peer| receipt::node_file(run_directory, peer, "reconciled").is_file());
    match campaign_polls {
        None if all_reconciled => {
            election_timeout(node).await?;
            Ok(Some(0))
        }
        Some(polls) if polls >= CAMPAIGN_RETRY_POLLS => {
            election_timeout(node).await?;
            Ok(Some(0))
        }
        Some(polls) => Ok(Some(polls + 1)),
        None => Ok(None),
    }
}

async fn send_stale_probes(node: &mut Node, stale_term: u64) -> crate::error::Result<()> {
    let state = node.service.state();
    let append = Message::AppendEntries {
        term: stale_term,
        leader_id: fixture::NODE_A.to_string(),
        prev_log_index: INITIAL_LOG_INDEX,
        prev_log_term: stale_term,
        entries: Vec::new(),
        leader_commit: INITIAL_LOG_INDEX,
        config_epoch: state.membership.config_epoch,
        fencing_epoch: state.profile.fencing_epoch,
    };
    let vote = Message::RequestVote {
        term: stale_term,
        candidate_id: fixture::NODE_A.to_string(),
        last_log_index: INITIAL_LOG_INDEX,
        last_log_term: stale_term,
        config_epoch: state.membership.config_epoch,
        fencing_epoch: state.profile.fencing_epoch,
    };
    let stale_fencing = Message::AppendEntries {
        term: stale_term,
        leader_id: fixture::NODE_A.to_string(),
        prev_log_index: INITIAL_LOG_INDEX,
        prev_log_term: stale_term,
        entries: Vec::new(),
        leader_commit: INITIAL_LOG_INDEX,
        config_epoch: state.membership.config_epoch,
        fencing_epoch: state.profile.fencing_epoch - 1,
    };
    let group_binding_ref = state.profile.group_binding_ref.clone();
    let generation = state.profile.service_generation;
    let envelopes = [
        (generation, append.clone()),
        (generation, vote),
        (generation - 1, append),
        (generation, stale_fencing),
    ];
    for (service_generation, message) in envelopes {
        let envelope = ReplicaMessageEnvelope {
            group_binding_ref: group_binding_ref.clone(),
            service_generation,
            from: fixture::NODE_A.to_string(),
            to: fixture::NODE_B.to_string(),
            message,
        };
        node.service.ports_mut().transport.send(&envelope).await?;
    }
    Ok(())
}

/// Classify a frame from the crashed leader's process at the re-elected leader.
/// The recovered node A never campaigns nor leads, so every append or vote
/// request it sends to node B is one of the stale probes.
pub(super) fn stale_probe(event: &ReplicaEvent) -> Option<Probe> {
    let ReplicaEvent::Message { envelope } = event else {
        return None;
    };
    if envelope.from != fixture::NODE_A {
        return None;
    }
    let is_append = matches!(envelope.message, Message::AppendEntries { .. });
    let is_vote = matches!(envelope.message, Message::RequestVote { .. });
    if !is_append && !is_vote {
        return None;
    }
    Some(if envelope.service_generation != fixture::SERVICE_GENERATION {
        Probe::ServiceGeneration
    } else if envelope.message.fencing_epoch() != fixture::FENCING_EPOCH {
        Probe::FencingEpoch
    } else if is_vote {
        Probe::TermVote
    } else {
        Probe::TermAppend
    })
}

async fn handle_probe(
    node: &mut Node,
    run_directory: &std::path::Path,
    probe: Probe,
    event: ReplicaEvent,
) -> crate::error::Result<()> {
    let before = node.service.state().clone();
    let outcome = node.service.handle_event(event).await;
    let after = node.service.state();
    let class = match outcome {
        ReplicaExecutionOutcome::Applied(_) => "applied",
        ReplicaExecutionOutcome::Denied { .. } => "denied",
        ReplicaExecutionOutcome::Failed(_) => "failed",
    };
    let recorded = receipt::ProbeOutcome {
        probe: probe.as_str().to_string(),
        outcome: class.to_string(),
        leader_retained: before.role == ReplicaRole::Leader && after.role == ReplicaRole::Leader,
        term_retained: after.current_term == before.current_term,
        commit_retained: after.commit_index == before.commit_index,
        log_retained: after.log == before.log,
    };
    super::super::peer::write_value(&receipt::probe_file(run_directory, probe), &recorded.value())
}

/// At the stale leader's process: the re-elected leader's answer to a stale probe.
fn record_rejection(
    run_directory: &std::path::Path,
    event: &ReplicaEvent,
    stale_term: u64,
) -> crate::error::Result<()> {
    let ReplicaEvent::Message { envelope } = event else {
        return Ok(());
    };
    if envelope.from != fixture::NODE_B {
        return Ok(());
    }
    let (probe, is_rejected) = match &envelope.message {
        Message::AppendResponse { success, .. } => (Probe::TermAppend, !success),
        Message::VoteResponse { granted, .. } => (Probe::TermVote, !granted),
        _ => return Ok(()),
    };
    let output = receipt::rejection_file(run_directory, probe);
    if output.is_file() {
        return Ok(());
    }
    let rejection = receipt::Rejection {
        probe: probe.as_str().to_string(),
        is_rejected,
        is_responder_term_newer: envelope.message.term() > stale_term,
    };
    super::super::peer::write_value(&output, &rejection.value())
}

async fn finish(mut node: Node, run_directory: &std::path::Path) -> crate::error::Result<()> {
    if !is_voter(&node) {
        return Err(super::super::super::invalid("recovered live Raft replica never regained voting admission"));
    }
    let terminal = replica_receipt(&node)?;
    let ingress = node
        .ingress
        .take()
        .ok_or_else(|| super::super::super::invalid("live Raft ingress pump is absent"))?;
    let listener = ingress.shutdown().await?;
    drop(node);
    listener.drain_and_close(crate::fabric_transport::ListenerDrainReason::OperatorRequest).await?;
    super::super::peer::write_value(
        &receipt::node_file(run_directory, &terminal.node_id, "terminal"),
        &terminal.value(),
    )
}

fn replica_receipt(node: &Node) -> crate::error::Result<receipt::Replica> {
    let state = node.service.state();
    let committed = state.log.iter().filter(|entry| entry.index <= state.commit_index).cloned().collect::<Vec<_>>();
    let handler = node.service.ports().application.handler();
    let commit_quorum =
        node.service
            .evidence()
            .records()
            .iter()
            .find(|record| record.kind == ReplicaKind::Commit)
            .map(|record| receipt::CommitQuorum {
                term: record.term,
                index: record.index,
                source_ref: record.source_ref.clone(),
                evidence_ref: record.quorum_evidence_ref.clone().unwrap_or_default(),
                members: record.quorum_members.clone(),
            });
    Ok(receipt::Replica {
        node_id: node.node_id.clone(),
        process_id: u64::from(std::process::id()),
        endpoint_identity: node.endpoint_identity.clone(),
        role: state.role.as_str().to_string(),
        is_voter: receipt::rejoin_is_voter(&state.rejoin),
        term: state.current_term,
        commit_index: state.commit_index,
        last_applied: state.last_applied,
        committed_log_ref: crate::preserves_rail::canonical_hash(&committed_log_value(&committed))?,
        applied_request_refs: handler.request_refs.clone(),
        application_state_ref: handler.state_ref()?,
        group_binding_ref: state.profile.group_binding_ref.clone(),
        membership_ref: state.membership.membership_ref.clone(),
        config_epoch: state.membership.config_epoch,
        voters: state.membership.voters.clone(),
        durable_record_count: u64::try_from(node.service.ports().durability.adapter().state().durable_log.len())
            .map_err(|_| super::super::super::invalid("live Raft durable record count exceeds u64"))?,
        commit_quorum,
        recovery: node.recovery.clone(),
    })
}

fn is_recovering(node: &Node) -> bool {
    matches!(node.service.state().rejoin, ReplicaRejoin::Recovering(_))
}

fn is_voter(node: &Node) -> bool {
    receipt::rejoin_is_voter(&node.service.state().rejoin)
}

async fn election_timeout(node: &mut Node) -> crate::error::Result<()> {
    let timer_ref = node.service.state().active_election_timer_ref.clone();
    require_applied(node.service.handle_event(ReplicaEvent::ElectionTimeout { timer_ref }).await)
}

async fn poll_event(node: &mut Node) -> crate::error::Result<Option<ReplicaEvent>> {
    let ingress = node
        .ingress
        .as_mut()
        .ok_or_else(|| super::super::super::invalid("live Raft ingress pump is absent"))?;
    tokio::select! {
        received = ingress.next() => received.map(|received| Some(received.event)),
        () = tokio::time::sleep(EVENT_POLL) => Ok(None),
    }
}

fn require_applied(outcome: ReplicaExecutionOutcome) -> crate::error::Result<()> {
    match outcome {
        ReplicaExecutionOutcome::Applied(_) => Ok(()),
        ReplicaExecutionOutcome::Denied { diagnostic, .. } => {
            Err(super::super::super::invalid(format!("live Raft replica turn denied: {diagnostic}")))
        }
        ReplicaExecutionOutcome::Failed(failed) => Err(super::super::super::invalid(format!(
            "live Raft replica effect {} failed: {}",
            failed.failed_kind.as_str(),
            failed.diagnostic
        ))),
    }
}

fn require_before(deadline: tokio::time::Instant) -> crate::error::Result<()> {
    if tokio::time::Instant::now() >= deadline {
        return Err(super::super::super::invalid("live Raft replica exceeded its bounded phase"));
    }
    Ok(())
}

fn publish(run_directory: &std::path::Path, node_id: &str, phase: &'static str) -> crate::error::Result<()> {
    publish_file(&receipt::node_file(run_directory, node_id, phase), phase)
}

fn publish_file(path: &std::path::Path, label: &str) -> crate::error::Result<()> {
    if path.is_file() {
        return Ok(());
    }
    super::super::peer::write_value(path, &receipt::signal(label))
}

/// Bounded asynchronous wait for a producer or peer signal, then a strict read.
async fn await_value(
    path: &std::path::Path,
    deadline: tokio::time::Instant,
) -> crate::error::Result<preserves::IOValue> {
    while !path.is_file() {
        require_before(deadline)?;
        tokio::time::sleep(SIGNAL_POLL).await;
    }
    super::super::peer::wait_value(path, SIGNAL_POLL)
}

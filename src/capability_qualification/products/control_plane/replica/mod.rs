//! One live Raft replica peer process. It assembles the production scoped live
//! replica (capability-rooted Redb durable log, real Iroh transport and ingress
//! pump, Tokio time port, admitted application port, control channel) under the
//! fixed group, and drives only explicit, phase-signalled events so leader
//! identity, terms and the commit quorum do not depend on timer races.

mod assembly;
mod fresh;
mod restart;
pub(super) mod stale;

const EVENT_POLL_MILLISECONDS: u64 = 100;
const EVENT_POLL: std::time::Duration = std::time::Duration::from_millis(EVENT_POLL_MILLISECONDS);
const SIGNAL_POLL: std::time::Duration = std::time::Duration::from_millis(10);
/// Bound of every peer phase; the producer kills a peer long before it.
const PHASE_BOUND_SECONDS: u64 = 120;
const PHASE_BOUND_NANOS: u64 = PHASE_BOUND_SECONDS * 1_000_000_000;
const INGRESS_DELIVERY_LIMIT: u64 = 1_024;
const INGRESS_DELIVERY_WINDOW_SECONDS: u64 = 1;
const INGRESS_DELIVERY_WINDOW: std::time::Duration = std::time::Duration::from_secs(INGRESS_DELIVERY_WINDOW_SECONDS);
/// Upper bound of event turns in one peer phase. A turn either idles one
/// `EVENT_POLL` or consumes one ingress delivery, and the ingress pump admits
/// at most `INGRESS_DELIVERY_LIMIT` deliveries per `INGRESS_DELIVERY_WINDOW`,
/// so the phase deadline passes before this many turns.
const PHASE_TURNS: u64 = PHASE_BOUND_SECONDS * 1_000 / EVENT_POLL_MILLISECONDS
    + (PHASE_BOUND_SECONDS / INGRESS_DELIVERY_WINDOW_SECONDS + 1) * INGRESS_DELIVERY_LIMIT;

type Ports = crate::fabric_consistency::raft::ConcreteReplicaPortBundle<
    crate::fabric_time::OperatingSystemEntropySource,
    AppliedCommands,
>;

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

impl crate::fabric_consistency::raft::CommittedBatchHandler for AppliedCommands {
    fn restore_snapshot(
        &mut self,
        _snapshot: &crate::fabric_consistency::raft::SnapshotRestore,
    ) -> crate::error::Result<String> {
        // The qualification run never snapshots; a restore is outside its admitted history.
        Err(crate::capability_qualification::invalid(
            "qualification live Raft application admits no snapshot restore",
        ))
    }

    fn apply_batch(&mut self, commands: &[crate::fabric_consistency::raft::Command]) -> crate::error::Result<String> {
        self.request_refs.extend(commands.iter().map(|command| command.request_ref.clone()));
        self.state_ref()
    }
}

pub(super) fn application_state_value(request_refs: &[String]) -> preserves::IOValue {
    crate::preserves_rail::record("qualification-live-raft-application-state-v1", vec![
        crate::preserves_rail::sequence(request_refs.iter().map(crate::preserves_rail::string).collect()),
    ])
}

pub(super) fn committed_log_value(entries: &[crate::fabric_consistency::raft::ReplicatedEntry]) -> preserves::IOValue {
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
    super::fixture::reference("committed-request")
}

pub(super) fn command_ref() -> String {
    super::fixture::reference("committed-command")
}

struct Node {
    node_id: String,
    endpoint_identity: String,
    service: crate::fabric_consistency::raft::ScopedLiveReplica<Ports>,
    ingress: Option<crate::fabric_consistency::raft::ReplicaIngressPump>,
    recovery: Option<super::receipt::Recovery>,
    _control: tokio::sync::mpsc::Receiver<crate::fabric_consistency::raft::ReplicaObservation>,
}

/// The monotonic deadline of one peer phase, read through the live clock
/// capability. It is checked only between turns, so a selected event always
/// finishes its handling before the bound can stop the peer.
struct Deadline {
    clock: crate::fabric_time::LiveClockAdapter,
}

impl Deadline {
    fn start() -> crate::error::Result<Self> {
        let time = super::fixture::time_profile()?;
        Ok(Self {
            clock: crate::fabric_time::LiveClockAdapter::new(&time.profile, 0)?,
        })
    }

    fn require_before(&mut self) -> crate::error::Result<()> {
        if self.clock.observe_monotonic()?.ticks >= PHASE_BOUND_NANOS {
            return Err(exceeded_bound());
        }
        Ok(())
    }
}

/// Peer entry: one replica process in one mode, bounded by its phase deadline.
pub(super) fn run(node_id: &str, mode: Mode, run_directory: &std::path::Path) -> crate::error::Result<()> {
    let runtime = tokio::runtime::Runtime::new().map_err(crate::error::Failure::from)?;
    runtime.block_on(async {
        let mut deadline = Deadline::start()?;
        let mut node = assembly::open(node_id, mode, run_directory, &mut deadline).await?;
        match (mode, node_id) {
            (Mode::Fresh, super::fixture::NODE_A) => fresh::leader(&mut node, run_directory, &mut deadline).await,
            (Mode::Fresh, _) => fresh::follower(&mut node, run_directory, &mut deadline).await,
            (Mode::Recover, _) => restart::recovered(node, run_directory, &mut deadline).await,
        }
    })
}

fn exceeded_bound() -> crate::error::Failure {
    crate::capability_qualification::invalid("live Raft replica exceeded its bounded phase")
}

fn node_receipt(node: &Node) -> crate::error::Result<super::receipt::Replica> {
    let state = node.service.state();
    let committed = state.log.iter().filter(|entry| entry.index <= state.commit_index).cloned().collect::<Vec<_>>();
    let handler = node.service.ports().application.handler();
    let commit_quorum = node
        .service
        .evidence()
        .records()
        .iter()
        .find(|record| record.kind == crate::fabric_consistency::raft::ReplicaKind::Commit)
        .map(|record| super::receipt::CommitQuorum {
            term: record.term,
            index: record.index,
            source_ref: record.source_ref.clone(),
            evidence_ref: record.quorum_evidence_ref.clone().unwrap_or_default(),
            members: record.quorum_members.clone(),
        });
    Ok(super::receipt::Replica {
        node_id: node.node_id.clone(),
        process_id: u64::from(std::process::id()),
        endpoint_identity: node.endpoint_identity.clone(),
        role: state.role.as_str().to_string(),
        is_voter: super::receipt::rejoin_is_voter(&state.rejoin),
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
            .map_err(|_| crate::capability_qualification::invalid("live Raft durable record count exceeds u64"))?,
        commit_quorum,
        recovery: node.recovery.clone(),
    })
}

fn is_recovering(node: &Node) -> bool {
    matches!(node.service.state().rejoin, crate::fabric_consistency::raft::ReplicaRejoin::Recovering(_))
}

async fn election_timeout(node: &mut Node) -> crate::error::Result<()> {
    let timer_ref = node.service.state().active_election_timer_ref.clone();
    require_applied(
        node.service
            .handle_event(crate::fabric_consistency::raft::ReplicaEvent::ElectionTimeout { timer_ref })
            .await,
    )
}

async fn poll_event(node: &mut Node) -> crate::error::Result<Option<crate::fabric_consistency::raft::ReplicaEvent>> {
    let ingress = node
        .ingress
        .as_mut()
        .ok_or_else(|| crate::capability_qualification::invalid("live Raft ingress pump is absent"))?;
    tokio::select! {
        received = ingress.next() => received.map(|received| Some(received.event)),
        () = tokio::time::sleep(EVENT_POLL) => Ok(None),
    }
}

fn require_applied(outcome: crate::fabric_consistency::raft::ReplicaExecutionOutcome) -> crate::error::Result<()> {
    match outcome {
        crate::fabric_consistency::raft::ReplicaExecutionOutcome::Applied(_) => Ok(()),
        crate::fabric_consistency::raft::ReplicaExecutionOutcome::Denied { diagnostic, .. } => {
            Err(crate::capability_qualification::invalid(format!("live Raft replica turn denied: {diagnostic}")))
        }
        crate::fabric_consistency::raft::ReplicaExecutionOutcome::Failed(failed) => {
            Err(crate::capability_qualification::invalid(format!(
                "live Raft replica effect {} failed: {}",
                failed.failed_kind.as_str(),
                failed.diagnostic
            )))
        }
    }
}

fn publish(run_directory: &std::path::Path, node_id: &str, phase: &'static str) -> crate::error::Result<()> {
    publish_file(&super::receipt::node_file(run_directory, node_id, phase), phase)
}

fn publish_file(path: &std::path::Path, label: &str) -> crate::error::Result<()> {
    if path.is_file() {
        return Ok(());
    }
    super::super::peer::write_value(path, &super::receipt::signal(label))
}

/// Bounded asynchronous wait for a producer or peer signal, then a strict read.
async fn await_value(path: &std::path::Path, deadline: &mut Deadline) -> crate::error::Result<preserves::IOValue> {
    while !path.is_file() {
        deadline.require_before()?;
        tokio::time::sleep(SIGNAL_POLL).await;
    }
    super::super::peer::wait_value(path, SIGNAL_POLL)
}

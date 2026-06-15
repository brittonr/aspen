use std::fs;
use std::path::Path;
use std::path::PathBuf;

use clap::Subcommand;
use molten::error::MoltenError;
use molten::error::Result;
use molten::node_daemon;
use molten::node_runtime;
use molten::preserves_rail::canonical_hash;
use molten::preserves_rail::parse_text;
use molten::preserves_rail::to_text;
use molten::provenance;

#[derive(Debug, Subcommand)]
pub(crate) enum NodeCommand {
    Init {
        #[arg(long)]
        state_root: PathBuf,
        #[arg(long, default_value = "node:local")]
        node_id: String,
        #[arg(long)]
        config_out: Option<PathBuf>,
        #[arg(long)]
        identity_receipt_out: Option<PathBuf>,
    },
    Run {
        #[arg(long)]
        state_root: PathBuf,
        #[arg(long)]
        startup_out: Option<PathBuf>,
    },
    RunLoop {
        #[arg(long)]
        state_root: PathBuf,
        #[arg(long, default_value_t = node_daemon::DEFAULT_CONTROL_LOOP_REQUESTS)]
        max_requests: u64,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
        #[arg(long)]
        heartbeat_out: Option<PathBuf>,
    },
    Serve {
        #[arg(long)]
        state_root: PathBuf,
        #[arg(long, default_value = node_daemon::DEFAULT_CONTROL_INGRESS_TOPIC)]
        topic: String,
        #[arg(long, default_value_t = node_daemon::DEFAULT_CONTROL_SERVICE_TICKS)]
        max_ticks: u64,
        #[arg(long, default_value_t = node_daemon::DEFAULT_CONTROL_LOOP_REQUESTS)]
        max_requests_per_tick: u64,
        #[arg(long)]
        live_iroh: bool,
        #[arg(long, default_value_t = node_daemon::DEFAULT_CONTROL_LIVE_LISTENER_EVENTS)]
        live_max_events: u64,
        #[arg(long, default_value_t = node_daemon::DEFAULT_CONTROL_LIVE_LISTENER_TIMEOUT_MS)]
        live_event_timeout_ms: u64,
        #[arg(long)]
        service_receipt_out: Option<PathBuf>,
        #[arg(long)]
        live_ticket_out: Option<PathBuf>,
        #[arg(long)]
        supervisor_policy: Option<PathBuf>,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    Status {
        #[arg(long)]
        state_root: PathBuf,
        #[arg(long)]
        health_out: Option<PathBuf>,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    Stop {
        #[arg(long)]
        state_root: PathBuf,
        #[arg(long)]
        shutdown_out: Option<PathBuf>,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    Show {
        artifact: PathBuf,
    },
    ControlRequest {
        #[arg(long)]
        operation: String,
        #[arg(long)]
        out: PathBuf,
        #[arg(long)]
        target: Option<String>,
        #[arg(long)]
        payload: Option<String>,
        #[arg(long = "authority")]
        authority_refs: Vec<String>,
        #[arg(long = "policy")]
        policy_refs: Vec<String>,
        #[arg(long = "resource")]
        resource_refs: Vec<String>,
        #[arg(long = "evidence")]
        evidence_refs: Vec<String>,
    },
    ProvenanceFixture {
        #[arg(long)]
        artifact_ref: String,
        #[arg(long)]
        out: PathBuf,
    },
    AuthorityGrantFixture {
        #[arg(long)]
        state_root: Option<PathBuf>,
        #[arg(long)]
        peer: String,
        #[arg(long)]
        node: String,
        #[arg(long = "operation")]
        operations: Vec<String>,
        #[arg(long, default_value = "*")]
        target_scope: String,
        #[arg(long, default_value = "*")]
        resource_scope: String,
        #[arg(long, default_value_t = 1)]
        epoch: u64,
        #[arg(long)]
        expires_at: Option<u64>,
        #[arg(long = "policy")]
        policy_refs: Vec<String>,
        #[arg(long = "revocation")]
        revocation_refs: Vec<String>,
        #[arg(long = "evidence")]
        evidence_refs: Vec<String>,
        #[arg(long)]
        out: PathBuf,
    },
    AuthorityGrantImport {
        #[arg(long)]
        state_root: PathBuf,
        grant: PathBuf,
        #[arg(long)]
        peer: Option<String>,
        #[arg(long)]
        node: Option<String>,
        #[arg(long = "operation")]
        operations: Vec<String>,
        #[arg(long)]
        target_scope: Option<String>,
        #[arg(long)]
        resource_scope: Option<String>,
        #[arg(long, default_value_t = 1)]
        as_of_epoch: u64,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    SupervisorPolicyFixture {
        #[arg(long)]
        state_root: Option<PathBuf>,
        #[arg(long, default_value_t = 0)]
        max_restarts: u64,
        #[arg(long, default_value_t = 1)]
        restart_window_ticks: u64,
        #[arg(long, default_value_t = 1)]
        heartbeat_timeout_ticks: u64,
        #[arg(long, default_value_t = 1)]
        shutdown_drain_ticks: u64,
        #[arg(long)]
        allow_stale_lock_recovery: bool,
        #[arg(long = "policy")]
        policy_refs: Vec<String>,
        #[arg(long = "evidence")]
        evidence_refs: Vec<String>,
        #[arg(long)]
        out: PathBuf,
    },
    LiveTicketExport {
        #[arg(long)]
        state_root: PathBuf,
        #[arg(long, default_value = node_daemon::DEFAULT_CONTROL_INGRESS_TOPIC)]
        topic: String,
        #[arg(long = "policy")]
        policy_refs: Vec<String>,
        #[arg(long = "evidence")]
        evidence_refs: Vec<String>,
        #[arg(long)]
        out: PathBuf,
    },
    LiveTicketImport {
        #[arg(long)]
        state_root: PathBuf,
        ticket: PathBuf,
        #[arg(long)]
        peer_admission: Option<PathBuf>,
        #[arg(long)]
        expected_node: Option<String>,
        #[arg(long)]
        expected_topic: Option<String>,
        #[arg(long)]
        expected_endpoint: Option<String>,
        #[arg(long)]
        expected_peer: Option<String>,
        #[arg(long, default_value_t = 1)]
        as_of_sequence: u64,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    LivePeerAdmit {
        #[arg(long)]
        state_root: PathBuf,
        #[arg(long)]
        peer: String,
        #[arg(long, default_value_t = 1)]
        sequence: u64,
        #[arg(long)]
        expires_at: Option<u64>,
        #[arg(long = "policy")]
        policy_refs: Vec<String>,
        #[arg(long = "evidence")]
        evidence_refs: Vec<String>,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
        ticket: PathBuf,
    },
    ControlSubmit {
        #[arg(long)]
        state_root: PathBuf,
        request: PathBuf,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    ControlDispatch {
        #[arg(long)]
        state_root: PathBuf,
        #[arg(long)]
        request: Option<PathBuf>,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    ControlIngressBuild {
        request: PathBuf,
        #[arg(long)]
        out: PathBuf,
        #[arg(long)]
        from_peer: String,
        #[arg(long)]
        to_node: String,
        #[arg(long, default_value = node_daemon::DEFAULT_CONTROL_INGRESS_TOPIC)]
        topic: String,
        #[arg(long, default_value_t = 1)]
        sequence: u64,
        #[arg(long = "peer-bootstrap")]
        peer_bootstrap_refs: Vec<String>,
        #[arg(long = "authority")]
        authority_refs: Vec<String>,
        #[arg(long = "policy")]
        policy_refs: Vec<String>,
        #[arg(long = "resource")]
        resource_refs: Vec<String>,
        #[arg(long = "evidence")]
        evidence_refs: Vec<String>,
    },
    ControlIngressLiveBuild {
        request: PathBuf,
        #[arg(long)]
        out: PathBuf,
        #[arg(long)]
        from_peer: String,
        #[arg(long)]
        to_node: String,
        #[arg(long, default_value = node_daemon::DEFAULT_CONTROL_INGRESS_TOPIC)]
        topic: String,
        #[arg(long, default_value_t = 1)]
        sequence: u64,
        #[arg(long = "peer-bootstrap")]
        peer_bootstrap_refs: Vec<String>,
        #[arg(long = "authority")]
        authority_refs: Vec<String>,
        #[arg(long = "policy")]
        policy_refs: Vec<String>,
        #[arg(long = "resource")]
        resource_refs: Vec<String>,
        #[arg(long = "evidence")]
        evidence_refs: Vec<String>,
    },
    ControlIngressLiveLoopback {
        #[arg(long)]
        state_root: PathBuf,
        request: PathBuf,
        #[arg(long)]
        from_peer: String,
        #[arg(long)]
        to_node: String,
        #[arg(long, default_value = node_daemon::DEFAULT_CONTROL_INGRESS_TOPIC)]
        topic: String,
        #[arg(long, default_value_t = 1)]
        sequence: u64,
        #[arg(long = "peer-bootstrap")]
        peer_bootstrap_refs: Vec<String>,
        #[arg(long = "authority")]
        authority_refs: Vec<String>,
        #[arg(long = "policy")]
        policy_refs: Vec<String>,
        #[arg(long = "resource")]
        resource_refs: Vec<String>,
        #[arg(long = "evidence")]
        evidence_refs: Vec<String>,
        #[arg(long)]
        publish_receipt_out: Option<PathBuf>,
        #[arg(long)]
        receive_receipt_out: Option<PathBuf>,
    },
    ControlIngressLiveSend {
        #[arg(long)]
        state_root: Option<PathBuf>,
        request: PathBuf,
        ticket: PathBuf,
        #[arg(long)]
        from_peer: String,
        #[arg(long, default_value_t = 1)]
        sequence: u64,
        #[arg(long = "operation-id")]
        operation_id: Option<String>,
        #[arg(long = "expected-node")]
        expected_node: Option<String>,
        #[arg(long = "expected-topic")]
        expected_topic: Option<String>,
        #[arg(long = "expected-endpoint")]
        expected_endpoint: Option<String>,
        #[arg(long, default_value_t = node_daemon::DEFAULT_CONTROL_LIVE_SEND_ATTEMPTS)]
        max_attempts: u64,
        #[arg(long = "peer-bootstrap")]
        peer_bootstrap_refs: Vec<String>,
        #[arg(long = "authority")]
        authority_refs: Vec<String>,
        #[arg(long = "policy")]
        policy_refs: Vec<String>,
        #[arg(long = "resource")]
        resource_refs: Vec<String>,
        #[arg(long = "evidence")]
        evidence_refs: Vec<String>,
        #[arg(long, default_value_t = 10_000)]
        join_timeout_ms: u64,
        #[arg(long)]
        transport_receipt_out: Option<PathBuf>,
        #[arg(long)]
        retry_receipts_dir: Option<PathBuf>,
        #[arg(long)]
        duplicate_receipt_out: Option<PathBuf>,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    LiveWorkflowBundle {
        #[arg(long)]
        state_root: Option<PathBuf>,
        #[arg(long)]
        ticket: PathBuf,
        #[arg(long)]
        peer_admission: PathBuf,
        #[arg(long)]
        authority_grant: PathBuf,
        #[arg(long)]
        send_receipt: PathBuf,
        #[arg(long = "receive-receipt")]
        receive_receipts: Vec<PathBuf>,
        #[arg(long)]
        listener_receipt: Option<PathBuf>,
        #[arg(long)]
        service_receipt: PathBuf,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    LiveWorkflowBundleExport {
        #[arg(long)]
        ticket: PathBuf,
        #[arg(long)]
        peer_admission: PathBuf,
        #[arg(long)]
        authority_grant: PathBuf,
        #[arg(long = "receipt")]
        receipt_values: Vec<PathBuf>,
        #[arg(long)]
        out: PathBuf,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    LiveWorkflowBundleVerify {
        bundle: PathBuf,
        #[arg(long)]
        expected_node: Option<String>,
        #[arg(long)]
        expected_topic: Option<String>,
        #[arg(long)]
        expected_endpoint: Option<String>,
        #[arg(long)]
        expected_peer: Option<String>,
        #[arg(long = "operation")]
        operations: Vec<String>,
        #[arg(long)]
        target_scope: Option<String>,
        #[arg(long)]
        resource_scope: Option<String>,
        #[arg(long, default_value_t = 1)]
        as_of_sequence: u64,
        #[arg(long, default_value_t = 1)]
        as_of_epoch: u64,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    LiveWorkflowBundleGate {
        bundle: PathBuf,
        #[arg(long)]
        verify_receipt: Option<PathBuf>,
        #[arg(long)]
        require_verify_receipt: bool,
        #[arg(long)]
        expected_node: Option<String>,
        #[arg(long)]
        expected_topic: Option<String>,
        #[arg(long)]
        expected_endpoint: Option<String>,
        #[arg(long)]
        expected_peer: Option<String>,
        #[arg(long = "operation")]
        operations: Vec<String>,
        #[arg(long)]
        target_scope: Option<String>,
        #[arg(long)]
        resource_scope: Option<String>,
        #[arg(long, default_value_t = 1)]
        as_of_sequence: u64,
        #[arg(long, default_value_t = 1)]
        as_of_epoch: u64,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    LiveWorkflowBundleApply {
        #[arg(long)]
        state_root: PathBuf,
        bundle: PathBuf,
        #[arg(long)]
        gate_receipt: Option<PathBuf>,
        #[arg(long)]
        require_gate_receipt: bool,
        #[arg(long)]
        request: Option<PathBuf>,
        #[arg(long)]
        send: bool,
        #[arg(long)]
        from_peer: Option<String>,
        #[arg(long, default_value_t = 1)]
        sequence: u64,
        #[arg(long = "operation-id")]
        operation_id: Option<String>,
        #[arg(long)]
        expected_node: Option<String>,
        #[arg(long)]
        expected_topic: Option<String>,
        #[arg(long)]
        expected_endpoint: Option<String>,
        #[arg(long)]
        expected_peer: Option<String>,
        #[arg(long = "operation")]
        operations: Vec<String>,
        #[arg(long)]
        target_scope: Option<String>,
        #[arg(long)]
        resource_scope: Option<String>,
        #[arg(long, default_value_t = 1)]
        as_of_sequence: u64,
        #[arg(long, default_value_t = 1)]
        as_of_epoch: u64,
        #[arg(long = "peer-bootstrap")]
        peer_bootstrap_refs: Vec<String>,
        #[arg(long = "authority")]
        authority_refs: Vec<String>,
        #[arg(long = "policy")]
        policy_refs: Vec<String>,
        #[arg(long = "resource")]
        resource_refs: Vec<String>,
        #[arg(long = "evidence")]
        evidence_refs: Vec<String>,
        #[arg(long, default_value_t = node_daemon::DEFAULT_CONTROL_LIVE_SEND_ATTEMPTS)]
        max_attempts: u64,
        #[arg(long, default_value_t = 10_000)]
        join_timeout_ms: u64,
        #[arg(long)]
        send_receipt_out: Option<PathBuf>,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    LiveWorkflowBundleReconcile {
        apply_receipt: PathBuf,
        #[arg(long)]
        send_receipt: Option<PathBuf>,
        #[arg(long)]
        ingress_receipt: Option<PathBuf>,
        #[arg(long)]
        queue_receipt: Option<PathBuf>,
        #[arg(long)]
        control_receipt: Option<PathBuf>,
        #[arg(long)]
        expected_envelope: Option<String>,
        #[arg(long)]
        expected_operation: Option<String>,
        #[arg(long)]
        expected_request: Option<String>,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    LiveWorkflowBundleAckExport {
        apply_receipt: PathBuf,
        #[arg(long)]
        send_receipt: Option<PathBuf>,
        #[arg(long)]
        ingress_receipt: Option<PathBuf>,
        #[arg(long)]
        queue_receipt: Option<PathBuf>,
        #[arg(long)]
        control_receipt: Option<PathBuf>,
        #[arg(long)]
        reconcile_receipt: PathBuf,
        #[arg(long)]
        out: PathBuf,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    LiveWorkflowBundleAckImport {
        #[arg(long)]
        state_root: PathBuf,
        ack: PathBuf,
        #[arg(long)]
        expected_bundle: Option<String>,
        #[arg(long)]
        expected_envelope: Option<String>,
        #[arg(long)]
        expected_operation: Option<String>,
        #[arg(long)]
        expected_request: Option<String>,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    LiveWorkflowBundleProtocolGate {
        bundle: PathBuf,
        #[arg(long)]
        gate_receipt: PathBuf,
        #[arg(long)]
        apply_receipt: PathBuf,
        #[arg(long)]
        reconcile_receipt: PathBuf,
        #[arg(long)]
        ack: PathBuf,
        #[arg(long)]
        expected_envelope: Option<String>,
        #[arg(long)]
        expected_operation: Option<String>,
        #[arg(long)]
        expected_request: Option<String>,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    LiveWorkflowBundleImport {
        #[arg(long)]
        state_root: PathBuf,
        bundle: PathBuf,
        #[arg(long)]
        expected_node: Option<String>,
        #[arg(long)]
        expected_topic: Option<String>,
        #[arg(long)]
        expected_endpoint: Option<String>,
        #[arg(long)]
        expected_peer: Option<String>,
        #[arg(long = "operation")]
        operations: Vec<String>,
        #[arg(long)]
        target_scope: Option<String>,
        #[arg(long)]
        resource_scope: Option<String>,
        #[arg(long, default_value_t = 1)]
        as_of_sequence: u64,
        #[arg(long, default_value_t = 1)]
        as_of_epoch: u64,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    ControlIngressPublish {
        #[arg(long)]
        state_root: PathBuf,
        envelope: PathBuf,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    ControlIngressDeliver {
        #[arg(long)]
        state_root: PathBuf,
        #[arg(long, default_value = node_daemon::DEFAULT_CONTROL_INGRESS_TOPIC)]
        topic: String,
        envelope_ref: String,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    ControlDeny {
        request: PathBuf,
        #[arg(long)]
        startup: String,
        #[arg(long)]
        diagnostic: String,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    Shutdown {
        #[arg(long)]
        startup: String,
        #[arg(long = "adapter")]
        adapters: Vec<String>,
        #[arg(long = "drained-job")]
        drained_jobs: Vec<String>,
        #[arg(long = "index")]
        index_receipt_refs: Vec<String>,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    Health {
        startup_receipt: PathBuf,
        #[arg(long)]
        shutdown: Option<String>,
        #[arg(long = "index")]
        index_receipt_refs: Vec<String>,
        #[arg(long = "head")]
        head_refs: Vec<String>,
        #[arg(long = "open-job")]
        open_job_refs: Vec<String>,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
}

pub(crate) fn run_node_command(command: NodeCommand) -> Result<()> {
    match command {
        NodeCommand::Init {
            state_root,
            node_id,
            config_out,
            identity_receipt_out,
        } => {
            let init = node_daemon::init_local_node(&node_daemon::NodeDaemonInitInput {
                state_root: &state_root,
                node_id: &node_id,
            })?;
            if let Some(path) = config_out.as_ref() {
                write_file(path, &to_text(&init.config_value)?)?;
            }
            if let Some(path) = identity_receipt_out.as_ref() {
                write_file(path, &to_text(&init.identity_receipt_value)?)?;
            }
            println!(
                "node init config={} identity={} identity_receipt={} state_root={}",
                init.config_ref,
                init.identity_ref,
                init.identity_receipt_ref,
                state_root.display()
            );
            Ok(())
        }
        NodeCommand::Run {
            state_root,
            startup_out,
        } => {
            let run = node_daemon::run_local_node(&node_daemon::NodeDaemonRunInput {
                state_root: &state_root,
            })?;
            if let Some(path) = startup_out.as_ref() {
                write_file(path, &to_text(&run.startup_value)?)?;
            }
            println!(
                "node run startup={} adapters={} state_root={}",
                run.startup_ref,
                run.adapter_receipt_refs.len(),
                state_root.display()
            );
            Ok(())
        }
        NodeCommand::RunLoop {
            state_root,
            max_requests,
            receipt_out,
            heartbeat_out,
        } => {
            let loop_run = node_daemon::run_control_loop(&node_daemon::NodeControlLoopInput {
                state_root: &state_root,
                max_requests,
            })?;
            if let Some(path) = heartbeat_out.as_ref() {
                write_file(path, &to_text(&loop_run.heartbeat_receipt_value)?)?;
            }
            emit_named_receipt(receipt_out.as_ref(), "node control loop receipt", &loop_run.loop_receipt_value)?;
            println!(
                "node run-loop loop_receipt={} heartbeat={} processed={} stopped={}",
                loop_run.loop_receipt_ref,
                loop_run.heartbeat_receipt_ref,
                loop_run.processed_request_refs.len(),
                if loop_run.has_stopped { "yes" } else { "no" }
            );
            Ok(())
        }
        NodeCommand::Serve {
            state_root,
            topic,
            max_ticks,
            max_requests_per_tick,
            live_iroh,
            live_max_events,
            live_event_timeout_ms,
            service_receipt_out,
            live_ticket_out,
            supervisor_policy,
            receipt_out,
        } => {
            let supervisor_policy_value =
                supervisor_policy.as_ref().map(|path| read_preserves_file(path)).transpose()?;
            if live_iroh {
                let runtime =
                    tokio::runtime::Builder::new_multi_thread().enable_all().build().map_err(MoltenError::from)?;
                let served = runtime.block_on(node_daemon::serve_node_control_live_listener(
                    &node_daemon::NodeControlLiveServeInput {
                        state_root: &state_root,
                        topic: &topic,
                        max_events: live_max_events,
                        event_timeout_ms: live_event_timeout_ms,
                        max_requests_per_tick,
                        supervisor_policy_value: supervisor_policy_value.as_ref(),
                    },
                ))?;
                if let Some(path) = service_receipt_out.as_ref() {
                    write_file(path, &to_text(&served.service.service_receipt_value)?)?;
                }
                if let Some(path) = live_ticket_out.as_ref()
                    && let Some(ticket_value) = served.live_ticket_value.as_ref()
                {
                    write_file(path, &to_text(ticket_value)?)?;
                }
                emit_named_receipt(
                    receipt_out.as_ref(),
                    "node control live listener receipt",
                    &served.listener_receipt_value,
                )?;
                println!(
                    "node serve live-iroh listener={} service={} endpoint={} events={} transports={} processed={} stopped={}",
                    served.listener_receipt_ref,
                    served.service.service_receipt_ref,
                    served.bound_endpoint_id,
                    served.observed_events,
                    served.transport_receipt_refs.len(),
                    served.service.processed_request_refs.len(),
                    if served.service.has_stopped { "yes" } else { "no" }
                );
                Ok(())
            } else {
                let served = node_daemon::serve_node_control(&node_daemon::NodeControlServeInput {
                    state_root: &state_root,
                    topic: &topic,
                    max_ticks,
                    max_requests_per_tick,
                    supervisor_policy_value: supervisor_policy_value.as_ref(),
                })?;
                emit_named_receipt(
                    receipt_out.as_ref(),
                    "node control service run receipt",
                    &served.service_receipt_value,
                )?;
                println!(
                    "node serve decision={} receipt={} ticks={} heartbeats={} ingress={} loops={} processed={} stopped={}",
                    served.decision,
                    served.service_receipt_ref,
                    served.ticks,
                    served.heartbeat_receipt_refs.len(),
                    served.ingress_receipt_refs.len(),
                    served.loop_receipt_refs.len(),
                    served.processed_request_refs.len(),
                    if served.has_stopped { "yes" } else { "no" }
                );
                Ok(())
            }
        }
        NodeCommand::Status {
            state_root,
            health_out,
            receipt_out,
        } => {
            let status = node_daemon::status_local_node(&node_daemon::NodeDaemonStatusInput {
                state_root: &state_root,
            })?;
            if let Some(path) = health_out.as_ref() {
                write_file(path, &to_text(&status.health_value)?)?;
            }
            emit_named_receipt(receipt_out.as_ref(), "node control receipt", &status.control_receipt_value)?;
            println!(
                "node status {} health={} control_receipt={}",
                status.status, status.health_ref, status.control_receipt_ref
            );
            Ok(())
        }
        NodeCommand::Stop {
            state_root,
            shutdown_out,
            receipt_out,
        } => {
            let stop = node_daemon::stop_local_node(&node_daemon::NodeDaemonStopInput {
                state_root: &state_root,
            })?;
            if let Some(path) = shutdown_out.as_ref() {
                write_file(path, &to_text(&stop.shutdown_value)?)?;
            }
            emit_named_receipt(receipt_out.as_ref(), "node control receipt", &stop.control_receipt_value)?;
            println!("node stop shutdown={} control_receipt={}", stop.shutdown_ref, stop.control_receipt_ref);
            Ok(())
        }
        NodeCommand::Show { artifact } => {
            let value = read_preserves_file(&artifact)?;
            println!("{}", node_daemon::node_daemon_summary(&value)?);
            Ok(())
        }
        NodeCommand::ControlRequest {
            operation,
            out,
            target,
            payload,
            authority_refs,
            policy_refs,
            resource_refs,
            evidence_refs,
        } => {
            let value = node_runtime::node_control_request_value(&node_runtime::ControlRequestValueInput {
                operation: &operation,
                target_ref: target.as_deref(),
                payload_ref: payload.as_deref(),
                authority_refs: &authority_refs,
                policy_refs: &policy_refs,
                resource_refs: &resource_refs,
                evidence_refs: &evidence_refs,
            })?;
            write_file(&out, &to_text(&value)?)?;
            println!("node control request {} written to {}", canonical_hash(&value)?, out.display());
            Ok(())
        }
        NodeCommand::ProvenanceFixture { artifact_ref, out } => {
            let value = provenance::synthetic_reviewed_provenance_record(&artifact_ref)?;
            write_file(&out, &to_text(&value)?)?;
            println!("node provenance fixture {} written to {}", canonical_hash(&value)?, out.display());
            Ok(())
        }
        NodeCommand::AuthorityGrantFixture {
            state_root,
            peer,
            node,
            operations,
            target_scope,
            resource_scope,
            epoch,
            expires_at,
            policy_refs,
            revocation_refs,
            evidence_refs,
            out,
        } => {
            let operations = if operations.is_empty() {
                vec!["status".to_string()]
            } else {
                operations
            };
            let value =
                node_daemon::node_control_authority_grant_value(&node_daemon::NodeControlAuthorityGrantInput {
                    peer_id: &peer,
                    node_id: &node,
                    operations: &operations,
                    target_scope: &target_scope,
                    resource_scope: &resource_scope,
                    epoch,
                    expires_at,
                    policy_refs: &policy_refs,
                    revocation_refs: &revocation_refs,
                    evidence_refs: &evidence_refs,
                })?;
            let grant_ref = canonical_hash(&value)?;
            write_file(&out, &to_text(&value)?)?;
            if let Some(state_root) = state_root.as_ref() {
                node_daemon::import_node_control_authority_grant(state_root, &value)?;
            }
            println!("node authority grant {} written to {}", grant_ref, out.display());
            Ok(())
        }
        NodeCommand::AuthorityGrantImport {
            state_root,
            grant,
            peer,
            node,
            operations,
            target_scope,
            resource_scope,
            as_of_epoch,
            receipt_out,
        } => {
            let grant_value = read_preserves_file(&grant)?;
            let imported = node_daemon::import_node_control_authority_grant_checked(
                &node_daemon::NodeControlAuthorityGrantImportInput {
                    state_root: &state_root,
                    grant_value: &grant_value,
                    expected_peer: peer.as_deref(),
                    expected_node: node.as_deref(),
                    expected_operations: &operations,
                    expected_target_scope: target_scope.as_deref(),
                    expected_resource_scope: resource_scope.as_deref(),
                    as_of_epoch,
                },
            )?;
            emit_named_receipt(receipt_out.as_ref(), "node authority grant import receipt", &imported.receipt_value)?;
            println!(
                "node authority grant import decision={} grant={} imported={} diagnostics={}",
                imported.decision,
                imported.grant_ref,
                imported.imported_refs.len(),
                imported.diagnostics.len()
            );
            Ok(())
        }
        NodeCommand::SupervisorPolicyFixture {
            state_root,
            max_restarts,
            restart_window_ticks,
            heartbeat_timeout_ticks,
            shutdown_drain_ticks,
            allow_stale_lock_recovery,
            policy_refs,
            evidence_refs,
            out,
        } => {
            let value =
                node_daemon::node_control_supervisor_policy_value(&node_daemon::NodeControlSupervisorPolicyInput {
                    max_restarts,
                    restart_window_ticks,
                    heartbeat_timeout_ticks,
                    shutdown_drain_ticks,
                    stale_lock_recovery: allow_stale_lock_recovery,
                    policy_refs: &policy_refs,
                    evidence_refs: &evidence_refs,
                })?;
            let policy_ref = canonical_hash(&value)?;
            write_file(&out, &to_text(&value)?)?;
            if let Some(state_root) = state_root.as_ref() {
                node_daemon::import_node_control_supervisor_policy(state_root, &value)?;
            }
            println!("node supervisor policy {} written to {}", policy_ref, out.display());
            Ok(())
        }
        NodeCommand::LiveTicketExport {
            state_root,
            topic,
            policy_refs,
            evidence_refs,
            out,
        } => {
            let ticket =
                node_daemon::export_node_control_live_ticket(&node_daemon::NodeControlLiveTicketExportInput {
                    state_root: &state_root,
                    topic: &topic,
                    policy_refs: &policy_refs,
                    evidence_refs: &evidence_refs,
                })?;
            write_file(&out, &to_text(&ticket.value)?)?;
            println!("node live ticket {} written to {}", ticket.ticket_ref, out.display());
            Ok(())
        }
        NodeCommand::LiveTicketImport {
            state_root,
            ticket,
            peer_admission,
            expected_node,
            expected_topic,
            expected_endpoint,
            expected_peer,
            as_of_sequence,
            receipt_out,
        } => {
            let ticket_value = read_preserves_file(&ticket)?;
            let peer_admission_value = peer_admission.as_ref().map(|path| read_preserves_file(path)).transpose()?;
            let imported =
                node_daemon::import_node_control_live_ticket(&node_daemon::NodeControlLiveTicketImportInput {
                    state_root: &state_root,
                    ticket_value: &ticket_value,
                    peer_admission_value: peer_admission_value.as_ref(),
                    expected_node: expected_node.as_deref(),
                    expected_topic: expected_topic.as_deref(),
                    expected_endpoint: expected_endpoint.as_deref(),
                    expected_peer: expected_peer.as_deref(),
                    as_of_sequence,
                })?;
            emit_named_receipt(receipt_out.as_ref(), "node live ticket import receipt", &imported.receipt_value)?;
            println!(
                "node live ticket import decision={} ticket={} admission={} imported={} diagnostics={}",
                imported.decision,
                imported.ticket_ref,
                imported.peer_admission_ref.as_deref().unwrap_or("none"),
                imported.imported_refs.len(),
                imported.diagnostics.len()
            );
            Ok(())
        }
        NodeCommand::LivePeerAdmit {
            state_root,
            peer,
            sequence,
            expires_at,
            policy_refs,
            evidence_refs,
            receipt_out,
            ticket,
        } => {
            let ticket_value = read_preserves_file(&ticket)?;
            let admission = node_daemon::admit_node_control_live_peer(&node_daemon::NodeControlLivePeerAdmitInput {
                state_root: &state_root,
                ticket_value: &ticket_value,
                peer_id: &peer,
                sequence,
                expires_at,
                policy_refs: &policy_refs,
                evidence_refs: &evidence_refs,
            })?;
            emit_named_receipt(receipt_out.as_ref(), "node live peer admission", &admission.value)?;
            println!(
                "node live peer admit decision={} admission={} peer={} node={} topic={}",
                admission.decision, admission.admission_ref, admission.peer_id, admission.node_id, admission.topic
            );
            Ok(())
        }
        NodeCommand::ControlSubmit {
            state_root,
            request,
            receipt_out,
        } => {
            let request_value = read_preserves_file(&request)?;
            let submitted = node_daemon::submit_control_request(&node_daemon::NodeControlSubmitInput {
                state_root: &state_root,
                request_value: &request_value,
            })?;
            emit_named_receipt(receipt_out.as_ref(), "node control queue receipt", &submitted.queue_receipt_value)?;
            println!(
                "node control submit request={} queue_receipt={} inbox={}",
                submitted.request_ref,
                submitted.queue_receipt_ref,
                submitted.inbox_path.display()
            );
            Ok(())
        }
        NodeCommand::ControlDispatch {
            state_root,
            request,
            receipt_out,
        } => {
            let dispatched = node_daemon::dispatch_control_request(&node_daemon::NodeControlDispatchInput {
                state_root: &state_root,
                request_path: request.as_deref(),
            })?;
            emit_named_receipt(receipt_out.as_ref(), "node control receipt", &dispatched.control_receipt_value)?;
            println!(
                "node control dispatch operation={} request={} control_receipt={} subreceipts={}",
                dispatched.operation,
                dispatched.request_ref,
                dispatched.control_receipt_ref,
                dispatched.subreceipt_refs.len()
            );
            Ok(())
        }
        NodeCommand::ControlIngressBuild {
            request,
            out,
            from_peer,
            to_node,
            topic,
            sequence,
            peer_bootstrap_refs,
            authority_refs,
            policy_refs,
            resource_refs,
            evidence_refs,
        } => {
            let request_value = read_preserves_file(&request)?;
            let envelope = node_daemon::node_control_ingress_envelope(&node_daemon::NodeControlIngressEnvelopeInput {
                request_value: &request_value,
                from_peer: &from_peer,
                to_node: &to_node,
                topic: &topic,
                sequence,
                peer_bootstrap_refs: &peer_bootstrap_refs,
                authority_refs: &authority_refs,
                policy_refs: &policy_refs,
                resource_refs: &resource_refs,
                evidence_refs: &evidence_refs,
            })?;
            write_file(&out, &to_text(&envelope.value)?)?;
            println!(
                "node control ingress envelope={} request={} written to {}",
                envelope.envelope_ref,
                envelope.request.request_ref,
                out.display()
            );
            Ok(())
        }
        NodeCommand::ControlIngressLiveBuild {
            request,
            out,
            from_peer,
            to_node,
            topic,
            sequence,
            peer_bootstrap_refs,
            authority_refs,
            policy_refs,
            resource_refs,
            evidence_refs,
        } => {
            let request_value = read_preserves_file(&request)?;
            let envelope =
                node_daemon::node_control_live_ingress_envelope(&node_daemon::NodeControlIngressEnvelopeInput {
                    request_value: &request_value,
                    from_peer: &from_peer,
                    to_node: &to_node,
                    topic: &topic,
                    sequence,
                    peer_bootstrap_refs: &peer_bootstrap_refs,
                    authority_refs: &authority_refs,
                    policy_refs: &policy_refs,
                    resource_refs: &resource_refs,
                    evidence_refs: &evidence_refs,
                })?;
            write_file(&out, &to_text(&envelope.value)?)?;
            println!(
                "node control live ingress envelope={} request={} written to {}",
                envelope.envelope_ref,
                envelope.request.request_ref,
                out.display()
            );
            Ok(())
        }
        NodeCommand::ControlIngressLiveLoopback {
            state_root,
            request,
            from_peer,
            to_node,
            topic,
            sequence,
            peer_bootstrap_refs,
            authority_refs,
            policy_refs,
            resource_refs,
            evidence_refs,
            publish_receipt_out,
            receive_receipt_out,
        } => {
            let request_value = read_preserves_file(&request)?;
            let runtime =
                tokio::runtime::Builder::new_multi_thread().enable_all().build().map_err(MoltenError::from)?;
            let loopback = runtime.block_on(node_daemon::node_control_live_iroh_loopback(
                &node_daemon::NodeControlLiveLoopbackInput {
                    state_root: &state_root,
                    request_value: &request_value,
                    from_peer: &from_peer,
                    to_node: &to_node,
                    topic: &topic,
                    sequence,
                    peer_bootstrap_refs: &peer_bootstrap_refs,
                    authority_refs: &authority_refs,
                    policy_refs: &policy_refs,
                    resource_refs: &resource_refs,
                    evidence_refs: &evidence_refs,
                },
            ))?;
            if let Some(path) = publish_receipt_out.as_ref() {
                write_file(path, &to_text(&loopback.publish_receipt_value)?)?;
            }
            emit_named_receipt(
                receive_receipt_out.as_ref(),
                "node control live transport receipt",
                &loopback.receive_receipt_value,
            )?;
            println!(
                "node control live ingress loopback envelope={} publish_receipt={} receive_receipt={} ingress_receipt={} enqueued={}",
                loopback.envelope_ref,
                loopback.publish_receipt_ref,
                loopback.receive_receipt_ref,
                loopback.ingress_receipt_ref,
                if loopback.has_enqueued { "yes" } else { "no" }
            );
            Ok(())
        }
        NodeCommand::ControlIngressLiveSend {
            state_root,
            request,
            ticket,
            from_peer,
            sequence,
            operation_id,
            expected_node,
            expected_topic,
            expected_endpoint,
            max_attempts,
            peer_bootstrap_refs,
            authority_refs,
            policy_refs,
            resource_refs,
            evidence_refs,
            join_timeout_ms,
            transport_receipt_out,
            retry_receipts_dir,
            duplicate_receipt_out,
            receipt_out,
        } => {
            let request_value = read_preserves_file(&request)?;
            let ticket_value = read_preserves_file(&ticket)?;
            let runtime =
                tokio::runtime::Builder::new_multi_thread().enable_all().build().map_err(MoltenError::from)?;
            let sent = runtime.block_on(node_daemon::send_node_control_live_ingress(
                &node_daemon::NodeControlLiveSendInput {
                    state_root: state_root.as_deref(),
                    request_value: &request_value,
                    receiver_ticket_value: &ticket_value,
                    from_peer: &from_peer,
                    sequence,
                    expected_operation_ref: operation_id.as_deref(),
                    expected_receiver_node: expected_node.as_deref(),
                    expected_topic: expected_topic.as_deref(),
                    expected_endpoint: expected_endpoint.as_deref(),
                    max_attempts,
                    peer_bootstrap_refs: &peer_bootstrap_refs,
                    authority_refs: &authority_refs,
                    policy_refs: &policy_refs,
                    resource_refs: &resource_refs,
                    evidence_refs: &evidence_refs,
                    join_timeout_ms,
                },
            ))?;
            if let (Some(path), Some(value)) = (transport_receipt_out.as_ref(), sent.transport_receipt_value.as_ref()) {
                write_file(path, &to_text(value)?)?;
            }
            if let Some(dir) = retry_receipts_dir.as_ref() {
                fs::create_dir_all(dir).map_err(MoltenError::from)?;
                for (reference, value) in sent.retry_receipt_refs.iter().zip(sent.retry_receipt_values.iter()) {
                    let path = dir.join(format!("{}.preserves", reference.replace(':', "-")));
                    write_file(&path, &to_text(value)?)?;
                }
            }
            if let (Some(path), Some(value)) = (duplicate_receipt_out.as_ref(), sent.duplicate_receipt_value.as_ref()) {
                write_file(path, &to_text(value)?)?;
            }
            emit_named_receipt(receipt_out.as_ref(), "node control live send receipt", &sent.send_receipt_value)?;
            println!(
                "node control live ingress send envelope={} operation={} ticket={} endpoint={} transport_receipt={} send_receipt={} retries={} duplicate_receipt={}",
                sent.envelope_ref,
                sent.operation_ref,
                sent.receiver_ticket_ref,
                sent.receiver_endpoint_id,
                sent.transport_receipt_ref.as_deref().unwrap_or("none"),
                sent.send_receipt_ref,
                sent.retry_receipt_refs.len(),
                sent.duplicate_receipt_ref.as_deref().unwrap_or("none")
            );
            Ok(())
        }
        NodeCommand::LiveWorkflowBundle {
            state_root,
            ticket,
            peer_admission,
            authority_grant,
            send_receipt,
            receive_receipts,
            listener_receipt,
            service_receipt,
            receipt_out,
        } => {
            let ticket_value = read_preserves_file(&ticket)?;
            let peer_admission_value = read_preserves_file(&peer_admission)?;
            let authority_grant_value = read_preserves_file(&authority_grant)?;
            let send_receipt_value = read_preserves_file(&send_receipt)?;
            let receive_values =
                receive_receipts.iter().map(|path| read_preserves_file(path)).collect::<Result<Vec<_>>>()?;
            let receive_value_refs = receive_values.iter().collect::<Vec<_>>();
            let listener_value = listener_receipt.as_ref().map(|path| read_preserves_file(path)).transpose()?;
            let service_receipt_value = read_preserves_file(&service_receipt)?;
            let workflow =
                node_daemon::node_control_live_workflow_receipt(&node_daemon::NodeControlLiveWorkflowInput {
                    state_root: state_root.as_deref(),
                    receiver_ticket_value: &ticket_value,
                    peer_admission_value: &peer_admission_value,
                    authority_grant_value: &authority_grant_value,
                    send_receipt_value: &send_receipt_value,
                    receive_receipt_values: &receive_value_refs,
                    listener_receipt_value: listener_value.as_ref(),
                    service_receipt_value: &service_receipt_value,
                })?;
            emit_named_receipt(receipt_out.as_ref(), "node control live workflow receipt", &workflow.receipt_value)?;
            println!(
                "node live workflow bundle decision={} receipt={} diagnostics={}",
                workflow.decision,
                workflow.receipt_ref,
                workflow.diagnostics.len()
            );
            Ok(())
        }
        NodeCommand::LiveWorkflowBundleExport {
            ticket,
            peer_admission,
            authority_grant,
            receipt_values,
            out,
            receipt_out,
        } => {
            let ticket_value = read_preserves_file(&ticket)?;
            let peer_admission_value = read_preserves_file(&peer_admission)?;
            let authority_grant_value = read_preserves_file(&authority_grant)?;
            let receipt_values =
                receipt_values.iter().map(|path| read_preserves_file(path)).collect::<Result<Vec<_>>>()?;
            let receipt_value_refs = receipt_values.iter().collect::<Vec<_>>();
            let exported = node_daemon::export_node_control_live_workflow_bundle(
                &node_daemon::NodeControlLiveWorkflowBundleExportInput {
                    receiver_ticket_value: &ticket_value,
                    peer_admission_value: &peer_admission_value,
                    authority_grant_value: &authority_grant_value,
                    receipt_values: &receipt_value_refs,
                },
            )?;
            write_file(&out, &to_text(&exported.bundle.bundle_value)?)?;
            emit_named_receipt(
                receipt_out.as_ref(),
                "node control live workflow bundle export receipt",
                &exported.receipt_value,
            )?;
            println!(
                "node live workflow bundle export decision={} bundle={} ticket={} admission={} grant={} diagnostics={}",
                exported.decision,
                exported.bundle.bundle_ref,
                exported.bundle.ticket_ref,
                exported.bundle.peer_admission_ref,
                exported.bundle.authority_grant_ref,
                exported.diagnostics.len()
            );
            Ok(())
        }
        NodeCommand::LiveWorkflowBundleVerify {
            bundle,
            expected_node,
            expected_topic,
            expected_endpoint,
            expected_peer,
            operations,
            target_scope,
            resource_scope,
            as_of_sequence,
            as_of_epoch,
            receipt_out,
        } => {
            let bundle_value = read_preserves_file(&bundle)?;
            let verified = node_daemon::verify_node_control_live_workflow_bundle(
                &node_daemon::NodeControlLiveWorkflowBundleVerifyInput {
                    bundle_value: &bundle_value,
                    expected_node: expected_node.as_deref(),
                    expected_topic: expected_topic.as_deref(),
                    expected_endpoint: expected_endpoint.as_deref(),
                    expected_peer: expected_peer.as_deref(),
                    expected_operations: &operations,
                    expected_target_scope: target_scope.as_deref(),
                    expected_resource_scope: resource_scope.as_deref(),
                    as_of_sequence,
                    as_of_epoch,
                },
            )?;
            emit_named_receipt(
                receipt_out.as_ref(),
                "node control live workflow bundle verify receipt",
                &verified.receipt_value,
            )?;
            println!(
                "node live workflow bundle verify decision={} bundle={} ticket={} admission={} grant={} diagnostics={}",
                verified.decision,
                verified.bundle_ref,
                verified.ticket_ref.as_deref().unwrap_or("none"),
                verified.peer_admission_ref.as_deref().unwrap_or("none"),
                verified.authority_grant_ref.as_deref().unwrap_or("none"),
                verified.diagnostics.len()
            );
            Ok(())
        }
        NodeCommand::LiveWorkflowBundleGate {
            bundle,
            verify_receipt,
            require_verify_receipt,
            expected_node,
            expected_topic,
            expected_endpoint,
            expected_peer,
            operations,
            target_scope,
            resource_scope,
            as_of_sequence,
            as_of_epoch,
            receipt_out,
        } => {
            let bundle_value = read_preserves_file(&bundle)?;
            let verify_receipt_value = verify_receipt.as_ref().map(|path| read_preserves_file(path)).transpose()?;
            let gated = node_daemon::gate_node_control_live_workflow_bundle(
                &node_daemon::NodeControlLiveWorkflowBundleGateInput {
                    bundle_value: &bundle_value,
                    verify_receipt_value: verify_receipt_value.as_ref(),
                    require_verify_receipt,
                    expected_node: expected_node.as_deref(),
                    expected_topic: expected_topic.as_deref(),
                    expected_endpoint: expected_endpoint.as_deref(),
                    expected_peer: expected_peer.as_deref(),
                    expected_operations: &operations,
                    expected_target_scope: target_scope.as_deref(),
                    expected_resource_scope: resource_scope.as_deref(),
                    as_of_sequence,
                    as_of_epoch,
                },
            )?;
            emit_named_receipt(
                receipt_out.as_ref(),
                "node control live workflow bundle gate receipt",
                &gated.receipt_value,
            )?;
            println!(
                "node live workflow bundle gate decision={} bundle={} verify={} recomputed-verify={} diagnostics={}",
                gated.decision,
                gated.bundle_ref,
                gated.verify_receipt_ref.as_deref().unwrap_or("none"),
                gated.recomputed_verify_receipt_ref,
                gated.diagnostics.len()
            );
            print_live_workflow_bundle_gate_next_step(&gated);
            Ok(())
        }
        NodeCommand::LiveWorkflowBundleApply {
            state_root,
            bundle,
            gate_receipt,
            require_gate_receipt,
            request,
            send,
            from_peer,
            sequence,
            operation_id,
            expected_node,
            expected_topic,
            expected_endpoint,
            expected_peer,
            operations,
            target_scope,
            resource_scope,
            as_of_sequence,
            as_of_epoch,
            peer_bootstrap_refs,
            authority_refs,
            policy_refs,
            resource_refs,
            evidence_refs,
            max_attempts,
            join_timeout_ms,
            send_receipt_out,
            receipt_out,
        } => {
            let bundle_value = read_preserves_file(&bundle)?;
            let gate_receipt_value = gate_receipt.as_ref().map(|path| read_preserves_file(path)).transpose()?;
            let request_value = request.as_ref().map(|path| read_preserves_file(path)).transpose()?;
            let runtime =
                tokio::runtime::Builder::new_multi_thread().enable_all().build().map_err(MoltenError::from)?;
            let applied = runtime.block_on(node_daemon::apply_node_control_live_workflow_bundle(
                &node_daemon::NodeControlLiveWorkflowBundleApplyInput {
                    state_root: &state_root,
                    bundle_value: &bundle_value,
                    gate_receipt_value: gate_receipt_value.as_ref(),
                    is_gate_receipt_required: require_gate_receipt,
                    request_value: request_value.as_ref(),
                    should_send: send,
                    from_peer: from_peer.as_deref(),
                    sequence,
                    expected_operation_ref: operation_id.as_deref(),
                    expected_node: expected_node.as_deref(),
                    expected_topic: expected_topic.as_deref(),
                    expected_endpoint: expected_endpoint.as_deref(),
                    expected_peer: expected_peer.as_deref(),
                    expected_operations: &operations,
                    expected_target_scope: target_scope.as_deref(),
                    expected_resource_scope: resource_scope.as_deref(),
                    as_of_sequence,
                    as_of_epoch,
                    peer_bootstrap_refs: &peer_bootstrap_refs,
                    authority_refs: &authority_refs,
                    policy_refs: &policy_refs,
                    resource_refs: &resource_refs,
                    evidence_refs: &evidence_refs,
                    max_attempts,
                    join_timeout_ms,
                },
            ))?;
            if let (Some(path), Some(value)) = (send_receipt_out.as_ref(), applied.send_receipt_value.as_ref()) {
                write_file(path, &to_text(value)?)?;
            }
            emit_named_receipt(
                receipt_out.as_ref(),
                "node control live workflow bundle apply receipt",
                &applied.receipt_value,
            )?;
            println!(
                "node live workflow bundle apply decision={} bundle={} gate={} import={} imported={} send={} diagnostics={}",
                applied.decision,
                applied.bundle_ref,
                applied.gate_receipt_ref.as_deref().unwrap_or("none"),
                applied.import_receipt_ref.as_deref().unwrap_or("none"),
                applied.imported_refs.len(),
                applied.send_receipt_ref.as_deref().unwrap_or("none"),
                applied.diagnostics.len()
            );
            print_live_workflow_bundle_apply_next_step(&applied, request_value.is_some(), send);
            Ok(())
        }
        NodeCommand::LiveWorkflowBundleReconcile {
            apply_receipt,
            send_receipt,
            ingress_receipt,
            queue_receipt,
            control_receipt,
            expected_envelope,
            expected_operation,
            expected_request,
            receipt_out,
        } => {
            let apply_receipt_value = read_preserves_file(&apply_receipt)?;
            let send_receipt_value = send_receipt.as_ref().map(|path| read_preserves_file(path)).transpose()?;
            let ingress_receipt_value = ingress_receipt.as_ref().map(|path| read_preserves_file(path)).transpose()?;
            let queue_receipt_value = queue_receipt.as_ref().map(|path| read_preserves_file(path)).transpose()?;
            let control_receipt_value = control_receipt.as_ref().map(|path| read_preserves_file(path)).transpose()?;
            let reconciled = node_daemon::reconcile_node_control_live_workflow_bundle(
                &node_daemon::NodeControlLiveWorkflowBundleReconcileInput {
                    apply_receipt_value: &apply_receipt_value,
                    send_receipt_value: send_receipt_value.as_ref(),
                    ingress_receipt_value: ingress_receipt_value.as_ref(),
                    queue_receipt_value: queue_receipt_value.as_ref(),
                    control_receipt_value: control_receipt_value.as_ref(),
                    expected_envelope_ref: expected_envelope.as_deref(),
                    expected_operation_ref: expected_operation.as_deref(),
                    expected_request_ref: expected_request.as_deref(),
                },
            )?;
            emit_named_receipt(
                receipt_out.as_ref(),
                "node control live workflow bundle reconcile receipt",
                &reconciled.receipt_value,
            )?;
            println!(
                "node live workflow bundle reconcile decision={} bundle={} apply={} ingress={} queue={} control={} diagnostics={}",
                reconciled.decision,
                reconciled.bundle_ref,
                reconciled.apply_receipt_ref,
                reconciled.ingress_receipt_ref.as_deref().unwrap_or("none"),
                reconciled.queue_receipt_ref.as_deref().unwrap_or("none"),
                reconciled.control_receipt_ref.as_deref().unwrap_or("none"),
                reconciled.diagnostics.len()
            );
            print_live_workflow_bundle_reconcile_next_step(&reconciled);
            Ok(())
        }
        NodeCommand::LiveWorkflowBundleAckExport {
            apply_receipt,
            send_receipt,
            ingress_receipt,
            queue_receipt,
            control_receipt,
            reconcile_receipt,
            out,
            receipt_out,
        } => {
            let apply_receipt_value = read_preserves_file(&apply_receipt)?;
            let send_receipt_value = send_receipt.as_ref().map(|path| read_preserves_file(path)).transpose()?;
            let ingress_receipt_value = ingress_receipt.as_ref().map(|path| read_preserves_file(path)).transpose()?;
            let queue_receipt_value = queue_receipt.as_ref().map(|path| read_preserves_file(path)).transpose()?;
            let control_receipt_value = control_receipt.as_ref().map(|path| read_preserves_file(path)).transpose()?;
            let reconcile_receipt_value = read_preserves_file(&reconcile_receipt)?;
            let exported = node_daemon::export_node_control_live_workflow_bundle_ack(
                &node_daemon::NodeControlLiveWorkflowBundleAckExportInput {
                    apply_receipt_value: &apply_receipt_value,
                    send_receipt_value: send_receipt_value.as_ref(),
                    ingress_receipt_value: ingress_receipt_value.as_ref(),
                    queue_receipt_value: queue_receipt_value.as_ref(),
                    control_receipt_value: control_receipt_value.as_ref(),
                    reconcile_receipt_value: &reconcile_receipt_value,
                },
            )?;
            write_file(&out, &to_text(&exported.ack.ack_value)?)?;
            emit_named_receipt(
                receipt_out.as_ref(),
                "node control live workflow bundle ack export receipt",
                &exported.receipt_value,
            )?;
            println!(
                "node live workflow bundle ack export decision={} ack={} bundle={} receiver_decision={} diagnostics={}",
                exported.decision,
                exported.ack.ack_ref,
                exported.ack.bundle_ref,
                exported.receiver_decision,
                exported.diagnostics.len()
            );
            print_live_workflow_bundle_ack_export_next_step(&exported);
            Ok(())
        }
        NodeCommand::LiveWorkflowBundleAckImport {
            state_root,
            ack,
            expected_bundle,
            expected_envelope,
            expected_operation,
            expected_request,
            receipt_out,
        } => {
            let ack_value = read_preserves_file(&ack)?;
            let imported = node_daemon::import_node_control_live_workflow_bundle_ack(
                &node_daemon::NodeControlLiveWorkflowBundleAckImportInput {
                    state_root: &state_root,
                    ack_value: &ack_value,
                    expected_bundle_ref: expected_bundle.as_deref(),
                    expected_envelope_ref: expected_envelope.as_deref(),
                    expected_operation_ref: expected_operation.as_deref(),
                    expected_request_ref: expected_request.as_deref(),
                },
            )?;
            emit_named_receipt(
                receipt_out.as_ref(),
                "node control live workflow bundle ack import receipt",
                &imported.receipt_value,
            )?;
            println!(
                "node live workflow bundle ack import decision={} ack={} bundle={} imported={} receiver_decision={} diagnostics={}",
                imported.decision,
                imported.ack_ref,
                imported.bundle_ref,
                imported.imported_refs.len(),
                imported.receiver_decision,
                imported.diagnostics.len()
            );
            print_live_workflow_bundle_ack_import_next_step(&imported);
            Ok(())
        }
        NodeCommand::LiveWorkflowBundleProtocolGate {
            bundle,
            gate_receipt,
            apply_receipt,
            reconcile_receipt,
            ack,
            expected_envelope,
            expected_operation,
            expected_request,
            receipt_out,
        } => {
            let bundle_value = read_preserves_file(&bundle)?;
            let gate_receipt_value = read_preserves_file(&gate_receipt)?;
            let apply_receipt_value = read_preserves_file(&apply_receipt)?;
            let reconcile_receipt_value = read_preserves_file(&reconcile_receipt)?;
            let ack_value = read_preserves_file(&ack)?;
            let gated = node_daemon::gate_node_control_live_workflow_protocol(
                &node_daemon::NodeControlLiveWorkflowProtocolGateInput {
                    bundle_value: &bundle_value,
                    gate_receipt_value: &gate_receipt_value,
                    apply_receipt_value: &apply_receipt_value,
                    reconcile_receipt_value: &reconcile_receipt_value,
                    ack_value: &ack_value,
                    expected_envelope_ref: expected_envelope.as_deref(),
                    expected_operation_ref: expected_operation.as_deref(),
                    expected_request_ref: expected_request.as_deref(),
                },
            )?;
            emit_named_receipt(
                receipt_out.as_ref(),
                "node control live workflow protocol gate receipt",
                &gated.receipt_value,
            )?;
            println!(
                "node live workflow protocol gate decision={} receipt={} protocol={} session={} operations={} messages={} diagnostics={}",
                gated.decision,
                gated.receipt_ref,
                gated.protocol_ref,
                gated.session_id,
                gated.operation_count,
                gated.message_count,
                gated.diagnostics.len()
            );
            print_live_workflow_protocol_gate_next_step(&gated);
            Ok(())
        }
        NodeCommand::LiveWorkflowBundleImport {
            state_root,
            bundle,
            expected_node,
            expected_topic,
            expected_endpoint,
            expected_peer,
            operations,
            target_scope,
            resource_scope,
            as_of_sequence,
            as_of_epoch,
            receipt_out,
        } => {
            let bundle_value = read_preserves_file(&bundle)?;
            let imported = node_daemon::import_node_control_live_workflow_bundle(
                &node_daemon::NodeControlLiveWorkflowBundleImportInput {
                    state_root: &state_root,
                    bundle_value: &bundle_value,
                    expected_node: expected_node.as_deref(),
                    expected_topic: expected_topic.as_deref(),
                    expected_endpoint: expected_endpoint.as_deref(),
                    expected_peer: expected_peer.as_deref(),
                    expected_operations: &operations,
                    expected_target_scope: target_scope.as_deref(),
                    expected_resource_scope: resource_scope.as_deref(),
                    as_of_sequence,
                    as_of_epoch,
                },
            )?;
            emit_named_receipt(
                receipt_out.as_ref(),
                "node control live workflow bundle import receipt",
                &imported.receipt_value,
            )?;
            println!(
                "node live workflow bundle import decision={} bundle={} ticket_import={} authority_import={} imported={} diagnostics={}",
                imported.decision,
                imported.bundle_ref,
                imported.ticket_import_ref.as_deref().unwrap_or("none"),
                imported.authority_import_ref.as_deref().unwrap_or("none"),
                imported.imported_refs.len(),
                imported.diagnostics.len()
            );
            Ok(())
        }
        NodeCommand::ControlIngressPublish {
            state_root,
            envelope,
            receipt_out,
        } => {
            let envelope_value = read_preserves_file(&envelope)?;
            let published = node_daemon::publish_node_control_ingress(&node_daemon::NodeControlIngressPublishInput {
                state_root: &state_root,
                envelope_value: &envelope_value,
            })?;
            emit_named_receipt(receipt_out.as_ref(), "node control ingress receipt", &published.receipt_value)?;
            println!(
                "node control ingress publish envelope={} receipt={} path={}",
                published.envelope_ref,
                published.receipt_ref,
                published.envelope_path.display()
            );
            Ok(())
        }
        NodeCommand::ControlIngressDeliver {
            state_root,
            topic,
            envelope_ref,
            receipt_out,
        } => {
            let delivered = node_daemon::deliver_node_control_ingress(&node_daemon::NodeControlIngressDeliverInput {
                state_root: &state_root,
                topic: &topic,
                envelope_ref: &envelope_ref,
            })?;
            emit_named_receipt(receipt_out.as_ref(), "node control ingress receipt", &delivered.ingress_receipt_value)?;
            println!(
                "node control ingress deliver envelope={} request={} receipt={} enqueued={}",
                delivered.envelope_ref,
                delivered.request_ref,
                delivered.ingress_receipt_ref,
                if delivered.has_enqueued { "yes" } else { "no" }
            );
            Ok(())
        }
        NodeCommand::ControlDeny {
            request,
            startup,
            diagnostic,
            receipt_out,
        } => {
            let request_value = read_preserves_file(&request)?;
            let request = node_runtime::parse_node_control_request(&request_value)?;
            let receipt = node_runtime::node_control_deny_receipt_value(&request, &startup, &diagnostic)?;
            emit_named_receipt(receipt_out.as_ref(), "node control receipt", &receipt)?;
            println!("node control deny receipt={}", canonical_hash(&receipt)?);
            Ok(())
        }
        NodeCommand::Shutdown {
            startup,
            adapters,
            drained_jobs,
            index_receipt_refs,
            receipt_out,
        } => {
            let adapter_receipts = parse_node_adapter_receipt_args(&adapters)?;
            let receipt = node_runtime::node_shutdown_receipt_value(&node_runtime::ShutdownReceiptValueInput {
                decision: "pass",
                startup_receipt_ref: &startup,
                adapter_receipts: &adapter_receipts,
                drained_job_refs: &drained_jobs,
                index_receipt_refs: &index_receipt_refs,
                diagnostics: &[],
            })?;
            emit_named_receipt(receipt_out.as_ref(), "node shutdown receipt", &receipt)?;
            println!("node shutdown receipt={}", canonical_hash(&receipt)?);
            Ok(())
        }
        NodeCommand::Health {
            startup_receipt,
            shutdown,
            index_receipt_refs,
            head_refs,
            open_job_refs,
            receipt_out,
        } => {
            let startup_value = read_preserves_file(&startup_receipt)?;
            let startup = node_runtime::parse_node_startup_receipt(&startup_value)?;
            let receipt =
                node_runtime::node_restart_health_receipt_value(&node_runtime::RestartHealthReceiptValueInput {
                    startup_receipt: &startup,
                    shutdown_receipt_ref: shutdown.as_deref(),
                    index_receipt_refs: &index_receipt_refs,
                    head_refs: &head_refs,
                    open_job_refs: &open_job_refs,
                    diagnostics: &[],
                })?;
            emit_named_receipt(receipt_out.as_ref(), "node health receipt", &receipt)?;
            println!("node health receipt={}", canonical_hash(&receipt)?);
            Ok(())
        }
    }
}

fn print_live_workflow_bundle_ack_export_next_step(exported: &node_daemon::NodeControlLiveWorkflowBundleAckExport) {
    if exported.decision != "pass" {
        println!(
            "next-step=collect-receiver-evidence command=\"molten node live-workflow-bundle-reconcile ... --ingress-receipt <receipt> --queue-receipt <receipt>\""
        );
        return;
    }
    println!(
        "next-step=import-ack command=\"molten node live-workflow-bundle-ack-import --state-root <sender> <ack>\""
    );
}

fn print_live_workflow_bundle_ack_import_next_step(imported: &node_daemon::NodeControlLiveWorkflowBundleAckImport) {
    if imported.decision != "pass" {
        println!("next-step=inspect-ack-diagnostics command=\"molten node show <ack-import-receipt>\"");
        return;
    }
    if imported.receiver_decision == "pass" {
        println!("next-step=inspect-receiver-control command=\"molten node show <control-receipt>\"");
    } else {
        println!("next-step=inspect-receiver-denial command=\"molten node show <reconcile-receipt>\"");
    }
}

fn print_live_workflow_protocol_gate_next_step(gated: &node_daemon::NodeControlLiveWorkflowProtocolGate) {
    if gated.decision == "pass" {
        println!("next-step=archive-workflow-protocol command=\"molten node show <protocol-gate-receipt>\"");
    } else {
        println!(
            "next-step=inspect-workflow-protocol-diagnostics command=\"molten node show <protocol-gate-receipt>\""
        );
    }
}

fn print_live_workflow_bundle_reconcile_next_step(reconciled: &node_daemon::NodeControlLiveWorkflowBundleReconcile) {
    if reconciled.decision == "pass" {
        if reconciled.control_receipt_ref.is_some() {
            println!("next-step=inspect-control-receipt command=\"molten node show <control-receipt>\"");
        } else {
            println!("next-step=run-receiver-control-loop command=\"molten node run-loop --state-root <receiver>\"");
        }
        return;
    }
    let has_missing_ingress = reconciled
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.contains("requires receiver ingress receipt"));
    if has_missing_ingress {
        println!(
            "next-step=wait-or-import-receiver-ingress command=\"molten node live-workflow-bundle-reconcile ... --ingress-receipt <receipt>\""
        );
        return;
    }
    let has_control_denial =
        reconciled.diagnostics.iter().any(|diagnostic| diagnostic.contains("receiver control receipt"));
    if has_control_denial {
        println!("next-step=inspect-receiver-denial command=\"molten node show <control-receipt>\"");
        return;
    }
    println!("next-step=inspect-reconcile-diagnostics command=\"molten node show <reconcile-receipt>\"");
}

fn print_live_workflow_bundle_apply_next_step(
    applied: &node_daemon::NodeControlLiveWorkflowBundleApply,
    has_request: bool,
    was_send_requested: bool,
) {
    if applied.decision == "pass" {
        if was_send_requested {
            println!("next-step=inspect-live-send-receipt command=\"molten node show <send-receipt>\"");
        } else if has_request {
            println!(
                "next-step=send-live-workflow-bundle command=\"molten node live-workflow-bundle-apply ... --send\""
            );
        } else {
            println!(
                "next-step=dry-run-or-send-request command=\"molten node live-workflow-bundle-apply ... --request <request> [--send]\""
            );
        }
        return;
    }
    let has_gate_problem = applied.diagnostics.iter().any(|diagnostic| {
        diagnostic.contains("gate receipt")
            || diagnostic.contains("requires a current gate receipt")
            || diagnostic.contains("recomputed verify")
    });
    if has_gate_problem {
        println!("next-step=rerun-gate command=\"molten node live-workflow-bundle-gate ... --receipt-out ...\"");
        return;
    }
    let has_address_problem = applied
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.contains("no endpoint addresses") || diagnostic.contains("unsupported address"));
    if has_address_problem {
        println!("next-step=refresh-bound-live-ticket command=\"molten node serve --live-iroh --live-ticket-out ...\"");
        return;
    }
    println!("next-step=inspect-apply-diagnostics command=\"molten node show <apply-receipt>\"");
}

fn print_live_workflow_bundle_gate_next_step(gated: &node_daemon::NodeControlLiveWorkflowBundleGate) {
    if gated.decision == "pass" {
        println!("next-step=import-bundle command=\"molten node live-workflow-bundle-import ...\"");
        return;
    }
    let has_malformed_bundle = gated.diagnostics.iter().any(|diagnostic| {
        diagnostic.contains("bundle parse failed") || diagnostic.contains("unsupported receipt kind")
    });
    if has_malformed_bundle {
        println!("next-step=fix-malformed-bundle rerun=\"molten node live-workflow-bundle-verify ...\"");
        return;
    }
    let has_verify_receipt_problem = gated.diagnostics.iter().any(|diagnostic| {
        diagnostic.contains("verify receipt")
            || diagnostic.contains("requires a current verify receipt")
            || diagnostic.contains("does not match recomputed")
    });
    if has_verify_receipt_problem {
        println!(
            "next-step=rerun-verify-receipt command=\"molten node live-workflow-bundle-verify ... --receipt-out ...\""
        );
        return;
    }
    println!(
        "next-step=import-missing-ticket-or-grant command=\"molten node live-ticket-import ...; molten node authority-grant-import ...\""
    );
}

fn parse_node_adapter_receipt_args(args: &[String]) -> Result<Vec<node_runtime::NodeAdapterReceiptRef>> {
    args.iter()
        .map(|arg| {
            let (name, receipt_ref) = arg.split_once('=').ok_or_else(|| {
                MoltenError::invalid_harness(format!("node adapter receipt arg `{arg}` must be name=blake3:ref"))
            })?;
            Ok(node_runtime::NodeAdapterReceiptRef {
                name: name.to_string(),
                receipt_ref: receipt_ref.to_string(),
            })
        })
        .collect()
}

fn read_preserves_file(path: &Path) -> Result<preserves::IOValue> {
    let text = fs::read_to_string(path).map_err(MoltenError::from)?;
    parse_text(&text)
}

fn emit_named_receipt(path: Option<&PathBuf>, label: &str, receipt: &preserves::IOValue) -> Result<()> {
    let receipt_text = to_text(receipt)?;
    let receipt_ref = canonical_hash(receipt)?;
    if let Some(path) = path {
        write_file(path, &receipt_text)?;
        println!("{label} {receipt_ref} written to {}", path.display());
    } else {
        println!("{receipt_text}");
        eprintln!("{label} {receipt_ref}");
    }
    Ok(())
}

fn write_file(path: &Path, contents: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(MoltenError::from)?;
    }
    fs::write(path, contents).map_err(MoltenError::from)
}

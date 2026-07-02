type IoValue = preserves::IOValue;
type Ipv4Addr = std::net::Ipv4Addr;
type MoltenError = crate::error::MoltenError;
type Path = std::path::Path;
type PathBuf = std::path::PathBuf;
type Result<T> = crate::error::Result<T>;
type SocketAddr = std::net::SocketAddr;

#[cfg(test)]
type Counter = std::sync::atomic::AtomicU64;

#[cfg(test)]
const RELAXED: std::sync::atomic::Ordering = std::sync::atomic::Ordering::Relaxed;

mod fs {
    pub(super) fn create_dir_all(path: impl AsRef<std::path::Path>) -> std::io::Result<()> {
        std::fs::create_dir_all(path)
    }

    pub(super) fn read_dir(path: impl AsRef<std::path::Path>) -> std::io::Result<std::fs::ReadDir> {
        std::fs::read_dir(path)
    }

    pub(super) fn read_to_string(path: impl AsRef<std::path::Path>) -> std::io::Result<String> {
        std::fs::read_to_string(path)
    }

    pub(super) fn remove_file(path: impl AsRef<std::path::Path>) -> std::io::Result<()> {
        std::fs::remove_file(path)
    }

    pub(super) fn write(path: impl AsRef<std::path::Path>, contents: impl AsRef<[u8]>) -> std::io::Result<()> {
        std::fs::write(path, contents)
    }

    #[cfg(test)]
    pub(super) fn remove_dir_all(path: impl AsRef<std::path::Path>) -> std::io::Result<()> {
        std::fs::remove_dir_all(path)
    }
}

use n0_future::StreamExt;

use crate::bounded::VecSink;

const CONFIG_FILE: &str = "config.preserves";
const STARTUP_FILE: &str = "startup-receipt.preserves";
const HEALTH_FILE: &str = "health-receipt.preserves";
const SHUTDOWN_FILE: &str = "shutdown-receipt.preserves";
const CONTROL_STATUS_FILE: &str = "status-control-receipt.preserves";
const CONTROL_STOP_FILE: &str = "stop-control-receipt.preserves";
const CONTROL_INBOX_DIR: &str = "control/inbox";
const CONTROL_OUTBOX_DIR: &str = "control/outbox";
const CONTROL_INGRESS_DIR: &str = "control/iroh-ingress";
const CONTROL_IDEMPOTENCY_DIR: &str = "control/idempotency";
const CONTROL_SERVICE_DIR: &str = "control/service";
pub const DEFAULT_CONTROL_INGRESS_TOPIC: &str = "node-control";
pub const LOCAL_CONTROL_INGRESS_TRANSPORT: &str = "iroh-local-gossip";
pub const LIVE_CONTROL_INGRESS_TRANSPORT: &str = "iroh-gossip";
const CONTROL_LOCK_FILE: &str = "control/node.lock.preserves";
const CONTROL_SERVICE_LOCK_FILE: &str = "control/service/service.lock.preserves";
const IDENTITY_RECEIPT_FILE: &str = "identity-receipt.preserves";
const IDENTITY_FILE: &str = "identity.preserves";
const MAX_PENDING_CONTROL_REQUESTS: usize = 1024;
const MAX_CONTROL_LOOP_REQUESTS: u64 = 1024;
pub const DEFAULT_CONTROL_LOOP_REQUESTS: u64 = 64;
const MAX_CONTROL_SERVICE_TICKS: u64 = 4096;
const MAX_CONTROL_LIVE_LISTENER_EVENTS: u64 = 4096;
const MAX_CONTROL_LIVE_SEND_ATTEMPTS: u64 = 5;
const MAX_CONTROL_LIVE_SEND_TIMEOUT_MS: u64 = 60_000;
pub const DEFAULT_CONTROL_LIVE_SEND_ATTEMPTS: u64 = 1;
pub const DEFAULT_CONTROL_SERVICE_TICKS: u64 = 1;
pub const DEFAULT_CONTROL_LIVE_LISTENER_EVENTS: u64 = 1;
pub const DEFAULT_CONTROL_LIVE_LISTENER_TIMEOUT_MS: u64 = 250;
const LIVE_WORKFLOW_PROTOCOL_ID: &str = "proto:molten.node-control.live-workflow-bundle.v1";
const LIVE_WORKFLOW_PROTOCOL_SESSION_PREFIX: &str = "session:node-control-live-workflow:";

const _: () = assert!(MAX_PENDING_CONTROL_REQUESTS > 0);
const _: () = assert!(MAX_CONTROL_LOOP_REQUESTS > 0);
const _: () = assert!(DEFAULT_CONTROL_LOOP_REQUESTS > 0);
const _: () = assert!(DEFAULT_CONTROL_LOOP_REQUESTS <= MAX_CONTROL_LOOP_REQUESTS);
const _: () = assert!(MAX_CONTROL_SERVICE_TICKS > 0);
const _: () = assert!(DEFAULT_CONTROL_SERVICE_TICKS > 0);
const _: () = assert!(DEFAULT_CONTROL_SERVICE_TICKS <= MAX_CONTROL_SERVICE_TICKS);
const _: () = assert!(MAX_CONTROL_LIVE_LISTENER_EVENTS > 0);
const _: () = assert!(MAX_CONTROL_LIVE_SEND_ATTEMPTS > 0);
const _: () = assert!(MAX_CONTROL_LIVE_SEND_TIMEOUT_MS > 0);
const _: () = assert!(DEFAULT_CONTROL_LIVE_SEND_ATTEMPTS > 0);
const _: () = assert!(DEFAULT_CONTROL_LIVE_SEND_ATTEMPTS <= MAX_CONTROL_LIVE_SEND_ATTEMPTS);
const _: () = assert!(DEFAULT_CONTROL_LIVE_LISTENER_EVENTS <= MAX_CONTROL_LIVE_LISTENER_EVENTS);
const _: () = assert!(DEFAULT_CONTROL_LIVE_LISTENER_TIMEOUT_MS > 0);

#[derive(Debug, Clone, Copy)]
pub struct InitInput<'a> {
    pub state_root: &'a Path,
    pub node_id: &'a str,
}

#[derive(Debug, Clone, Copy)]
pub struct RunInput<'a> {
    pub state_root: &'a Path,
}

#[derive(Debug, Clone, Copy)]
pub struct StatusInput<'a> {
    pub state_root: &'a Path,
}

#[derive(Debug, Clone, Copy)]
pub struct StopInput<'a> {
    pub state_root: &'a Path,
}

#[derive(Debug, Clone, Copy)]
pub struct ControlSubmitInput<'a> {
    pub state_root: &'a Path,
    pub request_value: &'a IoValue,
}

#[derive(Debug, Clone, Copy)]
pub struct ControlDispatchInput<'a> {
    pub state_root: &'a Path,
    pub request_path: Option<&'a Path>,
}

#[derive(Debug, Clone, Copy)]
pub struct ControlLoopInput<'a> {
    pub state_root: &'a Path,
    pub max_requests: u64,
}

#[derive(Debug, Clone, Copy)]
pub struct ControlServeInput<'a> {
    pub state_root: &'a Path,
    pub topic: &'a str,
    pub max_ticks: u64,
    pub max_requests_per_tick: u64,
    pub supervisor_policy_value: Option<&'a IoValue>,
}

#[derive(Debug, Clone, Copy)]
pub struct ControlSupervisorPolicyInput<'a> {
    pub max_restarts: u64,
    pub restart_window_ticks: u64,
    pub heartbeat_timeout_ticks: u64,
    pub shutdown_drain_ticks: u64,
    pub stale_lock_recovery: bool,
    pub policy_refs: &'a [String],
    pub evidence_refs: &'a [String],
}

#[derive(Debug, Clone, Copy)]
struct SupervisorReceiptValueInput<'a> {
    decision: &'a str,
    operation: &'a str,
    startup_receipt_ref: &'a str,
    service_lock_ref: Option<&'a str>,
    supervisor_policy_ref: Option<&'a str>,
    topic: &'a str,
    diagnostics: &'a [String],
}

#[derive(Debug, Clone, Copy)]
struct ServiceLockValueInput<'a> {
    state_root: &'a Path,
    startup_receipt_ref: &'a str,
    node_id: &'a str,
    topic: &'a str,
    max_ticks: u64,
    max_requests_per_tick: u64,
    service_run_ref: &'a str,
}

#[derive(Debug, Clone, Copy)]
struct ServiceHeartbeatValueInput<'a> {
    startup_receipt_ref: &'a str,
    service_lock_ref: &'a str,
    tick: u64,
    delivered_count: u64,
    processed_count: u64,
    diagnostics: &'a [String],
}

#[derive(Debug, Clone, Copy)]
struct ServiceRunReceiptValueInput<'a> {
    decision: &'a str,
    startup_receipt_ref: &'a str,
    service_lock_ref: Option<&'a str>,
    topic: &'a str,
    max_ticks: u64,
    max_requests_per_tick: u64,
    ticks: u64,
    heartbeat_receipt_refs: &'a [String],
    ingress_receipt_refs: &'a [String],
    loop_receipt_refs: &'a [String],
    processed_request_refs: &'a [String],
    has_stopped: bool,
    supervisor_policy_ref: Option<&'a str>,
    supervisor_receipt_refs: &'a [String],
    diagnostics: &'a [String],
}

#[derive(Debug, Clone, Copy)]
pub struct ControlIngressEnvelopeInput<'a> {
    pub request_value: &'a IoValue,
    pub from_peer: &'a str,
    pub to_node: &'a str,
    pub topic: &'a str,
    pub sequence: u64,
    pub peer_bootstrap_refs: &'a [String],
    pub authority_refs: &'a [String],
    pub policy_refs: &'a [String],
    pub resource_refs: &'a [String],
    pub evidence_refs: &'a [String],
}

#[derive(Debug, Clone, Copy)]
pub struct ControlIngressPublishInput<'a> {
    pub state_root: &'a Path,
    pub envelope_value: &'a IoValue,
}

#[derive(Debug, Clone, Copy)]
pub struct ControlIngressDeliverInput<'a> {
    pub state_root: &'a Path,
    pub topic: &'a str,
    pub envelope_ref: &'a str,
}

#[derive(Debug, Clone, Copy)]
pub struct ControlLiveIngressPublishInput<'a> {
    pub sender: &'a iroh_gossip::api::GossipSender,
    pub envelope_value: &'a IoValue,
    pub node_id: &'a str,
}

#[derive(Debug, Clone, Copy)]
pub struct ControlLiveIngressReceiveBytesInput<'a> {
    pub state_root: &'a Path,
    pub topic: &'a str,
    pub receiver_node: &'a str,
    pub delivered_from: &'a str,
    pub bytes: &'a [u8],
}

#[derive(Debug, Clone, Copy)]
pub struct ControlLiveLoopbackInput<'a> {
    pub state_root: &'a Path,
    pub request_value: &'a IoValue,
    pub from_peer: &'a str,
    pub to_node: &'a str,
    pub topic: &'a str,
    pub sequence: u64,
    pub peer_bootstrap_refs: &'a [String],
    pub authority_refs: &'a [String],
    pub policy_refs: &'a [String],
    pub resource_refs: &'a [String],
    pub evidence_refs: &'a [String],
}

#[derive(Debug, Clone, Copy)]
pub struct ControlLiveServeInput<'a> {
    pub state_root: &'a Path,
    pub topic: &'a str,
    pub max_events: u64,
    pub event_timeout_ms: u64,
    pub max_requests_per_tick: u64,
    pub supervisor_policy_value: Option<&'a IoValue>,
}

#[derive(Debug, Clone, Copy)]
pub struct ControlLiveServeLoopbackInput<'a> {
    pub state_root: &'a Path,
    pub request_value: &'a IoValue,
    pub from_peer: &'a str,
    pub to_node: &'a str,
    pub topic: &'a str,
    pub sequence: u64,
    pub peer_bootstrap_refs: &'a [String],
    pub authority_refs: &'a [String],
    pub policy_refs: &'a [String],
    pub resource_refs: &'a [String],
    pub evidence_refs: &'a [String],
    pub max_requests_per_tick: u64,
}

#[derive(Debug, Clone, Copy)]
pub struct ControlLiveSendInput<'a> {
    pub state_root: Option<&'a Path>,
    pub request_value: &'a IoValue,
    pub receiver_ticket_value: &'a IoValue,
    pub from_peer: &'a str,
    pub sequence: u64,
    pub expected_operation_ref: Option<&'a str>,
    pub expected_receiver_node: Option<&'a str>,
    pub expected_topic: Option<&'a str>,
    pub expected_endpoint: Option<&'a str>,
    pub max_attempts: u64,
    pub peer_bootstrap_refs: &'a [String],
    pub authority_refs: &'a [String],
    pub policy_refs: &'a [String],
    pub resource_refs: &'a [String],
    pub evidence_refs: &'a [String],
    pub join_timeout_ms: u64,
}

#[derive(Debug, Clone, Copy)]
pub struct ControlLiveWorkflowInput<'a> {
    pub state_root: Option<&'a Path>,
    pub receiver_ticket_value: &'a IoValue,
    pub peer_admission_value: &'a IoValue,
    pub authority_grant_value: &'a IoValue,
    pub send_receipt_value: &'a IoValue,
    pub receive_receipt_values: &'a [&'a IoValue],
    pub listener_receipt_value: Option<&'a IoValue>,
    pub service_receipt_value: &'a IoValue,
}

#[derive(Debug, Clone, Copy)]
pub struct ControlLiveWorkflowBundleExportInput<'a> {
    pub receiver_ticket_value: &'a IoValue,
    pub peer_admission_value: &'a IoValue,
    pub authority_grant_value: &'a IoValue,
    pub receipt_values: &'a [&'a IoValue],
}

#[derive(Debug, Clone, Copy)]
pub struct ControlLiveWorkflowBundleVerifyInput<'a> {
    pub bundle_value: &'a IoValue,
    pub expected_node: Option<&'a str>,
    pub expected_topic: Option<&'a str>,
    pub expected_endpoint: Option<&'a str>,
    pub expected_peer: Option<&'a str>,
    pub expected_operations: &'a [String],
    pub expected_target_scope: Option<&'a str>,
    pub expected_resource_scope: Option<&'a str>,
    pub as_of_sequence: u64,
    pub as_of_epoch: u64,
}

#[derive(Debug, Clone, Copy)]
pub struct ControlLiveWorkflowBundleGateInput<'a> {
    pub bundle_value: &'a IoValue,
    pub verify_receipt_value: Option<&'a IoValue>,
    pub require_verify_receipt: bool,
    pub expected_node: Option<&'a str>,
    pub expected_topic: Option<&'a str>,
    pub expected_endpoint: Option<&'a str>,
    pub expected_peer: Option<&'a str>,
    pub expected_operations: &'a [String],
    pub expected_target_scope: Option<&'a str>,
    pub expected_resource_scope: Option<&'a str>,
    pub as_of_sequence: u64,
    pub as_of_epoch: u64,
}

#[derive(Debug, Clone, Copy)]
pub struct ControlLiveWorkflowBundleApplyInput<'a> {
    pub state_root: &'a Path,
    pub bundle_value: &'a IoValue,
    pub gate_receipt_value: Option<&'a IoValue>,
    pub is_gate_receipt_required: bool,
    pub request_value: Option<&'a IoValue>,
    pub should_send: bool,
    pub from_peer: Option<&'a str>,
    pub sequence: u64,
    pub expected_operation_ref: Option<&'a str>,
    pub expected_node: Option<&'a str>,
    pub expected_topic: Option<&'a str>,
    pub expected_endpoint: Option<&'a str>,
    pub expected_peer: Option<&'a str>,
    pub expected_operations: &'a [String],
    pub expected_target_scope: Option<&'a str>,
    pub expected_resource_scope: Option<&'a str>,
    pub as_of_sequence: u64,
    pub as_of_epoch: u64,
    pub peer_bootstrap_refs: &'a [String],
    pub authority_refs: &'a [String],
    pub policy_refs: &'a [String],
    pub resource_refs: &'a [String],
    pub evidence_refs: &'a [String],
    pub max_attempts: u64,
    pub join_timeout_ms: u64,
}

#[derive(Debug, Clone, Copy)]
pub struct ControlLiveWorkflowBundleReconcileInput<'a> {
    pub apply_receipt_value: &'a IoValue,
    pub send_receipt_value: Option<&'a IoValue>,
    pub ingress_receipt_value: Option<&'a IoValue>,
    pub queue_receipt_value: Option<&'a IoValue>,
    pub control_receipt_value: Option<&'a IoValue>,
    pub expected_envelope_ref: Option<&'a str>,
    pub expected_operation_ref: Option<&'a str>,
    pub expected_request_ref: Option<&'a str>,
}

#[derive(Debug, Clone, Copy)]
pub struct ControlLiveWorkflowBundleAckExportInput<'a> {
    pub apply_receipt_value: &'a IoValue,
    pub send_receipt_value: Option<&'a IoValue>,
    pub ingress_receipt_value: Option<&'a IoValue>,
    pub queue_receipt_value: Option<&'a IoValue>,
    pub control_receipt_value: Option<&'a IoValue>,
    pub reconcile_receipt_value: &'a IoValue,
}

#[derive(Debug, Clone, Copy)]
pub struct ControlLiveWorkflowBundleAckImportInput<'a> {
    pub state_root: &'a Path,
    pub ack_value: &'a IoValue,
    pub expected_bundle_ref: Option<&'a str>,
    pub expected_envelope_ref: Option<&'a str>,
    pub expected_operation_ref: Option<&'a str>,
    pub expected_request_ref: Option<&'a str>,
}

#[derive(Debug, Clone, Copy)]
pub struct ControlLiveWorkflowProtocolGateInput<'a> {
    pub bundle_value: &'a IoValue,
    pub gate_receipt_value: &'a IoValue,
    pub apply_receipt_value: &'a IoValue,
    pub reconcile_receipt_value: &'a IoValue,
    pub ack_value: &'a IoValue,
    pub expected_envelope_ref: Option<&'a str>,
    pub expected_operation_ref: Option<&'a str>,
    pub expected_request_ref: Option<&'a str>,
}

#[derive(Debug, Clone, Copy)]
pub struct ControlLiveWorkflowBundleImportInput<'a> {
    pub state_root: &'a Path,
    pub bundle_value: &'a IoValue,
    pub expected_node: Option<&'a str>,
    pub expected_topic: Option<&'a str>,
    pub expected_endpoint: Option<&'a str>,
    pub expected_peer: Option<&'a str>,
    pub expected_operations: &'a [String],
    pub expected_target_scope: Option<&'a str>,
    pub expected_resource_scope: Option<&'a str>,
    pub as_of_sequence: u64,
    pub as_of_epoch: u64,
}

#[derive(Debug, Clone, Copy)]
pub struct ControlAuthorityGrantInput<'a> {
    pub peer_id: &'a str,
    pub node_id: &'a str,
    pub operations: &'a [String],
    pub target_scope: &'a str,
    pub resource_scope: &'a str,
    pub epoch: u64,
    pub expires_at: Option<u64>,
    pub policy_refs: &'a [String],
    pub revocation_refs: &'a [String],
    pub evidence_refs: &'a [String],
}

#[derive(Debug, Clone, Copy)]
pub struct ControlLiveTicketInput<'a> {
    pub node_id: &'a str,
    pub node_identity_ref: &'a str,
    pub logical_endpoint_id: &'a str,
    pub live_endpoint_id: &'a str,
    pub topic: &'a str,
    pub address_refs: &'a [String],
    pub policy_refs: &'a [String],
    pub evidence_refs: &'a [String],
}

#[derive(Debug, Clone, Copy)]
pub struct ControlLiveTicketExportInput<'a> {
    pub state_root: &'a Path,
    pub topic: &'a str,
    pub policy_refs: &'a [String],
    pub evidence_refs: &'a [String],
}

#[derive(Debug, Clone, Copy)]
pub struct ControlLiveTicketImportInput<'a> {
    pub state_root: &'a Path,
    pub ticket_value: &'a IoValue,
    pub peer_admission_value: Option<&'a IoValue>,
    pub expected_node: Option<&'a str>,
    pub expected_topic: Option<&'a str>,
    pub expected_endpoint: Option<&'a str>,
    pub expected_peer: Option<&'a str>,
    pub as_of_sequence: u64,
}

#[derive(Debug, Clone, Copy)]
pub struct ControlAuthorityGrantImportInput<'a> {
    pub state_root: &'a Path,
    pub grant_value: &'a IoValue,
    pub expected_peer: Option<&'a str>,
    pub expected_node: Option<&'a str>,
    pub expected_operations: &'a [String],
    pub expected_target_scope: Option<&'a str>,
    pub expected_resource_scope: Option<&'a str>,
    pub as_of_epoch: u64,
}

#[derive(Debug, Clone, Copy)]
pub struct ControlLivePeerAdmitInput<'a> {
    pub state_root: &'a Path,
    pub ticket_value: &'a IoValue,
    pub peer_id: &'a str,
    pub sequence: u64,
    pub expires_at: Option<u64>,
    pub policy_refs: &'a [String],
    pub evidence_refs: &'a [String],
}

#[derive(Debug, Clone, Copy)]
struct AuthorityReceiptValueInput<'a> {
    decision: &'a str,
    envelope: &'a ControlIngressEnvelope,
    grant_ref: Option<&'a str>,
    diagnostics: &'a [String],
}

#[derive(Debug, Clone, Copy)]
struct ListenerReceiptValueInput<'a> {
    decision: &'a str,
    startup_receipt_ref: &'a str,
    node_id: &'a str,
    logical_endpoint_id: &'a str,
    bound_endpoint_id: &'a str,
    topic: &'a str,
    max_events: u64,
    observed_events: u64,
    transport_receipt_refs: &'a [String],
    neighbor_events: &'a [String],
    service_receipt_ref: &'a str,
    diagnostics: &'a [String],
}

#[derive(Debug, Clone, Copy)]
struct LiveTransportReceiptValueInput<'a> {
    operation: &'a str,
    decision: &'a str,
    node_id: &'a str,
    delivered_from: Option<&'a str>,
    envelope: &'a ControlIngressEnvelope,
    ingress_receipt_ref: Option<&'a str>,
    diagnostics: &'a [String],
}

#[derive(Debug, Clone, Copy)]
struct LiveSendReceiptValueInput<'a> {
    decision: &'a str,
    from_peer: &'a str,
    ticket: &'a ControlLiveTicket,
    envelope: &'a ControlIngressEnvelope,
    transport_receipt_ref: Option<&'a str>,
    diagnostics: &'a [String],
}

#[derive(Debug, Clone, Copy)]
struct LiveSendRetryReceiptValueInput<'a> {
    decision: &'a str,
    attempt: u64,
    max_attempts: u64,
    from_peer: &'a str,
    ticket: &'a ControlLiveTicket,
    envelope: &'a ControlIngressEnvelope,
    diagnostics: &'a [String],
}

#[derive(Debug, Clone, Copy)]
struct LiveSendDuplicateReceiptValueInput<'a> {
    from_peer: &'a str,
    ticket: &'a ControlLiveTicket,
    envelope: &'a ControlIngressEnvelope,
    prior_send_receipt_ref: &'a str,
    diagnostics: &'a [String],
}

#[derive(Debug, Clone, Copy)]
struct LivePeerAdmissionValueInput<'a> {
    decision: &'a str,
    peer_id: &'a str,
    ticket: &'a ControlLiveTicket,
    admission_sequence: u64,
    expires_at: Option<u64>,
    policy_refs: &'a [String],
    evidence_refs: &'a [String],
    diagnostics: &'a [String],
}

#[derive(Debug, Clone, Copy)]
struct LiveTicketImportReceiptValueInput<'a> {
    decision: &'a str,
    state_root: &'a Path,
    ticket: &'a ControlLiveTicket,
    peer_admission_ref: Option<&'a str>,
    peer_id: Option<&'a str>,
    as_of_sequence: u64,
    imported_refs: &'a [String],
    diagnostics: &'a [String],
}

#[derive(Debug, Clone, Copy)]
struct AuthorityGrantImportReceiptValueInput<'a> {
    decision: &'a str,
    state_root: &'a Path,
    grant: &'a ControlAuthorityGrant,
    as_of_epoch: u64,
    imported_refs: &'a [String],
    diagnostics: &'a [String],
}

#[derive(Debug, Clone, Copy)]
struct LiveWorkflowBundleValueInput<'a> {
    ticket: &'a ControlLiveTicket,
    admission: &'a ControlLivePeerAdmission,
    authority: &'a ControlAuthorityGrant,
    ticket_value: &'a IoValue,
    admission_value: &'a IoValue,
    authority_value: &'a IoValue,
    receipt_values: &'a [&'a IoValue],
    diagnostics: &'a [String],
}

#[derive(Debug, Clone, Copy)]
struct LiveWorkflowBundleExportReceiptValueInput<'a> {
    decision: &'a str,
    bundle: &'a ControlLiveWorkflowBundle,
    diagnostics: &'a [String],
}

#[derive(Debug, Clone, Copy)]
struct LiveWorkflowBundleVerifyReceiptValueInput<'a> {
    decision: &'a str,
    bundle_ref: &'a str,
    ticket_ref: Option<&'a str>,
    peer_admission_ref: Option<&'a str>,
    authority_grant_ref: Option<&'a str>,
    receipt_refs: &'a [String],
    expected: &'a LiveWorkflowBundleExpectedInput<'a>,
    diagnostics: &'a [String],
}

#[derive(Debug, Clone, Copy)]
struct LiveWorkflowBundleGateReceiptValueInput<'a> {
    decision: &'a str,
    bundle_ref: &'a str,
    verify_receipt_ref: Option<&'a str>,
    recomputed_verify_receipt_ref: &'a str,
    ticket_ref: Option<&'a str>,
    peer_admission_ref: Option<&'a str>,
    authority_grant_ref: Option<&'a str>,
    receipt_refs: &'a [String],
    expected: &'a LiveWorkflowBundleExpectedInput<'a>,
    diagnostics: &'a [String],
}

#[derive(Debug, Clone, Copy)]
struct LiveWorkflowBundleApplyReceiptValueInput<'a> {
    decision: &'a str,
    state_root: &'a Path,
    bundle_ref: &'a str,
    gate_receipt_ref: Option<&'a str>,
    recomputed_verify_receipt_ref: &'a str,
    import_receipt_ref: Option<&'a str>,
    imported_refs: &'a [String],
    mode: &'a str,
    envelope_ref: Option<&'a str>,
    operation_ref: Option<&'a str>,
    send_receipt_ref: Option<&'a str>,
    expected: &'a LiveWorkflowBundleExpectedInput<'a>,
    diagnostics: &'a [String],
}

#[derive(Debug, Clone, Copy)]
struct LiveWorkflowBundleReconcileReceiptValueInput<'a> {
    decision: &'a str,
    apply_receipt_ref: &'a str,
    bundle_ref: &'a str,
    send_receipt_ref: Option<&'a str>,
    ingress_receipt_ref: Option<&'a str>,
    queue_receipt_ref: Option<&'a str>,
    control_receipt_ref: Option<&'a str>,
    envelope_ref: Option<&'a str>,
    operation_ref: Option<&'a str>,
    request_ref: Option<&'a str>,
    diagnostics: &'a [String],
}

#[derive(Debug, Clone, Copy)]
struct LiveWorkflowBundleAckValueInput<'a> {
    apply_receipt_value: &'a IoValue,
    send_receipt_value: Option<&'a IoValue>,
    ingress_receipt_value: Option<&'a IoValue>,
    queue_receipt_value: Option<&'a IoValue>,
    control_receipt_value: Option<&'a IoValue>,
    reconcile_receipt_value: &'a IoValue,
    apply_receipt_ref: &'a str,
    send_receipt_ref: Option<&'a str>,
    ingress_receipt_ref: Option<&'a str>,
    queue_receipt_ref: Option<&'a str>,
    control_receipt_ref: Option<&'a str>,
    reconcile_receipt_ref: &'a str,
    bundle_ref: &'a str,
    envelope_ref: Option<&'a str>,
    operation_ref: Option<&'a str>,
    request_ref: Option<&'a str>,
    receiver_decision: &'a str,
    receiver_diagnostics: &'a [String],
    diagnostics: &'a [String],
}

#[derive(Debug, Clone, Copy)]
struct LiveWorkflowBundleAckExportReceiptValueInput<'a> {
    decision: &'a str,
    ack: &'a ControlLiveWorkflowBundleAck,
    diagnostics: &'a [String],
}

#[derive(Debug, Clone, Copy)]
struct LiveWorkflowBundleAckImportReceiptValueInput<'a> {
    decision: &'a str,
    state_root: &'a Path,
    ack: &'a ControlLiveWorkflowBundleAck,
    imported_refs: &'a [String],
    diagnostics: &'a [String],
}

#[derive(Debug)]
struct ImportParts {
    imported_refs: Vec<String>,
    ticket_import_ref: Option<String>,
    authority_import_ref: Option<String>,
}

#[derive(Debug, Clone, Copy)]
struct ReconcileArtifacts<'a> {
    apply: &'a ControlLiveWorkflowBundleApplyReceipt,
    send: Option<&'a ControlLiveSendReceipt>,
    ingress: Option<&'a ControlIngressReceipt>,
    queue: Option<&'a ControlQueueReceipt>,
    control: Option<&'a crate::node_runtime::ControlReceipt>,
}

#[derive(Debug, Clone, Copy)]
struct LiveWorkflowBundleReconcileBindings<'a> {
    send_receipt_ref: Option<&'a str>,
    ingress_receipt_ref: Option<&'a str>,
    queue_receipt_ref: Option<&'a str>,
    control_receipt_ref: Option<&'a str>,
    envelope_ref: Option<&'a str>,
    operation_ref: Option<&'a str>,
    request_ref: Option<&'a str>,
}

#[derive(Debug, Clone, Copy)]
struct LiveWorkflowBundleExpectedInput<'a> {
    expected_node: Option<&'a str>,
    expected_topic: Option<&'a str>,
    expected_endpoint: Option<&'a str>,
    expected_peer: Option<&'a str>,
    expected_operations: &'a [String],
    expected_target_scope: Option<&'a str>,
    expected_resource_scope: Option<&'a str>,
    as_of_sequence: u64,
    as_of_epoch: u64,
}

#[derive(Debug, Clone, Copy)]
struct LiveWorkflowBundleImportReceiptValueInput<'a> {
    decision: &'a str,
    state_root: &'a Path,
    bundle: &'a ControlLiveWorkflowBundle,
    ticket_import_ref: Option<&'a str>,
    authority_import_ref: Option<&'a str>,
    imported_refs: &'a [String],
    diagnostics: &'a [String],
}

#[derive(Debug)]
struct DeniedLiveSendInput<'a> {
    input: &'a ControlLiveSendInput<'a>,
    ticket: &'a ControlLiveTicket,
    envelope: ControlIngressEnvelope,
    diagnostics: Vec<String>,
    retry_receipt_refs: Vec<String>,
    retry_receipt_values: Vec<IoValue>,
}

#[derive(Debug, Clone, Copy)]
struct LiveWorkflowReceiptValueInput<'a> {
    decision: &'a str,
    ticket: &'a ControlLiveTicket,
    admission: &'a ControlLivePeerAdmission,
    authority: &'a ControlAuthorityGrant,
    send: &'a ControlLiveSendReceipt,
    receive_receipt_refs: &'a [String],
    listener_receipt_ref: Option<&'a str>,
    service_receipt_ref: &'a str,
    diagnostics: &'a [String],
}

#[derive(Debug, Clone, Copy)]
struct IngressReceiptValueInput<'a> {
    decision: &'a str,
    phase: &'a str,
    transport: &'a str,
    envelope: &'a ControlIngressEnvelope,
    idempotency_receipt_ref: Option<&'a str>,
    queue_receipt_ref: Option<&'a str>,
    diagnostics: &'a [String],
}

#[derive(Debug, Clone, Copy)]
struct QueueReceiptValueInput<'a> {
    decision: &'a str,
    phase: &'a str,
    operation: &'a str,
    request_ref: &'a str,
    location_ref: &'a str,
    diagnostics: &'a [String],
}

#[derive(Debug, Clone, Copy)]
struct OperationReceiptValueInput<'a> {
    decision: &'a str,
    request: &'a crate::node_runtime::ControlRequest,
    diagnostics: &'a [String],
}

#[derive(Debug, Clone, Copy)]
struct HeartbeatReceiptValueInput<'a> {
    startup_receipt_ref: &'a str,
    lock_ref: &'a str,
    loop_sequence: u64,
    processed_count: u64,
    diagnostics: &'a [String],
}

#[derive(Debug, Clone, Copy)]
struct LoopReceiptValueInput<'a> {
    decision: &'a str,
    startup_receipt_ref: &'a str,
    heartbeat_receipt_ref: &'a str,
    max_requests: u64,
    processed_request_refs: &'a [String],
    dispatch_receipt_refs: &'a [String],
    has_stopped: bool,
    diagnostics: &'a [String],
}

#[derive(Debug, Clone, Copy)]
struct OperationFinalizeInput<'a> {
    state_root: &'a Path,
    request: &'a crate::node_runtime::ControlRequest,
    startup_receipt_ref: &'a str,
    subreceipt_refs: &'a [String],
    diagnostics: &'a [String],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Init {
    pub config_ref: String,
    pub identity_ref: String,
    pub identity_receipt_ref: String,
    pub config_value: IoValue,
    pub identity_receipt_value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Run {
    pub startup_ref: String,
    pub startup_value: IoValue,
    pub adapter_receipt_refs: Vec<crate::node_runtime::NodeAdapterReceiptRef>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Status {
    pub health_ref: String,
    pub control_receipt_ref: String,
    pub health_value: IoValue,
    pub control_receipt_value: IoValue,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stop {
    pub shutdown_ref: String,
    pub control_receipt_ref: String,
    pub shutdown_value: IoValue,
    pub control_receipt_value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlSubmit {
    pub request_ref: String,
    pub inbox_path: PathBuf,
    pub queue_receipt_ref: String,
    pub queue_receipt_value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlDispatch {
    pub operation: String,
    pub request_ref: String,
    pub control_receipt_ref: String,
    pub control_receipt_value: IoValue,
    pub subreceipt_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlLoop {
    pub loop_receipt_ref: String,
    pub loop_receipt_value: IoValue,
    pub heartbeat_receipt_ref: String,
    pub heartbeat_receipt_value: IoValue,
    pub processed_request_refs: Vec<String>,
    pub dispatch_receipt_refs: Vec<String>,
    pub has_stopped: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlServe {
    pub service_receipt_ref: String,
    pub service_receipt_value: IoValue,
    pub service_lock_ref: Option<String>,
    pub heartbeat_receipt_refs: Vec<String>,
    pub ingress_receipt_refs: Vec<String>,
    pub loop_receipt_refs: Vec<String>,
    pub processed_request_refs: Vec<String>,
    pub supervisor_policy_ref: Option<String>,
    pub supervisor_receipt_refs: Vec<String>,
    pub ticks: u64,
    pub has_stopped: bool,
    pub decision: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlSupervisorPolicy {
    pub policy_ref: String,
    pub max_restarts: u64,
    pub restart_window_ticks: u64,
    pub heartbeat_timeout_ticks: u64,
    pub shutdown_drain_ticks: u64,
    pub stale_lock_recovery: bool,
    pub policy_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlSupervisorReceipt {
    pub receipt_ref: String,
    pub decision: String,
    pub operation: String,
    pub supervisor_policy_ref: Option<String>,
    pub diagnostics: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlIngressEnvelope {
    pub envelope_ref: String,
    pub transport: String,
    pub topic: String,
    pub from_peer: String,
    pub to_node: String,
    pub sequence: u64,
    pub operation_ref: String,
    pub request: crate::node_runtime::ControlRequest,
    pub peer_bootstrap_refs: Vec<String>,
    pub authority_refs: Vec<String>,
    pub policy_refs: Vec<String>,
    pub resource_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlIngressPublish {
    pub envelope_ref: String,
    pub envelope_path: PathBuf,
    pub receipt_ref: String,
    pub receipt_value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlIngressDeliver {
    pub envelope_ref: String,
    pub request_ref: String,
    pub ingress_receipt_ref: String,
    pub ingress_receipt_value: IoValue,
    pub idempotency_receipt_ref: Option<String>,
    pub queue_receipt_ref: Option<String>,
    pub has_enqueued: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlLiveIngressPublish {
    pub envelope_ref: String,
    pub transport_receipt_ref: String,
    pub transport_receipt_value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlLiveIngressReceive {
    pub envelope_ref: String,
    pub transport_receipt_ref: String,
    pub transport_receipt_value: IoValue,
    pub ingress_receipt_ref: String,
    pub ingress_receipt_value: IoValue,
    pub has_enqueued: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlLiveLoopback {
    pub envelope_ref: String,
    pub publish_receipt_ref: String,
    pub publish_receipt_value: IoValue,
    pub receive_receipt_ref: String,
    pub receive_receipt_value: IoValue,
    pub ingress_receipt_ref: String,
    pub has_enqueued: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlLiveSend {
    pub envelope_ref: String,
    pub envelope_value: IoValue,
    pub operation_ref: String,
    pub receiver_ticket_ref: String,
    pub receiver_endpoint_id: String,
    pub transport_receipt_ref: Option<String>,
    pub transport_receipt_value: Option<IoValue>,
    pub retry_receipt_refs: Vec<String>,
    pub retry_receipt_values: Vec<IoValue>,
    pub duplicate_receipt_ref: Option<String>,
    pub duplicate_receipt_value: Option<IoValue>,
    pub send_receipt_ref: String,
    pub send_receipt_value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlLiveSendReceipt {
    pub receipt_ref: String,
    pub decision: String,
    pub from_peer: String,
    pub to_node: String,
    pub topic: String,
    pub receiver_ticket_ref: String,
    pub receiver_endpoint_id: String,
    pub receiver_address_refs: Vec<String>,
    pub envelope_ref: String,
    pub transport_receipt_ref: Option<String>,
    pub diagnostics: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlLiveWorkflowReceipt {
    pub receipt_ref: String,
    pub receipt_value: IoValue,
    pub decision: String,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlLiveWorkflowBundle {
    pub bundle_ref: String,
    pub bundle_value: IoValue,
    pub ticket_ref: String,
    pub peer_admission_ref: String,
    pub authority_grant_ref: String,
    pub receipt_refs: Vec<String>,
    pub ticket_value: IoValue,
    pub peer_admission_value: IoValue,
    pub authority_grant_value: IoValue,
    pub receipt_values: Vec<IoValue>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlLiveWorkflowBundleExport {
    pub bundle: ControlLiveWorkflowBundle,
    pub receipt_ref: String,
    pub receipt_value: IoValue,
    pub decision: String,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlLiveWorkflowBundleVerify {
    pub bundle_ref: String,
    pub ticket_ref: Option<String>,
    pub peer_admission_ref: Option<String>,
    pub authority_grant_ref: Option<String>,
    pub receipt_refs: Vec<String>,
    pub diagnostics: Vec<String>,
    pub receipt_ref: String,
    pub receipt_value: IoValue,
    pub decision: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlLiveWorkflowBundleVerifyReceipt {
    pub receipt_ref: String,
    pub decision: String,
    pub bundle_ref: String,
    pub ticket_ref: Option<String>,
    pub peer_admission_ref: Option<String>,
    pub authority_grant_ref: Option<String>,
    pub receipt_refs: Vec<String>,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlLiveWorkflowBundleGate {
    pub bundle_ref: String,
    pub verify_receipt_ref: Option<String>,
    pub recomputed_verify_receipt_ref: String,
    pub ticket_ref: Option<String>,
    pub peer_admission_ref: Option<String>,
    pub authority_grant_ref: Option<String>,
    pub receipt_refs: Vec<String>,
    pub diagnostics: Vec<String>,
    pub receipt_ref: String,
    pub receipt_value: IoValue,
    pub decision: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlLiveWorkflowBundleGateReceipt {
    pub receipt_ref: String,
    pub decision: String,
    pub bundle_ref: String,
    pub verify_receipt_ref: Option<String>,
    pub recomputed_verify_receipt_ref: String,
    pub ticket_ref: Option<String>,
    pub peer_admission_ref: Option<String>,
    pub authority_grant_ref: Option<String>,
    pub receipt_refs: Vec<String>,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlLiveSendPreflight {
    pub decision: String,
    pub envelope_ref: String,
    pub operation_ref: String,
    pub receiver_ticket_ref: String,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlLiveWorkflowBundleApply {
    pub bundle_ref: String,
    pub gate_receipt_ref: Option<String>,
    pub recomputed_verify_receipt_ref: String,
    pub import_receipt_ref: Option<String>,
    pub imported_refs: Vec<String>,
    pub envelope_ref: Option<String>,
    pub operation_ref: Option<String>,
    pub send_receipt_ref: Option<String>,
    pub send_receipt_value: Option<IoValue>,
    pub diagnostics: Vec<String>,
    pub receipt_ref: String,
    pub receipt_value: IoValue,
    pub decision: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlLiveWorkflowBundleApplyReceipt {
    pub receipt_ref: String,
    pub decision: String,
    pub bundle_ref: String,
    pub gate_receipt_ref: Option<String>,
    pub recomputed_verify_receipt_ref: String,
    pub import_receipt_ref: Option<String>,
    pub imported_refs: Vec<String>,
    pub mode: String,
    pub envelope_ref: Option<String>,
    pub operation_ref: Option<String>,
    pub send_receipt_ref: Option<String>,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlIngressReceipt {
    pub receipt_ref: String,
    pub decision: String,
    pub phase: String,
    pub transport: String,
    pub topic: String,
    pub from_peer: String,
    pub to_node: String,
    pub sequence: u64,
    pub envelope_ref: String,
    pub operation_ref: String,
    pub request_ref: String,
    pub idempotency_receipt_ref: Option<String>,
    pub queue_receipt_ref: Option<String>,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlQueueReceipt {
    pub receipt_ref: String,
    pub decision: String,
    pub phase: String,
    pub operation: String,
    pub request_ref: String,
    pub location_ref: String,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlLiveWorkflowBundleReconcile {
    pub bundle_ref: String,
    pub apply_receipt_ref: String,
    pub send_receipt_ref: Option<String>,
    pub ingress_receipt_ref: Option<String>,
    pub queue_receipt_ref: Option<String>,
    pub control_receipt_ref: Option<String>,
    pub envelope_ref: Option<String>,
    pub operation_ref: Option<String>,
    pub request_ref: Option<String>,
    pub diagnostics: Vec<String>,
    pub receipt_ref: String,
    pub receipt_value: IoValue,
    pub decision: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlLiveWorkflowBundleReconcileReceipt {
    pub receipt_ref: String,
    pub decision: String,
    pub apply_receipt_ref: String,
    pub bundle_ref: String,
    pub send_receipt_ref: Option<String>,
    pub ingress_receipt_ref: Option<String>,
    pub queue_receipt_ref: Option<String>,
    pub control_receipt_ref: Option<String>,
    pub envelope_ref: Option<String>,
    pub operation_ref: Option<String>,
    pub request_ref: Option<String>,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlLiveWorkflowBundleAck {
    pub ack_ref: String,
    pub ack_value: IoValue,
    pub apply_receipt_ref: String,
    pub send_receipt_ref: Option<String>,
    pub ingress_receipt_ref: Option<String>,
    pub queue_receipt_ref: Option<String>,
    pub control_receipt_ref: Option<String>,
    pub reconcile_receipt_ref: String,
    pub bundle_ref: String,
    pub envelope_ref: Option<String>,
    pub operation_ref: Option<String>,
    pub request_ref: Option<String>,
    pub receiver_decision: String,
    pub receiver_diagnostics: Vec<String>,
    pub diagnostics: Vec<String>,
    pub apply_receipt_value: IoValue,
    pub send_receipt_value: Option<IoValue>,
    pub ingress_receipt_value: Option<IoValue>,
    pub queue_receipt_value: Option<IoValue>,
    pub control_receipt_value: Option<IoValue>,
    pub reconcile_receipt_value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlLiveWorkflowBundleAckExport {
    pub ack: ControlLiveWorkflowBundleAck,
    pub receipt_ref: String,
    pub receipt_value: IoValue,
    pub decision: String,
    pub receiver_decision: String,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlLiveWorkflowBundleAckImport {
    pub ack_ref: String,
    pub bundle_ref: String,
    pub imported_refs: Vec<String>,
    pub receiver_decision: String,
    pub diagnostics: Vec<String>,
    pub receipt_ref: String,
    pub receipt_value: IoValue,
    pub decision: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlLiveWorkflowProtocolGate {
    pub session_id: String,
    pub install_receipt_ref: String,
    pub protocol_ref: String,
    pub receipt_ref: String,
    pub receipt_value: IoValue,
    pub decision: String,
    pub operation_count: usize,
    pub message_count: usize,
    pub diagnostics: Vec<String>,
    pub manifest_value: IoValue,
    pub install_receipt_value: IoValue,
    pub initial_state_values: Vec<IoValue>,
    pub operation_receipt_values: Vec<IoValue>,
    pub message_values: Vec<IoValue>,
    pub next_state_values: Vec<IoValue>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlLiveWorkflowBundleImport {
    pub bundle_ref: String,
    pub ticket_import_ref: Option<String>,
    pub authority_import_ref: Option<String>,
    pub imported_refs: Vec<String>,
    pub diagnostics: Vec<String>,
    pub receipt_ref: String,
    pub receipt_value: IoValue,
    pub decision: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlLiveServe {
    pub listener_receipt_ref: String,
    pub listener_receipt_value: IoValue,
    pub service: ControlServe,
    pub transport_receipt_refs: Vec<String>,
    pub neighbor_events: Vec<String>,
    pub observed_events: u64,
    pub bound_endpoint_id: String,
    pub live_ticket_ref: Option<String>,
    pub live_ticket_value: Option<IoValue>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlLiveServeLoopback {
    pub envelope_ref: String,
    pub publish_receipt_ref: String,
    pub listener: ControlLiveServe,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlAuthorityGrant {
    pub grant_ref: String,
    pub peer_id: String,
    pub node_id: String,
    pub operations: Vec<String>,
    pub target_scope: String,
    pub resource_scope: String,
    pub epoch: u64,
    pub expires_at: Option<u64>,
    pub policy_refs: Vec<String>,
    pub revocation_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlLiveTicket {
    pub ticket_ref: String,
    pub node_id: String,
    pub node_identity_ref: String,
    pub logical_endpoint_id: String,
    pub live_endpoint_id: String,
    pub topic: String,
    pub address_refs: Vec<String>,
    pub policy_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlLivePeerAdmission {
    pub admission_ref: String,
    pub decision: String,
    pub peer_id: String,
    pub ticket_ref: String,
    pub node_id: String,
    pub topic: String,
    pub sequence: u64,
    pub expires_at: Option<u64>,
    pub policy_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub diagnostics: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlLiveTicketImport {
    pub decision: String,
    pub ticket_ref: String,
    pub peer_admission_ref: Option<String>,
    pub imported_refs: Vec<String>,
    pub diagnostics: Vec<String>,
    pub receipt_ref: String,
    pub receipt_value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlAuthorityGrantImport {
    pub decision: String,
    pub grant_ref: String,
    pub imported_refs: Vec<String>,
    pub diagnostics: Vec<String>,
    pub receipt_ref: String,
    pub receipt_value: IoValue,
}

pub fn control_authority_grant_value(input: &ControlAuthorityGrantInput<'_>) -> Result<IoValue> {
    validate_node_id(input.peer_id)?;
    validate_node_id(input.node_id)?;
    validate_node_id(input.target_scope)?;
    validate_node_id(input.resource_scope)?;
    if input.operations.is_empty() {
        return Err(MoltenError::invalid_harness("node control authority grant operations missing"));
    }
    for operation in input.operations {
        validate_node_id(operation)?;
    }
    validate_ingress_refs(input.policy_refs, "node control authority grant policy ref")?;
    validate_ingress_refs(input.revocation_refs, "node control authority grant revocation ref")?;
    validate_ingress_refs(input.evidence_refs, "node control authority grant evidence ref")?;
    Ok(crate::preserves_rail::record("node-control-authority-grant-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::NODE_CONTROL_AUTHORITY_GRANT_SCHEMA),
        crate::preserves_rail::record("peer", vec![crate::preserves_rail::string(input.peer_id)]),
        crate::preserves_rail::record("node", vec![crate::preserves_rail::string(input.node_id)]),
        crate::preserves_rail::record("operations", vec![crate::preserves_rail::sequence(
            input.operations.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("target-scope", vec![crate::preserves_rail::string(input.target_scope)]),
        crate::preserves_rail::record("resource-scope", vec![crate::preserves_rail::string(input.resource_scope)]),
        crate::preserves_rail::record("epoch", vec![crate::preserves_rail::string(input.epoch.to_string())]),
        crate::preserves_rail::record("expires-at", vec![optional_string(
            input.expires_at.map(|value| value.to_string()).as_deref(),
        )]),
        crate::preserves_rail::record("policy", vec![crate::preserves_rail::sequence(
            input.policy_refs.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("revocations", vec![crate::preserves_rail::sequence(
            input.revocation_refs.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("evidence", vec![crate::preserves_rail::sequence(
            input.evidence_refs.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("checks", vec![crate::preserves_rail::sequence(vec![
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("peer-node-bound"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("operation-scope-bound"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("revocation-checked-at-ingress"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("transport-is-not-authority"),
                crate::preserves_rail::string("pass"),
            ]),
        ])]),
    ]))
}

pub fn parse_control_authority_grant(value: &IoValue) -> Result<ControlAuthorityGrant> {
    let fields = value
        .collect_simple_record("node-control-authority-grant-v1", Some(12))
        .ok_or_else(|| MoltenError::invalid_harness("expected <node-control-authority-grant-v1 ...>"))?;
    require_schema(
        &fields[0],
        crate::preserves_rail::NODE_CONTROL_AUTHORITY_GRANT_SCHEMA,
        "node control authority grant",
    )?;
    let operations = record_strings(&fields[3], "operations")?;
    if operations.is_empty() {
        return Err(MoltenError::invalid_harness("node control authority grant operations missing"));
    }
    Ok(ControlAuthorityGrant {
        grant_ref: crate::preserves_rail::canonical_hash(value)?,
        peer_id: record_string(&fields[1], "peer")?,
        node_id: record_string(&fields[2], "node")?,
        operations,
        target_scope: record_string(&fields[4], "target-scope")?,
        resource_scope: record_string(&fields[5], "resource-scope")?,
        epoch: record_u64_string(&fields[6], "epoch")?,
        expires_at: record_optional_u64_string(&fields[7], "expires-at")?,
        policy_refs: record_ref_strings(&fields[8], "policy")?,
        revocation_refs: record_ref_strings(&fields[9], "revocations")?,
        evidence_refs: record_ref_strings(&fields[10], "evidence")?,
        value: value.clone(),
    })
}

pub fn import_control_authority_grant(state_root: &Path, grant_value: &IoValue) -> Result<ControlAuthorityGrant> {
    validate_state_root(state_root)?;
    ensure_state_layout(state_root)?;
    let grant = parse_control_authority_grant(grant_value)?;
    import_artifact(state_root, grant_value)?;
    Ok(grant)
}

pub fn control_live_ticket_value(input: &ControlLiveTicketInput<'_>) -> Result<IoValue> {
    validate_node_id(input.node_id)?;
    validate_ingress_ref(input.node_identity_ref, "node control live ticket identity ref")?;
    validate_node_id(input.logical_endpoint_id)?;
    validate_node_id(input.live_endpoint_id)?;
    validate_node_id(input.topic)?;
    validate_ingress_refs(input.policy_refs, "node control live ticket policy ref")?;
    validate_ingress_refs(input.evidence_refs, "node control live ticket evidence ref")?;
    Ok(crate::preserves_rail::record("node-control-live-ticket-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::NODE_CONTROL_LIVE_TICKET_SCHEMA),
        crate::preserves_rail::record("node", vec![
            crate::preserves_rail::record("id", vec![crate::preserves_rail::string(input.node_id)]),
            crate::preserves_rail::record("identity", vec![crate::preserves_rail::string(input.node_identity_ref)]),
            crate::preserves_rail::record("logical-endpoint", vec![crate::preserves_rail::string(
                input.logical_endpoint_id,
            )]),
        ]),
        crate::preserves_rail::record("live", vec![
            crate::preserves_rail::record("endpoint-id", vec![crate::preserves_rail::string(input.live_endpoint_id)]),
            crate::preserves_rail::record("topic", vec![crate::preserves_rail::string(input.topic)]),
            crate::preserves_rail::record("addresses", vec![crate::preserves_rail::sequence(
                input.address_refs.iter().map(crate::preserves_rail::string).collect(),
            )]),
        ]),
        crate::preserves_rail::record("policy", vec![crate::preserves_rail::sequence(
            input.policy_refs.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("evidence", vec![crate::preserves_rail::sequence(
            input.evidence_refs.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("checks", vec![crate::preserves_rail::sequence(vec![
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("node-identity-bound"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("live-endpoint-bound"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("ticket-is-bootstrap-not-authority"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("authority-grant-still-required"),
                crate::preserves_rail::string("pass"),
            ]),
        ])]),
    ]))
}

pub fn parse_control_live_ticket(value: &IoValue) -> Result<ControlLiveTicket> {
    let fields = value
        .collect_simple_record("node-control-live-ticket-v1", Some(6))
        .ok_or_else(|| MoltenError::invalid_harness("expected <node-control-live-ticket-v1 ...>"))?;
    require_schema(&fields[0], crate::preserves_rail::NODE_CONTROL_LIVE_TICKET_SCHEMA, "node control live ticket")?;
    let node = crate::preserves_rail::value_to_iovalue(&fields[1]);
    let node_fields = node
        .collect_simple_record("node", Some(3))
        .ok_or_else(|| MoltenError::invalid_harness("node control live ticket missing node"))?;
    let live = crate::preserves_rail::value_to_iovalue(&fields[2]);
    let live_fields = live
        .collect_simple_record("live", Some(3))
        .ok_or_else(|| MoltenError::invalid_harness("node control live ticket missing live endpoint"))?;
    Ok(ControlLiveTicket {
        ticket_ref: crate::preserves_rail::canonical_hash(value)?,
        node_id: record_string(&node_fields[0], "id")?,
        node_identity_ref: record_ref_string(&node_fields[1], "identity")?,
        logical_endpoint_id: record_string(&node_fields[2], "logical-endpoint")?,
        live_endpoint_id: record_string(&live_fields[0], "endpoint-id")?,
        topic: record_string(&live_fields[1], "topic")?,
        address_refs: record_strings(&live_fields[2], "addresses")?,
        policy_refs: record_ref_strings(&fields[3], "policy")?,
        evidence_refs: record_ref_strings(&fields[4], "evidence")?,
        value: value.clone(),
    })
}

pub fn export_control_live_ticket(input: &ControlLiveTicketExportInput<'_>) -> Result<ControlLiveTicket> {
    validate_state_root(input.state_root)?;
    validate_node_id(input.topic)?;
    ensure_state_layout(input.state_root)?;
    let identity = crate::node_identity::parse_node_identity(&read_preserves(&input.state_root.join(IDENTITY_FILE))?)?;
    let address_refs = Vec::new();
    let value = control_live_ticket_value(&ControlLiveTicketInput {
        node_id: &identity.node_id,
        node_identity_ref: &identity.identity_ref,
        logical_endpoint_id: &identity.endpoint_id,
        live_endpoint_id: &stable_live_endpoint_id(&identity),
        topic: input.topic,
        address_refs: &address_refs,
        policy_refs: input.policy_refs,
        evidence_refs: input.evidence_refs,
    })?;
    let ticket = parse_control_live_ticket(&value)?;
    import_artifact(input.state_root, &value)?;
    Ok(ticket)
}

pub fn admit_control_live_peer(input: &ControlLivePeerAdmitInput<'_>) -> Result<ControlLivePeerAdmission> {
    validate_state_root(input.state_root)?;
    validate_node_id(input.peer_id)?;
    validate_ingress_refs(input.policy_refs, "node control live peer admission policy ref")?;
    validate_ingress_refs(input.evidence_refs, "node control live peer admission evidence ref")?;
    ensure_state_layout(input.state_root)?;
    let ticket = parse_control_live_ticket(input.ticket_value)?;
    import_artifact(input.state_root, input.ticket_value)?;
    let identity = crate::node_identity::parse_node_identity(&read_preserves(&input.state_root.join(IDENTITY_FILE))?)?;
    let mut diagnostics = Vec::new();
    if ticket.node_id != identity.node_id {
        diagnostics.push(format!(
            "node control live ticket node {} does not match local node {}",
            ticket.node_id, identity.node_id
        ));
    }
    if ticket.node_identity_ref != identity.identity_ref {
        diagnostics.push("node control live ticket identity ref does not match local identity".to_string());
    }
    let expected_live_endpoint = stable_live_endpoint_id(&identity);
    if ticket.live_endpoint_id != expected_live_endpoint {
        diagnostics.push(format!(
            "node control live ticket endpoint {} does not match local endpoint {}",
            ticket.live_endpoint_id, expected_live_endpoint
        ));
    }
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let value = control_live_peer_admission_value(&LivePeerAdmissionValueInput {
        decision,
        peer_id: input.peer_id,
        ticket: &ticket,
        admission_sequence: input.sequence,
        expires_at: input.expires_at,
        policy_refs: input.policy_refs,
        evidence_refs: input.evidence_refs,
        diagnostics: &diagnostics,
    })?;
    let admission = parse_control_live_peer_admission(&value)?;
    import_artifact(input.state_root, &value)?;
    Ok(admission)
}

fn control_live_peer_admission_value(input: &LivePeerAdmissionValueInput<'_>) -> Result<IoValue> {
    validate_decision(input.decision)?;
    Ok(crate::preserves_rail::record("node-control-live-peer-admission-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::NODE_CONTROL_LIVE_PEER_ADMISSION_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(input.decision)]),
        crate::preserves_rail::record("peer", vec![crate::preserves_rail::string(input.peer_id)]),
        crate::preserves_rail::record("ticket", vec![crate::preserves_rail::string(&input.ticket.ticket_ref)]),
        crate::preserves_rail::record("node", vec![crate::preserves_rail::string(&input.ticket.node_id)]),
        crate::preserves_rail::record("topic", vec![crate::preserves_rail::string(&input.ticket.topic)]),
        crate::preserves_rail::record("sequence", vec![crate::preserves_rail::string(
            input.admission_sequence.to_string(),
        )]),
        crate::preserves_rail::record("expires-at", vec![optional_string(
            input.expires_at.map(|value| value.to_string()).as_deref(),
        )]),
        crate::preserves_rail::record("policy", vec![crate::preserves_rail::sequence(
            input.policy_refs.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("evidence", vec![crate::preserves_rail::sequence(
            input.evidence_refs.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("diagnostics", vec![crate::preserves_rail::sequence(
            input.diagnostics.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("checks", vec![crate::preserves_rail::sequence(vec![
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("ticket-bound"),
                crate::preserves_rail::string(if input.decision == "pass" { "pass" } else { "fail" }),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("peer-topic-bound"),
                crate::preserves_rail::string(if input.decision == "pass" { "pass" } else { "fail" }),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("bootstrap-not-authority"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("authority-grant-still-required"),
                crate::preserves_rail::string("pass"),
            ]),
        ])]),
    ]))
}

pub fn parse_control_live_peer_admission(value: &IoValue) -> Result<ControlLivePeerAdmission> {
    let fields = value
        .collect_simple_record("node-control-live-peer-admission-v1", Some(12))
        .ok_or_else(|| MoltenError::invalid_harness("expected <node-control-live-peer-admission-v1 ...>"))?;
    require_schema(
        &fields[0],
        crate::preserves_rail::NODE_CONTROL_LIVE_PEER_ADMISSION_SCHEMA,
        "node control live peer admission",
    )?;
    Ok(ControlLivePeerAdmission {
        admission_ref: crate::preserves_rail::canonical_hash(value)?,
        decision: record_string(&fields[1], "decision")?,
        peer_id: record_string(&fields[2], "peer")?,
        ticket_ref: record_ref_string(&fields[3], "ticket")?,
        node_id: record_string(&fields[4], "node")?,
        topic: record_string(&fields[5], "topic")?,
        sequence: record_u64_string(&fields[6], "sequence")?,
        expires_at: record_optional_u64_string(&fields[7], "expires-at")?,
        policy_refs: record_ref_strings(&fields[8], "policy")?,
        evidence_refs: record_ref_strings(&fields[9], "evidence")?,
        diagnostics: record_strings(&fields[10], "diagnostics")?,
        value: value.clone(),
    })
}

pub fn import_control_live_ticket(input: &ControlLiveTicketImportInput<'_>) -> Result<ControlLiveTicketImport> {
    validate_state_root(input.state_root)?;
    ensure_state_layout(input.state_root)?;
    if let Some(node) = input.expected_node {
        validate_node_id(node)?;
    }
    if let Some(topic) = input.expected_topic {
        validate_node_id(topic)?;
    }
    if let Some(endpoint) = input.expected_endpoint {
        validate_node_id(endpoint)?;
    }
    if let Some(peer) = input.expected_peer {
        validate_node_id(peer)?;
    }
    let ticket = parse_control_live_ticket(input.ticket_value)?;
    let admission = input.peer_admission_value.map(parse_control_live_peer_admission).transpose()?;
    let mut diagnostics = live_ticket_import_diagnostics(input, &ticket, admission.as_ref());
    if input.peer_admission_value.is_some() && admission.is_none() {
        diagnostics.push("node control live ticket import admission was not parsed".to_string());
    }
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let mut imported_refs = Vec::with_capacity(2);
    if diagnostics.is_empty() {
        imported_refs.push(import_artifact(input.state_root, input.ticket_value)?);
        if let Some(value) = input.peer_admission_value {
            imported_refs.push(import_artifact(input.state_root, value)?);
        }
    }
    let peer_id = admission.as_ref().map(|value| value.peer_id.as_str()).or(input.expected_peer);
    let receipt_value = live_ticket_import_receipt_value(&LiveTicketImportReceiptValueInput {
        decision,
        state_root: input.state_root,
        ticket: &ticket,
        peer_admission_ref: admission.as_ref().map(|value| value.admission_ref.as_str()),
        peer_id,
        as_of_sequence: input.as_of_sequence,
        imported_refs: &imported_refs,
        diagnostics: &diagnostics,
    })?;
    let receipt_ref = crate::preserves_rail::canonical_hash(&receipt_value)?;
    import_artifact(input.state_root, &receipt_value)?;
    Ok(ControlLiveTicketImport {
        decision: decision.to_string(),
        ticket_ref: ticket.ticket_ref,
        peer_admission_ref: admission.map(|value| value.admission_ref),
        imported_refs,
        diagnostics,
        receipt_ref,
        receipt_value,
    })
}

pub fn import_control_authority_grant_checked(
    input: &ControlAuthorityGrantImportInput<'_>,
) -> Result<ControlAuthorityGrantImport> {
    validate_state_root(input.state_root)?;
    ensure_state_layout(input.state_root)?;
    if let Some(peer) = input.expected_peer {
        validate_node_id(peer)?;
    }
    if let Some(node) = input.expected_node {
        validate_node_id(node)?;
    }
    for operation in input.expected_operations {
        validate_node_id(operation)?;
    }
    if let Some(scope) = input.expected_target_scope {
        validate_node_id(scope)?;
    }
    if let Some(scope) = input.expected_resource_scope {
        validate_node_id(scope)?;
    }
    let grant = parse_control_authority_grant(input.grant_value)?;
    let diagnostics = authority_grant_import_diagnostics(input, &grant);
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let mut imported_refs = Vec::with_capacity(1);
    if diagnostics.is_empty() {
        imported_refs.push(import_artifact(input.state_root, input.grant_value)?);
    }
    let receipt_value = authority_grant_import_receipt_value(&AuthorityGrantImportReceiptValueInput {
        decision,
        state_root: input.state_root,
        grant: &grant,
        as_of_epoch: input.as_of_epoch,
        imported_refs: &imported_refs,
        diagnostics: &diagnostics,
    })?;
    let receipt_ref = crate::preserves_rail::canonical_hash(&receipt_value)?;
    import_artifact(input.state_root, &receipt_value)?;
    Ok(ControlAuthorityGrantImport {
        decision: decision.to_string(),
        grant_ref: grant.grant_ref,
        imported_refs,
        diagnostics,
        receipt_ref,
        receipt_value,
    })
}

pub fn export_control_live_workflow_bundle(
    input: &ControlLiveWorkflowBundleExportInput<'_>,
) -> Result<ControlLiveWorkflowBundleExport> {
    let ticket = parse_control_live_ticket(input.receiver_ticket_value)?;
    let admission = parse_control_live_peer_admission(input.peer_admission_value)?;
    let authority = parse_control_authority_grant(input.authority_grant_value)?;
    let receipt_refs = live_workflow_bundle_receipt_refs(input.receipt_values)?;
    let mut diagnostics = live_workflow_bundle_binding_diagnostics(&ticket, &admission, &authority);
    diagnostics.extend(live_workflow_bundle_receipt_diagnostics(input.receipt_values));
    let bundle_value = live_workflow_bundle_value(&LiveWorkflowBundleValueInput {
        ticket: &ticket,
        admission: &admission,
        authority: &authority,
        ticket_value: input.receiver_ticket_value,
        admission_value: input.peer_admission_value,
        authority_value: input.authority_grant_value,
        receipt_values: input.receipt_values,
        diagnostics: &diagnostics,
    })?;
    let bundle_ref = crate::preserves_rail::canonical_hash(&bundle_value)?;
    let bundle = ControlLiveWorkflowBundle {
        bundle_ref,
        bundle_value,
        ticket_ref: ticket.ticket_ref,
        peer_admission_ref: admission.admission_ref,
        authority_grant_ref: authority.grant_ref,
        receipt_refs,
        ticket_value: input.receiver_ticket_value.clone(),
        peer_admission_value: input.peer_admission_value.clone(),
        authority_grant_value: input.authority_grant_value.clone(),
        receipt_values: input.receipt_values.iter().map(|value| (**value).clone()).collect(),
    };
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let receipt_value = live_workflow_bundle_export_receipt_value(&LiveWorkflowBundleExportReceiptValueInput {
        decision,
        bundle: &bundle,
        diagnostics: &diagnostics,
    })?;
    let receipt_ref = crate::preserves_rail::canonical_hash(&receipt_value)?;
    Ok(ControlLiveWorkflowBundleExport {
        bundle,
        receipt_ref,
        receipt_value,
        decision: decision.to_string(),
        diagnostics,
    })
}

pub fn verify_control_live_workflow_bundle(
    input: &ControlLiveWorkflowBundleVerifyInput<'_>,
) -> Result<ControlLiveWorkflowBundleVerify> {
    validate_live_workflow_bundle_verify_input(input)?;
    let bundle_ref = crate::preserves_rail::canonical_hash(input.bundle_value)?;
    let expected = live_workflow_bundle_expected_input_from_verify(input);
    let parsed = parse_control_live_workflow_bundle(input.bundle_value);
    let (ticket_ref, peer_admission_ref, authority_grant_ref, receipt_refs, diagnostics) = match parsed {
        Ok(bundle) => {
            let ticket = parse_control_live_ticket(&bundle.ticket_value)?;
            let admission = parse_control_live_peer_admission(&bundle.peer_admission_value)?;
            let authority = parse_control_authority_grant(&bundle.authority_grant_value)?;
            let receipt_value_refs = bundle.receipt_values.iter().collect::<Vec<_>>();
            let mut diagnostics = live_workflow_bundle_expected_diagnostics(&expected, &ticket, &admission, &authority);
            diagnostics.extend(live_workflow_bundle_receipt_diagnostics(&receipt_value_refs));
            (
                Some(bundle.ticket_ref),
                Some(bundle.peer_admission_ref),
                Some(bundle.authority_grant_ref),
                bundle.receipt_refs,
                diagnostics,
            )
        }
        Err(error) => (None, None, None, Vec::new(), vec![format!(
            "node control live workflow bundle parse failed: {error}"
        )]),
    };
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let receipt_value = live_workflow_bundle_verify_receipt_value(&LiveWorkflowBundleVerifyReceiptValueInput {
        decision,
        bundle_ref: &bundle_ref,
        ticket_ref: ticket_ref.as_deref(),
        peer_admission_ref: peer_admission_ref.as_deref(),
        authority_grant_ref: authority_grant_ref.as_deref(),
        receipt_refs: &receipt_refs,
        expected: &expected,
        diagnostics: &diagnostics,
    })?;
    let receipt_ref = crate::preserves_rail::canonical_hash(&receipt_value)?;
    Ok(ControlLiveWorkflowBundleVerify {
        bundle_ref,
        ticket_ref,
        peer_admission_ref,
        authority_grant_ref,
        receipt_refs,
        diagnostics,
        receipt_ref,
        receipt_value,
        decision: decision.to_string(),
    })
}

pub fn gate_control_live_workflow_bundle(
    input: &ControlLiveWorkflowBundleGateInput<'_>,
) -> Result<ControlLiveWorkflowBundleGate> {
    let verify_input = live_workflow_bundle_verify_input_from_gate(input);
    let verified = verify_control_live_workflow_bundle(&verify_input)?;
    let expected = live_workflow_bundle_expected_input_from_verify(&verify_input);
    let mut diagnostics = verified.diagnostics.clone();
    let verify_receipt_ref = match input.verify_receipt_value {
        Some(value) => match parse_control_live_workflow_bundle_verify_receipt(value) {
            Ok(receipt) => {
                if receipt.receipt_ref != verified.receipt_ref {
                    diagnostics.push(format!(
                        "node control live workflow bundle gate verify receipt {} does not match recomputed {}",
                        receipt.receipt_ref, verified.receipt_ref
                    ));
                }
                Some(receipt.receipt_ref)
            }
            Err(error) => {
                let receipt_ref = crate::preserves_rail::canonical_hash(value)?;
                diagnostics
                    .push(format!("node control live workflow bundle gate verify receipt parse failed: {error}"));
                Some(receipt_ref)
            }
        },
        None => {
            if input.require_verify_receipt {
                diagnostics
                    .push("node control live workflow bundle gate requires a current verify receipt".to_string());
            }
            None
        }
    };
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let receipt_value = live_workflow_bundle_gate_receipt_value(&LiveWorkflowBundleGateReceiptValueInput {
        decision,
        bundle_ref: &verified.bundle_ref,
        verify_receipt_ref: verify_receipt_ref.as_deref(),
        recomputed_verify_receipt_ref: &verified.receipt_ref,
        ticket_ref: verified.ticket_ref.as_deref(),
        peer_admission_ref: verified.peer_admission_ref.as_deref(),
        authority_grant_ref: verified.authority_grant_ref.as_deref(),
        receipt_refs: &verified.receipt_refs,
        expected: &expected,
        diagnostics: &diagnostics,
    })?;
    let receipt_ref = crate::preserves_rail::canonical_hash(&receipt_value)?;
    Ok(ControlLiveWorkflowBundleGate {
        bundle_ref: verified.bundle_ref,
        verify_receipt_ref,
        recomputed_verify_receipt_ref: verified.receipt_ref,
        ticket_ref: verified.ticket_ref,
        peer_admission_ref: verified.peer_admission_ref,
        authority_grant_ref: verified.authority_grant_ref,
        receipt_refs: verified.receipt_refs,
        diagnostics,
        receipt_ref,
        receipt_value,
        decision: decision.to_string(),
    })
}

#[derive(Debug, Default)]
struct GateCheck {
    receipt_ref: Option<String>,
    diagnostics: Vec<String>,
}

#[derive(Debug, Default)]
struct ImportStep {
    receipt_ref: Option<String>,
    imported_refs: Vec<String>,
    diagnostics: Vec<String>,
}

#[derive(Debug, Default)]
struct TransferStep {
    envelope_ref: Option<String>,
    operation_ref: Option<String>,
    send_receipt_ref: Option<String>,
    send_receipt_value: Option<IoValue>,
    diagnostics: Vec<String>,
}

struct FinishInput<'a> {
    input: &'a ControlLiveWorkflowBundleApplyInput<'a>,
    verified: ControlLiveWorkflowBundleVerify,
    expected: LiveWorkflowBundleExpectedInput<'a>,
    gate_receipt_ref: Option<String>,
    import_receipt_ref: Option<String>,
    imported_refs: Vec<String>,
    envelope_ref: Option<String>,
    operation_ref: Option<String>,
    send_receipt_ref: Option<String>,
    send_receipt_value: Option<IoValue>,
    diagnostics: Vec<String>,
}

fn apply_gate_check(
    input: &ControlLiveWorkflowBundleApplyInput<'_>,
    verified: &ControlLiveWorkflowBundleVerify,
) -> Result<GateCheck> {
    let mut diagnostics = Vec::new();
    let receipt_ref = match input.gate_receipt_value {
        Some(value) => match parse_control_live_workflow_bundle_gate_receipt(value) {
            Ok(receipt) => {
                if receipt.decision != "pass" {
                    diagnostics.push(format!(
                        "node control live workflow bundle apply gate receipt {} decision {}",
                        receipt.receipt_ref, receipt.decision
                    ));
                }
                if receipt.bundle_ref != verified.bundle_ref {
                    diagnostics.push(format!(
                        "node control live workflow bundle apply gate bundle {} does not match {}",
                        receipt.bundle_ref, verified.bundle_ref
                    ));
                }
                if receipt.recomputed_verify_receipt_ref != verified.receipt_ref {
                    diagnostics.push(format!(
                        "node control live workflow bundle apply gate recomputed verify {} does not match current {}",
                        receipt.recomputed_verify_receipt_ref, verified.receipt_ref
                    ));
                }
                Some(receipt.receipt_ref)
            }
            Err(error) => {
                let receipt_ref = crate::preserves_rail::canonical_hash(value)?;
                diagnostics.push(format!("node control live workflow bundle apply gate receipt parse failed: {error}"));
                Some(receipt_ref)
            }
        },
        None => {
            if input.is_gate_receipt_required {
                diagnostics.push("node control live workflow bundle apply requires a current gate receipt".to_string());
            }
            None
        }
    };
    Ok(GateCheck {
        receipt_ref,
        diagnostics,
    })
}

fn apply_import_step(input: &ControlLiveWorkflowBundleApplyInput<'_>) -> Result<ImportStep> {
    let imported = import_control_live_workflow_bundle(&live_workflow_bundle_import_input_from_apply(input))?;
    if imported.decision == "pass" {
        Ok(ImportStep {
            receipt_ref: Some(imported.receipt_ref),
            imported_refs: imported.imported_refs,
            diagnostics: Vec::new(),
        })
    } else {
        Ok(ImportStep {
            receipt_ref: Some(imported.receipt_ref),
            imported_refs: Vec::new(),
            diagnostics: imported.diagnostics,
        })
    }
}

async fn apply_transfer_step(input: &ControlLiveWorkflowBundleApplyInput<'_>) -> Result<TransferStep> {
    let Some(request_value) = input.request_value else {
        return Ok(TransferStep::default());
    };
    let bundle = parse_control_live_workflow_bundle(input.bundle_value)?;
    let authority = parse_control_authority_grant(&bundle.authority_grant_value)?;
    let from_peer = input.from_peer.unwrap_or(&authority.peer_id);
    let peer_bootstrap_refs = if input.peer_bootstrap_refs.is_empty() {
        vec![bundle.peer_admission_ref.clone()]
    } else {
        input.peer_bootstrap_refs.to_vec()
    };
    let authority_refs = if input.authority_refs.is_empty() {
        vec![bundle.authority_grant_ref.clone()]
    } else {
        input.authority_refs.to_vec()
    };
    let send_input = ControlLiveSendInput {
        state_root: Some(input.state_root),
        request_value,
        receiver_ticket_value: &bundle.ticket_value,
        from_peer,
        sequence: input.sequence,
        expected_operation_ref: input.expected_operation_ref,
        expected_receiver_node: input.expected_node,
        expected_topic: input.expected_topic,
        expected_endpoint: input.expected_endpoint,
        max_attempts: input.max_attempts,
        peer_bootstrap_refs: &peer_bootstrap_refs,
        authority_refs: &authority_refs,
        policy_refs: input.policy_refs,
        resource_refs: input.resource_refs,
        evidence_refs: input.evidence_refs,
        join_timeout_ms: input.join_timeout_ms,
    };
    if input.should_send {
        let sent = send_control_live_ingress(&send_input).await?;
        let send_receipt = parse_control_live_send_receipt(&sent.send_receipt_value)?;
        let diagnostics = if send_receipt.decision == "pass" {
            Vec::new()
        } else {
            send_receipt.diagnostics
        };
        Ok(TransferStep {
            envelope_ref: Some(sent.envelope_ref),
            operation_ref: Some(sent.operation_ref),
            send_receipt_ref: Some(sent.send_receipt_ref),
            send_receipt_value: Some(sent.send_receipt_value),
            diagnostics,
        })
    } else {
        let preflight = preflight_control_live_send(&send_input)?;
        let diagnostics = if preflight.decision == "pass" {
            Vec::new()
        } else {
            preflight.diagnostics
        };
        Ok(TransferStep {
            envelope_ref: Some(preflight.envelope_ref),
            operation_ref: Some(preflight.operation_ref),
            diagnostics,
            ..TransferStep::default()
        })
    }
}

fn finish_apply(input: FinishInput<'_>) -> Result<ControlLiveWorkflowBundleApply> {
    let decision = if input.diagnostics.is_empty() { "pass" } else { "deny" };
    let mode = if input.input.should_send {
        "send"
    } else if input.input.request_value.is_some() {
        "dry-run"
    } else {
        "import"
    };
    let receipt_value = live_workflow_bundle_apply_receipt_value(&LiveWorkflowBundleApplyReceiptValueInput {
        decision,
        state_root: input.input.state_root,
        bundle_ref: &input.verified.bundle_ref,
        gate_receipt_ref: input.gate_receipt_ref.as_deref(),
        recomputed_verify_receipt_ref: &input.verified.receipt_ref,
        import_receipt_ref: input.import_receipt_ref.as_deref(),
        imported_refs: &input.imported_refs,
        mode,
        envelope_ref: input.envelope_ref.as_deref(),
        operation_ref: input.operation_ref.as_deref(),
        send_receipt_ref: input.send_receipt_ref.as_deref(),
        expected: &input.expected,
        diagnostics: &input.diagnostics,
    })?;
    let receipt_ref = crate::preserves_rail::canonical_hash(&receipt_value)?;
    import_artifact(input.input.state_root, &receipt_value)?;
    Ok(ControlLiveWorkflowBundleApply {
        bundle_ref: input.verified.bundle_ref,
        gate_receipt_ref: input.gate_receipt_ref,
        recomputed_verify_receipt_ref: input.verified.receipt_ref,
        import_receipt_ref: input.import_receipt_ref,
        imported_refs: input.imported_refs,
        envelope_ref: input.envelope_ref,
        operation_ref: input.operation_ref,
        send_receipt_ref: input.send_receipt_ref,
        send_receipt_value: input.send_receipt_value,
        diagnostics: input.diagnostics,
        receipt_ref,
        receipt_value,
        decision: decision.to_string(),
    })
}

pub async fn apply_control_live_workflow_bundle(
    input: &ControlLiveWorkflowBundleApplyInput<'_>,
) -> Result<ControlLiveWorkflowBundleApply> {
    validate_live_workflow_bundle_apply_input(input)?;
    ensure_state_layout(input.state_root)?;
    let verify_input = live_workflow_bundle_verify_input_from_apply(input);
    let verified = verify_control_live_workflow_bundle(&verify_input)?;
    let expected = live_workflow_bundle_expected_input_from_verify(&verify_input);
    let GateCheck {
        receipt_ref: gate_receipt_ref,
        diagnostics: gate_diagnostics,
    } = apply_gate_check(input, &verified)?;
    let mut diagnostics = verified.diagnostics.clone();
    diagnostics.extend(gate_diagnostics);
    if input.should_send && input.request_value.is_none() {
        diagnostics.push("node control live workflow bundle apply send requested without a request".to_string());
    }
    let ImportStep {
        receipt_ref: import_receipt_ref,
        imported_refs,
        diagnostics: import_diagnostics,
    } = if diagnostics.is_empty() {
        apply_import_step(input)?
    } else {
        ImportStep::default()
    };
    diagnostics.extend(import_diagnostics);
    let TransferStep {
        envelope_ref,
        operation_ref,
        send_receipt_ref,
        send_receipt_value,
        diagnostics: transfer_diagnostics,
    } = if diagnostics.is_empty() {
        apply_transfer_step(input).await?
    } else {
        TransferStep::default()
    };
    diagnostics.extend(transfer_diagnostics);
    finish_apply(FinishInput {
        input,
        verified,
        expected,
        gate_receipt_ref,
        import_receipt_ref,
        imported_refs,
        envelope_ref,
        operation_ref,
        send_receipt_ref,
        send_receipt_value,
        diagnostics,
    })
}

pub fn reconcile_control_live_workflow_bundle(
    input: &ControlLiveWorkflowBundleReconcileInput<'_>,
) -> Result<ControlLiveWorkflowBundleReconcile> {
    validate_live_workflow_bundle_reconcile_input(input)?;
    let apply = parse_control_live_workflow_bundle_apply_receipt(input.apply_receipt_value)?;
    let send = input.send_receipt_value.map(parse_control_live_send_receipt).transpose()?;
    let ingress = input.ingress_receipt_value.map(parse_control_ingress_receipt).transpose()?;
    let queue = input.queue_receipt_value.map(parse_control_queue_receipt).transpose()?;
    let control = input.control_receipt_value.map(crate::node_runtime::parse_control_receipt).transpose()?;
    let artifacts = ReconcileArtifacts {
        apply: &apply,
        send: send.as_ref(),
        ingress: ingress.as_ref(),
        queue: queue.as_ref(),
        control: control.as_ref(),
    };
    let mut diagnostics = live_workflow_bundle_reconcile_diagnostics(input, &artifacts)?;
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let bindings = live_workflow_bundle_reconcile_bindings(&artifacts);
    let receipt_value = live_workflow_bundle_reconcile_receipt_value(&LiveWorkflowBundleReconcileReceiptValueInput {
        decision,
        apply_receipt_ref: &apply.receipt_ref,
        bundle_ref: &apply.bundle_ref,
        send_receipt_ref: bindings.send_receipt_ref,
        ingress_receipt_ref: bindings.ingress_receipt_ref,
        queue_receipt_ref: bindings.queue_receipt_ref,
        control_receipt_ref: bindings.control_receipt_ref,
        envelope_ref: bindings.envelope_ref,
        operation_ref: bindings.operation_ref,
        request_ref: bindings.request_ref,
        diagnostics: &diagnostics,
    })?;
    let receipt_ref = crate::preserves_rail::canonical_hash(&receipt_value)?;
    diagnostics.shrink_to_fit();
    Ok(ControlLiveWorkflowBundleReconcile {
        bundle_ref: apply.bundle_ref.clone(),
        apply_receipt_ref: apply.receipt_ref.clone(),
        send_receipt_ref: bindings.send_receipt_ref.map(ToString::to_string),
        ingress_receipt_ref: bindings.ingress_receipt_ref.map(ToString::to_string),
        queue_receipt_ref: bindings.queue_receipt_ref.map(ToString::to_string),
        control_receipt_ref: bindings.control_receipt_ref.map(ToString::to_string),
        envelope_ref: bindings.envelope_ref.map(ToString::to_string),
        operation_ref: bindings.operation_ref.map(ToString::to_string),
        request_ref: bindings.request_ref.map(ToString::to_string),
        diagnostics,
        receipt_ref,
        receipt_value,
        decision: decision.to_string(),
    })
}

pub fn export_control_live_workflow_bundle_ack(
    input: &ControlLiveWorkflowBundleAckExportInput<'_>,
) -> Result<ControlLiveWorkflowBundleAckExport> {
    let reconciled = reconcile_control_live_workflow_bundle(&ControlLiveWorkflowBundleReconcileInput {
        apply_receipt_value: input.apply_receipt_value,
        send_receipt_value: input.send_receipt_value,
        ingress_receipt_value: input.ingress_receipt_value,
        queue_receipt_value: input.queue_receipt_value,
        control_receipt_value: input.control_receipt_value,
        expected_envelope_ref: None,
        expected_operation_ref: None,
        expected_request_ref: None,
    })?;
    let reconcile = parse_control_live_workflow_bundle_reconcile_receipt(input.reconcile_receipt_value)?;
    let mut diagnostics = live_workflow_bundle_ack_export_diagnostics(input, &reconciled, &reconcile)?;
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let ack_value = live_workflow_bundle_ack_value(&LiveWorkflowBundleAckValueInput {
        apply_receipt_value: input.apply_receipt_value,
        send_receipt_value: input.send_receipt_value,
        ingress_receipt_value: input.ingress_receipt_value,
        queue_receipt_value: input.queue_receipt_value,
        control_receipt_value: input.control_receipt_value,
        reconcile_receipt_value: input.reconcile_receipt_value,
        apply_receipt_ref: &reconcile.apply_receipt_ref,
        send_receipt_ref: reconcile.send_receipt_ref.as_deref(),
        ingress_receipt_ref: reconcile.ingress_receipt_ref.as_deref(),
        queue_receipt_ref: reconcile.queue_receipt_ref.as_deref(),
        control_receipt_ref: reconcile.control_receipt_ref.as_deref(),
        reconcile_receipt_ref: &reconcile.receipt_ref,
        bundle_ref: &reconcile.bundle_ref,
        envelope_ref: reconcile.envelope_ref.as_deref(),
        operation_ref: reconcile.operation_ref.as_deref(),
        request_ref: reconcile.request_ref.as_deref(),
        receiver_decision: &reconcile.decision,
        receiver_diagnostics: &reconcile.diagnostics,
        diagnostics: &diagnostics,
    })?;
    let ack = parse_control_live_workflow_bundle_ack(&ack_value)?;
    let receipt_value = live_workflow_bundle_ack_export_receipt_value(&LiveWorkflowBundleAckExportReceiptValueInput {
        decision,
        ack: &ack,
        diagnostics: &diagnostics,
    })?;
    let receipt_ref = crate::preserves_rail::canonical_hash(&receipt_value)?;
    diagnostics.shrink_to_fit();
    Ok(ControlLiveWorkflowBundleAckExport {
        receiver_decision: ack.receiver_decision.clone(),
        ack,
        receipt_ref,
        receipt_value,
        decision: decision.to_string(),
        diagnostics,
    })
}

pub fn import_control_live_workflow_bundle_ack(
    input: &ControlLiveWorkflowBundleAckImportInput<'_>,
) -> Result<ControlLiveWorkflowBundleAckImport> {
    validate_live_workflow_bundle_ack_import_input(input)?;
    ensure_state_layout(input.state_root)?;
    let ack = parse_control_live_workflow_bundle_ack(input.ack_value)?;
    let mut diagnostics = live_workflow_bundle_ack_import_diagnostics(input, &ack)?;
    let mut imported_refs = Vec::with_capacity(8);
    if diagnostics.is_empty() {
        imported_refs.extend(import_live_workflow_bundle_ack_members(input.state_root, &ack)?);
    }
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let receipt_value = live_workflow_bundle_ack_import_receipt_value(&LiveWorkflowBundleAckImportReceiptValueInput {
        decision,
        state_root: input.state_root,
        ack: &ack,
        imported_refs: &imported_refs,
        diagnostics: &diagnostics,
    })?;
    let receipt_ref = crate::preserves_rail::canonical_hash(&receipt_value)?;
    import_artifact(input.state_root, &receipt_value)?;
    diagnostics.shrink_to_fit();
    Ok(ControlLiveWorkflowBundleAckImport {
        ack_ref: ack.ack_ref.clone(),
        bundle_ref: ack.bundle_ref.clone(),
        imported_refs,
        receiver_decision: ack.receiver_decision.clone(),
        diagnostics,
        receipt_ref,
        receipt_value,
        decision: decision.to_string(),
    })
}

pub fn gate_control_live_workflow_protocol(
    input: &ControlLiveWorkflowProtocolGateInput<'_>,
) -> Result<ControlLiveWorkflowProtocolGate> {
    validate_live_workflow_protocol_gate_input(input)?;
    let (evidence, diagnostics) = live_workflow_protocol_evidence(input)?;
    let manifest_value = live_workflow_protocol_manifest_value()?;
    let install = crate::protocol_session::install_protocol_manifest_value(&manifest_value)?;
    let authority_refs = vec![evidence.authority_ref.clone()];
    let resource_refs = vec![evidence.resource_ref.clone()];
    let values = run_values(input, &install, &evidence, &authority_refs, &resource_refs)?;
    let gate = crate::protocol_session::gate_protocol_session_lifecycle_with_diagnostics(
        crate::protocol_session::ProtocolSessionGateInput {
            install_receipt: install.value.clone(),
            initial_states: values.initial_state_values.clone(),
            operation_receipts: values.operation_receipt_values.clone(),
            messages: values.message_values.clone(),
            next_states: values.next_state_values.clone(),
        },
        diagnostics,
    )?;
    Ok(ControlLiveWorkflowProtocolGate {
        session_id: evidence.session_id,
        install_receipt_ref: gate.install_ref.clone(),
        protocol_ref: gate.protocol_ref.clone(),
        receipt_ref: gate.receipt_ref.clone(),
        receipt_value: gate.value.clone(),
        decision: gate.decision,
        operation_count: gate.operation_count,
        message_count: gate.message_count,
        diagnostics: gate.diagnostics,
        manifest_value,
        install_receipt_value: install.value,
        initial_state_values: values.initial_state_values,
        operation_receipt_values: values.operation_receipt_values,
        message_values: values.message_values,
        next_state_values: values.next_state_values,
    })
}

struct RolePair {
    sender: crate::protocol_session::ProtocolSessionState,
    receiver: crate::protocol_session::ProtocolSessionState,
}

struct LegInput<'a> {
    origin_state: &'a IoValue,
    target_state: &'a IoValue,
    target_role: &'a str,
    label: &'a str,
    payload_tag: &'a str,
    body_or_ref: &'a IoValue,
    authority_refs: &'a [String],
    resource_refs: &'a [String],
    evidence_refs: Vec<String>,
    carrier_refs: Vec<String>,
    message_label: &'a str,
    origin_label: &'a str,
    target_label: &'a str,
}

struct LegOutput {
    send: crate::protocol_session::ProtocolOperationRun,
    receive: crate::protocol_session::ProtocolOperationRun,
    message: crate::protocol_session::ProtocolMessage,
    origin_next: crate::protocol_session::ProtocolSessionState,
    target_next: crate::protocol_session::ProtocolSessionState,
}

struct RunValues {
    initial_state_values: Vec<IoValue>,
    operation_receipt_values: Vec<IoValue>,
    message_values: Vec<IoValue>,
    next_state_values: Vec<IoValue>,
}

fn start_pair(
    install: &crate::protocol_session::ProtocolInstallReceipt,
    session_id: &str,
    authority_refs: &[String],
    resource_refs: &[String],
) -> Result<RolePair> {
    Ok(RolePair {
        sender: crate::protocol_session::start_protocol_session(
            install,
            "sender",
            session_id,
            authority_refs.to_vec(),
            resource_refs.to_vec(),
        )?,
        receiver: crate::protocol_session::start_protocol_session(
            install,
            "receiver",
            session_id,
            authority_refs.to_vec(),
            resource_refs.to_vec(),
        )?,
    })
}

fn step_leg(input: LegInput<'_>) -> Result<LegOutput> {
    let authority_refs = input.authority_refs.to_vec();
    let resource_refs = input.resource_refs.to_vec();
    let send = crate::protocol_session::send_protocol_message(crate::protocol_session::ProtocolSendInput {
        state: input.origin_state.clone(),
        to_role: input.target_role.to_string(),
        label: input.label.to_string(),
        payload_tag: input.payload_tag.to_string(),
        body_or_ref: input.body_or_ref.clone(),
        authority_refs: authority_refs.clone(),
        resource_refs: resource_refs.clone(),
        evidence_refs: input.evidence_refs,
    })?;
    let message = protocol_message(&send, input.message_label)?;
    let receive = crate::protocol_session::receive_protocol_message(crate::protocol_session::ProtocolReceiveInput {
        state: input.target_state.clone(),
        message: message.value.clone(),
        authority_refs,
        resource_refs,
        carrier_refs: input.carrier_refs,
    })?;
    Ok(LegOutput {
        origin_next: protocol_next_state(&send, input.origin_label)?,
        target_next: protocol_next_state(&receive, input.target_label)?,
        send,
        receive,
        message,
    })
}

fn run_values(
    input: &ControlLiveWorkflowProtocolGateInput<'_>,
    install: &crate::protocol_session::ProtocolInstallReceipt,
    evidence: &LiveWorkflowProtocolEvidence,
    authority_refs: &[String],
    resource_refs: &[String],
) -> Result<RunValues> {
    let initial = start_pair(install, &evidence.session_id, authority_refs, resource_refs)?;
    let handoff = step_leg(LegInput {
        origin_state: &initial.sender.value,
        target_state: &initial.receiver.value,
        target_role: "receiver",
        label: "bundle-handoff",
        payload_tag: "workflow-bundle",
        body_or_ref: input.bundle_value,
        authority_refs,
        resource_refs,
        evidence_refs: vec![evidence.gate_receipt_ref.clone()],
        carrier_refs: vec![evidence.gate_receipt_ref.clone()],
        message_label: "bundle handoff",
        origin_label: "bundle handoff sender",
        target_label: "bundle handoff receiver",
    })?;
    let apply = step_leg(LegInput {
        origin_state: &handoff.origin_next.value,
        target_state: &handoff.target_next.value,
        target_role: "receiver",
        label: "apply-evidence",
        payload_tag: "apply-receipt",
        body_or_ref: input.apply_receipt_value,
        authority_refs,
        resource_refs,
        evidence_refs: vec![evidence.apply_receipt_ref.clone(), evidence.gate_receipt_ref.clone()],
        carrier_refs: vec![evidence.apply_receipt_ref.clone()],
        message_label: "apply evidence",
        origin_label: "apply evidence sender",
        target_label: "apply evidence receiver",
    })?;
    let ack = step_leg(LegInput {
        origin_state: &apply.target_next.value,
        target_state: &apply.origin_next.value,
        target_role: "sender",
        label: "ack-evidence",
        payload_tag: "workflow-ack",
        body_or_ref: input.ack_value,
        authority_refs,
        resource_refs,
        evidence_refs: vec![evidence.reconcile_receipt_ref.clone(), evidence.ack_ref.clone()],
        carrier_refs: vec![evidence.ack_ref.clone()],
        message_label: "workflow ack",
        origin_label: "workflow ack receiver",
        target_label: "workflow ack sender",
    })?;
    Ok(RunValues {
        initial_state_values: vec![initial.sender.value.clone(), initial.receiver.value.clone()],
        operation_receipt_values: vec![
            handoff.send.receipt.value.clone(),
            handoff.receive.receipt.value.clone(),
            apply.send.receipt.value.clone(),
            apply.receive.receipt.value.clone(),
            ack.send.receipt.value.clone(),
            ack.receive.receipt.value.clone(),
        ],
        message_values: vec![handoff.message.value, apply.message.value, ack.message.value],
        next_state_values: vec![
            handoff.origin_next.value,
            handoff.target_next.value,
            apply.origin_next.value,
            apply.target_next.value,
            ack.origin_next.value,
            ack.target_next.value,
        ],
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LiveWorkflowProtocolEvidence {
    session_id: String,
    authority_ref: String,
    resource_ref: String,
    gate_receipt_ref: String,
    apply_receipt_ref: String,
    reconcile_receipt_ref: String,
    ack_ref: String,
}

fn validate_live_workflow_protocol_gate_input(input: &ControlLiveWorkflowProtocolGateInput<'_>) -> Result<()> {
    if let Some(reference) = input.expected_envelope_ref {
        validate_ingress_ref(reference, "node control live workflow protocol expected envelope ref")?;
    }
    if let Some(reference) = input.expected_operation_ref {
        validate_ingress_ref(reference, "node control live workflow protocol expected operation ref")?;
    }
    if let Some(reference) = input.expected_request_ref {
        validate_ingress_ref(reference, "node control live workflow protocol expected request ref")?;
    }
    Ok(())
}

fn parsed_or_note<T>(
    diagnostics: &mut impl VecSink<String>,
    label: &str,
    parse: impl FnOnce() -> Result<T>,
) -> Option<T> {
    match parse() {
        Ok(parsed) => Some(parsed),
        Err(error) => {
            diagnostics.push_item(format!("{label} parse failed: {error}"));
            None
        }
    }
}

fn note_receipt_decision(
    diagnostics: &mut impl VecSink<String>,
    label: &str,
    receipt_ref: &str,
    decision: &str,
    notes: &[String],
) {
    if decision != "pass" {
        diagnostics.push_item(format!("{label} receipt {receipt_ref} decision {decision}"));
        diagnostics.extend_cloned_items(notes);
    }
}

fn note_ref_mismatch(diagnostics: &mut impl VecSink<String>, label: &str, observed: &str, expected: &str) {
    if observed != expected {
        diagnostics.push_item(format!("{label} {observed} does not match {expected}"));
    }
}

fn note_optional_ref_mismatch(
    diagnostics: &mut impl VecSink<String>,
    label: &str,
    observed: Option<&str>,
    expected: &str,
) {
    if observed != Some(expected) {
        diagnostics.push_item(format!("{label} {} does not match {expected}", observed.unwrap_or("none")));
    }
}

fn note_expected_ref(
    diagnostics: &mut impl VecSink<String>,
    label: &str,
    observed: Option<&str>,
    expected: Option<&str>,
) {
    if let Some(expected) = expected
        && observed != Some(expected)
    {
        diagnostics.push_item(format!("{label} {} does not match expected {expected}", observed.unwrap_or("none")));
    }
}

struct ReceiptRefs<'a> {
    bundle: &'a str,
    gate: &'a str,
    apply: &'a str,
    reconcile: &'a str,
}

struct ExpectedRefs<'a> {
    envelope: Option<&'a str>,
    operation: Option<&'a str>,
    request: Option<&'a str>,
}

fn note_gate_part(
    diagnostics: &mut impl VecSink<String>,
    gate: &ControlLiveWorkflowBundleGateReceipt,
    refs: &ReceiptRefs<'_>,
) {
    note_receipt_decision(
        diagnostics,
        "node control live workflow protocol gate",
        &gate.receipt_ref,
        &gate.decision,
        &gate.diagnostics,
    );
    note_ref_mismatch(diagnostics, "node control live workflow protocol gate bundle", &gate.bundle_ref, refs.bundle);
}

fn note_apply_part(
    diagnostics: &mut impl VecSink<String>,
    apply: &ControlLiveWorkflowBundleApplyReceipt,
    refs: &ReceiptRefs<'_>,
) {
    note_receipt_decision(
        diagnostics,
        "node control live workflow protocol apply",
        &apply.receipt_ref,
        &apply.decision,
        &apply.diagnostics,
    );
    note_ref_mismatch(diagnostics, "node control live workflow protocol apply bundle", &apply.bundle_ref, refs.bundle);
    note_optional_ref_mismatch(
        diagnostics,
        "node control live workflow protocol apply gate",
        apply.gate_receipt_ref.as_deref(),
        refs.gate,
    );
}

fn note_reconcile_part(
    diagnostics: &mut impl VecSink<String>,
    reconcile: &ControlLiveWorkflowBundleReconcileReceipt,
    refs: &ReceiptRefs<'_>,
) {
    note_receipt_decision(
        diagnostics,
        "node control live workflow protocol reconcile",
        &reconcile.receipt_ref,
        &reconcile.decision,
        &reconcile.diagnostics,
    );
    note_ref_mismatch(
        diagnostics,
        "node control live workflow protocol reconcile apply",
        &reconcile.apply_receipt_ref,
        refs.apply,
    );
    note_ref_mismatch(
        diagnostics,
        "node control live workflow protocol reconcile bundle",
        &reconcile.bundle_ref,
        refs.bundle,
    );
}

fn note_ack_part(
    diagnostics: &mut impl VecSink<String>,
    ack: &ControlLiveWorkflowBundleAck,
    refs: &ReceiptRefs<'_>,
    expected: &ExpectedRefs<'_>,
) {
    if ack.receiver_decision != "pass" {
        diagnostics
            .push_item(format!("node control live workflow protocol ack receiver decision {}", ack.receiver_decision));
        diagnostics.extend_cloned_items(&ack.receiver_diagnostics);
    }
    if !ack.diagnostics.is_empty() {
        diagnostics.extend_cloned_items(&ack.diagnostics);
    }
    note_ref_mismatch(diagnostics, "node control live workflow protocol ack apply", &ack.apply_receipt_ref, refs.apply);
    note_ref_mismatch(
        diagnostics,
        "node control live workflow protocol ack reconcile",
        &ack.reconcile_receipt_ref,
        refs.reconcile,
    );
    note_ref_mismatch(diagnostics, "node control live workflow protocol ack bundle", &ack.bundle_ref, refs.bundle);
    note_expected_ref(
        diagnostics,
        "node control live workflow protocol ack envelope",
        ack.envelope_ref.as_deref(),
        expected.envelope,
    );
    note_expected_ref(
        diagnostics,
        "node control live workflow protocol ack operation",
        ack.operation_ref.as_deref(),
        expected.operation,
    );
    note_expected_ref(
        diagnostics,
        "node control live workflow protocol ack request",
        ack.request_ref.as_deref(),
        expected.request,
    );
}

fn live_workflow_protocol_evidence(
    input: &ControlLiveWorkflowProtocolGateInput<'_>,
) -> Result<(LiveWorkflowProtocolEvidence, Vec<String>)> {
    let mut diagnostics = Vec::with_capacity(16);
    let bundle_ref = crate::preserves_rail::canonical_hash(input.bundle_value)?;
    let gate_receipt_ref = crate::preserves_rail::canonical_hash(input.gate_receipt_value)?;
    let apply_receipt_ref = crate::preserves_rail::canonical_hash(input.apply_receipt_value)?;
    let reconcile_receipt_ref = crate::preserves_rail::canonical_hash(input.reconcile_receipt_value)?;
    let ack_ref = crate::preserves_rail::canonical_hash(input.ack_value)?;
    let bundle = parsed_or_note(&mut diagnostics, "node control live workflow protocol bundle", || {
        parse_control_live_workflow_bundle(input.bundle_value)
    });
    let gate = parsed_or_note(&mut diagnostics, "node control live workflow protocol gate receipt", || {
        parse_control_live_workflow_bundle_gate_receipt(input.gate_receipt_value)
    });
    let apply = parsed_or_note(&mut diagnostics, "node control live workflow protocol apply receipt", || {
        parse_control_live_workflow_bundle_apply_receipt(input.apply_receipt_value)
    });
    let reconcile = parsed_or_note(&mut diagnostics, "node control live workflow protocol reconcile receipt", || {
        parse_control_live_workflow_bundle_reconcile_receipt(input.reconcile_receipt_value)
    });
    let ack = parsed_or_note(&mut diagnostics, "node control live workflow protocol ack", || {
        parse_control_live_workflow_bundle_ack(input.ack_value)
    });
    let refs = ReceiptRefs {
        bundle: &bundle_ref,
        gate: &gate_receipt_ref,
        apply: &apply_receipt_ref,
        reconcile: &reconcile_receipt_ref,
    };
    let expected = ExpectedRefs {
        envelope: input.expected_envelope_ref,
        operation: input.expected_operation_ref,
        request: input.expected_request_ref,
    };
    if let Some(gate) = gate.as_ref() {
        note_gate_part(&mut diagnostics, gate, &refs);
    }
    if let Some(apply) = apply.as_ref() {
        note_apply_part(&mut diagnostics, apply, &refs);
    }
    if let Some(reconcile) = reconcile.as_ref() {
        note_reconcile_part(&mut diagnostics, reconcile, &refs);
    }
    if let Some(ack) = ack.as_ref() {
        note_ack_part(&mut diagnostics, ack, &refs, &expected);
    }
    let authority_ref = if let Some(bundle) = bundle.as_ref() {
        bundle.authority_grant_ref.clone()
    } else {
        local_ref("node-control-live-workflow-protocol-authority", &bundle_ref)?
    };
    let resource_ref = bundle.as_ref().map(|bundle| bundle.bundle_ref.clone()).unwrap_or(bundle_ref.clone());
    let session_id = format!("{LIVE_WORKFLOW_PROTOCOL_SESSION_PREFIX}{}", ref_file_stem(&bundle_ref));
    Ok((
        LiveWorkflowProtocolEvidence {
            session_id,
            authority_ref,
            resource_ref,
            gate_receipt_ref,
            apply_receipt_ref,
            reconcile_receipt_ref,
            ack_ref,
        },
        diagnostics,
    ))
}

fn live_workflow_protocol_manifest_value() -> Result<IoValue> {
    let global = crate::protocol_session::protocol_global_script_value(&[
        crate::protocol_session::ProtocolCommInput {
            from_role: "sender".to_string(),
            to_role: "receiver".to_string(),
            label: "bundle-handoff".to_string(),
            payload_tag: "workflow-bundle".to_string(),
        },
        crate::protocol_session::ProtocolCommInput {
            from_role: "sender".to_string(),
            to_role: "receiver".to_string(),
            label: "apply-evidence".to_string(),
            payload_tag: "apply-receipt".to_string(),
        },
        crate::protocol_session::ProtocolCommInput {
            from_role: "receiver".to_string(),
            to_role: "sender".to_string(),
            label: "ack-evidence".to_string(),
            payload_tag: "workflow-ack".to_string(),
        },
    ])?;
    crate::protocol_session::protocol_manifest_value(&crate::protocol_session::ProtocolManifestInput {
        protocol_id: LIVE_WORKFLOW_PROTOCOL_ID.to_string(),
        roles: vec!["sender".to_string(), "receiver".to_string()],
        labels: vec![
            "bundle-handoff".to_string(),
            "apply-evidence".to_string(),
            "ack-evidence".to_string(),
        ],
        payloads: vec![
            crate::protocol_session::ProtocolPayloadInput {
                tag: "workflow-bundle".to_string(),
                schema_ref: local_ref("node-control-live-workflow-protocol-schema", "workflow-bundle")?,
            },
            crate::protocol_session::ProtocolPayloadInput {
                tag: "apply-receipt".to_string(),
                schema_ref: local_ref("node-control-live-workflow-protocol-schema", "apply-receipt")?,
            },
            crate::protocol_session::ProtocolPayloadInput {
                tag: "workflow-ack".to_string(),
                schema_ref: local_ref("node-control-live-workflow-protocol-schema", "workflow-ack")?,
            },
        ],
        global,
        policy_refs: vec![local_ref("node-control-live-workflow-protocol-policy", "v1")?],
        capability_refs: vec![local_ref("node-control-live-workflow-protocol-capability", "v1")?],
        resource_refs: vec![local_ref("node-control-live-workflow-protocol-resource", "v1")?],
    })
}

fn protocol_message(
    run: &crate::protocol_session::ProtocolOperationRun,
    label: &str,
) -> Result<crate::protocol_session::ProtocolMessage> {
    run.message.clone().ok_or_else(|| {
        MoltenError::invalid_harness(format!("node control live workflow protocol missing {label} message"))
    })
}

fn protocol_next_state(
    run: &crate::protocol_session::ProtocolOperationRun,
    label: &str,
) -> Result<crate::protocol_session::ProtocolSessionState> {
    run.next_state.clone().ok_or_else(|| {
        MoltenError::invalid_harness(format!("node control live workflow protocol missing {label} next state"))
    })
}

fn validate_live_workflow_bundle_reconcile_input(input: &ControlLiveWorkflowBundleReconcileInput<'_>) -> Result<()> {
    if let Some(reference) = input.expected_envelope_ref {
        validate_ingress_ref(reference, "node control live workflow bundle reconcile expected envelope ref")?;
    }
    if let Some(reference) = input.expected_operation_ref {
        validate_ingress_ref(reference, "node control live workflow bundle reconcile expected operation ref")?;
    }
    if let Some(reference) = input.expected_request_ref {
        validate_ingress_ref(reference, "node control live workflow bundle reconcile expected request ref")?;
    }
    Ok(())
}

fn live_workflow_bundle_reconcile_diagnostics(
    input: &ControlLiveWorkflowBundleReconcileInput<'_>,
    artifacts: &ReconcileArtifacts<'_>,
) -> Result<Vec<String>> {
    let mut diagnostics = Vec::with_capacity(16);
    if artifacts.apply.decision != "pass" {
        diagnostics.push(format!(
            "node control live workflow bundle reconcile apply receipt {} decision {}",
            artifacts.apply.receipt_ref, artifacts.apply.decision
        ));
        diagnostics.extend(artifacts.apply.diagnostics.clone());
    }
    if artifacts.apply.envelope_ref.is_none() {
        diagnostics.push(
            "node control live workflow bundle reconcile apply receipt has no live envelope; rerun apply with --request"
                .to_string(),
        );
    }
    diagnostics.extend(live_workflow_bundle_reconcile_send_diagnostics(artifacts));
    diagnostics.extend(live_workflow_bundle_reconcile_ingress_diagnostics(input, artifacts));
    diagnostics.extend(live_workflow_bundle_reconcile_queue_diagnostics(artifacts));
    diagnostics.extend(live_workflow_bundle_reconcile_control_diagnostics(artifacts));
    Ok(diagnostics)
}

fn live_workflow_bundle_reconcile_send_diagnostics(artifacts: &ReconcileArtifacts<'_>) -> Vec<String> {
    let mut diagnostics = Vec::with_capacity(4);
    match (&artifacts.apply.send_receipt_ref, artifacts.send) {
        (Some(expected), Some(send)) => {
            if send.receipt_ref != *expected {
                diagnostics.push(format!(
                    "node control live workflow bundle reconcile send receipt {} does not match apply {}",
                    send.receipt_ref, expected
                ));
            }
        }
        (Some(expected), None) => diagnostics.push(format!(
            "node control live workflow bundle reconcile requires send receipt {expected} from apply receipt"
        )),
        (None, Some(_)) => {}
        (None, None) => {}
    }
    if let Some(send) = artifacts.send {
        if send.decision != "pass" {
            diagnostics.push(format!(
                "node control live workflow bundle reconcile send receipt {} decision {}",
                send.receipt_ref, send.decision
            ));
            diagnostics.extend(send.diagnostics.clone());
        }
        if let Some(envelope_ref) = artifacts.apply.envelope_ref.as_ref()
            && send.envelope_ref != *envelope_ref
        {
            diagnostics.push(format!(
                "node control live workflow bundle reconcile send envelope {} does not match apply {}",
                send.envelope_ref, envelope_ref
            ));
        }
    }
    diagnostics
}

fn receiver_ref_note(kind: &str, actual: &str, expected: Option<&str>, source: &str) -> Option<String> {
    let expected = expected?;
    if actual == expected {
        None
    } else {
        Some(format!(
            "node control live workflow bundle reconcile receiver {kind} {actual} does not match {source} {expected}"
        ))
    }
}

fn live_workflow_bundle_reconcile_ingress_diagnostics(
    input: &ControlLiveWorkflowBundleReconcileInput<'_>,
    artifacts: &ReconcileArtifacts<'_>,
) -> Vec<String> {
    let mut diagnostics = Vec::with_capacity(8);
    let Some(ingress) = artifacts.ingress else {
        diagnostics.push(
            "node control live workflow bundle reconcile requires receiver ingress receipt for the live envelope"
                .to_string(),
        );
        return diagnostics;
    };
    if ingress.decision != "pass" {
        diagnostics.push(format!(
            "node control live workflow bundle reconcile receiver ingress receipt {} decision {}",
            ingress.receipt_ref, ingress.decision
        ));
        diagnostics.extend(ingress.diagnostics.clone());
    }
    if let Some(note) = receiver_ref_note("envelope", &ingress.envelope_ref, input.expected_envelope_ref, "expected") {
        diagnostics.push(note);
    }
    if let Some(note) =
        receiver_ref_note("envelope", &ingress.envelope_ref, artifacts.apply.envelope_ref.as_deref(), "apply")
    {
        diagnostics.push(note);
    }
    if let Some(send) = artifacts.send
        && let Some(note) =
            receiver_ref_note("envelope", &ingress.envelope_ref, Some(send.envelope_ref.as_str()), "send")
    {
        diagnostics.push(note);
    }
    if let Some(note) = receiver_ref_note("operation", &ingress.operation_ref, input.expected_operation_ref, "expected")
    {
        diagnostics.push(note);
    }
    if let Some(note) =
        receiver_ref_note("operation", &ingress.operation_ref, artifacts.apply.operation_ref.as_deref(), "apply")
    {
        diagnostics.push(note);
    }
    if let Some(note) = receiver_ref_note("request", &ingress.request_ref, input.expected_request_ref, "expected") {
        diagnostics.push(note);
    }
    if ingress.decision == "pass" && ingress.queue_receipt_ref.is_none() {
        diagnostics.push(
            "node control live workflow bundle reconcile receiver ingress passed without durable queue receipt"
                .to_string(),
        );
    }
    diagnostics
}

fn live_workflow_bundle_reconcile_queue_diagnostics(artifacts: &ReconcileArtifacts<'_>) -> Vec<String> {
    let mut diagnostics = Vec::with_capacity(4);
    if let Some(queue) = artifacts.queue {
        if queue.decision != "pass" {
            diagnostics.push(format!(
                "node control live workflow bundle reconcile queue receipt {} decision {}",
                queue.receipt_ref, queue.decision
            ));
            diagnostics.extend(queue.diagnostics.clone());
        }
        if let Some(ingress) = artifacts.ingress {
            if let Some(expected) = ingress.queue_receipt_ref.as_ref()
                && queue.receipt_ref != *expected
            {
                diagnostics.push(format!(
                    "node control live workflow bundle reconcile queue receipt {} does not match ingress {}",
                    queue.receipt_ref, expected
                ));
            }
            if queue.request_ref != ingress.request_ref {
                diagnostics.push(format!(
                    "node control live workflow bundle reconcile queue request {} does not match ingress {}",
                    queue.request_ref, ingress.request_ref
                ));
            }
        }
    }
    diagnostics
}

fn live_workflow_bundle_reconcile_control_diagnostics(artifacts: &ReconcileArtifacts<'_>) -> Vec<String> {
    let mut diagnostics = Vec::with_capacity(4);
    if let Some(control) = artifacts.control {
        if control.decision != "pass" {
            diagnostics.push(format!(
                "node control live workflow bundle reconcile receiver control receipt {} decision {}",
                control.receipt_ref, control.decision
            ));
            diagnostics.extend(control.diagnostics.clone());
        }
        if let Some(ingress) = artifacts.ingress
            && control.request_ref != ingress.request_ref
        {
            diagnostics.push(format!(
                "node control live workflow bundle reconcile control request {} does not match ingress {}",
                control.request_ref, ingress.request_ref
            ));
        }
    }
    diagnostics
}

fn live_workflow_bundle_reconcile_bindings<'a>(
    artifacts: &'a ReconcileArtifacts<'a>,
) -> LiveWorkflowBundleReconcileBindings<'a> {
    let send_receipt_ref =
        artifacts.send.map(|send| send.receipt_ref.as_str()).or(artifacts.apply.send_receipt_ref.as_deref());
    let ingress_receipt_ref = artifacts.ingress.map(|ingress| ingress.receipt_ref.as_str());
    let queue_receipt_ref = artifacts
        .queue
        .map(|queue| queue.receipt_ref.as_str())
        .or_else(|| artifacts.ingress.and_then(|ingress| ingress.queue_receipt_ref.as_deref()));
    let control_receipt_ref = artifacts.control.map(|control| control.receipt_ref.as_str());
    let envelope_ref = artifacts
        .ingress
        .map(|ingress| ingress.envelope_ref.as_str())
        .or_else(|| artifacts.send.map(|send| send.envelope_ref.as_str()))
        .or(artifacts.apply.envelope_ref.as_deref());
    let operation_ref = artifacts
        .ingress
        .map(|ingress| ingress.operation_ref.as_str())
        .or(artifacts.apply.operation_ref.as_deref());
    let request_ref = artifacts
        .control
        .map(|control| control.request_ref.as_str())
        .or_else(|| artifacts.queue.map(|queue| queue.request_ref.as_str()))
        .or_else(|| artifacts.ingress.map(|ingress| ingress.request_ref.as_str()));
    LiveWorkflowBundleReconcileBindings {
        send_receipt_ref,
        ingress_receipt_ref,
        queue_receipt_ref,
        control_receipt_ref,
        envelope_ref,
        operation_ref,
        request_ref,
    }
}

fn live_workflow_bundle_ack_export_diagnostics(
    input: &ControlLiveWorkflowBundleAckExportInput<'_>,
    reconciled: &ControlLiveWorkflowBundleReconcile,
    reconcile: &ControlLiveWorkflowBundleReconcileReceipt,
) -> Result<Vec<String>> {
    let mut diagnostics = Vec::with_capacity(8);
    if reconcile.receipt_ref != reconciled.receipt_ref {
        diagnostics.push(format!(
            "node control live workflow bundle ack reconcile receipt {} does not match recomputed {}",
            reconcile.receipt_ref, reconciled.receipt_ref
        ));
    }
    if input.ingress_receipt_value.is_none() {
        diagnostics.push("node control live workflow bundle ack requires receiver ingress receipt".to_string());
    }
    let ingress = input.ingress_receipt_value.map(parse_control_ingress_receipt).transpose()?;
    if let Some(ingress) = ingress.as_ref() {
        if ingress.decision == "pass" && input.queue_receipt_value.is_none() {
            diagnostics.push(format!(
                "node control live workflow bundle ack requires queue receipt {} from receiver ingress",
                ingress.queue_receipt_ref.as_deref().unwrap_or("none")
            ));
        }
        if let Some(queue_receipt_ref) = ingress.queue_receipt_ref.as_ref()
            && input.queue_receipt_value.is_none()
        {
            diagnostics.push(format!(
                "node control live workflow bundle ack missing durable queue receipt {queue_receipt_ref}"
            ));
        }
    }
    Ok(diagnostics)
}

fn validate_live_workflow_bundle_ack_import_input(input: &ControlLiveWorkflowBundleAckImportInput<'_>) -> Result<()> {
    validate_state_root(input.state_root)?;
    if let Some(reference) = input.expected_bundle_ref {
        validate_ingress_ref(reference, "node control live workflow bundle ack import expected bundle ref")?;
    }
    if let Some(reference) = input.expected_envelope_ref {
        validate_ingress_ref(reference, "node control live workflow bundle ack import expected envelope ref")?;
    }
    if let Some(reference) = input.expected_operation_ref {
        validate_ingress_ref(reference, "node control live workflow bundle ack import expected operation ref")?;
    }
    if let Some(reference) = input.expected_request_ref {
        validate_ingress_ref(reference, "node control live workflow bundle ack import expected request ref")?;
    }
    Ok(())
}

fn live_workflow_bundle_ack_import_diagnostics(
    input: &ControlLiveWorkflowBundleAckImportInput<'_>,
    ack: &ControlLiveWorkflowBundleAck,
) -> Result<Vec<String>> {
    let recomputed = reconcile_control_live_workflow_bundle(&ControlLiveWorkflowBundleReconcileInput {
        apply_receipt_value: &ack.apply_receipt_value,
        send_receipt_value: ack.send_receipt_value.as_ref(),
        ingress_receipt_value: ack.ingress_receipt_value.as_ref(),
        queue_receipt_value: ack.queue_receipt_value.as_ref(),
        control_receipt_value: ack.control_receipt_value.as_ref(),
        expected_envelope_ref: None,
        expected_operation_ref: None,
        expected_request_ref: None,
    })?;
    let mut diagnostics = ack.diagnostics.clone();
    if ack.reconcile_receipt_ref != recomputed.receipt_ref {
        diagnostics.push(format!(
            "node control live workflow bundle ack import reconcile receipt {} does not match recomputed {}",
            ack.reconcile_receipt_ref, recomputed.receipt_ref
        ));
    }
    if ack.ingress_receipt_value.is_none() {
        diagnostics.push("node control live workflow bundle ack import requires receiver ingress receipt".to_string());
    }
    if let Some(ingress_value) = ack.ingress_receipt_value.as_ref() {
        let ingress = parse_control_ingress_receipt(ingress_value)?;
        if ingress.decision == "pass" && ack.queue_receipt_value.is_none() {
            diagnostics.push(format!(
                "node control live workflow bundle ack import requires queue receipt {} from receiver ingress",
                ingress.queue_receipt_ref.as_deref().unwrap_or("none")
            ));
        }
    }
    if let Some(expected) = input.expected_bundle_ref
        && ack.bundle_ref != expected
    {
        diagnostics.push(format!(
            "node control live workflow bundle ack import bundle {} does not match expected {}",
            ack.bundle_ref, expected
        ));
    }
    if let Some(expected) = input.expected_envelope_ref
        && ack.envelope_ref.as_deref() != Some(expected)
    {
        diagnostics.push(format!(
            "node control live workflow bundle ack import envelope {} does not match expected {}",
            ack.envelope_ref.as_deref().unwrap_or("none"),
            expected
        ));
    }
    if let Some(expected) = input.expected_operation_ref
        && ack.operation_ref.as_deref() != Some(expected)
    {
        diagnostics.push(format!(
            "node control live workflow bundle ack import operation {} does not match expected {}",
            ack.operation_ref.as_deref().unwrap_or("none"),
            expected
        ));
    }
    if let Some(expected) = input.expected_request_ref
        && ack.request_ref.as_deref() != Some(expected)
    {
        diagnostics.push(format!(
            "node control live workflow bundle ack import request {} does not match expected {}",
            ack.request_ref.as_deref().unwrap_or("none"),
            expected
        ));
    }
    Ok(diagnostics)
}

fn import_live_workflow_bundle_ack_members(
    state_root: &Path,
    ack: &ControlLiveWorkflowBundleAck,
) -> Result<Vec<String>> {
    let mut imported_refs = Vec::with_capacity(8);
    imported_refs.push(import_artifact(state_root, &ack.apply_receipt_value)?);
    if let Some(value) = ack.send_receipt_value.as_ref() {
        imported_refs.push(import_artifact(state_root, value)?);
    }
    if let Some(value) = ack.ingress_receipt_value.as_ref() {
        imported_refs.push(import_artifact(state_root, value)?);
    }
    if let Some(value) = ack.queue_receipt_value.as_ref() {
        imported_refs.push(import_artifact(state_root, value)?);
    }
    if let Some(value) = ack.control_receipt_value.as_ref() {
        imported_refs.push(import_artifact(state_root, value)?);
    }
    imported_refs.push(import_artifact(state_root, &ack.reconcile_receipt_value)?);
    imported_refs.push(import_artifact(state_root, &ack.ack_value)?);
    Ok(imported_refs)
}

pub fn parse_control_live_workflow_bundle_apply_receipt(
    value: &IoValue,
) -> Result<ControlLiveWorkflowBundleApplyReceipt> {
    let fields = value
        .collect_simple_record("node-control-live-workflow-bundle-apply-receipt-v1", Some(15))
        .ok_or_else(|| {
            MoltenError::invalid_harness("expected <node-control-live-workflow-bundle-apply-receipt-v1 ...>")
        })?;
    require_schema(
        &fields[0],
        crate::preserves_rail::NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_APPLY_RECEIPT_SCHEMA,
        "node control live workflow bundle apply receipt",
    )?;
    let gate_receipt_ref = record_optional_ref_string(&fields[4], "gate-receipt")?;
    let import_receipt_ref = record_optional_ref_string(&fields[6], "import-receipt")?;
    let envelope_ref = record_optional_ref_string(&fields[9], "envelope")?;
    let operation_ref = record_optional_ref_string(&fields[10], "operation")?;
    let send_receipt_ref = record_optional_ref_string(&fields[11], "send-receipt")?;
    let _expected = record_value(&fields[12], "expected")?;
    let _checks = record_sequence_len(&fields[14], "checks")?;
    let decision = record_string(&fields[1], "decision")?;
    validate_decision(&decision)?;
    Ok(ControlLiveWorkflowBundleApplyReceipt {
        receipt_ref: crate::preserves_rail::canonical_hash(value)?,
        decision,
        bundle_ref: record_ref_string(&fields[3], "bundle")?,
        gate_receipt_ref,
        recomputed_verify_receipt_ref: record_ref_string(&fields[5], "recomputed-verify")?,
        import_receipt_ref,
        imported_refs: record_ref_strings(&fields[7], "imported")?,
        mode: record_string(&fields[8], "mode")?,
        envelope_ref,
        operation_ref,
        send_receipt_ref,
        diagnostics: record_strings(&fields[13], "diagnostics")?,
    })
}

pub fn parse_control_live_workflow_bundle_reconcile_receipt(
    value: &IoValue,
) -> Result<ControlLiveWorkflowBundleReconcileReceipt> {
    let fields = value
        .collect_simple_record("node-control-live-workflow-bundle-reconcile-receipt-v1", Some(13))
        .ok_or_else(|| {
            MoltenError::invalid_harness("expected <node-control-live-workflow-bundle-reconcile-receipt-v1 ...>")
        })?;
    require_schema(
        &fields[0],
        crate::preserves_rail::NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_RECONCILE_RECEIPT_SCHEMA,
        "node control live workflow bundle reconcile receipt",
    )?;
    let send_receipt_ref = record_optional_ref_string(&fields[4], "send-receipt")?;
    let ingress_receipt_ref = record_optional_ref_string(&fields[5], "ingress-receipt")?;
    let queue_receipt_ref = record_optional_ref_string(&fields[6], "queue-receipt")?;
    let control_receipt_ref = record_optional_ref_string(&fields[7], "control-receipt")?;
    let envelope_ref = record_optional_ref_string(&fields[8], "envelope")?;
    let operation_ref = record_optional_ref_string(&fields[9], "operation")?;
    let request_ref = record_optional_ref_string(&fields[10], "request")?;
    let _checks = record_sequence_len(&fields[12], "checks")?;
    let decision = record_string(&fields[1], "decision")?;
    validate_decision(&decision)?;
    Ok(ControlLiveWorkflowBundleReconcileReceipt {
        receipt_ref: crate::preserves_rail::canonical_hash(value)?,
        decision,
        apply_receipt_ref: record_ref_string(&fields[2], "apply-receipt")?,
        bundle_ref: record_ref_string(&fields[3], "bundle")?,
        send_receipt_ref,
        ingress_receipt_ref,
        queue_receipt_ref,
        control_receipt_ref,
        envelope_ref,
        operation_ref,
        request_ref,
        diagnostics: record_strings(&fields[11], "diagnostics")?,
    })
}

pub fn parse_control_ingress_receipt(value: &IoValue) -> Result<ControlIngressReceipt> {
    let fields = value
        .collect_simple_record("node-control-ingress-receipt-v1", Some(15))
        .ok_or_else(|| MoltenError::invalid_harness("expected <node-control-ingress-receipt-v1 ...>"))?;
    require_schema(
        &fields[0],
        crate::preserves_rail::NODE_CONTROL_INGRESS_RECEIPT_SCHEMA,
        "node control ingress receipt",
    )?;
    let idempotency_receipt_ref = record_optional_ref_string(&fields[11], "idempotency")?;
    let queue_receipt_ref = record_optional_ref_string(&fields[12], "queue")?;
    let _checks = record_sequence_len(&fields[14], "checks")?;
    let decision = record_string(&fields[1], "decision")?;
    validate_decision(&decision)?;
    Ok(ControlIngressReceipt {
        receipt_ref: crate::preserves_rail::canonical_hash(value)?,
        decision,
        phase: record_string(&fields[2], "phase")?,
        transport: record_string(&fields[3], "transport")?,
        topic: record_string(&fields[4], "topic")?,
        from_peer: record_string(&fields[5], "from-peer")?,
        to_node: record_string(&fields[6], "to-node")?,
        sequence: record_u64_string(&fields[7], "sequence")?,
        envelope_ref: record_ref_string(&fields[8], "envelope")?,
        operation_ref: record_ref_string(&fields[9], "operation")?,
        request_ref: record_ref_string(&fields[10], "request")?,
        idempotency_receipt_ref,
        queue_receipt_ref,
        diagnostics: record_strings(&fields[13], "diagnostics")?,
    })
}

pub fn parse_control_queue_receipt(value: &IoValue) -> Result<ControlQueueReceipt> {
    let fields = value
        .collect_simple_record("node-control-queue-receipt-v1", Some(9))
        .ok_or_else(|| MoltenError::invalid_harness("expected <node-control-queue-receipt-v1 ...>"))?;
    require_schema(&fields[0], crate::preserves_rail::NODE_CONTROL_QUEUE_RECEIPT_SCHEMA, "node control queue receipt")?;
    let _checks = record_sequence_len(&fields[8], "checks")?;
    let decision = record_string(&fields[1], "decision")?;
    validate_decision(&decision)?;
    Ok(ControlQueueReceipt {
        receipt_ref: crate::preserves_rail::canonical_hash(value)?,
        decision,
        phase: record_string(&fields[2], "phase")?,
        operation: record_string(&fields[3], "operation")?,
        request_ref: record_ref_string(&fields[4], "request")?,
        location_ref: record_ref_string(&fields[6], "location")?,
        diagnostics: record_strings(&fields[7], "diagnostics")?,
    })
}

pub fn parse_control_live_workflow_bundle_verify_receipt(
    value: &IoValue,
) -> Result<ControlLiveWorkflowBundleVerifyReceipt> {
    let fields = value
        .collect_simple_record("node-control-live-workflow-bundle-verify-receipt-v1", Some(10))
        .ok_or_else(|| {
            MoltenError::invalid_harness("expected <node-control-live-workflow-bundle-verify-receipt-v1 ...>")
        })?;
    require_schema(
        &fields[0],
        crate::preserves_rail::NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_VERIFY_RECEIPT_SCHEMA,
        "node control live workflow bundle verify receipt",
    )?;
    let ticket_ref = record_optional_string(&fields[3], "ticket")?;
    let peer_admission_ref = record_optional_string(&fields[4], "peer-admission")?;
    let authority_grant_ref = record_optional_string(&fields[5], "authority-grant")?;
    for (reference, label) in [
        (ticket_ref.as_deref(), "node control live workflow bundle verify ticket ref"),
        (peer_admission_ref.as_deref(), "node control live workflow bundle verify peer admission ref"),
        (authority_grant_ref.as_deref(), "node control live workflow bundle verify authority grant ref"),
    ] {
        if let Some(reference) = reference {
            validate_ingress_ref(reference, label)?;
        }
    }
    let _expected = record_value(&fields[7], "expected")?;
    let _checks = record_sequence_len(&fields[9], "checks")?;
    let decision = record_string(&fields[1], "decision")?;
    validate_decision(&decision)?;
    Ok(ControlLiveWorkflowBundleVerifyReceipt {
        receipt_ref: crate::preserves_rail::canonical_hash(value)?,
        decision,
        bundle_ref: record_ref_string(&fields[2], "bundle")?,
        ticket_ref,
        peer_admission_ref,
        authority_grant_ref,
        receipt_refs: record_ref_strings(&fields[6], "receipts")?,
        diagnostics: record_strings(&fields[8], "diagnostics")?,
    })
}

pub fn parse_control_live_workflow_bundle_gate_receipt(
    value: &IoValue,
) -> Result<ControlLiveWorkflowBundleGateReceipt> {
    let fields = value
        .collect_simple_record("node-control-live-workflow-bundle-gate-receipt-v1", Some(12))
        .ok_or_else(|| {
            MoltenError::invalid_harness("expected <node-control-live-workflow-bundle-gate-receipt-v1 ...>")
        })?;
    require_schema(
        &fields[0],
        crate::preserves_rail::NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_GATE_RECEIPT_SCHEMA,
        "node control live workflow bundle gate receipt",
    )?;
    let verify_receipt_ref = record_optional_string(&fields[3], "verify-receipt")?;
    let ticket_ref = record_optional_string(&fields[5], "ticket")?;
    let peer_admission_ref = record_optional_string(&fields[6], "peer-admission")?;
    let authority_grant_ref = record_optional_string(&fields[7], "authority-grant")?;
    for (reference, label) in [
        (verify_receipt_ref.as_deref(), "node control live workflow bundle gate verify receipt ref"),
        (ticket_ref.as_deref(), "node control live workflow bundle gate ticket ref"),
        (peer_admission_ref.as_deref(), "node control live workflow bundle gate peer admission ref"),
        (authority_grant_ref.as_deref(), "node control live workflow bundle gate authority grant ref"),
    ] {
        if let Some(reference) = reference {
            validate_ingress_ref(reference, label)?;
        }
    }
    let _expected = record_value(&fields[9], "expected")?;
    let _checks = record_sequence_len(&fields[11], "checks")?;
    let decision = record_string(&fields[1], "decision")?;
    validate_decision(&decision)?;
    Ok(ControlLiveWorkflowBundleGateReceipt {
        receipt_ref: crate::preserves_rail::canonical_hash(value)?,
        decision,
        bundle_ref: record_ref_string(&fields[2], "bundle")?,
        verify_receipt_ref,
        recomputed_verify_receipt_ref: record_ref_string(&fields[4], "recomputed-verify")?,
        ticket_ref,
        peer_admission_ref,
        authority_grant_ref,
        receipt_refs: record_ref_strings(&fields[8], "receipts")?,
        diagnostics: record_strings(&fields[10], "diagnostics")?,
    })
}

pub fn parse_control_live_workflow_bundle(value: &IoValue) -> Result<ControlLiveWorkflowBundle> {
    let fields = value
        .collect_simple_record("node-control-live-workflow-bundle-v1", Some(10))
        .ok_or_else(|| MoltenError::invalid_harness("expected <node-control-live-workflow-bundle-v1 ...>"))?;
    require_schema(
        &fields[0],
        crate::preserves_rail::NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_SCHEMA,
        "node control live workflow bundle",
    )?;
    let ticket_value = record_value(&fields[1], "ticket")?;
    let peer_admission_value = record_value(&fields[2], "peer-admission")?;
    let authority_grant_value = record_value(&fields[3], "authority-grant")?;
    let receipt_values = record_values(&fields[4], "receipts")?;
    let ticket_ref = record_ref_string(&fields[5], "ticket-ref")?;
    let peer_admission_ref = record_ref_string(&fields[6], "peer-admission-ref")?;
    let authority_grant_ref = record_ref_string(&fields[7], "authority-grant-ref")?;
    let receipt_refs = record_ref_strings(&fields[8], "receipt-refs")?;
    let parsed_ticket = parse_control_live_ticket(&ticket_value)?;
    let parsed_admission = parse_control_live_peer_admission(&peer_admission_value)?;
    let parsed_authority = parse_control_authority_grant(&authority_grant_value)?;
    if parsed_ticket.ticket_ref != ticket_ref {
        return Err(MoltenError::invalid_harness("node control live workflow bundle ticket ref mismatch"));
    }
    if parsed_admission.admission_ref != peer_admission_ref {
        return Err(MoltenError::invalid_harness("node control live workflow bundle peer admission ref mismatch"));
    }
    if parsed_authority.grant_ref != authority_grant_ref {
        return Err(MoltenError::invalid_harness("node control live workflow bundle authority grant ref mismatch"));
    }
    let parsed_receipt_refs = live_workflow_bundle_receipt_refs_from_values(&receipt_values)?;
    if parsed_receipt_refs != receipt_refs {
        return Err(MoltenError::invalid_harness("node control live workflow bundle receipt refs mismatch"));
    }
    Ok(ControlLiveWorkflowBundle {
        bundle_ref: crate::preserves_rail::canonical_hash(value)?,
        bundle_value: value.clone(),
        ticket_ref,
        peer_admission_ref,
        authority_grant_ref,
        receipt_refs,
        ticket_value,
        peer_admission_value,
        authority_grant_value,
        receipt_values,
    })
}

struct AckParts {
    apply_receipt_ref: String,
    send_receipt_ref: Option<String>,
    ingress_receipt_ref: Option<String>,
    queue_receipt_ref: Option<String>,
    control_receipt_ref: Option<String>,
    reconcile_receipt_ref: String,
    bundle_ref: String,
    envelope_ref: Option<String>,
    operation_ref: Option<String>,
    request_ref: Option<String>,
    receiver_decision: String,
    receiver_diagnostics: Vec<String>,
    diagnostics: Vec<String>,
    apply_receipt_value: IoValue,
    send_receipt_value: Option<IoValue>,
    ingress_receipt_value: Option<IoValue>,
    queue_receipt_value: Option<IoValue>,
    control_receipt_value: Option<IoValue>,
    reconcile_receipt_value: IoValue,
}

impl AckParts {
    fn into_ack(self, value: &IoValue) -> Result<ControlLiveWorkflowBundleAck> {
        Ok(ControlLiveWorkflowBundleAck {
            ack_ref: crate::preserves_rail::canonical_hash(value)?,
            ack_value: value.clone(),
            apply_receipt_ref: self.apply_receipt_ref,
            send_receipt_ref: self.send_receipt_ref,
            ingress_receipt_ref: self.ingress_receipt_ref,
            queue_receipt_ref: self.queue_receipt_ref,
            control_receipt_ref: self.control_receipt_ref,
            reconcile_receipt_ref: self.reconcile_receipt_ref,
            bundle_ref: self.bundle_ref,
            envelope_ref: self.envelope_ref,
            operation_ref: self.operation_ref,
            request_ref: self.request_ref,
            receiver_decision: self.receiver_decision,
            receiver_diagnostics: self.receiver_diagnostics,
            diagnostics: self.diagnostics,
            apply_receipt_value: self.apply_receipt_value,
            send_receipt_value: self.send_receipt_value,
            ingress_receipt_value: self.ingress_receipt_value,
            queue_receipt_value: self.queue_receipt_value,
            control_receipt_value: self.control_receipt_value,
            reconcile_receipt_value: self.reconcile_receipt_value,
        })
    }
}

fn validate_ack_members(parts: &AckParts) -> Result<()> {
    let apply = parse_control_live_workflow_bundle_apply_receipt(&parts.apply_receipt_value)?;
    let reconcile = parse_control_live_workflow_bundle_reconcile_receipt(&parts.reconcile_receipt_value)?;
    if let Some(value) = parts.send_receipt_value.as_ref() {
        parse_control_live_send_receipt(value)?;
    }
    if let Some(value) = parts.ingress_receipt_value.as_ref() {
        parse_control_ingress_receipt(value)?;
    }
    if let Some(value) = parts.queue_receipt_value.as_ref() {
        parse_control_queue_receipt(value)?;
    }
    if let Some(value) = parts.control_receipt_value.as_ref() {
        crate::node_runtime::parse_control_receipt(value)?;
    }
    validate_member_ref(&apply.receipt_ref, &parts.apply_receipt_ref, "ack apply receipt")?;
    validate_member_ref(&reconcile.receipt_ref, &parts.reconcile_receipt_ref, "ack reconcile receipt")?;
    validate_optional_member_ref(
        parts.send_receipt_value.as_ref(),
        parts.send_receipt_ref.as_deref(),
        "ack send receipt",
    )?;
    validate_optional_member_ref(
        parts.ingress_receipt_value.as_ref(),
        parts.ingress_receipt_ref.as_deref(),
        "ack ingress receipt",
    )?;
    validate_optional_member_ref(
        parts.queue_receipt_value.as_ref(),
        parts.queue_receipt_ref.as_deref(),
        "ack queue receipt",
    )?;
    validate_optional_member_ref(
        parts.control_receipt_value.as_ref(),
        parts.control_receipt_ref.as_deref(),
        "ack control receipt",
    )?;
    validate_ack_reconcile(parts, &reconcile)
}

fn validate_ack_reconcile(parts: &AckParts, reconcile: &ControlLiveWorkflowBundleReconcileReceipt) -> Result<()> {
    if reconcile.apply_receipt_ref != parts.apply_receipt_ref {
        return Err(MoltenError::invalid_harness("node control live workflow bundle ack apply ref mismatch"));
    }
    if reconcile.bundle_ref != parts.bundle_ref {
        return Err(MoltenError::invalid_harness("node control live workflow bundle ack bundle ref mismatch"));
    }
    if reconcile.send_receipt_ref != parts.send_receipt_ref {
        return Err(MoltenError::invalid_harness("node control live workflow bundle ack send ref mismatch"));
    }
    if reconcile.ingress_receipt_ref != parts.ingress_receipt_ref {
        return Err(MoltenError::invalid_harness("node control live workflow bundle ack ingress ref mismatch"));
    }
    if reconcile.queue_receipt_ref != parts.queue_receipt_ref {
        return Err(MoltenError::invalid_harness("node control live workflow bundle ack queue ref mismatch"));
    }
    if reconcile.control_receipt_ref != parts.control_receipt_ref {
        return Err(MoltenError::invalid_harness("node control live workflow bundle ack control ref mismatch"));
    }
    if reconcile.envelope_ref != parts.envelope_ref {
        return Err(MoltenError::invalid_harness("node control live workflow bundle ack envelope ref mismatch"));
    }
    if reconcile.operation_ref != parts.operation_ref {
        return Err(MoltenError::invalid_harness("node control live workflow bundle ack operation ref mismatch"));
    }
    if reconcile.request_ref != parts.request_ref {
        return Err(MoltenError::invalid_harness("node control live workflow bundle ack request ref mismatch"));
    }
    if reconcile.decision != parts.receiver_decision {
        return Err(MoltenError::invalid_harness("node control live workflow bundle ack receiver decision mismatch"));
    }
    if reconcile.diagnostics != parts.receiver_diagnostics {
        return Err(MoltenError::invalid_harness(
            "node control live workflow bundle ack receiver diagnostics mismatch",
        ));
    }
    Ok(())
}

pub fn parse_control_live_workflow_bundle_ack(value: &IoValue) -> Result<ControlLiveWorkflowBundleAck> {
    let fields = value
        .collect_simple_record("node-control-live-workflow-bundle-ack-v1", Some(22))
        .ok_or_else(|| MoltenError::invalid_harness("expected <node-control-live-workflow-bundle-ack-v1 ...>"))?;
    require_schema(
        &fields[0],
        crate::preserves_rail::NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_ACK_SCHEMA,
        "node control live workflow bundle ack",
    )?;
    let receiver_decision = record_string(&fields[17], "receiver-decision")?;
    validate_decision(&receiver_decision)?;
    let _checks = record_sequence_len(&fields[20], "checks")?;
    let _member_refs = record_sequence_len(&fields[21], "member-refs")?;
    let parts = AckParts {
        apply_receipt_value: record_value(&fields[1], "apply-receipt")?,
        send_receipt_value: record_optional_value(&fields[2], "send-receipt")?,
        ingress_receipt_value: record_optional_value(&fields[3], "ingress-receipt")?,
        queue_receipt_value: record_optional_value(&fields[4], "queue-receipt")?,
        control_receipt_value: record_optional_value(&fields[5], "control-receipt")?,
        reconcile_receipt_value: record_value(&fields[6], "reconcile-receipt")?,
        apply_receipt_ref: record_ref_string(&fields[7], "apply-ref")?,
        send_receipt_ref: record_optional_ref_string(&fields[8], "send-ref")?,
        ingress_receipt_ref: record_optional_ref_string(&fields[9], "ingress-ref")?,
        queue_receipt_ref: record_optional_ref_string(&fields[10], "queue-ref")?,
        control_receipt_ref: record_optional_ref_string(&fields[11], "control-ref")?,
        reconcile_receipt_ref: record_ref_string(&fields[12], "reconcile-ref")?,
        bundle_ref: record_ref_string(&fields[13], "bundle")?,
        envelope_ref: record_optional_ref_string(&fields[14], "envelope")?,
        operation_ref: record_optional_ref_string(&fields[15], "operation")?,
        request_ref: record_optional_ref_string(&fields[16], "request")?,
        receiver_decision,
        receiver_diagnostics: record_strings(&fields[18], "receiver-diagnostics")?,
        diagnostics: record_strings(&fields[19], "diagnostics")?,
    };
    validate_ack_members(&parts)?;
    parts.into_ack(value)
}

pub fn import_control_live_workflow_bundle(
    input: &ControlLiveWorkflowBundleImportInput<'_>,
) -> Result<ControlLiveWorkflowBundleImport> {
    validate_state_root(input.state_root)?;
    ensure_state_layout(input.state_root)?;
    if let Some(node) = input.expected_node {
        validate_node_id(node)?;
    }
    if let Some(topic) = input.expected_topic {
        validate_node_id(topic)?;
    }
    if let Some(endpoint) = input.expected_endpoint {
        validate_node_id(endpoint)?;
    }
    if let Some(peer) = input.expected_peer {
        validate_node_id(peer)?;
    }
    for operation in input.expected_operations {
        validate_node_id(operation)?;
    }
    if let Some(scope) = input.expected_target_scope {
        validate_node_id(scope)?;
    }
    if let Some(scope) = input.expected_resource_scope {
        validate_node_id(scope)?;
    }
    let bundle = parse_control_live_workflow_bundle(input.bundle_value)?;
    let ticket = parse_control_live_ticket(&bundle.ticket_value)?;
    let admission = parse_control_live_peer_admission(&bundle.peer_admission_value)?;
    let authority = parse_control_authority_grant(&bundle.authority_grant_value)?;
    let mut diagnostics = live_workflow_bundle_import_diagnostics(input, &ticket, &admission, &authority);
    let mut parts = ImportParts {
        imported_refs: Vec::with_capacity(bundle.receipt_values.len().saturating_add(5)),
        ticket_import_ref: None,
        authority_import_ref: None,
    };
    if diagnostics.is_empty() {
        let (imported, import_diagnostics) = import_parts(input, &bundle)?;
        parts = imported;
        diagnostics.extend(import_diagnostics);
    }
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let receipt_value = live_workflow_bundle_import_receipt_value(&LiveWorkflowBundleImportReceiptValueInput {
        decision,
        state_root: input.state_root,
        bundle: &bundle,
        ticket_import_ref: parts.ticket_import_ref.as_deref(),
        authority_import_ref: parts.authority_import_ref.as_deref(),
        imported_refs: &parts.imported_refs,
        diagnostics: &diagnostics,
    })?;
    let receipt_ref = crate::preserves_rail::canonical_hash(&receipt_value)?;
    import_artifact(input.state_root, &receipt_value)?;
    Ok(ControlLiveWorkflowBundleImport {
        bundle_ref: bundle.bundle_ref,
        ticket_import_ref: parts.ticket_import_ref,
        authority_import_ref: parts.authority_import_ref,
        imported_refs: parts.imported_refs,
        diagnostics,
        receipt_ref,
        receipt_value,
        decision: decision.to_string(),
    })
}

fn import_parts(
    input: &ControlLiveWorkflowBundleImportInput<'_>,
    bundle: &ControlLiveWorkflowBundle,
) -> Result<(ImportParts, Vec<String>)> {
    let mut diagnostics = Vec::new();
    let mut parts = ImportParts {
        imported_refs: Vec::with_capacity(bundle.receipt_values.len().saturating_add(5)),
        ticket_import_ref: None,
        authority_import_ref: None,
    };
    let ticket_import = import_control_live_ticket(&ControlLiveTicketImportInput {
        state_root: input.state_root,
        ticket_value: &bundle.ticket_value,
        peer_admission_value: Some(&bundle.peer_admission_value),
        expected_node: input.expected_node,
        expected_topic: input.expected_topic,
        expected_endpoint: input.expected_endpoint,
        expected_peer: input.expected_peer,
        as_of_sequence: input.as_of_sequence,
    })?;
    let authority_import = import_control_authority_grant_checked(&ControlAuthorityGrantImportInput {
        state_root: input.state_root,
        grant_value: &bundle.authority_grant_value,
        expected_peer: input.expected_peer,
        expected_node: input.expected_node,
        expected_operations: input.expected_operations,
        expected_target_scope: input.expected_target_scope,
        expected_resource_scope: input.expected_resource_scope,
        as_of_epoch: input.as_of_epoch,
    })?;
    parts.ticket_import_ref = Some(ticket_import.receipt_ref.clone());
    parts.authority_import_ref = Some(authority_import.receipt_ref.clone());
    if ticket_import.decision != "pass" {
        diagnostics.extend(ticket_import.diagnostics.iter().cloned());
    }
    if authority_import.decision != "pass" {
        diagnostics.extend(authority_import.diagnostics.iter().cloned());
    }
    if diagnostics.is_empty() {
        parts.imported_refs.extend(ticket_import.imported_refs);
        parts.imported_refs.extend(authority_import.imported_refs);
        parts.imported_refs.push(import_artifact(input.state_root, input.bundle_value)?);
        for receipt_value in &bundle.receipt_values {
            parts.imported_refs.push(import_artifact(input.state_root, receipt_value)?);
        }
    }
    Ok((parts, diagnostics))
}

fn validate_live_workflow_bundle_verify_input(input: &ControlLiveWorkflowBundleVerifyInput<'_>) -> Result<()> {
    if let Some(node) = input.expected_node {
        validate_node_id(node)?;
    }
    if let Some(topic) = input.expected_topic {
        validate_node_id(topic)?;
    }
    if let Some(endpoint) = input.expected_endpoint {
        validate_node_id(endpoint)?;
    }
    if let Some(peer) = input.expected_peer {
        validate_node_id(peer)?;
    }
    for operation in input.expected_operations {
        validate_node_id(operation)?;
    }
    if let Some(scope) = input.expected_target_scope {
        validate_node_id(scope)?;
    }
    if let Some(scope) = input.expected_resource_scope {
        validate_node_id(scope)?;
    }
    Ok(())
}

fn validate_live_workflow_bundle_apply_input(input: &ControlLiveWorkflowBundleApplyInput<'_>) -> Result<()> {
    validate_state_root(input.state_root)?;
    validate_live_workflow_bundle_verify_input(&live_workflow_bundle_verify_input_from_apply(input))?;
    if let Some(from_peer) = input.from_peer {
        validate_node_id(from_peer)?;
    }
    if let Some(operation_ref) = input.expected_operation_ref {
        validate_ingress_ref(operation_ref, "node control live workflow bundle apply operation id")?;
    }
    validate_ingress_refs(input.peer_bootstrap_refs, "node control live workflow bundle apply peer bootstrap ref")?;
    validate_ingress_refs(input.authority_refs, "node control live workflow bundle apply authority ref")?;
    validate_ingress_refs(input.policy_refs, "node control live workflow bundle apply policy ref")?;
    validate_ingress_refs(input.resource_refs, "node control live workflow bundle apply resource ref")?;
    validate_ingress_refs(input.evidence_refs, "node control live workflow bundle apply evidence ref")?;
    if input.request_value.is_some() || input.should_send {
        validate_live_send_timeout(input.join_timeout_ms)?;
        validate_live_send_attempts(input.max_attempts)?;
    }
    Ok(())
}

fn live_workflow_bundle_expected_input_from_verify<'a>(
    input: &'a ControlLiveWorkflowBundleVerifyInput<'a>,
) -> LiveWorkflowBundleExpectedInput<'a> {
    LiveWorkflowBundleExpectedInput {
        expected_node: input.expected_node,
        expected_topic: input.expected_topic,
        expected_endpoint: input.expected_endpoint,
        expected_peer: input.expected_peer,
        expected_operations: input.expected_operations,
        expected_target_scope: input.expected_target_scope,
        expected_resource_scope: input.expected_resource_scope,
        as_of_sequence: input.as_of_sequence,
        as_of_epoch: input.as_of_epoch,
    }
}

fn live_workflow_bundle_verify_input_from_gate<'a>(
    input: &'a ControlLiveWorkflowBundleGateInput<'a>,
) -> ControlLiveWorkflowBundleVerifyInput<'a> {
    ControlLiveWorkflowBundleVerifyInput {
        bundle_value: input.bundle_value,
        expected_node: input.expected_node,
        expected_topic: input.expected_topic,
        expected_endpoint: input.expected_endpoint,
        expected_peer: input.expected_peer,
        expected_operations: input.expected_operations,
        expected_target_scope: input.expected_target_scope,
        expected_resource_scope: input.expected_resource_scope,
        as_of_sequence: input.as_of_sequence,
        as_of_epoch: input.as_of_epoch,
    }
}

fn live_workflow_bundle_verify_input_from_apply<'a>(
    input: &'a ControlLiveWorkflowBundleApplyInput<'a>,
) -> ControlLiveWorkflowBundleVerifyInput<'a> {
    ControlLiveWorkflowBundleVerifyInput {
        bundle_value: input.bundle_value,
        expected_node: input.expected_node,
        expected_topic: input.expected_topic,
        expected_endpoint: input.expected_endpoint,
        expected_peer: input.expected_peer,
        expected_operations: input.expected_operations,
        expected_target_scope: input.expected_target_scope,
        expected_resource_scope: input.expected_resource_scope,
        as_of_sequence: input.as_of_sequence,
        as_of_epoch: input.as_of_epoch,
    }
}

fn live_workflow_bundle_import_input_from_apply<'a>(
    input: &'a ControlLiveWorkflowBundleApplyInput<'a>,
) -> ControlLiveWorkflowBundleImportInput<'a> {
    ControlLiveWorkflowBundleImportInput {
        state_root: input.state_root,
        bundle_value: input.bundle_value,
        expected_node: input.expected_node,
        expected_topic: input.expected_topic,
        expected_endpoint: input.expected_endpoint,
        expected_peer: input.expected_peer,
        expected_operations: input.expected_operations,
        expected_target_scope: input.expected_target_scope,
        expected_resource_scope: input.expected_resource_scope,
        as_of_sequence: input.as_of_sequence,
        as_of_epoch: input.as_of_epoch,
    }
}

fn live_workflow_bundle_expected_input_from_import<'a>(
    input: &'a ControlLiveWorkflowBundleImportInput<'a>,
) -> LiveWorkflowBundleExpectedInput<'a> {
    LiveWorkflowBundleExpectedInput {
        expected_node: input.expected_node,
        expected_topic: input.expected_topic,
        expected_endpoint: input.expected_endpoint,
        expected_peer: input.expected_peer,
        expected_operations: input.expected_operations,
        expected_target_scope: input.expected_target_scope,
        expected_resource_scope: input.expected_resource_scope,
        as_of_sequence: input.as_of_sequence,
        as_of_epoch: input.as_of_epoch,
    }
}

fn live_workflow_bundle_import_diagnostics(
    input: &ControlLiveWorkflowBundleImportInput<'_>,
    ticket: &ControlLiveTicket,
    admission: &ControlLivePeerAdmission,
    authority: &ControlAuthorityGrant,
) -> Vec<String> {
    live_workflow_bundle_expected_diagnostics(
        &live_workflow_bundle_expected_input_from_import(input),
        ticket,
        admission,
        authority,
    )
}

fn live_workflow_bundle_expected_diagnostics(
    input: &LiveWorkflowBundleExpectedInput<'_>,
    ticket: &ControlLiveTicket,
    admission: &ControlLivePeerAdmission,
    authority: &ControlAuthorityGrant,
) -> Vec<String> {
    let mut diagnostics = live_workflow_bundle_binding_diagnostics(ticket, admission, authority);
    diagnostics.extend(live_ticket_expected_diagnostics(input, ticket, admission));
    diagnostics.extend(authority_grant_expected_diagnostics(input, authority));
    diagnostics
}

fn live_ticket_expected_diagnostics(
    input: &LiveWorkflowBundleExpectedInput<'_>,
    ticket: &ControlLiveTicket,
    admission: &ControlLivePeerAdmission,
) -> Vec<String> {
    live_ticket_import_diagnostics(
        &ControlLiveTicketImportInput {
            state_root: Path::new("."),
            ticket_value: &ticket.value,
            peer_admission_value: Some(&admission.value),
            expected_node: input.expected_node,
            expected_topic: input.expected_topic,
            expected_endpoint: input.expected_endpoint,
            expected_peer: input.expected_peer,
            as_of_sequence: input.as_of_sequence,
        },
        ticket,
        Some(admission),
    )
}

fn authority_grant_expected_diagnostics(
    input: &LiveWorkflowBundleExpectedInput<'_>,
    authority: &ControlAuthorityGrant,
) -> Vec<String> {
    authority_grant_import_diagnostics(
        &ControlAuthorityGrantImportInput {
            state_root: Path::new("."),
            grant_value: &authority.value,
            expected_peer: input.expected_peer,
            expected_node: input.expected_node,
            expected_operations: input.expected_operations,
            expected_target_scope: input.expected_target_scope,
            expected_resource_scope: input.expected_resource_scope,
            as_of_epoch: input.as_of_epoch,
        },
        authority,
    )
}

fn live_workflow_bundle_binding_diagnostics(
    ticket: &ControlLiveTicket,
    admission: &ControlLivePeerAdmission,
    authority: &ControlAuthorityGrant,
) -> Vec<String> {
    let mut diagnostics = Vec::with_capacity(8);
    if admission.decision != "pass" {
        diagnostics.push(format!(
            "node control live workflow bundle peer admission {} decision {}",
            admission.admission_ref, admission.decision
        ));
    }
    if admission.ticket_ref != ticket.ticket_ref {
        diagnostics.push("node control live workflow bundle admission does not bind ticket".to_string());
    }
    if admission.node_id != ticket.node_id {
        diagnostics.push("node control live workflow bundle admission node does not match ticket".to_string());
    }
    if admission.topic != ticket.topic {
        diagnostics.push("node control live workflow bundle admission topic does not match ticket".to_string());
    }
    if authority.peer_id != admission.peer_id {
        diagnostics.push("node control live workflow bundle authority peer does not match admission".to_string());
    }
    if authority.node_id != ticket.node_id {
        diagnostics.push("node control live workflow bundle authority node does not match ticket".to_string());
    }
    if !authority.revocation_refs.is_empty() {
        diagnostics.push("node control live workflow bundle authority grant has revocation refs".to_string());
    }
    diagnostics
}

fn live_workflow_bundle_receipt_refs(values: &[&IoValue]) -> Result<Vec<String>> {
    let owned_values = values.iter().map(|value| (**value).clone()).collect::<Vec<_>>();
    live_workflow_bundle_receipt_refs_from_values(&owned_values)
}

fn live_workflow_bundle_receipt_refs_from_values(values: &[IoValue]) -> Result<Vec<String>> {
    let mut refs = Vec::with_capacity(values.len());
    for value in values {
        refs.push(crate::preserves_rail::canonical_hash(value)?);
    }
    Ok(refs)
}

fn live_workflow_bundle_receipt_diagnostics(values: &[&IoValue]) -> Vec<String> {
    let mut diagnostics = Vec::with_capacity(values.len());
    for value in values {
        let kind = crate::ledger::artifact_kind(value);
        if !is_live_workflow_bundle_receipt_kind(kind) {
            diagnostics.push(format!("node control live workflow bundle unsupported receipt kind {kind}"));
        }
    }
    diagnostics
}

fn is_live_workflow_bundle_receipt_kind(kind: &str) -> bool {
    matches!(
        kind,
        "node-control-live-ticket-import-receipt"
            | "node-control-authority-grant-import-receipt"
            | "node-control-live-send-receipt"
            | "node-control-live-send-retry-receipt"
            | "node-control-live-send-duplicate-receipt"
            | "node-control-live-workflow-receipt"
            | "node-control-live-workflow-bundle-verify-receipt"
            | "node-control-live-workflow-bundle-gate-receipt"
            | "node-control-live-workflow-bundle-apply-receipt"
            | "node-control-live-workflow-bundle-reconcile-receipt"
            | "node-control-live-transport-receipt"
            | "node-control-live-listener-receipt"
            | "node-control-service-run-receipt"
    )
}

fn live_ticket_import_diagnostics(
    input: &ControlLiveTicketImportInput<'_>,
    ticket: &ControlLiveTicket,
    admission: Option<&ControlLivePeerAdmission>,
) -> Vec<String> {
    let mut diagnostics = Vec::with_capacity(8);
    if let Some(expected) = input.expected_node
        && ticket.node_id != expected
    {
        diagnostics.push(format!(
            "node control live ticket import node {} does not match expected {expected}",
            ticket.node_id
        ));
    }
    if let Some(expected) = input.expected_topic
        && ticket.topic != expected
    {
        diagnostics
            .push(format!("node control live ticket import topic {} does not match expected {expected}", ticket.topic));
    }
    if let Some(expected) = input.expected_endpoint
        && ticket.live_endpoint_id != expected
    {
        diagnostics.push(format!(
            "node control live ticket import endpoint {} does not match expected {expected}",
            ticket.live_endpoint_id
        ));
    }
    if let Some(admission) = admission {
        diagnostics.extend(live_ticket_admission_import_diagnostics(input, ticket, admission));
    }
    diagnostics
}

fn live_ticket_admission_import_diagnostics(
    input: &ControlLiveTicketImportInput<'_>,
    ticket: &ControlLiveTicket,
    admission: &ControlLivePeerAdmission,
) -> Vec<String> {
    let mut diagnostics = Vec::with_capacity(7);
    if admission.decision != "pass" {
        diagnostics.push(format!(
            "node control live peer admission {} decision {}",
            admission.admission_ref, admission.decision
        ));
    }
    if admission.ticket_ref != ticket.ticket_ref {
        diagnostics.push(format!(
            "node control live peer admission {} ticket {} does not match ticket {}",
            admission.admission_ref, admission.ticket_ref, ticket.ticket_ref
        ));
    }
    if admission.node_id != ticket.node_id {
        diagnostics.push(format!(
            "node control live peer admission {} node {} does not match ticket node {}",
            admission.admission_ref, admission.node_id, ticket.node_id
        ));
    }
    if admission.topic != ticket.topic {
        diagnostics.push(format!(
            "node control live peer admission {} topic {} does not match ticket topic {}",
            admission.admission_ref, admission.topic, ticket.topic
        ));
    }
    if let Some(expected) = input.expected_peer
        && admission.peer_id != expected
    {
        diagnostics.push(format!(
            "node control live peer admission {} peer {} does not match expected {expected}",
            admission.admission_ref, admission.peer_id
        ));
    }
    if admission.sequence > input.as_of_sequence {
        diagnostics.push(format!(
            "node control live peer admission {} is not valid until sequence {}",
            admission.admission_ref, admission.sequence
        ));
    }
    if let Some(expires_at) = admission.expires_at
        && expires_at < input.as_of_sequence
    {
        diagnostics.push(format!(
            "node control live peer admission {} expired at sequence {expires_at}",
            admission.admission_ref
        ));
    }
    diagnostics
}

fn authority_grant_import_diagnostics(
    input: &ControlAuthorityGrantImportInput<'_>,
    grant: &ControlAuthorityGrant,
) -> Vec<String> {
    let mut diagnostics = Vec::with_capacity(8);
    if let Some(expected) = input.expected_peer
        && grant.peer_id != expected
    {
        diagnostics.push(format!(
            "node control authority grant import peer {} does not match expected {expected}",
            grant.peer_id
        ));
    }
    if let Some(expected) = input.expected_node
        && grant.node_id != expected
    {
        diagnostics.push(format!(
            "node control authority grant import node {} does not match expected {expected}",
            grant.node_id
        ));
    }
    for operation in input.expected_operations {
        if !grant.operations.iter().any(|candidate| candidate == "*" || candidate == operation) {
            diagnostics.push(format!("node control authority grant import does not allow operation {operation}"));
        }
    }
    if let Some(expected) = input.expected_target_scope
        && grant.target_scope != "*"
        && grant.target_scope != expected
    {
        diagnostics.push(format!(
            "node control authority grant import target scope {} does not cover expected {expected}",
            grant.target_scope
        ));
    }
    if let Some(expected) = input.expected_resource_scope
        && grant.resource_scope != "*"
        && grant.resource_scope != expected
    {
        diagnostics.push(format!(
            "node control authority grant import resource scope {} does not cover expected {expected}",
            grant.resource_scope
        ));
    }
    if grant.epoch > input.as_of_epoch {
        diagnostics.push(format!("node control authority grant import is not valid until epoch {}", grant.epoch));
    }
    if let Some(expires_at) = grant.expires_at
        && expires_at < input.as_of_epoch
    {
        diagnostics.push(format!("node control authority grant import expired at epoch {expires_at}"));
    }
    if !grant.revocation_refs.is_empty() {
        diagnostics.push("node control authority grant import has revocation refs".to_string());
    }
    diagnostics
}

fn live_ticket_import_receipt_value(input: &LiveTicketImportReceiptValueInput<'_>) -> Result<IoValue> {
    validate_decision(input.decision)?;
    let binding_status = if input.decision == "pass" { "pass" } else { "fail" };
    Ok(crate::preserves_rail::record("node-control-live-ticket-import-receipt-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::NODE_CONTROL_LIVE_TICKET_IMPORT_RECEIPT_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(input.decision)]),
        crate::preserves_rail::record("state-root", vec![crate::preserves_rail::string(&state_root_profile_ref(
            input.state_root,
        )?)]),
        crate::preserves_rail::record("ticket", vec![crate::preserves_rail::string(&input.ticket.ticket_ref)]),
        crate::preserves_rail::record("node", vec![crate::preserves_rail::string(&input.ticket.node_id)]),
        crate::preserves_rail::record("topic", vec![crate::preserves_rail::string(&input.ticket.topic)]),
        crate::preserves_rail::record("endpoint", vec![crate::preserves_rail::string(&input.ticket.live_endpoint_id)]),
        crate::preserves_rail::record("peer-admission", vec![optional_string(input.peer_admission_ref)]),
        crate::preserves_rail::record("peer", vec![optional_string(input.peer_id)]),
        crate::preserves_rail::record("as-of-sequence", vec![crate::preserves_rail::string(
            input.as_of_sequence.to_string(),
        )]),
        crate::preserves_rail::record("imported", vec![crate::preserves_rail::sequence(
            input.imported_refs.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("diagnostics", vec![crate::preserves_rail::sequence(
            input.diagnostics.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("checks", vec![crate::preserves_rail::sequence(vec![
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("ticket-kind-version"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("ticket-topic-endpoint-bound"),
                crate::preserves_rail::string(binding_status),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("peer-admission-kind-version"),
                crate::preserves_rail::string(binding_status),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("import-receipt-is-not-authority"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("provenance-still-required"),
                crate::preserves_rail::string("pass"),
            ]),
        ])]),
    ]))
}

fn authority_grant_import_receipt_value(input: &AuthorityGrantImportReceiptValueInput<'_>) -> Result<IoValue> {
    validate_decision(input.decision)?;
    let binding_status = if input.decision == "pass" { "pass" } else { "fail" };
    Ok(crate::preserves_rail::record("node-control-authority-grant-import-receipt-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::NODE_CONTROL_AUTHORITY_GRANT_IMPORT_RECEIPT_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(input.decision)]),
        crate::preserves_rail::record("state-root", vec![crate::preserves_rail::string(&state_root_profile_ref(
            input.state_root,
        )?)]),
        crate::preserves_rail::record("grant", vec![crate::preserves_rail::string(&input.grant.grant_ref)]),
        crate::preserves_rail::record("peer", vec![crate::preserves_rail::string(&input.grant.peer_id)]),
        crate::preserves_rail::record("node", vec![crate::preserves_rail::string(&input.grant.node_id)]),
        crate::preserves_rail::record("operations", vec![crate::preserves_rail::sequence(
            input.grant.operations.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("target-scope", vec![crate::preserves_rail::string(&input.grant.target_scope)]),
        crate::preserves_rail::record("resource-scope", vec![crate::preserves_rail::string(
            &input.grant.resource_scope,
        )]),
        crate::preserves_rail::record("as-of-epoch", vec![crate::preserves_rail::string(
            input.as_of_epoch.to_string(),
        )]),
        crate::preserves_rail::record("imported", vec![crate::preserves_rail::sequence(
            input.imported_refs.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("diagnostics", vec![crate::preserves_rail::sequence(
            input.diagnostics.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("checks", vec![crate::preserves_rail::sequence(vec![
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("grant-kind-version"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("peer-node-operation-scope-bound"),
                crate::preserves_rail::string(binding_status),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("grant-fresh-and-unrevoked"),
                crate::preserves_rail::string(binding_status),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("import-receipt-is-not-authority"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("provenance-still-required"),
                crate::preserves_rail::string("pass"),
            ]),
        ])]),
    ]))
}

fn live_workflow_bundle_value(input: &LiveWorkflowBundleValueInput<'_>) -> Result<IoValue> {
    let binding_status = if input.diagnostics.is_empty() { "pass" } else { "fail" };
    let receipt_refs = live_workflow_bundle_receipt_refs(input.receipt_values)?;
    Ok(crate::preserves_rail::record("node-control-live-workflow-bundle-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_SCHEMA),
        crate::preserves_rail::record("ticket", vec![(*input.ticket_value).clone()]),
        crate::preserves_rail::record("peer-admission", vec![(*input.admission_value).clone()]),
        crate::preserves_rail::record("authority-grant", vec![(*input.authority_value).clone()]),
        crate::preserves_rail::record("receipts", vec![crate::preserves_rail::sequence(
            input.receipt_values.iter().map(|value| (**value).clone()).collect(),
        )]),
        crate::preserves_rail::record("ticket-ref", vec![crate::preserves_rail::string(&input.ticket.ticket_ref)]),
        crate::preserves_rail::record("peer-admission-ref", vec![crate::preserves_rail::string(
            &input.admission.admission_ref,
        )]),
        crate::preserves_rail::record("authority-grant-ref", vec![crate::preserves_rail::string(
            &input.authority.grant_ref,
        )]),
        crate::preserves_rail::record("receipt-refs", vec![crate::preserves_rail::sequence(
            receipt_refs.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("checks", vec![crate::preserves_rail::sequence(vec![
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("ticket-kind-version"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("peer-admission-kind-version"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("authority-grant-kind-version"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("ticket-admission-bound"),
                crate::preserves_rail::string(binding_status),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("authority-grant-bound"),
                crate::preserves_rail::string(binding_status),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("bundle-is-not-authority"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("provenance-still-required"),
                crate::preserves_rail::string("pass"),
            ]),
        ])]),
    ]))
}

fn live_workflow_bundle_export_receipt_value(input: &LiveWorkflowBundleExportReceiptValueInput<'_>) -> Result<IoValue> {
    validate_decision(input.decision)?;
    let binding_status = if input.decision == "pass" { "pass" } else { "fail" };
    Ok(crate::preserves_rail::record("node-control-live-workflow-bundle-export-receipt-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_EXPORT_RECEIPT_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(input.decision)]),
        crate::preserves_rail::record("bundle", vec![crate::preserves_rail::string(&input.bundle.bundle_ref)]),
        crate::preserves_rail::record("ticket", vec![crate::preserves_rail::string(&input.bundle.ticket_ref)]),
        crate::preserves_rail::record("peer-admission", vec![crate::preserves_rail::string(
            &input.bundle.peer_admission_ref,
        )]),
        crate::preserves_rail::record("authority-grant", vec![crate::preserves_rail::string(
            &input.bundle.authority_grant_ref,
        )]),
        crate::preserves_rail::record("receipts", vec![crate::preserves_rail::sequence(
            input.bundle.receipt_refs.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("diagnostics", vec![crate::preserves_rail::sequence(
            input.diagnostics.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("checks", vec![crate::preserves_rail::sequence(vec![
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("bundle-kind-version"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("bundle-member-bindings"),
                crate::preserves_rail::string(binding_status),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("bundle-receipt-kinds"),
                crate::preserves_rail::string(binding_status),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("bundle-is-not-authority"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("provenance-still-required"),
                crate::preserves_rail::string("pass"),
            ]),
        ])]),
    ]))
}

fn live_workflow_bundle_verify_receipt_value(input: &LiveWorkflowBundleVerifyReceiptValueInput<'_>) -> Result<IoValue> {
    validate_decision(input.decision)?;
    let binding_status = if input.decision == "pass" { "pass" } else { "fail" };
    Ok(crate::preserves_rail::record("node-control-live-workflow-bundle-verify-receipt-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_VERIFY_RECEIPT_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(input.decision)]),
        crate::preserves_rail::record("bundle", vec![crate::preserves_rail::string(input.bundle_ref)]),
        crate::preserves_rail::record("ticket", vec![optional_string(input.ticket_ref)]),
        crate::preserves_rail::record("peer-admission", vec![optional_string(input.peer_admission_ref)]),
        crate::preserves_rail::record("authority-grant", vec![optional_string(input.authority_grant_ref)]),
        crate::preserves_rail::record("receipts", vec![crate::preserves_rail::sequence(
            input.receipt_refs.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("expected", vec![live_workflow_bundle_expected_value(input.expected)]),
        crate::preserves_rail::record("diagnostics", vec![crate::preserves_rail::sequence(
            input.diagnostics.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("checks", vec![crate::preserves_rail::sequence(vec![
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("bundle-kind-version"),
                crate::preserves_rail::string(binding_status),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("bundle-member-bindings"),
                crate::preserves_rail::string(binding_status),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("bundle-receipt-kinds"),
                crate::preserves_rail::string(binding_status),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("expected-bindings"),
                crate::preserves_rail::string(binding_status),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("verify-receipt-is-not-authority"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("provenance-still-required"),
                crate::preserves_rail::string("pass"),
            ]),
        ])]),
    ]))
}

fn live_workflow_bundle_gate_receipt_value(input: &LiveWorkflowBundleGateReceiptValueInput<'_>) -> Result<IoValue> {
    validate_decision(input.decision)?;
    let gate_status = if input.decision == "pass" { "pass" } else { "fail" };
    Ok(crate::preserves_rail::record("node-control-live-workflow-bundle-gate-receipt-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_GATE_RECEIPT_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(input.decision)]),
        crate::preserves_rail::record("bundle", vec![crate::preserves_rail::string(input.bundle_ref)]),
        crate::preserves_rail::record("verify-receipt", vec![optional_string(input.verify_receipt_ref)]),
        crate::preserves_rail::record("recomputed-verify", vec![crate::preserves_rail::string(
            input.recomputed_verify_receipt_ref,
        )]),
        crate::preserves_rail::record("ticket", vec![optional_string(input.ticket_ref)]),
        crate::preserves_rail::record("peer-admission", vec![optional_string(input.peer_admission_ref)]),
        crate::preserves_rail::record("authority-grant", vec![optional_string(input.authority_grant_ref)]),
        crate::preserves_rail::record("receipts", vec![crate::preserves_rail::sequence(
            input.receipt_refs.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("expected", vec![live_workflow_bundle_expected_value(input.expected)]),
        crate::preserves_rail::record("diagnostics", vec![crate::preserves_rail::sequence(
            input.diagnostics.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("checks", vec![crate::preserves_rail::sequence(vec![
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("bundle-verification"),
                crate::preserves_rail::string(gate_status),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("verify-receipt-current"),
                crate::preserves_rail::string(gate_status),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("expected-bindings"),
                crate::preserves_rail::string(gate_status),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("gate-receipt-is-not-authority"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("bundle-import-still-required"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("provenance-still-required"),
                crate::preserves_rail::string("pass"),
            ]),
        ])]),
    ]))
}

fn live_workflow_bundle_apply_receipt_value(input: &LiveWorkflowBundleApplyReceiptValueInput<'_>) -> Result<IoValue> {
    validate_decision(input.decision)?;
    let apply_status = if input.decision == "pass" { "pass" } else { "fail" };
    Ok(crate::preserves_rail::record("node-control-live-workflow-bundle-apply-receipt-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_APPLY_RECEIPT_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(input.decision)]),
        crate::preserves_rail::record("state-root", vec![crate::preserves_rail::string(
            input.state_root.display().to_string(),
        )]),
        crate::preserves_rail::record("bundle", vec![crate::preserves_rail::string(input.bundle_ref)]),
        crate::preserves_rail::record("gate-receipt", vec![optional_string(input.gate_receipt_ref)]),
        crate::preserves_rail::record("recomputed-verify", vec![crate::preserves_rail::string(
            input.recomputed_verify_receipt_ref,
        )]),
        crate::preserves_rail::record("import-receipt", vec![optional_string(input.import_receipt_ref)]),
        crate::preserves_rail::record("imported", vec![crate::preserves_rail::sequence(
            input.imported_refs.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("mode", vec![crate::preserves_rail::string(input.mode)]),
        crate::preserves_rail::record("envelope", vec![optional_string(input.envelope_ref)]),
        crate::preserves_rail::record("operation", vec![optional_string(input.operation_ref)]),
        crate::preserves_rail::record("send-receipt", vec![optional_string(input.send_receipt_ref)]),
        crate::preserves_rail::record("expected", vec![live_workflow_bundle_expected_value(input.expected)]),
        crate::preserves_rail::record("diagnostics", vec![crate::preserves_rail::sequence(
            input.diagnostics.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("checks", vec![crate::preserves_rail::sequence(vec![
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("bundle-verification"),
                crate::preserves_rail::string(apply_status),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("gate-receipt-current"),
                crate::preserves_rail::string(apply_status),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("bundle-imported"),
                crate::preserves_rail::string(apply_status),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("send-preflight-or-dispatch"),
                crate::preserves_rail::string(apply_status),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("apply-receipt-is-not-authority"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("provenance-still-required"),
                crate::preserves_rail::string("pass"),
            ]),
        ])]),
    ]))
}

fn live_workflow_bundle_reconcile_receipt_value(
    input: &LiveWorkflowBundleReconcileReceiptValueInput<'_>,
) -> Result<IoValue> {
    validate_decision(input.decision)?;
    let reconcile_status = if input.decision == "pass" { "pass" } else { "fail" };
    Ok(crate::preserves_rail::record("node-control-live-workflow-bundle-reconcile-receipt-v1", vec![
        crate::preserves_rail::string(
            crate::preserves_rail::NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_RECONCILE_RECEIPT_SCHEMA,
        ),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(input.decision)]),
        crate::preserves_rail::record("apply-receipt", vec![crate::preserves_rail::string(input.apply_receipt_ref)]),
        crate::preserves_rail::record("bundle", vec![crate::preserves_rail::string(input.bundle_ref)]),
        crate::preserves_rail::record("send-receipt", vec![optional_string(input.send_receipt_ref)]),
        crate::preserves_rail::record("ingress-receipt", vec![optional_string(input.ingress_receipt_ref)]),
        crate::preserves_rail::record("queue-receipt", vec![optional_string(input.queue_receipt_ref)]),
        crate::preserves_rail::record("control-receipt", vec![optional_string(input.control_receipt_ref)]),
        crate::preserves_rail::record("envelope", vec![optional_string(input.envelope_ref)]),
        crate::preserves_rail::record("operation", vec![optional_string(input.operation_ref)]),
        crate::preserves_rail::record("request", vec![optional_string(input.request_ref)]),
        crate::preserves_rail::record("diagnostics", vec![crate::preserves_rail::sequence(
            input.diagnostics.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("checks", vec![crate::preserves_rail::sequence(vec![
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("apply-receipt-bound"),
                crate::preserves_rail::string(reconcile_status),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("send-receipt-current"),
                crate::preserves_rail::string(reconcile_status),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("receiver-ingress-bound"),
                crate::preserves_rail::string(reconcile_status),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("durable-enqueue-or-deny"),
                crate::preserves_rail::string(reconcile_status),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("control-dispatch-bound"),
                crate::preserves_rail::string(reconcile_status),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("reconcile-receipt-is-not-authority"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("provenance-still-required"),
                crate::preserves_rail::string("pass"),
            ]),
        ])]),
    ]))
}

fn live_workflow_bundle_ack_value(input: &LiveWorkflowBundleAckValueInput<'_>) -> Result<IoValue> {
    validate_decision(input.receiver_decision)?;
    Ok(crate::preserves_rail::record("node-control-live-workflow-bundle-ack-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_ACK_SCHEMA),
        crate::preserves_rail::record("apply-receipt", vec![input.apply_receipt_value.clone()]),
        crate::preserves_rail::record("send-receipt", vec![optional_value(input.send_receipt_value)]),
        crate::preserves_rail::record("ingress-receipt", vec![optional_value(input.ingress_receipt_value)]),
        crate::preserves_rail::record("queue-receipt", vec![optional_value(input.queue_receipt_value)]),
        crate::preserves_rail::record("control-receipt", vec![optional_value(input.control_receipt_value)]),
        crate::preserves_rail::record("reconcile-receipt", vec![input.reconcile_receipt_value.clone()]),
        crate::preserves_rail::record("apply-ref", vec![crate::preserves_rail::string(input.apply_receipt_ref)]),
        crate::preserves_rail::record("send-ref", vec![optional_string(input.send_receipt_ref)]),
        crate::preserves_rail::record("ingress-ref", vec![optional_string(input.ingress_receipt_ref)]),
        crate::preserves_rail::record("queue-ref", vec![optional_string(input.queue_receipt_ref)]),
        crate::preserves_rail::record("control-ref", vec![optional_string(input.control_receipt_ref)]),
        crate::preserves_rail::record("reconcile-ref", vec![crate::preserves_rail::string(
            input.reconcile_receipt_ref,
        )]),
        crate::preserves_rail::record("bundle", vec![crate::preserves_rail::string(input.bundle_ref)]),
        crate::preserves_rail::record("envelope", vec![optional_string(input.envelope_ref)]),
        crate::preserves_rail::record("operation", vec![optional_string(input.operation_ref)]),
        crate::preserves_rail::record("request", vec![optional_string(input.request_ref)]),
        crate::preserves_rail::record("receiver-decision", vec![crate::preserves_rail::string(
            input.receiver_decision,
        )]),
        crate::preserves_rail::record("receiver-diagnostics", vec![crate::preserves_rail::sequence(
            input.receiver_diagnostics.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("diagnostics", vec![crate::preserves_rail::sequence(
            input.diagnostics.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("checks", vec![crate::preserves_rail::sequence(vec![
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("ack-member-refs-bound"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("receiver-outcome-recorded"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("ack-bundle-is-not-authority"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("provenance-still-required"),
                crate::preserves_rail::string("pass"),
            ]),
        ])]),
        crate::preserves_rail::record("member-refs", vec![crate::preserves_rail::sequence(
            [
                Some(input.apply_receipt_ref),
                input.send_receipt_ref,
                input.ingress_receipt_ref,
                input.queue_receipt_ref,
                input.control_receipt_ref,
                Some(input.reconcile_receipt_ref),
            ]
            .into_iter()
            .flatten()
            .map(crate::preserves_rail::string)
            .collect(),
        )]),
    ]))
}

fn live_workflow_bundle_ack_export_receipt_value(
    input: &LiveWorkflowBundleAckExportReceiptValueInput<'_>,
) -> Result<IoValue> {
    validate_decision(input.decision)?;
    let ack_status = if input.decision == "pass" { "pass" } else { "fail" };
    Ok(crate::preserves_rail::record("node-control-live-workflow-bundle-ack-export-receipt-v1", vec![
        crate::preserves_rail::string(
            crate::preserves_rail::NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_ACK_EXPORT_RECEIPT_SCHEMA,
        ),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(input.decision)]),
        crate::preserves_rail::record("ack", vec![crate::preserves_rail::string(&input.ack.ack_ref)]),
        crate::preserves_rail::record("bundle", vec![crate::preserves_rail::string(&input.ack.bundle_ref)]),
        crate::preserves_rail::record("apply-receipt", vec![crate::preserves_rail::string(
            &input.ack.apply_receipt_ref,
        )]),
        crate::preserves_rail::record("send-receipt", vec![optional_string(input.ack.send_receipt_ref.as_deref())]),
        crate::preserves_rail::record("ingress-receipt", vec![optional_string(
            input.ack.ingress_receipt_ref.as_deref(),
        )]),
        crate::preserves_rail::record("queue-receipt", vec![optional_string(input.ack.queue_receipt_ref.as_deref())]),
        crate::preserves_rail::record("control-receipt", vec![optional_string(
            input.ack.control_receipt_ref.as_deref(),
        )]),
        crate::preserves_rail::record("reconcile-receipt", vec![crate::preserves_rail::string(
            &input.ack.reconcile_receipt_ref,
        )]),
        crate::preserves_rail::record("envelope", vec![optional_string(input.ack.envelope_ref.as_deref())]),
        crate::preserves_rail::record("operation", vec![optional_string(input.ack.operation_ref.as_deref())]),
        crate::preserves_rail::record("request", vec![optional_string(input.ack.request_ref.as_deref())]),
        crate::preserves_rail::record("receiver-decision", vec![crate::preserves_rail::string(
            &input.ack.receiver_decision,
        )]),
        crate::preserves_rail::record("receiver-diagnostics", vec![crate::preserves_rail::sequence(
            input.ack.receiver_diagnostics.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("diagnostics", vec![crate::preserves_rail::sequence(
            input.diagnostics.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("checks", vec![crate::preserves_rail::sequence(vec![
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("ack-bundle-kind-version"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("receiver-evidence-packaged"),
                crate::preserves_rail::string(ack_status),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("reconcile-receipt-current"),
                crate::preserves_rail::string(ack_status),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("ack-export-is-not-authority"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("provenance-still-required"),
                crate::preserves_rail::string("pass"),
            ]),
        ])]),
    ]))
}

fn live_workflow_bundle_ack_import_receipt_value(
    input: &LiveWorkflowBundleAckImportReceiptValueInput<'_>,
) -> Result<IoValue> {
    validate_decision(input.decision)?;
    let ack_status = if input.decision == "pass" { "pass" } else { "fail" };
    Ok(crate::preserves_rail::record("node-control-live-workflow-bundle-ack-import-receipt-v1", vec![
        crate::preserves_rail::string(
            crate::preserves_rail::NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_ACK_IMPORT_RECEIPT_SCHEMA,
        ),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(input.decision)]),
        crate::preserves_rail::record("state-root", vec![crate::preserves_rail::string(
            input.state_root.display().to_string(),
        )]),
        crate::preserves_rail::record("ack", vec![crate::preserves_rail::string(&input.ack.ack_ref)]),
        crate::preserves_rail::record("bundle", vec![crate::preserves_rail::string(&input.ack.bundle_ref)]),
        crate::preserves_rail::record("imported", vec![crate::preserves_rail::sequence(
            input.imported_refs.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("receiver-decision", vec![crate::preserves_rail::string(
            &input.ack.receiver_decision,
        )]),
        crate::preserves_rail::record("receiver-diagnostics", vec![crate::preserves_rail::sequence(
            input.ack.receiver_diagnostics.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("diagnostics", vec![crate::preserves_rail::sequence(
            input.diagnostics.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("checks", vec![crate::preserves_rail::sequence(vec![
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("ack-bundle-kind-version"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("ack-member-bindings"),
                crate::preserves_rail::string(ack_status),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("sender-ledger-imported"),
                crate::preserves_rail::string(ack_status),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("ack-import-is-not-authority"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("provenance-still-required"),
                crate::preserves_rail::string("pass"),
            ]),
        ])]),
    ]))
}

fn live_workflow_bundle_expected_value(input: &LiveWorkflowBundleExpectedInput<'_>) -> IoValue {
    crate::preserves_rail::record("expected", vec![crate::preserves_rail::sequence(vec![
        crate::preserves_rail::record("node", vec![optional_string(input.expected_node)]),
        crate::preserves_rail::record("topic", vec![optional_string(input.expected_topic)]),
        crate::preserves_rail::record("endpoint", vec![optional_string(input.expected_endpoint)]),
        crate::preserves_rail::record("peer", vec![optional_string(input.expected_peer)]),
        crate::preserves_rail::record("operations", vec![crate::preserves_rail::sequence(
            input.expected_operations.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("target-scope", vec![optional_string(input.expected_target_scope)]),
        crate::preserves_rail::record("resource-scope", vec![optional_string(input.expected_resource_scope)]),
        crate::preserves_rail::record("as-of-sequence", vec![crate::preserves_rail::string(
            input.as_of_sequence.to_string(),
        )]),
        crate::preserves_rail::record("as-of-epoch", vec![crate::preserves_rail::string(
            input.as_of_epoch.to_string(),
        )]),
    ])])
}

fn live_workflow_bundle_import_receipt_value(input: &LiveWorkflowBundleImportReceiptValueInput<'_>) -> Result<IoValue> {
    validate_decision(input.decision)?;
    let binding_status = if input.decision == "pass" { "pass" } else { "fail" };
    Ok(crate::preserves_rail::record("node-control-live-workflow-bundle-import-receipt-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_IMPORT_RECEIPT_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(input.decision)]),
        crate::preserves_rail::record("state-root", vec![crate::preserves_rail::string(&state_root_profile_ref(
            input.state_root,
        )?)]),
        crate::preserves_rail::record("bundle", vec![crate::preserves_rail::string(&input.bundle.bundle_ref)]),
        crate::preserves_rail::record("ticket", vec![crate::preserves_rail::string(&input.bundle.ticket_ref)]),
        crate::preserves_rail::record("peer-admission", vec![crate::preserves_rail::string(
            &input.bundle.peer_admission_ref,
        )]),
        crate::preserves_rail::record("authority-grant", vec![crate::preserves_rail::string(
            &input.bundle.authority_grant_ref,
        )]),
        crate::preserves_rail::record("ticket-import", vec![optional_string(input.ticket_import_ref)]),
        crate::preserves_rail::record("authority-import", vec![optional_string(input.authority_import_ref)]),
        crate::preserves_rail::record("imported", vec![crate::preserves_rail::sequence(
            input.imported_refs.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("diagnostics", vec![crate::preserves_rail::sequence(
            input.diagnostics.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("checks", vec![crate::preserves_rail::sequence(vec![
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("bundle-kind-version"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("ticket-admission-imported"),
                crate::preserves_rail::string(binding_status),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("authority-grant-imported"),
                crate::preserves_rail::string(binding_status),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("bundle-receipt-imported"),
                crate::preserves_rail::string(binding_status),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("bundle-import-is-not-authority"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("provenance-still-required"),
                crate::preserves_rail::string("pass"),
            ]),
        ])]),
    ]))
}

pub fn control_supervisor_policy_value(input: &ControlSupervisorPolicyInput<'_>) -> Result<IoValue> {
    validate_supervisor_policy_bounds(input.max_restarts, "max restarts")?;
    validate_supervisor_policy_bounds(input.restart_window_ticks, "restart window ticks")?;
    validate_supervisor_policy_bounds(input.heartbeat_timeout_ticks, "heartbeat timeout ticks")?;
    validate_supervisor_policy_bounds(input.shutdown_drain_ticks, "shutdown drain ticks")?;
    validate_ingress_refs(input.policy_refs, "node control supervisor policy ref")?;
    validate_ingress_refs(input.evidence_refs, "node control supervisor evidence ref")?;
    Ok(crate::preserves_rail::record("node-control-supervisor-policy-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::NODE_CONTROL_SUPERVISOR_POLICY_SCHEMA),
        crate::preserves_rail::record("max-restarts", vec![crate::preserves_rail::string(
            input.max_restarts.to_string(),
        )]),
        crate::preserves_rail::record("restart-window-ticks", vec![crate::preserves_rail::string(
            input.restart_window_ticks.to_string(),
        )]),
        crate::preserves_rail::record("heartbeat-timeout-ticks", vec![crate::preserves_rail::string(
            input.heartbeat_timeout_ticks.to_string(),
        )]),
        crate::preserves_rail::record("shutdown-drain-ticks", vec![crate::preserves_rail::string(
            input.shutdown_drain_ticks.to_string(),
        )]),
        crate::preserves_rail::record("stale-lock-recovery", vec![crate::preserves_rail::string(
            if input.stale_lock_recovery { "allow" } else { "deny" },
        )]),
        crate::preserves_rail::record("policy", vec![crate::preserves_rail::sequence(
            input.policy_refs.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("evidence", vec![crate::preserves_rail::sequence(
            input.evidence_refs.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("checks", vec![crate::preserves_rail::sequence(vec![
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("bounded-restarts"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("bounded-heartbeat-timeout"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("explicit-stale-lock-policy"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("shutdown-drain-bound"),
                crate::preserves_rail::string("pass"),
            ]),
        ])]),
    ]))
}

pub fn parse_control_supervisor_policy(value: &IoValue) -> Result<ControlSupervisorPolicy> {
    let fields = value
        .collect_simple_record("node-control-supervisor-policy-v1", Some(9))
        .ok_or_else(|| MoltenError::invalid_harness("expected <node-control-supervisor-policy-v1 ...>"))?;
    require_schema(
        &fields[0],
        crate::preserves_rail::NODE_CONTROL_SUPERVISOR_POLICY_SCHEMA,
        "node control supervisor policy",
    )?;
    let has_stale_lock_recovery = match record_string(&fields[5], "stale-lock-recovery")?.as_str() {
        "allow" => true,
        "deny" => false,
        other => {
            return Err(MoltenError::invalid_harness(format!(
                "node control supervisor stale lock recovery must be allow or deny, got {other}"
            )));
        }
    };
    let max_restarts = record_u64_string(&fields[1], "max-restarts")?;
    let restart_window_ticks = record_u64_string(&fields[2], "restart-window-ticks")?;
    let heartbeat_timeout_ticks = record_u64_string(&fields[3], "heartbeat-timeout-ticks")?;
    let shutdown_drain_ticks = record_u64_string(&fields[4], "shutdown-drain-ticks")?;
    validate_supervisor_policy_bounds(max_restarts, "max restarts")?;
    validate_supervisor_policy_bounds(restart_window_ticks, "restart window ticks")?;
    validate_supervisor_policy_bounds(heartbeat_timeout_ticks, "heartbeat timeout ticks")?;
    validate_supervisor_policy_bounds(shutdown_drain_ticks, "shutdown drain ticks")?;
    Ok(ControlSupervisorPolicy {
        policy_ref: crate::preserves_rail::canonical_hash(value)?,
        max_restarts,
        restart_window_ticks,
        heartbeat_timeout_ticks,
        shutdown_drain_ticks,
        stale_lock_recovery: has_stale_lock_recovery,
        policy_refs: record_ref_strings(&fields[6], "policy")?,
        evidence_refs: record_ref_strings(&fields[7], "evidence")?,
        value: value.clone(),
    })
}

pub fn import_control_supervisor_policy(state_root: &Path, policy_value: &IoValue) -> Result<ControlSupervisorPolicy> {
    validate_state_root(state_root)?;
    ensure_state_layout(state_root)?;
    let policy = parse_control_supervisor_policy(policy_value)?;
    import_artifact(state_root, policy_value)?;
    Ok(policy)
}

fn parse_control_supervisor_receipt(value: &IoValue) -> Result<ControlSupervisorReceipt> {
    let fields = value
        .collect_simple_record("node-control-supervisor-receipt-v1", Some(9))
        .ok_or_else(|| MoltenError::invalid_harness("expected <node-control-supervisor-receipt-v1 ...>"))?;
    require_schema(
        &fields[0],
        crate::preserves_rail::NODE_CONTROL_SUPERVISOR_RECEIPT_SCHEMA,
        "node control supervisor receipt",
    )?;
    Ok(ControlSupervisorReceipt {
        receipt_ref: crate::preserves_rail::canonical_hash(value)?,
        decision: record_string(&fields[1], "decision")?,
        operation: record_string(&fields[2], "operation")?,
        supervisor_policy_ref: record_optional_string(&fields[5], "policy")?,
        diagnostics: record_strings(&fields[7], "diagnostics")?,
        value: value.clone(),
    })
}

fn service_run_supervisor_policy_ref(value: &IoValue) -> Result<Option<String>> {
    if let Some(fields) = value.collect_simple_record("node-control-service-run-receipt-v1", Some(17)) {
        return record_optional_string(&fields[13], "supervisor-policy");
    }
    Ok(None)
}

fn count_prior_supervised_service_runs(state_root: &Path, supervisor_policy_ref: &str) -> Result<u64> {
    let service_dir = state_root.join(CONTROL_SERVICE_DIR);
    if !service_dir.exists() {
        return Ok(0);
    }
    let mut count = 0_u64;
    for entry in fs::read_dir(&service_dir)
        .map_err(|error| MoltenError::invalid_harness(format!("read node control service dir failed: {error}")))?
    {
        let entry = entry.map_err(|error| {
            MoltenError::invalid_harness(format!("read node control service entry failed: {error}"))
        })?;
        let path = entry.path();
        if !path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.ends_with(".service-run-receipt.preserves"))
        {
            continue;
        }
        let value = read_preserves(&path)?;
        if service_run_supervisor_policy_ref(&value)?.as_deref() == Some(supervisor_policy_ref) {
            count = count.saturating_add(1);
        }
    }
    Ok(count)
}

pub fn init_local(input: &InitInput<'_>) -> Result<Init> {
    validate_state_root(input.state_root)?;
    validate_node_id(input.node_id)?;
    ensure_state_layout(input.state_root)?;
    let policy_refs = vec![local_ref("node-policy", input.node_id)?];
    let identity_config = crate::node_identity::NodeIdentityConfig {
        node_id: input.node_id.to_string(),
        display_name: input.node_id.to_string(),
        data_dir: input.state_root.join("identity"),
        explicit_key: None,
        allow_generate: true,
        allow_rotation: false,
        policy_refs: policy_refs.clone(),
    };
    let identity_resolution = crate::node_identity::resolve_node_identity(&identity_config)?;
    let identity = identity_resolution
        .identity
        .ok_or_else(|| MoltenError::invalid_harness("node daemon identity resolution denied"))?;
    let adapters = default_adapter_bindings(input.state_root)?;
    let capability_refs = vec![local_ref("node-capability", input.node_id)?];
    let resource_refs = vec![local_ref("node-resource", input.node_id)?];
    let effect_profile_refs = vec![local_ref("node-effect-profile", input.node_id)?];
    let state_root_ref = state_root_profile_ref(input.state_root)?;
    let config_value = crate::node_runtime::node_config_value(&crate::node_runtime::ConfigValueInput {
        node_identity_ref: &identity.identity_ref,
        state_root_ref: &state_root_ref,
        adapters: &adapters,
        policy_refs: &policy_refs,
        capability_refs: &capability_refs,
        resource_refs: &resource_refs,
        effect_profile_refs: &effect_profile_refs,
    })?;
    write_preserves(&input.state_root.join(CONFIG_FILE), &config_value)?;
    write_preserves(&input.state_root.join(IDENTITY_RECEIPT_FILE), &identity_resolution.receipt_value)?;
    write_preserves(&input.state_root.join(IDENTITY_FILE), &identity.value)?;
    Ok(Init {
        config_ref: crate::preserves_rail::canonical_hash(&config_value)?,
        identity_ref: identity.identity_ref,
        identity_receipt_ref: identity_resolution.receipt_ref,
        config_value,
        identity_receipt_value: identity_resolution.receipt_value,
    })
}

pub fn run_local(input: &RunInput<'_>) -> Result<Run> {
    ensure_state_layout(input.state_root)?;
    verify_restart_state(input.state_root)?;
    let config_value = read_preserves(&input.state_root.join(CONFIG_FILE))?;
    let identity_receipt = read_preserves(&input.state_root.join(IDENTITY_RECEIPT_FILE))?;
    let identity_receipt_ref = crate::preserves_rail::canonical_hash(&identity_receipt)?;
    let index_receipt_refs = index_receipt_refs(input.state_root)?;
    let resource_receipt_refs = resource_receipt_refs(input.state_root)?;
    let capability_receipt_refs = capability_receipt_refs(input.state_root)?;
    let version_refs = vec![local_ref("molten-binary-version", env!("CARGO_PKG_VERSION"))?];
    let source_gate_value = crate::octet_gate::synthetic_clean_octet_gate_receipt_for_tests()?;
    let source_gate_ref = crate::preserves_rail::canonical_hash(&source_gate_value)?;
    let run = crate::node_runtime::start_node_runtime(&crate::node_runtime::NodeRuntimeStartInput {
        config_value,
        identity_receipt_ref,
        index_receipt_refs,
        source_gate_receipt_refs: vec![source_gate_ref],
        source_gate_receipt_values: vec![source_gate_value],
        capability_receipt_refs,
        resource_receipt_refs,
        version_refs,
    })?;
    for (adapter, value) in run.adapter_receipts.iter().zip(run.adapter_receipt_values.iter()) {
        write_preserves(
            &input.state_root.join("receipts").join(format!("adapter-start-{}.preserves", adapter.name)),
            value,
        )?;
    }
    write_preserves(&input.state_root.join(STARTUP_FILE), &run.startup_receipt.value)?;
    if run.decision != "pass" {
        return Err(MoltenError::invalid_harness(format!(
            "node daemon startup denied receipt={}",
            run.startup_receipt.receipt_ref
        )));
    }
    let startup_ref = run.startup_receipt.receipt_ref.clone();
    write_active_lock(input.state_root, &startup_ref)?;
    import_artifact(input.state_root, &run.startup_receipt.value)?;
    Ok(Run {
        startup_ref,
        startup_value: run.startup_receipt.value,
        adapter_receipt_refs: run.adapter_receipts,
    })
}

pub fn status_local(input: &StatusInput<'_>) -> Result<Status> {
    let request = status_request()?;
    status_local_node_with_request(input, &request)
}

fn status_local_node_with_request(
    input: &StatusInput<'_>,
    request: &crate::node_runtime::ControlRequest,
) -> Result<Status> {
    let startup_value = read_preserves(&input.state_root.join(STARTUP_FILE))?;
    let startup = crate::node_runtime::parse_node_startup_receipt(&startup_value)?;
    let shutdown_ref = if input.state_root.join(SHUTDOWN_FILE).exists() {
        Some(crate::preserves_rail::canonical_hash(&read_preserves(&input.state_root.join(SHUTDOWN_FILE))?)?)
    } else {
        None
    };
    let status = if shutdown_ref.is_some() { "stopped" } else { "running" }.to_string();
    let health_value = crate::node_runtime::node_health_receipt_value(&crate::node_runtime::HealthReceiptValueInput {
        decision: "pass",
        startup_receipt_ref: &startup.receipt_ref,
        shutdown_receipt_ref: shutdown_ref.as_deref(),
        adapter_receipts: &startup.adapters,
        index_receipt_refs: &index_receipt_refs(input.state_root)?,
        head_refs: std::slice::from_ref(&startup.receipt_ref),
        open_job_refs: &[],
        replay_is_eligible: shutdown_ref.is_some(),
        diagnostics: &[],
    })?;
    let health_ref = crate::preserves_rail::canonical_hash(&health_value)?;
    write_preserves(&input.state_root.join(HEALTH_FILE), &health_value)?;
    import_artifact(input.state_root, &health_value)?;
    let control_receipt_value = control_receipt_for_request(
        input.state_root,
        request,
        &startup.receipt_ref,
        std::slice::from_ref(&health_ref),
        &[],
    )?;
    let control_receipt_ref = crate::preserves_rail::canonical_hash(&control_receipt_value)?;
    write_preserves(&input.state_root.join(CONTROL_STATUS_FILE), &control_receipt_value)?;
    import_artifact(input.state_root, &control_receipt_value)?;
    Ok(Status {
        health_ref,
        control_receipt_ref,
        health_value,
        control_receipt_value,
        status,
    })
}

pub fn stop_local(input: &StopInput<'_>) -> Result<Stop> {
    let request = shutdown_request()?;
    stop_local_node_with_request(input, &request)
}

fn stop_local_node_with_request(input: &StopInput<'_>, request: &crate::node_runtime::ControlRequest) -> Result<Stop> {
    let startup_value = read_preserves(&input.state_root.join(STARTUP_FILE))?;
    let startup = crate::node_runtime::parse_node_startup_receipt(&startup_value)?;
    let mut shutdown_adapters = Vec::with_capacity(startup.adapters.len());
    for adapter in startup.adapters.iter().rev() {
        let binding = crate::node_runtime::node_adapter_binding(&adapter.name, &adapter.receipt_ref)?;
        let value = crate::node_runtime::node_adapter_lifecycle_receipt_value(
            &crate::node_runtime::AdapterLifecycleReceiptInput {
                operation: "shutdown",
                decision: "pass",
                adapter: &binding,
                index_receipt_refs: &index_receipt_refs(input.state_root)?,
                resource_receipt_refs: &resource_receipt_refs(input.state_root)?,
                diagnostics: &[],
            },
        )?;
        let receipt_ref = crate::preserves_rail::canonical_hash(&value)?;
        write_preserves(
            &input.state_root.join("receipts").join(format!("adapter-shutdown-{}.preserves", adapter.name)),
            &value,
        )?;
        import_artifact(input.state_root, &value)?;
        shutdown_adapters.push(crate::node_runtime::NodeAdapterReceiptRef {
            name: adapter.name.clone(),
            receipt_ref,
        });
    }
    let index_refs = index_receipt_refs(input.state_root)?;
    let shutdown_value =
        crate::node_runtime::node_shutdown_receipt_value(&crate::node_runtime::ShutdownReceiptValueInput {
            decision: "pass",
            startup_receipt_ref: &startup.receipt_ref,
            adapter_receipts: &shutdown_adapters,
            drained_job_refs: &[],
            index_receipt_refs: &index_refs,
            diagnostics: &[],
        })?;
    let shutdown_ref = crate::preserves_rail::canonical_hash(&shutdown_value)?;
    write_preserves(&input.state_root.join(SHUTDOWN_FILE), &shutdown_value)?;
    import_artifact(input.state_root, &shutdown_value)?;
    let control_receipt_value = control_receipt_for_request(
        input.state_root,
        request,
        &startup.receipt_ref,
        std::slice::from_ref(&shutdown_ref),
        &[],
    )?;
    let control_receipt_ref = crate::preserves_rail::canonical_hash(&control_receipt_value)?;
    write_preserves(&input.state_root.join(CONTROL_STOP_FILE), &control_receipt_value)?;
    import_artifact(input.state_root, &control_receipt_value)?;
    remove_active_lock(input.state_root)?;
    Ok(Stop {
        shutdown_ref,
        control_receipt_ref,
        shutdown_value,
        control_receipt_value,
    })
}

pub fn submit_control_request(input: &ControlSubmitInput<'_>) -> Result<ControlSubmit> {
    validate_state_root(input.state_root)?;
    ensure_state_layout(input.state_root)?;
    let request = crate::node_runtime::parse_control_request(input.request_value)?;
    import_artifact(input.state_root, input.request_value)?;
    let inbox_path = control_inbox_path(input.state_root, &request.request_ref);
    write_preserves(&inbox_path, input.request_value)?;
    let location_ref = local_ref("node-control-inbox-path", &inbox_path.display().to_string())?;
    let receipt_value = queue_receipt_value(&QueueReceiptValueInput {
        decision: "pass",
        phase: "enqueue",
        operation: &request.operation,
        request_ref: &request.request_ref,
        location_ref: &location_ref,
        diagnostics: &[],
    })?;
    let queue_receipt_ref = crate::preserves_rail::canonical_hash(&receipt_value)?;
    write_preserves(&queue_receipt_path(input.state_root, &request.request_ref), &receipt_value)?;
    import_artifact(input.state_root, &receipt_value)?;
    Ok(ControlSubmit {
        request_ref: request.request_ref,
        inbox_path,
        queue_receipt_ref,
        queue_receipt_value: receipt_value,
    })
}

pub fn dispatch_control_request(input: &ControlDispatchInput<'_>) -> Result<ControlDispatch> {
    validate_state_root(input.state_root)?;
    ensure_state_layout(input.state_root)?;
    require_active_lock(input.state_root)?;
    let request_path = match input.request_path {
        Some(path) => path.to_path_buf(),
        None => first_pending_control_request(input.state_root)?,
    };
    let request_value = read_preserves(&request_path)?;
    let request = crate::node_runtime::parse_control_request(&request_value)?;
    import_artifact(input.state_root, &request_value)?;
    if let Some(prior) = prior_dispatch_for_request(input.state_root, &request)? {
        archive_dispatched_request(input.state_root, &request_path, &request.value)?;
        write_dispatch_queue_receipt(input.state_root, &request, "duplicate-dispatch")?;
        return Ok(prior);
    }
    let dispatch = match request.operation.as_str() {
        "status" => dispatch_status_request(input.state_root, &request)?,
        "shutdown" => dispatch_shutdown_request(input.state_root, &request)?,
        "install" => dispatch_install_request(input.state_root, &request)?,
        "run" => dispatch_run_request(input.state_root, &request)?,
        "gate" => dispatch_gate_request(input.state_root, &request)?,
        other => {
            return Err(MoltenError::invalid_harness(format!("node control request operation unsupported: {other}")));
        }
    };
    archive_dispatched_request(input.state_root, &request_path, &request.value)?;
    write_dispatch_queue_receipt(input.state_root, &request, "dispatch")?;
    Ok(dispatch)
}

pub fn run_control_loop(input: &ControlLoopInput<'_>) -> Result<ControlLoop> {
    validate_state_root(input.state_root)?;
    ensure_state_layout(input.state_root)?;
    let max_requests = validate_loop_request_limit(input.max_requests)?;
    require_active_lock(input.state_root)?;
    let startup = current_startup_receipt(input.state_root)?;
    let lock_value = read_preserves(&input.state_root.join(CONTROL_LOCK_FILE))?;
    let lock_ref = crate::preserves_rail::canonical_hash(&lock_value)?;
    let initial_diagnostics = Vec::new();
    let heartbeat_value = heartbeat_receipt_value(&HeartbeatReceiptValueInput {
        startup_receipt_ref: &startup.receipt_ref,
        lock_ref: &lock_ref,
        loop_sequence: 0,
        processed_count: 0,
        diagnostics: &initial_diagnostics,
    })?;
    let heartbeat_receipt_ref = crate::preserves_rail::canonical_hash(&heartbeat_value)?;
    write_preserves(&control_heartbeat_receipt_path(input.state_root, &heartbeat_receipt_ref), &heartbeat_value)?;
    import_artifact(input.state_root, &heartbeat_value)?;

    let mut processed_request_refs = Vec::with_capacity(max_requests);
    let mut dispatch_receipt_refs = Vec::with_capacity(max_requests);
    let mut diagnostics = Vec::new();
    let mut has_stopped = false;
    for _ in 0..max_requests {
        let Some(request_path) = next_pending_control_request(input.state_root)? else {
            break;
        };
        let dispatched = dispatch_control_request(&ControlDispatchInput {
            state_root: input.state_root,
            request_path: Some(&request_path),
        })?;
        let control = crate::node_runtime::parse_control_receipt(&dispatched.control_receipt_value)?;
        processed_request_refs.push(dispatched.request_ref.clone());
        dispatch_receipt_refs.push(dispatched.control_receipt_ref.clone());
        if dispatched.operation == "shutdown" && control.decision == "pass" {
            has_stopped = true;
            break;
        }
    }
    if processed_request_refs.len() == max_requests && next_pending_control_request(input.state_root)?.is_some() {
        diagnostics.push("node control loop reached max requests with pending inbox entries".to_string());
    }
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let loop_value = loop_receipt_value(&LoopReceiptValueInput {
        decision,
        startup_receipt_ref: &startup.receipt_ref,
        heartbeat_receipt_ref: &heartbeat_receipt_ref,
        max_requests: input.max_requests,
        processed_request_refs: &processed_request_refs,
        dispatch_receipt_refs: &dispatch_receipt_refs,
        has_stopped,
        diagnostics: &diagnostics,
    })?;
    let loop_receipt_ref = crate::preserves_rail::canonical_hash(&loop_value)?;
    write_preserves(&control_loop_receipt_path(input.state_root, &loop_receipt_ref), &loop_value)?;
    import_artifact(input.state_root, &loop_value)?;
    Ok(ControlLoop {
        loop_receipt_ref,
        loop_receipt_value: loop_value,
        heartbeat_receipt_ref,
        heartbeat_receipt_value: heartbeat_value,
        processed_request_refs,
        dispatch_receipt_refs,
        has_stopped,
    })
}

pub fn serve_control(input: &ControlServeInput<'_>) -> Result<ControlServe> {
    validate_state_root(input.state_root)?;
    validate_node_id(input.topic)?;
    ensure_state_layout(input.state_root)?;
    let max_ticks = validate_service_tick_limit(input.max_ticks)?;
    let max_requests_per_tick = validate_loop_request_limit(input.max_requests_per_tick)?;
    let supervisor_policy = input
        .supervisor_policy_value
        .map(|value| import_control_supervisor_policy(input.state_root, value))
        .transpose()?;
    require_active_lock(input.state_root)?;
    let startup = current_startup_receipt(input.state_root)?;

    let existing_lock = handle_existing_service_lock(input, &startup, supervisor_policy.as_ref(), Vec::new())?;
    if let Some(denied) = existing_lock.denied {
        return Ok(denied);
    }
    if let Some(policy) = supervisor_policy.as_ref() {
        let prior_runs = count_prior_supervised_service_runs(input.state_root, &policy.policy_ref)?;
        if prior_runs > policy.max_restarts {
            return denied_restart_attempt(input, &startup, policy, prior_runs, &existing_lock.supervisor_receipt_refs);
        }
    }

    let start = start_service_run(input, &startup, supervisor_policy.as_ref(), existing_lock.supervisor_receipt_refs)?;
    let run = run_service_ticks(ServiceTickInput {
        state_root: input.state_root,
        topic: input.topic,
        max_ticks: input.max_ticks,
        max_requests_per_tick: input.max_requests_per_tick,
        tick_capacity: max_ticks,
        event_capacity: max_ticks.saturating_mul(max_requests_per_tick),
        startup_receipt_ref: &startup.receipt_ref,
        service_lock_ref: &start.service_lock_ref,
    })?;
    let shutdown = note_shutdown_drain(ShutdownDrainInput {
        state_root: input.state_root,
        topic: input.topic,
        startup_receipt_ref: &startup.receipt_ref,
        service_lock_ref: &start.service_lock_ref,
        policy: supervisor_policy.as_ref(),
        run,
        supervisor_receipt_refs: start.supervisor_receipt_refs,
    })?;
    remove_service_lock(input.state_root, &start.service_lock_ref)?;
    finish_service_run(FinishServiceInput {
        state_root: input.state_root,
        topic: input.topic,
        max_ticks: input.max_ticks,
        max_requests_per_tick: input.max_requests_per_tick,
        startup_receipt_ref: &startup.receipt_ref,
        service_lock_ref: &start.service_lock_ref,
        supervisor_policy_ref: supervisor_policy.as_ref().map(|policy| policy.policy_ref.as_str()),
        supervisor_receipt_refs: shutdown.supervisor_receipt_refs,
        run: shutdown.run,
    })
}

struct ExistingServiceLock {
    supervisor_receipt_refs: Vec<String>,
    denied: Option<ControlServe>,
}

struct ServiceStart {
    service_lock_ref: String,
    supervisor_receipt_refs: Vec<String>,
}

struct ServiceTickInput<'a> {
    state_root: &'a Path,
    topic: &'a str,
    max_ticks: u64,
    max_requests_per_tick: u64,
    tick_capacity: usize,
    event_capacity: usize,
    startup_receipt_ref: &'a str,
    service_lock_ref: &'a str,
}

struct ServiceRunParts {
    heartbeat_receipt_refs: Vec<String>,
    ingress_receipt_refs: Vec<String>,
    loop_receipt_refs: Vec<String>,
    processed_request_refs: Vec<String>,
    diagnostics: Vec<String>,
    ticks: u64,
    has_stopped: bool,
}

struct ShutdownDrainInput<'a> {
    state_root: &'a Path,
    topic: &'a str,
    startup_receipt_ref: &'a str,
    service_lock_ref: &'a str,
    policy: Option<&'a ControlSupervisorPolicy>,
    run: ServiceRunParts,
    supervisor_receipt_refs: Vec<String>,
}

struct ShutdownDrain {
    run: ServiceRunParts,
    supervisor_receipt_refs: Vec<String>,
}

struct FinishServiceInput<'a> {
    state_root: &'a Path,
    topic: &'a str,
    max_ticks: u64,
    max_requests_per_tick: u64,
    startup_receipt_ref: &'a str,
    service_lock_ref: &'a str,
    supervisor_policy_ref: Option<&'a str>,
    supervisor_receipt_refs: Vec<String>,
    run: ServiceRunParts,
}

fn handle_existing_service_lock(
    input: &ControlServeInput<'_>,
    startup: &crate::node_runtime::NodeStartupReceipt,
    supervisor_policy: Option<&ControlSupervisorPolicy>,
    mut supervisor_receipt_refs: Vec<String>,
) -> Result<ExistingServiceLock> {
    if !input.state_root.join(CONTROL_SERVICE_LOCK_FILE).exists() {
        return Ok(ExistingServiceLock {
            supervisor_receipt_refs,
            denied: None,
        });
    }
    if let Some(policy) = supervisor_policy
        && policy.stale_lock_recovery
    {
        let lock_value = read_preserves(&input.state_root.join(CONTROL_SERVICE_LOCK_FILE))?;
        let stale_lock_ref = crate::preserves_rail::canonical_hash(&lock_value)?;
        let diagnostics = vec!["node control stale service lock recovered by supervisor policy".to_string()];
        let receipt_ref = write_supervisor_receipt(input.state_root, &SupervisorReceiptValueInput {
            decision: "pass",
            operation: "stale-lock-recover",
            startup_receipt_ref: &startup.receipt_ref,
            service_lock_ref: Some(&stale_lock_ref),
            supervisor_policy_ref: Some(&policy.policy_ref),
            topic: input.topic,
            diagnostics: &diagnostics,
        })?;
        supervisor_receipt_refs.push(receipt_ref);
        fs::remove_file(input.state_root.join(CONTROL_SERVICE_LOCK_FILE)).map_err(MoltenError::from)?;
        return Ok(ExistingServiceLock {
            supervisor_receipt_refs,
            denied: None,
        });
    }
    let denied = denied_duplicate_service_run(input, startup, supervisor_policy, &supervisor_receipt_refs)?;
    Ok(ExistingServiceLock {
        supervisor_receipt_refs,
        denied: Some(denied),
    })
}

fn denied_restart_attempt(
    input: &ControlServeInput<'_>,
    startup: &crate::node_runtime::NodeStartupReceipt,
    policy: &ControlSupervisorPolicy,
    prior_runs: u64,
    inherited_supervisor_receipt_refs: &[String],
) -> Result<ControlServe> {
    let diagnostics = vec![format!(
        "node control supervisor restart attempts {prior_runs} exceeded bound {}",
        policy.max_restarts
    )];
    let mut supervisor_receipt_refs = inherited_supervisor_receipt_refs.to_vec();
    let receipt_ref = write_supervisor_receipt(input.state_root, &SupervisorReceiptValueInput {
        decision: "deny",
        operation: "restart-attempt-deny",
        startup_receipt_ref: &startup.receipt_ref,
        service_lock_ref: None,
        supervisor_policy_ref: Some(&policy.policy_ref),
        topic: input.topic,
        diagnostics: &diagnostics,
    })?;
    supervisor_receipt_refs.push(receipt_ref);
    let receipt_value = service_run_receipt_value(&ServiceRunReceiptValueInput {
        decision: "deny",
        startup_receipt_ref: &startup.receipt_ref,
        service_lock_ref: None,
        topic: input.topic,
        max_ticks: input.max_ticks,
        max_requests_per_tick: input.max_requests_per_tick,
        ticks: 0,
        heartbeat_receipt_refs: &[],
        ingress_receipt_refs: &[],
        loop_receipt_refs: &[],
        processed_request_refs: &[],
        has_stopped: false,
        supervisor_policy_ref: Some(&policy.policy_ref),
        supervisor_receipt_refs: &supervisor_receipt_refs,
        diagnostics: &diagnostics,
    })?;
    let service_receipt_ref = crate::preserves_rail::canonical_hash(&receipt_value)?;
    write_preserves(&control_service_run_receipt_path(input.state_root, &service_receipt_ref), &receipt_value)?;
    import_artifact(input.state_root, &receipt_value)?;
    Ok(ControlServe {
        service_receipt_ref,
        service_receipt_value: receipt_value,
        service_lock_ref: None,
        heartbeat_receipt_refs: Vec::new(),
        ingress_receipt_refs: Vec::new(),
        loop_receipt_refs: Vec::new(),
        processed_request_refs: Vec::new(),
        supervisor_policy_ref: Some(policy.policy_ref.clone()),
        supervisor_receipt_refs,
        ticks: 0,
        has_stopped: false,
        decision: "deny".to_string(),
    })
}

fn start_service_run(
    input: &ControlServeInput<'_>,
    startup: &crate::node_runtime::NodeStartupReceipt,
    supervisor_policy: Option<&ControlSupervisorPolicy>,
    mut supervisor_receipt_refs: Vec<String>,
) -> Result<ServiceStart> {
    let identity = crate::node_identity::parse_node_identity(&read_preserves(&input.state_root.join(IDENTITY_FILE))?)?;
    let service_run_id = local_ref(
        "node-control-service-run",
        &format!("{}:{}:{}:{}", startup.receipt_ref, input.topic, input.max_ticks, input.max_requests_per_tick),
    )?;
    let lock_value = service_lock_value(&ServiceLockValueInput {
        state_root: input.state_root,
        startup_receipt_ref: &startup.receipt_ref,
        node_id: &identity.node_id,
        topic: input.topic,
        max_ticks: input.max_ticks,
        max_requests_per_tick: input.max_requests_per_tick,
        service_run_ref: &service_run_id,
    })?;
    let service_lock_ref = crate::preserves_rail::canonical_hash(&lock_value)?;
    write_preserves(&input.state_root.join(CONTROL_SERVICE_LOCK_FILE), &lock_value)?;
    import_artifact(input.state_root, &lock_value)?;
    if let Some(policy) = supervisor_policy {
        let receipt_ref = write_supervisor_receipt(input.state_root, &SupervisorReceiptValueInput {
            decision: "pass",
            operation: "restart-attempt",
            startup_receipt_ref: &startup.receipt_ref,
            service_lock_ref: Some(&service_lock_ref),
            supervisor_policy_ref: Some(&policy.policy_ref),
            topic: input.topic,
            diagnostics: &[],
        })?;
        supervisor_receipt_refs.push(receipt_ref);
    }
    Ok(ServiceStart {
        service_lock_ref,
        supervisor_receipt_refs,
    })
}

fn run_service_ticks(input: ServiceTickInput<'_>) -> Result<ServiceRunParts> {
    let mut run = ServiceRunParts {
        heartbeat_receipt_refs: Vec::with_capacity(input.tick_capacity),
        ingress_receipt_refs: Vec::with_capacity(input.event_capacity),
        loop_receipt_refs: Vec::with_capacity(input.tick_capacity),
        processed_request_refs: Vec::with_capacity(input.event_capacity),
        diagnostics: Vec::with_capacity(input.tick_capacity.saturating_mul(2)),
        ticks: 0,
        has_stopped: false,
    };

    for tick in 0..input.max_ticks {
        run.ticks = tick + 1;
        if run_service_tick(&input, &mut run, tick)? {
            break;
        }
    }
    if !run.has_stopped {
        match has_pending_service_work(input.state_root, input.topic) {
            Ok(true) => run.diagnostics.push("node control service reached max ticks with pending work".to_string()),
            Ok(false) => {}
            Err(error) => run.diagnostics.push(format!("node control service pending-work scan failed: {error}")),
        }
    }
    Ok(run)
}

fn run_service_tick(input: &ServiceTickInput<'_>, run: &mut ServiceRunParts, tick: u64) -> Result<bool> {
    write_service_heartbeat(input, run, tick)?;
    if deliver_service_ingress(input, run)? {
        return Ok(true);
    }
    process_service_loop(input, run)
}

fn write_service_heartbeat(input: &ServiceTickInput<'_>, run: &mut ServiceRunParts, tick: u64) -> Result<()> {
    let heartbeat_value = service_heartbeat_receipt_value(&ServiceHeartbeatValueInput {
        startup_receipt_ref: input.startup_receipt_ref,
        service_lock_ref: input.service_lock_ref,
        tick,
        delivered_count: run.ingress_receipt_refs.len() as u64,
        processed_count: run.processed_request_refs.len() as u64,
        diagnostics: &run.diagnostics,
    })?;
    let heartbeat_ref = crate::preserves_rail::canonical_hash(&heartbeat_value)?;
    write_preserves(&control_service_heartbeat_path(input.state_root, &heartbeat_ref), &heartbeat_value)?;
    import_artifact(input.state_root, &heartbeat_value)?;
    run.heartbeat_receipt_refs.push(heartbeat_ref);
    Ok(())
}

fn deliver_service_ingress(input: &ServiceTickInput<'_>, run: &mut ServiceRunParts) -> Result<bool> {
    let envelope_refs = match pending_ingress_envelope_refs(input.state_root, input.topic) {
        Ok(envelope_refs) => envelope_refs,
        Err(error) => {
            run.diagnostics.push(format!("node control service ingress scan failed: {error}"));
            return Ok(true);
        }
    };
    for envelope_ref in envelope_refs {
        let delivered = match deliver_control_ingress(&ControlIngressDeliverInput {
            state_root: input.state_root,
            topic: input.topic,
            envelope_ref: &envelope_ref,
        }) {
            Ok(delivered) => delivered,
            Err(error) => {
                run.diagnostics
                    .push(format!("node control service ingress delivery {envelope_ref} failed: {error}"));
                continue;
            }
        };
        let receipt = ingress_receipt_decision(&delivered.ingress_receipt_value)?;
        if receipt != "pass" {
            run.diagnostics
                .push(format!("node control service ingress {} decision {}", delivered.envelope_ref, receipt));
        }
        run.ingress_receipt_refs.push(delivered.ingress_receipt_ref);
    }
    Ok(false)
}

fn process_service_loop(input: &ServiceTickInput<'_>, run: &mut ServiceRunParts) -> Result<bool> {
    if !input.state_root.join(CONTROL_LOCK_FILE).exists() {
        run.has_stopped = true;
        return Ok(true);
    }
    let loop_run = match run_control_loop(&ControlLoopInput {
        state_root: input.state_root,
        max_requests: input.max_requests_per_tick,
    }) {
        Ok(loop_run) => loop_run,
        Err(error) => {
            run.diagnostics.push(format!("node control service loop failed: {error}"));
            return Ok(true);
        }
    };
    run.processed_request_refs.extend(loop_run.processed_request_refs.iter().cloned());
    run.loop_receipt_refs.push(loop_run.loop_receipt_ref);
    if loop_run.has_stopped || !input.state_root.join(CONTROL_LOCK_FILE).exists() {
        run.has_stopped = true;
        return Ok(true);
    }
    Ok(false)
}

fn note_shutdown_drain(input: ShutdownDrainInput<'_>) -> Result<ShutdownDrain> {
    let mut run = input.run;
    let mut supervisor_receipt_refs = input.supervisor_receipt_refs;
    if let Some(policy) = input.policy
        && run.has_stopped
    {
        let mut shutdown_diagnostics = Vec::new();
        if run.ticks > policy.shutdown_drain_ticks {
            let diagnostic = format!(
                "node control shutdown drain ticks {} exceeded supervisor bound {}",
                run.ticks, policy.shutdown_drain_ticks
            );
            run.diagnostics.push(diagnostic.clone());
            shutdown_diagnostics.push(diagnostic);
        }
        let receipt_ref = write_supervisor_receipt(input.state_root, &SupervisorReceiptValueInput {
            decision: if shutdown_diagnostics.is_empty() {
                "pass"
            } else {
                "deny"
            },
            operation: "shutdown-drain",
            startup_receipt_ref: input.startup_receipt_ref,
            service_lock_ref: Some(input.service_lock_ref),
            supervisor_policy_ref: Some(&policy.policy_ref),
            topic: input.topic,
            diagnostics: &shutdown_diagnostics,
        })?;
        supervisor_receipt_refs.push(receipt_ref);
    }
    Ok(ShutdownDrain {
        run,
        supervisor_receipt_refs,
    })
}

fn finish_service_run(input: FinishServiceInput<'_>) -> Result<ControlServe> {
    let decision = if input.run.diagnostics.is_empty() {
        "pass"
    } else {
        "deny"
    };
    let receipt_value = service_run_receipt_value(&ServiceRunReceiptValueInput {
        decision,
        startup_receipt_ref: input.startup_receipt_ref,
        service_lock_ref: Some(input.service_lock_ref),
        topic: input.topic,
        max_ticks: input.max_ticks,
        max_requests_per_tick: input.max_requests_per_tick,
        ticks: input.run.ticks,
        heartbeat_receipt_refs: &input.run.heartbeat_receipt_refs,
        ingress_receipt_refs: &input.run.ingress_receipt_refs,
        loop_receipt_refs: &input.run.loop_receipt_refs,
        processed_request_refs: &input.run.processed_request_refs,
        has_stopped: input.run.has_stopped,
        supervisor_policy_ref: input.supervisor_policy_ref,
        supervisor_receipt_refs: &input.supervisor_receipt_refs,
        diagnostics: &input.run.diagnostics,
    })?;
    let service_receipt_ref = crate::preserves_rail::canonical_hash(&receipt_value)?;
    write_preserves(&control_service_run_receipt_path(input.state_root, &service_receipt_ref), &receipt_value)?;
    import_artifact(input.state_root, &receipt_value)?;
    Ok(ControlServe {
        service_receipt_ref,
        service_receipt_value: receipt_value,
        service_lock_ref: Some(input.service_lock_ref.to_string()),
        heartbeat_receipt_refs: input.run.heartbeat_receipt_refs,
        ingress_receipt_refs: input.run.ingress_receipt_refs,
        loop_receipt_refs: input.run.loop_receipt_refs,
        processed_request_refs: input.run.processed_request_refs,
        supervisor_policy_ref: input.supervisor_policy_ref.map(|value| value.to_string()),
        supervisor_receipt_refs: input.supervisor_receipt_refs,
        ticks: input.run.ticks,
        has_stopped: input.run.has_stopped,
        decision: decision.to_string(),
    })
}

fn denied_duplicate_service_run(
    input: &ControlServeInput<'_>,
    startup: &crate::node_runtime::NodeStartupReceipt,
    supervisor_policy: Option<&ControlSupervisorPolicy>,
    inherited_supervisor_receipt_refs: &[String],
) -> Result<ControlServe> {
    let lock_value = read_preserves(&input.state_root.join(CONTROL_SERVICE_LOCK_FILE))?;
    let service_lock_ref = crate::preserves_rail::canonical_hash(&lock_value)?;
    let diagnostics = vec!["node control service runner already active".to_string()];
    let mut supervisor_receipt_refs = inherited_supervisor_receipt_refs.to_vec();
    if let Some(policy) = supervisor_policy {
        let receipt_ref = write_supervisor_receipt(input.state_root, &SupervisorReceiptValueInput {
            decision: "deny",
            operation: "duplicate-runner-deny",
            startup_receipt_ref: &startup.receipt_ref,
            service_lock_ref: Some(&service_lock_ref),
            supervisor_policy_ref: Some(&policy.policy_ref),
            topic: input.topic,
            diagnostics: &diagnostics,
        })?;
        supervisor_receipt_refs.push(receipt_ref);
    }
    let receipt_value = service_run_receipt_value(&ServiceRunReceiptValueInput {
        decision: "deny",
        startup_receipt_ref: &startup.receipt_ref,
        service_lock_ref: Some(&service_lock_ref),
        topic: input.topic,
        max_ticks: input.max_ticks,
        max_requests_per_tick: input.max_requests_per_tick,
        ticks: 0,
        heartbeat_receipt_refs: &[],
        ingress_receipt_refs: &[],
        loop_receipt_refs: &[],
        processed_request_refs: &[],
        has_stopped: false,
        supervisor_policy_ref: supervisor_policy.map(|policy| policy.policy_ref.as_str()),
        supervisor_receipt_refs: &supervisor_receipt_refs,
        diagnostics: &diagnostics,
    })?;
    let service_receipt_ref = crate::preserves_rail::canonical_hash(&receipt_value)?;
    write_preserves(&control_service_run_receipt_path(input.state_root, &service_receipt_ref), &receipt_value)?;
    import_artifact(input.state_root, &receipt_value)?;
    Ok(ControlServe {
        service_receipt_ref,
        service_receipt_value: receipt_value,
        service_lock_ref: Some(service_lock_ref),
        heartbeat_receipt_refs: Vec::new(),
        ingress_receipt_refs: Vec::new(),
        loop_receipt_refs: Vec::new(),
        processed_request_refs: Vec::new(),
        supervisor_policy_ref: supervisor_policy.map(|policy| policy.policy_ref.clone()),
        supervisor_receipt_refs,
        ticks: 0,
        has_stopped: false,
        decision: "deny".to_string(),
    })
}

fn pending_ingress_envelope_refs(state_root: &Path, topic: &str) -> Result<Vec<String>> {
    let topic_dir = state_root.join(CONTROL_INGRESS_DIR).join(topic);
    if !topic_dir.exists() {
        return Ok(Vec::new());
    }
    let mut paths = Vec::with_capacity(MAX_PENDING_CONTROL_REQUESTS);
    for entry_result in fs::read_dir(&topic_dir).map_err(MoltenError::from)? {
        let entry = entry_result.map_err(MoltenError::from)?;
        let file_type = entry.file_type().map_err(MoltenError::from)?;
        if file_type.is_file() {
            if paths.len() >= MAX_PENDING_CONTROL_REQUESTS {
                return Err(MoltenError::invalid_harness("node control ingress pending envelope bound exceeded"));
            }
            paths.push(entry.path());
        }
    }
    paths.sort();
    let mut envelope_refs = Vec::with_capacity(paths.len());
    for path in paths {
        let value = read_preserves(&path)?;
        let envelope = parse_control_ingress_envelope(&value)?;
        if !control_ingress_receipt_path(state_root, &envelope.envelope_ref, "deliver").exists() {
            envelope_refs.push(envelope.envelope_ref);
        }
    }
    Ok(envelope_refs)
}

fn has_pending_service_work(state_root: &Path, topic: &str) -> Result<bool> {
    if !pending_ingress_envelope_refs(state_root, topic)?.is_empty() {
        return Ok(true);
    }
    next_pending_control_request(state_root).map(|pending| pending.is_some())
}

fn remove_service_lock(state_root: &Path, service_lock_ref: &str) -> Result<()> {
    let path = state_root.join(CONTROL_SERVICE_LOCK_FILE);
    if !path.exists() {
        return Ok(());
    }
    let current_ref = crate::preserves_rail::canonical_hash(&read_preserves(&path)?)?;
    if current_ref != service_lock_ref {
        return Err(MoltenError::invalid_harness("node control service lock changed during serve"));
    }
    fs::remove_file(path).map_err(MoltenError::from)
}

fn ingress_receipt_decision(value: &IoValue) -> Result<String> {
    let fields = value
        .collect_simple_record("node-control-ingress-receipt-v1", Some(15))
        .ok_or_else(|| MoltenError::invalid_harness("expected <node-control-ingress-receipt-v1 ...>"))?;
    require_schema(
        &fields[0],
        crate::preserves_rail::NODE_CONTROL_INGRESS_RECEIPT_SCHEMA,
        "node control ingress receipt",
    )?;
    record_string(&fields[1], "decision")
}

pub fn control_ingress_envelope(input: &ControlIngressEnvelopeInput<'_>) -> Result<ControlIngressEnvelope> {
    control_ingress_envelope_for_transport(input, LOCAL_CONTROL_INGRESS_TRANSPORT, "iroh-local-ingress")
}

pub fn control_live_ingress_envelope(input: &ControlIngressEnvelopeInput<'_>) -> Result<ControlIngressEnvelope> {
    control_ingress_envelope_for_transport(input, LIVE_CONTROL_INGRESS_TRANSPORT, "live-iroh-gossip")
}

fn control_ingress_envelope_for_transport(
    input: &ControlIngressEnvelopeInput<'_>,
    transport: &str,
    transport_check: &str,
) -> Result<ControlIngressEnvelope> {
    let request = crate::node_runtime::parse_control_request(input.request_value)?;
    validate_node_id(input.from_peer)?;
    validate_node_id(input.to_node)?;
    validate_node_id(input.topic)?;
    validate_node_id(transport)?;
    validate_ingress_refs(input.peer_bootstrap_refs, "node control ingress peer bootstrap ref")?;
    validate_ingress_refs(input.authority_refs, "node control ingress authority ref")?;
    validate_ingress_refs(input.policy_refs, "node control ingress policy ref")?;
    validate_ingress_refs(input.resource_refs, "node control ingress resource ref")?;
    validate_ingress_refs(input.evidence_refs, "node control ingress evidence ref")?;
    let scope_ref = crate::delivery_idempotency::remote_topic_scope_ref(input.topic, input.to_node)?;
    let operation = crate::delivery_idempotency::derive_operation_id(crate::delivery_idempotency::OperationIdInput {
        scope_ref,
        producer: input.from_peer.to_string(),
        consumer: input.to_node.to_string(),
        sequence: input.sequence,
        intent: "node-control-ingress".to_string(),
        payload_ref: request.request_ref.clone(),
        policy_refs: input.policy_refs.to_vec(),
    })?;
    let value = ingress_envelope_value(input, &request, &operation.operation_ref, transport, transport_check)?;
    parse_control_ingress_envelope(&value)
}

pub async fn publish_control_live_ingress(
    input: &ControlLiveIngressPublishInput<'_>,
) -> Result<ControlLiveIngressPublish> {
    validate_node_id(input.node_id)?;
    let envelope = parse_control_ingress_envelope(input.envelope_value)?;
    let mut diagnostics = Vec::new();
    if envelope.transport != LIVE_CONTROL_INGRESS_TRANSPORT {
        diagnostics.push(format!(
            "node control live publish requires transport {LIVE_CONTROL_INGRESS_TRANSPORT}, got {}",
            envelope.transport
        ));
    }
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    if diagnostics.is_empty() {
        input
            .sender
            .broadcast(crate::preserves_rail::canonical_bytes(&envelope.value)?.into())
            .await
            .map_err(|error| MoltenError::invalid_harness(format!("live Iroh node control publish failed: {error}")))?;
    }
    let receipt_value = live_transport_receipt_value(&LiveTransportReceiptValueInput {
        operation: "publish",
        decision,
        node_id: input.node_id,
        delivered_from: None,
        envelope: &envelope,
        ingress_receipt_ref: None,
        diagnostics: &diagnostics,
    })?;
    let transport_receipt_ref = crate::preserves_rail::canonical_hash(&receipt_value)?;
    Ok(ControlLiveIngressPublish {
        envelope_ref: envelope.envelope_ref,
        transport_receipt_ref,
        transport_receipt_value: receipt_value,
    })
}

pub fn receive_control_live_ingress_event(
    state_root: &Path,
    event: &iroh_gossip::api::Event,
    topic: &str,
    receiver_node: &str,
) -> Result<Option<ControlLiveIngressReceive>> {
    match event {
        iroh_gossip::api::Event::Received(message) => {
            receive_control_live_ingress_bytes(&ControlLiveIngressReceiveBytesInput {
                state_root,
                topic,
                receiver_node,
                delivered_from: &format!("iroh:{}", message.delivered_from),
                bytes: message.content.as_ref(),
            })
            .map(Some)
        }
        iroh_gossip::api::Event::NeighborUp(_)
        | iroh_gossip::api::Event::NeighborDown(_)
        | iroh_gossip::api::Event::Lagged => Ok(None),
    }
}

pub fn receive_control_live_ingress_bytes(
    input: &ControlLiveIngressReceiveBytesInput<'_>,
) -> Result<ControlLiveIngressReceive> {
    validate_state_root(input.state_root)?;
    validate_node_id(input.topic)?;
    validate_node_id(input.receiver_node)?;
    validate_node_id(input.delivered_from)?;
    ensure_state_layout(input.state_root)?;
    let value = crate::preserves_rail::parse_canonical_bytes(input.bytes)?;
    let envelope = parse_control_ingress_envelope(&value)?;
    let mut diagnostics = live_receive_diagnostics(input, &envelope);
    write_ingress_envelope_and_verify(input.state_root, input.topic, &envelope)?;
    import_artifact(input.state_root, &value)?;
    let delivered = if diagnostics.is_empty() {
        deliver_control_ingress(&ControlIngressDeliverInput {
            state_root: input.state_root,
            topic: input.topic,
            envelope_ref: &envelope.envelope_ref,
        })?
    } else {
        denied_live_ingress_delivery(input.state_root, &envelope, &diagnostics)?
    };
    let ingress_decision = ingress_receipt_decision(&delivered.ingress_receipt_value)?;
    if ingress_decision != "pass" {
        diagnostics.push(format!("node control live ingress delivery decision {ingress_decision}"));
    }
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let receipt_value = live_transport_receipt_value(&LiveTransportReceiptValueInput {
        operation: "receive",
        decision,
        node_id: input.receiver_node,
        delivered_from: Some(input.delivered_from),
        envelope: &envelope,
        ingress_receipt_ref: Some(&delivered.ingress_receipt_ref),
        diagnostics: &diagnostics,
    })?;
    let transport_receipt_ref = crate::preserves_rail::canonical_hash(&receipt_value)?;
    write_preserves(
        &control_live_transport_receipt_path(input.state_root, &envelope.envelope_ref, "receive"),
        &receipt_value,
    )?;
    import_artifact(input.state_root, &receipt_value)?;
    Ok(ControlLiveIngressReceive {
        envelope_ref: envelope.envelope_ref,
        transport_receipt_ref,
        transport_receipt_value: receipt_value,
        ingress_receipt_ref: delivered.ingress_receipt_ref,
        ingress_receipt_value: delivered.ingress_receipt_value,
        has_enqueued: delivered.has_enqueued,
    })
}

fn envelope_for_loopback(input: &ControlLiveLoopbackInput<'_>) -> Result<ControlIngressEnvelope> {
    control_live_ingress_envelope(&ControlIngressEnvelopeInput {
        request_value: input.request_value,
        from_peer: input.from_peer,
        to_node: input.to_node,
        topic: input.topic,
        sequence: input.sequence,
        peer_bootstrap_refs: input.peer_bootstrap_refs,
        authority_refs: input.authority_refs,
        policy_refs: input.policy_refs,
        resource_refs: input.resource_refs,
        evidence_refs: input.evidence_refs,
    })
}

pub async fn control_live_iroh_loopback(input: &ControlLiveLoopbackInput<'_>) -> Result<ControlLiveLoopback> {
    validate_state_root(input.state_root)?;
    ensure_state_layout(input.state_root)?;
    let envelope = envelope_for_loopback(input)?;
    let topic_id = control_live_topic_id(input.topic);
    let lookup = iroh::address_lookup::memory::MemoryLookup::new();
    let receiver_endpoint = live_gossip_endpoint(&lookup, None).await?;
    let sender_endpoint = live_gossip_endpoint(&lookup, None).await?;
    lookup.add_endpoint_info(receiver_endpoint.addr());
    lookup.add_endpoint_info(sender_endpoint.addr());
    let receiver_id = receiver_endpoint.id();
    let sender_id = sender_endpoint.id();
    let receiver_gossip = iroh_gossip::Gossip::builder().spawn(receiver_endpoint.clone());
    let sender_gossip = iroh_gossip::Gossip::builder().spawn(sender_endpoint.clone());
    let receiver_router = iroh::protocol::Router::builder(receiver_endpoint)
        .accept(iroh_gossip::ALPN, receiver_gossip.clone())
        .spawn();
    let sender_router = iroh::protocol::Router::builder(sender_endpoint)
        .accept(iroh_gossip::ALPN, sender_gossip.clone())
        .spawn();
    let mut receiver_topic = receiver_gossip
        .subscribe(topic_id, vec![sender_id])
        .await
        .map_err(|error| MoltenError::invalid_harness(format!("live Iroh receiver subscribe failed: {error}")))?;
    let sender_topic = sender_gossip
        .subscribe_and_join(topic_id, vec![receiver_id])
        .await
        .map_err(|error| MoltenError::invalid_harness(format!("live Iroh sender join failed: {error}")))?;
    let (sender, _receiver_unused) = sender_topic.split();
    receiver_topic
        .joined()
        .await
        .map_err(|error| MoltenError::invalid_harness(format!("live Iroh receiver join failed: {error}")))?;
    let published = publish_control_live_ingress(&ControlLiveIngressPublishInput {
        sender: &sender,
        envelope_value: &envelope.value,
        node_id: input.from_peer,
    })
    .await?;
    let received = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        receive_first_live_ingress_event(input.state_root, &mut receiver_topic, input.topic, input.to_node),
    )
    .await
    .map_err(|_| MoltenError::invalid_harness("live Iroh node control loopback timed out waiting for envelope"))??;
    receiver_router
        .shutdown()
        .await
        .map_err(|error| MoltenError::invalid_harness(format!("live Iroh receiver router shutdown failed: {error}")))?;
    sender_router
        .shutdown()
        .await
        .map_err(|error| MoltenError::invalid_harness(format!("live Iroh sender router shutdown failed: {error}")))?;
    Ok(ControlLiveLoopback {
        envelope_ref: envelope.envelope_ref,
        publish_receipt_ref: published.transport_receipt_ref,
        publish_receipt_value: published.transport_receipt_value,
        receive_receipt_ref: received.transport_receipt_ref,
        receive_receipt_value: received.transport_receipt_value,
        ingress_receipt_ref: received.ingress_receipt_ref,
        has_enqueued: received.has_enqueued,
    })
}

pub fn preflight_control_live_send(input: &ControlLiveSendInput<'_>) -> Result<ControlLiveSendPreflight> {
    if let Some(state_root) = input.state_root {
        validate_state_root(state_root)?;
        ensure_state_layout(state_root)?;
    }
    validate_node_id(input.from_peer)?;
    validate_live_send_timeout(input.join_timeout_ms)?;
    validate_live_send_attempts(input.max_attempts)?;
    if let Some(operation_ref) = input.expected_operation_ref {
        validate_ingress_ref(operation_ref, "node control live send operation id")?;
    }
    if let Some(node) = input.expected_receiver_node {
        validate_node_id(node)?;
    }
    if let Some(topic) = input.expected_topic {
        validate_node_id(topic)?;
    }
    if let Some(endpoint) = input.expected_endpoint {
        validate_node_id(endpoint)?;
    }
    let ticket = parse_control_live_ticket(input.receiver_ticket_value)?;
    let envelope = control_live_ingress_envelope(&ControlIngressEnvelopeInput {
        request_value: input.request_value,
        from_peer: input.from_peer,
        to_node: &ticket.node_id,
        topic: &ticket.topic,
        sequence: input.sequence,
        peer_bootstrap_refs: input.peer_bootstrap_refs,
        authority_refs: input.authority_refs,
        policy_refs: input.policy_refs,
        resource_refs: input.resource_refs,
        evidence_refs: input.evidence_refs,
    })?;
    let mut diagnostics = Vec::with_capacity(8);
    if let Some(operation_ref) = input.expected_operation_ref
        && operation_ref != envelope.operation_ref
    {
        diagnostics.push(format!(
            "node control live send operation-id {operation_ref} does not match derived {}",
            envelope.operation_ref
        ));
    }
    diagnostics.extend(live_send_ticket_diagnostics(input, &ticket));
    if let Some(state_root) = input.state_root {
        diagnostics.extend(live_send_state_root_evidence_diagnostics(state_root, input, &envelope)?);
    }
    if ticket.address_refs.is_empty() {
        diagnostics.push(
            "node control live send ticket has no endpoint addresses; import a bound live ticket with live-ticket-import or use serve --live-ticket-out"
                .to_string(),
        );
    } else if let Err(error) = live_ticket_endpoint_addr(&ticket) {
        diagnostics.push(format!(
            "node control live send ticket address unsupported or malformed: {error}; import a fresh live ticket with live-ticket-import"
        ));
    }
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    Ok(ControlLiveSendPreflight {
        decision: decision.to_string(),
        envelope_ref: envelope.envelope_ref,
        operation_ref: envelope.operation_ref,
        receiver_ticket_ref: ticket.ticket_ref,
        diagnostics,
    })
}

pub async fn send_control_live_ingress(input: &ControlLiveSendInput<'_>) -> Result<ControlLiveSend> {
    validate_send_input(input)?;
    let ticket = parse_control_live_ticket(input.receiver_ticket_value)?;
    let envelope = send_envelope(input, &ticket)?;
    if let Some(operation_ref) = input.expected_operation_ref
        && operation_ref != envelope.operation_ref
    {
        let diagnostics = vec![format!(
            "node control live send operation-id {operation_ref} does not match derived {}",
            envelope.operation_ref
        )];
        return denied_control_live_send_with_diagnostics(DeniedLiveSendInput {
            input,
            ticket: &ticket,
            envelope,
            diagnostics,
            retry_receipt_refs: Vec::new(),
            retry_receipt_values: Vec::new(),
        });
    }
    let receiver_addr = match send_receiver_addr(input, &ticket, &envelope)? {
        Ok(addr) => addr,
        Err(diagnostics) => {
            return denied_control_live_send_with_diagnostics(DeniedLiveSendInput {
                input,
                ticket: &ticket,
                envelope,
                diagnostics,
                retry_receipt_refs: Vec::new(),
                retry_receipt_values: Vec::new(),
            });
        }
    };
    if let Some(state_root) = input.state_root
        && let Some(duplicate) = duplicate_control_live_send(input, state_root, &ticket, &envelope)?
    {
        return Ok(duplicate);
    }
    let retries = publish_with_retries(input, &receiver_addr, &ticket, &envelope).await?;
    let Some(published) = retries.published else {
        return denied_control_live_send_with_diagnostics(DeniedLiveSendInput {
            input,
            ticket: &ticket,
            envelope,
            diagnostics: retries.diagnostics,
            retry_receipt_refs: retries.retry_receipt_refs,
            retry_receipt_values: retries.retry_receipt_values,
        });
    };
    finish_send(FinishSendInput {
        input,
        ticket: &ticket,
        envelope,
        published,
        retry_receipt_refs: retries.retry_receipt_refs,
        retry_receipt_values: retries.retry_receipt_values,
    })
}

#[derive(Debug)]
struct SendRetryOutcome {
    published: Option<ControlLiveIngressPublish>,
    retry_receipt_refs: Vec<String>,
    retry_receipt_values: Vec<IoValue>,
    diagnostics: Vec<String>,
}

#[derive(Debug)]
struct FinishSendInput<'a> {
    input: &'a ControlLiveSendInput<'a>,
    ticket: &'a ControlLiveTicket,
    envelope: ControlIngressEnvelope,
    published: ControlLiveIngressPublish,
    retry_receipt_refs: Vec<String>,
    retry_receipt_values: Vec<IoValue>,
}

fn validate_send_input(input: &ControlLiveSendInput<'_>) -> Result<()> {
    if let Some(state_root) = input.state_root {
        validate_state_root(state_root)?;
        ensure_state_layout(state_root)?;
    }
    validate_node_id(input.from_peer)?;
    validate_live_send_timeout(input.join_timeout_ms)?;
    validate_live_send_attempts(input.max_attempts)?;
    if let Some(operation_ref) = input.expected_operation_ref {
        validate_ingress_ref(operation_ref, "node control live send operation id")?;
    }
    if let Some(node) = input.expected_receiver_node {
        validate_node_id(node)?;
    }
    if let Some(topic) = input.expected_topic {
        validate_node_id(topic)?;
    }
    if let Some(endpoint) = input.expected_endpoint {
        validate_node_id(endpoint)?;
    }
    Ok(())
}

fn send_envelope(input: &ControlLiveSendInput<'_>, ticket: &ControlLiveTicket) -> Result<ControlIngressEnvelope> {
    control_live_ingress_envelope(&ControlIngressEnvelopeInput {
        request_value: input.request_value,
        from_peer: input.from_peer,
        to_node: &ticket.node_id,
        topic: &ticket.topic,
        sequence: input.sequence,
        peer_bootstrap_refs: input.peer_bootstrap_refs,
        authority_refs: input.authority_refs,
        policy_refs: input.policy_refs,
        resource_refs: input.resource_refs,
        evidence_refs: input.evidence_refs,
    })
}

fn send_receiver_addr(
    input: &ControlLiveSendInput<'_>,
    ticket: &ControlLiveTicket,
    envelope: &ControlIngressEnvelope,
) -> Result<std::result::Result<iroh::EndpointAddr, Vec<String>>> {
    let mut diagnostics = live_send_ticket_diagnostics(input, ticket);
    if let Some(state_root) = input.state_root {
        diagnostics.extend(live_send_state_root_evidence_diagnostics(state_root, input, envelope)?);
    }
    if ticket.address_refs.is_empty() {
        diagnostics.push(
            "node control live send ticket has no endpoint addresses; import a bound live ticket with live-ticket-import or use serve --live-ticket-out"
                .to_string(),
        );
        return Ok(Err(diagnostics));
    }
    match live_ticket_endpoint_addr(ticket) {
        Ok(addr) if diagnostics.is_empty() => Ok(Ok(addr)),
        Ok(_) => Ok(Err(diagnostics)),
        Err(error) => {
            diagnostics.push(format!(
                "node control live send ticket address unsupported or malformed: {error}; import a fresh live ticket with live-ticket-import"
            ));
            Ok(Err(diagnostics))
        }
    }
}

async fn publish_with_retries(
    input: &ControlLiveSendInput<'_>,
    receiver_addr: &iroh::EndpointAddr,
    ticket: &ControlLiveTicket,
    envelope: &ControlIngressEnvelope,
) -> Result<SendRetryOutcome> {
    let attempt_capacity = usize::try_from(input.max_attempts)
        .map_err(|_| MoltenError::invalid_harness("node control live send attempts exceed usize capacity"))?;
    let mut retry_receipt_refs = Vec::with_capacity(attempt_capacity);
    let mut retry_receipt_values = Vec::with_capacity(attempt_capacity);
    let mut diagnostics = Vec::with_capacity(attempt_capacity);
    let mut published = None;
    for attempt in 1..=input.max_attempts {
        match attempt_control_live_send(input, receiver_addr, envelope).await? {
            Ok(receipt) => {
                published = Some(receipt);
                break;
            }
            Err(diagnostic) => {
                let attempt_diagnostics = vec![format!(
                    "node control live send attempt {attempt}/{} failed: {diagnostic}",
                    input.max_attempts
                )];
                diagnostics.extend(attempt_diagnostics.iter().cloned());
                let retry_value = live_send_retry_receipt_value(&LiveSendRetryReceiptValueInput {
                    decision: if attempt == input.max_attempts { "deny" } else { "fail" },
                    attempt,
                    max_attempts: input.max_attempts,
                    from_peer: input.from_peer,
                    ticket,
                    envelope,
                    diagnostics: &attempt_diagnostics,
                })?;
                let retry_ref = crate::preserves_rail::canonical_hash(&retry_value)?;
                if let Some(state_root) = input.state_root {
                    write_preserves(&control_live_send_retry_receipt_path(state_root, &retry_ref), &retry_value)?;
                    import_artifact(state_root, &retry_value)?;
                }
                retry_receipt_refs.push(retry_ref);
                retry_receipt_values.push(retry_value);
            }
        }
    }
    Ok(SendRetryOutcome {
        published,
        retry_receipt_refs,
        retry_receipt_values,
        diagnostics,
    })
}

fn finish_send(input: FinishSendInput<'_>) -> Result<ControlLiveSend> {
    let send_receipt_value = live_send_receipt_value(&LiveSendReceiptValueInput {
        decision: "pass",
        from_peer: input.input.from_peer,
        ticket: input.ticket,
        envelope: &input.envelope,
        transport_receipt_ref: Some(&input.published.transport_receipt_ref),
        diagnostics: &[],
    })?;
    let send_receipt_ref = crate::preserves_rail::canonical_hash(&send_receipt_value)?;
    if let Some(state_root) = input.input.state_root {
        import_artifact(state_root, input.input.receiver_ticket_value)?;
        write_ingress_envelope_and_verify(state_root, &input.ticket.topic, &input.envelope)?;
        import_artifact(state_root, &input.envelope.value)?;
        write_preserves(
            &control_live_transport_receipt_path(state_root, &input.envelope.envelope_ref, "send"),
            &input.published.transport_receipt_value,
        )?;
        import_artifact(state_root, &input.published.transport_receipt_value)?;
        write_preserves(&control_live_send_receipt_path(state_root, &send_receipt_ref), &send_receipt_value)?;
        import_artifact(state_root, &send_receipt_value)?;
    }
    Ok(ControlLiveSend {
        envelope_ref: input.envelope.envelope_ref,
        envelope_value: input.envelope.value,
        operation_ref: input.envelope.operation_ref,
        receiver_ticket_ref: input.ticket.ticket_ref.clone(),
        receiver_endpoint_id: input.ticket.live_endpoint_id.clone(),
        transport_receipt_ref: Some(input.published.transport_receipt_ref),
        transport_receipt_value: Some(input.published.transport_receipt_value),
        retry_receipt_refs: input.retry_receipt_refs,
        retry_receipt_values: input.retry_receipt_values,
        duplicate_receipt_ref: None,
        duplicate_receipt_value: None,
        send_receipt_ref,
        send_receipt_value,
    })
}

async fn attempt_control_live_send(
    input: &ControlLiveSendInput<'_>,
    receiver_addr: &iroh::EndpointAddr,
    envelope: &ControlIngressEnvelope,
) -> Result<std::result::Result<ControlLiveIngressPublish, String>> {
    let lookup = iroh::address_lookup::memory::MemoryLookup::new();
    lookup.add_endpoint_info(receiver_addr.clone());
    let sender_endpoint = match live_gossip_endpoint(&lookup, None).await {
        Ok(endpoint) => endpoint,
        Err(error) => return Ok(Err(format!("live Iroh sender endpoint failed: {error}"))),
    };
    lookup.add_endpoint_info(sender_endpoint.addr());
    let sender_gossip = iroh_gossip::Gossip::builder().spawn(sender_endpoint.clone());
    let sender_router = iroh::protocol::Router::builder(sender_endpoint)
        .accept(iroh_gossip::ALPN, sender_gossip.clone())
        .spawn();
    let topic_id = control_live_topic_id(&envelope.topic);
    let join_timeout = std::time::Duration::from_millis(input.join_timeout_ms);
    let join_result =
        tokio::time::timeout(join_timeout, sender_gossip.subscribe_and_join(topic_id, vec![receiver_addr.id])).await;
    let mut result = match join_result {
        Err(_) => Err(format!(
            "live Iroh node control send timed out joining topic {} at endpoint {}",
            envelope.topic, receiver_addr.id
        )),
        Ok(Err(error)) => Err(format!(
            "live Iroh node control send join failed for topic {} endpoint {}: {error}",
            envelope.topic, receiver_addr.id
        )),
        Ok(Ok(sender_topic)) => {
            let (sender, _receiver_unused) = sender_topic.split();
            let published = publish_control_live_ingress(&ControlLiveIngressPublishInput {
                sender: &sender,
                envelope_value: &envelope.value,
                node_id: input.from_peer,
            })
            .await;
            if published.is_ok() {
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
            published.map_err(|error| format!("live Iroh node control send publish failed: {error}"))
        }
    };
    if let Err(error) = sender_router.shutdown().await {
        let diagnostic = format!("live Iroh sender router shutdown failed: {error}");
        if result.is_ok() {
            return Ok(Err(diagnostic));
        }
        result = result.map_err(|existing| format!("{existing}; {diagnostic}"));
    }
    Ok(result)
}

fn duplicate_control_live_send(
    input: &ControlLiveSendInput<'_>,
    state_root: &Path,
    ticket: &ControlLiveTicket,
    envelope: &ControlIngressEnvelope,
) -> Result<Option<ControlLiveSend>> {
    let transport_receipt_value = live_transport_receipt_value(&LiveTransportReceiptValueInput {
        operation: "publish",
        decision: "pass",
        node_id: input.from_peer,
        delivered_from: None,
        envelope,
        ingress_receipt_ref: None,
        diagnostics: &[],
    })?;
    let transport_receipt_ref = crate::preserves_rail::canonical_hash(&transport_receipt_value)?;
    let send_receipt_value = live_send_receipt_value(&LiveSendReceiptValueInput {
        decision: "pass",
        from_peer: input.from_peer,
        ticket,
        envelope,
        transport_receipt_ref: Some(&transport_receipt_ref),
        diagnostics: &[],
    })?;
    let send_receipt_ref = crate::preserves_rail::canonical_hash(&send_receipt_value)?;
    let send_path = control_live_send_receipt_path(state_root, &send_receipt_ref);
    if !send_path.exists() {
        return Ok(None);
    }
    let prior_send_value = read_preserves(&send_path)?;
    let prior_send = parse_control_live_send_receipt(&prior_send_value)?;
    if prior_send.receipt_ref != send_receipt_ref {
        return Err(MoltenError::invalid_harness("node control live send prior receipt path is stale"));
    }
    if prior_send.decision != "pass" || prior_send.envelope_ref != envelope.envelope_ref {
        return Ok(None);
    }
    let diagnostics = vec![format!(
        "node control live send duplicate operation {} reused prior send receipt {send_receipt_ref}",
        envelope.operation_ref
    )];
    let duplicate_receipt_value = live_send_duplicate_receipt_value(&LiveSendDuplicateReceiptValueInput {
        from_peer: input.from_peer,
        ticket,
        envelope,
        prior_send_receipt_ref: &send_receipt_ref,
        diagnostics: &diagnostics,
    })?;
    let duplicate_receipt_ref = crate::preserves_rail::canonical_hash(&duplicate_receipt_value)?;
    write_preserves(
        &control_live_send_duplicate_receipt_path(state_root, &duplicate_receipt_ref),
        &duplicate_receipt_value,
    )?;
    import_artifact(state_root, &duplicate_receipt_value)?;
    Ok(Some(ControlLiveSend {
        envelope_ref: envelope.envelope_ref.clone(),
        envelope_value: envelope.value.clone(),
        operation_ref: envelope.operation_ref.clone(),
        receiver_ticket_ref: ticket.ticket_ref.clone(),
        receiver_endpoint_id: ticket.live_endpoint_id.clone(),
        transport_receipt_ref: Some(transport_receipt_ref),
        transport_receipt_value: Some(transport_receipt_value),
        retry_receipt_refs: Vec::new(),
        retry_receipt_values: Vec::new(),
        duplicate_receipt_ref: Some(duplicate_receipt_ref),
        duplicate_receipt_value: Some(duplicate_receipt_value),
        send_receipt_ref,
        send_receipt_value: prior_send_value,
    }))
}

fn denied_control_live_send_with_diagnostics(denied: DeniedLiveSendInput<'_>) -> Result<ControlLiveSend> {
    let send_receipt_value = live_send_receipt_value(&LiveSendReceiptValueInput {
        decision: "deny",
        from_peer: denied.input.from_peer,
        ticket: denied.ticket,
        envelope: &denied.envelope,
        transport_receipt_ref: None,
        diagnostics: &denied.diagnostics,
    })?;
    let send_receipt_ref = crate::preserves_rail::canonical_hash(&send_receipt_value)?;
    if let Some(state_root) = denied.input.state_root {
        import_artifact(state_root, denied.input.receiver_ticket_value)?;
        write_ingress_envelope_and_verify(state_root, &denied.ticket.topic, &denied.envelope)?;
        import_artifact(state_root, &denied.envelope.value)?;
        write_preserves(&control_live_send_receipt_path(state_root, &send_receipt_ref), &send_receipt_value)?;
        import_artifact(state_root, &send_receipt_value)?;
    }
    Ok(ControlLiveSend {
        envelope_ref: denied.envelope.envelope_ref,
        envelope_value: denied.envelope.value,
        operation_ref: denied.envelope.operation_ref,
        receiver_ticket_ref: denied.ticket.ticket_ref.clone(),
        receiver_endpoint_id: denied.ticket.live_endpoint_id.clone(),
        transport_receipt_ref: None,
        transport_receipt_value: None,
        retry_receipt_refs: denied.retry_receipt_refs,
        retry_receipt_values: denied.retry_receipt_values,
        duplicate_receipt_ref: None,
        duplicate_receipt_value: None,
        send_receipt_ref,
        send_receipt_value,
    })
}

pub fn parse_control_live_send_receipt(value: &IoValue) -> Result<ControlLiveSendReceipt> {
    let fields = value
        .collect_simple_record("node-control-live-send-receipt-v1", Some(13))
        .ok_or_else(|| MoltenError::invalid_harness("expected <node-control-live-send-receipt-v1 ...>"))?;
    require_schema(
        &fields[0],
        crate::preserves_rail::NODE_CONTROL_LIVE_SEND_RECEIPT_SCHEMA,
        "node control live send receipt",
    )?;
    let transport_receipt_ref = record_optional_string(&fields[10], "transport-receipt")?;
    if let Some(reference) = transport_receipt_ref.as_ref() {
        validate_ingress_ref(reference, "node control live send transport receipt ref")?;
    }
    Ok(ControlLiveSendReceipt {
        receipt_ref: crate::preserves_rail::canonical_hash(value)?,
        decision: record_string(&fields[1], "decision")?,
        topic: record_string(&fields[3], "topic")?,
        from_peer: record_string(&fields[4], "from-peer")?,
        to_node: record_string(&fields[5], "to-node")?,
        receiver_ticket_ref: record_ref_string(&fields[6], "receiver-ticket")?,
        receiver_endpoint_id: record_string(&fields[7], "receiver-endpoint")?,
        receiver_address_refs: record_strings(&fields[8], "receiver-addresses")?,
        envelope_ref: record_ref_string(&fields[9], "envelope")?,
        transport_receipt_ref,
        diagnostics: record_strings(&fields[11], "diagnostics")?,
        value: value.clone(),
    })
}

struct FlowChecks<'a> {
    ticket: &'a ControlLiveTicket,
    admission: &'a ControlLivePeerAdmission,
    authority: &'a ControlAuthorityGrant,
    send: &'a ControlLiveSendReceipt,
    service_receipt_ref: &'a str,
}

struct FlowRefs {
    receive_receipt_refs: Vec<String>,
    listener_receipt_ref: Option<String>,
}

impl FlowChecks<'_> {
    fn note_bindings(&self, diagnostics: &mut impl VecSink<String>) {
        if self.admission.ticket_ref != self.ticket.ticket_ref {
            diagnostics.push_item("node control live workflow admission does not bind receiver ticket".to_string());
        }
        if self.admission.decision != "pass" {
            diagnostics.push_item(format!("node control live workflow admission decision {}", self.admission.decision));
        }
        if self.authority.peer_id != self.admission.peer_id {
            diagnostics
                .push_item("node control live workflow authority grant peer does not match admission".to_string());
        }
        if self.authority.node_id != self.ticket.node_id {
            diagnostics.push_item("node control live workflow authority grant node does not match ticket".to_string());
        }
        if self.send.receiver_ticket_ref != self.ticket.ticket_ref {
            diagnostics.push_item("node control live workflow send receipt does not bind receiver ticket".to_string());
        }
        if self.send.from_peer != self.admission.peer_id {
            diagnostics.push_item("node control live workflow send peer does not match admission".to_string());
        }
        if self.send.to_node != self.ticket.node_id || self.send.topic != self.ticket.topic {
            diagnostics.push_item("node control live workflow send destination does not match ticket".to_string());
        }
        if self.send.decision != "pass" {
            diagnostics.push_item(format!("node control live workflow send decision {}", self.send.decision));
        }
    }

    fn collect_refs(
        &self,
        input: &ControlLiveWorkflowInput<'_>,
        diagnostics: &mut impl VecSink<String>,
    ) -> Result<FlowRefs> {
        let mut receive_receipt_refs = Vec::with_capacity(input.receive_receipt_values.len());
        for receive_value in input.receive_receipt_values {
            let (receipt_ref, operation, envelope_ref) = live_transport_receipt_ref(receive_value)?;
            if operation != "receive" {
                diagnostics.push_item(format!(
                    "node control live workflow transport receipt operation {operation} is not receive"
                ));
            }
            if envelope_ref != self.send.envelope_ref {
                diagnostics
                    .push_item("node control live workflow receive envelope does not match send envelope".to_string());
            }
            receive_receipt_refs.push(receipt_ref);
        }
        if receive_receipt_refs.is_empty() {
            diagnostics.push_item("node control live workflow missing receive receipt".to_string());
        }
        let listener_receipt_ref = if let Some(listener_value) = input.listener_receipt_value {
            let (listener_ref, listener_transport_refs, listener_service_ref) =
                live_listener_receipt_refs(listener_value)?;
            for receive_ref in &receive_receipt_refs {
                if !listener_transport_refs.iter().any(|reference| reference == receive_ref) {
                    diagnostics
                        .push_item("node control live workflow listener does not bind receive receipt".to_string());
                }
            }
            if listener_service_ref != self.service_receipt_ref {
                diagnostics.push_item(
                    "node control live workflow listener service run does not match service receipt".to_string(),
                );
            }
            Some(listener_ref)
        } else {
            None
        };
        Ok(FlowRefs {
            receive_receipt_refs,
            listener_receipt_ref,
        })
    }
}

fn import_flow_values(
    state_root: &Path,
    input: &ControlLiveWorkflowInput<'_>,
    receipt_ref: &str,
    receipt_value: &IoValue,
) -> Result<()> {
    import_artifact(state_root, input.receiver_ticket_value)?;
    import_artifact(state_root, input.peer_admission_value)?;
    import_artifact(state_root, input.authority_grant_value)?;
    import_artifact(state_root, input.send_receipt_value)?;
    for receive_value in input.receive_receipt_values {
        import_artifact(state_root, receive_value)?;
    }
    if let Some(listener_value) = input.listener_receipt_value {
        import_artifact(state_root, listener_value)?;
    }
    import_artifact(state_root, input.service_receipt_value)?;
    write_preserves(&control_live_workflow_receipt_path(state_root, receipt_ref), receipt_value)?;
    import_artifact(state_root, receipt_value)?;
    Ok(())
}

pub fn control_live_workflow_receipt(input: &ControlLiveWorkflowInput<'_>) -> Result<ControlLiveWorkflowReceipt> {
    if let Some(state_root) = input.state_root {
        validate_state_root(state_root)?;
        ensure_state_layout(state_root)?;
    }
    let ticket = parse_control_live_ticket(input.receiver_ticket_value)?;
    let admission = parse_control_live_peer_admission(input.peer_admission_value)?;
    let authority = parse_control_authority_grant(input.authority_grant_value)?;
    let send = parse_control_live_send_receipt(input.send_receipt_value)?;
    let service_receipt_ref = service_run_receipt_ref(input.service_receipt_value)?;
    let checks = FlowChecks {
        ticket: &ticket,
        admission: &admission,
        authority: &authority,
        send: &send,
        service_receipt_ref: &service_receipt_ref,
    };
    let mut diagnostics = Vec::with_capacity(input.receive_receipt_values.len().saturating_add(8));
    checks.note_bindings(&mut diagnostics);
    let refs = checks.collect_refs(input, &mut diagnostics)?;
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let receipt_value = live_workflow_receipt_value(&LiveWorkflowReceiptValueInput {
        decision,
        ticket: &ticket,
        admission: &admission,
        authority: &authority,
        send: &send,
        receive_receipt_refs: &refs.receive_receipt_refs,
        listener_receipt_ref: refs.listener_receipt_ref.as_deref(),
        service_receipt_ref: &service_receipt_ref,
        diagnostics: &diagnostics,
    })?;
    let receipt_ref = crate::preserves_rail::canonical_hash(&receipt_value)?;
    if let Some(state_root) = input.state_root {
        import_flow_values(state_root, input, &receipt_ref, &receipt_value)?;
    }
    Ok(ControlLiveWorkflowReceipt {
        receipt_ref,
        receipt_value,
        decision: decision.to_string(),
        diagnostics,
    })
}

fn service_run_receipt_ref(value: &IoValue) -> Result<String> {
    if let Some(fields) = value.collect_simple_record("node-control-service-run-receipt-v1", Some(17)) {
        require_schema(
            &fields[0],
            crate::preserves_rail::NODE_CONTROL_SERVICE_RUN_RECEIPT_SCHEMA,
            "node control service run receipt",
        )?;
        return crate::preserves_rail::canonical_hash(value);
    }
    if let Some(fields) = value.collect_simple_record("node-control-service-run-receipt-v1", Some(15)) {
        require_schema(
            &fields[0],
            crate::preserves_rail::NODE_CONTROL_SERVICE_RUN_RECEIPT_SCHEMA,
            "node control service run receipt",
        )?;
        return crate::preserves_rail::canonical_hash(value);
    }
    Err(MoltenError::invalid_harness("expected <node-control-service-run-receipt-v1 ...>"))
}

fn live_transport_receipt_ref(value: &IoValue) -> Result<(String, String, String)> {
    let fields = value
        .collect_simple_record("node-control-live-transport-receipt-v1", Some(11))
        .ok_or_else(|| MoltenError::invalid_harness("expected <node-control-live-transport-receipt-v1 ...>"))?;
    require_schema(
        &fields[0],
        crate::preserves_rail::NODE_CONTROL_LIVE_TRANSPORT_RECEIPT_SCHEMA,
        "node control live transport receipt",
    )?;
    Ok((
        crate::preserves_rail::canonical_hash(value)?,
        record_string(&fields[1], "operation")?,
        record_ref_string(&fields[7], "envelope")?,
    ))
}

fn live_listener_receipt_refs(value: &IoValue) -> Result<(String, Vec<String>, String)> {
    let fields = value
        .collect_simple_record("node-control-live-listener-receipt-v1", Some(14))
        .ok_or_else(|| MoltenError::invalid_harness("expected <node-control-live-listener-receipt-v1 ...>"))?;
    require_schema(
        &fields[0],
        crate::preserves_rail::NODE_CONTROL_LIVE_LISTENER_RECEIPT_SCHEMA,
        "node control live listener receipt",
    )?;
    Ok((
        crate::preserves_rail::canonical_hash(value)?,
        record_ref_strings(&fields[9], "transport-receipts")?,
        record_ref_string(&fields[11], "service-run")?,
    ))
}

pub async fn serve_control_live_listener(input: &ControlLiveServeInput<'_>) -> Result<ControlLiveServe> {
    validate_state_root(input.state_root)?;
    validate_node_id(input.topic)?;
    validate_listener_event_limit(input.max_events)?;
    validate_loop_request_limit(input.max_requests_per_tick)?;
    ensure_state_layout(input.state_root)?;
    let identity = crate::node_identity::parse_node_identity(&read_preserves(&input.state_root.join(IDENTITY_FILE))?)?;
    let lookup = iroh::address_lookup::memory::MemoryLookup::new();
    let endpoint = live_gossip_endpoint(&lookup, Some(stable_live_endpoint_secret(&identity))).await?;
    let bound_endpoint_id = format!("iroh:{}", endpoint.id());
    let live_ticket = live_ticket_for_bound_endpoint(input.state_root, &identity, input.topic, &endpoint.addr())?;
    lookup.add_endpoint_info(endpoint.addr());
    let gossip = iroh_gossip::Gossip::builder().spawn(endpoint.clone());
    let router = iroh::protocol::Router::builder(endpoint).accept(iroh_gossip::ALPN, gossip.clone()).spawn();
    let mut topic = gossip
        .subscribe(control_live_topic_id(input.topic), Vec::new())
        .await
        .map_err(|error| MoltenError::invalid_harness(format!("live Iroh serve subscribe failed: {error}")))?;
    let served = serve_node_control_live_listener_with_topic(
        input,
        &mut topic,
        &identity.node_id,
        &identity.endpoint_id,
        &bound_endpoint_id,
    )
    .await;
    router
        .shutdown()
        .await
        .map_err(|error| MoltenError::invalid_harness(format!("live Iroh serve router shutdown failed: {error}")))?;
    let mut served = served?;
    served.live_ticket_ref = Some(live_ticket.ticket_ref);
    served.live_ticket_value = Some(live_ticket.value);
    Ok(served)
}

pub async fn control_live_serve_listener_loopback(
    input: &ControlLiveServeLoopbackInput<'_>,
) -> Result<ControlLiveServeLoopback> {
    validate_state_root(input.state_root)?;
    ensure_state_layout(input.state_root)?;
    let envelope_input = ControlIngressEnvelopeInput {
        request_value: input.request_value,
        from_peer: input.from_peer,
        to_node: input.to_node,
        topic: input.topic,
        sequence: input.sequence,
        peer_bootstrap_refs: input.peer_bootstrap_refs,
        authority_refs: input.authority_refs,
        policy_refs: input.policy_refs,
        resource_refs: input.resource_refs,
        evidence_refs: input.evidence_refs,
    };
    let envelope = control_live_ingress_envelope(&envelope_input)?;
    let LoopbackPair {
        ticket_ref,
        ticket_value,
        bound_endpoint_id,
        mut receiver_topic,
        sender,
        receiver_router,
        sender_router,
        node_id,
        endpoint_id,
    } = loopback_pair(input.state_root, input.topic).await?;
    let published = publish_control_live_ingress(&ControlLiveIngressPublishInput {
        sender: &sender,
        envelope_value: &envelope.value,
        node_id: input.from_peer,
    })
    .await?;
    let listener_input = ControlLiveServeInput {
        state_root: input.state_root,
        topic: input.topic,
        max_events: 4,
        event_timeout_ms: 1_000,
        max_requests_per_tick: input.max_requests_per_tick,
        supervisor_policy_value: None,
    };
    let mut listener = serve_node_control_live_listener_with_topic(
        &listener_input,
        &mut receiver_topic,
        &node_id,
        &endpoint_id,
        &bound_endpoint_id,
    )
    .await?;
    listener.live_ticket_ref = Some(ticket_ref);
    listener.live_ticket_value = Some(ticket_value);
    receiver_router.shutdown().await.map_err(|error| {
        MoltenError::invalid_harness(format!("live Iroh listener receiver shutdown failed: {error}"))
    })?;
    sender_router
        .shutdown()
        .await
        .map_err(|error| MoltenError::invalid_harness(format!("live Iroh listener sender shutdown failed: {error}")))?;
    Ok(ControlLiveServeLoopback {
        envelope_ref: envelope.envelope_ref,
        publish_receipt_ref: published.transport_receipt_ref,
        listener,
    })
}

struct LoopbackPair {
    ticket_ref: String,
    ticket_value: IoValue,
    bound_endpoint_id: String,
    receiver_topic: iroh_gossip::api::GossipTopic,
    sender: iroh_gossip::api::GossipSender,
    receiver_router: iroh::protocol::Router,
    sender_router: iroh::protocol::Router,
    node_id: String,
    endpoint_id: String,
}

async fn loopback_pair(state_root: &Path, topic: &str) -> Result<LoopbackPair> {
    let identity = crate::node_identity::parse_node_identity(&read_preserves(&state_root.join(IDENTITY_FILE))?)?;
    let lookup = iroh::address_lookup::memory::MemoryLookup::new();
    let receiver_endpoint = live_gossip_endpoint(&lookup, Some(stable_live_endpoint_secret(&identity))).await?;
    let sender_endpoint = live_gossip_endpoint(&lookup, None).await?;
    let ticket = live_ticket_for_bound_endpoint(state_root, &identity, topic, &receiver_endpoint.addr())?;
    lookup.add_endpoint_info(receiver_endpoint.addr());
    lookup.add_endpoint_info(sender_endpoint.addr());
    let receiver_id = receiver_endpoint.id();
    let sender_id = sender_endpoint.id();
    let bound_endpoint_id = format!("iroh:{receiver_id}");
    let receiver_gossip = iroh_gossip::Gossip::builder().spawn(receiver_endpoint.clone());
    let sender_gossip = iroh_gossip::Gossip::builder().spawn(sender_endpoint.clone());
    let receiver_router = iroh::protocol::Router::builder(receiver_endpoint)
        .accept(iroh_gossip::ALPN, receiver_gossip.clone())
        .spawn();
    let sender_router = iroh::protocol::Router::builder(sender_endpoint)
        .accept(iroh_gossip::ALPN, sender_gossip.clone())
        .spawn();
    let topic_id = control_live_topic_id(topic);
    let receiver_topic = receiver_gossip.subscribe(topic_id, vec![sender_id]).await.map_err(|error| {
        MoltenError::invalid_harness(format!("live Iroh listener receiver subscribe failed: {error}"))
    })?;
    let sender_topic = sender_gossip
        .subscribe_and_join(topic_id, vec![receiver_id])
        .await
        .map_err(|error| MoltenError::invalid_harness(format!("live Iroh listener sender join failed: {error}")))?;
    let (sender, _unused_receiver) = sender_topic.split();
    Ok(LoopbackPair {
        ticket_ref: ticket.ticket_ref,
        ticket_value: ticket.value,
        bound_endpoint_id,
        receiver_topic,
        sender,
        receiver_router,
        sender_router,
        node_id: identity.node_id.clone(),
        endpoint_id: identity.endpoint_id.clone(),
    })
}

struct EventScan {
    diagnostics: Vec<String>,
    transport_receipt_refs: Vec<String>,
    neighbor_events: Vec<String>,
    observed_events: u64,
}

async fn scan_events(
    input: &ControlLiveServeInput<'_>,
    receiver: &mut iroh_gossip::api::GossipTopic,
    node_id: &str,
) -> Result<EventScan> {
    let event_capacity = usize::try_from(input.max_events)
        .map_err(|_| MoltenError::invalid_harness("node control live listener max events exceeds usize capacity"))?;
    let mut diagnostics = Vec::with_capacity(event_capacity.saturating_add(2));
    let mut transport_receipt_refs = Vec::with_capacity(event_capacity);
    let mut neighbor_events = Vec::with_capacity(event_capacity);
    let mut observed_events = 0_u64;
    let timeout = std::time::Duration::from_millis(input.event_timeout_ms);
    for _ in 0..input.max_events {
        let event = match tokio::time::timeout(timeout, receiver.next()).await {
            Ok(Some(Ok(event))) => event,
            Ok(Some(Err(error))) => {
                diagnostics.push(format!("live Iroh serve listener receive failed: {error}"));
                break;
            }
            Ok(None) => break,
            Err(_) => break,
        };
        observed_events += 1;
        match &event {
            iroh_gossip::api::Event::NeighborUp(endpoint) => {
                neighbor_events.push(format!("up:iroh:{endpoint}"));
            }
            iroh_gossip::api::Event::NeighborDown(endpoint) => {
                neighbor_events.push(format!("down:iroh:{endpoint}"));
            }
            iroh_gossip::api::Event::Lagged => diagnostics.push("live Iroh serve listener lagged".to_string()),
            iroh_gossip::api::Event::Received(_) => {
                if let Some(received) =
                    receive_control_live_ingress_event(input.state_root, &event, input.topic, node_id)?
                {
                    transport_receipt_refs.push(received.transport_receipt_ref);
                }
            }
        }
        if !transport_receipt_refs.is_empty() {
            break;
        }
    }
    Ok(EventScan {
        diagnostics,
        transport_receipt_refs,
        neighbor_events,
        observed_events,
    })
}

async fn serve_node_control_live_listener_with_topic(
    input: &ControlLiveServeInput<'_>,
    receiver: &mut iroh_gossip::api::GossipTopic,
    node_id: &str,
    logical_endpoint_id: &str,
    bound_endpoint_id: &str,
) -> Result<ControlLiveServe> {
    validate_listener_event_limit(input.max_events)?;
    validate_loop_request_limit(input.max_requests_per_tick)?;
    let startup = current_startup_receipt(input.state_root)?;
    let mut scan = scan_events(input, receiver, node_id).await?;
    let service = serve_control(&ControlServeInput {
        state_root: input.state_root,
        topic: input.topic,
        max_ticks: 1,
        max_requests_per_tick: input.max_requests_per_tick,
        supervisor_policy_value: input.supervisor_policy_value,
    })?;
    if service.decision != "pass" {
        scan.diagnostics
            .push(format!("node control live listener service drain decision {}", service.decision));
    }
    let decision = if scan.diagnostics.is_empty() { "pass" } else { "deny" };
    let receipt_value = live_listener_receipt_value(&ListenerReceiptValueInput {
        decision,
        startup_receipt_ref: &startup.receipt_ref,
        node_id,
        logical_endpoint_id,
        bound_endpoint_id,
        topic: input.topic,
        max_events: input.max_events,
        observed_events: scan.observed_events,
        transport_receipt_refs: &scan.transport_receipt_refs,
        neighbor_events: &scan.neighbor_events,
        service_receipt_ref: &service.service_receipt_ref,
        diagnostics: &scan.diagnostics,
    })?;
    let listener_receipt_ref = crate::preserves_rail::canonical_hash(&receipt_value)?;
    write_preserves(&control_live_listener_receipt_path(input.state_root, &listener_receipt_ref), &receipt_value)?;
    import_artifact(input.state_root, &receipt_value)?;
    Ok(ControlLiveServe {
        listener_receipt_ref,
        listener_receipt_value: receipt_value,
        service,
        transport_receipt_refs: scan.transport_receipt_refs,
        neighbor_events: scan.neighbor_events,
        observed_events: scan.observed_events,
        bound_endpoint_id: bound_endpoint_id.to_string(),
        live_ticket_ref: None,
        live_ticket_value: None,
    })
}

async fn receive_first_live_ingress_event(
    state_root: &Path,
    receiver: &mut iroh_gossip::api::GossipTopic,
    topic: &str,
    receiver_node: &str,
) -> Result<ControlLiveIngressReceive> {
    for _ in 0..MAX_CONTROL_LIVE_LISTENER_EVENTS {
        let Some(event) = receiver.next().await else {
            return Err(MoltenError::invalid_harness("live Iroh receiver closed before node control envelope arrived"));
        };
        let event =
            event.map_err(|error| MoltenError::invalid_harness(format!("live Iroh receive failed: {error}")))?;
        if let Some(received) = receive_control_live_ingress_event(state_root, &event, topic, receiver_node)? {
            return Ok(received);
        }
    }
    Err(MoltenError::invalid_harness(
        "live Iroh receiver exceeded bounded event scan before node control envelope arrived",
    ))
}

fn stable_live_endpoint_secret(identity: &crate::node_identity::NodeIdentity) -> iroh::SecretKey {
    let seed = blake3::hash(
        format!("molten.node-control.live.endpoint.v1:{}:{}", identity.node_id, identity.endpoint_id).as_bytes(),
    );
    iroh::SecretKey::from_bytes(seed.as_bytes())
}

fn stable_live_endpoint_id(identity: &crate::node_identity::NodeIdentity) -> String {
    format!("iroh:{}", stable_live_endpoint_secret(identity).public())
}

fn live_ticket_address_refs(addr: &iroh::EndpointAddr) -> Vec<String> {
    addr.addrs.iter().map(ToString::to_string).collect()
}

fn live_ticket_for_bound_endpoint(
    state_root: &Path,
    identity: &crate::node_identity::NodeIdentity,
    topic: &str,
    addr: &iroh::EndpointAddr,
) -> Result<ControlLiveTicket> {
    let address_refs = live_ticket_address_refs(addr);
    let value = control_live_ticket_value(&ControlLiveTicketInput {
        node_id: &identity.node_id,
        node_identity_ref: &identity.identity_ref,
        logical_endpoint_id: &identity.endpoint_id,
        live_endpoint_id: &format!("iroh:{}", addr.id),
        topic,
        address_refs: &address_refs,
        policy_refs: &identity.policy_refs,
        evidence_refs: &identity.receipt_refs,
    })?;
    let ticket = parse_control_live_ticket(&value)?;
    import_artifact(state_root, &value)?;
    Ok(ticket)
}

fn live_send_ticket_diagnostics(input: &ControlLiveSendInput<'_>, ticket: &ControlLiveTicket) -> Vec<String> {
    let mut diagnostics = Vec::with_capacity(3);
    if let Some(expected) = input.expected_receiver_node
        && ticket.node_id != expected
    {
        diagnostics
            .push(format!("node control live send ticket node {} does not match expected {expected}", ticket.node_id));
    }
    if let Some(expected) = input.expected_topic
        && ticket.topic != expected
    {
        diagnostics
            .push(format!("node control live send ticket topic {} does not match expected {expected}", ticket.topic));
    }
    if let Some(expected) = input.expected_endpoint
        && ticket.live_endpoint_id != expected
    {
        diagnostics.push(format!(
            "node control live send ticket endpoint {} does not match expected {expected}",
            ticket.live_endpoint_id
        ));
    }
    diagnostics
}

fn live_send_state_root_evidence_diagnostics(
    state_root: &Path,
    input: &ControlLiveSendInput<'_>,
    envelope: &ControlIngressEnvelope,
) -> Result<Vec<String>> {
    let mut diagnostics = Vec::with_capacity(
        input.peer_bootstrap_refs.len().saturating_add(input.authority_refs.len()).saturating_add(4),
    );
    if input.peer_bootstrap_refs.is_empty() {
        diagnostics.push(
            "node control live send peer admission refs missing; run live-ticket-import --peer-admission before live send"
                .to_string(),
        );
    } else {
        let peer_diagnostics = evaluate_live_peer_bootstrap(state_root, envelope)?;
        if !peer_diagnostics.is_empty() {
            diagnostics.extend(peer_diagnostics);
            diagnostics.push(
                "node control live send peer admission unavailable in sender state root; run live-ticket-import --peer-admission before live send"
                    .to_string(),
            );
        }
    }
    if input.authority_refs.is_empty() || envelope.request.authority_refs.is_empty() {
        diagnostics.push(
            "node control live send authority grant refs missing; run authority-grant-import before live send"
                .to_string(),
        );
    } else {
        let authority_diagnostics = live_send_authority_grant_diagnostics(state_root, envelope)?;
        if !authority_diagnostics.is_empty() {
            diagnostics.extend(authority_diagnostics);
            diagnostics.push(
                "node control live send authority grant unavailable in sender state root; run authority-grant-import before live send"
                    .to_string(),
            );
        }
    }
    Ok(diagnostics)
}

fn live_send_authority_grant_diagnostics(state_root: &Path, envelope: &ControlIngressEnvelope) -> Result<Vec<String>> {
    let mut diagnostics = Vec::with_capacity(envelope.authority_refs.len().saturating_add(2));
    let mut has_candidate_authority = false;
    let mut has_admitted_grant = false;
    for authority_ref in envelope
        .authority_refs
        .iter()
        .filter(|authority_ref| envelope.request.authority_refs.contains(*authority_ref))
    {
        has_candidate_authority = true;
        match read_ledger_artifact(state_root, authority_ref) {
            Ok(value) => match parse_control_authority_grant(&value) {
                Ok(grant) => {
                    let grant_diagnostics = authority_grant_diagnostics(envelope, &grant);
                    if grant_diagnostics.is_empty() {
                        has_admitted_grant = true;
                        break;
                    }
                    diagnostics.extend(grant_diagnostics);
                }
                Err(error) => diagnostics
                    .push(format!("node control live send authority ref {authority_ref} is not a grant: {error}")),
            },
            Err(error) => diagnostics.push(format!(
                "node control live send authority grant {authority_ref} not found in sender state root: {error}"
            )),
        }
    }
    if !has_candidate_authority {
        diagnostics.push("node control live send authority refs are not bound to the request".to_string());
    }
    if !has_admitted_grant {
        diagnostics.push("node control live send authority delegation missing admitted grant".to_string());
    }
    Ok(diagnostics)
}

fn live_ticket_endpoint_addr(ticket: &ControlLiveTicket) -> Result<iroh::EndpointAddr> {
    let endpoint_id = ticket
        .live_endpoint_id
        .strip_prefix("iroh:")
        .ok_or_else(|| MoltenError::invalid_harness("node control live ticket endpoint must use iroh: prefix"))?
        .parse::<iroh::EndpointId>()
        .map_err(|error| {
            MoltenError::invalid_harness(format!("node control live ticket endpoint parse failed: {error}"))
        })?;
    let mut addrs = Vec::with_capacity(ticket.address_refs.len());
    for address_ref in &ticket.address_refs {
        let addr = if let Some(ip_addr) = address_ref.strip_prefix("ip:") {
            iroh::TransportAddr::Ip(ip_addr.parse::<SocketAddr>().map_err(|error| {
                MoltenError::invalid_harness(format!("node control live ticket ip address parse failed: {error}"))
            })?)
        } else if let Some(relay_url) = address_ref.strip_prefix("relay:") {
            iroh::TransportAddr::Relay(relay_url.parse::<iroh::RelayUrl>().map_err(|error| {
                MoltenError::invalid_harness(format!("node control live ticket relay address parse failed: {error}"))
            })?)
        } else {
            return Err(MoltenError::invalid_harness(format!(
                "node control live ticket unsupported transport address {address_ref}"
            )));
        };
        addrs.push(addr);
    }
    Ok(iroh::EndpointAddr::from_parts(endpoint_id, addrs))
}

async fn live_gossip_endpoint(
    lookup: &iroh::address_lookup::memory::MemoryLookup,
    secret_key: Option<iroh::SecretKey>,
) -> Result<iroh::Endpoint> {
    let mut builder = iroh::Endpoint::builder(iroh::endpoint::presets::Minimal)
        .relay_mode(iroh::RelayMode::Disabled)
        .address_lookup(lookup.clone())
        .alpns(vec![iroh_gossip::ALPN.to_vec()])
        .clear_ip_transports()
        .bind_addr((Ipv4Addr::LOCALHOST, 0))
        .map_err(|error| MoltenError::invalid_harness(format!("live Iroh endpoint bind addr failed: {error}")))?;
    if let Some(secret_key) = secret_key {
        builder = builder.secret_key(secret_key);
    }
    builder
        .bind()
        .await
        .map_err(|error| MoltenError::invalid_harness(format!("live Iroh endpoint bind failed: {error}")))
}

fn control_live_topic_id(topic: &str) -> iroh_gossip::TopicId {
    let digest = blake3::hash(format!("molten.node-control.live.topic.v1:{topic}").as_bytes());
    iroh_gossip::TopicId::from_bytes(*digest.as_bytes())
}

fn denied_live_ingress_delivery(
    state_root: &Path,
    envelope: &ControlIngressEnvelope,
    diagnostics: &[String],
) -> Result<ControlIngressDeliver> {
    let receipt_value = ingress_receipt_value(&IngressReceiptValueInput {
        decision: "deny",
        phase: "live-receive-deny",
        transport: &envelope.transport,
        envelope,
        idempotency_receipt_ref: None,
        queue_receipt_ref: None,
        diagnostics,
    })?;
    let ingress_receipt_ref = crate::preserves_rail::canonical_hash(&receipt_value)?;
    write_preserves(&control_ingress_receipt_path(state_root, &envelope.envelope_ref, "deliver"), &receipt_value)?;
    import_artifact(state_root, &receipt_value)?;
    Ok(ControlIngressDeliver {
        envelope_ref: envelope.envelope_ref.clone(),
        request_ref: envelope.request.request_ref.clone(),
        ingress_receipt_ref,
        ingress_receipt_value: receipt_value,
        idempotency_receipt_ref: None,
        queue_receipt_ref: None,
        has_enqueued: false,
    })
}

fn live_receive_diagnostics(
    input: &ControlLiveIngressReceiveBytesInput<'_>,
    envelope: &ControlIngressEnvelope,
) -> Vec<String> {
    let mut diagnostics = Vec::new();
    if envelope.transport != LIVE_CONTROL_INGRESS_TRANSPORT {
        diagnostics.push(format!(
            "node control live receive requires transport {LIVE_CONTROL_INGRESS_TRANSPORT}, got {}",
            envelope.transport
        ));
    }
    if envelope.topic != input.topic {
        diagnostics.push(format!(
            "node control live receive topic {} does not match subscribed topic {}",
            envelope.topic, input.topic
        ));
    }
    if envelope.to_node != input.receiver_node {
        diagnostics.push(format!(
            "node control live receive target {} does not match receiver {}",
            envelope.to_node, input.receiver_node
        ));
    }
    if envelope.peer_bootstrap_refs.is_empty() {
        diagnostics.push("node control live receive peer bootstrap refs missing".to_string());
    }
    diagnostics
}

pub fn parse_control_ingress_envelope(value: &IoValue) -> Result<ControlIngressEnvelope> {
    let fields = value
        .collect_simple_record("node-control-ingress-envelope-v1", Some(15))
        .ok_or_else(|| MoltenError::invalid_harness("expected <node-control-ingress-envelope-v1 ...>"))?;
    require_schema(
        &fields[0],
        crate::preserves_rail::NODE_CONTROL_INGRESS_ENVELOPE_SCHEMA,
        "node control ingress envelope",
    )?;
    let transport = record_string(&fields[1], "transport")?;
    let topic = record_string(&fields[2], "topic")?;
    let from_peer = record_string(&fields[3], "from-peer")?;
    let to_node = record_string(&fields[4], "to-node")?;
    let sequence = record_u64_string(&fields[5], "sequence")?;
    let operation_ref = record_ref_string(&fields[6], "operation")?;
    let request_ref = record_ref_string(&fields[7], "request-ref")?;
    let request_value = record_value(&fields[8], "request")?;
    let request = crate::node_runtime::parse_control_request(&request_value)?;
    if request.request_ref != request_ref {
        return Err(MoltenError::invalid_harness("node control ingress embedded request ref mismatch"));
    }
    let peer_bootstrap_refs = record_ref_strings(&fields[9], "peer-bootstrap")?;
    let authority_refs = record_ref_strings(&fields[10], "authority")?;
    let policy_refs = record_ref_strings(&fields[11], "policy")?;
    let resource_refs = record_ref_strings(&fields[12], "resource")?;
    let evidence_refs = record_ref_strings(&fields[13], "evidence")?;
    let expected_scope = crate::delivery_idempotency::remote_topic_scope_ref(&topic, &to_node)?;
    let expected_operation =
        crate::delivery_idempotency::derive_operation_id(crate::delivery_idempotency::OperationIdInput {
            scope_ref: expected_scope,
            producer: from_peer.clone(),
            consumer: to_node.clone(),
            sequence,
            intent: "node-control-ingress".to_string(),
            payload_ref: request.request_ref.clone(),
            policy_refs: policy_refs.clone(),
        })?;
    if expected_operation.operation_ref != operation_ref {
        return Err(MoltenError::invalid_harness("node control ingress operation ref mismatch"));
    }
    Ok(ControlIngressEnvelope {
        envelope_ref: crate::preserves_rail::canonical_hash(value)?,
        transport,
        topic,
        from_peer,
        to_node,
        sequence,
        operation_ref,
        request,
        peer_bootstrap_refs,
        authority_refs,
        policy_refs,
        resource_refs,
        evidence_refs,
        value: value.clone(),
    })
}

pub fn publish_control_ingress(input: &ControlIngressPublishInput<'_>) -> Result<ControlIngressPublish> {
    validate_state_root(input.state_root)?;
    ensure_state_layout(input.state_root)?;
    let envelope = parse_control_ingress_envelope(input.envelope_value)?;
    let envelope_path = control_ingress_envelope_path(input.state_root, &envelope.topic, &envelope.envelope_ref);
    write_ingress_envelope_and_verify(input.state_root, &envelope.topic, &envelope)?;
    import_artifact(input.state_root, &envelope.value)?;
    let diagnostics = Vec::new();
    let receipt_value = ingress_receipt_value(&IngressReceiptValueInput {
        decision: "pass",
        phase: "publish",
        transport: &envelope.transport,
        envelope: &envelope,
        idempotency_receipt_ref: None,
        queue_receipt_ref: None,
        diagnostics: &diagnostics,
    })?;
    let receipt_ref = crate::preserves_rail::canonical_hash(&receipt_value)?;
    write_preserves(
        &control_ingress_receipt_path(input.state_root, &envelope.envelope_ref, "publish"),
        &receipt_value,
    )?;
    import_artifact(input.state_root, &receipt_value)?;
    Ok(ControlIngressPublish {
        envelope_ref: envelope.envelope_ref,
        envelope_path,
        receipt_ref,
        receipt_value,
    })
}

#[derive(Debug, Default)]
struct EnqueueOutcome {
    idempotency_receipt_ref: Option<String>,
    queue_receipt_ref: Option<String>,
    has_enqueued: bool,
    diagnostics: Vec<String>,
}

fn apply_ingress_enqueue(state_root: &Path, envelope: &ControlIngressEnvelope) -> Result<EnqueueOutcome> {
    let idempotency_evidence_refs = ingress_idempotency_evidence_refs(envelope);
    let scope_ref = crate::delivery_idempotency::remote_topic_scope_ref(&envelope.topic, &envelope.to_node)?;
    let delivery = crate::delivery_idempotency::check_delivery(crate::delivery_idempotency::DeliveryCheckInput {
        root: &state_root.join(CONTROL_IDEMPOTENCY_DIR),
        scope_profile: crate::delivery_idempotency::SCOPE_REMOTE_TOPIC,
        scope_ref: &scope_ref,
        producer: &envelope.from_peer,
        consumer: &envelope.to_node,
        sequence: envelope.sequence,
        intent: "node-control-ingress",
        payload_ref: &envelope.request.request_ref,
        policy_refs: &envelope.policy_refs,
        evidence_refs: &idempotency_evidence_refs,
        semantic_result_ref: Some(&envelope.request.request_ref),
        gap_policy: crate::delivery_idempotency::GapPolicy::Deny,
    })?;
    let idempotency_receipt_ref = Some(delivery.receipt.receipt_ref.clone());
    import_artifact(state_root, &delivery.receipt.value)?;
    if delivery.should_commit_side_effect {
        let submitted = submit_control_request(&ControlSubmitInput {
            state_root,
            request_value: &envelope.request.value,
        })?;
        return Ok(EnqueueOutcome {
            idempotency_receipt_ref,
            queue_receipt_ref: Some(submitted.queue_receipt_ref),
            has_enqueued: true,
            diagnostics: Vec::new(),
        });
    }
    if delivery.receipt.decision == "duplicate" {
        return Ok(EnqueueOutcome {
            idempotency_receipt_ref,
            queue_receipt_ref: prior_queue_receipt_ref(state_root, &envelope.request.request_ref).ok(),
            has_enqueued: false,
            diagnostics: Vec::new(),
        });
    }
    let mut diagnostics = delivery.receipt.diagnostics.clone();
    diagnostics.push(format!("node control ingress idempotency decision {}", delivery.receipt.decision));
    Ok(EnqueueOutcome {
        idempotency_receipt_ref,
        queue_receipt_ref: None,
        has_enqueued: false,
        diagnostics,
    })
}

pub fn deliver_control_ingress(input: &ControlIngressDeliverInput<'_>) -> Result<ControlIngressDeliver> {
    validate_state_root(input.state_root)?;
    validate_node_id(input.topic)?;
    validate_ingress_ref(input.envelope_ref, "node control ingress envelope ref")?;
    ensure_state_layout(input.state_root)?;
    let envelope_value =
        read_preserves(&control_ingress_envelope_path(input.state_root, input.topic, input.envelope_ref))?;
    let envelope = parse_control_ingress_envelope(&envelope_value)?;
    if envelope.envelope_ref != input.envelope_ref {
        return Err(MoltenError::invalid_harness(format!(
            "node control ingress materialized envelope ref {} does not match requested {}",
            envelope.envelope_ref, input.envelope_ref
        )));
    }
    let mut diagnostics = ingress_pre_enqueue_diagnostics(input.state_root, input.topic, &envelope)?;
    let mut enqueue = EnqueueOutcome::default();
    if diagnostics.is_empty() {
        enqueue = apply_ingress_enqueue(input.state_root, &envelope)?;
        diagnostics.append(&mut enqueue.diagnostics);
    }
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let receipt_value = ingress_receipt_value(&IngressReceiptValueInput {
        decision,
        phase: if enqueue.has_enqueued {
            "deliver"
        } else {
            "duplicate-or-deny"
        },
        transport: &envelope.transport,
        envelope: &envelope,
        idempotency_receipt_ref: enqueue.idempotency_receipt_ref.as_deref(),
        queue_receipt_ref: enqueue.queue_receipt_ref.as_deref(),
        diagnostics: &diagnostics,
    })?;
    let ingress_receipt_ref = crate::preserves_rail::canonical_hash(&receipt_value)?;
    write_preserves(
        &control_ingress_receipt_path(input.state_root, &envelope.envelope_ref, "deliver"),
        &receipt_value,
    )?;
    import_artifact(input.state_root, &receipt_value)?;
    Ok(ControlIngressDeliver {
        envelope_ref: envelope.envelope_ref,
        request_ref: envelope.request.request_ref,
        ingress_receipt_ref,
        ingress_receipt_value: receipt_value,
        idempotency_receipt_ref: enqueue.idempotency_receipt_ref,
        queue_receipt_ref: enqueue.queue_receipt_ref,
        has_enqueued: enqueue.has_enqueued,
    })
}

fn ingress_pre_enqueue_diagnostics(
    state_root: &Path,
    topic: &str,
    envelope: &ControlIngressEnvelope,
) -> Result<Vec<String>> {
    let mut diagnostics = Vec::new();
    if !matches!(envelope.transport.as_str(), LOCAL_CONTROL_INGRESS_TRANSPORT | LIVE_CONTROL_INGRESS_TRANSPORT) {
        diagnostics.push(format!("unsupported node control ingress transport {}", envelope.transport));
    }
    if envelope.topic != topic {
        diagnostics.push(format!("node control ingress topic {} does not match requested {topic}", envelope.topic));
    }
    let identity = crate::node_identity::parse_node_identity(&read_preserves(&state_root.join(IDENTITY_FILE))?)?;
    if envelope.to_node != identity.node_id {
        diagnostics
            .push(format!("node control ingress target {} does not match node {}", envelope.to_node, identity.node_id));
    }
    if envelope.peer_bootstrap_refs.is_empty() {
        diagnostics.push("node control ingress peer bootstrap refs missing".to_string());
    }
    if envelope.authority_refs.is_empty() || envelope.request.authority_refs.is_empty() {
        diagnostics.push("node control ingress authority refs missing".to_string());
    }
    if envelope.policy_refs.is_empty() || envelope.request.policy_refs.is_empty() {
        diagnostics.push("node control ingress policy refs missing".to_string());
    }
    if envelope.resource_refs.is_empty() || envelope.request.resource_refs.is_empty() {
        diagnostics.push("node control ingress resource refs missing".to_string());
    }
    if diagnostics.is_empty() && envelope.transport == LIVE_CONTROL_INGRESS_TRANSPORT {
        diagnostics.extend(evaluate_live_peer_bootstrap(state_root, envelope)?);
    }
    if diagnostics.is_empty() && envelope.transport == LIVE_CONTROL_INGRESS_TRANSPORT {
        diagnostics.extend(evaluate_live_authority_delegation(state_root, envelope)?);
    }
    Ok(diagnostics)
}

fn evaluate_live_peer_bootstrap(state_root: &Path, envelope: &ControlIngressEnvelope) -> Result<Vec<String>> {
    let mut diagnostics = Vec::with_capacity(envelope.peer_bootstrap_refs.len().saturating_add(1));
    let mut admitted_peer_ref = None;
    for peer_ref in envelope.peer_bootstrap_refs.iter() {
        match read_ledger_artifact(state_root, peer_ref) {
            Ok(value) => match parse_control_live_peer_admission(&value) {
                Ok(admission) => {
                    let admission_diagnostics = live_peer_admission_diagnostics(state_root, envelope, &admission)?;
                    if admission_diagnostics.is_empty() {
                        admitted_peer_ref = Some(admission.admission_ref);
                        break;
                    }
                    diagnostics.extend(admission_diagnostics);
                }
                Err(error) => diagnostics
                    .push(format!("node control live peer bootstrap ref {peer_ref} is not an admission: {error}")),
            },
            Err(error) => diagnostics.push(format!("node control live peer bootstrap {peer_ref} not found: {error}")),
        }
    }
    if admitted_peer_ref.is_none() {
        diagnostics.push("node control live peer bootstrap missing admitted ticket".to_string());
    }
    Ok(diagnostics)
}

fn live_peer_admission_diagnostics(
    state_root: &Path,
    envelope: &ControlIngressEnvelope,
    admission: &ControlLivePeerAdmission,
) -> Result<Vec<String>> {
    let mut diagnostics = Vec::with_capacity(8);
    if admission.decision != "pass" {
        diagnostics.push(format!(
            "node control live peer admission {} decision {}",
            admission.admission_ref, admission.decision
        ));
    }
    if admission.peer_id != envelope.from_peer {
        diagnostics.push(format!(
            "node control live peer admission {} peer {} does not match {}",
            admission.admission_ref, admission.peer_id, envelope.from_peer
        ));
    }
    if admission.node_id != envelope.to_node {
        diagnostics.push(format!(
            "node control live peer admission {} node {} does not match {}",
            admission.admission_ref, admission.node_id, envelope.to_node
        ));
    }
    if admission.topic != envelope.topic {
        diagnostics.push(format!(
            "node control live peer admission {} topic {} does not match {}",
            admission.admission_ref, admission.topic, envelope.topic
        ));
    }
    match read_ledger_artifact(state_root, &admission.ticket_ref) {
        Ok(value) => match parse_control_live_ticket(&value) {
            Ok(ticket) => {
                if ticket.node_id != admission.node_id || ticket.topic != admission.topic {
                    diagnostics.push(format!(
                        "node control live peer admission {} ticket binding mismatch",
                        admission.admission_ref
                    ));
                }
            }
            Err(error) => diagnostics.push(format!(
                "node control live peer admission {} ticket is not a live ticket: {error}",
                admission.admission_ref
            )),
        },
        Err(error) => diagnostics.push(format!(
            "node control live peer admission {} ticket {} not found: {error}",
            admission.admission_ref, admission.ticket_ref
        )),
    }
    if admission.sequence > envelope.sequence {
        diagnostics.push(format!(
            "node control live peer admission {} is not valid until sequence {}",
            admission.admission_ref, admission.sequence
        ));
    }
    if let Some(expires_at) = admission.expires_at
        && expires_at < envelope.sequence
    {
        diagnostics.push(format!(
            "node control live peer admission {} expired at sequence {expires_at}",
            admission.admission_ref
        ));
    }
    Ok(diagnostics)
}

fn evaluate_live_authority_delegation(state_root: &Path, envelope: &ControlIngressEnvelope) -> Result<Vec<String>> {
    let mut diagnostics = Vec::with_capacity(envelope.authority_refs.len().saturating_add(2));
    let mut admitted_grant_ref = None;
    let candidate_authority_refs = envelope
        .authority_refs
        .iter()
        .filter(|authority_ref| envelope.request.authority_refs.contains(*authority_ref))
        .collect::<Vec<_>>();
    if candidate_authority_refs.is_empty() {
        diagnostics.push("node control live authority refs are not bound to the request".to_string());
    }
    for authority_ref in candidate_authority_refs {
        match read_ledger_artifact(state_root, authority_ref) {
            Ok(value) => match parse_control_authority_grant(&value) {
                Ok(grant) => {
                    let grant_diagnostics = authority_grant_diagnostics(envelope, &grant);
                    if grant_diagnostics.is_empty() {
                        admitted_grant_ref = Some(grant.grant_ref);
                        break;
                    }
                    diagnostics.extend(grant_diagnostics);
                }
                Err(error) => {
                    diagnostics.push(format!("node control authority ref {authority_ref} is not a grant: {error}"))
                }
            },
            Err(error) => diagnostics.push(format!("node control authority grant {authority_ref} not found: {error}")),
        }
    }
    if admitted_grant_ref.is_none() {
        diagnostics.push("node control live authority delegation missing admitted grant".to_string());
    }
    let decision = if admitted_grant_ref.is_some() { "pass" } else { "deny" };
    let receipt_value = authority_receipt_value(&AuthorityReceiptValueInput {
        decision,
        envelope,
        grant_ref: admitted_grant_ref.as_deref(),
        diagnostics: &diagnostics,
    })?;
    let receipt_ref = crate::preserves_rail::canonical_hash(&receipt_value)?;
    write_preserves(&control_authority_receipt_path(state_root, &envelope.envelope_ref), &receipt_value)?;
    import_artifact(state_root, &receipt_value)?;
    if decision == "deny" {
        diagnostics.push(format!("node control authority receipt {receipt_ref} denied"));
    }
    Ok(diagnostics)
}

fn authority_grant_diagnostics(envelope: &ControlIngressEnvelope, grant: &ControlAuthorityGrant) -> Vec<String> {
    let mut diagnostics = Vec::with_capacity(8);
    if grant.peer_id != envelope.from_peer {
        diagnostics.push(format!(
            "node control authority grant {} peer {} does not match {}",
            grant.grant_ref, grant.peer_id, envelope.from_peer
        ));
    }
    if grant.node_id != envelope.to_node {
        diagnostics.push(format!(
            "node control authority grant {} node {} does not match {}",
            grant.grant_ref, grant.node_id, envelope.to_node
        ));
    }
    if !grant
        .operations
        .iter()
        .any(|operation| operation == "*" || operation == &envelope.request.operation)
    {
        diagnostics.push(format!(
            "node control authority grant {} does not allow operation {}",
            grant.grant_ref, envelope.request.operation
        ));
    }
    if grant.epoch > envelope.sequence {
        diagnostics
            .push(format!("node control authority grant {} is not valid until epoch {}", grant.grant_ref, grant.epoch));
    }
    if let Some(expires_at) = grant.expires_at
        && expires_at < envelope.sequence
    {
        diagnostics.push(format!("node control authority grant {} expired at epoch {expires_at}", grant.grant_ref));
    }
    if !grant.revocation_refs.is_empty() {
        diagnostics.push(format!("node control authority grant {} has revocation refs", grant.grant_ref));
    }
    if !scope_matches_request(
        &grant.target_scope,
        envelope.request.target_ref.as_deref(),
        envelope.request.payload_ref.as_deref(),
    ) {
        diagnostics.push(format!(
            "node control authority grant {} target scope {} does not match request",
            grant.grant_ref, grant.target_scope
        ));
    }
    if !scope_matches_refs(&grant.resource_scope, &envelope.resource_refs, &envelope.request.resource_refs) {
        diagnostics.push(format!(
            "node control authority grant {} resource scope {} does not match request",
            grant.grant_ref, grant.resource_scope
        ));
    }
    diagnostics
}

fn scope_matches_request(scope: &str, target_ref: Option<&str>, payload_ref: Option<&str>) -> bool {
    scope == "*" || target_ref == Some(scope) || payload_ref == Some(scope)
}

fn scope_matches_refs(scope: &str, envelope_refs: &[String], request_refs: &[String]) -> bool {
    scope == "*"
        || envelope_refs.iter().any(|reference| reference == scope)
        || request_refs.iter().any(|reference| reference == scope)
}

fn ingress_idempotency_evidence_refs(envelope: &ControlIngressEnvelope) -> Vec<String> {
    let mut refs = Vec::with_capacity(
        envelope.peer_bootstrap_refs.len()
            + envelope.authority_refs.len()
            + envelope.resource_refs.len()
            + envelope.evidence_refs.len(),
    );
    refs.extend(envelope.peer_bootstrap_refs.iter().cloned());
    refs.extend(envelope.authority_refs.iter().cloned());
    refs.extend(envelope.resource_refs.iter().cloned());
    refs.extend(envelope.evidence_refs.iter().cloned());
    refs.sort();
    refs.dedup();
    refs
}

fn prior_queue_receipt_ref(state_root: &Path, request_ref: &str) -> Result<String> {
    let receipt = read_preserves(&queue_receipt_path(state_root, request_ref))?;
    crate::preserves_rail::canonical_hash(&receipt)
}

fn prior_dispatch_for_request(
    state_root: &Path,
    request: &crate::node_runtime::ControlRequest,
) -> Result<Option<ControlDispatch>> {
    let receipt_path = control_outbox_receipt_path(state_root, &request.request_ref);
    if !receipt_path.exists() {
        return Ok(None);
    }
    let archived_path = control_outbox_request_path(state_root, &request.request_ref);
    if archived_path.exists() {
        let archived_value = read_preserves(&archived_path)?;
        let archived_ref = crate::preserves_rail::canonical_hash(&archived_value)?;
        if archived_ref != request.request_ref {
            return Err(MoltenError::invalid_harness(
                "node control duplicate request conflicts with archived request evidence",
            ));
        }
    }
    let control_receipt_value = read_preserves(&receipt_path)?;
    let control = crate::node_runtime::parse_control_receipt(&control_receipt_value)?;
    if control.request_ref != request.request_ref {
        return Err(MoltenError::invalid_harness("node control duplicate receipt conflicts with request ref"));
    }
    Ok(Some(ControlDispatch {
        operation: request.operation.clone(),
        request_ref: request.request_ref.clone(),
        control_receipt_ref: control.receipt_ref,
        control_receipt_value: control.value,
        subreceipt_refs: control.subreceipt_refs,
    }))
}

fn write_dispatch_queue_receipt(
    state_root: &Path,
    request: &crate::node_runtime::ControlRequest,
    phase: &str,
) -> Result<String> {
    let location_ref = local_ref(
        "node-control-outbox-path",
        &control_outbox_receipt_path(state_root, &request.request_ref).display().to_string(),
    )?;
    let diagnostics = Vec::new();
    let queue_receipt = queue_receipt_value(&QueueReceiptValueInput {
        decision: "pass",
        phase,
        operation: &request.operation,
        request_ref: &request.request_ref,
        location_ref: &location_ref,
        diagnostics: &diagnostics,
    })?;
    let queue_receipt_ref = crate::preserves_rail::canonical_hash(&queue_receipt)?;
    write_preserves(&dispatch_receipt_path(state_root, &request.request_ref), &queue_receipt)?;
    import_artifact(state_root, &queue_receipt)?;
    Ok(queue_receipt_ref)
}

fn dispatch_status_request(
    state_root: &Path,
    request: &crate::node_runtime::ControlRequest,
) -> Result<ControlDispatch> {
    let status = status_local_node_with_request(&StatusInput { state_root }, request)?;
    write_preserves(&control_outbox_receipt_path(state_root, &request.request_ref), &status.control_receipt_value)?;
    Ok(ControlDispatch {
        operation: request.operation.clone(),
        request_ref: request.request_ref.clone(),
        control_receipt_ref: status.control_receipt_ref,
        control_receipt_value: status.control_receipt_value,
        subreceipt_refs: vec![status.health_ref],
    })
}

fn dispatch_shutdown_request(
    state_root: &Path,
    request: &crate::node_runtime::ControlRequest,
) -> Result<ControlDispatch> {
    let stop = stop_local_node_with_request(&StopInput { state_root }, request)?;
    write_preserves(&control_outbox_receipt_path(state_root, &request.request_ref), &stop.control_receipt_value)?;
    Ok(ControlDispatch {
        operation: request.operation.clone(),
        request_ref: request.request_ref.clone(),
        control_receipt_ref: stop.control_receipt_ref,
        control_receipt_value: stop.control_receipt_value,
        subreceipt_refs: vec![stop.shutdown_ref],
    })
}

#[derive(Debug, Clone, Copy)]
struct ControlProvenanceInput<'a> {
    state_root: &'a Path,
    request: &'a crate::node_runtime::ControlRequest,
    artifact_ref: &'a str,
    operation: &'a str,
    subreceipt_kind: &'a str,
}

fn evaluate_control_provenance(input: &ControlProvenanceInput<'_>) -> Result<crate::provenance::ProvenanceEvaluation> {
    let mut provenance_diagnostics = Vec::with_capacity(input.request.evidence_refs.len().saturating_add(1));
    if input.request.evidence_refs.is_empty() {
        provenance_diagnostics.push("node control provenance evidence refs missing".to_string());
    }
    let mut provenance_values = Vec::with_capacity(input.request.evidence_refs.len());
    let mut build_verification_values = Vec::with_capacity(input.request.evidence_refs.len());
    for evidence_ref in &input.request.evidence_refs {
        match read_ledger_artifact(input.state_root, evidence_ref) {
            Ok(value) => {
                if crate::provenance::parse_provenance_build_verification_receipt(&value).is_ok() {
                    build_verification_values.push(value);
                } else {
                    provenance_values.push(value);
                }
            }
            Err(error) => provenance_diagnostics
                .push(format!("node control provenance evidence {evidence_ref} not found in node ledger: {error}")),
        }
    }
    let evaluation = crate::provenance::evaluate_provenance(&crate::provenance::ProvenanceEvaluationInput {
        operation: input.operation,
        profile: "node-control",
        artifact_ref: input.artifact_ref,
        provenance_values: &provenance_values,
        build_verification_values: &build_verification_values,
        prior_diagnostics: &provenance_diagnostics,
    })?;
    write_preserves(
        &control_operation_subreceipt_path(input.state_root, &input.request.request_ref, input.subreceipt_kind),
        &evaluation.receipt_value,
    )?;
    import_artifact(input.state_root, &evaluation.receipt_value)?;
    Ok(evaluation)
}

struct InstallRefs {
    schema_refs: Vec<String>,
    evidence_refs: Vec<String>,
}

struct InstallFinishInput<'a> {
    state_root: &'a Path,
    request: &'a crate::node_runtime::ControlRequest,
    startup_receipt_ref: &'a str,
    payload_ref: &'a str,
    payload_value: IoValue,
    provenance: crate::provenance::ProvenanceEvaluation,
    diagnostics: Vec<String>,
}

fn finish_install_dispatch(
    state_root: &Path,
    request: &crate::node_runtime::ControlRequest,
    startup_receipt_ref: &str,
    subreceipt_refs: &[String],
    diagnostics: &[String],
) -> Result<ControlDispatch> {
    finalize_operation_dispatch(&OperationFinalizeInput {
        state_root,
        request,
        startup_receipt_ref,
        subreceipt_refs,
        diagnostics,
    })
}

fn install_refs(
    request: &crate::node_runtime::ControlRequest,
    payload_ref: &str,
    provenance_receipt_ref: &str,
) -> Result<InstallRefs> {
    let schema_refs = match request.target_ref.as_ref() {
        Some(target_ref) => vec![target_ref.clone()],
        None => vec![local_ref("node-control-install-schema", &request.request_ref)?],
    };
    let extra_evidence_refs = if request.target_ref.is_some() { 3 } else { 2 };
    let mut evidence_refs =
        Vec::with_capacity(request.resource_refs.len() + request.evidence_refs.len() + extra_evidence_refs);
    evidence_refs.extend(request.resource_refs.iter().cloned());
    evidence_refs.extend(request.evidence_refs.iter().cloned());
    evidence_refs.push(provenance_receipt_ref.to_string());
    evidence_refs.push(payload_ref.to_string());
    if let Some(target_ref) = request.target_ref.as_ref() {
        evidence_refs.push(target_ref.clone());
    }
    Ok(InstallRefs {
        schema_refs,
        evidence_refs,
    })
}

fn finish_install(input: InstallFinishInput<'_>) -> Result<ControlDispatch> {
    let mut diagnostics = input.diagnostics;
    let provenance_receipt_refs = [input.provenance.receipt_ref.clone()];
    let refs = install_refs(input.request, input.payload_ref, &provenance_receipt_refs[0])?;
    let install = match crate::artifacts::install_artifact(
        &input.state_root.join("registry"),
        &crate::artifacts::ArtifactInstallInput {
            kind: "node-control-artifact".to_string(),
            payload: input.payload_value,
            schema_refs: refs.schema_refs,
            dependency_refs: Vec::new(),
            effect_manifest_ref: None,
            policy_refs: input.request.policy_refs.clone(),
            evidence_refs: refs.evidence_refs,
            installer_ref: input.request.request_ref.clone(),
            capability_refs: input.request.authority_refs.clone(),
        },
    ) {
        Ok(install) => install,
        Err(error) => {
            diagnostics.push(format!("node control artifact install failed: {error}"));
            return finish_install_dispatch(
                input.state_root,
                input.request,
                input.startup_receipt_ref,
                &provenance_receipt_refs,
                &diagnostics,
            );
        }
    };
    let install_receipt_ref = crate::preserves_rail::canonical_hash(&install.receipt_value)?;
    write_preserves(
        &control_operation_subreceipt_path(input.state_root, &input.request.request_ref, "artifact-install"),
        &install.receipt_value,
    )?;
    import_artifact(input.state_root, &install.receipt_value)?;
    if install.decision == "pass" {
        import_artifact(input.state_root, &install.artifact.value)?;
    } else if install.missing_dependencies.is_empty() {
        diagnostics.push("node control artifact install denied".to_string());
    } else {
        diagnostics
            .extend(install.missing_dependencies.iter().map(|reference| format!("missing dependency {reference}")));
    }
    let subreceipt_refs = [provenance_receipt_refs[0].clone(), install_receipt_ref];
    finish_install_dispatch(input.state_root, input.request, input.startup_receipt_ref, &subreceipt_refs, &diagnostics)
}

fn dispatch_install_request(
    state_root: &Path,
    request: &crate::node_runtime::ControlRequest,
) -> Result<ControlDispatch> {
    let startup = current_startup_receipt(state_root)?;
    let mut diagnostics = side_effect_preflight_diagnostics(request);
    let Some(payload_ref) = request.payload_ref.as_deref() else {
        diagnostics.push("node control install requires payload ref".to_string());
        return finish_install_dispatch(state_root, request, &startup.receipt_ref, &[], &diagnostics);
    };
    if !diagnostics.is_empty() {
        return finish_install_dispatch(state_root, request, &startup.receipt_ref, &[], &diagnostics);
    }
    let payload_value = match read_ledger_artifact(state_root, payload_ref) {
        Ok(value) => value,
        Err(error) => {
            diagnostics.push(format!("node control install payload not found in node ledger: {error}"));
            return finish_install_dispatch(state_root, request, &startup.receipt_ref, &[], &diagnostics);
        }
    };
    let provenance = evaluate_control_provenance(&ControlProvenanceInput {
        state_root,
        request,
        artifact_ref: payload_ref,
        operation: "install",
        subreceipt_kind: "artifact-provenance",
    })?;
    let provenance_receipt_refs = [provenance.receipt_ref.clone()];
    diagnostics.extend(provenance.diagnostics.iter().cloned());
    if provenance.decision != "pass" {
        return finish_install_dispatch(
            state_root,
            request,
            &startup.receipt_ref,
            &provenance_receipt_refs,
            &diagnostics,
        );
    }
    finish_install(InstallFinishInput {
        state_root,
        request,
        startup_receipt_ref: &startup.receipt_ref,
        payload_ref,
        payload_value,
        provenance,
        diagnostics,
    })
}

struct PreparedRun {
    admission_ref: String,
    job_ref: String,
    execution_request_value: IoValue,
}

struct RunStart {
    diagnostics: Vec<String>,
    prepared: PreparedRun,
}

struct CompleteRunInput<'a> {
    state_root: &'a Path,
    request: &'a crate::node_runtime::ControlRequest,
    startup_receipt_ref: &'a str,
    prepared: PreparedRun,
    provenance: crate::provenance::ProvenanceEvaluation,
    diagnostics: Vec<String>,
}

type RunStartResult = std::result::Result<RunStart, Box<ControlDispatch>>;

struct RunDenyInput<'a> {
    state_root: &'a Path,
    request: &'a crate::node_runtime::ControlRequest,
    startup_receipt_ref: &'a str,
    diagnostics: Vec<String>,
}

fn deny_run_start(input: RunDenyInput<'_>) -> Result<RunStartResult> {
    let dispatch = finalize_operation_dispatch(&OperationFinalizeInput {
        state_root: input.state_root,
        request: input.request,
        startup_receipt_ref: input.startup_receipt_ref,
        subreceipt_refs: &[],
        diagnostics: &input.diagnostics,
    })?;
    Ok(Err(Box::new(dispatch)))
}

fn prepare_run(
    state_root: &Path,
    request: &crate::node_runtime::ControlRequest,
    startup_receipt_ref: &str,
) -> Result<RunStartResult> {
    let mut diagnostics = side_effect_preflight_diagnostics(request);
    let Some(execution_request_ref) = request.payload_ref.as_deref() else {
        diagnostics.push("node control run requires execution request payload ref".to_string());
        return deny_run_start(RunDenyInput {
            state_root,
            request,
            startup_receipt_ref,
            diagnostics,
        });
    };
    let Some(admission_ref) = request.target_ref.as_deref() else {
        diagnostics.push("node control run requires admission receipt target ref".to_string());
        return deny_run_start(RunDenyInput {
            state_root,
            request,
            startup_receipt_ref,
            diagnostics,
        });
    };
    if !diagnostics.is_empty() {
        return deny_run_start(RunDenyInput {
            state_root,
            request,
            startup_receipt_ref,
            diagnostics,
        });
    }
    let execution_request_value = match read_ledger_artifact(state_root, execution_request_ref) {
        Ok(value) => value,
        Err(error) => {
            diagnostics.push(format!("node control run execution request not found in node ledger: {error}"));
            return deny_run_start(RunDenyInput {
                state_root,
                request,
                startup_receipt_ref,
                diagnostics,
            });
        }
    };
    let execution_request = match crate::job_dag::parse_job_execution_request_value(&execution_request_value) {
        Ok(execution_request) => execution_request,
        Err(error) => {
            diagnostics.push(format!("node control run execution request malformed: {error}"));
            return deny_run_start(RunDenyInput {
                state_root,
                request,
                startup_receipt_ref,
                diagnostics,
            });
        }
    };
    Ok(Ok(RunStart {
        diagnostics,
        prepared: PreparedRun {
            admission_ref: admission_ref.to_string(),
            job_ref: execution_request.job_ref,
            execution_request_value,
        },
    }))
}

fn complete_run(input: CompleteRunInput<'_>) -> Result<ControlDispatch> {
    let mut diagnostics = input.diagnostics;
    let provenance_receipt_refs = [input.provenance.receipt_ref.clone()];
    let admission_receipt_value = match read_ledger_artifact(input.state_root, &input.prepared.admission_ref) {
        Ok(value) => value,
        Err(error) => {
            diagnostics.push(format!("node control run admission receipt not found in node ledger: {error}"));
            return finalize_operation_dispatch(&OperationFinalizeInput {
                state_root: input.state_root,
                request: input.request,
                startup_receipt_ref: input.startup_receipt_ref,
                subreceipt_refs: &provenance_receipt_refs,
                diagnostics: &diagnostics,
            });
        }
    };
    let execution = crate::job_dag::execution_loopback(crate::job_dag::ExecutionLoopbackInput {
        target_registry: &input.state_root.join("registry"),
        storage_root: &input.state_root.join("storage"),
        cache_root: &input.state_root.join("cache"),
        chunk_root: &input.state_root.join("chunks"),
        admission_receipt_value: &admission_receipt_value,
        request_value: &input.prepared.execution_request_value,
    })?;
    write_preserves(
        &control_operation_subreceipt_path(input.state_root, &input.request.request_ref, "job-execution"),
        &execution.receipt_value,
    )?;
    import_artifact(input.state_root, &execution.receipt_value)?;
    let mut subreceipt_refs = Vec::with_capacity(3);
    subreceipt_refs.push(input.provenance.receipt_ref);
    subreceipt_refs.push(execution.receipt_ref.clone());
    if let Some(run) = execution.run.as_ref() {
        let run_ref = crate::preserves_rail::canonical_hash(&run.receipt_value)?;
        write_preserves(
            &control_operation_subreceipt_path(input.state_root, &input.request.request_ref, "job-run"),
            &run.receipt_value,
        )?;
        import_artifact(input.state_root, &run.receipt_value)?;
        subreceipt_refs.push(run_ref);
    }
    diagnostics.extend(execution.diagnostics.iter().cloned());
    if execution.decision != "pass" && diagnostics.is_empty() {
        diagnostics.push("node control run execution denied".to_string());
    }
    finalize_operation_dispatch(&OperationFinalizeInput {
        state_root: input.state_root,
        request: input.request,
        startup_receipt_ref: input.startup_receipt_ref,
        subreceipt_refs: &subreceipt_refs,
        diagnostics: &diagnostics,
    })
}

fn dispatch_run_request(state_root: &Path, request: &crate::node_runtime::ControlRequest) -> Result<ControlDispatch> {
    let startup = current_startup_receipt(state_root)?;
    let start = match prepare_run(state_root, request, &startup.receipt_ref)? {
        Ok(start) => start,
        Err(dispatch) => return Ok(*dispatch),
    };
    let mut diagnostics = start.diagnostics;
    let provenance = evaluate_control_provenance(&ControlProvenanceInput {
        state_root,
        request,
        artifact_ref: &start.prepared.job_ref,
        operation: "run",
        subreceipt_kind: "job-provenance",
    })?;
    let provenance_receipt_refs = [provenance.receipt_ref.clone()];
    diagnostics.extend(provenance.diagnostics.iter().cloned());
    if provenance.decision != "pass" {
        return finalize_operation_dispatch(&OperationFinalizeInput {
            state_root,
            request,
            startup_receipt_ref: &startup.receipt_ref,
            subreceipt_refs: &provenance_receipt_refs,
            diagnostics: &diagnostics,
        });
    }
    complete_run(CompleteRunInput {
        state_root,
        request,
        startup_receipt_ref: &startup.receipt_ref,
        prepared: start.prepared,
        provenance,
        diagnostics,
    })
}

fn dispatch_gate_request(state_root: &Path, request: &crate::node_runtime::ControlRequest) -> Result<ControlDispatch> {
    let startup = current_startup_receipt(state_root)?;
    let mut diagnostics = side_effect_preflight_diagnostics(request);
    let Some(subject_ref) = request.target_ref.as_deref() else {
        diagnostics.push("node control gate requires target subject ref".to_string());
        return finalize_operation_dispatch(&OperationFinalizeInput {
            state_root,
            request,
            startup_receipt_ref: &startup.receipt_ref,
            subreceipt_refs: &[],
            diagnostics: &diagnostics,
        });
    };
    let Some(gate_receipt_ref) = request.payload_ref.as_deref() else {
        diagnostics.push("node control gate requires gate receipt payload ref".to_string());
        return finalize_operation_dispatch(&OperationFinalizeInput {
            state_root,
            request,
            startup_receipt_ref: &startup.receipt_ref,
            subreceipt_refs: &[],
            diagnostics: &diagnostics,
        });
    };
    if !diagnostics.is_empty() {
        return finalize_operation_dispatch(&OperationFinalizeInput {
            state_root,
            request,
            startup_receipt_ref: &startup.receipt_ref,
            subreceipt_refs: &[],
            diagnostics: &diagnostics,
        });
    }
    let gate_value = match read_ledger_artifact(state_root, gate_receipt_ref) {
        Ok(value) => value,
        Err(error) => {
            diagnostics.push(format!("node control gate receipt not found in node ledger: {error}"));
            return finalize_operation_dispatch(&OperationFinalizeInput {
                state_root,
                request,
                startup_receipt_ref: &startup.receipt_ref,
                subreceipt_refs: &[],
                diagnostics: &diagnostics,
            });
        }
    };
    let validation =
        crate::octet_gate::validate_octet_source_gate(&crate::octet_gate::OctetSourceGateValidationInput {
            consumer: "node-control-gate".to_string(),
            subject_ref: subject_ref.to_string(),
            gate_receipt_value: Some(gate_value),
            source_scope: crate::octet_gate::default_source_scope("node-control-gate")?,
        })?;
    write_preserves(
        &control_operation_subreceipt_path(state_root, &request.request_ref, "octet-source-gate"),
        &validation.value,
    )?;
    import_artifact(state_root, &validation.value)?;
    diagnostics.extend(validation.diagnostics.iter().cloned());
    if validation.decision != "pass" && diagnostics.is_empty() {
        diagnostics.push("node control gate validation denied".to_string());
    }
    finalize_operation_dispatch(&OperationFinalizeInput {
        state_root,
        request,
        startup_receipt_ref: &startup.receipt_ref,
        subreceipt_refs: std::slice::from_ref(&validation.validation_ref),
        diagnostics: &diagnostics,
    })
}

fn finalize_operation_dispatch(input: &OperationFinalizeInput<'_>) -> Result<ControlDispatch> {
    let decision = if input.diagnostics.is_empty() { "pass" } else { "deny" };
    let operation_receipt = operation_receipt_value(&OperationReceiptValueInput {
        decision,
        request: input.request,
        diagnostics: input.diagnostics,
    })?;
    let operation_receipt_ref = crate::preserves_rail::canonical_hash(&operation_receipt)?;
    write_preserves(&control_operation_receipt_path(input.state_root, &input.request.request_ref), &operation_receipt)?;
    import_artifact(input.state_root, &operation_receipt)?;
    let mut all_subreceipt_refs = Vec::with_capacity(input.subreceipt_refs.len() + 1);
    all_subreceipt_refs.extend(input.subreceipt_refs.iter().cloned());
    all_subreceipt_refs.push(operation_receipt_ref);
    let control_receipt = control_receipt_for_request(
        input.state_root,
        input.request,
        input.startup_receipt_ref,
        &all_subreceipt_refs,
        input.diagnostics,
    )?;
    let control_receipt_ref = crate::preserves_rail::canonical_hash(&control_receipt)?;
    write_preserves(&control_outbox_receipt_path(input.state_root, &input.request.request_ref), &control_receipt)?;
    import_artifact(input.state_root, &control_receipt)?;
    Ok(ControlDispatch {
        operation: input.request.operation.clone(),
        request_ref: input.request.request_ref.clone(),
        control_receipt_ref,
        control_receipt_value: control_receipt,
        subreceipt_refs: all_subreceipt_refs,
    })
}

fn side_effect_preflight_diagnostics(request: &crate::node_runtime::ControlRequest) -> Vec<String> {
    let mut diagnostics = Vec::with_capacity(3);
    if request.authority_refs.is_empty() {
        diagnostics.push("node control authority refs missing".to_string());
    }
    if request.policy_refs.is_empty() {
        diagnostics.push("node control policy refs missing".to_string());
    }
    if request.resource_refs.is_empty() {
        diagnostics.push("node control resource refs missing".to_string());
    }
    diagnostics
}

fn read_ledger_artifact(state_root: &Path, artifact_ref: &str) -> Result<IoValue> {
    crate::ledger::read_artifact(&state_root.join("ledger"), artifact_ref)
}

fn control_receipt_for_request(
    state_root: &Path,
    request: &crate::node_runtime::ControlRequest,
    startup_receipt_ref: &str,
    subreceipt_refs: &[String],
    diagnostics: &[String],
) -> Result<IoValue> {
    let decision = if diagnostics.is_empty()
        && !request.authority_refs.is_empty()
        && !request.policy_refs.is_empty()
        && !request.resource_refs.is_empty()
    {
        "pass"
    } else {
        "deny"
    };
    let mut receipt_diagnostics = Vec::with_capacity(diagnostics.len() + 3);
    receipt_diagnostics.extend(diagnostics.iter().cloned());
    if request.authority_refs.is_empty() {
        receipt_diagnostics.push("node control authority refs missing".to_string());
    }
    if request.policy_refs.is_empty() {
        receipt_diagnostics.push("node control policy refs missing".to_string());
    }
    if request.resource_refs.is_empty() {
        receipt_diagnostics.push("node control resource refs missing".to_string());
    }
    let final_decision = if receipt_diagnostics.is_empty() {
        decision
    } else {
        "deny"
    };
    let authority_receipt_refs = if final_decision == "pass" {
        capability_receipt_refs(state_root)?
    } else {
        Vec::new()
    };
    let resource_receipt_refs = if final_decision == "pass" {
        resource_receipt_refs(state_root)?
    } else {
        Vec::new()
    };
    crate::node_runtime::control_receipt_value(&crate::node_runtime::ControlReceiptValueInput {
        decision: final_decision,
        request,
        startup_receipt_ref,
        authority_receipt_refs: &authority_receipt_refs,
        resource_receipt_refs: &resource_receipt_refs,
        subreceipt_refs,
        diagnostics: &receipt_diagnostics,
    })
}

fn authority_receipt_value(input: &AuthorityReceiptValueInput<'_>) -> Result<IoValue> {
    validate_decision(input.decision)?;
    Ok(crate::preserves_rail::record("node-control-authority-receipt-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::NODE_CONTROL_AUTHORITY_RECEIPT_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(input.decision)]),
        crate::preserves_rail::record("envelope", vec![crate::preserves_rail::string(&input.envelope.envelope_ref)]),
        crate::preserves_rail::record("request", vec![crate::preserves_rail::string(
            &input.envelope.request.request_ref,
        )]),
        crate::preserves_rail::record("from-peer", vec![crate::preserves_rail::string(&input.envelope.from_peer)]),
        crate::preserves_rail::record("to-node", vec![crate::preserves_rail::string(&input.envelope.to_node)]),
        crate::preserves_rail::record("operation", vec![crate::preserves_rail::string(
            &input.envelope.request.operation,
        )]),
        crate::preserves_rail::record("grant", vec![optional_string(input.grant_ref)]),
        crate::preserves_rail::record("diagnostics", vec![crate::preserves_rail::sequence(
            input.diagnostics.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("checks", vec![crate::preserves_rail::sequence(vec![
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("peer-node-bound"),
                crate::preserves_rail::string(if input.grant_ref.is_some() { "pass" } else { "fail" }),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("operation-scope-bound"),
                crate::preserves_rail::string(if input.grant_ref.is_some() { "pass" } else { "fail" }),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("revocation-checked-at-ingress"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("transport-is-not-authority"),
                crate::preserves_rail::string("pass"),
            ]),
        ])]),
    ]))
}

fn live_listener_receipt_value(input: &ListenerReceiptValueInput<'_>) -> Result<IoValue> {
    validate_decision(input.decision)?;
    Ok(crate::preserves_rail::record("node-control-live-listener-receipt-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::NODE_CONTROL_LIVE_LISTENER_RECEIPT_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(input.decision)]),
        crate::preserves_rail::record("startup", vec![crate::preserves_rail::string(input.startup_receipt_ref)]),
        crate::preserves_rail::record("node", vec![crate::preserves_rail::string(input.node_id)]),
        crate::preserves_rail::record("logical-endpoint", vec![crate::preserves_rail::string(
            input.logical_endpoint_id,
        )]),
        crate::preserves_rail::record("bound-endpoint", vec![crate::preserves_rail::string(input.bound_endpoint_id)]),
        crate::preserves_rail::record("topic", vec![crate::preserves_rail::string(input.topic)]),
        crate::preserves_rail::record("max-events", vec![crate::preserves_rail::string(input.max_events.to_string())]),
        crate::preserves_rail::record("observed-events", vec![crate::preserves_rail::string(
            input.observed_events.to_string(),
        )]),
        crate::preserves_rail::record("transport-receipts", vec![crate::preserves_rail::sequence(
            input.transport_receipt_refs.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("neighbor-events", vec![crate::preserves_rail::sequence(
            input.neighbor_events.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("service-run", vec![crate::preserves_rail::string(input.service_receipt_ref)]),
        crate::preserves_rail::record("diagnostics", vec![crate::preserves_rail::sequence(
            input.diagnostics.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("checks", vec![crate::preserves_rail::sequence(vec![
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("live-iroh-listener"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("receive-before-drain"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("session-evidence-not-authority"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("bounded-listener"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("durable-inbox-boundary"),
                crate::preserves_rail::string("pass"),
            ]),
        ])]),
    ]))
}

fn live_transport_receipt_value(input: &LiveTransportReceiptValueInput<'_>) -> Result<IoValue> {
    validate_decision(input.decision)?;
    let has_peer_bootstrap = !input.envelope.peer_bootstrap_refs.is_empty();
    Ok(crate::preserves_rail::record("node-control-live-transport-receipt-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::NODE_CONTROL_LIVE_TRANSPORT_RECEIPT_SCHEMA),
        crate::preserves_rail::record("operation", vec![crate::preserves_rail::string(input.operation)]),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(input.decision)]),
        crate::preserves_rail::record("transport", vec![crate::preserves_rail::string(LIVE_CONTROL_INGRESS_TRANSPORT)]),
        crate::preserves_rail::record("topic", vec![crate::preserves_rail::string(&input.envelope.topic)]),
        crate::preserves_rail::record("node", vec![crate::preserves_rail::string(input.node_id)]),
        crate::preserves_rail::record("delivered-from", vec![optional_string(input.delivered_from)]),
        crate::preserves_rail::record("envelope", vec![crate::preserves_rail::string(&input.envelope.envelope_ref)]),
        crate::preserves_rail::record("ingress-receipt", vec![optional_string(input.ingress_receipt_ref)]),
        crate::preserves_rail::record("diagnostics", vec![crate::preserves_rail::sequence(
            input.diagnostics.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("checks", vec![crate::preserves_rail::sequence(vec![
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("canonical-envelope-ref"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("live-iroh-gossip"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("peer-bootstrap-before-enqueue"),
                crate::preserves_rail::string(if has_peer_bootstrap { "pass" } else { "fail" }),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("transport-is-not-authority"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("durable-inbox-boundary"),
                crate::preserves_rail::string("pass"),
            ]),
        ])]),
    ]))
}

fn live_workflow_receipt_value(input: &LiveWorkflowReceiptValueInput<'_>) -> Result<IoValue> {
    validate_decision(input.decision)?;
    Ok(crate::preserves_rail::record("node-control-live-workflow-receipt-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::NODE_CONTROL_LIVE_WORKFLOW_RECEIPT_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(input.decision)]),
        crate::preserves_rail::record("topic", vec![crate::preserves_rail::string(&input.ticket.topic)]),
        crate::preserves_rail::record("peer", vec![crate::preserves_rail::string(&input.admission.peer_id)]),
        crate::preserves_rail::record("node", vec![crate::preserves_rail::string(&input.ticket.node_id)]),
        crate::preserves_rail::record("receiver-ticket", vec![crate::preserves_rail::string(&input.ticket.ticket_ref)]),
        crate::preserves_rail::record("peer-admission", vec![crate::preserves_rail::string(
            &input.admission.admission_ref,
        )]),
        crate::preserves_rail::record("authority-grant", vec![crate::preserves_rail::string(
            &input.authority.grant_ref,
        )]),
        crate::preserves_rail::record("send-receipt", vec![crate::preserves_rail::string(&input.send.receipt_ref)]),
        crate::preserves_rail::record("receive-receipts", vec![crate::preserves_rail::sequence(
            input.receive_receipt_refs.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("listener-receipt", vec![optional_string(input.listener_receipt_ref)]),
        crate::preserves_rail::record("service-run", vec![crate::preserves_rail::string(input.service_receipt_ref)]),
        crate::preserves_rail::record("diagnostics", vec![crate::preserves_rail::sequence(
            input.diagnostics.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("checks", vec![live_workflow_check_sequence(input)]),
    ]))
}

fn live_workflow_check_sequence(input: &LiveWorkflowReceiptValueInput<'_>) -> IoValue {
    crate::preserves_rail::sequence(vec![
        receipt_check_value("ticket-admission-bound", pass_if(input.admission.ticket_ref == input.ticket.ticket_ref)),
        receipt_check_value(
            "authority-grant-bound",
            pass_if(
                input.authority.peer_id == input.admission.peer_id && input.authority.node_id == input.ticket.node_id,
            ),
        ),
        receipt_check_value("send-ticket-bound", pass_if(input.send.receiver_ticket_ref == input.ticket.ticket_ref)),
        receipt_check_value("receive-before-service", fail_if(input.receive_receipt_refs.is_empty())),
        receipt_check_value("transport-is-not-authority", "pass"),
        receipt_check_value("durable-inbox-boundary", "pass"),
    ])
}

struct LiveSendReceiptChecks {
    has_addresses: bool,
    has_supported_addresses: bool,
    has_expected_ticket_binding: bool,
    has_operation_mismatch: bool,
    has_state_root_evidence: bool,
    has_transport_success: bool,
}

fn pass_if(condition: bool) -> &'static str {
    if condition { "pass" } else { "fail" }
}

fn fail_if(condition: bool) -> &'static str {
    if condition { "fail" } else { "pass" }
}

fn receipt_check_value(name: &str, status: &str) -> IoValue {
    crate::preserves_rail::record("check", vec![
        crate::preserves_rail::string(name),
        crate::preserves_rail::string(status),
    ])
}

fn live_send_receipt_checks(input: &LiveSendReceiptValueInput<'_>) -> LiveSendReceiptChecks {
    let has_addresses = !input.ticket.address_refs.is_empty();
    LiveSendReceiptChecks {
        has_addresses,
        has_operation_mismatch: diagnostics_include(input.diagnostics, "operation-id"),
        has_supported_addresses: has_addresses
            && !diagnostics_include(input.diagnostics, "unsupported transport address")
            && !diagnostics_include(input.diagnostics, "address unsupported or malformed")
            && !diagnostics_include(input.diagnostics, "address parse failed")
            && !diagnostics_include(input.diagnostics, "endpoint parse failed"),
        has_expected_ticket_binding: !diagnostics_include(input.diagnostics, "ticket node")
            && !diagnostics_include(input.diagnostics, "ticket topic")
            && !diagnostics_include(input.diagnostics, "ticket endpoint"),
        has_state_root_evidence: !diagnostics_include(input.diagnostics, "sender state root")
            && !diagnostics_include(input.diagnostics, "peer admission refs missing")
            && !diagnostics_include(input.diagnostics, "authority grant refs missing"),
        has_transport_success: input.transport_receipt_ref.is_some(),
    }
}

fn live_send_check_sequence(checks: &LiveSendReceiptChecks) -> IoValue {
    crate::preserves_rail::sequence(vec![
        receipt_check_value("receiver-ticket-bound", "pass"),
        receipt_check_value("receiver-address-bound", pass_if(checks.has_addresses)),
        receipt_check_value("receiver-address-supported", pass_if(checks.has_supported_addresses)),
        receipt_check_value("receiver-ticket-expected", pass_if(checks.has_expected_ticket_binding)),
        receipt_check_value("operation-id-bound", fail_if(checks.has_operation_mismatch)),
        receipt_check_value("sender-state-root-evidence", pass_if(checks.has_state_root_evidence)),
        receipt_check_value("join-or-publish-succeeded", pass_if(checks.has_transport_success)),
        receipt_check_value("canonical-envelope-ref", "pass"),
        receipt_check_value("live-iroh-gossip", "pass"),
        receipt_check_value("transport-is-not-authority", "pass"),
        receipt_check_value("durable-inbox-boundary", "pass"),
    ])
}

fn live_send_receipt_value(input: &LiveSendReceiptValueInput<'_>) -> Result<IoValue> {
    validate_decision(input.decision)?;
    let checks = live_send_receipt_checks(input);
    Ok(crate::preserves_rail::record("node-control-live-send-receipt-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::NODE_CONTROL_LIVE_SEND_RECEIPT_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(input.decision)]),
        crate::preserves_rail::record("transport", vec![crate::preserves_rail::string(LIVE_CONTROL_INGRESS_TRANSPORT)]),
        crate::preserves_rail::record("topic", vec![crate::preserves_rail::string(&input.envelope.topic)]),
        crate::preserves_rail::record("from-peer", vec![crate::preserves_rail::string(input.from_peer)]),
        crate::preserves_rail::record("to-node", vec![crate::preserves_rail::string(&input.ticket.node_id)]),
        crate::preserves_rail::record("receiver-ticket", vec![crate::preserves_rail::string(&input.ticket.ticket_ref)]),
        crate::preserves_rail::record("receiver-endpoint", vec![crate::preserves_rail::string(
            &input.ticket.live_endpoint_id,
        )]),
        crate::preserves_rail::record("receiver-addresses", vec![crate::preserves_rail::sequence(
            input.ticket.address_refs.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("envelope", vec![crate::preserves_rail::string(&input.envelope.envelope_ref)]),
        crate::preserves_rail::record("transport-receipt", vec![optional_string(input.transport_receipt_ref)]),
        crate::preserves_rail::record("diagnostics", vec![crate::preserves_rail::sequence(
            input.diagnostics.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("checks", vec![live_send_check_sequence(&checks)]),
    ]))
}

fn live_send_retry_receipt_value(input: &LiveSendRetryReceiptValueInput<'_>) -> Result<IoValue> {
    validate_decision(input.decision)?;
    Ok(crate::preserves_rail::record("node-control-live-send-retry-receipt-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::NODE_CONTROL_LIVE_SEND_RETRY_RECEIPT_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(input.decision)]),
        crate::preserves_rail::record("attempt", vec![crate::preserves_rail::string(input.attempt.to_string())]),
        crate::preserves_rail::record("max-attempts", vec![crate::preserves_rail::string(
            input.max_attempts.to_string(),
        )]),
        crate::preserves_rail::record("transport", vec![crate::preserves_rail::string(LIVE_CONTROL_INGRESS_TRANSPORT)]),
        crate::preserves_rail::record("topic", vec![crate::preserves_rail::string(&input.envelope.topic)]),
        crate::preserves_rail::record("from-peer", vec![crate::preserves_rail::string(input.from_peer)]),
        crate::preserves_rail::record("to-node", vec![crate::preserves_rail::string(&input.ticket.node_id)]),
        crate::preserves_rail::record("receiver-ticket", vec![crate::preserves_rail::string(&input.ticket.ticket_ref)]),
        crate::preserves_rail::record("receiver-endpoint", vec![crate::preserves_rail::string(
            &input.ticket.live_endpoint_id,
        )]),
        crate::preserves_rail::record("envelope", vec![crate::preserves_rail::string(&input.envelope.envelope_ref)]),
        crate::preserves_rail::record("operation", vec![crate::preserves_rail::string(&input.envelope.operation_ref)]),
        crate::preserves_rail::record("diagnostics", vec![crate::preserves_rail::sequence(
            input.diagnostics.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("checks", vec![crate::preserves_rail::sequence(vec![
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("bounded-retry"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("operation-id-bound"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("transport-is-not-authority"),
                crate::preserves_rail::string("pass"),
            ]),
        ])]),
    ]))
}

fn live_send_duplicate_receipt_value(input: &LiveSendDuplicateReceiptValueInput<'_>) -> Result<IoValue> {
    Ok(crate::preserves_rail::record("node-control-live-send-duplicate-receipt-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::NODE_CONTROL_LIVE_SEND_DUPLICATE_RECEIPT_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string("pass")]),
        crate::preserves_rail::record("transport", vec![crate::preserves_rail::string(LIVE_CONTROL_INGRESS_TRANSPORT)]),
        crate::preserves_rail::record("topic", vec![crate::preserves_rail::string(&input.envelope.topic)]),
        crate::preserves_rail::record("from-peer", vec![crate::preserves_rail::string(input.from_peer)]),
        crate::preserves_rail::record("to-node", vec![crate::preserves_rail::string(&input.ticket.node_id)]),
        crate::preserves_rail::record("receiver-ticket", vec![crate::preserves_rail::string(&input.ticket.ticket_ref)]),
        crate::preserves_rail::record("receiver-endpoint", vec![crate::preserves_rail::string(
            &input.ticket.live_endpoint_id,
        )]),
        crate::preserves_rail::record("envelope", vec![crate::preserves_rail::string(&input.envelope.envelope_ref)]),
        crate::preserves_rail::record("operation", vec![crate::preserves_rail::string(&input.envelope.operation_ref)]),
        crate::preserves_rail::record("prior-send-receipt", vec![crate::preserves_rail::string(
            input.prior_send_receipt_ref,
        )]),
        crate::preserves_rail::record("diagnostics", vec![crate::preserves_rail::sequence(
            input.diagnostics.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("checks", vec![crate::preserves_rail::sequence(vec![
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("duplicate-side-effect-suppressed"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("operation-id-bound"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("prior-send-receipt-bound"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("transport-is-not-authority"),
                crate::preserves_rail::string("pass"),
            ]),
        ])]),
    ]))
}

fn service_lock_value(input: &ServiceLockValueInput<'_>) -> Result<IoValue> {
    Ok(crate::preserves_rail::record("node-control-service-lock-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::NODE_CONTROL_SERVICE_LOCK_SCHEMA),
        crate::preserves_rail::record("state-root", vec![crate::preserves_rail::string(&state_root_profile_ref(
            input.state_root,
        )?)]),
        crate::preserves_rail::record("startup", vec![crate::preserves_rail::string(input.startup_receipt_ref)]),
        crate::preserves_rail::record("node", vec![crate::preserves_rail::string(input.node_id)]),
        crate::preserves_rail::record("topic", vec![crate::preserves_rail::string(input.topic)]),
        crate::preserves_rail::record("max-ticks", vec![crate::preserves_rail::string(input.max_ticks.to_string())]),
        crate::preserves_rail::record("max-requests-per-tick", vec![crate::preserves_rail::string(
            input.max_requests_per_tick.to_string(),
        )]),
        crate::preserves_rail::record("service-run", vec![crate::preserves_rail::string(input.service_run_ref)]),
        crate::preserves_rail::record("profile", vec![crate::preserves_rail::string(
            "local-supervised-node-control-v1",
        )]),
        crate::preserves_rail::record("checks", vec![crate::preserves_rail::sequence(vec![
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("startup-bound"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("single-active-service"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("bounded-ticks"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("not-authority-token"),
                crate::preserves_rail::string("pass"),
            ]),
        ])]),
    ]))
}

fn service_heartbeat_receipt_value(input: &ServiceHeartbeatValueInput<'_>) -> Result<IoValue> {
    Ok(crate::preserves_rail::record("node-control-service-heartbeat-receipt-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::NODE_CONTROL_SERVICE_HEARTBEAT_RECEIPT_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(
            if input.diagnostics.is_empty() { "pass" } else { "deny" },
        )]),
        crate::preserves_rail::record("startup", vec![crate::preserves_rail::string(input.startup_receipt_ref)]),
        crate::preserves_rail::record("service-lock", vec![crate::preserves_rail::string(input.service_lock_ref)]),
        crate::preserves_rail::record("tick", vec![crate::preserves_rail::string(input.tick.to_string())]),
        crate::preserves_rail::record("delivered-count", vec![crate::preserves_rail::string(
            input.delivered_count.to_string(),
        )]),
        crate::preserves_rail::record("processed-count", vec![crate::preserves_rail::string(
            input.processed_count.to_string(),
        )]),
        crate::preserves_rail::record("diagnostics", vec![crate::preserves_rail::sequence(
            input.diagnostics.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("checks", vec![crate::preserves_rail::sequence(vec![
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("service-lock-bound"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("startup-bound"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("monotonic-tick"),
                crate::preserves_rail::string("pass"),
            ]),
        ])]),
    ]))
}

fn supervisor_receipt_value(input: &SupervisorReceiptValueInput<'_>) -> Result<IoValue> {
    validate_decision(input.decision)?;
    Ok(crate::preserves_rail::record("node-control-supervisor-receipt-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::NODE_CONTROL_SUPERVISOR_RECEIPT_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(input.decision)]),
        crate::preserves_rail::record("operation", vec![crate::preserves_rail::string(input.operation)]),
        crate::preserves_rail::record("startup", vec![crate::preserves_rail::string(input.startup_receipt_ref)]),
        crate::preserves_rail::record("service-lock", vec![optional_string(input.service_lock_ref)]),
        crate::preserves_rail::record("policy", vec![optional_string(input.supervisor_policy_ref)]),
        crate::preserves_rail::record("topic", vec![crate::preserves_rail::string(input.topic)]),
        crate::preserves_rail::record("diagnostics", vec![crate::preserves_rail::sequence(
            input.diagnostics.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("checks", vec![crate::preserves_rail::sequence(vec![
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("supervisor-policy-bound"),
                crate::preserves_rail::string(if input.supervisor_policy_ref.is_some() {
                    "pass"
                } else {
                    "fail"
                }),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("single-active-service"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("bounded-restart-policy"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("shutdown-drain-bound"),
                crate::preserves_rail::string("pass"),
            ]),
        ])]),
    ]))
}

fn service_run_check_sequence(input: &ServiceRunReceiptValueInput<'_>) -> IoValue {
    let has_supervisor_policy_binding =
        input.supervisor_policy_ref.is_none() || !input.supervisor_receipt_refs.is_empty();
    crate::preserves_rail::sequence(vec![
        receipt_check_value("single-active-service", pass_if(input.service_lock_ref.is_some())),
        receipt_check_value("ingress-before-loop", "pass"),
        receipt_check_value("loop-reuse", "pass"),
        receipt_check_value("shutdown-stop-semantics", "pass"),
        receipt_check_value("bounded-ticks", "pass"),
        receipt_check_value("supervisor-policy-bound", pass_if(has_supervisor_policy_binding)),
    ])
}

fn service_run_receipt_value(input: &ServiceRunReceiptValueInput<'_>) -> Result<IoValue> {
    validate_decision(input.decision)?;
    Ok(crate::preserves_rail::record("node-control-service-run-receipt-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::NODE_CONTROL_SERVICE_RUN_RECEIPT_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(input.decision)]),
        crate::preserves_rail::record("startup", vec![crate::preserves_rail::string(input.startup_receipt_ref)]),
        crate::preserves_rail::record("service-lock", vec![optional_string(input.service_lock_ref)]),
        crate::preserves_rail::record("topic", vec![crate::preserves_rail::string(input.topic)]),
        crate::preserves_rail::record("max-ticks", vec![crate::preserves_rail::string(input.max_ticks.to_string())]),
        crate::preserves_rail::record("max-requests-per-tick", vec![crate::preserves_rail::string(
            input.max_requests_per_tick.to_string(),
        )]),
        crate::preserves_rail::record("ticks", vec![crate::preserves_rail::string(input.ticks.to_string())]),
        crate::preserves_rail::record("heartbeats", vec![crate::preserves_rail::sequence(
            input.heartbeat_receipt_refs.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("ingress-receipts", vec![crate::preserves_rail::sequence(
            input.ingress_receipt_refs.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("loop-receipts", vec![crate::preserves_rail::sequence(
            input.loop_receipt_refs.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("processed-requests", vec![crate::preserves_rail::sequence(
            input.processed_request_refs.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("stopped", vec![crate::preserves_rail::string(if input.has_stopped {
            "true"
        } else {
            "false"
        })]),
        crate::preserves_rail::record("supervisor-policy", vec![optional_string(input.supervisor_policy_ref)]),
        crate::preserves_rail::record("supervisor-receipts", vec![crate::preserves_rail::sequence(
            input.supervisor_receipt_refs.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("diagnostics", vec![crate::preserves_rail::sequence(
            input.diagnostics.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("checks", vec![service_run_check_sequence(input)]),
    ]))
}

fn ingress_envelope_value(
    input: &ControlIngressEnvelopeInput<'_>,
    request: &crate::node_runtime::ControlRequest,
    operation_ref: &str,
    transport: &str,
    transport_check: &str,
) -> Result<IoValue> {
    Ok(crate::preserves_rail::record("node-control-ingress-envelope-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::NODE_CONTROL_INGRESS_ENVELOPE_SCHEMA),
        crate::preserves_rail::record("transport", vec![crate::preserves_rail::string(transport)]),
        crate::preserves_rail::record("topic", vec![crate::preserves_rail::string(input.topic)]),
        crate::preserves_rail::record("from-peer", vec![crate::preserves_rail::string(input.from_peer)]),
        crate::preserves_rail::record("to-node", vec![crate::preserves_rail::string(input.to_node)]),
        crate::preserves_rail::record("sequence", vec![crate::preserves_rail::string(input.sequence.to_string())]),
        crate::preserves_rail::record("operation", vec![crate::preserves_rail::string(operation_ref)]),
        crate::preserves_rail::record("request-ref", vec![crate::preserves_rail::string(&request.request_ref)]),
        crate::preserves_rail::record("request", vec![request.value.clone()]),
        crate::preserves_rail::record("peer-bootstrap", vec![crate::preserves_rail::sequence(
            input.peer_bootstrap_refs.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("authority", vec![crate::preserves_rail::sequence(
            input.authority_refs.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("policy", vec![crate::preserves_rail::sequence(
            input.policy_refs.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("resource", vec![crate::preserves_rail::sequence(
            input.resource_refs.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("evidence", vec![crate::preserves_rail::sequence(
            input.evidence_refs.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("checks", vec![crate::preserves_rail::sequence(vec![
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("canonical-request-ref"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("operation-id-bound"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string(transport_check),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("transport-is-not-authority"),
                crate::preserves_rail::string("pass"),
            ]),
        ])]),
    ]))
}

fn ingress_receipt_value(input: &IngressReceiptValueInput<'_>) -> Result<IoValue> {
    validate_decision(input.decision)?;
    Ok(crate::preserves_rail::record("node-control-ingress-receipt-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::NODE_CONTROL_INGRESS_RECEIPT_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(input.decision)]),
        crate::preserves_rail::record("phase", vec![crate::preserves_rail::string(input.phase)]),
        crate::preserves_rail::record("transport", vec![crate::preserves_rail::string(input.transport)]),
        crate::preserves_rail::record("topic", vec![crate::preserves_rail::string(&input.envelope.topic)]),
        crate::preserves_rail::record("from-peer", vec![crate::preserves_rail::string(&input.envelope.from_peer)]),
        crate::preserves_rail::record("to-node", vec![crate::preserves_rail::string(&input.envelope.to_node)]),
        crate::preserves_rail::record("sequence", vec![crate::preserves_rail::string(
            input.envelope.sequence.to_string(),
        )]),
        crate::preserves_rail::record("envelope", vec![crate::preserves_rail::string(&input.envelope.envelope_ref)]),
        crate::preserves_rail::record("operation", vec![crate::preserves_rail::string(&input.envelope.operation_ref)]),
        crate::preserves_rail::record("request", vec![crate::preserves_rail::string(
            &input.envelope.request.request_ref,
        )]),
        crate::preserves_rail::record("idempotency", vec![optional_string(input.idempotency_receipt_ref)]),
        crate::preserves_rail::record("queue", vec![optional_string(input.queue_receipt_ref)]),
        crate::preserves_rail::record("diagnostics", vec![crate::preserves_rail::sequence(
            input.diagnostics.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("checks", vec![ingress_check_sequence(input)]),
    ]))
}

fn ingress_check_sequence(input: &IngressReceiptValueInput<'_>) -> IoValue {
    let has_peer_bootstrap = !input.envelope.peer_bootstrap_refs.is_empty();
    let has_authority = !input.envelope.authority_refs.is_empty() && !input.envelope.request.authority_refs.is_empty();
    let has_policy = !input.envelope.policy_refs.is_empty() && !input.envelope.request.policy_refs.is_empty();
    let has_resource = !input.envelope.resource_refs.is_empty() && !input.envelope.request.resource_refs.is_empty();
    crate::preserves_rail::sequence(vec![
        receipt_check_value("peer-bootstrap-bound", pass_if(has_peer_bootstrap)),
        receipt_check_value("authority-before-enqueue", pass_if(has_authority)),
        receipt_check_value(
            "authority-delegation-before-enqueue",
            pass_if(input.envelope.transport != LIVE_CONTROL_INGRESS_TRANSPORT || input.decision == "pass"),
        ),
        receipt_check_value("policy-before-enqueue", pass_if(has_policy)),
        receipt_check_value("resource-before-enqueue", pass_if(has_resource)),
        receipt_check_value(
            "delivery-idempotency-before-enqueue",
            pass_if(input.phase == "publish" || input.idempotency_receipt_ref.is_some() || input.decision == "deny"),
        ),
        receipt_check_value("durable-inbox-boundary", "pass"),
    ])
}

fn queue_receipt_value(input: &QueueReceiptValueInput<'_>) -> Result<IoValue> {
    validate_decision(input.decision)?;
    Ok(crate::preserves_rail::record("node-control-queue-receipt-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::NODE_CONTROL_QUEUE_RECEIPT_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(input.decision)]),
        crate::preserves_rail::record("phase", vec![crate::preserves_rail::string(input.phase)]),
        crate::preserves_rail::record("operation", vec![crate::preserves_rail::string(input.operation)]),
        crate::preserves_rail::record("request", vec![crate::preserves_rail::string(input.request_ref)]),
        crate::preserves_rail::record("profile", vec![crate::preserves_rail::string(
            "local-preserves-control-file-v1",
        )]),
        crate::preserves_rail::record("location", vec![crate::preserves_rail::string(input.location_ref)]),
        crate::preserves_rail::record("diagnostics", vec![crate::preserves_rail::sequence(
            input.diagnostics.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("checks", vec![crate::preserves_rail::sequence(vec![
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("canonical-request-ref"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("durable-control-profile"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("explicit-state-root"),
                crate::preserves_rail::string("pass"),
            ]),
        ])]),
    ]))
}

fn operation_receipt_value(input: &OperationReceiptValueInput<'_>) -> Result<IoValue> {
    validate_decision(input.decision)?;
    Ok(crate::preserves_rail::record("node-control-operation-receipt-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::NODE_CONTROL_OPERATION_RECEIPT_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(input.decision)]),
        crate::preserves_rail::record("operation", vec![crate::preserves_rail::string(&input.request.operation)]),
        crate::preserves_rail::record("request", vec![crate::preserves_rail::string(&input.request.request_ref)]),
        crate::preserves_rail::record("target", vec![optional_string(input.request.target_ref.as_deref())]),
        crate::preserves_rail::record("payload", vec![optional_string(input.request.payload_ref.as_deref())]),
        crate::preserves_rail::record("diagnostics", vec![crate::preserves_rail::sequence(
            input.diagnostics.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("checks", vec![crate::preserves_rail::sequence(vec![
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("operation-dispatch-explicit"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("side-effects-receipted"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("canonical-receipt"),
                crate::preserves_rail::string("pass"),
            ]),
        ])]),
    ]))
}

fn heartbeat_receipt_value(input: &HeartbeatReceiptValueInput<'_>) -> Result<IoValue> {
    Ok(crate::preserves_rail::record("node-control-heartbeat-receipt-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::NODE_CONTROL_HEARTBEAT_RECEIPT_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(
            if input.diagnostics.is_empty() { "pass" } else { "deny" },
        )]),
        crate::preserves_rail::record("startup", vec![crate::preserves_rail::string(input.startup_receipt_ref)]),
        crate::preserves_rail::record("lock", vec![crate::preserves_rail::string(input.lock_ref)]),
        crate::preserves_rail::record("loop-sequence", vec![crate::preserves_rail::string(
            input.loop_sequence.to_string(),
        )]),
        crate::preserves_rail::record("processed-count", vec![crate::preserves_rail::string(
            input.processed_count.to_string(),
        )]),
        crate::preserves_rail::record("profile", vec![crate::preserves_rail::string(
            "local-preserves-control-loop-v1",
        )]),
        crate::preserves_rail::record("diagnostics", vec![crate::preserves_rail::sequence(
            input.diagnostics.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("checks", vec![crate::preserves_rail::sequence(vec![
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("active-lock-bound"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("heartbeat-is-receipted"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("no-ambient-socket-authority"),
                crate::preserves_rail::string("pass"),
            ]),
        ])]),
    ]))
}

fn loop_receipt_value(input: &LoopReceiptValueInput<'_>) -> Result<IoValue> {
    validate_decision(input.decision)?;
    Ok(crate::preserves_rail::record("node-control-loop-receipt-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::NODE_CONTROL_LOOP_RECEIPT_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(input.decision)]),
        crate::preserves_rail::record("startup", vec![crate::preserves_rail::string(input.startup_receipt_ref)]),
        crate::preserves_rail::record("heartbeat", vec![crate::preserves_rail::string(input.heartbeat_receipt_ref)]),
        crate::preserves_rail::record("max-requests", vec![crate::preserves_rail::string(
            input.max_requests.to_string(),
        )]),
        crate::preserves_rail::record("processed-requests", vec![crate::preserves_rail::sequence(
            input.processed_request_refs.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("dispatch-receipts", vec![crate::preserves_rail::sequence(
            input.dispatch_receipt_refs.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("stopped", vec![crate::preserves_rail::string(if input.has_stopped {
            "yes"
        } else {
            "no"
        })]),
        crate::preserves_rail::record("profile", vec![crate::preserves_rail::string(
            "local-preserves-control-loop-v1",
        )]),
        crate::preserves_rail::record("diagnostics", vec![crate::preserves_rail::sequence(
            input.diagnostics.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("checks", vec![crate::preserves_rail::sequence(vec![
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("bounded-request-loop"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("deterministic-inbox-order"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("idempotent-request-dispatch"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("shutdown-stops-loop"),
                crate::preserves_rail::string("pass"),
            ]),
        ])]),
    ]))
}

pub fn summary(value: &IoValue) -> Result<String> {
    if let Some(summary) = runtime_summary(value)? {
        return Ok(summary);
    }
    if let Some(summary) = import_summary(value)? {
        return Ok(summary);
    }
    if let Some(summary) = access_summary(value)? {
        return Ok(summary);
    }
    if let Some(summary) = flow_summary(value)? {
        return Ok(summary);
    }
    if let Some(summary) = bundle_summary(value)? {
        return Ok(summary);
    }
    if let Some(summary) = gate_summary(value)? {
        return Ok(summary);
    }
    if let Some(summary) = apply_summary(value)? {
        return Ok(summary);
    }
    if let Some(summary) = send_summary(value)? {
        return Ok(summary);
    }
    if let Some(summary) = state_summary(value)? {
        return Ok(summary);
    }
    if let Some(summary) = supervisor_summary(value)? {
        return Ok(summary);
    }
    if let Some(summary) = control_summary(value)? {
        return Ok(summary);
    }
    if let Ok(summary) = crate::protocol_session::protocol_summary(value) {
        return Ok(summary);
    }
    if let Ok(summary) = crate::provenance::provenance_summary(value) {
        return Ok(summary);
    }
    Err(MoltenError::invalid_harness("unsupported node daemon artifact for show"))
}

fn runtime_summary(value: &IoValue) -> Result<Option<String>> {
    if let Ok(config) = crate::node_runtime::parse_node_config(value) {
        return Ok(Some(format!(
            "node config ref={} identity={} adapters={}",
            config.config_ref,
            config.node_identity_ref,
            config.adapters.len()
        )));
    }
    if let Ok(startup) = crate::node_runtime::parse_node_startup_receipt(value) {
        return Ok(Some(format!(
            "node startup decision={} receipt={} adapters={}",
            startup.decision,
            startup.receipt_ref,
            startup.adapters.len()
        )));
    }
    if let Ok(control) = crate::node_runtime::parse_control_receipt(value) {
        return Ok(Some(format!(
            "node control decision={} receipt={} request={}",
            control.decision, control.receipt_ref, control.request_ref
        )));
    }
    if let Ok(ingress) = parse_control_ingress_envelope(value) {
        return Ok(Some(format!(
            "node control ingress envelope ref={} topic={} from={} to={} request={}",
            ingress.envelope_ref, ingress.topic, ingress.from_peer, ingress.to_node, ingress.request.request_ref
        )));
    }
    if let Some(fields) = value.collect_simple_record("node-control-ingress-receipt-v1", Some(15)) {
        require_schema(
            &fields[0],
            crate::preserves_rail::NODE_CONTROL_INGRESS_RECEIPT_SCHEMA,
            "node control ingress receipt",
        )?;
        return Ok(Some(format!(
            "node control ingress decision={} phase={} envelope={} request={}",
            record_string(&fields[1], "decision")?,
            record_string(&fields[2], "phase")?,
            record_string(&fields[8], "envelope")?,
            record_string(&fields[10], "request")?
        )));
    }
    Ok(None)
}

fn import_summary(value: &IoValue) -> Result<Option<String>> {
    if let Some(fields) = value.collect_simple_record("node-control-live-ticket-import-receipt-v1", Some(13)) {
        require_schema(
            &fields[0],
            crate::preserves_rail::NODE_CONTROL_LIVE_TICKET_IMPORT_RECEIPT_SCHEMA,
            "node control live ticket import receipt",
        )?;
        return Ok(Some(format!(
            "node control live ticket import decision={} ticket={} imported={}",
            record_string(&fields[1], "decision")?,
            record_string(&fields[3], "ticket")?,
            record_sequence_len(&fields[10], "imported")?
        )));
    }
    if let Some(fields) = value.collect_simple_record("node-control-authority-grant-import-receipt-v1", Some(13)) {
        require_schema(
            &fields[0],
            crate::preserves_rail::NODE_CONTROL_AUTHORITY_GRANT_IMPORT_RECEIPT_SCHEMA,
            "node control authority grant import receipt",
        )?;
        return Ok(Some(format!(
            "node control authority grant import decision={} grant={} imported={}",
            record_string(&fields[1], "decision")?,
            record_string(&fields[3], "grant")?,
            record_sequence_len(&fields[10], "imported")?
        )));
    }
    Ok(None)
}

fn access_summary(value: &IoValue) -> Result<Option<String>> {
    if let Ok(ticket) = parse_control_live_ticket(value) {
        return Ok(Some(format!(
            "node control live ticket ref={} node={} topic={} endpoint={}",
            ticket.ticket_ref, ticket.node_id, ticket.topic, ticket.live_endpoint_id
        )));
    }
    if let Ok(admission) = parse_control_live_peer_admission(value) {
        return Ok(Some(format!(
            "node control live peer admission decision={} peer={} node={} topic={}",
            admission.decision, admission.peer_id, admission.node_id, admission.topic
        )));
    }
    if let Ok(grant) = parse_control_authority_grant(value) {
        return Ok(Some(format!(
            "node control authority grant ref={} peer={} node={} operations={}",
            grant.grant_ref,
            grant.peer_id,
            grant.node_id,
            grant.operations.join(",")
        )));
    }
    if let Some(fields) = value.collect_simple_record("node-control-authority-receipt-v1", Some(10)) {
        require_schema(
            &fields[0],
            crate::preserves_rail::NODE_CONTROL_AUTHORITY_RECEIPT_SCHEMA,
            "node control authority receipt",
        )?;
        return Ok(Some(format!(
            "node control authority decision={} envelope={} operation={} grant={}",
            record_string(&fields[1], "decision")?,
            record_string(&fields[2], "envelope")?,
            record_string(&fields[6], "operation")?,
            record_optional_string(&fields[7], "grant")?.unwrap_or_else(|| "none".to_string())
        )));
    }
    Ok(None)
}

fn flow_summary(value: &IoValue) -> Result<Option<String>> {
    if let Some(fields) = value.collect_simple_record("node-control-live-listener-receipt-v1", Some(14)) {
        require_schema(
            &fields[0],
            crate::preserves_rail::NODE_CONTROL_LIVE_LISTENER_RECEIPT_SCHEMA,
            "node control live listener receipt",
        )?;
        return Ok(Some(format!(
            "node control live listener decision={} topic={} events={} service={}",
            record_string(&fields[1], "decision")?,
            record_string(&fields[6], "topic")?,
            record_string(&fields[8], "observed-events")?,
            record_string(&fields[11], "service-run")?
        )));
    }
    if let Some(fields) = value.collect_simple_record("node-control-live-workflow-receipt-v1", Some(14)) {
        require_schema(
            &fields[0],
            crate::preserves_rail::NODE_CONTROL_LIVE_WORKFLOW_RECEIPT_SCHEMA,
            "node control live workflow receipt",
        )?;
        return Ok(Some(format!(
            "node control live workflow decision={} peer={} node={} send={} service={}",
            record_string(&fields[1], "decision")?,
            record_string(&fields[3], "peer")?,
            record_string(&fields[4], "node")?,
            record_string(&fields[8], "send-receipt")?,
            record_string(&fields[11], "service-run")?
        )));
    }
    Ok(None)
}

fn bundle_summary(value: &IoValue) -> Result<Option<String>> {
    if let Some(fields) = value.collect_simple_record("node-control-live-workflow-bundle-v1", Some(10)) {
        require_schema(
            &fields[0],
            crate::preserves_rail::NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_SCHEMA,
            "node control live workflow bundle",
        )?;
        return Ok(Some(format!(
            "node control live workflow bundle ticket={} admission={} grant={} receipts={}",
            record_string(&fields[5], "ticket-ref")?,
            record_string(&fields[6], "peer-admission-ref")?,
            record_string(&fields[7], "authority-grant-ref")?,
            record_sequence_len(&fields[8], "receipt-refs")?
        )));
    }
    if let Some(fields) = value.collect_simple_record("node-control-live-workflow-bundle-export-receipt-v1", Some(9)) {
        require_schema(
            &fields[0],
            crate::preserves_rail::NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_EXPORT_RECEIPT_SCHEMA,
            "node control live workflow bundle export receipt",
        )?;
        return Ok(Some(format!(
            "node control live workflow bundle export decision={} bundle={} receipts={}",
            record_string(&fields[1], "decision")?,
            record_string(&fields[2], "bundle")?,
            record_sequence_len(&fields[6], "receipts")?
        )));
    }
    if let Some(fields) = value.collect_simple_record("node-control-live-workflow-bundle-import-receipt-v1", Some(12)) {
        require_schema(
            &fields[0],
            crate::preserves_rail::NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_IMPORT_RECEIPT_SCHEMA,
            "node control live workflow bundle import receipt",
        )?;
        return Ok(Some(format!(
            "node control live workflow bundle import decision={} bundle={} imported={}",
            record_string(&fields[1], "decision")?,
            record_string(&fields[3], "bundle")?,
            record_sequence_len(&fields[9], "imported")?
        )));
    }
    Ok(None)
}

fn gate_summary(value: &IoValue) -> Result<Option<String>> {
    if let Some(fields) = value.collect_simple_record("node-control-live-workflow-bundle-verify-receipt-v1", Some(10)) {
        require_schema(
            &fields[0],
            crate::preserves_rail::NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_VERIFY_RECEIPT_SCHEMA,
            "node control live workflow bundle verify receipt",
        )?;
        return Ok(Some(format!(
            "node control live workflow bundle verify decision={} bundle={} receipts={}",
            record_string(&fields[1], "decision")?,
            record_string(&fields[2], "bundle")?,
            record_sequence_len(&fields[6], "receipts")?
        )));
    }
    if let Some(fields) = value.collect_simple_record("node-control-live-workflow-bundle-gate-receipt-v1", Some(12)) {
        require_schema(
            &fields[0],
            crate::preserves_rail::NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_GATE_RECEIPT_SCHEMA,
            "node control live workflow bundle gate receipt",
        )?;
        return Ok(Some(format!(
            "node control live workflow bundle gate decision={} bundle={} verify={}",
            record_string(&fields[1], "decision")?,
            record_string(&fields[2], "bundle")?,
            record_optional_string(&fields[3], "verify-receipt")?.unwrap_or_else(|| "none".to_string())
        )));
    }
    Ok(None)
}

fn apply_summary(value: &IoValue) -> Result<Option<String>> {
    if let Some(fields) = value.collect_simple_record("node-control-live-workflow-bundle-apply-receipt-v1", Some(15)) {
        require_schema(
            &fields[0],
            crate::preserves_rail::NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_APPLY_RECEIPT_SCHEMA,
            "node control live workflow bundle apply receipt",
        )?;
        return Ok(Some(format!(
            "node control live workflow bundle apply decision={} bundle={} mode={} send={}",
            record_string(&fields[1], "decision")?,
            record_string(&fields[3], "bundle")?,
            record_string(&fields[8], "mode")?,
            record_optional_string(&fields[11], "send-receipt")?.unwrap_or_else(|| "none".to_string())
        )));
    }
    if let Some(fields) =
        value.collect_simple_record("node-control-live-workflow-bundle-reconcile-receipt-v1", Some(13))
    {
        require_schema(
            &fields[0],
            crate::preserves_rail::NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_RECONCILE_RECEIPT_SCHEMA,
            "node control live workflow bundle reconcile receipt",
        )?;
        return Ok(Some(format!(
            "node control live workflow bundle reconcile decision={} bundle={} envelope={} control={}",
            record_string(&fields[1], "decision")?,
            record_string(&fields[3], "bundle")?,
            record_optional_string(&fields[8], "envelope")?.unwrap_or_else(|| "none".to_string()),
            record_optional_string(&fields[7], "control-receipt")?.unwrap_or_else(|| "none".to_string())
        )));
    }
    Ok(None)
}

fn send_summary(value: &IoValue) -> Result<Option<String>> {
    if let Some(fields) = value.collect_simple_record("node-control-live-send-retry-receipt-v1", Some(14)) {
        require_schema(
            &fields[0],
            crate::preserves_rail::NODE_CONTROL_LIVE_SEND_RETRY_RECEIPT_SCHEMA,
            "node control live send retry receipt",
        )?;
        return Ok(Some(format!(
            "node control live send retry decision={} attempt={}/{} envelope={}",
            record_string(&fields[1], "decision")?,
            record_string(&fields[2], "attempt")?,
            record_string(&fields[3], "max-attempts")?,
            record_string(&fields[10], "envelope")?
        )));
    }
    if let Some(fields) = value.collect_simple_record("node-control-live-send-duplicate-receipt-v1", Some(13)) {
        require_schema(
            &fields[0],
            crate::preserves_rail::NODE_CONTROL_LIVE_SEND_DUPLICATE_RECEIPT_SCHEMA,
            "node control live send duplicate receipt",
        )?;
        return Ok(Some(format!(
            "node control live send duplicate operation={} prior={}",
            record_string(&fields[9], "operation")?,
            record_string(&fields[10], "prior-send-receipt")?
        )));
    }
    if let Some(fields) = value.collect_simple_record("node-control-live-send-receipt-v1", Some(13)) {
        require_schema(
            &fields[0],
            crate::preserves_rail::NODE_CONTROL_LIVE_SEND_RECEIPT_SCHEMA,
            "node control live send receipt",
        )?;
        return Ok(Some(format!(
            "node control live send decision={} from={} to={} ticket={} envelope={}",
            record_string(&fields[1], "decision")?,
            record_string(&fields[4], "from-peer")?,
            record_string(&fields[5], "to-node")?,
            record_string(&fields[6], "receiver-ticket")?,
            record_string(&fields[9], "envelope")?
        )));
    }
    if let Some(fields) = value.collect_simple_record("node-control-live-transport-receipt-v1", Some(11)) {
        require_schema(
            &fields[0],
            crate::preserves_rail::NODE_CONTROL_LIVE_TRANSPORT_RECEIPT_SCHEMA,
            "node control live transport receipt",
        )?;
        return Ok(Some(format!(
            "node control live transport operation={} decision={} envelope={} ingress={}",
            record_string(&fields[1], "operation")?,
            record_string(&fields[2], "decision")?,
            record_string(&fields[7], "envelope")?,
            record_optional_string(&fields[8], "ingress-receipt")?.unwrap_or_else(|| "none".to_string())
        )));
    }
    Ok(None)
}

fn state_summary(value: &IoValue) -> Result<Option<String>> {
    if let Ok(health) = crate::node_runtime::parse_node_health_receipt(value) {
        return Ok(Some(format!(
            "node health decision={} receipt={} replay={}",
            health.decision, health.receipt_ref, health.replay_status
        )));
    }
    if let Ok(shutdown) = crate::node_runtime::parse_node_shutdown_receipt(value) {
        return Ok(Some(format!(
            "node shutdown decision={} receipt={} adapters={}",
            shutdown.decision,
            shutdown.receipt_ref,
            shutdown.adapters.len()
        )));
    }
    if let Some(fields) = value.collect_simple_record("node-control-lock-v1", Some(6)) {
        return Ok(Some(format!(
            "node control lock startup={} owner={}",
            record_string(&fields[2], "startup")?,
            record_string(&fields[3], "owner")?
        )));
    }
    if let Some(fields) = value.collect_simple_record("node-control-service-lock-v1", Some(10)) {
        return Ok(Some(format!(
            "node control service lock startup={} topic={} max_ticks={}",
            record_string(&fields[2], "startup")?,
            record_string(&fields[4], "topic")?,
            record_string(&fields[5], "max-ticks")?
        )));
    }
    if let Some(fields) = value.collect_simple_record("node-control-service-heartbeat-receipt-v1", Some(9)) {
        return Ok(Some(format!(
            "node control service heartbeat decision={} startup={} tick={}",
            record_string(&fields[1], "decision")?,
            record_string(&fields[2], "startup")?,
            record_string(&fields[4], "tick")?
        )));
    }
    Ok(None)
}

fn supervisor_summary(value: &IoValue) -> Result<Option<String>> {
    if let Ok(policy) = parse_control_supervisor_policy(value) {
        return Ok(Some(format!(
            "node control supervisor policy ref={} restarts={} stale_lock_recovery={}",
            policy.policy_ref, policy.max_restarts, policy.stale_lock_recovery
        )));
    }
    if let Ok(receipt) = parse_control_supervisor_receipt(value) {
        return Ok(Some(format!(
            "node control supervisor decision={} operation={} policy={}",
            receipt.decision,
            receipt.operation,
            receipt.supervisor_policy_ref.unwrap_or_else(|| "none".to_string())
        )));
    }
    if let Some(fields) = value.collect_simple_record("node-control-service-run-receipt-v1", Some(17)) {
        return Ok(Some(format!(
            "node control service run decision={} ticks={} heartbeats={} stopped={}",
            record_string(&fields[1], "decision")?,
            record_string(&fields[7], "ticks")?,
            record_sequence_len(&fields[8], "heartbeats")?,
            record_string(&fields[12], "stopped")?
        )));
    }
    if let Some(fields) = value.collect_simple_record("node-control-service-run-receipt-v1", Some(15)) {
        return Ok(Some(format!(
            "node control service run decision={} ticks={} heartbeats={} stopped={}",
            record_string(&fields[1], "decision")?,
            record_string(&fields[7], "ticks")?,
            record_sequence_len(&fields[8], "heartbeats")?,
            record_string(&fields[12], "stopped")?
        )));
    }
    Ok(None)
}

fn control_summary(value: &IoValue) -> Result<Option<String>> {
    if let Some(fields) = value.collect_simple_record("node-control-queue-receipt-v1", Some(9)) {
        return Ok(Some(format!(
            "node control queue decision={} phase={} request={}",
            record_string(&fields[1], "decision")?,
            record_string(&fields[2], "phase")?,
            record_string(&fields[4], "request")?
        )));
    }
    if let Some(fields) = value.collect_simple_record("node-control-operation-receipt-v1", Some(8)) {
        return Ok(Some(format!(
            "node control operation decision={} operation={} request={}",
            record_string(&fields[1], "decision")?,
            record_string(&fields[2], "operation")?,
            record_string(&fields[3], "request")?
        )));
    }
    if let Some(fields) = value.collect_simple_record("node-control-heartbeat-receipt-v1", Some(9)) {
        return Ok(Some(format!(
            "node control heartbeat decision={} startup={} processed={}",
            record_string(&fields[1], "decision")?,
            record_string(&fields[2], "startup")?,
            record_string(&fields[5], "processed-count")?
        )));
    }
    if let Some(fields) = value.collect_simple_record("node-control-loop-receipt-v1", Some(11)) {
        return Ok(Some(format!(
            "node control loop decision={} startup={} processed={} stopped={}",
            record_string(&fields[1], "decision")?,
            record_string(&fields[2], "startup")?,
            record_sequence_len(&fields[5], "processed-requests")?,
            record_string(&fields[7], "stopped")?
        )));
    }
    Ok(None)
}

fn current_startup_receipt(state_root: &Path) -> Result<crate::node_runtime::NodeStartupReceipt> {
    let startup_value = read_preserves(&state_root.join(STARTUP_FILE))?;
    crate::node_runtime::parse_node_startup_receipt(&startup_value)
}

fn write_active_lock(state_root: &Path, startup_receipt_ref: &str) -> Result<()> {
    let lock_value = active_lock_value(state_root, startup_receipt_ref)?;
    write_preserves(&state_root.join(CONTROL_LOCK_FILE), &lock_value)?;
    import_artifact(state_root, &lock_value)?;
    Ok(())
}

fn require_active_lock(state_root: &Path) -> Result<()> {
    let lock_path = state_root.join(CONTROL_LOCK_FILE);
    if !lock_path.exists() {
        return Err(MoltenError::invalid_harness("node control dispatch requires active node lock"));
    }
    let lock_value = read_preserves(&lock_path)?;
    let fields = lock_value
        .collect_simple_record("node-control-lock-v1", Some(6))
        .ok_or_else(|| MoltenError::invalid_harness("expected <node-control-lock-v1 ...>"))?;
    require_schema(&fields[0], crate::preserves_rail::NODE_CONTROL_LOCK_SCHEMA, "node control lock")?;
    let locked_startup = record_string(&fields[2], "startup")?;
    let startup = current_startup_receipt(state_root)?;
    if locked_startup != startup.receipt_ref {
        return Err(MoltenError::invalid_harness("node control lock is stale for current startup receipt"));
    }
    Ok(())
}

fn remove_active_lock(state_root: &Path) -> Result<()> {
    let path = state_root.join(CONTROL_LOCK_FILE);
    if path.exists() {
        fs::remove_file(path).map_err(MoltenError::from)?;
    }
    Ok(())
}

fn active_lock_value(state_root: &Path, startup_receipt_ref: &str) -> Result<IoValue> {
    Ok(crate::preserves_rail::record("node-control-lock-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::NODE_CONTROL_LOCK_SCHEMA),
        crate::preserves_rail::record("state-root", vec![crate::preserves_rail::string(&state_root_profile_ref(
            state_root,
        )?)]),
        crate::preserves_rail::record("startup", vec![crate::preserves_rail::string(startup_receipt_ref)]),
        crate::preserves_rail::record("owner", vec![crate::preserves_rail::string(&local_ref(
            "node-control-owner",
            startup_receipt_ref,
        )?)]),
        crate::preserves_rail::record("profile", vec![crate::preserves_rail::string(
            "local-preserves-control-file-v1",
        )]),
        crate::preserves_rail::record("checks", vec![crate::preserves_rail::sequence(vec![
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("startup-bound"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("not-authority-token"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("explicit-state-root"),
                crate::preserves_rail::string("pass"),
            ]),
        ])]),
    ]))
}

fn import_artifact(state_root: &Path, value: &IoValue) -> Result<String> {
    let imported = crate::ledger::import_artifact(&state_root.join("ledger"), value)?;
    let receipt_path = state_root
        .join("receipts")
        .join(format!("ledger-import-{}.preserves", ref_file_stem(&imported.artifact_ref)));
    write_preserves(&receipt_path, &imported.receipt_value)?;
    Ok(imported.artifact_ref)
}

fn first_pending_control_request(state_root: &Path) -> Result<PathBuf> {
    next_pending_control_request(state_root)?
        .ok_or_else(|| MoltenError::invalid_harness("node control inbox has no pending requests"))
}

fn next_pending_control_request(state_root: &Path) -> Result<Option<PathBuf>> {
    let mut paths = pending_control_request_paths(state_root)?;
    Ok(paths.pop())
}

fn pending_control_request_paths(state_root: &Path) -> Result<Vec<PathBuf>> {
    let inbox = state_root.join(CONTROL_INBOX_DIR);
    let mut paths = Vec::with_capacity(MAX_PENDING_CONTROL_REQUESTS);
    for entry_result in fs::read_dir(&inbox).map_err(MoltenError::from)? {
        if paths.len() >= MAX_PENDING_CONTROL_REQUESTS {
            return Err(MoltenError::invalid_harness("too many pending node control requests"));
        }
        let entry = entry_result.map_err(MoltenError::from)?;
        let path = entry.path();
        let name = path.file_name().and_then(|value| value.to_str()).unwrap_or_default();
        if path.is_file() && name.ends_with(".preserves") && !name.contains("receipt") {
            paths.push(path);
        }
    }
    paths.sort_by(|left, right| right.cmp(left));
    Ok(paths)
}

fn archive_dispatched_request(state_root: &Path, request_path: &Path, request_value: &IoValue) -> Result<()> {
    let request_ref = crate::preserves_rail::canonical_hash(request_value)?;
    let archived = control_outbox_request_path(state_root, &request_ref);
    write_preserves(&archived, request_value)?;
    if request_path.starts_with(state_root.join(CONTROL_INBOX_DIR)) && request_path.exists() {
        fs::remove_file(request_path).map_err(MoltenError::from)?;
    }
    Ok(())
}

fn control_inbox_path(state_root: &Path, request_ref: &str) -> PathBuf {
    state_root.join(CONTROL_INBOX_DIR).join(format!("{}.preserves", ref_file_stem(request_ref)))
}

fn queue_receipt_path(state_root: &Path, request_ref: &str) -> PathBuf {
    state_root
        .join(CONTROL_INBOX_DIR)
        .join(format!("{}.queue-receipt.preserves", ref_file_stem(request_ref)))
}

fn dispatch_receipt_path(state_root: &Path, request_ref: &str) -> PathBuf {
    state_root
        .join(CONTROL_OUTBOX_DIR)
        .join(format!("{}.dispatch-receipt.preserves", ref_file_stem(request_ref)))
}

fn control_outbox_request_path(state_root: &Path, request_ref: &str) -> PathBuf {
    state_root
        .join(CONTROL_OUTBOX_DIR)
        .join(format!("{}.request.preserves", ref_file_stem(request_ref)))
}

fn control_outbox_receipt_path(state_root: &Path, request_ref: &str) -> PathBuf {
    state_root
        .join(CONTROL_OUTBOX_DIR)
        .join(format!("{}.control-receipt.preserves", ref_file_stem(request_ref)))
}

fn control_operation_receipt_path(state_root: &Path, request_ref: &str) -> PathBuf {
    state_root
        .join(CONTROL_OUTBOX_DIR)
        .join(format!("{}.operation-receipt.preserves", ref_file_stem(request_ref)))
}

fn control_operation_subreceipt_path(state_root: &Path, request_ref: &str, label: &str) -> PathBuf {
    state_root
        .join(CONTROL_OUTBOX_DIR)
        .join(format!("{}.{}.preserves", ref_file_stem(request_ref), label))
}

fn control_heartbeat_receipt_path(state_root: &Path, heartbeat_ref: &str) -> PathBuf {
    state_root
        .join(CONTROL_OUTBOX_DIR)
        .join(format!("{}.heartbeat-receipt.preserves", ref_file_stem(heartbeat_ref)))
}

fn control_loop_receipt_path(state_root: &Path, loop_ref: &str) -> PathBuf {
    state_root
        .join(CONTROL_OUTBOX_DIR)
        .join(format!("{}.loop-receipt.preserves", ref_file_stem(loop_ref)))
}

fn control_service_heartbeat_path(state_root: &Path, heartbeat_ref: &str) -> PathBuf {
    state_root
        .join(CONTROL_SERVICE_DIR)
        .join(format!("{}.service-heartbeat.preserves", ref_file_stem(heartbeat_ref)))
}

fn control_service_run_receipt_path(state_root: &Path, service_run_ref: &str) -> PathBuf {
    state_root
        .join(CONTROL_SERVICE_DIR)
        .join(format!("{}.service-run-receipt.preserves", ref_file_stem(service_run_ref)))
}

fn control_supervisor_receipt_path(state_root: &Path, receipt_ref: &str) -> PathBuf {
    state_root
        .join(CONTROL_SERVICE_DIR)
        .join(format!("{}.supervisor-receipt.preserves", ref_file_stem(receipt_ref)))
}

fn write_supervisor_receipt(state_root: &Path, input: &SupervisorReceiptValueInput<'_>) -> Result<String> {
    let value = supervisor_receipt_value(input)?;
    let receipt_ref = crate::preserves_rail::canonical_hash(&value)?;
    write_preserves(&control_supervisor_receipt_path(state_root, &receipt_ref), &value)?;
    import_artifact(state_root, &value)?;
    Ok(receipt_ref)
}

fn control_ingress_envelope_path(state_root: &Path, topic: &str, envelope_ref: &str) -> PathBuf {
    state_root
        .join(CONTROL_INGRESS_DIR)
        .join(topic)
        .join(format!("{}.envelope.preserves", ref_file_stem(envelope_ref)))
}

fn write_ingress_envelope_and_verify(state_root: &Path, topic: &str, envelope: &ControlIngressEnvelope) -> Result<()> {
    let path = control_ingress_envelope_path(state_root, topic, &envelope.envelope_ref);
    write_preserves(&path, &envelope.value)?;
    let read_value = read_preserves(&path)?;
    let read_envelope = parse_control_ingress_envelope(&read_value)?;
    if read_envelope.envelope_ref != envelope.envelope_ref {
        return Err(MoltenError::invalid_harness(format!(
            "node control ingress materialized envelope ref {} does not match written {}",
            read_envelope.envelope_ref, envelope.envelope_ref
        )));
    }
    Ok(())
}

fn control_ingress_receipt_path(state_root: &Path, envelope_ref: &str, phase: &str) -> PathBuf {
    state_root.join(CONTROL_INGRESS_DIR).join("receipts").join(format!(
        "{}.{}.receipt.preserves",
        ref_file_stem(envelope_ref),
        phase
    ))
}

fn control_live_transport_receipt_path(state_root: &Path, envelope_ref: &str, operation: &str) -> PathBuf {
    state_root.join(CONTROL_INGRESS_DIR).join("receipts").join(format!(
        "{}.live-{}.receipt.preserves",
        ref_file_stem(envelope_ref),
        operation
    ))
}

fn control_live_send_receipt_path(state_root: &Path, send_ref: &str) -> PathBuf {
    state_root
        .join(CONTROL_INGRESS_DIR)
        .join("receipts")
        .join(format!("{}.live-send.receipt.preserves", ref_file_stem(send_ref)))
}

fn control_live_send_retry_receipt_path(state_root: &Path, retry_ref: &str) -> PathBuf {
    state_root
        .join(CONTROL_INGRESS_DIR)
        .join("receipts")
        .join(format!("{}.live-send-retry.receipt.preserves", ref_file_stem(retry_ref)))
}

fn control_live_send_duplicate_receipt_path(state_root: &Path, duplicate_ref: &str) -> PathBuf {
    state_root
        .join(CONTROL_INGRESS_DIR)
        .join("receipts")
        .join(format!("{}.live-send-duplicate.receipt.preserves", ref_file_stem(duplicate_ref)))
}

fn control_live_workflow_receipt_path(state_root: &Path, workflow_ref: &str) -> PathBuf {
    state_root
        .join(CONTROL_INGRESS_DIR)
        .join("receipts")
        .join(format!("{}.live-workflow.receipt.preserves", ref_file_stem(workflow_ref)))
}

fn control_live_listener_receipt_path(state_root: &Path, listener_ref: &str) -> PathBuf {
    state_root
        .join(CONTROL_SERVICE_DIR)
        .join(format!("{}.live-listener-receipt.preserves", ref_file_stem(listener_ref)))
}

fn control_authority_receipt_path(state_root: &Path, envelope_ref: &str) -> PathBuf {
    state_root
        .join(CONTROL_INGRESS_DIR)
        .join("receipts")
        .join(format!("{}.authority-receipt.preserves", ref_file_stem(envelope_ref)))
}

fn ref_file_stem(value_ref: &str) -> String {
    value_ref.replace(':', "-")
}

fn optional_string(value: Option<&str>) -> IoValue {
    match value {
        Some(value) => crate::preserves_rail::record("some", vec![crate::preserves_rail::string(value)]),
        None => crate::preserves_rail::record("none", Vec::new()),
    }
}

fn optional_value(value: Option<&IoValue>) -> IoValue {
    match value {
        Some(value) => crate::preserves_rail::record("some", vec![value.clone()]),
        None => crate::preserves_rail::record("none", Vec::new()),
    }
}

fn diagnostics_include(diagnostics: &[String], needle: &str) -> bool {
    diagnostics.iter().any(|diagnostic| diagnostic.contains(needle))
}

fn record_strings(value: &preserves::Value<preserves::IOValue>, tag: &str) -> Result<Vec<String>> {
    let record_value = crate::preserves_rail::value_to_iovalue(value);
    let fields = record_value
        .collect_simple_record(tag, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{tag} [...]>")))?;
    let items = fields[0]
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness(format!("{tag} must contain a sequence")))?
        .into_owned();
    let mut values = Vec::with_capacity(items.len());
    for item in items {
        let item = item
            .as_string()
            .map(|value| value.into_owned())
            .ok_or_else(|| MoltenError::invalid_harness(format!("{tag} sequence contains non-string")))?;
        values.push(item);
    }
    Ok(values)
}

fn record_optional_string(value: &preserves::Value<preserves::IOValue>, tag: &str) -> Result<Option<String>> {
    let record_value = crate::preserves_rail::value_to_iovalue(value);
    let fields = record_value
        .collect_simple_record(tag, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{tag} optional>")))?;
    let inner = crate::preserves_rail::value_to_iovalue(&fields[0]);
    if inner.collect_simple_record("none", Some(0)).is_some() {
        return Ok(None);
    }
    let some = inner
        .collect_simple_record("some", Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("{tag} must contain <some string> or <none>")))?;
    let value = some[0]
        .as_string()
        .map(|value| value.into_owned())
        .ok_or_else(|| MoltenError::invalid_harness(format!("{tag} <some> must contain a string")))?;
    Ok(Some(value))
}

fn record_optional_value(value: &preserves::Value<preserves::IOValue>, tag: &str) -> Result<Option<IoValue>> {
    let record_value = crate::preserves_rail::value_to_iovalue(value);
    let fields = record_value
        .collect_simple_record(tag, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{tag} optional>")))?;
    let inner = crate::preserves_rail::value_to_iovalue(&fields[0]);
    if inner.collect_simple_record("none", Some(0)).is_some() {
        return Ok(None);
    }
    let some = inner
        .collect_simple_record("some", Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("{tag} must contain <some value> or <none>")))?;
    Ok(Some(crate::preserves_rail::value_to_iovalue(&some[0])))
}

fn record_optional_ref_string(value: &preserves::Value<preserves::IOValue>, tag: &str) -> Result<Option<String>> {
    let reference = record_optional_string(value, tag)?;
    if let Some(reference) = reference.as_ref() {
        validate_ingress_ref(reference, tag)?;
    }
    Ok(reference)
}

fn record_optional_u64_string(value: &preserves::Value<preserves::IOValue>, tag: &str) -> Result<Option<u64>> {
    match record_optional_string(value, tag)? {
        Some(value) => value.parse::<u64>().map(Some).map_err(|_| {
            MoltenError::invalid_harness(format!("{tag} optional value must contain an unsigned integer string"))
        }),
        None => Ok(None),
    }
}

fn validate_decision(decision: &str) -> Result<()> {
    if matches!(decision, "pass" | "deny" | "fail") {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("invalid node control decision `{decision}`")))
    }
}

fn validate_live_send_timeout(timeout_ms: u64) -> Result<()> {
    if timeout_ms == 0 {
        return Err(MoltenError::invalid_harness("node control live send timeout must be positive"));
    }
    if timeout_ms > MAX_CONTROL_LIVE_SEND_TIMEOUT_MS {
        return Err(MoltenError::invalid_harness(format!(
            "node control live send timeout exceeds bounded limit {MAX_CONTROL_LIVE_SEND_TIMEOUT_MS}"
        )));
    }
    Ok(())
}

fn validate_live_send_attempts(max_attempts: u64) -> Result<()> {
    if max_attempts == 0 {
        return Err(MoltenError::invalid_harness("node control live send attempts must be positive"));
    }
    if max_attempts > MAX_CONTROL_LIVE_SEND_ATTEMPTS {
        return Err(MoltenError::invalid_harness(format!(
            "node control live send attempts exceed bounded limit {MAX_CONTROL_LIVE_SEND_ATTEMPTS}"
        )));
    }
    Ok(())
}

fn validate_listener_event_limit(max_events: u64) -> Result<()> {
    if max_events > MAX_CONTROL_LIVE_LISTENER_EVENTS {
        return Err(MoltenError::invalid_harness(format!(
            "node control live listener max events exceeds bounded limit {MAX_CONTROL_LIVE_LISTENER_EVENTS}"
        )));
    }
    Ok(())
}

fn validate_supervisor_policy_bounds(value: u64, label: &str) -> Result<()> {
    if value > MAX_CONTROL_SERVICE_TICKS {
        return Err(MoltenError::invalid_harness(format!(
            "node control supervisor policy {label} exceeds bounded limit {MAX_CONTROL_SERVICE_TICKS}"
        )));
    }
    Ok(())
}

fn validate_service_tick_limit(max_ticks: u64) -> Result<usize> {
    if max_ticks == 0 {
        return Err(MoltenError::invalid_harness("node control service max ticks must be positive"));
    }
    if max_ticks > MAX_CONTROL_SERVICE_TICKS {
        return Err(MoltenError::invalid_harness(format!(
            "node control service max ticks exceeds bounded limit {MAX_CONTROL_SERVICE_TICKS}"
        )));
    }
    usize::try_from(max_ticks)
        .map_err(|_| MoltenError::invalid_harness("node control service max ticks does not fit this platform"))
}

fn validate_loop_request_limit(max_requests: u64) -> Result<usize> {
    if max_requests == 0 {
        return Err(MoltenError::invalid_harness("node control loop max requests must be positive"));
    }
    if max_requests > MAX_CONTROL_LOOP_REQUESTS {
        return Err(MoltenError::invalid_harness(format!(
            "node control loop max requests exceeds bounded limit {MAX_CONTROL_LOOP_REQUESTS}"
        )));
    }
    usize::try_from(max_requests)
        .map_err(|_| MoltenError::invalid_harness("node control loop max requests does not fit this platform"))
}

fn record_string(value: &preserves::Value<preserves::IOValue>, tag: &str) -> Result<String> {
    let record_value = crate::preserves_rail::value_to_iovalue(value);
    let fields = record_value
        .collect_simple_record(tag, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{tag} string>")))?;
    fields[0]
        .as_string()
        .map(|value| value.into_owned())
        .ok_or_else(|| MoltenError::invalid_harness(format!("{tag} must contain a string")))
}

fn record_sequence_len(value: &preserves::Value<preserves::IOValue>, tag: &str) -> Result<usize> {
    let record_value = crate::preserves_rail::value_to_iovalue(value);
    let fields = record_value
        .collect_simple_record(tag, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{tag} sequence>")))?;
    fields[0]
        .collect_sequence()
        .map(|items| items.len())
        .ok_or_else(|| MoltenError::invalid_harness(format!("{tag} must contain a sequence")))
}

fn record_value(value: &preserves::Value<preserves::IOValue>, tag: &str) -> Result<IoValue> {
    let record_value = crate::preserves_rail::value_to_iovalue(value);
    let fields = record_value
        .collect_simple_record(tag, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{tag} value>")))?;
    Ok(crate::preserves_rail::value_to_iovalue(&fields[0]))
}

fn record_values(value: &preserves::Value<preserves::IOValue>, tag: &str) -> Result<Vec<IoValue>> {
    let record_value = crate::preserves_rail::value_to_iovalue(value);
    let fields = record_value
        .collect_simple_record(tag, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{tag} values>")))?;
    let items = fields[0]
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness(format!("{tag} must contain a sequence")))?;
    Ok(items.iter().map(crate::preserves_rail::value_to_iovalue).collect())
}

fn record_ref_string(value: &preserves::Value<preserves::IOValue>, tag: &str) -> Result<String> {
    let reference = record_string(value, tag)?;
    validate_ingress_ref(&reference, tag)?;
    Ok(reference)
}

fn record_ref_strings(value: &preserves::Value<preserves::IOValue>, tag: &str) -> Result<Vec<String>> {
    let record_value = crate::preserves_rail::value_to_iovalue(value);
    let fields = record_value
        .collect_simple_record(tag, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{tag} sequence>")))?;
    let items = fields[0]
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness(format!("{tag} must contain a sequence")))?;
    let mut refs = Vec::with_capacity(items.len());
    for item in items.iter() {
        let reference = item
            .as_string()
            .map(|value| value.into_owned())
            .ok_or_else(|| MoltenError::invalid_harness(format!("{tag} entries must be strings")))?;
        validate_ingress_ref(&reference, tag)?;
        refs.push(reference);
    }
    Ok(refs)
}

fn record_u64_string(value: &preserves::Value<preserves::IOValue>, tag: &str) -> Result<u64> {
    record_string(value, tag)?.parse::<u64>().map_err(|error| {
        MoltenError::invalid_harness(format!("{tag} must contain an unsigned integer string: {error}"))
    })
}

fn validate_ingress_refs(refs: &[String], label: &str) -> Result<()> {
    for reference in refs {
        validate_ingress_ref(reference, label)?;
    }
    Ok(())
}

fn validate_ingress_ref(reference: &str, label: &str) -> Result<()> {
    crate::preserves_rail::validate_content_ref(reference).map_err(|error| {
        MoltenError::invalid_harness(format!("{label} must be a canonical blake3 content ref: {error}"))
    })
}

fn validate_member_ref(actual: &str, expected: &str, label: &str) -> Result<()> {
    if actual == expected {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("{label} ref {actual} does not match {expected}")))
    }
}

fn validate_optional_member_ref(value: Option<&IoValue>, expected_ref: Option<&str>, label: &str) -> Result<()> {
    match (value, expected_ref) {
        (Some(value), Some(expected)) => {
            validate_member_ref(&crate::preserves_rail::canonical_hash(value)?, expected, label)
        }
        (Some(_), None) => Err(MoltenError::invalid_harness(format!("{label} value present without ref"))),
        (None, Some(expected)) => {
            Err(MoltenError::invalid_harness(format!("{label} ref {expected} present without value")))
        }
        (None, None) => Ok(()),
    }
}

fn require_schema(value: &preserves::Value<preserves::IOValue>, expected: &str, context: &str) -> Result<()> {
    let actual = value
        .as_string()
        .ok_or_else(|| MoltenError::invalid_harness(format!("{context} schema must be a string")))?;
    if actual == expected {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!(
            "{context} schema mismatch: expected {expected}, got {actual}"
        )))
    }
}

fn verify_restart_state(state_root: &Path) -> Result<()> {
    let startup_path = state_root.join(STARTUP_FILE);
    if startup_path.exists() {
        let shutdown_path = state_root.join(SHUTDOWN_FILE);
        if !shutdown_path.exists() {
            return Err(MoltenError::invalid_harness(
                "node daemon restart denied: previous startup has no clean shutdown receipt",
            ));
        }
        let startup_value = read_preserves(&startup_path)?;
        let startup = crate::node_runtime::parse_node_startup_receipt(&startup_value)?;
        let shutdown_ref = crate::preserves_rail::canonical_hash(&read_preserves(&shutdown_path)?)?;
        let head_refs = vec![startup.receipt_ref.clone()];
        let health_value = crate::node_runtime::node_restart_health_receipt_value(
            &crate::node_runtime::RestartHealthReceiptValueInput {
                startup_receipt: &startup,
                shutdown_receipt_ref: Some(&shutdown_ref),
                index_receipt_refs: &index_receipt_refs(state_root)?,
                head_refs: &head_refs,
                open_job_refs: &[],
                diagnostics: &[],
            },
        )?;
        let health = crate::node_runtime::parse_node_health_receipt(&health_value)?;
        write_preserves(&state_root.join(HEALTH_FILE), &health_value)?;
        if health.decision != "pass" {
            return Err(MoltenError::invalid_harness(format!(
                "node daemon restart recovery denied receipt={}",
                health.receipt_ref
            )));
        }
        fs::remove_file(shutdown_path).map_err(MoltenError::from)?;
    }
    Ok(())
}

fn default_adapter_bindings(state_root: &Path) -> Result<Vec<crate::node_runtime::NodeAdapterBinding>> {
    let mut adapters = Vec::with_capacity(crate::node_runtime::REQUIRED_RUNTIME_ADAPTERS.len());
    for name in crate::node_runtime::REQUIRED_RUNTIME_ADAPTERS {
        let profile_ref =
            local_ref("node-adapter-profile", &format!("{}:{name}", state_root_profile_ref(state_root)?))?;
        adapters.push(crate::node_runtime::node_adapter_binding(name, &profile_ref)?);
    }
    Ok(adapters)
}

fn status_request() -> Result<crate::node_runtime::ControlRequest> {
    control_request("status")
}

fn shutdown_request() -> Result<crate::node_runtime::ControlRequest> {
    control_request("shutdown")
}

fn control_request(operation: &str) -> Result<crate::node_runtime::ControlRequest> {
    let authority_refs = vec![local_ref("node-control-authority", operation)?];
    let policy_refs = vec![local_ref("node-control-policy", operation)?];
    let resource_refs = vec![local_ref("node-control-resource", operation)?];
    let value = crate::node_runtime::control_request_value(&crate::node_runtime::ControlRequestValueInput {
        operation,
        target_ref: None,
        payload_ref: None,
        authority_refs: &authority_refs,
        policy_refs: &policy_refs,
        resource_refs: &resource_refs,
        evidence_refs: &[],
    })?;
    crate::node_runtime::parse_control_request(&value)
}

#[cfg(test)]
fn test_live_authority_refs(
    state_root: &Path,
    peer_id: &str,
    node_id: &str,
    operation: &str,
    policy_refs: &[String],
) -> Result<Vec<String>> {
    let operations = vec![operation.to_string()];
    let grant_value = control_authority_grant_value(&ControlAuthorityGrantInput {
        peer_id,
        node_id,
        operations: &operations,
        target_scope: "*",
        resource_scope: "*",
        epoch: 1,
        expires_at: None,
        policy_refs,
        revocation_refs: &[],
        evidence_refs: &[],
    })?;
    let grant = import_control_authority_grant(state_root, &grant_value)?;
    Ok(vec![grant.grant_ref])
}

#[cfg(test)]
fn test_live_peer_bootstrap_refs(
    state_root: &Path,
    peer_id: &str,
    topic: &str,
    policy_refs: &[String],
) -> Result<Vec<String>> {
    let ticket = export_control_live_ticket(&ControlLiveTicketExportInput {
        state_root,
        topic,
        policy_refs,
        evidence_refs: &[],
    })?;
    let admission = admit_control_live_peer(&ControlLivePeerAdmitInput {
        state_root,
        ticket_value: &ticket.value,
        peer_id,
        sequence: 1,
        expires_at: None,
        policy_refs,
        evidence_refs: &[],
    })?;
    Ok(vec![admission.admission_ref])
}

fn index_receipt_refs(state_root: &Path) -> Result<Vec<String>> {
    let root_ref = state_root_profile_ref(state_root)?;
    let mut refs = Vec::with_capacity(crate::node_runtime::REQUIRED_RUNTIME_ADAPTERS.len());
    for name in crate::node_runtime::REQUIRED_RUNTIME_ADAPTERS {
        refs.push(local_ref("node-index-verify", &format!("{root_ref}:{name}"))?);
    }
    Ok(refs)
}

fn resource_receipt_refs(state_root: &Path) -> Result<Vec<String>> {
    Ok(vec![local_ref(
        "node-resource-profile",
        &state_root_profile_ref(state_root)?,
    )?])
}

fn capability_receipt_refs(state_root: &Path) -> Result<Vec<String>> {
    Ok(vec![local_ref(
        "node-authority-profile",
        &state_root_profile_ref(state_root)?,
    )?])
}

fn state_root_profile_ref(state_root: &Path) -> Result<String> {
    local_ref("node-state-root-profile", &state_root.display().to_string())
}

fn local_ref(kind: &str, label: &str) -> Result<String> {
    crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("node-daemon-local-ref-v1", vec![
        crate::preserves_rail::string(kind),
        crate::preserves_rail::string(label),
    ]))
}

fn ensure_state_layout(state_root: &Path) -> Result<()> {
    fs::create_dir_all(state_root).map_err(MoltenError::from)?;
    for child in [
        "identity",
        "ledger",
        "registry",
        "chunks",
        "storage",
        "cache",
        "remote-dataspace",
        "services",
        "jobs",
        "coordination",
        "plugin-host",
        "catalog-mcp",
        "control",
        CONTROL_INBOX_DIR,
        CONTROL_OUTBOX_DIR,
        CONTROL_INGRESS_DIR,
        CONTROL_IDEMPOTENCY_DIR,
        CONTROL_SERVICE_DIR,
        "receipts",
    ] {
        fs::create_dir_all(state_root.join(child)).map_err(MoltenError::from)?;
    }
    Ok(())
}

fn validate_state_root(state_root: &Path) -> Result<()> {
    if state_root.as_os_str().is_empty() {
        return Err(MoltenError::invalid_harness("node daemon requires explicit state root"));
    }
    if state_root == Path::new(".") {
        return Err(MoltenError::invalid_harness("node daemon state root cannot be ambient current directory"));
    }
    Ok(())
}

fn validate_node_id(node_id: &str) -> Result<()> {
    if node_id.trim().is_empty() {
        Err(MoltenError::invalid_harness("node daemon id must not be empty"))
    } else {
        Ok(())
    }
}

fn write_preserves(path: &Path, value: &IoValue) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(MoltenError::from)?;
    }
    fs::write(path, crate::preserves_rail::to_text(value)?).map_err(MoltenError::from)
}

fn read_preserves(path: &Path) -> Result<IoValue> {
    let text = fs::read_to_string(path).map_err(MoltenError::from)?;
    crate::preserves_rail::parse_text(&text)
}

pub fn config_path(state_root: &Path) -> PathBuf {
    state_root.join(CONFIG_FILE)
}

pub fn startup_path(state_root: &Path) -> PathBuf {
    state_root.join(STARTUP_FILE)
}

pub fn shutdown_path(state_root: &Path) -> PathBuf {
    state_root.join(SHUTDOWN_FILE)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ingress_ref_parser_rejects_short_fixture_refs() {
        let error = validate_ingress_ref("blake3:fixture", "node control ingress payload ref")
            .expect_err("short fixture ref denied");
        assert!(error.to_string().contains("canonical blake3 content ref"));
        validate_ingress_ref(
            "blake3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
            "node control ingress payload ref",
        )
        .expect("valid canonical ref");
    }

    #[test]
    fn local_node_init_run_status_stop_and_restart_recovery_are_receipted() {
        let root = temp_dir("node-daemon-lifecycle");
        let init = init_local(&InitInput {
            state_root: &root,
            node_id: "node:test",
        })
        .expect("init node");
        crate::preserves_rail::validate_content_ref(&init.config_ref).expect("config ref is canonical");
        let run = run_local(&RunInput { state_root: &root }).expect("run node");
        crate::preserves_rail::validate_content_ref(&run.startup_ref).expect("startup ref is canonical");
        assert_eq!(run.adapter_receipt_refs.len(), crate::node_runtime::REQUIRED_RUNTIME_ADAPTERS.len());
        let status = status_local(&StatusInput { state_root: &root }).expect("status node");
        assert_eq!(status.status, "running");
        let stop = stop_local(&StopInput { state_root: &root }).expect("stop node");
        crate::preserves_rail::validate_content_ref(&stop.shutdown_ref).expect("shutdown ref is canonical");
        let stopped = status_local(&StatusInput { state_root: &root }).expect("stopped status");
        assert_eq!(stopped.status, "stopped");
        let restarted = run_local(&RunInput { state_root: &root }).expect("restart node");
        crate::preserves_rail::validate_content_ref(&restarted.startup_ref).expect("restart startup ref is canonical");
        let restarted_status = status_local(&StatusInput { state_root: &root }).expect("restarted status");
        assert_eq!(restarted_status.status, "running");
        let stale = run_local(&RunInput { state_root: &root }).expect_err("stale running state denied");
        assert!(stale.to_string().contains("previous startup has no clean shutdown receipt"));
        let startup = crate::node_runtime::parse_node_startup_receipt(&run.startup_value).expect("startup parse");
        let restart = crate::node_runtime::node_restart_health_receipt_value(
            &crate::node_runtime::RestartHealthReceiptValueInput {
                startup_receipt: &startup,
                shutdown_receipt_ref: Some(&stop.shutdown_ref),
                index_receipt_refs: &index_receipt_refs(&root).expect("index refs"),
                head_refs: std::slice::from_ref(&run.startup_ref),
                open_job_refs: &[],
                diagnostics: &[],
            },
        )
        .expect("restart health");
        let restart_health = crate::node_runtime::parse_node_health_receipt(&restart).expect("parse health");
        assert_eq!(restart_health.decision, "pass");
    }

    #[test]
    fn ambient_current_directory_state_root_is_denied() {
        let denied = init_local(&InitInput {
            state_root: Path::new("."),
            node_id: "node:test",
        })
        .expect_err("ambient state denied");
        assert!(denied.to_string().contains("ambient current directory"));
        let request = status_request().expect("status request");
        let control_denied = submit_control_request(&ControlSubmitInput {
            state_root: Path::new("."),
            request_value: &request.value,
        })
        .expect_err("ambient control denied");
        assert!(control_denied.to_string().contains("ambient current directory"));
    }

    #[test]
    fn control_inbox_dispatch_imports_receipts_and_denies_missing_operation_payloads() {
        let root = initialized_control_root("node-control-socket", "node:control");
        let status = assert_status_dispatch(&root);
        assert_missing_payload_denied(&root, &status);
        assert_missing_authority_denied(&root, &status);
        assert_shutdown_dispatch(&root);
        assert_dispatch_requires_lock(&root);
    }

    fn initialized_control_root(label: &str, node_id: &str) -> PathBuf {
        let root = temp_dir(label);
        init_local(&InitInput {
            state_root: &root,
            node_id,
        })
        .expect("init node");
        run_local(&RunInput { state_root: &root }).expect("run node");
        root
    }

    fn submit_and_dispatch(root: &Path, request_value: &IoValue) -> ControlDispatch {
        let submitted = submit_control_request(&ControlSubmitInput {
            state_root: root,
            request_value,
        })
        .expect("submit request");
        assert!(submitted.inbox_path.exists());
        dispatch_control_request(&ControlDispatchInput {
            state_root: root,
            request_path: Some(&submitted.inbox_path),
        })
        .expect("dispatch request")
    }

    fn assert_status_dispatch(root: &Path) -> crate::node_runtime::ControlRequest {
        let request = status_request().expect("status request");
        let dispatched = submit_and_dispatch(root, &request.value);
        assert_eq!(dispatched.operation, "status");
        let receipt =
            crate::node_runtime::parse_control_receipt(&dispatched.control_receipt_value).expect("control receipt");
        assert_eq!(receipt.decision, "pass");
        assert_eq!(receipt.request_ref, request.request_ref);
        assert_ledger_contains(root, &[
            "node-control-request",
            "node-control-queue-receipt",
            "node-health-receipt",
            "node-control-receipt",
        ]);
        request
    }

    fn assert_ledger_contains(root: &Path, expected: &[&str]) {
        let kinds = crate::ledger::list_artifacts(&root.join("ledger"))
            .expect("list ledger")
            .into_iter()
            .map(|entry| entry.artifact_kind)
            .collect::<Vec<_>>();
        for expected_kind in expected {
            assert!(kinds.iter().any(|kind| kind.as_str() == *expected_kind), "missing ledger kind {expected_kind}");
        }
    }

    fn assert_missing_payload_denied(root: &Path, status: &crate::node_runtime::ControlRequest) {
        let target_ref = local_ref("install-target", "fixture").expect("target ref");
        let install_value =
            crate::node_runtime::control_request_value(&crate::node_runtime::ControlRequestValueInput {
                operation: "install",
                target_ref: Some(&target_ref),
                payload_ref: None,
                authority_refs: &status.authority_refs,
                policy_refs: &status.policy_refs,
                resource_refs: &status.resource_refs,
                evidence_refs: &[],
            })
            .expect("install request");
        let dispatch = submit_and_dispatch(root, &install_value);
        let receipt =
            crate::node_runtime::parse_control_receipt(&dispatch.control_receipt_value).expect("install receipt");
        assert_eq!(receipt.decision, "deny");
        assert!(receipt.diagnostics.iter().any(|diagnostic| diagnostic.contains("requires payload ref")));
    }

    fn assert_missing_authority_denied(root: &Path, status: &crate::node_runtime::ControlRequest) {
        let missing_authority =
            crate::node_runtime::control_request_value(&crate::node_runtime::ControlRequestValueInput {
                operation: "status",
                target_ref: None,
                payload_ref: None,
                authority_refs: &[],
                policy_refs: &status.policy_refs,
                resource_refs: &status.resource_refs,
                evidence_refs: &[],
            })
            .expect("missing authority request");
        let dispatch = submit_and_dispatch(root, &missing_authority);
        let receipt =
            crate::node_runtime::parse_control_receipt(&dispatch.control_receipt_value).expect("missing receipt");
        assert_eq!(receipt.decision, "deny");
        assert!(receipt.diagnostics.iter().any(|diagnostic| diagnostic.contains("authority refs missing")));
    }

    fn assert_shutdown_dispatch(root: &Path) {
        let request = shutdown_request().expect("shutdown request");
        let dispatch = submit_and_dispatch(root, &request.value);
        let receipt =
            crate::node_runtime::parse_control_receipt(&dispatch.control_receipt_value).expect("shutdown receipt");
        assert_eq!(receipt.decision, "pass");
        assert!(!root.join(CONTROL_LOCK_FILE).exists());
    }

    fn assert_dispatch_requires_lock(root: &Path) {
        let error = dispatch_control_request(&ControlDispatchInput {
            state_root: root,
            request_path: None,
        })
        .expect_err("dispatch requires lock");
        assert!(error.to_string().contains("active node lock"));
    }

    #[test]
    fn control_loop_processes_queue_idempotently_and_stops_on_shutdown() {
        let root = temp_dir("node-control-loop");
        init_local(&InitInput {
            state_root: &root,
            node_id: "node:loop",
        })
        .expect("init node");
        run_local(&RunInput { state_root: &root }).expect("run node");
        let status_request = status_request().expect("status request");
        submit_control_request(&ControlSubmitInput {
            state_root: &root,
            request_value: &status_request.value,
        })
        .expect("submit status");
        let first_loop = run_control_loop(&ControlLoopInput {
            state_root: &root,
            max_requests: 1,
        })
        .expect("run one status request");
        assert_eq!(first_loop.processed_request_refs, vec![status_request.request_ref.clone()]);
        assert!(!first_loop.has_stopped);
        assert_eq!(crate::ledger::artifact_kind(&first_loop.loop_receipt_value), "node-control-loop-receipt");
        assert_eq!(crate::ledger::artifact_kind(&first_loop.heartbeat_receipt_value), "node-control-heartbeat-receipt");

        submit_control_request(&ControlSubmitInput {
            state_root: &root,
            request_value: &status_request.value,
        })
        .expect("resubmit duplicate status");
        let duplicate_loop = run_control_loop(&ControlLoopInput {
            state_root: &root,
            max_requests: 1,
        })
        .expect("run duplicate status request");
        assert_eq!(duplicate_loop.processed_request_refs, vec![status_request.request_ref.clone()]);
        assert_eq!(duplicate_loop.dispatch_receipt_refs, first_loop.dispatch_receipt_refs);

        let shutdown_request = shutdown_request().expect("shutdown request");
        submit_control_request(&ControlSubmitInput {
            state_root: &root,
            request_value: &shutdown_request.value,
        })
        .expect("submit shutdown");
        let shutdown_loop = run_control_loop(&ControlLoopInput {
            state_root: &root,
            max_requests: DEFAULT_CONTROL_LOOP_REQUESTS,
        })
        .expect("run shutdown request");
        assert!(shutdown_loop.has_stopped);
        assert!(!root.join(CONTROL_LOCK_FILE).exists());
        let after_stop = run_control_loop(&ControlLoopInput {
            state_root: &root,
            max_requests: 1,
        })
        .expect_err("stopped node loop denied");
        assert!(after_stop.to_string().contains("active node lock"));

        let kinds = crate::ledger::list_artifacts(&root.join("ledger"))
            .expect("list loop ledger")
            .into_iter()
            .map(|entry| entry.artifact_kind)
            .collect::<Vec<_>>();
        assert!(kinds.iter().any(|kind| kind == "node-control-loop-receipt"));
        assert!(kinds.iter().any(|kind| kind == "node-control-heartbeat-receipt"));
    }

    #[test]
    fn duplicate_request_with_conflicting_archive_fails_closed() {
        let root = temp_dir("node-control-duplicate-conflict");
        init_local(&InitInput {
            state_root: &root,
            node_id: "node:duplicate",
        })
        .expect("init node");
        run_local(&RunInput { state_root: &root }).expect("run node");
        let status_request = status_request().expect("status request");
        let submitted = submit_control_request(&ControlSubmitInput {
            state_root: &root,
            request_value: &status_request.value,
        })
        .expect("submit status");
        dispatch_control_request(&ControlDispatchInput {
            state_root: &root,
            request_path: Some(&submitted.inbox_path),
        })
        .expect("dispatch status");
        write_preserves(
            &control_outbox_request_path(&root, &status_request.request_ref),
            &crate::preserves_rail::record("tampered-node-control-request", vec![crate::preserves_rail::string(
                "conflict",
            )]),
        )
        .expect("tamper archived request");
        let duplicate = submit_control_request(&ControlSubmitInput {
            state_root: &root,
            request_value: &status_request.value,
        })
        .expect("resubmit duplicate");
        let denied = dispatch_control_request(&ControlDispatchInput {
            state_root: &root,
            request_path: Some(&duplicate.inbox_path),
        })
        .expect_err("conflicting duplicate denied");
        assert!(denied.to_string().contains("conflicts with archived request evidence"));
    }

    #[test]
    fn control_provenance_gate_denies_missing_and_tampered_evidence_before_side_effects() {
        let root = initialized_control_root("node-control-provenance", "node:provenance");
        let refs = case_refs("provenance");

        assert_missing_case(&root, &refs);
        assert_queued_case(&root, &refs);
        assert_tampered_case(&root, &refs);
    }

    struct CaseRefs {
        authority_refs: Vec<String>,
        policy_refs: Vec<String>,
        resource_refs: Vec<String>,
    }

    fn case_refs(label: &str) -> CaseRefs {
        CaseRefs {
            authority_refs: vec![local_ref("node-control-authority", label).expect("authority ref")],
            policy_refs: vec![local_ref("node-control-policy", label).expect("policy ref")],
            resource_refs: vec![local_ref("node-control-resource", label).expect("resource ref")],
        }
    }

    fn request_value(payload_ref: &str, refs: &CaseRefs, evidence_refs: &[String]) -> IoValue {
        crate::node_runtime::control_request_value(&crate::node_runtime::ControlRequestValueInput {
            operation: "install",
            target_ref: None,
            payload_ref: Some(payload_ref),
            authority_refs: &refs.authority_refs,
            policy_refs: &refs.policy_refs,
            resource_refs: &refs.resource_refs,
            evidence_refs,
        })
        .expect("install request")
    }

    fn assert_registry_empty(root: &Path) {
        assert!(
            crate::artifacts::list_artifacts(&root.join("registry"), Some("node-control-artifact"))
                .expect("list registry")
                .is_empty()
        );
    }

    fn assert_missing_case(root: &Path, refs: &CaseRefs) {
        let payload_value =
            crate::preserves_rail::record("node-control-install-payload", vec![crate::preserves_rail::string(
                "missing-provenance",
            )]);
        let payload_ref = import_artifact(root, &payload_value).expect("import payload");
        let request = request_value(&payload_ref, refs, &[]);
        let dispatch = submit_and_dispatch(root, &request);
        let receipt =
            crate::node_runtime::parse_control_receipt(&dispatch.control_receipt_value).expect("control receipt");
        assert_eq!(receipt.decision, "deny");
        assert!(
            receipt
                .subreceipt_refs
                .iter()
                .any(|reference| crate::preserves_rail::validate_content_ref(reference).is_ok())
        );
        assert!(receipt.diagnostics.iter().any(|diagnostic| diagnostic.contains("provenance evidence refs missing")));
        assert_registry_empty(root);
    }

    fn assert_queued_case(root: &Path, refs: &CaseRefs) {
        let payload =
            crate::preserves_rail::record("node-control-install-payload", vec![crate::preserves_rail::string(
                "queued-missing-provenance",
            )]);
        let payload_ref = import_artifact(root, &payload).expect("import queued payload");
        let request = request_value(&payload_ref, refs, &[]);
        let queued = crate::node_runtime::parse_control_request(&request).expect("queued request parse");
        submit_control_request(&ControlSubmitInput {
            state_root: root,
            request_value: &request,
        })
        .expect("submit queued missing provenance");
        let loop_result = run_control_loop(&ControlLoopInput {
            state_root: root,
            max_requests: 1,
        })
        .expect("process queued missing provenance");
        assert_eq!(loop_result.processed_request_refs, vec![queued.request_ref.clone()]);
        let value =
            read_preserves(&control_outbox_receipt_path(root, &queued.request_ref)).expect("queued receipt value");
        let receipt = crate::node_runtime::parse_control_receipt(&value).expect("queued receipt");
        assert_eq!(receipt.decision, "deny");
        assert!(receipt.diagnostics.iter().any(|diagnostic| diagnostic.contains("missing provenance evidence")));
    }

    fn assert_tampered_case(root: &Path, refs: &CaseRefs) {
        let payload =
            crate::preserves_rail::record("node-control-install-payload", vec![crate::preserves_rail::string(
                "tampered-provenance",
            )]);
        let payload_ref = import_artifact(root, &payload).expect("import tampered payload");
        let wrong_artifact_ref = local_ref("node-control-wrong-provenance-artifact", "tampered").expect("wrong ref");
        let wrong_provenance =
            crate::provenance::synthetic_reviewed_provenance_record(&wrong_artifact_ref).expect("wrong provenance");
        let wrong_ref = import_artifact(root, &wrong_provenance).expect("import wrong provenance");
        let evidence_refs = vec![wrong_ref];
        let request = request_value(&payload_ref, refs, &evidence_refs);
        let dispatch = submit_and_dispatch(root, &request);
        let receipt =
            crate::node_runtime::parse_control_receipt(&dispatch.control_receipt_value).expect("tampered receipt");
        assert_eq!(receipt.decision, "deny");
        assert!(receipt.diagnostics.iter().any(|diagnostic| diagnostic.contains("no provenance record matches")));
        assert_registry_empty(root);
    }

    #[test]
    fn control_reproducible_provenance_requires_build_verification_binding() {
        let root = temp_dir("node-control-reproducible-provenance");
        init_local(&InitInput {
            state_root: &root,
            node_id: "node:reproducible-provenance",
        })
        .expect("init node");
        run_local(&RunInput { state_root: &root }).expect("run node");

        let case = build_case(&root);
        assert_install_passes(&root, &case);
    }

    struct BuildMaterial {
        authority_refs: Vec<String>,
        policy_refs: Vec<String>,
        resource_refs: Vec<String>,
        payload_ref: String,
        source_refs: Vec<String>,
        toolchain_refs: Vec<String>,
        dependency_ref: String,
        builder_ref: String,
    }

    struct BuildCase {
        material: BuildMaterial,
        evidence_refs: Vec<String>,
    }

    fn build_case(root: &Path) -> BuildCase {
        let material = build_material(root);
        let evidence_refs = verified_refs(root, &material);
        BuildCase {
            material,
            evidence_refs,
        }
    }

    fn build_material(root: &Path) -> BuildMaterial {
        let payload_value =
            crate::preserves_rail::record("node-control-install-payload", vec![crate::preserves_rail::string(
                "reproducible-provenance",
            )]);
        BuildMaterial {
            authority_refs: vec![local_ref("node-control-authority", "reproducible").expect("authority ref")],
            policy_refs: vec![local_ref("node-control-policy", "reproducible").expect("policy ref")],
            resource_refs: vec![local_ref("node-control-resource", "reproducible").expect("resource ref")],
            payload_ref: import_artifact(root, &payload_value).expect("import payload"),
            source_refs: vec![local_ref("node-control-source", "reproducible").expect("source ref")],
            toolchain_refs: vec![local_ref("node-control-toolchain", "reproducible").expect("toolchain ref")],
            dependency_ref: local_ref("node-control-deps", "reproducible").expect("deps ref"),
            builder_ref: local_ref("node-control-builder", "reproducible").expect("builder ref"),
        }
    }

    fn build_record_for(material: &BuildMaterial) -> IoValue {
        crate::provenance::provenance_build_record_value(&crate::provenance::ProvenanceBuildRecordInput {
            expected_artifact_ref: &material.payload_ref,
            source_refs: &material.source_refs,
            dependency_closure_ref: &material.dependency_ref,
            toolchain_refs: &material.toolchain_refs,
            build_params: &[],
            builder_ref: &material.builder_ref,
            nix_derivation_refs: &[],
            policy_refs: &material.policy_refs,
            evidence_refs: &[],
        })
        .expect("build record")
    }

    fn provenance_record_for(material: &BuildMaterial, build_record_refs: &[String]) -> IoValue {
        crate::provenance::provenance_record_value(&crate::provenance::ProvenanceRecordInput {
            artifact_ref: &material.payload_ref,
            trust_state: crate::provenance::TRUST_STATE_REPRODUCIBLE_VERIFIED,
            source_refs: &material.source_refs,
            dependency_closure_ref: &material.dependency_ref,
            toolchain_refs: &material.toolchain_refs,
            builder_ref: &material.builder_ref,
            review_refs: &[],
            test_refs: &[],
            source_gate_refs: &[],
            policy_refs: &material.policy_refs,
            build_record_refs,
        })
        .expect("reproducible provenance")
    }

    fn verified_refs(root: &Path, material: &BuildMaterial) -> Vec<String> {
        let build_record = build_record_for(material);
        let build_record_ref = import_artifact(root, &build_record).expect("import build record");
        let build_verification =
            crate::provenance::verify_provenance_build(&crate::provenance::ProvenanceBuildVerificationInput {
                build_record_value: &build_record,
                actual_artifact_ref: &material.payload_ref,
                prior_diagnostics: &[],
            })
            .expect("verify build");
        let build_verification_ref =
            import_artifact(root, &build_verification.receipt_value).expect("import build verification");
        let build_record_refs = vec![build_record_ref];
        let provenance_record = provenance_record_for(material, &build_record_refs);
        let provenance_ref = import_artifact(root, &provenance_record).expect("import provenance");
        vec![provenance_ref, build_verification_ref]
    }

    fn install_request_for(case: &BuildCase) -> IoValue {
        crate::node_runtime::control_request_value(&crate::node_runtime::ControlRequestValueInput {
            operation: "install",
            target_ref: None,
            payload_ref: Some(&case.material.payload_ref),
            authority_refs: &case.material.authority_refs,
            policy_refs: &case.material.policy_refs,
            resource_refs: &case.material.resource_refs,
            evidence_refs: &case.evidence_refs,
        })
        .expect("reproducible install request")
    }

    fn assert_install_passes(root: &Path, case: &BuildCase) {
        let request = install_request_for(case);
        let submitted = submit_control_request(&ControlSubmitInput {
            state_root: root,
            request_value: &request,
        })
        .expect("submit reproducible request");
        let dispatch = dispatch_control_request(&ControlDispatchInput {
            state_root: root,
            request_path: Some(&submitted.inbox_path),
        })
        .expect("dispatch reproducible request");
        let receipt = crate::node_runtime::parse_control_receipt(&dispatch.control_receipt_value)
            .expect("reproducible control receipt");
        assert_eq!(receipt.decision, "pass");
        assert!(receipt.diagnostics.is_empty());
        assert!(
            !crate::artifacts::list_artifacts(&root.join("registry"), Some("node-control-artifact"))
                .expect("list registry")
                .is_empty()
        );
    }

    #[test]
    fn control_ingress_enqueues_once_and_preserves_provenance_gate() {
        let root = temp_dir("node-control-ingress");
        init_local(&InitInput {
            state_root: &root,
            node_id: "node:ingress",
        })
        .expect("init node");
        run_local(&RunInput { state_root: &root }).expect("run node");
        let authority_refs = vec![local_ref("node-control-authority", "ingress").expect("authority ref")];
        let policy_refs = vec![local_ref("node-control-policy", "ingress").expect("policy ref")];
        let resource_refs = vec![local_ref("node-control-resource", "ingress").expect("resource ref")];
        let peer_bootstrap_refs = vec![local_ref("peer-bootstrap", "peer:operator").expect("bootstrap ref")];

        let payload_value =
            crate::preserves_rail::record("node-control-ingress-payload", vec![crate::preserves_rail::string(
                "missing-provenance",
            )]);
        let payload_ref = import_artifact(&root, &payload_value).expect("import payload");
        let request_value =
            crate::node_runtime::control_request_value(&crate::node_runtime::ControlRequestValueInput {
                operation: "install",
                target_ref: None,
                payload_ref: Some(&payload_ref),
                authority_refs: &authority_refs,
                policy_refs: &policy_refs,
                resource_refs: &resource_refs,
                evidence_refs: &[],
            })
            .expect("install request");
        let envelope = control_ingress_envelope(&ControlIngressEnvelopeInput {
            request_value: &request_value,
            from_peer: "peer:operator",
            to_node: "node:ingress",
            topic: DEFAULT_CONTROL_INGRESS_TOPIC,
            sequence: 1,
            peer_bootstrap_refs: &peer_bootstrap_refs,
            authority_refs: &authority_refs,
            policy_refs: &policy_refs,
            resource_refs: &resource_refs,
            evidence_refs: &[],
        })
        .expect("ingress envelope");
        assert_enqueued_then_denied(&root, &envelope);
    }

    fn assert_enqueued_then_denied(root: &Path, envelope: &ControlIngressEnvelope) {
        let published = publish_control_ingress(&ControlIngressPublishInput {
            state_root: root,
            envelope_value: &envelope.value,
        })
        .expect("publish ingress");
        assert_eq!(crate::ledger::artifact_kind(&published.receipt_value), "node-control-ingress-receipt");
        let delivered = deliver_control_ingress(&ControlIngressDeliverInput {
            state_root: root,
            topic: DEFAULT_CONTROL_INGRESS_TOPIC,
            envelope_ref: &envelope.envelope_ref,
        })
        .expect("deliver ingress");
        assert!(delivered.has_enqueued);
        assert!(delivered.queue_receipt_ref.is_some());

        let duplicate = deliver_control_ingress(&ControlIngressDeliverInput {
            state_root: root,
            topic: DEFAULT_CONTROL_INGRESS_TOPIC,
            envelope_ref: &envelope.envelope_ref,
        })
        .expect("duplicate ingress");
        assert!(!duplicate.has_enqueued);
        assert!(duplicate.idempotency_receipt_ref.is_some());

        let loop_result = run_control_loop(&ControlLoopInput {
            state_root: root,
            max_requests: 1,
        })
        .expect("dispatch ingress request");
        assert_eq!(loop_result.processed_request_refs.len(), 1);
        let control_value = read_preserves(&control_outbox_receipt_path(root, &delivered.request_ref))
            .expect("read ingress dispatch receipt");
        let control = crate::node_runtime::parse_control_receipt(&control_value).expect("parse control receipt");
        assert_eq!(control.decision, "deny");
        assert!(control.diagnostics.iter().any(|diagnostic| diagnostic.contains("provenance evidence refs missing")));
    }

    #[test]
    fn control_ingress_denies_tampered_materialized_envelope_ref() {
        let pair = materialized_ingress_pair();
        publish_control_ingress(&ControlIngressPublishInput {
            state_root: &pair.root,
            envelope_value: &pair.first.value,
        })
        .expect("publish first");
        write_preserves(
            &control_ingress_envelope_path(&pair.root, DEFAULT_CONTROL_INGRESS_TOPIC, &pair.first.envelope_ref),
            &pair.second.value,
        )
        .expect("tamper materialized envelope");
        let denied = deliver_control_ingress(&ControlIngressDeliverInput {
            state_root: &pair.root,
            topic: DEFAULT_CONTROL_INGRESS_TOPIC,
            envelope_ref: &pair.first.envelope_ref,
        })
        .expect_err("materialized ref mismatch denied");
        assert!(denied.to_string().contains("materialized envelope ref"));
    }

    struct MaterializedIngressPair {
        root: PathBuf,
        first: ControlIngressEnvelope,
        second: ControlIngressEnvelope,
    }

    struct MaterializedIngressRefs {
        authority_refs: Vec<String>,
        policy_refs: Vec<String>,
        resource_refs: Vec<String>,
        peer_bootstrap_refs: Vec<String>,
    }

    const FIRST_MATERIALIZED_ENVELOPE_SEQUENCE: u64 = 1;
    const SECOND_MATERIALIZED_ENVELOPE_SEQUENCE: u64 = 2;

    fn materialized_ingress_pair() -> MaterializedIngressPair {
        let root = initialized_materialized_ingress_root();
        let refs = materialized_ingress_refs();
        let request_value = materialized_request_value(&root, &refs);
        MaterializedIngressPair {
            first: materialized_envelope(&request_value, &refs, FIRST_MATERIALIZED_ENVELOPE_SEQUENCE),
            second: materialized_envelope(&request_value, &refs, SECOND_MATERIALIZED_ENVELOPE_SEQUENCE),
            root,
        }
    }

    fn initialized_materialized_ingress_root() -> PathBuf {
        let root = temp_dir("node-control-ingress-materialized-ref");
        init_local(&InitInput {
            state_root: &root,
            node_id: "node:ingress-materialized",
        })
        .expect("init node");
        run_local(&RunInput { state_root: &root }).expect("run node");
        root
    }

    fn materialized_ingress_refs() -> MaterializedIngressRefs {
        MaterializedIngressRefs {
            authority_refs: vec![local_ref("node-control-authority", "materialized").expect("authority ref")],
            policy_refs: vec![local_ref("node-control-policy", "materialized").expect("policy ref")],
            resource_refs: vec![local_ref("node-control-resource", "materialized").expect("resource ref")],
            peer_bootstrap_refs: vec![local_ref("peer-bootstrap", "peer:materialized").expect("bootstrap ref")],
        }
    }

    fn materialized_request_value(root: &Path, refs: &MaterializedIngressRefs) -> IoValue {
        let payload_value =
            crate::preserves_rail::record("node-control-ingress-payload", vec![crate::preserves_rail::string(
                "materialized",
            )]);
        let payload_ref = import_artifact(root, &payload_value).expect("import payload");
        crate::node_runtime::control_request_value(&crate::node_runtime::ControlRequestValueInput {
            operation: "install",
            target_ref: None,
            payload_ref: Some(&payload_ref),
            authority_refs: &refs.authority_refs,
            policy_refs: &refs.policy_refs,
            resource_refs: &refs.resource_refs,
            evidence_refs: &[],
        })
        .expect("request")
    }

    fn materialized_envelope(
        request_value: &IoValue,
        refs: &MaterializedIngressRefs,
        sequence: u64,
    ) -> ControlIngressEnvelope {
        control_ingress_envelope(&ControlIngressEnvelopeInput {
            request_value,
            from_peer: "peer:materialized",
            to_node: "node:ingress-materialized",
            topic: DEFAULT_CONTROL_INGRESS_TOPIC,
            sequence,
            peer_bootstrap_refs: &refs.peer_bootstrap_refs,
            authority_refs: &refs.authority_refs,
            policy_refs: &refs.policy_refs,
            resource_refs: &refs.resource_refs,
            evidence_refs: &[],
        })
        .expect("materialized envelope")
    }

    #[test]
    fn control_live_workflow_bundle_reconcile_binds_receiver_evidence() {
        let case = reconcile_case();
        let reconciled = assert_reconcile_pass(&case);
        let denials = assert_reconcile_denials(&case);
        let ack = assert_ack_pass(&case, &reconciled, &denials.wrong_envelope);
        assert_ack_denials(&case, &denials, &ack);
    }

    struct ReconcileSeed {
        root: PathBuf,
        policy_refs: Vec<String>,
        resource_refs: Vec<String>,
        peer_bootstrap_refs: Vec<String>,
        authority_refs: Vec<String>,
    }

    struct ReconcileDelivery {
        root: PathBuf,
        request: crate::node_runtime::ControlRequest,
        envelope: ControlIngressEnvelope,
        delivered: ControlIngressDeliver,
        queue_value: IoValue,
        control_value: IoValue,
        control_receipt_ref: String,
        policy_refs: Vec<String>,
        operations: Vec<String>,
    }

    struct ReconcileCase {
        delivery: ReconcileDelivery,
        exported: ControlLiveWorkflowBundleExport,
        gated: ControlLiveWorkflowBundleGate,
        apply_receipt_value: IoValue,
    }

    struct ReconcileDenials {
        missing_receiver: ControlLiveWorkflowBundleReconcile,
        denied_control: IoValue,
        denied_reconcile: ControlLiveWorkflowBundleReconcile,
        wrong_envelope: String,
    }

    struct AckPass {
        import_root: PathBuf,
    }

    fn reconcile_expected<'a>(operations: &'a [String]) -> LiveWorkflowBundleExpectedInput<'a> {
        LiveWorkflowBundleExpectedInput {
            expected_node: Some("node:reconcile"),
            expected_topic: Some(DEFAULT_CONTROL_INGRESS_TOPIC),
            expected_endpoint: None,
            expected_peer: Some("peer:reconcile"),
            expected_operations: operations,
            expected_target_scope: Some("*"),
            expected_resource_scope: Some("*"),
            as_of_sequence: 1,
            as_of_epoch: 1,
        }
    }

    fn reconcile_seed() -> ReconcileSeed {
        let root = temp_dir("node-control-live-workflow-reconcile");
        init_local(&InitInput {
            state_root: &root,
            node_id: "node:reconcile",
        })
        .expect("init node");
        run_local(&RunInput { state_root: &root }).expect("run node");
        let policy_refs = vec![local_ref("node-control-policy", "reconcile").expect("policy ref")];
        let resource_refs = vec![local_ref("node-control-resource", "reconcile").expect("resource ref")];
        let peer_bootstrap_refs =
            test_live_peer_bootstrap_refs(&root, "peer:reconcile", DEFAULT_CONTROL_INGRESS_TOPIC, &policy_refs)
                .expect("peer bootstrap");
        let authority_refs =
            test_live_authority_refs(&root, "peer:reconcile", "node:reconcile", "status", &policy_refs)
                .expect("authority refs");
        ReconcileSeed {
            root,
            policy_refs,
            resource_refs,
            peer_bootstrap_refs,
            authority_refs,
        }
    }

    fn reconcile_request(seed: &ReconcileSeed) -> (IoValue, crate::node_runtime::ControlRequest) {
        let request_value =
            crate::node_runtime::control_request_value(&crate::node_runtime::ControlRequestValueInput {
                operation: "status",
                target_ref: None,
                payload_ref: None,
                authority_refs: &seed.authority_refs,
                policy_refs: &seed.policy_refs,
                resource_refs: &seed.resource_refs,
                evidence_refs: &[],
            })
            .expect("status request");
        let request = crate::node_runtime::parse_control_request(&request_value).expect("request");
        (request_value, request)
    }

    fn deliver_reconcile_envelope(
        seed: &ReconcileSeed,
        request_value: &IoValue,
    ) -> (ControlIngressEnvelope, ControlIngressDeliver) {
        let envelope = control_live_ingress_envelope(&ControlIngressEnvelopeInput {
            request_value,
            from_peer: "peer:reconcile",
            to_node: "node:reconcile",
            topic: DEFAULT_CONTROL_INGRESS_TOPIC,
            sequence: 1,
            peer_bootstrap_refs: &seed.peer_bootstrap_refs,
            authority_refs: &seed.authority_refs,
            policy_refs: &seed.policy_refs,
            resource_refs: &seed.resource_refs,
            evidence_refs: &[],
        })
        .expect("live envelope");
        publish_control_ingress(&ControlIngressPublishInput {
            state_root: &seed.root,
            envelope_value: &envelope.value,
        })
        .expect("publish envelope");
        let delivered = deliver_control_ingress(&ControlIngressDeliverInput {
            state_root: &seed.root,
            topic: DEFAULT_CONTROL_INGRESS_TOPIC,
            envelope_ref: &envelope.envelope_ref,
        })
        .expect("deliver envelope");
        assert!(
            delivered.has_enqueued,
            "{}",
            crate::preserves_rail::to_text(&delivered.ingress_receipt_value).expect("ingress receipt text")
        );
        (envelope, delivered)
    }

    fn dispatched_reconcile(seed: &ReconcileSeed, delivered: &ControlIngressDeliver) -> (IoValue, IoValue, String) {
        run_control_loop(&ControlLoopInput {
            state_root: &seed.root,
            max_requests: 1,
        })
        .expect("dispatch request");
        let queue_value =
            read_preserves(&queue_receipt_path(&seed.root, &delivered.request_ref)).expect("queue receipt");
        let control_value =
            read_preserves(&control_outbox_receipt_path(&seed.root, &delivered.request_ref)).expect("control receipt");
        let control = crate::node_runtime::parse_control_receipt(&control_value).expect("parse control");
        assert_eq!(control.decision, "pass");
        (queue_value, control_value, control.receipt_ref)
    }

    fn reconcile_delivery() -> ReconcileDelivery {
        let seed = reconcile_seed();
        let (request_value, request) = reconcile_request(&seed);
        let (envelope, delivered) = deliver_reconcile_envelope(&seed, &request_value);
        let (queue_value, control_value, control_receipt_ref) = dispatched_reconcile(&seed, &delivered);
        ReconcileDelivery {
            root: seed.root,
            request,
            envelope,
            delivered,
            queue_value,
            control_value,
            control_receipt_ref,
            policy_refs: seed.policy_refs,
            operations: vec!["status".to_string()],
        }
    }

    fn export_reconcile_bundle(delivery: &ReconcileDelivery) -> ControlLiveWorkflowBundleExport {
        let ticket = export_control_live_ticket(&ControlLiveTicketExportInput {
            state_root: &delivery.root,
            topic: DEFAULT_CONTROL_INGRESS_TOPIC,
            policy_refs: &delivery.policy_refs,
            evidence_refs: &[],
        })
        .expect("export reconcile ticket");
        let admission = admit_control_live_peer(&ControlLivePeerAdmitInput {
            state_root: &delivery.root,
            ticket_value: &ticket.value,
            peer_id: "peer:reconcile",
            sequence: 1,
            expires_at: None,
            policy_refs: &delivery.policy_refs,
            evidence_refs: &[],
        })
        .expect("admit reconcile peer");
        let authority_value = control_authority_grant_value(&ControlAuthorityGrantInput {
            peer_id: "peer:reconcile",
            node_id: "node:reconcile",
            operations: &delivery.operations,
            target_scope: "*",
            resource_scope: "*",
            epoch: 1,
            expires_at: None,
            policy_refs: &delivery.policy_refs,
            revocation_refs: &[],
            evidence_refs: &[],
        })
        .expect("reconcile authority value");
        let receipt_values: Vec<&IoValue> = Vec::new();
        let exported = export_control_live_workflow_bundle(&ControlLiveWorkflowBundleExportInput {
            receiver_ticket_value: &ticket.value,
            peer_admission_value: &admission.value,
            authority_grant_value: &authority_value,
            receipt_values: &receipt_values,
        })
        .expect("export reconcile workflow bundle");
        assert_eq!(exported.decision, "pass");
        exported
    }

    fn gate_reconcile_bundle(
        exported: &ControlLiveWorkflowBundleExport,
        expected: &LiveWorkflowBundleExpectedInput<'_>,
    ) -> (ControlLiveWorkflowBundleVerify, ControlLiveWorkflowBundleGate) {
        let verified = verify_control_live_workflow_bundle(&ControlLiveWorkflowBundleVerifyInput {
            bundle_value: &exported.bundle.bundle_value,
            expected_node: expected.expected_node,
            expected_topic: expected.expected_topic,
            expected_endpoint: expected.expected_endpoint,
            expected_peer: expected.expected_peer,
            expected_operations: expected.expected_operations,
            expected_target_scope: expected.expected_target_scope,
            expected_resource_scope: expected.expected_resource_scope,
            as_of_sequence: expected.as_of_sequence,
            as_of_epoch: expected.as_of_epoch,
        })
        .expect("verify reconcile workflow bundle");
        assert_eq!(verified.decision, "pass");
        let gated = gate_control_live_workflow_bundle(&ControlLiveWorkflowBundleGateInput {
            bundle_value: &exported.bundle.bundle_value,
            verify_receipt_value: Some(&verified.receipt_value),
            require_verify_receipt: true,
            expected_node: expected.expected_node,
            expected_topic: expected.expected_topic,
            expected_endpoint: expected.expected_endpoint,
            expected_peer: expected.expected_peer,
            expected_operations: expected.expected_operations,
            expected_target_scope: expected.expected_target_scope,
            expected_resource_scope: expected.expected_resource_scope,
            as_of_sequence: expected.as_of_sequence,
            as_of_epoch: expected.as_of_epoch,
        })
        .expect("gate reconcile workflow bundle");
        assert_eq!(gated.decision, "pass");
        (verified, gated)
    }

    fn apply_reconcile_value(
        delivery: &ReconcileDelivery,
        exported: &ControlLiveWorkflowBundleExport,
        verified: &ControlLiveWorkflowBundleVerify,
        gated: &ControlLiveWorkflowBundleGate,
        expected: &LiveWorkflowBundleExpectedInput<'_>,
    ) -> IoValue {
        let imported_refs = Vec::new();
        let diagnostics = Vec::new();
        live_workflow_bundle_apply_receipt_value(&LiveWorkflowBundleApplyReceiptValueInput {
            decision: "pass",
            state_root: &delivery.root,
            bundle_ref: &exported.bundle.bundle_ref,
            gate_receipt_ref: Some(&gated.receipt_ref),
            recomputed_verify_receipt_ref: &verified.receipt_ref,
            import_receipt_ref: None,
            imported_refs: &imported_refs,
            mode: "dry-run",
            envelope_ref: Some(&delivery.envelope.envelope_ref),
            operation_ref: Some(&delivery.envelope.operation_ref),
            send_receipt_ref: None,
            expected,
            diagnostics: &diagnostics,
        })
        .expect("apply receipt")
    }

    fn reconcile_case() -> ReconcileCase {
        let delivery = reconcile_delivery();
        let expected = reconcile_expected(&delivery.operations);
        let exported = export_reconcile_bundle(&delivery);
        let (verified, gated) = gate_reconcile_bundle(&exported, &expected);
        let apply_receipt_value = apply_reconcile_value(&delivery, &exported, &verified, &gated, &expected);
        ReconcileCase {
            delivery,
            exported,
            gated,
            apply_receipt_value,
        }
    }

    fn assert_reconcile_pass(case: &ReconcileCase) -> ControlLiveWorkflowBundleReconcile {
        let delivery = &case.delivery;
        let reconciled = reconcile_control_live_workflow_bundle(&ControlLiveWorkflowBundleReconcileInput {
            apply_receipt_value: &case.apply_receipt_value,
            send_receipt_value: None,
            ingress_receipt_value: Some(&delivery.delivered.ingress_receipt_value),
            queue_receipt_value: Some(&delivery.queue_value),
            control_receipt_value: Some(&delivery.control_value),
            expected_envelope_ref: Some(&delivery.envelope.envelope_ref),
            expected_operation_ref: Some(&delivery.envelope.operation_ref),
            expected_request_ref: Some(&delivery.delivered.request_ref),
        })
        .expect("reconcile");
        assert_eq!(reconciled.decision, "pass");
        assert_eq!(
            crate::ledger::artifact_kind(&reconciled.receipt_value),
            "node-control-live-workflow-bundle-reconcile-receipt"
        );
        assert_eq!(reconciled.ingress_receipt_ref.as_deref(), Some(delivery.delivered.ingress_receipt_ref.as_str()));
        assert_eq!(reconciled.control_receipt_ref.as_deref(), Some(delivery.control_receipt_ref.as_str()));
        assert!(parse_control_authority_grant(&reconciled.receipt_value).is_err());
        assert!(
            crate::preserves_rail::to_text(&reconciled.receipt_value)
                .expect("reconcile text")
                .contains("reconcile-receipt-is-not-authority")
        );
        import_artifact(&delivery.root, &reconciled.receipt_value).expect("import reconcile receipt");
        assert_reconcile_not_authority(case, &reconciled);
        reconciled
    }

    fn assert_reconcile_not_authority(case: &ReconcileCase, reconciled: &ControlLiveWorkflowBundleReconcile) {
        let refs = vec![reconciled.receipt_ref.clone()];
        let request_value =
            crate::node_runtime::control_request_value(&crate::node_runtime::ControlRequestValueInput {
                operation: "status",
                target_ref: None,
                payload_ref: None,
                authority_refs: &refs,
                policy_refs: &[],
                resource_refs: &[],
                evidence_refs: &[],
            })
            .expect("reconcile authority request");
        let envelope = control_live_ingress_envelope(&ControlIngressEnvelopeInput {
            request_value: &request_value,
            from_peer: "peer:reconcile",
            to_node: "node:reconcile",
            topic: DEFAULT_CONTROL_INGRESS_TOPIC,
            sequence: 2,
            peer_bootstrap_refs: &[],
            authority_refs: &refs,
            policy_refs: &[],
            resource_refs: &[],
            evidence_refs: &[],
        })
        .expect("reconcile authority envelope");
        let diagnostics = live_send_authority_grant_diagnostics(&case.delivery.root, &envelope)
            .expect("reconcile authority diagnostics");
        assert!(diagnostics.iter().any(|value| value.contains("is not a grant")));
        assert!(diagnostics.iter().any(|value| value.contains("authority delegation missing admitted grant")));
    }

    fn assert_reconcile_denials(case: &ReconcileCase) -> ReconcileDenials {
        let delivery = &case.delivery;
        let missing_receiver = reconcile_control_live_workflow_bundle(&ControlLiveWorkflowBundleReconcileInput {
            apply_receipt_value: &case.apply_receipt_value,
            send_receipt_value: None,
            ingress_receipt_value: None,
            queue_receipt_value: None,
            control_receipt_value: None,
            expected_envelope_ref: Some(&delivery.envelope.envelope_ref),
            expected_operation_ref: Some(&delivery.envelope.operation_ref),
            expected_request_ref: Some(&delivery.delivered.request_ref),
        })
        .expect("missing receiver reconcile");
        assert_eq!(missing_receiver.decision, "deny");
        assert!(
            missing_receiver
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.contains("requires receiver ingress receipt"))
        );

        let wrong_envelope = local_ref("node-control-envelope", "wrong-reconcile").expect("wrong envelope");
        let wrong_reconcile = reconcile_control_live_workflow_bundle(&ControlLiveWorkflowBundleReconcileInput {
            apply_receipt_value: &case.apply_receipt_value,
            send_receipt_value: None,
            ingress_receipt_value: Some(&delivery.delivered.ingress_receipt_value),
            queue_receipt_value: Some(&delivery.queue_value),
            control_receipt_value: Some(&delivery.control_value),
            expected_envelope_ref: Some(&wrong_envelope),
            expected_operation_ref: Some(&delivery.envelope.operation_ref),
            expected_request_ref: Some(&delivery.delivered.request_ref),
        })
        .expect("wrong envelope reconcile");
        assert_eq!(wrong_reconcile.decision, "deny");
        assert!(wrong_reconcile.diagnostics.iter().any(|diagnostic| diagnostic.contains("does not match expected")));

        let denied_control = crate::node_runtime::control_deny_receipt_value(
            &delivery.request,
            &local_ref("node-startup", "reconcile-deny").expect("startup ref"),
            "receiver denial propagated",
        )
        .expect("denied control");
        let denied_reconcile = reconcile_control_live_workflow_bundle(&ControlLiveWorkflowBundleReconcileInput {
            apply_receipt_value: &case.apply_receipt_value,
            send_receipt_value: None,
            ingress_receipt_value: Some(&delivery.delivered.ingress_receipt_value),
            queue_receipt_value: Some(&delivery.queue_value),
            control_receipt_value: Some(&denied_control),
            expected_envelope_ref: Some(&delivery.envelope.envelope_ref),
            expected_operation_ref: Some(&delivery.envelope.operation_ref),
            expected_request_ref: Some(&delivery.delivered.request_ref),
        })
        .expect("denied reconcile");
        assert_eq!(denied_reconcile.decision, "deny");
        assert!(
            denied_reconcile
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.contains("receiver denial propagated"))
        );

        ReconcileDenials {
            missing_receiver,
            denied_control,
            denied_reconcile,
            wrong_envelope,
        }
    }

    fn assert_ack_pass(
        case: &ReconcileCase,
        reconciled: &ControlLiveWorkflowBundleReconcile,
        wrong_envelope: &str,
    ) -> AckPass {
        let delivery = &case.delivery;
        let ack_export = export_control_live_workflow_bundle_ack(&ControlLiveWorkflowBundleAckExportInput {
            apply_receipt_value: &case.apply_receipt_value,
            send_receipt_value: None,
            ingress_receipt_value: Some(&delivery.delivered.ingress_receipt_value),
            queue_receipt_value: Some(&delivery.queue_value),
            control_receipt_value: Some(&delivery.control_value),
            reconcile_receipt_value: &reconciled.receipt_value,
        })
        .expect("ack export");
        assert_eq!(ack_export.decision, "pass");
        assert_eq!(ack_export.receiver_decision, "pass");
        assert_eq!(crate::ledger::artifact_kind(&ack_export.ack.ack_value), "node-control-live-workflow-bundle-ack");
        assert_eq!(
            crate::ledger::artifact_kind(&ack_export.receipt_value),
            "node-control-live-workflow-bundle-ack-export-receipt"
        );
        assert!(parse_control_authority_grant(&ack_export.ack.ack_value).is_err());
        assert!(
            crate::preserves_rail::to_text(&ack_export.ack.ack_value)
                .expect("ack text")
                .contains("ack-bundle-is-not-authority")
        );
        let import_root = temp_dir("node-control-live-workflow-ack-import");
        init_local(&InitInput {
            state_root: &import_root,
            node_id: "node:ack-import",
        })
        .expect("init ack import root");
        let ack_import = import_control_live_workflow_bundle_ack(&ControlLiveWorkflowBundleAckImportInput {
            state_root: &import_root,
            ack_value: &ack_export.ack.ack_value,
            expected_bundle_ref: Some(&case.exported.bundle.bundle_ref),
            expected_envelope_ref: Some(&delivery.envelope.envelope_ref),
            expected_operation_ref: Some(&delivery.envelope.operation_ref),
            expected_request_ref: Some(&delivery.delivered.request_ref),
        })
        .expect("ack import");
        assert_eq!(ack_import.decision, "pass");
        assert!(ack_import.imported_refs.iter().any(|reference| reference == &ack_export.ack.ack_ref));
        assert_eq!(
            crate::ledger::artifact_kind(&ack_import.receipt_value),
            "node-control-live-workflow-bundle-ack-import-receipt"
        );
        assert!(
            crate::preserves_rail::to_text(&ack_import.receipt_value)
                .expect("ack import text")
                .contains("ack-import-is-not-authority")
        );
        read_ledger_artifact(&import_root, &ack_export.ack.ack_ref).expect("ack imported");
        read_ledger_artifact(&import_root, &reconciled.receipt_ref).expect("reconcile imported");
        assert_protocol_pass(case, reconciled, &ack_export.ack.ack_value);
        let wrong_ack_import = import_control_live_workflow_bundle_ack(&ControlLiveWorkflowBundleAckImportInput {
            state_root: &import_root,
            ack_value: &ack_export.ack.ack_value,
            expected_bundle_ref: Some(&case.exported.bundle.bundle_ref),
            expected_envelope_ref: Some(wrong_envelope),
            expected_operation_ref: Some(&delivery.envelope.operation_ref),
            expected_request_ref: Some(&delivery.delivered.request_ref),
        })
        .expect("wrong ack import");
        assert_eq!(wrong_ack_import.decision, "deny");
        assert!(wrong_ack_import.diagnostics.iter().any(|value| value.contains("does not match expected")));
        AckPass { import_root }
    }

    fn assert_protocol_pass(
        case: &ReconcileCase,
        reconciled: &ControlLiveWorkflowBundleReconcile,
        ack_value: &IoValue,
    ) {
        let delivery = &case.delivery;
        let protocol_gate = gate_control_live_workflow_protocol(&ControlLiveWorkflowProtocolGateInput {
            bundle_value: &case.exported.bundle.bundle_value,
            gate_receipt_value: &case.gated.receipt_value,
            apply_receipt_value: &case.apply_receipt_value,
            reconcile_receipt_value: &reconciled.receipt_value,
            ack_value,
            expected_envelope_ref: Some(&delivery.envelope.envelope_ref),
            expected_operation_ref: Some(&delivery.envelope.operation_ref),
            expected_request_ref: Some(&delivery.delivered.request_ref),
        })
        .expect("workflow protocol gate");
        assert_eq!(protocol_gate.decision, "pass");
        assert_eq!(protocol_gate.operation_count, 6);
        assert_eq!(protocol_gate.message_count, 3);
        assert_eq!(crate::ledger::artifact_kind(&protocol_gate.receipt_value), "protocol-session-gate-receipt");
        assert!(parse_control_authority_grant(&protocol_gate.receipt_value).is_err());
    }

    fn assert_ack_denials(case: &ReconcileCase, denials: &ReconcileDenials, ack: &AckPass) {
        let delivery = &case.delivery;
        let missing_ack_export = export_control_live_workflow_bundle_ack(&ControlLiveWorkflowBundleAckExportInput {
            apply_receipt_value: &case.apply_receipt_value,
            send_receipt_value: None,
            ingress_receipt_value: None,
            queue_receipt_value: None,
            control_receipt_value: None,
            reconcile_receipt_value: &denials.missing_receiver.receipt_value,
        })
        .expect("missing ack export");
        assert_eq!(missing_ack_export.decision, "deny");
        assert!(
            missing_ack_export
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.contains("requires receiver ingress receipt"))
        );

        let denied_ack_export = export_control_live_workflow_bundle_ack(&ControlLiveWorkflowBundleAckExportInput {
            apply_receipt_value: &case.apply_receipt_value,
            send_receipt_value: None,
            ingress_receipt_value: Some(&delivery.delivered.ingress_receipt_value),
            queue_receipt_value: Some(&delivery.queue_value),
            control_receipt_value: Some(&denials.denied_control),
            reconcile_receipt_value: &denials.denied_reconcile.receipt_value,
        })
        .expect("denied ack export");
        assert_eq!(denied_ack_export.decision, "pass");
        assert_eq!(denied_ack_export.receiver_decision, "deny");
        assert!(
            denied_ack_export
                .ack
                .receiver_diagnostics
                .iter()
                .any(|diagnostic| diagnostic.contains("receiver denial propagated"))
        );
        let denied_ack_import = import_control_live_workflow_bundle_ack(&ControlLiveWorkflowBundleAckImportInput {
            state_root: &ack.import_root,
            ack_value: &denied_ack_export.ack.ack_value,
            expected_bundle_ref: Some(&case.exported.bundle.bundle_ref),
            expected_envelope_ref: Some(&delivery.envelope.envelope_ref),
            expected_operation_ref: Some(&delivery.envelope.operation_ref),
            expected_request_ref: Some(&delivery.delivered.request_ref),
        })
        .expect("denied ack import");
        assert_eq!(denied_ack_import.decision, "pass");
        assert_eq!(denied_ack_import.receiver_decision, "deny");
        let denied_protocol_gate = gate_control_live_workflow_protocol(&ControlLiveWorkflowProtocolGateInput {
            bundle_value: &case.exported.bundle.bundle_value,
            gate_receipt_value: &case.gated.receipt_value,
            apply_receipt_value: &case.apply_receipt_value,
            reconcile_receipt_value: &denials.denied_reconcile.receipt_value,
            ack_value: &denied_ack_export.ack.ack_value,
            expected_envelope_ref: Some(&delivery.envelope.envelope_ref),
            expected_operation_ref: Some(&delivery.envelope.operation_ref),
            expected_request_ref: Some(&delivery.delivered.request_ref),
        })
        .expect("denied workflow protocol gate");
        assert_eq!(denied_protocol_gate.decision, "deny");
        assert!(
            denied_protocol_gate
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.contains("ack receiver decision deny"))
        );
    }

    #[test]
    fn control_ingress_denies_missing_authority_before_enqueue() {
        let root = temp_dir("node-control-ingress-deny");
        init_local(&InitInput {
            state_root: &root,
            node_id: "node:ingress-deny",
        })
        .expect("init node");
        run_local(&RunInput { state_root: &root }).expect("run node");
        let request = status_request().expect("status request");
        let peer_bootstrap_refs = vec![local_ref("peer-bootstrap", "peer:operator").expect("bootstrap ref")];
        let policy_refs = vec![local_ref("node-control-policy", "ingress-deny").expect("policy ref")];
        let resource_refs = vec![local_ref("node-control-resource", "ingress-deny").expect("resource ref")];
        let envelope = control_ingress_envelope(&ControlIngressEnvelopeInput {
            request_value: &request.value,
            from_peer: "peer:operator",
            to_node: "node:ingress-deny",
            topic: DEFAULT_CONTROL_INGRESS_TOPIC,
            sequence: 1,
            peer_bootstrap_refs: &peer_bootstrap_refs,
            authority_refs: &[],
            policy_refs: &policy_refs,
            resource_refs: &resource_refs,
            evidence_refs: &[],
        })
        .expect("missing authority envelope");
        publish_control_ingress(&ControlIngressPublishInput {
            state_root: &root,
            envelope_value: &envelope.value,
        })
        .expect("publish denied ingress");
        let delivered = deliver_control_ingress(&ControlIngressDeliverInput {
            state_root: &root,
            topic: DEFAULT_CONTROL_INGRESS_TOPIC,
            envelope_ref: &envelope.envelope_ref,
        })
        .expect("deliver denied ingress");
        assert!(!delivered.has_enqueued);
        let receipt_text = crate::preserves_rail::to_text(&delivered.ingress_receipt_value).expect("receipt text");
        assert!(receipt_text.contains("authority refs missing"));
        assert!(next_pending_control_request(&root).expect("pending request scan").is_none());
    }

    struct PeerDelivery<'a> {
        root: &'a Path,
        request_value: &'a IoValue,
        from_peer: &'a str,
        to_node: &'a str,
        peer_bootstrap_refs: &'a [String],
        authority_refs: &'a [String],
        policy_refs: &'a [String],
        resource_refs: &'a [String],
        is_expected_enqueued: bool,
        expected_note: Option<&'a str>,
    }

    fn assert_peer_delivery(input: PeerDelivery<'_>) {
        let envelope = control_live_ingress_envelope(&ControlIngressEnvelopeInput {
            request_value: input.request_value,
            from_peer: input.from_peer,
            to_node: input.to_node,
            topic: DEFAULT_CONTROL_INGRESS_TOPIC,
            sequence: 1,
            peer_bootstrap_refs: input.peer_bootstrap_refs,
            authority_refs: input.authority_refs,
            policy_refs: input.policy_refs,
            resource_refs: input.resource_refs,
            evidence_refs: &[],
        })
        .expect("live envelope");
        publish_control_ingress(&ControlIngressPublishInput {
            state_root: input.root,
            envelope_value: &envelope.value,
        })
        .expect("publish envelope");
        let delivered = deliver_control_ingress(&ControlIngressDeliverInput {
            state_root: input.root,
            topic: DEFAULT_CONTROL_INGRESS_TOPIC,
            envelope_ref: &envelope.envelope_ref,
        })
        .expect("deliver envelope");
        assert_eq!(delivered.has_enqueued, input.is_expected_enqueued);
        if let Some(expected_note) = input.expected_note {
            let receipt_text = crate::preserves_rail::to_text(&delivered.ingress_receipt_value).expect("receipt text");
            assert!(receipt_text.contains(expected_note));
        }
    }

    #[test]
    fn control_live_peer_ticket_admission_gates_bootstrap() {
        let root = temp_dir("node-control-live-peer-ticket");
        init_local(&InitInput {
            state_root: &root,
            node_id: "node:live-ticket",
        })
        .expect("init node");
        run_local(&RunInput { state_root: &root }).expect("run node");
        let policy_refs = vec![local_ref("node-control-policy", "live-ticket").expect("policy ref")];
        let resource_refs = vec![local_ref("node-control-resource", "live-ticket").expect("resource ref")];
        let peer_bootstrap_refs =
            test_live_peer_bootstrap_refs(&root, "peer:ticket", DEFAULT_CONTROL_INGRESS_TOPIC, &policy_refs)
                .expect("peer admission ref");
        let authority_refs = test_live_authority_refs(&root, "peer:ticket", "node:live-ticket", "status", &policy_refs)
            .expect("authority grant ref");
        let request_value =
            crate::node_runtime::control_request_value(&crate::node_runtime::ControlRequestValueInput {
                operation: "status",
                target_ref: None,
                payload_ref: None,
                authority_refs: &authority_refs,
                policy_refs: &policy_refs,
                resource_refs: &resource_refs,
                evidence_refs: &[],
            })
            .expect("status request");
        assert_peer_delivery(PeerDelivery {
            root: &root,
            request_value: &request_value,
            from_peer: "peer:ticket",
            to_node: "node:live-ticket",
            peer_bootstrap_refs: &peer_bootstrap_refs,
            authority_refs: &authority_refs,
            policy_refs: &policy_refs,
            resource_refs: &resource_refs,
            is_expected_enqueued: true,
            expected_note: None,
        });
        assert_peer_delivery(PeerDelivery {
            root: &root,
            request_value: &request_value,
            from_peer: "peer:other-ticket",
            to_node: "node:live-ticket",
            peer_bootstrap_refs: &peer_bootstrap_refs,
            authority_refs: &authority_refs,
            policy_refs: &policy_refs,
            resource_refs: &resource_refs,
            is_expected_enqueued: false,
            expected_note: Some("peer peer:ticket does not match peer:other-ticket"),
        });
    }

    struct ImportCase {
        sender: PathBuf,
        policy_refs: Vec<String>,
        ticket: ControlLiveTicket,
        admission: ControlLivePeerAdmission,
    }

    fn import_case() -> ImportCase {
        let receiver = temp_dir("node-control-live-import-receiver");
        let sender = temp_dir("node-control-live-import-sender");
        init_local(&InitInput {
            state_root: &receiver,
            node_id: "node:live-import",
        })
        .expect("init receiver");
        run_local(&RunInput { state_root: &receiver }).expect("run receiver");
        init_local(&InitInput {
            state_root: &sender,
            node_id: "node:live-import-sender",
        })
        .expect("init sender");
        let policy_refs = vec![local_ref("node-control-policy", "live-import").expect("policy ref")];
        let ticket = export_control_live_ticket(&ControlLiveTicketExportInput {
            state_root: &receiver,
            topic: DEFAULT_CONTROL_INGRESS_TOPIC,
            policy_refs: &policy_refs,
            evidence_refs: &[],
        })
        .expect("export ticket");
        let admission = admit_control_live_peer(&ControlLivePeerAdmitInput {
            state_root: &receiver,
            ticket_value: &ticket.value,
            peer_id: "peer:live-import",
            sequence: 1,
            expires_at: Some(4),
            policy_refs: &policy_refs,
            evidence_refs: &[],
        })
        .expect("admit peer");
        ImportCase {
            sender,
            policy_refs,
            ticket,
            admission,
        }
    }

    fn assert_ticket_imports(case: &ImportCase) {
        let imported_ticket = import_control_live_ticket(&ControlLiveTicketImportInput {
            state_root: &case.sender,
            ticket_value: &case.ticket.value,
            peer_admission_value: Some(&case.admission.value),
            expected_node: Some("node:live-import"),
            expected_topic: Some(DEFAULT_CONTROL_INGRESS_TOPIC),
            expected_endpoint: Some(&case.ticket.live_endpoint_id),
            expected_peer: Some("peer:live-import"),
            as_of_sequence: 2,
        })
        .expect("import ticket");
        assert_eq!(imported_ticket.decision, "pass");
        assert_eq!(imported_ticket.imported_refs.len(), 2);
        assert_eq!(
            crate::ledger::artifact_kind(&imported_ticket.receipt_value),
            "node-control-live-ticket-import-receipt"
        );
        read_ledger_artifact(&case.sender, &case.ticket.ticket_ref).expect("ticket imported");
        read_ledger_artifact(&case.sender, &case.admission.admission_ref).expect("admission imported");

        let stale_ticket = import_control_live_ticket(&ControlLiveTicketImportInput {
            state_root: &case.sender,
            ticket_value: &case.ticket.value,
            peer_admission_value: Some(&case.admission.value),
            expected_node: Some("node:live-import"),
            expected_topic: Some(DEFAULT_CONTROL_INGRESS_TOPIC),
            expected_endpoint: Some(&case.ticket.live_endpoint_id),
            expected_peer: Some("peer:live-import"),
            as_of_sequence: 8,
        })
        .expect("stale ticket import receipt");
        assert_eq!(stale_ticket.decision, "deny");
        assert!(stale_ticket.imported_refs.is_empty());
        assert!(stale_ticket.diagnostics.iter().any(|value| value.contains("expired at sequence")));
    }

    fn assert_grant_imports(case: &ImportCase) {
        let operations = vec!["status".to_string()];
        let grant_value = control_authority_grant_value(&ControlAuthorityGrantInput {
            peer_id: "peer:live-import",
            node_id: "node:live-import",
            operations: &operations,
            target_scope: "*",
            resource_scope: "*",
            epoch: 1,
            expires_at: Some(4),
            policy_refs: &case.policy_refs,
            revocation_refs: &[],
            evidence_refs: &[],
        })
        .expect("grant value");
        let imported_grant = import_control_authority_grant_checked(&ControlAuthorityGrantImportInput {
            state_root: &case.sender,
            grant_value: &grant_value,
            expected_peer: Some("peer:live-import"),
            expected_node: Some("node:live-import"),
            expected_operations: &operations,
            expected_target_scope: Some("*"),
            expected_resource_scope: Some("*"),
            as_of_epoch: 2,
        })
        .expect("import grant");
        assert_eq!(imported_grant.decision, "pass");
        assert_eq!(imported_grant.imported_refs.len(), 1);
        assert_eq!(
            crate::ledger::artifact_kind(&imported_grant.receipt_value),
            "node-control-authority-grant-import-receipt"
        );
        read_ledger_artifact(&case.sender, &imported_grant.grant_ref).expect("grant imported");

        let bad_operations = vec!["shutdown".to_string()];
        let denied_grant = import_control_authority_grant_checked(&ControlAuthorityGrantImportInput {
            state_root: &case.sender,
            grant_value: &grant_value,
            expected_peer: Some("peer:live-import"),
            expected_node: Some("node:live-import"),
            expected_operations: &bad_operations,
            expected_target_scope: Some("*"),
            expected_resource_scope: Some("*"),
            as_of_epoch: 2,
        })
        .expect("denied grant import");
        assert_eq!(denied_grant.decision, "deny");
        assert!(denied_grant.imported_refs.is_empty());
        assert!(denied_grant.diagnostics.iter().any(|value| value.contains("operation shutdown")));
    }

    #[test]
    fn control_live_ticket_and_authority_import_receipts_gate_bindings() {
        let case = import_case();
        assert_ticket_imports(&case);
        assert_grant_imports(&case);
    }

    struct FlowSeed {
        bundle_sender: PathBuf,
        operations: Vec<String>,
        ticket: ControlLiveTicket,
        admission: ControlLivePeerAdmission,
        authority_value: IoValue,
        receipt_values: Vec<IoValue>,
        authority_import_ref: String,
    }

    struct FlowCase {
        bundle_sender: PathBuf,
        operations: Vec<String>,
        ticket: ControlLiveTicket,
        admission: ControlLivePeerAdmission,
        authority_import_ref: String,
        exported: ControlLiveWorkflowBundleExport,
        verified: ControlLiveWorkflowBundleVerify,
        gated: ControlLiveWorkflowBundleGate,
    }

    struct FlowImports {
        receipt_values: Vec<IoValue>,
        authority_import_ref: String,
    }

    struct FlowApplyInput<'a> {
        state_root: &'a Path,
        gate_receipt_value: Option<&'a IoValue>,
        request_value: Option<&'a IoValue>,
        is_send_requested: bool,
        sequence: u64,
        expect_message: &'a str,
    }

    fn init_flow_root(label: &str, node_id: &str) -> PathBuf {
        let root = temp_dir(label);
        init_local(&InitInput {
            state_root: &root,
            node_id,
        })
        .expect("init flow root");
        root
    }

    fn flow_roots() -> (PathBuf, PathBuf, PathBuf) {
        let receiver = init_flow_root("node-control-live-workflow-bundle-receiver", "node:live-bundle");
        run_local(&RunInput { state_root: &receiver }).expect("run receiver");
        let staging = init_flow_root("node-control-live-workflow-bundle-staging", "node:live-bundle-staging");
        let sender = init_flow_root("node-control-live-workflow-bundle-sender", "node:live-bundle-sender");
        (receiver, staging, sender)
    }

    fn flow_ticket(root: &Path, policy_refs: &[String]) -> ControlLiveTicket {
        export_control_live_ticket(&ControlLiveTicketExportInput {
            state_root: root,
            topic: DEFAULT_CONTROL_INGRESS_TOPIC,
            policy_refs,
            evidence_refs: &[],
        })
        .expect("export ticket")
    }

    fn flow_admission(root: &Path, ticket: &ControlLiveTicket, policy_refs: &[String]) -> ControlLivePeerAdmission {
        admit_control_live_peer(&ControlLivePeerAdmitInput {
            state_root: root,
            ticket_value: &ticket.value,
            peer_id: "peer:live-bundle",
            sequence: 1,
            expires_at: Some(8),
            policy_refs,
            evidence_refs: &[],
        })
        .expect("admit peer")
    }

    fn flow_authority_value(policy_refs: &[String], operations: &[String]) -> IoValue {
        control_authority_grant_value(&ControlAuthorityGrantInput {
            peer_id: "peer:live-bundle",
            node_id: "node:live-bundle",
            operations,
            target_scope: "*",
            resource_scope: "*",
            epoch: 1,
            expires_at: Some(8),
            policy_refs,
            revocation_refs: &[],
            evidence_refs: &[],
        })
        .expect("authority grant value")
    }

    fn flow_imports(
        staging: &Path,
        ticket: &ControlLiveTicket,
        admission: &ControlLivePeerAdmission,
        authority_value: &IoValue,
        operations: &[String],
    ) -> FlowImports {
        let ticket_import = import_control_live_ticket(&ControlLiveTicketImportInput {
            state_root: staging,
            ticket_value: &ticket.value,
            peer_admission_value: Some(&admission.value),
            expected_node: Some("node:live-bundle"),
            expected_topic: Some(DEFAULT_CONTROL_INGRESS_TOPIC),
            expected_endpoint: Some(&ticket.live_endpoint_id),
            expected_peer: Some("peer:live-bundle"),
            as_of_sequence: 2,
        })
        .expect("ticket import");
        let authority_import = import_control_authority_grant_checked(&ControlAuthorityGrantImportInput {
            state_root: staging,
            grant_value: authority_value,
            expected_peer: Some("peer:live-bundle"),
            expected_node: Some("node:live-bundle"),
            expected_operations: operations,
            expected_target_scope: Some("*"),
            expected_resource_scope: Some("*"),
            as_of_epoch: 2,
        })
        .expect("authority import");
        FlowImports {
            receipt_values: vec![ticket_import.receipt_value, authority_import.receipt_value],
            authority_import_ref: authority_import.grant_ref,
        }
    }

    fn flow_seed() -> FlowSeed {
        let (receiver, staging, bundle_sender) = flow_roots();
        let policy_refs = vec![local_ref("node-control-policy", "live-bundle").expect("policy ref")];
        let operations = vec!["status".to_string()];
        let ticket = flow_ticket(&receiver, &policy_refs);
        let admission = flow_admission(&receiver, &ticket, &policy_refs);
        let authority_value = flow_authority_value(&policy_refs, &operations);
        let imports = flow_imports(&staging, &ticket, &admission, &authority_value, &operations);
        FlowSeed {
            bundle_sender,
            operations,
            ticket,
            admission,
            authority_value,
            receipt_values: imports.receipt_values,
            authority_import_ref: imports.authority_import_ref,
        }
    }

    fn export_flow(seed: &FlowSeed) -> ControlLiveWorkflowBundleExport {
        let receipt_values = seed.receipt_values.iter().collect::<Vec<_>>();
        export_control_live_workflow_bundle(&ControlLiveWorkflowBundleExportInput {
            receiver_ticket_value: &seed.ticket.value,
            peer_admission_value: &seed.admission.value,
            authority_grant_value: &seed.authority_value,
            receipt_values: &receipt_values,
        })
        .expect("export bundle")
    }

    fn verify_flow(seed: &FlowSeed, exported: &ControlLiveWorkflowBundleExport) -> ControlLiveWorkflowBundleVerify {
        verify_control_live_workflow_bundle(&ControlLiveWorkflowBundleVerifyInput {
            bundle_value: &exported.bundle.bundle_value,
            expected_node: Some("node:live-bundle"),
            expected_topic: Some(DEFAULT_CONTROL_INGRESS_TOPIC),
            expected_endpoint: Some(&seed.ticket.live_endpoint_id),
            expected_peer: Some("peer:live-bundle"),
            expected_operations: &seed.operations,
            expected_target_scope: Some("*"),
            expected_resource_scope: Some("*"),
            as_of_sequence: 2,
            as_of_epoch: 2,
        })
        .expect("verify bundle")
    }

    fn gate_flow(
        seed: &FlowSeed,
        exported: &ControlLiveWorkflowBundleExport,
        verified: &ControlLiveWorkflowBundleVerify,
    ) -> ControlLiveWorkflowBundleGate {
        gate_control_live_workflow_bundle(&ControlLiveWorkflowBundleGateInput {
            bundle_value: &exported.bundle.bundle_value,
            verify_receipt_value: Some(&verified.receipt_value),
            require_verify_receipt: true,
            expected_node: Some("node:live-bundle"),
            expected_topic: Some(DEFAULT_CONTROL_INGRESS_TOPIC),
            expected_endpoint: Some(&seed.ticket.live_endpoint_id),
            expected_peer: Some("peer:live-bundle"),
            expected_operations: &seed.operations,
            expected_target_scope: Some("*"),
            expected_resource_scope: Some("*"),
            as_of_sequence: 2,
            as_of_epoch: 2,
        })
        .expect("gate bundle")
    }

    fn flow_case() -> FlowCase {
        let seed = flow_seed();
        let exported = export_flow(&seed);
        assert_eq!(exported.decision, "pass");
        assert_eq!(crate::ledger::artifact_kind(&exported.bundle.bundle_value), "node-control-live-workflow-bundle");
        assert!(parse_control_authority_grant(&exported.bundle.bundle_value).is_err());
        let verified = verify_flow(&seed, &exported);
        assert_eq!(verified.decision, "pass");
        assert_eq!(
            crate::ledger::artifact_kind(&verified.receipt_value),
            "node-control-live-workflow-bundle-verify-receipt"
        );
        assert!(parse_control_authority_grant(&verified.receipt_value).is_err());
        assert!(
            crate::preserves_rail::to_text(&verified.receipt_value)
                .expect("verify receipt text")
                .contains("verify-receipt-is-not-authority")
        );
        let gated = gate_flow(&seed, &exported, &verified);
        assert_eq!(gated.decision, "pass");
        assert_eq!(
            crate::ledger::artifact_kind(&gated.receipt_value),
            "node-control-live-workflow-bundle-gate-receipt"
        );
        assert_eq!(gated.verify_receipt_ref.as_deref(), Some(verified.receipt_ref.as_str()));
        assert!(parse_control_authority_grant(&gated.receipt_value).is_err());
        assert!(
            crate::preserves_rail::to_text(&gated.receipt_value)
                .expect("gate receipt text")
                .contains("gate-receipt-is-not-authority")
        );
        FlowCase {
            bundle_sender: seed.bundle_sender,
            operations: seed.operations,
            ticket: seed.ticket,
            admission: seed.admission,
            authority_import_ref: seed.authority_import_ref,
            exported,
            verified,
            gated,
        }
    }

    fn assert_flow_gate_denials(case: &FlowCase) {
        let missing_verify_gate = gate_control_live_workflow_bundle(&ControlLiveWorkflowBundleGateInput {
            bundle_value: &case.exported.bundle.bundle_value,
            verify_receipt_value: None,
            require_verify_receipt: true,
            expected_node: Some("node:live-bundle"),
            expected_topic: Some(DEFAULT_CONTROL_INGRESS_TOPIC),
            expected_endpoint: Some(&case.ticket.live_endpoint_id),
            expected_peer: Some("peer:live-bundle"),
            expected_operations: &case.operations,
            expected_target_scope: Some("*"),
            expected_resource_scope: Some("*"),
            as_of_sequence: 2,
            as_of_epoch: 2,
        })
        .expect("missing verify gate receipt");
        assert_eq!(missing_verify_gate.decision, "deny");
        assert!(
            missing_verify_gate
                .diagnostics
                .iter()
                .any(|value| value.contains("requires a current verify receipt"))
        );
        let malformed_verify_gate = gate_control_live_workflow_bundle(&ControlLiveWorkflowBundleGateInput {
            bundle_value: &case.exported.bundle.bundle_value,
            verify_receipt_value: Some(&case.exported.bundle.bundle_value),
            require_verify_receipt: true,
            expected_node: Some("node:live-bundle"),
            expected_topic: Some(DEFAULT_CONTROL_INGRESS_TOPIC),
            expected_endpoint: Some(&case.ticket.live_endpoint_id),
            expected_peer: Some("peer:live-bundle"),
            expected_operations: &case.operations,
            expected_target_scope: Some("*"),
            expected_resource_scope: Some("*"),
            as_of_sequence: 2,
            as_of_epoch: 2,
        })
        .expect("malformed verify gate receipt");
        assert_eq!(malformed_verify_gate.decision, "deny");
        assert!(malformed_verify_gate.diagnostics.iter().any(|value| value.contains("verify receipt parse failed")));
    }

    fn run_flow_apply(
        runtime: &tokio::runtime::Runtime,
        case: &FlowCase,
        input: FlowApplyInput<'_>,
    ) -> ControlLiveWorkflowBundleApply {
        runtime
            .block_on(apply_control_live_workflow_bundle(&ControlLiveWorkflowBundleApplyInput {
                state_root: input.state_root,
                bundle_value: &case.exported.bundle.bundle_value,
                gate_receipt_value: input.gate_receipt_value,
                is_gate_receipt_required: true,
                request_value: input.request_value,
                should_send: input.is_send_requested,
                from_peer: None,
                sequence: input.sequence,
                expected_operation_ref: None,
                expected_node: Some("node:live-bundle"),
                expected_topic: Some(DEFAULT_CONTROL_INGRESS_TOPIC),
                expected_endpoint: Some(&case.ticket.live_endpoint_id),
                expected_peer: Some("peer:live-bundle"),
                expected_operations: &case.operations,
                expected_target_scope: Some("*"),
                expected_resource_scope: Some("*"),
                as_of_sequence: 2,
                as_of_epoch: 2,
                peer_bootstrap_refs: &[],
                authority_refs: &[],
                policy_refs: &[],
                resource_refs: &[],
                evidence_refs: &[],
                max_attempts: DEFAULT_CONTROL_LIVE_SEND_ATTEMPTS,
                join_timeout_ms: 10_000,
            }))
            .expect(input.expect_message)
    }

    fn assert_flow_apply_pass(case: &FlowCase, runtime: &tokio::runtime::Runtime) -> ControlLiveWorkflowBundleApply {
        let applied = run_flow_apply(runtime, case, FlowApplyInput {
            state_root: &case.bundle_sender,
            gate_receipt_value: Some(&case.gated.receipt_value),
            request_value: None,
            is_send_requested: false,
            sequence: 1,
            expect_message: "apply bundle",
        });
        assert_eq!(applied.decision, "pass");
        assert_eq!(
            crate::ledger::artifact_kind(&applied.receipt_value),
            "node-control-live-workflow-bundle-apply-receipt"
        );
        assert!(applied.import_receipt_ref.is_some());
        assert!(applied.imported_refs.iter().any(|reference| reference == &case.exported.bundle.bundle_ref));
        assert!(parse_control_authority_grant(&applied.receipt_value).is_err());
        assert!(
            crate::preserves_rail::to_text(&applied.receipt_value)
                .expect("apply receipt text")
                .contains("apply-receipt-is-not-authority")
        );
        read_ledger_artifact(&case.bundle_sender, &case.exported.bundle.bundle_ref).expect("apply imported bundle");
        applied
    }

    fn assert_flow_missing_gate(case: &FlowCase, runtime: &tokio::runtime::Runtime) {
        let root = init_flow_root(
            "node-control-live-workflow-bundle-apply-missing-gate",
            "node:live-bundle-apply-missing-gate",
        );
        let receipt = run_flow_apply(runtime, case, FlowApplyInput {
            state_root: &root,
            gate_receipt_value: None,
            request_value: None,
            is_send_requested: false,
            sequence: 1,
            expect_message: "missing gate apply receipt",
        });
        assert_eq!(receipt.decision, "deny");
        assert!(receipt.imported_refs.is_empty());
        assert!(receipt.diagnostics.iter().any(|value| value.contains("requires a current gate receipt")));
        assert!(read_ledger_artifact(&root, &case.exported.bundle.bundle_ref).is_err());
    }

    fn assert_flow_send_denial(case: &FlowCase, runtime: &tokio::runtime::Runtime) {
        let root = init_flow_root("node-control-live-workflow-bundle-apply-send", "node:live-bundle-apply-send");
        let authority_refs = vec![case.exported.bundle.authority_grant_ref.clone()];
        let request_value =
            crate::node_runtime::control_request_value(&crate::node_runtime::ControlRequestValueInput {
                operation: "status",
                target_ref: None,
                payload_ref: None,
                authority_refs: &authority_refs,
                policy_refs: &[],
                resource_refs: &[],
                evidence_refs: &[],
            })
            .expect("apply send request");
        let receipt = run_flow_apply(runtime, case, FlowApplyInput {
            state_root: &root,
            gate_receipt_value: Some(&case.gated.receipt_value),
            request_value: Some(&request_value),
            is_send_requested: true,
            sequence: 7,
            expect_message: "apply send receipt",
        });
        assert_eq!(receipt.decision, "deny");
        assert!(receipt.import_receipt_ref.is_some());
        assert!(receipt.send_receipt_ref.is_some());
        assert!(receipt.diagnostics.iter().any(|value| value.contains("no endpoint addresses")));
        assert!(receipt.send_receipt_value.is_some());
    }

    fn assert_flow_import_pass(case: &FlowCase) {
        let imported = import_control_live_workflow_bundle(&ControlLiveWorkflowBundleImportInput {
            state_root: &case.bundle_sender,
            bundle_value: &case.exported.bundle.bundle_value,
            expected_node: Some("node:live-bundle"),
            expected_topic: Some(DEFAULT_CONTROL_INGRESS_TOPIC),
            expected_endpoint: Some(&case.ticket.live_endpoint_id),
            expected_peer: Some("peer:live-bundle"),
            expected_operations: &case.operations,
            expected_target_scope: Some("*"),
            expected_resource_scope: Some("*"),
            as_of_sequence: 2,
            as_of_epoch: 2,
        })
        .expect("import bundle");
        assert_eq!(imported.decision, "pass");
        assert!(imported.imported_refs.iter().any(|reference| reference == &case.exported.bundle.bundle_ref));
        read_ledger_artifact(&case.bundle_sender, &case.ticket.ticket_ref).expect("bundle imported ticket");
        read_ledger_artifact(&case.bundle_sender, &case.admission.admission_ref).expect("bundle imported admission");
        read_ledger_artifact(&case.bundle_sender, &case.authority_import_ref).expect("bundle imported authority");
        assert!(parse_control_authority_grant(&imported.receipt_value).is_err());
        assert!(
            crate::preserves_rail::to_text(&imported.receipt_value)
                .expect("import receipt text")
                .contains("bundle-import-is-not-authority")
        );
    }

    fn assert_flow_wrong_topic(case: &FlowCase) -> ControlLiveWorkflowBundleGate {
        let root = init_flow_root("node-control-live-workflow-bundle-wrong-topic", "node:live-bundle-wrong-topic");
        let wrong_topic = import_control_live_workflow_bundle(&ControlLiveWorkflowBundleImportInput {
            state_root: &root,
            bundle_value: &case.exported.bundle.bundle_value,
            expected_node: Some("node:live-bundle"),
            expected_topic: Some("wrong-topic"),
            expected_endpoint: Some(&case.ticket.live_endpoint_id),
            expected_peer: Some("peer:live-bundle"),
            expected_operations: &case.operations,
            expected_target_scope: Some("*"),
            expected_resource_scope: Some("*"),
            as_of_sequence: 2,
            as_of_epoch: 2,
        })
        .expect("wrong topic receipt");
        assert_eq!(wrong_topic.decision, "deny");
        assert!(wrong_topic.imported_refs.is_empty());
        assert!(wrong_topic.diagnostics.iter().any(|value| value.contains("wrong-topic")));
        assert!(read_ledger_artifact(&root, &case.exported.bundle.bundle_ref).is_err());
        let wrong_verify = verify_control_live_workflow_bundle(&ControlLiveWorkflowBundleVerifyInput {
            bundle_value: &case.exported.bundle.bundle_value,
            expected_node: Some("node:live-bundle"),
            expected_topic: Some("wrong-topic"),
            expected_endpoint: Some(&case.ticket.live_endpoint_id),
            expected_peer: Some("peer:live-bundle"),
            expected_operations: &case.operations,
            expected_target_scope: Some("*"),
            expected_resource_scope: Some("*"),
            as_of_sequence: 2,
            as_of_epoch: 2,
        })
        .expect("wrong topic verify receipt");
        assert_eq!(wrong_verify.decision, "deny");
        assert!(wrong_verify.diagnostics.iter().any(|value| value.contains("wrong-topic")));
        let stale_gate = gate_control_live_workflow_bundle(&ControlLiveWorkflowBundleGateInput {
            bundle_value: &case.exported.bundle.bundle_value,
            verify_receipt_value: Some(&wrong_verify.receipt_value),
            require_verify_receipt: true,
            expected_node: Some("node:live-bundle"),
            expected_topic: Some(DEFAULT_CONTROL_INGRESS_TOPIC),
            expected_endpoint: Some(&case.ticket.live_endpoint_id),
            expected_peer: Some("peer:live-bundle"),
            expected_operations: &case.operations,
            expected_target_scope: Some("*"),
            expected_resource_scope: Some("*"),
            as_of_sequence: 2,
            as_of_epoch: 2,
        })
        .expect("stale verify gate receipt");
        assert_eq!(stale_gate.decision, "deny");
        assert!(stale_gate.diagnostics.iter().any(|value| value.contains("does not match recomputed")));
        stale_gate
    }

    fn assert_flow_stale_gate(
        case: &FlowCase,
        runtime: &tokio::runtime::Runtime,
        stale_gate: &ControlLiveWorkflowBundleGate,
    ) {
        let root =
            init_flow_root("node-control-live-workflow-bundle-apply-stale-gate", "node:live-bundle-apply-stale-gate");
        let receipt = run_flow_apply(runtime, case, FlowApplyInput {
            state_root: &root,
            gate_receipt_value: Some(&stale_gate.receipt_value),
            request_value: None,
            is_send_requested: false,
            sequence: 1,
            expect_message: "stale gate apply receipt",
        });
        assert_eq!(receipt.decision, "deny");
        assert!(receipt.imported_refs.is_empty());
        assert!(receipt.diagnostics.iter().any(|value| value.contains("decision deny")));
        assert!(read_ledger_artifact(&root, &case.exported.bundle.bundle_ref).is_err());
    }

    fn assert_flow_wrong_peer(case: &FlowCase) {
        let root = init_flow_root("node-control-live-workflow-bundle-wrong-peer", "node:live-bundle-wrong-peer");
        let wrong_peer = import_control_live_workflow_bundle(&ControlLiveWorkflowBundleImportInput {
            state_root: &root,
            bundle_value: &case.exported.bundle.bundle_value,
            expected_node: Some("node:live-bundle"),
            expected_topic: Some(DEFAULT_CONTROL_INGRESS_TOPIC),
            expected_endpoint: Some(&case.ticket.live_endpoint_id),
            expected_peer: Some("peer:other-live-bundle"),
            expected_operations: &case.operations,
            expected_target_scope: Some("*"),
            expected_resource_scope: Some("*"),
            as_of_sequence: 2,
            as_of_epoch: 2,
        })
        .expect("wrong peer receipt");
        assert_eq!(wrong_peer.decision, "deny");
        assert!(wrong_peer.imported_refs.is_empty());
        assert!(wrong_peer.diagnostics.iter().any(|value| value.contains("peer:other-live-bundle")));
        let wrong_verify = verify_control_live_workflow_bundle(&ControlLiveWorkflowBundleVerifyInput {
            bundle_value: &case.exported.bundle.bundle_value,
            expected_node: Some("node:live-bundle"),
            expected_topic: Some(DEFAULT_CONTROL_INGRESS_TOPIC),
            expected_endpoint: Some(&case.ticket.live_endpoint_id),
            expected_peer: Some("peer:other-live-bundle"),
            expected_operations: &case.operations,
            expected_target_scope: Some("*"),
            expected_resource_scope: Some("*"),
            as_of_sequence: 2,
            as_of_epoch: 2,
        })
        .expect("wrong peer verify receipt");
        assert_eq!(wrong_verify.decision, "deny");
        assert!(wrong_verify.diagnostics.iter().any(|value| value.contains("peer:other-live-bundle")));
    }

    fn assert_flow_wrong_operation(case: &FlowCase) {
        let root =
            init_flow_root("node-control-live-workflow-bundle-wrong-operation", "node:live-bundle-wrong-operation");
        let wrong_operations = vec!["shutdown".to_string()];
        let wrong_operation = import_control_live_workflow_bundle(&ControlLiveWorkflowBundleImportInput {
            state_root: &root,
            bundle_value: &case.exported.bundle.bundle_value,
            expected_node: Some("node:live-bundle"),
            expected_topic: Some(DEFAULT_CONTROL_INGRESS_TOPIC),
            expected_endpoint: Some(&case.ticket.live_endpoint_id),
            expected_peer: Some("peer:live-bundle"),
            expected_operations: &wrong_operations,
            expected_target_scope: Some("*"),
            expected_resource_scope: Some("*"),
            as_of_sequence: 2,
            as_of_epoch: 2,
        })
        .expect("wrong operation receipt");
        assert_eq!(wrong_operation.decision, "deny");
        assert!(wrong_operation.imported_refs.is_empty());
        assert!(wrong_operation.diagnostics.iter().any(|value| value.contains("operation shutdown")));
        let wrong_verify = verify_control_live_workflow_bundle(&ControlLiveWorkflowBundleVerifyInput {
            bundle_value: &case.exported.bundle.bundle_value,
            expected_node: Some("node:live-bundle"),
            expected_topic: Some(DEFAULT_CONTROL_INGRESS_TOPIC),
            expected_endpoint: Some(&case.ticket.live_endpoint_id),
            expected_peer: Some("peer:live-bundle"),
            expected_operations: &wrong_operations,
            expected_target_scope: Some("*"),
            expected_resource_scope: Some("*"),
            as_of_sequence: 2,
            as_of_epoch: 2,
        })
        .expect("wrong operation verify receipt");
        assert_eq!(wrong_verify.decision, "deny");
        assert!(wrong_verify.diagnostics.iter().any(|value| value.contains("operation shutdown")));
    }

    fn assert_flow_wrong_grant(case: &FlowCase) {
        let wrong_grant_ref = local_ref("authority-grant", "wrong-live-bundle").expect("wrong grant ref");
        let wrong_bundle = crate::preserves_rail::record("node-control-live-workflow-bundle-v1", vec![
            crate::preserves_rail::string(crate::preserves_rail::NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_SCHEMA),
            crate::preserves_rail::record("ticket", vec![case.exported.bundle.ticket_value.clone()]),
            crate::preserves_rail::record("peer-admission", vec![case.exported.bundle.peer_admission_value.clone()]),
            crate::preserves_rail::record("authority-grant", vec![case.exported.bundle.authority_grant_value.clone()]),
            crate::preserves_rail::record("receipts", vec![crate::preserves_rail::sequence(
                case.exported.bundle.receipt_values.clone(),
            )]),
            crate::preserves_rail::record("ticket-ref", vec![crate::preserves_rail::string(
                &case.exported.bundle.ticket_ref,
            )]),
            crate::preserves_rail::record("peer-admission-ref", vec![crate::preserves_rail::string(
                &case.exported.bundle.peer_admission_ref,
            )]),
            crate::preserves_rail::record("authority-grant-ref", vec![crate::preserves_rail::string(&wrong_grant_ref)]),
            crate::preserves_rail::record("receipt-refs", vec![crate::preserves_rail::sequence(
                case.exported.bundle.receipt_refs.iter().map(crate::preserves_rail::string).collect(),
            )]),
            crate::preserves_rail::record("checks", vec![crate::preserves_rail::sequence(Vec::<IoValue>::new())]),
        ]);
        let wrong_verify = verify_control_live_workflow_bundle(&ControlLiveWorkflowBundleVerifyInput {
            bundle_value: &wrong_bundle,
            expected_node: Some("node:live-bundle"),
            expected_topic: Some(DEFAULT_CONTROL_INGRESS_TOPIC),
            expected_endpoint: Some(&case.ticket.live_endpoint_id),
            expected_peer: Some("peer:live-bundle"),
            expected_operations: &case.operations,
            expected_target_scope: Some("*"),
            expected_resource_scope: Some("*"),
            as_of_sequence: 2,
            as_of_epoch: 2,
        })
        .expect("wrong grant verify receipt");
        assert_eq!(wrong_verify.decision, "deny");
        assert!(wrong_verify.diagnostics.iter().any(|value| value.contains("authority grant ref mismatch")));
    }

    fn assert_ref_not_grant(root: &Path, authority_ref: &str, sequence: u64) {
        let authority_refs = vec![authority_ref.to_string()];
        let request_value =
            crate::node_runtime::control_request_value(&crate::node_runtime::ControlRequestValueInput {
                operation: "status",
                target_ref: None,
                payload_ref: None,
                authority_refs: &authority_refs,
                policy_refs: &[],
                resource_refs: &[],
                evidence_refs: &[],
            })
            .expect("authority request");
        let envelope = control_live_ingress_envelope(&ControlIngressEnvelopeInput {
            request_value: &request_value,
            from_peer: "peer:live-bundle",
            to_node: "node:live-bundle",
            topic: DEFAULT_CONTROL_INGRESS_TOPIC,
            sequence,
            peer_bootstrap_refs: &[],
            authority_refs: &authority_refs,
            policy_refs: &[],
            resource_refs: &[],
            evidence_refs: &[],
        })
        .expect("authority envelope");
        let diagnostics = live_send_authority_grant_diagnostics(root, &envelope).expect("authority diagnostics");
        assert!(diagnostics.iter().any(|value| value.contains("is not a grant")));
        assert!(diagnostics.iter().any(|value| value.contains("authority delegation missing admitted grant")));
    }

    fn assert_flow_receipts_not_grants(case: &FlowCase, applied: &ControlLiveWorkflowBundleApply) {
        import_artifact(&case.bundle_sender, &case.verified.receipt_value).expect("import verify receipt");
        assert_ref_not_grant(&case.bundle_sender, &case.verified.receipt_ref, 3);
        import_artifact(&case.bundle_sender, &case.gated.receipt_value).expect("import gate receipt");
        assert_ref_not_grant(&case.bundle_sender, &case.gated.receipt_ref, 4);
        assert_ref_not_grant(&case.bundle_sender, &applied.receipt_ref, 5);
    }

    fn assert_flow_malformed(case: &FlowCase) {
        let root = init_flow_root("node-control-live-workflow-bundle-malformed", "node:live-bundle-malformed");
        let malformed =
            crate::preserves_rail::record("node-control-live-workflow-bundle-v1", vec![crate::preserves_rail::string(
                crate::preserves_rail::NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_SCHEMA,
            )]);
        assert!(
            import_control_live_workflow_bundle(&ControlLiveWorkflowBundleImportInput {
                state_root: &root,
                bundle_value: &malformed,
                expected_node: Some("node:live-bundle"),
                expected_topic: Some(DEFAULT_CONTROL_INGRESS_TOPIC),
                expected_endpoint: Some(&case.ticket.live_endpoint_id),
                expected_peer: Some("peer:live-bundle"),
                expected_operations: &case.operations,
                expected_target_scope: Some("*"),
                expected_resource_scope: Some("*"),
                as_of_sequence: 2,
                as_of_epoch: 2,
            })
            .is_err()
        );
        let malformed_verify = verify_control_live_workflow_bundle(&ControlLiveWorkflowBundleVerifyInput {
            bundle_value: &malformed,
            expected_node: Some("node:live-bundle"),
            expected_topic: Some(DEFAULT_CONTROL_INGRESS_TOPIC),
            expected_endpoint: Some(&case.ticket.live_endpoint_id),
            expected_peer: Some("peer:live-bundle"),
            expected_operations: &case.operations,
            expected_target_scope: Some("*"),
            expected_resource_scope: Some("*"),
            as_of_sequence: 2,
            as_of_epoch: 2,
        })
        .expect("malformed verify receipt");
        assert_eq!(malformed_verify.decision, "deny");
        assert!(malformed_verify.diagnostics.iter().any(|value| value.contains("parse failed")));
        let malformed_gate = gate_control_live_workflow_bundle(&ControlLiveWorkflowBundleGateInput {
            bundle_value: &malformed,
            verify_receipt_value: Some(&malformed_verify.receipt_value),
            require_verify_receipt: true,
            expected_node: Some("node:live-bundle"),
            expected_topic: Some(DEFAULT_CONTROL_INGRESS_TOPIC),
            expected_endpoint: Some(&case.ticket.live_endpoint_id),
            expected_peer: Some("peer:live-bundle"),
            expected_operations: &case.operations,
            expected_target_scope: Some("*"),
            expected_resource_scope: Some("*"),
            as_of_sequence: 2,
            as_of_epoch: 2,
        })
        .expect("malformed gate receipt");
        assert_eq!(malformed_gate.decision, "deny");
        assert!(malformed_gate.diagnostics.iter().any(|value| value.contains("parse failed")));
    }

    #[test]
    fn control_live_workflow_bundle_import_export_gates_bindings() {
        let case = flow_case();
        assert_flow_gate_denials(&case);
        let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().expect("apply runtime");
        let applied = assert_flow_apply_pass(&case, &runtime);
        assert_flow_missing_gate(&case, &runtime);
        assert_flow_send_denial(&case, &runtime);
        assert_flow_import_pass(&case);
        let stale_gate = assert_flow_wrong_topic(&case);
        assert_flow_stale_gate(&case, &runtime, &stale_gate);
        assert_flow_wrong_peer(&case);
        assert_flow_wrong_operation(&case);
        assert_flow_wrong_grant(&case);
        assert_flow_receipts_not_grants(&case, &applied);
        assert_flow_malformed(&case);
    }

    fn assert_sent(sent: &ControlLiveSend) {
        assert_eq!(crate::ledger::artifact_kind(&sent.send_receipt_value), "node-control-live-send-receipt");
        assert!(sent.transport_receipt_ref.is_some());
        assert_eq!(
            sent.operation_ref,
            parse_control_ingress_envelope(&sent.envelope_value).expect("envelope").operation_ref
        );
    }

    fn assert_duplicate(first: &ControlLiveSend, duplicate: &ControlLiveSend) {
        assert_eq!(duplicate.send_receipt_ref, first.send_receipt_ref);
        assert!(duplicate.duplicate_receipt_ref.is_some());
        assert_eq!(
            crate::ledger::artifact_kind(duplicate.duplicate_receipt_value.as_ref().expect("duplicate receipt")),
            "node-control-live-send-duplicate-receipt"
        );
    }

    struct ServedCase<'a> {
        root: &'a std::path::Path,
        authority_ref: &'a str,
        ticket_value: &'a IoValue,
        admission_value: &'a IoValue,
        send_receipt_value: &'a IoValue,
        listener: &'a ControlLiveServe,
    }

    fn assert_served_case(case: ServedCase<'_>) {
        assert_eq!(case.listener.service.decision, "pass");
        assert_eq!(case.listener.service.processed_request_refs.len(), 1);
        assert_eq!(case.listener.transport_receipt_refs.len(), 1);
        assert!(case.listener.observed_events > 0);
        let authority_value = read_ledger_artifact(case.root, case.authority_ref).expect("authority value");
        let receive_values = case
            .listener
            .transport_receipt_refs
            .iter()
            .map(|reference| read_ledger_artifact(case.root, reference).expect("receive receipt value"))
            .collect::<Vec<_>>();
        let receive_value_refs = receive_values.iter().collect::<Vec<_>>();
        let workflow = control_live_workflow_receipt(&ControlLiveWorkflowInput {
            state_root: Some(case.root),
            receiver_ticket_value: case.ticket_value,
            peer_admission_value: case.admission_value,
            authority_grant_value: &authority_value,
            send_receipt_value: case.send_receipt_value,
            receive_receipt_values: &receive_value_refs,
            listener_receipt_value: Some(&case.listener.listener_receipt_value),
            service_receipt_value: &case.listener.service.service_receipt_value,
        })
        .expect("workflow receipt");
        assert_eq!(workflow.decision, "pass");
        assert_eq!(crate::ledger::artifact_kind(&workflow.receipt_value), "node-control-live-workflow-receipt");
    }

    struct SendMaterial {
        policy_refs: Vec<String>,
        resource_refs: Vec<String>,
        peer_bootstrap_refs: Vec<String>,
        authority_refs: Vec<String>,
        admission: ControlLivePeerAdmission,
        request_value: IoValue,
    }

    fn init_send_case() -> (std::path::PathBuf, crate::node_identity::NodeIdentity) {
        let root = temp_dir("node-control-live-send");
        init_local(&InitInput {
            state_root: &root,
            node_id: "node:live-send",
        })
        .expect("init node");
        run_local(&RunInput { state_root: &root }).expect("run node");
        let identity =
            crate::node_identity::parse_node_identity(&read_preserves(&root.join(IDENTITY_FILE)).expect("identity"))
                .expect("parse identity");
        (root, identity)
    }

    fn send_material(root: &std::path::Path, ticket: &ControlLiveTicket) -> SendMaterial {
        let policy_refs = vec![local_ref("node-control-policy", "live-send").expect("policy ref")];
        let resource_refs = vec![local_ref("node-control-resource", "live-send").expect("resource ref")];
        let admission = admit_control_live_peer(&ControlLivePeerAdmitInput {
            state_root: root,
            ticket_value: &ticket.value,
            peer_id: "peer:external-send",
            sequence: 1,
            expires_at: None,
            policy_refs: &policy_refs,
            evidence_refs: &[],
        })
        .expect("peer admission");
        let peer_bootstrap_refs = vec![admission.admission_ref.clone()];
        let authority_refs =
            test_live_authority_refs(root, "peer:external-send", "node:live-send", "status", &policy_refs)
                .expect("authority grant ref");
        let request_value =
            crate::node_runtime::control_request_value(&crate::node_runtime::ControlRequestValueInput {
                operation: "status",
                target_ref: None,
                payload_ref: None,
                authority_refs: &authority_refs,
                policy_refs: &policy_refs,
                resource_refs: &resource_refs,
                evidence_refs: &[],
            })
            .expect("status request");
        SendMaterial {
            policy_refs,
            resource_refs,
            peer_bootstrap_refs,
            authority_refs,
            admission,
            request_value,
        }
    }

    fn build_send_input<'a>(
        root: &'a std::path::Path,
        ticket: &'a ControlLiveTicket,
        material: &'a SendMaterial,
    ) -> ControlLiveSendInput<'a> {
        ControlLiveSendInput {
            state_root: Some(root),
            request_value: &material.request_value,
            receiver_ticket_value: &ticket.value,
            from_peer: "peer:external-send",
            sequence: 1,
            expected_operation_ref: None,
            expected_receiver_node: None,
            expected_topic: None,
            expected_endpoint: None,
            max_attempts: DEFAULT_CONTROL_LIVE_SEND_ATTEMPTS,
            peer_bootstrap_refs: &material.peer_bootstrap_refs,
            authority_refs: &material.authority_refs,
            policy_refs: &material.policy_refs,
            resource_refs: &material.resource_refs,
            evidence_refs: &[],
            join_timeout_ms: 10_000,
        }
    }

    fn build_listener_input(root: &std::path::Path) -> ControlLiveServeInput<'_> {
        ControlLiveServeInput {
            state_root: root,
            topic: DEFAULT_CONTROL_INGRESS_TOPIC,
            max_events: 8,
            event_timeout_ms: 1_000,
            max_requests_per_tick: 1,
            supervisor_policy_value: None,
        }
    }

    #[test]
    fn send_reaches_bounded_listener() {
        let runtime = tokio::runtime::Builder::new_multi_thread().enable_all().build().expect("runtime");
        runtime.block_on(async {
            let (root, identity) = init_send_case();
            let lookup = iroh::address_lookup::memory::MemoryLookup::new();
            let receiver_endpoint = live_gossip_endpoint(&lookup, Some(stable_live_endpoint_secret(&identity)))
                .await
                .expect("receiver endpoint");
            let receiver_addr = receiver_endpoint.addr();
            let live_ticket =
                live_ticket_for_bound_endpoint(&root, &identity, DEFAULT_CONTROL_INGRESS_TOPIC, &receiver_addr)
                    .expect("live ticket");
            lookup.add_endpoint_info(receiver_addr);
            let receiver_gossip = iroh_gossip::Gossip::builder().spawn(receiver_endpoint.clone());
            let receiver_router = iroh::protocol::Router::builder(receiver_endpoint)
                .accept(iroh_gossip::ALPN, receiver_gossip.clone())
                .spawn();
            let mut receiver_topic = receiver_gossip
                .subscribe(control_live_topic_id(DEFAULT_CONTROL_INGRESS_TOPIC), Vec::new())
                .await
                .expect("receiver subscribe");
            let material = send_material(&root, &live_ticket);
            let send_input = build_send_input(&root, &live_ticket, &material);
            let sent = send_control_live_ingress(&send_input).await.expect("live send");
            assert_sent(&sent);
            let duplicate = send_control_live_ingress(&send_input).await.expect("duplicate live send");
            assert_duplicate(&sent, &duplicate);
            let listener_input = build_listener_input(&root);
            let listener = serve_node_control_live_listener_with_topic(
                &listener_input,
                &mut receiver_topic,
                &identity.node_id,
                &identity.endpoint_id,
                &live_ticket.live_endpoint_id,
            )
            .await
            .expect("listener drain");
            receiver_router.shutdown().await.expect("receiver shutdown");
            assert_served_case(ServedCase {
                root: &root,
                authority_ref: &material.authority_refs[0],
                ticket_value: &live_ticket.value,
                admission_value: &material.admission.value,
                send_receipt_value: &sent.send_receipt_value,
                listener: &listener,
            });
        });
    }

    struct DenyCase<'a> {
        name: &'a str,
        grant_peer: Option<&'a str>,
        grant_node: &'a str,
        grant_operations: &'a [&'a str],
        target_ref: Option<&'a str>,
        target_scope: &'a str,
        resource_scope: &'a str,
        epoch: u64,
        expires_at: Option<u64>,
        is_revoked: bool,
        sequence: u64,
        expected: &'a str,
    }

    struct DenyCaseRefs {
        policy_refs: Vec<String>,
        resource_refs: Vec<String>,
        peer_bootstrap_refs: Vec<String>,
        authority_refs: Vec<String>,
    }

    const DENY_CASES: &[DenyCase<'static>] = &[
        DenyCase {
            name: "unknown-grant",
            grant_peer: None,
            grant_node: "node:live-authority",
            grant_operations: &["status"],
            target_ref: None,
            target_scope: "*",
            resource_scope: "*",
            epoch: 1,
            expires_at: None,
            is_revoked: false,
            sequence: 1,
            expected: "not found",
        },
        DenyCase {
            name: "wrong-peer",
            grant_peer: Some("peer:other"),
            grant_node: "node:live-authority",
            grant_operations: &["status"],
            target_ref: None,
            target_scope: "*",
            resource_scope: "*",
            epoch: 1,
            expires_at: None,
            is_revoked: false,
            sequence: 1,
            expected: "does not match peer:case",
        },
        DenyCase {
            name: "wrong-op",
            grant_peer: Some("peer:case"),
            grant_node: "node:live-authority",
            grant_operations: &["shutdown"],
            target_ref: None,
            target_scope: "*",
            resource_scope: "*",
            epoch: 1,
            expires_at: None,
            is_revoked: false,
            sequence: 1,
            expected: "does not allow operation status",
        },
        DenyCase {
            name: "wrong-target",
            grant_peer: Some("peer:case"),
            grant_node: "node:live-authority",
            grant_operations: &["status"],
            target_ref: Some("blake3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
            target_scope: "blake3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            resource_scope: "*",
            epoch: 1,
            expires_at: None,
            is_revoked: false,
            sequence: 1,
            expected: "target scope",
        },
        DenyCase {
            name: "wrong-resource",
            grant_peer: Some("peer:case"),
            grant_node: "node:live-authority",
            grant_operations: &["status"],
            target_ref: None,
            target_scope: "*",
            resource_scope: "blake3:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
            epoch: 1,
            expires_at: None,
            is_revoked: false,
            sequence: 1,
            expected: "resource scope",
        },
        DenyCase {
            name: "expired",
            grant_peer: Some("peer:case"),
            grant_node: "node:live-authority",
            grant_operations: &["status"],
            target_ref: None,
            target_scope: "*",
            resource_scope: "*",
            epoch: 1,
            expires_at: Some(1),
            is_revoked: false,
            sequence: 2,
            expected: "expired at epoch 1",
        },
        DenyCase {
            name: "revoked",
            grant_peer: Some("peer:case"),
            grant_node: "node:live-authority",
            grant_operations: &["status"],
            target_ref: None,
            target_scope: "*",
            resource_scope: "*",
            epoch: 1,
            expires_at: None,
            is_revoked: true,
            sequence: 1,
            expected: "has revocation refs",
        },
    ];

    fn denied_case_refs(root: &Path, case: &DenyCase<'_>) -> DenyCaseRefs {
        let policy_refs = vec![local_ref("node-control-policy", case.name).expect("policy ref")];
        let resource_refs = vec![local_ref("node-control-resource", case.name).expect("resource ref")];
        let peer_bootstrap_refs =
            test_live_peer_bootstrap_refs(root, "peer:case", DEFAULT_CONTROL_INGRESS_TOPIC, &policy_refs)
                .expect("peer admission ref");
        let authority_refs = if let Some(grant_peer) = case.grant_peer {
            let operations = case.grant_operations.iter().map(|operation| (*operation).to_string()).collect::<Vec<_>>();
            let revocation_refs = if case.is_revoked {
                vec![local_ref("node-control-revocation", case.name).expect("revocation ref")]
            } else {
                Vec::new()
            };
            let grant_value = control_authority_grant_value(&ControlAuthorityGrantInput {
                peer_id: grant_peer,
                node_id: case.grant_node,
                operations: &operations,
                target_scope: case.target_scope,
                resource_scope: case.resource_scope,
                epoch: case.epoch,
                expires_at: case.expires_at,
                policy_refs: &policy_refs,
                revocation_refs: &revocation_refs,
                evidence_refs: &[],
            })
            .expect("authority grant value");
            vec![import_control_authority_grant(root, &grant_value).expect("import authority grant").grant_ref]
        } else {
            vec![local_ref("node-control-authority", case.name).expect("authority ref")]
        };
        DenyCaseRefs {
            policy_refs,
            resource_refs,
            peer_bootstrap_refs,
            authority_refs,
        }
    }

    fn assert_denied_case(case: &DenyCase<'_>) {
        let root = temp_dir(&format!("node-control-live-authority-{}", case.name));
        init_local(&InitInput {
            state_root: &root,
            node_id: "node:live-authority",
        })
        .expect("init node");
        run_local(&RunInput { state_root: &root }).expect("run node");
        let refs = denied_case_refs(&root, case);
        let request_value =
            crate::node_runtime::control_request_value(&crate::node_runtime::ControlRequestValueInput {
                operation: "status",
                target_ref: case.target_ref,
                payload_ref: None,
                authority_refs: &refs.authority_refs,
                policy_refs: &refs.policy_refs,
                resource_refs: &refs.resource_refs,
                evidence_refs: &[],
            })
            .expect("status request");
        let envelope = control_live_ingress_envelope(&ControlIngressEnvelopeInput {
            request_value: &request_value,
            from_peer: "peer:case",
            to_node: "node:live-authority",
            topic: DEFAULT_CONTROL_INGRESS_TOPIC,
            sequence: case.sequence,
            peer_bootstrap_refs: &refs.peer_bootstrap_refs,
            authority_refs: &refs.authority_refs,
            policy_refs: &refs.policy_refs,
            resource_refs: &refs.resource_refs,
            evidence_refs: &[],
        })
        .expect("live envelope");
        publish_control_ingress(&ControlIngressPublishInput {
            state_root: &root,
            envelope_value: &envelope.value,
        })
        .expect("publish live envelope");
        let delivered = deliver_control_ingress(&ControlIngressDeliverInput {
            state_root: &root,
            topic: DEFAULT_CONTROL_INGRESS_TOPIC,
            envelope_ref: &envelope.envelope_ref,
        })
        .expect("deliver live envelope");
        assert!(!delivered.has_enqueued, "{} enqueued", case.name);
        let receipt_text = crate::preserves_rail::to_text(&delivered.ingress_receipt_value).expect("receipt text");
        assert!(receipt_text.contains(case.expected), "{} receipt: {receipt_text}", case.name);
        assert!(next_pending_control_request(&root).expect("pending request scan").is_none());
    }

    #[test]
    fn control_live_authority_delegation_fails_closed() {
        for case in DENY_CASES {
            assert_denied_case(case);
        }
    }

    #[tokio::test]
    async fn control_live_serve_listener_loopback_dispatches_through_service() {
        let root = temp_dir("node-control-live-listener");
        init_local(&InitInput {
            state_root: &root,
            node_id: "node:live-listener",
        })
        .expect("init node");
        run_local(&RunInput { state_root: &root }).expect("run node");
        let policy_refs = vec![local_ref("node-control-policy", "live-listener").expect("policy ref")];
        let authority_refs =
            test_live_authority_refs(&root, "peer:listener", "node:live-listener", "status", &policy_refs)
                .expect("authority grant ref");
        let resource_refs = vec![local_ref("node-control-resource", "live-listener").expect("resource ref")];
        let peer_bootstrap_refs =
            test_live_peer_bootstrap_refs(&root, "peer:listener", DEFAULT_CONTROL_INGRESS_TOPIC, &policy_refs)
                .expect("peer admission ref");
        let request_value =
            crate::node_runtime::control_request_value(&crate::node_runtime::ControlRequestValueInput {
                operation: "status",
                target_ref: None,
                payload_ref: None,
                authority_refs: &authority_refs,
                policy_refs: &policy_refs,
                resource_refs: &resource_refs,
                evidence_refs: &[],
            })
            .expect("status request");

        let loopback = control_live_serve_listener_loopback(&ControlLiveServeLoopbackInput {
            state_root: &root,
            request_value: &request_value,
            from_peer: "peer:listener",
            to_node: "node:live-listener",
            topic: DEFAULT_CONTROL_INGRESS_TOPIC,
            sequence: 1,
            peer_bootstrap_refs: &peer_bootstrap_refs,
            authority_refs: &authority_refs,
            policy_refs: &policy_refs,
            resource_refs: &resource_refs,
            evidence_refs: &[],
            max_requests_per_tick: 1,
        })
        .await
        .expect("live listener loopback");
        assert_eq!(
            crate::ledger::artifact_kind(&loopback.listener.listener_receipt_value),
            "node-control-live-listener-receipt"
        );
        assert_eq!(loopback.listener.service.decision, "pass");
        assert_eq!(loopback.listener.service.processed_request_refs.len(), 1);
        assert_eq!(loopback.listener.transport_receipt_refs.len(), 1);
        assert!(loopback.listener.observed_events >= 1);
    }

    #[tokio::test]
    async fn control_live_iroh_loopback_delivers_to_durable_inbox() {
        let root = temp_dir("node-control-live-iroh");
        init_local(&InitInput {
            state_root: &root,
            node_id: "node:live-ingress",
        })
        .expect("init node");
        run_local(&RunInput { state_root: &root }).expect("run node");
        let policy_refs = vec![local_ref("node-control-policy", "live-ingress").expect("policy ref")];
        let authority_refs = test_live_authority_refs(&root, "peer:live", "node:live-ingress", "status", &policy_refs)
            .expect("authority grant ref");
        let resource_refs = vec![local_ref("node-control-resource", "live-ingress").expect("resource ref")];
        let peer_bootstrap_refs =
            test_live_peer_bootstrap_refs(&root, "peer:live", DEFAULT_CONTROL_INGRESS_TOPIC, &policy_refs)
                .expect("peer admission ref");
        let request_value =
            crate::node_runtime::control_request_value(&crate::node_runtime::ControlRequestValueInput {
                operation: "status",
                target_ref: None,
                payload_ref: None,
                authority_refs: &authority_refs,
                policy_refs: &policy_refs,
                resource_refs: &resource_refs,
                evidence_refs: &[],
            })
            .expect("status request");

        let live = control_live_iroh_loopback(&ControlLiveLoopbackInput {
            state_root: &root,
            request_value: &request_value,
            from_peer: "peer:live",
            to_node: "node:live-ingress",
            topic: DEFAULT_CONTROL_INGRESS_TOPIC,
            sequence: 1,
            peer_bootstrap_refs: &peer_bootstrap_refs,
            authority_refs: &authority_refs,
            policy_refs: &policy_refs,
            resource_refs: &resource_refs,
            evidence_refs: &[],
        })
        .await
        .expect("live loopback");
        assert!(live.has_enqueued);
        assert_eq!(crate::ledger::artifact_kind(&live.publish_receipt_value), "node-control-live-transport-receipt");
        assert_eq!(crate::ledger::artifact_kind(&live.receive_receipt_value), "node-control-live-transport-receipt");

        let served = serve_control(&ControlServeInput {
            state_root: &root,
            topic: DEFAULT_CONTROL_INGRESS_TOPIC,
            max_ticks: 1,
            max_requests_per_tick: 1,
            supervisor_policy_value: None,
        })
        .expect("serve live ingress");
        assert_eq!(served.decision, "pass");
        assert_eq!(served.processed_request_refs.len(), 1);
    }

    #[test]
    fn control_service_delivers_ingress_and_dispatches_through_loop() {
        let root = temp_dir("node-control-service-ingress");
        init_local(&InitInput {
            state_root: &root,
            node_id: "node:service-ingress",
        })
        .expect("init node");
        run_local(&RunInput { state_root: &root }).expect("run node");
        let authority_refs = vec![local_ref("node-control-authority", "service-ingress").expect("authority ref")];
        let policy_refs = vec![local_ref("node-control-policy", "service-ingress").expect("policy ref")];
        let resource_refs = vec![local_ref("node-control-resource", "service-ingress").expect("resource ref")];
        let peer_bootstrap_refs = vec![local_ref("peer-bootstrap", "peer:service").expect("bootstrap ref")];
        let request_value =
            crate::node_runtime::control_request_value(&crate::node_runtime::ControlRequestValueInput {
                operation: "status",
                target_ref: None,
                payload_ref: None,
                authority_refs: &authority_refs,
                policy_refs: &policy_refs,
                resource_refs: &resource_refs,
                evidence_refs: &[],
            })
            .expect("status request");
        let envelope = control_ingress_envelope(&ControlIngressEnvelopeInput {
            request_value: &request_value,
            from_peer: "peer:service",
            to_node: "node:service-ingress",
            topic: DEFAULT_CONTROL_INGRESS_TOPIC,
            sequence: 1,
            peer_bootstrap_refs: &peer_bootstrap_refs,
            authority_refs: &authority_refs,
            policy_refs: &policy_refs,
            resource_refs: &resource_refs,
            evidence_refs: &[],
        })
        .expect("ingress envelope");
        publish_control_ingress(&ControlIngressPublishInput {
            state_root: &root,
            envelope_value: &envelope.value,
        })
        .expect("publish ingress");

        let served = serve_control(&ControlServeInput {
            state_root: &root,
            topic: DEFAULT_CONTROL_INGRESS_TOPIC,
            max_ticks: 1,
            max_requests_per_tick: 4,
            supervisor_policy_value: None,
        })
        .expect("serve ingress");
        assert_eq!(served.decision, "pass");
        assert_eq!(served.heartbeat_receipt_refs.len(), 1);
        assert_eq!(served.ingress_receipt_refs.len(), 1);
        assert_eq!(served.loop_receipt_refs.len(), 1);
        assert_eq!(served.processed_request_refs.len(), 1);
        assert_eq!(crate::ledger::artifact_kind(&served.service_receipt_value), "node-control-service-run-receipt");
        let control_value = read_preserves(&control_outbox_receipt_path(&root, &served.processed_request_refs[0]))
            .expect("read served control receipt");
        let control = crate::node_runtime::parse_control_receipt(&control_value).expect("parse served control");
        assert_eq!(control.decision, "pass");
    }

    #[test]
    fn control_service_duplicate_lock_denies_before_side_effects() {
        let root = temp_dir("node-control-service-duplicate");
        init_local(&InitInput {
            state_root: &root,
            node_id: "node:service-duplicate",
        })
        .expect("init node");
        run_local(&RunInput { state_root: &root }).expect("run node");
        let startup = current_startup_receipt(&root).expect("startup");
        let identity =
            crate::node_identity::parse_node_identity(&read_preserves(&root.join(IDENTITY_FILE)).expect("identity"))
                .expect("parse identity");
        let service_run_ref = local_ref("node-control-service-run", "already-active").expect("service run ref");
        let lock_value = service_lock_value(&ServiceLockValueInput {
            state_root: &root,
            startup_receipt_ref: &startup.receipt_ref,
            node_id: &identity.node_id,
            topic: DEFAULT_CONTROL_INGRESS_TOPIC,
            max_ticks: 1,
            max_requests_per_tick: 1,
            service_run_ref: &service_run_ref,
        })
        .expect("service lock");
        write_preserves(&root.join(CONTROL_SERVICE_LOCK_FILE), &lock_value).expect("write service lock");
        let request = status_request().expect("status request");
        submit_control_request(&ControlSubmitInput {
            state_root: &root,
            request_value: &request.value,
        })
        .expect("submit pending request");

        let served = serve_control(&ControlServeInput {
            state_root: &root,
            topic: DEFAULT_CONTROL_INGRESS_TOPIC,
            max_ticks: 1,
            max_requests_per_tick: 1,
            supervisor_policy_value: None,
        })
        .expect("duplicate service denial");
        assert_eq!(served.decision, "deny");
        assert_eq!(served.ticks, 0);
        assert!(served.processed_request_refs.is_empty());
        assert!(next_pending_control_request(&root).expect("pending scan").is_some());
        let text = crate::preserves_rail::to_text(&served.service_receipt_value).expect("service receipt text");
        assert!(text.contains("already active"));
    }

    #[test]
    fn control_supervisor_policy_recovers_stale_lock_and_bounds_shutdown() {
        let root = initialized_control_root("node-control-supervisor-policy", "node:supervisor-policy");
        write_active_service_lock(&root, "stale");
        let policy_refs = vec![local_ref("node-control-supervisor-policy", "recover").expect("policy ref")];
        let recover_policy = recovering_policy(&policy_refs);

        let recovered = serve_control(&ControlServeInput {
            state_root: &root,
            topic: DEFAULT_CONTROL_INGRESS_TOPIC,
            max_ticks: 1,
            max_requests_per_tick: 1,
            supervisor_policy_value: Some(&recover_policy),
        })
        .expect("recover stale lock");
        assert_eq!(recovered.decision, "pass");
        assert_eq!(recovered.supervisor_receipt_refs.len(), 2);
        assert!(recovered.supervisor_policy_ref.is_some());
        assert!(!root.join(CONTROL_SERVICE_LOCK_FILE).exists());
        let restart_once = serve_control(&ControlServeInput {
            state_root: &root,
            topic: DEFAULT_CONTROL_INGRESS_TOPIC,
            max_ticks: 1,
            max_requests_per_tick: 1,
            supervisor_policy_value: Some(&recover_policy),
        })
        .expect("allowed restart");
        assert_eq!(restart_once.decision, "pass");
        let restart_denied = serve_control(&ControlServeInput {
            state_root: &root,
            topic: DEFAULT_CONTROL_INGRESS_TOPIC,
            max_ticks: 1,
            max_requests_per_tick: 1,
            supervisor_policy_value: Some(&recover_policy),
        })
        .expect("bounded restart denial");
        assert_eq!(restart_denied.decision, "deny");
        assert_eq!(restart_denied.ticks, 0);
        let restart_denied_text =
            crate::preserves_rail::to_text(&restart_denied.service_receipt_value).expect("restart denial receipt text");
        assert!(restart_denied_text.contains("restart attempts"));

        let shutdown = shutdown_request().expect("shutdown request");
        submit_control_request(&ControlSubmitInput {
            state_root: &root,
            request_value: &shutdown.value,
        })
        .expect("submit shutdown");
        let tight_policy = bounded_shutdown_policy(&policy_refs);
        let stopped = serve_control(&ControlServeInput {
            state_root: &root,
            topic: DEFAULT_CONTROL_INGRESS_TOPIC,
            max_ticks: 4,
            max_requests_per_tick: 1,
            supervisor_policy_value: Some(&tight_policy),
        })
        .expect("shutdown serve");
        assert_eq!(stopped.decision, "deny");
        assert!(stopped.has_stopped);
        assert_eq!(stopped.supervisor_receipt_refs.len(), 2);
        let text = crate::preserves_rail::to_text(&stopped.service_receipt_value).expect("service receipt text");
        assert!(text.contains("exceeded supervisor bound"));
    }

    fn write_active_service_lock(root: &Path, service_suffix: &str) {
        let startup = current_startup_receipt(root).expect("startup");
        let identity =
            crate::node_identity::parse_node_identity(&read_preserves(&root.join(IDENTITY_FILE)).expect("identity"))
                .expect("parse identity");
        let service_run_ref = local_ref("node-control-service-run", service_suffix).expect("service run ref");
        let lock_value = service_lock_value(&ServiceLockValueInput {
            state_root: root,
            startup_receipt_ref: &startup.receipt_ref,
            node_id: &identity.node_id,
            topic: DEFAULT_CONTROL_INGRESS_TOPIC,
            max_ticks: 1,
            max_requests_per_tick: 1,
            service_run_ref: &service_run_ref,
        })
        .expect("service lock");
        write_preserves(&root.join(CONTROL_SERVICE_LOCK_FILE), &lock_value).expect("write service lock");
    }

    fn recovering_policy(policy_refs: &[String]) -> IoValue {
        control_supervisor_policy_value(&ControlSupervisorPolicyInput {
            max_restarts: 1,
            restart_window_ticks: 1,
            heartbeat_timeout_ticks: 1,
            shutdown_drain_ticks: 1,
            stale_lock_recovery: true,
            policy_refs,
            evidence_refs: &[],
        })
        .expect("recover policy")
    }

    fn bounded_shutdown_policy(policy_refs: &[String]) -> IoValue {
        control_supervisor_policy_value(&ControlSupervisorPolicyInput {
            max_restarts: 0,
            restart_window_ticks: 1,
            heartbeat_timeout_ticks: 1,
            shutdown_drain_ticks: 0,
            stale_lock_recovery: false,
            policy_refs,
            evidence_refs: &[],
        })
        .expect("tight policy")
    }

    #[test]
    fn control_service_heartbeats_continue_and_shutdown_stops() {
        let root = temp_dir("node-control-service-shutdown");
        init_local(&InitInput {
            state_root: &root,
            node_id: "node:service-shutdown",
        })
        .expect("init node");
        run_local(&RunInput { state_root: &root }).expect("run node");
        let idle = serve_control(&ControlServeInput {
            state_root: &root,
            topic: DEFAULT_CONTROL_INGRESS_TOPIC,
            max_ticks: 2,
            max_requests_per_tick: 1,
            supervisor_policy_value: None,
        })
        .expect("idle serve");
        assert_eq!(idle.decision, "pass");
        assert_eq!(idle.heartbeat_receipt_refs.len(), 2);
        assert_eq!(idle.loop_receipt_refs.len(), 2);
        assert!(!idle.has_stopped);

        let shutdown = shutdown_request().expect("shutdown request");
        submit_control_request(&ControlSubmitInput {
            state_root: &root,
            request_value: &shutdown.value,
        })
        .expect("submit shutdown");
        let stopped = serve_control(&ControlServeInput {
            state_root: &root,
            topic: DEFAULT_CONTROL_INGRESS_TOPIC,
            max_ticks: 4,
            max_requests_per_tick: 1,
            supervisor_policy_value: None,
        })
        .expect("shutdown serve");
        assert_eq!(stopped.decision, "pass");
        assert!(stopped.has_stopped);
        assert_eq!(stopped.processed_request_refs.len(), 1);
        assert!(!root.join(CONTROL_LOCK_FILE).exists());
        assert!(!root.join(CONTROL_SERVICE_LOCK_FILE).exists());
    }

    #[test]
    fn control_operation_dispatch_installs_runs_and_gates_with_receipts() {
        let case = op_case();
        assert_install(&case);
        assert_gate(&case);
        assert_run(&case);
        assert_ledger(&case.root);
    }

    struct OpCase {
        root: PathBuf,
        authority_refs: Vec<String>,
        policy_refs: Vec<String>,
        resource_refs: Vec<String>,
    }

    fn op_case() -> OpCase {
        let root = temp_dir("node-control-operations");
        init_local(&InitInput {
            state_root: &root,
            node_id: "node:ops",
        })
        .expect("init node");
        run_local(&RunInput { state_root: &root }).expect("run node");
        OpCase {
            root,
            authority_refs: vec![local_ref("node-control-authority", "ops").expect("authority ref")],
            policy_refs: vec![local_ref("node-control-policy", "ops").expect("policy ref")],
            resource_refs: vec![local_ref("node-control-resource", "ops").expect("resource ref")],
        }
    }

    fn dispatch_value(case: &OpCase, value: &IoValue) -> crate::node_runtime::ControlReceipt {
        let submitted = submit_control_request(&ControlSubmitInput {
            state_root: &case.root,
            request_value: value,
        })
        .expect("submit request");
        let dispatched = dispatch_control_request(&ControlDispatchInput {
            state_root: &case.root,
            request_path: Some(&submitted.inbox_path),
        })
        .expect("dispatch request");
        crate::node_runtime::parse_control_receipt(&dispatched.control_receipt_value).expect("control receipt")
    }

    fn assert_install(case: &OpCase) {
        let payload_value =
            crate::preserves_rail::record("node-control-install-payload", vec![crate::preserves_rail::string(
                "payload",
            )]);
        let payload_ref = import_artifact(&case.root, &payload_value).expect("import payload");
        let payload_provenance =
            crate::provenance::synthetic_reviewed_provenance_record(&payload_ref).expect("payload provenance");
        let payload_provenance_ref =
            import_artifact(&case.root, &payload_provenance).expect("import payload provenance");
        let install_evidence_refs = vec![payload_provenance_ref];
        let install_value =
            crate::node_runtime::control_request_value(&crate::node_runtime::ControlRequestValueInput {
                operation: "install",
                target_ref: None,
                payload_ref: Some(&payload_ref),
                authority_refs: &case.authority_refs,
                policy_refs: &case.policy_refs,
                resource_refs: &case.resource_refs,
                evidence_refs: &install_evidence_refs,
            })
            .expect("install request");
        let install_receipt = dispatch_value(case, &install_value);
        assert_eq!(install_receipt.decision, "pass");
        let installed = crate::artifacts::list_artifacts(&case.root.join("registry"), Some("node-control-artifact"))
            .expect("list installed artifacts");
        assert_eq!(installed.len(), 1);
    }

    fn assert_gate(case: &OpCase) {
        let gate_value = crate::octet_gate::synthetic_clean_octet_gate_receipt_for_tests().expect("gate receipt");
        let gate_ref = import_artifact(&case.root, &gate_value).expect("import gate");
        let gate_target = local_ref("node-control-gate-target", "ops").expect("gate target");
        let gate_request = crate::node_runtime::control_request_value(&crate::node_runtime::ControlRequestValueInput {
            operation: "gate",
            target_ref: Some(&gate_target),
            payload_ref: Some(&gate_ref),
            authority_refs: &case.authority_refs,
            policy_refs: &case.policy_refs,
            resource_refs: &case.resource_refs,
            evidence_refs: &[],
        })
        .expect("gate request");
        let gate_receipt = dispatch_value(case, &gate_request);
        assert_eq!(gate_receipt.decision, "pass");
        assert!(
            gate_receipt
                .subreceipt_refs
                .iter()
                .any(|reference| crate::preserves_rail::validate_content_ref(reference).is_ok())
        );
    }

    fn assert_run(case: &OpCase) {
        let job_fixture = install_job_fixture(&case.root);
        let execution_request_ref =
            import_artifact(&case.root, &job_fixture.execution_request).expect("import execution request");
        let admission_ref =
            import_artifact(&case.root, &job_fixture.admission_receipt).expect("import admission receipt");
        let job_provenance =
            crate::provenance::synthetic_reviewed_provenance_record(&job_fixture.job_ref).expect("job provenance");
        let job_provenance_ref = import_artifact(&case.root, &job_provenance).expect("import job provenance");
        let run_evidence_refs = vec![job_provenance_ref];
        let run_request = crate::node_runtime::control_request_value(&crate::node_runtime::ControlRequestValueInput {
            operation: "run",
            target_ref: Some(&admission_ref),
            payload_ref: Some(&execution_request_ref),
            authority_refs: &case.authority_refs,
            policy_refs: &case.policy_refs,
            resource_refs: &case.resource_refs,
            evidence_refs: &run_evidence_refs,
        })
        .expect("run request");
        let run_receipt = dispatch_value(case, &run_request);
        assert_eq!(run_receipt.decision, "pass");
    }

    fn assert_ledger(root: &Path) {
        let kinds = crate::ledger::list_artifacts(&root.join("ledger"))
            .expect("list operation ledger")
            .into_iter()
            .map(|entry| entry.artifact_kind)
            .collect::<Vec<_>>();
        assert!(kinds.iter().any(|kind| kind == "artifact-registry-receipt"));
        assert!(kinds.iter().any(|kind| kind == "provenance-record"));
        assert!(kinds.iter().any(|kind| kind == "provenance-receipt"));
        assert!(kinds.iter().any(|kind| kind == "job-execution-receipt"));
        assert!(kinds.iter().any(|kind| kind == "octet-source-gate-validation"));
        assert!(kinds.iter().any(|kind| kind == "node-control-operation-receipt"));
    }

    struct JobFixture {
        execution_request: IoValue,
        admission_receipt: IoValue,
        job_ref: String,
    }

    struct StagePair {
        source_ref: String,
        map_ref: String,
    }

    struct AdmissionParts {
        receipt_value: IoValue,
        receipt_ref: String,
        stage_order: Vec<String>,
        policy_refs: Vec<String>,
        capability_refs: Vec<String>,
        resource_refs: Vec<String>,
    }

    fn install_job_fixture(root: &Path) -> JobFixture {
        let registry = root.join("registry");
        let stages = install_stage_pair(&registry);
        let dag_value = graph_value(&stages);
        let installed = crate::job_dag::install_job_dag(&registry, &dag_value).expect("install job dag");
        let admission = admit_graph(&registry, &installed.job_ref);
        let execution_request = execution_request_value(&installed.job_ref, &admission);
        JobFixture {
            execution_request,
            admission_receipt: admission.receipt_value,
            job_ref: installed.job_ref,
        }
    }

    fn install_stage_pair(registry: &Path) -> StagePair {
        let stage_schema = local_ref("node-job-stage-schema", "ops").expect("stage schema");
        let stage_policy = local_ref("node-job-stage-policy", "ops").expect("stage policy");
        let stage_evidence = local_ref("node-job-stage-evidence", "ops").expect("stage evidence");
        let stage_installer = local_ref("node-job-stage-installer", "ops").expect("stage installer");
        let stage_capability = local_ref("node-job-stage-capability", "ops").expect("stage capability");
        let source_stage = crate::artifacts::install_artifact(registry, &crate::artifacts::ArtifactInstallInput {
            kind: "stage".to_string(),
            payload: crate::job_dag::builtin_stage_operation_value("source").expect("source operation"),
            schema_refs: vec![stage_schema.clone()],
            dependency_refs: Vec::new(),
            effect_manifest_ref: None,
            policy_refs: vec![stage_policy.clone()],
            evidence_refs: vec![stage_evidence.clone()],
            installer_ref: stage_installer.clone(),
            capability_refs: vec![stage_capability.clone()],
        })
        .expect("install source stage");
        let map_stage = crate::artifacts::install_artifact(registry, &crate::artifacts::ArtifactInstallInput {
            kind: "stage".to_string(),
            payload: crate::job_dag::builtin_stage_operation_value("identity").expect("identity operation"),
            schema_refs: vec![stage_schema],
            dependency_refs: Vec::new(),
            effect_manifest_ref: None,
            policy_refs: vec![stage_policy],
            evidence_refs: vec![stage_evidence],
            installer_ref: stage_installer,
            capability_refs: vec![stage_capability],
        })
        .expect("install map stage");
        StagePair {
            source_ref: source_stage.artifact_ref,
            map_ref: map_stage.artifact_ref,
        }
    }

    fn source_vertex_value(stage_ref: &str) -> IoValue {
        crate::job_dag::job_node_value(crate::job_dag::NodeValueInput {
            id: "source",
            kind: "source",
            stage_artifact_ref: Some(stage_ref),
            input_ports: &[],
            output_ports: &["out".to_string()],
            config: crate::preserves_rail::record("source", vec![crate::preserves_rail::record("values", vec![
                crate::preserves_rail::sequence(vec![crate::preserves_rail::string("node-job")]),
            ])]),
            effect_manifest_refs: &[],
            policy_refs: &[],
            evidence_refs: &[],
        })
        .expect("source node")
    }

    fn map_vertex_value(stage_ref: &str) -> IoValue {
        crate::job_dag::job_node_value(crate::job_dag::NodeValueInput {
            id: "map",
            kind: "map",
            stage_artifact_ref: Some(stage_ref),
            input_ports: &["in".to_string()],
            output_ports: &["out".to_string()],
            config: crate::preserves_rail::record("op", vec![crate::preserves_rail::string("identity")]),
            effect_manifest_refs: &[],
            policy_refs: &[],
            evidence_refs: &[],
        })
        .expect("map node")
    }

    fn fixture_edge_value() -> IoValue {
        crate::job_dag::job_edge_value(crate::job_dag::EdgeValueInput {
            from_node: "source",
            from_port: "out",
            to_node: "map",
            to_port: "in",
            schema_ref: None,
            partitioning: "single",
            materialization: "stream",
        })
        .expect("edge")
    }

    fn graph_value(stages: &StagePair) -> IoValue {
        crate::job_dag::job_dag_value(crate::job_dag::DagValueInput {
            nodes: vec![
                source_vertex_value(&stages.source_ref),
                map_vertex_value(&stages.map_ref),
            ],
            edges: vec![fixture_edge_value()],
            output_roots: &["map".to_string()],
            schema_refs: &[],
            effect_manifest_refs: &[],
            policy_refs: &[],
            evidence_refs: &[],
        })
        .expect("dag value")
    }

    fn admit_graph(registry: &Path, graph_ref: &str) -> AdmissionParts {
        let authority_ref = install_job_authority(registry, graph_ref);
        let gate_ref = install_clean_gate(registry);
        let sync_ref = local_ref("node-job-sync", graph_ref).expect("sync ref");
        let resource_refs = vec![local_ref("node-job-resource", graph_ref).expect("resource ref")];
        let policy_refs = vec![local_ref("node-job-policy", graph_ref).expect("policy ref")];
        let capability_refs = vec![authority_ref];
        let evidence_refs = vec![sync_ref.clone(), gate_ref];
        let admission_request =
            crate::job_dag::job_admission_request_value(crate::job_dag::AdmissionRequestValueInput {
                job_ref: graph_ref,
                sync_ref: &sync_ref,
                stage_ids: &[],
                target_peer: "node:ops",
                policy_refs: &policy_refs,
                capability_refs: &capability_refs,
                evidence_refs: &evidence_refs,
                resource_refs: &resource_refs,
            })
            .expect("admission request");
        let admission = crate::job_dag::admission_loopback(registry, &admission_request).expect("admission loopback");
        assert_eq!(admission.plan.decision, "pass");
        AdmissionParts {
            receipt_ref: crate::preserves_rail::canonical_hash(&admission.receipt_value).expect("admission ref"),
            receipt_value: admission.receipt_value,
            stage_order: admission.plan.stage_order,
            policy_refs,
            capability_refs,
            resource_refs,
        }
    }

    fn execution_request_value(graph_ref: &str, admission: &AdmissionParts) -> IoValue {
        crate::job_dag::job_execution_request_value(crate::job_dag::ExecutionRequestValueInput {
            job_ref: graph_ref,
            admission_ref: &admission.receipt_ref,
            stage_ids: &admission.stage_order,
            target_peer: "node:ops",
            storage_profile_ref: &local_ref("node-job-storage", graph_ref).expect("storage ref"),
            cache_profile_ref: &local_ref("node-job-cache", graph_ref).expect("cache ref"),
            chunk_profile_ref: &local_ref("node-job-chunks", graph_ref).expect("chunks ref"),
            policy_refs: &admission.policy_refs,
            capability_refs: &admission.capability_refs,
            resource_refs: &admission.resource_refs,
        })
        .expect("execution request")
    }

    fn install_job_authority(registry: &Path, job_ref: &str) -> String {
        let subject_ref = local_ref("node-job-authority-subject", job_ref).expect("authority subject");
        let policy_ref = local_ref("node-job-authority-policy", job_ref).expect("authority policy");
        let evidence_ref = local_ref("node-job-authority-evidence", job_ref).expect("authority evidence");
        let context_value = crate::authority::authority_context_value(crate::authority::ContextValueInput {
            subject_ref: &subject_ref,
            capabilities: &[crate::authority::AuthorityCapability {
                capability: "job:execute".to_string(),
                scope: job_ref.to_string(),
                attenuation: "scoped".to_string(),
            }],
            delegation_refs: &[],
            not_before: None,
            expires_at: None,
            revocation_refs: &[],
            key_refs: &[],
            policy_refs: std::slice::from_ref(&policy_ref),
            evidence_refs: std::slice::from_ref(&evidence_ref),
        })
        .expect("authority context");
        let context_ref = crate::preserves_rail::canonical_hash(&context_value).expect("authority context ref");
        let install = crate::artifacts::install_artifact(registry, &crate::artifacts::ArtifactInstallInput {
            kind: "authority-context".to_string(),
            payload: context_value,
            schema_refs: Vec::new(),
            dependency_refs: Vec::new(),
            effect_manifest_ref: None,
            policy_refs: vec![policy_ref],
            evidence_refs: vec![evidence_ref],
            installer_ref: local_ref("node-job-authority-installer", job_ref).expect("authority installer"),
            capability_refs: vec![local_ref("node-job-authority-capability", job_ref).expect("authority capability")],
        })
        .expect("install authority context");
        assert_eq!(install.decision, "pass");
        context_ref
    }

    fn install_clean_gate(registry: &Path) -> String {
        let gate_value = crate::octet_gate::synthetic_clean_octet_gate_receipt_for_tests().expect("clean gate");
        let gate_ref = crate::preserves_rail::canonical_hash(&gate_value).expect("gate ref");
        let install = crate::artifacts::install_artifact(registry, &crate::artifacts::ArtifactInstallInput {
            kind: "octet-gate-receipt".to_string(),
            payload: gate_value,
            schema_refs: Vec::new(),
            dependency_refs: Vec::new(),
            effect_manifest_ref: None,
            policy_refs: vec![local_ref("node-job-gate-policy", &gate_ref).expect("gate policy")],
            evidence_refs: vec![local_ref("node-job-gate-evidence", &gate_ref).expect("gate evidence")],
            installer_ref: local_ref("node-job-gate-installer", &gate_ref).expect("gate installer"),
            capability_refs: vec![local_ref("node-job-gate-capability", &gate_ref).expect("gate capability")],
        })
        .expect("install gate");
        assert_eq!(install.decision, "pass");
        gate_ref
    }

    fn temp_dir(name: &str) -> PathBuf {
        crate::test_support::cleanup_stale_molten_temp_dirs();
        static TEMP_DIR_COUNTER: Counter = Counter::new(0);
        let nonce = TEMP_DIR_COUNTER.fetch_add(1, RELAXED);
        let dir = std::env::temp_dir().join(format!("molten-{name}-{}-{nonce}", std::process::id()));
        if dir.exists() {
            fs::remove_dir_all(&dir).expect("remove stale temp dir");
        }
        fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }
}

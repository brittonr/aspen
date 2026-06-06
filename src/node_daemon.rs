use std::fs;
use std::net::Ipv4Addr;
use std::net::SocketAddr;
use std::path::Path;
use std::path::PathBuf;

use n0_future::StreamExt;
use preserves::IOValue;

use crate::artifacts;
use crate::delivery_idempotency;
use crate::error::MoltenError;
use crate::error::Result;
use crate::job_dag;
use crate::ledger;
use crate::node_identity;
use crate::node_runtime;
use crate::octet_gate;
use crate::preserves_rail::NODE_CONTROL_AUTHORITY_GRANT_IMPORT_RECEIPT_SCHEMA;
use crate::preserves_rail::NODE_CONTROL_AUTHORITY_GRANT_SCHEMA;
use crate::preserves_rail::NODE_CONTROL_AUTHORITY_RECEIPT_SCHEMA;
use crate::preserves_rail::NODE_CONTROL_HEARTBEAT_RECEIPT_SCHEMA;
use crate::preserves_rail::NODE_CONTROL_INGRESS_ENVELOPE_SCHEMA;
use crate::preserves_rail::NODE_CONTROL_INGRESS_RECEIPT_SCHEMA;
use crate::preserves_rail::NODE_CONTROL_LIVE_LISTENER_RECEIPT_SCHEMA;
use crate::preserves_rail::NODE_CONTROL_LIVE_PEER_ADMISSION_SCHEMA;
use crate::preserves_rail::NODE_CONTROL_LIVE_SEND_DUPLICATE_RECEIPT_SCHEMA;
use crate::preserves_rail::NODE_CONTROL_LIVE_SEND_RECEIPT_SCHEMA;
use crate::preserves_rail::NODE_CONTROL_LIVE_SEND_RETRY_RECEIPT_SCHEMA;
use crate::preserves_rail::NODE_CONTROL_LIVE_TICKET_IMPORT_RECEIPT_SCHEMA;
use crate::preserves_rail::NODE_CONTROL_LIVE_TICKET_SCHEMA;
use crate::preserves_rail::NODE_CONTROL_LIVE_TRANSPORT_RECEIPT_SCHEMA;
use crate::preserves_rail::NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_ACK_EXPORT_RECEIPT_SCHEMA;
use crate::preserves_rail::NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_ACK_IMPORT_RECEIPT_SCHEMA;
use crate::preserves_rail::NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_ACK_SCHEMA;
use crate::preserves_rail::NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_APPLY_RECEIPT_SCHEMA;
use crate::preserves_rail::NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_EXPORT_RECEIPT_SCHEMA;
use crate::preserves_rail::NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_GATE_RECEIPT_SCHEMA;
use crate::preserves_rail::NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_IMPORT_RECEIPT_SCHEMA;
use crate::preserves_rail::NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_RECONCILE_RECEIPT_SCHEMA;
use crate::preserves_rail::NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_SCHEMA;
use crate::preserves_rail::NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_VERIFY_RECEIPT_SCHEMA;
use crate::preserves_rail::NODE_CONTROL_LIVE_WORKFLOW_RECEIPT_SCHEMA;
use crate::preserves_rail::NODE_CONTROL_LOCK_SCHEMA;
use crate::preserves_rail::NODE_CONTROL_LOOP_RECEIPT_SCHEMA;
use crate::preserves_rail::NODE_CONTROL_OPERATION_RECEIPT_SCHEMA;
use crate::preserves_rail::NODE_CONTROL_QUEUE_RECEIPT_SCHEMA;
use crate::preserves_rail::NODE_CONTROL_SERVICE_HEARTBEAT_RECEIPT_SCHEMA;
use crate::preserves_rail::NODE_CONTROL_SERVICE_LOCK_SCHEMA;
use crate::preserves_rail::NODE_CONTROL_SERVICE_RUN_RECEIPT_SCHEMA;
use crate::preserves_rail::NODE_CONTROL_SUPERVISOR_POLICY_SCHEMA;
use crate::preserves_rail::NODE_CONTROL_SUPERVISOR_RECEIPT_SCHEMA;
use crate::preserves_rail::canonical_bytes;
use crate::preserves_rail::canonical_hash;
use crate::preserves_rail::parse_canonical_bytes;
use crate::preserves_rail::parse_text;
use crate::preserves_rail::record;
use crate::preserves_rail::sequence;
use crate::preserves_rail::string;
use crate::preserves_rail::to_text;
use crate::provenance;

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
pub struct NodeDaemonInitInput<'a> {
    pub state_root: &'a Path,
    pub node_id: &'a str,
}

#[derive(Debug, Clone, Copy)]
pub struct NodeDaemonRunInput<'a> {
    pub state_root: &'a Path,
}

#[derive(Debug, Clone, Copy)]
pub struct NodeDaemonStatusInput<'a> {
    pub state_root: &'a Path,
}

#[derive(Debug, Clone, Copy)]
pub struct NodeDaemonStopInput<'a> {
    pub state_root: &'a Path,
}

#[derive(Debug, Clone, Copy)]
pub struct NodeControlSubmitInput<'a> {
    pub state_root: &'a Path,
    pub request_value: &'a IOValue,
}

#[derive(Debug, Clone, Copy)]
pub struct NodeControlDispatchInput<'a> {
    pub state_root: &'a Path,
    pub request_path: Option<&'a Path>,
}

#[derive(Debug, Clone, Copy)]
pub struct NodeControlLoopInput<'a> {
    pub state_root: &'a Path,
    pub max_requests: u64,
}

#[derive(Debug, Clone, Copy)]
pub struct NodeControlServeInput<'a> {
    pub state_root: &'a Path,
    pub topic: &'a str,
    pub max_ticks: u64,
    pub max_requests_per_tick: u64,
    pub supervisor_policy_value: Option<&'a IOValue>,
}

#[derive(Debug, Clone, Copy)]
pub struct NodeControlSupervisorPolicyInput<'a> {
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
pub struct NodeControlIngressEnvelopeInput<'a> {
    pub request_value: &'a IOValue,
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
pub struct NodeControlIngressPublishInput<'a> {
    pub state_root: &'a Path,
    pub envelope_value: &'a IOValue,
}

#[derive(Debug, Clone, Copy)]
pub struct NodeControlIngressDeliverInput<'a> {
    pub state_root: &'a Path,
    pub topic: &'a str,
    pub envelope_ref: &'a str,
}

#[derive(Debug, Clone, Copy)]
pub struct NodeControlLiveIngressPublishInput<'a> {
    pub sender: &'a iroh_gossip::api::GossipSender,
    pub envelope_value: &'a IOValue,
    pub node_id: &'a str,
}

#[derive(Debug, Clone, Copy)]
pub struct NodeControlLiveIngressReceiveBytesInput<'a> {
    pub state_root: &'a Path,
    pub topic: &'a str,
    pub receiver_node: &'a str,
    pub delivered_from: &'a str,
    pub bytes: &'a [u8],
}

#[derive(Debug, Clone, Copy)]
pub struct NodeControlLiveLoopbackInput<'a> {
    pub state_root: &'a Path,
    pub request_value: &'a IOValue,
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
pub struct NodeControlLiveServeInput<'a> {
    pub state_root: &'a Path,
    pub topic: &'a str,
    pub max_events: u64,
    pub event_timeout_ms: u64,
    pub max_requests_per_tick: u64,
    pub supervisor_policy_value: Option<&'a IOValue>,
}

#[derive(Debug, Clone, Copy)]
pub struct NodeControlLiveServeLoopbackInput<'a> {
    pub state_root: &'a Path,
    pub request_value: &'a IOValue,
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
pub struct NodeControlLiveSendInput<'a> {
    pub state_root: Option<&'a Path>,
    pub request_value: &'a IOValue,
    pub receiver_ticket_value: &'a IOValue,
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
pub struct NodeControlLiveWorkflowInput<'a> {
    pub state_root: Option<&'a Path>,
    pub receiver_ticket_value: &'a IOValue,
    pub peer_admission_value: &'a IOValue,
    pub authority_grant_value: &'a IOValue,
    pub send_receipt_value: &'a IOValue,
    pub receive_receipt_values: &'a [&'a IOValue],
    pub listener_receipt_value: Option<&'a IOValue>,
    pub service_receipt_value: &'a IOValue,
}

#[derive(Debug, Clone, Copy)]
pub struct NodeControlLiveWorkflowBundleExportInput<'a> {
    pub receiver_ticket_value: &'a IOValue,
    pub peer_admission_value: &'a IOValue,
    pub authority_grant_value: &'a IOValue,
    pub receipt_values: &'a [&'a IOValue],
}

#[derive(Debug, Clone, Copy)]
pub struct NodeControlLiveWorkflowBundleVerifyInput<'a> {
    pub bundle_value: &'a IOValue,
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
pub struct NodeControlLiveWorkflowBundleGateInput<'a> {
    pub bundle_value: &'a IOValue,
    pub verify_receipt_value: Option<&'a IOValue>,
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
pub struct NodeControlLiveWorkflowBundleApplyInput<'a> {
    pub state_root: &'a Path,
    pub bundle_value: &'a IOValue,
    pub gate_receipt_value: Option<&'a IOValue>,
    pub is_gate_receipt_required: bool,
    pub request_value: Option<&'a IOValue>,
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
pub struct NodeControlLiveWorkflowBundleReconcileInput<'a> {
    pub apply_receipt_value: &'a IOValue,
    pub send_receipt_value: Option<&'a IOValue>,
    pub ingress_receipt_value: Option<&'a IOValue>,
    pub queue_receipt_value: Option<&'a IOValue>,
    pub control_receipt_value: Option<&'a IOValue>,
    pub expected_envelope_ref: Option<&'a str>,
    pub expected_operation_ref: Option<&'a str>,
    pub expected_request_ref: Option<&'a str>,
}

#[derive(Debug, Clone, Copy)]
pub struct NodeControlLiveWorkflowBundleAckExportInput<'a> {
    pub apply_receipt_value: &'a IOValue,
    pub send_receipt_value: Option<&'a IOValue>,
    pub ingress_receipt_value: Option<&'a IOValue>,
    pub queue_receipt_value: Option<&'a IOValue>,
    pub control_receipt_value: Option<&'a IOValue>,
    pub reconcile_receipt_value: &'a IOValue,
}

#[derive(Debug, Clone, Copy)]
pub struct NodeControlLiveWorkflowBundleAckImportInput<'a> {
    pub state_root: &'a Path,
    pub ack_value: &'a IOValue,
    pub expected_bundle_ref: Option<&'a str>,
    pub expected_envelope_ref: Option<&'a str>,
    pub expected_operation_ref: Option<&'a str>,
    pub expected_request_ref: Option<&'a str>,
}

#[derive(Debug, Clone, Copy)]
pub struct NodeControlLiveWorkflowBundleImportInput<'a> {
    pub state_root: &'a Path,
    pub bundle_value: &'a IOValue,
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
pub struct NodeControlAuthorityGrantInput<'a> {
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
pub struct NodeControlLiveTicketInput<'a> {
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
pub struct NodeControlLiveTicketExportInput<'a> {
    pub state_root: &'a Path,
    pub topic: &'a str,
    pub policy_refs: &'a [String],
    pub evidence_refs: &'a [String],
}

#[derive(Debug, Clone, Copy)]
pub struct NodeControlLiveTicketImportInput<'a> {
    pub state_root: &'a Path,
    pub ticket_value: &'a IOValue,
    pub peer_admission_value: Option<&'a IOValue>,
    pub expected_node: Option<&'a str>,
    pub expected_topic: Option<&'a str>,
    pub expected_endpoint: Option<&'a str>,
    pub expected_peer: Option<&'a str>,
    pub as_of_sequence: u64,
}

#[derive(Debug, Clone, Copy)]
pub struct NodeControlAuthorityGrantImportInput<'a> {
    pub state_root: &'a Path,
    pub grant_value: &'a IOValue,
    pub expected_peer: Option<&'a str>,
    pub expected_node: Option<&'a str>,
    pub expected_operations: &'a [String],
    pub expected_target_scope: Option<&'a str>,
    pub expected_resource_scope: Option<&'a str>,
    pub as_of_epoch: u64,
}

#[derive(Debug, Clone, Copy)]
pub struct NodeControlLivePeerAdmitInput<'a> {
    pub state_root: &'a Path,
    pub ticket_value: &'a IOValue,
    pub peer_id: &'a str,
    pub sequence: u64,
    pub expires_at: Option<u64>,
    pub policy_refs: &'a [String],
    pub evidence_refs: &'a [String],
}

#[derive(Debug, Clone, Copy)]
struct AuthorityReceiptValueInput<'a> {
    decision: &'a str,
    envelope: &'a NodeControlIngressEnvelope,
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
    envelope: &'a NodeControlIngressEnvelope,
    ingress_receipt_ref: Option<&'a str>,
    diagnostics: &'a [String],
}

#[derive(Debug, Clone, Copy)]
struct LiveSendReceiptValueInput<'a> {
    decision: &'a str,
    from_peer: &'a str,
    ticket: &'a NodeControlLiveTicket,
    envelope: &'a NodeControlIngressEnvelope,
    transport_receipt_ref: Option<&'a str>,
    diagnostics: &'a [String],
}

#[derive(Debug, Clone, Copy)]
struct LiveSendRetryReceiptValueInput<'a> {
    decision: &'a str,
    attempt: u64,
    max_attempts: u64,
    from_peer: &'a str,
    ticket: &'a NodeControlLiveTicket,
    envelope: &'a NodeControlIngressEnvelope,
    diagnostics: &'a [String],
}

#[derive(Debug, Clone, Copy)]
struct LiveSendDuplicateReceiptValueInput<'a> {
    from_peer: &'a str,
    ticket: &'a NodeControlLiveTicket,
    envelope: &'a NodeControlIngressEnvelope,
    prior_send_receipt_ref: &'a str,
    diagnostics: &'a [String],
}

#[derive(Debug, Clone, Copy)]
struct LivePeerAdmissionValueInput<'a> {
    decision: &'a str,
    peer_id: &'a str,
    ticket: &'a NodeControlLiveTicket,
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
    ticket: &'a NodeControlLiveTicket,
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
    grant: &'a NodeControlAuthorityGrant,
    as_of_epoch: u64,
    imported_refs: &'a [String],
    diagnostics: &'a [String],
}

#[derive(Debug, Clone, Copy)]
struct LiveWorkflowBundleValueInput<'a> {
    ticket: &'a NodeControlLiveTicket,
    admission: &'a NodeControlLivePeerAdmission,
    authority: &'a NodeControlAuthorityGrant,
    ticket_value: &'a IOValue,
    admission_value: &'a IOValue,
    authority_value: &'a IOValue,
    receipt_values: &'a [&'a IOValue],
    diagnostics: &'a [String],
}

#[derive(Debug, Clone, Copy)]
struct LiveWorkflowBundleExportReceiptValueInput<'a> {
    decision: &'a str,
    bundle: &'a NodeControlLiveWorkflowBundle,
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
    apply_receipt_value: &'a IOValue,
    send_receipt_value: Option<&'a IOValue>,
    ingress_receipt_value: Option<&'a IOValue>,
    queue_receipt_value: Option<&'a IOValue>,
    control_receipt_value: Option<&'a IOValue>,
    reconcile_receipt_value: &'a IOValue,
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
    ack: &'a NodeControlLiveWorkflowBundleAck,
    diagnostics: &'a [String],
}

#[derive(Debug, Clone, Copy)]
struct LiveWorkflowBundleAckImportReceiptValueInput<'a> {
    decision: &'a str,
    state_root: &'a Path,
    ack: &'a NodeControlLiveWorkflowBundleAck,
    imported_refs: &'a [String],
    diagnostics: &'a [String],
}

#[derive(Debug, Clone, Copy)]
struct ReconcileArtifacts<'a> {
    apply: &'a NodeControlLiveWorkflowBundleApplyReceipt,
    send: Option<&'a NodeControlLiveSendReceipt>,
    ingress: Option<&'a NodeControlIngressReceipt>,
    queue: Option<&'a NodeControlQueueReceipt>,
    control: Option<&'a node_runtime::NodeControlReceipt>,
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
    bundle: &'a NodeControlLiveWorkflowBundle,
    ticket_import_ref: Option<&'a str>,
    authority_import_ref: Option<&'a str>,
    imported_refs: &'a [String],
    diagnostics: &'a [String],
}

#[derive(Debug)]
struct DeniedLiveSendInput<'a> {
    input: &'a NodeControlLiveSendInput<'a>,
    ticket: &'a NodeControlLiveTicket,
    envelope: NodeControlIngressEnvelope,
    diagnostics: Vec<String>,
    retry_receipt_refs: Vec<String>,
    retry_receipt_values: Vec<IOValue>,
}

#[derive(Debug, Clone, Copy)]
struct LiveWorkflowReceiptValueInput<'a> {
    decision: &'a str,
    ticket: &'a NodeControlLiveTicket,
    admission: &'a NodeControlLivePeerAdmission,
    authority: &'a NodeControlAuthorityGrant,
    send: &'a NodeControlLiveSendReceipt,
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
    envelope: &'a NodeControlIngressEnvelope,
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
    request: &'a node_runtime::NodeControlRequest,
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
    request: &'a node_runtime::NodeControlRequest,
    startup_receipt_ref: &'a str,
    subreceipt_refs: &'a [String],
    diagnostics: &'a [String],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeDaemonInit {
    pub config_ref: String,
    pub identity_ref: String,
    pub identity_receipt_ref: String,
    pub config_value: IOValue,
    pub identity_receipt_value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeDaemonRun {
    pub startup_ref: String,
    pub startup_value: IOValue,
    pub adapter_receipt_refs: Vec<node_runtime::NodeAdapterReceiptRef>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeDaemonStatus {
    pub health_ref: String,
    pub control_receipt_ref: String,
    pub health_value: IOValue,
    pub control_receipt_value: IOValue,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeDaemonStop {
    pub shutdown_ref: String,
    pub control_receipt_ref: String,
    pub shutdown_value: IOValue,
    pub control_receipt_value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeControlSubmit {
    pub request_ref: String,
    pub inbox_path: PathBuf,
    pub queue_receipt_ref: String,
    pub queue_receipt_value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeControlDispatch {
    pub operation: String,
    pub request_ref: String,
    pub control_receipt_ref: String,
    pub control_receipt_value: IOValue,
    pub subreceipt_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeControlLoop {
    pub loop_receipt_ref: String,
    pub loop_receipt_value: IOValue,
    pub heartbeat_receipt_ref: String,
    pub heartbeat_receipt_value: IOValue,
    pub processed_request_refs: Vec<String>,
    pub dispatch_receipt_refs: Vec<String>,
    pub has_stopped: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeControlServe {
    pub service_receipt_ref: String,
    pub service_receipt_value: IOValue,
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
pub struct NodeControlSupervisorPolicy {
    pub policy_ref: String,
    pub max_restarts: u64,
    pub restart_window_ticks: u64,
    pub heartbeat_timeout_ticks: u64,
    pub shutdown_drain_ticks: u64,
    pub stale_lock_recovery: bool,
    pub policy_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeControlSupervisorReceipt {
    pub receipt_ref: String,
    pub decision: String,
    pub operation: String,
    pub supervisor_policy_ref: Option<String>,
    pub diagnostics: Vec<String>,
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeControlIngressEnvelope {
    pub envelope_ref: String,
    pub transport: String,
    pub topic: String,
    pub from_peer: String,
    pub to_node: String,
    pub sequence: u64,
    pub operation_ref: String,
    pub request: node_runtime::NodeControlRequest,
    pub peer_bootstrap_refs: Vec<String>,
    pub authority_refs: Vec<String>,
    pub policy_refs: Vec<String>,
    pub resource_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeControlIngressPublish {
    pub envelope_ref: String,
    pub envelope_path: PathBuf,
    pub receipt_ref: String,
    pub receipt_value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeControlIngressDeliver {
    pub envelope_ref: String,
    pub request_ref: String,
    pub ingress_receipt_ref: String,
    pub ingress_receipt_value: IOValue,
    pub idempotency_receipt_ref: Option<String>,
    pub queue_receipt_ref: Option<String>,
    pub has_enqueued: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeControlLiveIngressPublish {
    pub envelope_ref: String,
    pub transport_receipt_ref: String,
    pub transport_receipt_value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeControlLiveIngressReceive {
    pub envelope_ref: String,
    pub transport_receipt_ref: String,
    pub transport_receipt_value: IOValue,
    pub ingress_receipt_ref: String,
    pub ingress_receipt_value: IOValue,
    pub has_enqueued: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeControlLiveLoopback {
    pub envelope_ref: String,
    pub publish_receipt_ref: String,
    pub publish_receipt_value: IOValue,
    pub receive_receipt_ref: String,
    pub receive_receipt_value: IOValue,
    pub ingress_receipt_ref: String,
    pub has_enqueued: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeControlLiveSend {
    pub envelope_ref: String,
    pub envelope_value: IOValue,
    pub operation_ref: String,
    pub receiver_ticket_ref: String,
    pub receiver_endpoint_id: String,
    pub transport_receipt_ref: Option<String>,
    pub transport_receipt_value: Option<IOValue>,
    pub retry_receipt_refs: Vec<String>,
    pub retry_receipt_values: Vec<IOValue>,
    pub duplicate_receipt_ref: Option<String>,
    pub duplicate_receipt_value: Option<IOValue>,
    pub send_receipt_ref: String,
    pub send_receipt_value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeControlLiveSendReceipt {
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
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeControlLiveWorkflowReceipt {
    pub receipt_ref: String,
    pub receipt_value: IOValue,
    pub decision: String,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeControlLiveWorkflowBundle {
    pub bundle_ref: String,
    pub bundle_value: IOValue,
    pub ticket_ref: String,
    pub peer_admission_ref: String,
    pub authority_grant_ref: String,
    pub receipt_refs: Vec<String>,
    pub ticket_value: IOValue,
    pub peer_admission_value: IOValue,
    pub authority_grant_value: IOValue,
    pub receipt_values: Vec<IOValue>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeControlLiveWorkflowBundleExport {
    pub bundle: NodeControlLiveWorkflowBundle,
    pub receipt_ref: String,
    pub receipt_value: IOValue,
    pub decision: String,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeControlLiveWorkflowBundleVerify {
    pub bundle_ref: String,
    pub ticket_ref: Option<String>,
    pub peer_admission_ref: Option<String>,
    pub authority_grant_ref: Option<String>,
    pub receipt_refs: Vec<String>,
    pub diagnostics: Vec<String>,
    pub receipt_ref: String,
    pub receipt_value: IOValue,
    pub decision: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeControlLiveWorkflowBundleVerifyReceipt {
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
pub struct NodeControlLiveWorkflowBundleGate {
    pub bundle_ref: String,
    pub verify_receipt_ref: Option<String>,
    pub recomputed_verify_receipt_ref: String,
    pub ticket_ref: Option<String>,
    pub peer_admission_ref: Option<String>,
    pub authority_grant_ref: Option<String>,
    pub receipt_refs: Vec<String>,
    pub diagnostics: Vec<String>,
    pub receipt_ref: String,
    pub receipt_value: IOValue,
    pub decision: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeControlLiveWorkflowBundleGateReceipt {
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
pub struct NodeControlLiveSendPreflight {
    pub decision: String,
    pub envelope_ref: String,
    pub operation_ref: String,
    pub receiver_ticket_ref: String,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeControlLiveWorkflowBundleApply {
    pub bundle_ref: String,
    pub gate_receipt_ref: Option<String>,
    pub recomputed_verify_receipt_ref: String,
    pub import_receipt_ref: Option<String>,
    pub imported_refs: Vec<String>,
    pub envelope_ref: Option<String>,
    pub operation_ref: Option<String>,
    pub send_receipt_ref: Option<String>,
    pub send_receipt_value: Option<IOValue>,
    pub diagnostics: Vec<String>,
    pub receipt_ref: String,
    pub receipt_value: IOValue,
    pub decision: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeControlLiveWorkflowBundleApplyReceipt {
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
pub struct NodeControlIngressReceipt {
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
pub struct NodeControlQueueReceipt {
    pub receipt_ref: String,
    pub decision: String,
    pub phase: String,
    pub operation: String,
    pub request_ref: String,
    pub location_ref: String,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeControlLiveWorkflowBundleReconcile {
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
    pub receipt_value: IOValue,
    pub decision: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeControlLiveWorkflowBundleReconcileReceipt {
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
pub struct NodeControlLiveWorkflowBundleAck {
    pub ack_ref: String,
    pub ack_value: IOValue,
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
    pub apply_receipt_value: IOValue,
    pub send_receipt_value: Option<IOValue>,
    pub ingress_receipt_value: Option<IOValue>,
    pub queue_receipt_value: Option<IOValue>,
    pub control_receipt_value: Option<IOValue>,
    pub reconcile_receipt_value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeControlLiveWorkflowBundleAckExport {
    pub ack: NodeControlLiveWorkflowBundleAck,
    pub receipt_ref: String,
    pub receipt_value: IOValue,
    pub decision: String,
    pub receiver_decision: String,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeControlLiveWorkflowBundleAckImport {
    pub ack_ref: String,
    pub bundle_ref: String,
    pub imported_refs: Vec<String>,
    pub receiver_decision: String,
    pub diagnostics: Vec<String>,
    pub receipt_ref: String,
    pub receipt_value: IOValue,
    pub decision: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeControlLiveWorkflowBundleImport {
    pub bundle_ref: String,
    pub ticket_import_ref: Option<String>,
    pub authority_import_ref: Option<String>,
    pub imported_refs: Vec<String>,
    pub diagnostics: Vec<String>,
    pub receipt_ref: String,
    pub receipt_value: IOValue,
    pub decision: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeControlLiveServe {
    pub listener_receipt_ref: String,
    pub listener_receipt_value: IOValue,
    pub service: NodeControlServe,
    pub transport_receipt_refs: Vec<String>,
    pub neighbor_events: Vec<String>,
    pub observed_events: u64,
    pub bound_endpoint_id: String,
    pub live_ticket_ref: Option<String>,
    pub live_ticket_value: Option<IOValue>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeControlLiveServeLoopback {
    pub envelope_ref: String,
    pub publish_receipt_ref: String,
    pub listener: NodeControlLiveServe,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeControlAuthorityGrant {
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
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeControlLiveTicket {
    pub ticket_ref: String,
    pub node_id: String,
    pub node_identity_ref: String,
    pub logical_endpoint_id: String,
    pub live_endpoint_id: String,
    pub topic: String,
    pub address_refs: Vec<String>,
    pub policy_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeControlLivePeerAdmission {
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
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeControlLiveTicketImport {
    pub decision: String,
    pub ticket_ref: String,
    pub peer_admission_ref: Option<String>,
    pub imported_refs: Vec<String>,
    pub diagnostics: Vec<String>,
    pub receipt_ref: String,
    pub receipt_value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeControlAuthorityGrantImport {
    pub decision: String,
    pub grant_ref: String,
    pub imported_refs: Vec<String>,
    pub diagnostics: Vec<String>,
    pub receipt_ref: String,
    pub receipt_value: IOValue,
}

pub fn node_control_authority_grant_value(input: &NodeControlAuthorityGrantInput<'_>) -> Result<IOValue> {
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
    Ok(record("node-control-authority-grant-v1", vec![
        string(NODE_CONTROL_AUTHORITY_GRANT_SCHEMA),
        record("peer", vec![string(input.peer_id)]),
        record("node", vec![string(input.node_id)]),
        record("operations", vec![sequence(input.operations.iter().map(string).collect())]),
        record("target-scope", vec![string(input.target_scope)]),
        record("resource-scope", vec![string(input.resource_scope)]),
        record("epoch", vec![string(input.epoch.to_string())]),
        record("expires-at", vec![optional_string(
            input.expires_at.map(|value| value.to_string()).as_deref(),
        )]),
        record("policy", vec![sequence(input.policy_refs.iter().map(string).collect())]),
        record("revocations", vec![sequence(input.revocation_refs.iter().map(string).collect())]),
        record("evidence", vec![sequence(input.evidence_refs.iter().map(string).collect())]),
        record("checks", vec![sequence(vec![
            record("check", vec![string("peer-node-bound"), string("pass")]),
            record("check", vec![string("operation-scope-bound"), string("pass")]),
            record("check", vec![string("revocation-checked-at-ingress"), string("pass")]),
            record("check", vec![string("transport-is-not-authority"), string("pass")]),
        ])]),
    ]))
}

pub fn parse_node_control_authority_grant(value: &IOValue) -> Result<NodeControlAuthorityGrant> {
    let fields = value
        .collect_simple_record("node-control-authority-grant-v1", Some(12))
        .ok_or_else(|| MoltenError::invalid_harness("expected <node-control-authority-grant-v1 ...>"))?;
    require_schema(&fields[0], NODE_CONTROL_AUTHORITY_GRANT_SCHEMA, "node control authority grant")?;
    let operations = record_strings(&fields[3], "operations")?;
    if operations.is_empty() {
        return Err(MoltenError::invalid_harness("node control authority grant operations missing"));
    }
    Ok(NodeControlAuthorityGrant {
        grant_ref: canonical_hash(value)?,
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

pub fn import_node_control_authority_grant(
    state_root: &Path,
    grant_value: &IOValue,
) -> Result<NodeControlAuthorityGrant> {
    validate_state_root(state_root)?;
    ensure_state_layout(state_root)?;
    let grant = parse_node_control_authority_grant(grant_value)?;
    import_node_artifact(state_root, grant_value)?;
    Ok(grant)
}

pub fn node_control_live_ticket_value(input: &NodeControlLiveTicketInput<'_>) -> Result<IOValue> {
    validate_node_id(input.node_id)?;
    validate_ingress_ref(input.node_identity_ref, "node control live ticket identity ref")?;
    validate_node_id(input.logical_endpoint_id)?;
    validate_node_id(input.live_endpoint_id)?;
    validate_node_id(input.topic)?;
    validate_ingress_refs(input.policy_refs, "node control live ticket policy ref")?;
    validate_ingress_refs(input.evidence_refs, "node control live ticket evidence ref")?;
    Ok(record("node-control-live-ticket-v1", vec![
        string(NODE_CONTROL_LIVE_TICKET_SCHEMA),
        record("node", vec![
            record("id", vec![string(input.node_id)]),
            record("identity", vec![string(input.node_identity_ref)]),
            record("logical-endpoint", vec![string(input.logical_endpoint_id)]),
        ]),
        record("live", vec![
            record("endpoint-id", vec![string(input.live_endpoint_id)]),
            record("topic", vec![string(input.topic)]),
            record("addresses", vec![sequence(input.address_refs.iter().map(string).collect())]),
        ]),
        record("policy", vec![sequence(input.policy_refs.iter().map(string).collect())]),
        record("evidence", vec![sequence(input.evidence_refs.iter().map(string).collect())]),
        record("checks", vec![sequence(vec![
            record("check", vec![string("node-identity-bound"), string("pass")]),
            record("check", vec![string("live-endpoint-bound"), string("pass")]),
            record("check", vec![string("ticket-is-bootstrap-not-authority"), string("pass")]),
            record("check", vec![string("authority-grant-still-required"), string("pass")]),
        ])]),
    ]))
}

pub fn parse_node_control_live_ticket(value: &IOValue) -> Result<NodeControlLiveTicket> {
    let fields = value
        .collect_simple_record("node-control-live-ticket-v1", Some(6))
        .ok_or_else(|| MoltenError::invalid_harness("expected <node-control-live-ticket-v1 ...>"))?;
    require_schema(&fields[0], NODE_CONTROL_LIVE_TICKET_SCHEMA, "node control live ticket")?;
    let node = crate::preserves_rail::value_to_iovalue(&fields[1]);
    let node_fields = node
        .collect_simple_record("node", Some(3))
        .ok_or_else(|| MoltenError::invalid_harness("node control live ticket missing node"))?;
    let live = crate::preserves_rail::value_to_iovalue(&fields[2]);
    let live_fields = live
        .collect_simple_record("live", Some(3))
        .ok_or_else(|| MoltenError::invalid_harness("node control live ticket missing live endpoint"))?;
    Ok(NodeControlLiveTicket {
        ticket_ref: canonical_hash(value)?,
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

pub fn export_node_control_live_ticket(input: &NodeControlLiveTicketExportInput<'_>) -> Result<NodeControlLiveTicket> {
    validate_state_root(input.state_root)?;
    validate_node_id(input.topic)?;
    ensure_state_layout(input.state_root)?;
    let identity = node_identity::parse_node_identity(&read_preserves(&input.state_root.join(IDENTITY_FILE))?)?;
    let address_refs = Vec::new();
    let value = node_control_live_ticket_value(&NodeControlLiveTicketInput {
        node_id: &identity.node_id,
        node_identity_ref: &identity.identity_ref,
        logical_endpoint_id: &identity.endpoint_id,
        live_endpoint_id: &stable_live_endpoint_id(&identity),
        topic: input.topic,
        address_refs: &address_refs,
        policy_refs: input.policy_refs,
        evidence_refs: input.evidence_refs,
    })?;
    let ticket = parse_node_control_live_ticket(&value)?;
    import_node_artifact(input.state_root, &value)?;
    Ok(ticket)
}

pub fn admit_node_control_live_peer(input: &NodeControlLivePeerAdmitInput<'_>) -> Result<NodeControlLivePeerAdmission> {
    validate_state_root(input.state_root)?;
    validate_node_id(input.peer_id)?;
    validate_ingress_refs(input.policy_refs, "node control live peer admission policy ref")?;
    validate_ingress_refs(input.evidence_refs, "node control live peer admission evidence ref")?;
    ensure_state_layout(input.state_root)?;
    let ticket = parse_node_control_live_ticket(input.ticket_value)?;
    import_node_artifact(input.state_root, input.ticket_value)?;
    let identity = node_identity::parse_node_identity(&read_preserves(&input.state_root.join(IDENTITY_FILE))?)?;
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
    let value = node_control_live_peer_admission_value(&LivePeerAdmissionValueInput {
        decision,
        peer_id: input.peer_id,
        ticket: &ticket,
        admission_sequence: input.sequence,
        expires_at: input.expires_at,
        policy_refs: input.policy_refs,
        evidence_refs: input.evidence_refs,
        diagnostics: &diagnostics,
    })?;
    let admission = parse_node_control_live_peer_admission(&value)?;
    import_node_artifact(input.state_root, &value)?;
    Ok(admission)
}

fn node_control_live_peer_admission_value(input: &LivePeerAdmissionValueInput<'_>) -> Result<IOValue> {
    validate_decision(input.decision)?;
    Ok(record("node-control-live-peer-admission-v1", vec![
        string(NODE_CONTROL_LIVE_PEER_ADMISSION_SCHEMA),
        record("decision", vec![string(input.decision)]),
        record("peer", vec![string(input.peer_id)]),
        record("ticket", vec![string(&input.ticket.ticket_ref)]),
        record("node", vec![string(&input.ticket.node_id)]),
        record("topic", vec![string(&input.ticket.topic)]),
        record("sequence", vec![string(input.admission_sequence.to_string())]),
        record("expires-at", vec![optional_string(
            input.expires_at.map(|value| value.to_string()).as_deref(),
        )]),
        record("policy", vec![sequence(input.policy_refs.iter().map(string).collect())]),
        record("evidence", vec![sequence(input.evidence_refs.iter().map(string).collect())]),
        record("diagnostics", vec![sequence(input.diagnostics.iter().map(string).collect())]),
        record("checks", vec![sequence(vec![
            record("check", vec![
                string("ticket-bound"),
                string(if input.decision == "pass" { "pass" } else { "fail" }),
            ]),
            record("check", vec![
                string("peer-topic-bound"),
                string(if input.decision == "pass" { "pass" } else { "fail" }),
            ]),
            record("check", vec![string("bootstrap-not-authority"), string("pass")]),
            record("check", vec![string("authority-grant-still-required"), string("pass")]),
        ])]),
    ]))
}

pub fn parse_node_control_live_peer_admission(value: &IOValue) -> Result<NodeControlLivePeerAdmission> {
    let fields = value
        .collect_simple_record("node-control-live-peer-admission-v1", Some(12))
        .ok_or_else(|| MoltenError::invalid_harness("expected <node-control-live-peer-admission-v1 ...>"))?;
    require_schema(&fields[0], NODE_CONTROL_LIVE_PEER_ADMISSION_SCHEMA, "node control live peer admission")?;
    Ok(NodeControlLivePeerAdmission {
        admission_ref: canonical_hash(value)?,
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

pub fn import_node_control_live_ticket(
    input: &NodeControlLiveTicketImportInput<'_>,
) -> Result<NodeControlLiveTicketImport> {
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
    let ticket = parse_node_control_live_ticket(input.ticket_value)?;
    let admission = input.peer_admission_value.map(parse_node_control_live_peer_admission).transpose()?;
    let mut diagnostics = live_ticket_import_diagnostics(input, &ticket, admission.as_ref());
    if input.peer_admission_value.is_some() && admission.is_none() {
        diagnostics.push("node control live ticket import admission was not parsed".to_string());
    }
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let mut imported_refs = Vec::with_capacity(2);
    if diagnostics.is_empty() {
        imported_refs.push(import_node_artifact(input.state_root, input.ticket_value)?);
        if let Some(value) = input.peer_admission_value {
            imported_refs.push(import_node_artifact(input.state_root, value)?);
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
    let receipt_ref = canonical_hash(&receipt_value)?;
    import_node_artifact(input.state_root, &receipt_value)?;
    Ok(NodeControlLiveTicketImport {
        decision: decision.to_string(),
        ticket_ref: ticket.ticket_ref,
        peer_admission_ref: admission.map(|value| value.admission_ref),
        imported_refs,
        diagnostics,
        receipt_ref,
        receipt_value,
    })
}

pub fn import_node_control_authority_grant_checked(
    input: &NodeControlAuthorityGrantImportInput<'_>,
) -> Result<NodeControlAuthorityGrantImport> {
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
    let grant = parse_node_control_authority_grant(input.grant_value)?;
    let diagnostics = authority_grant_import_diagnostics(input, &grant);
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let mut imported_refs = Vec::with_capacity(1);
    if diagnostics.is_empty() {
        imported_refs.push(import_node_artifact(input.state_root, input.grant_value)?);
    }
    let receipt_value = authority_grant_import_receipt_value(&AuthorityGrantImportReceiptValueInput {
        decision,
        state_root: input.state_root,
        grant: &grant,
        as_of_epoch: input.as_of_epoch,
        imported_refs: &imported_refs,
        diagnostics: &diagnostics,
    })?;
    let receipt_ref = canonical_hash(&receipt_value)?;
    import_node_artifact(input.state_root, &receipt_value)?;
    Ok(NodeControlAuthorityGrantImport {
        decision: decision.to_string(),
        grant_ref: grant.grant_ref,
        imported_refs,
        diagnostics,
        receipt_ref,
        receipt_value,
    })
}

pub fn export_node_control_live_workflow_bundle(
    input: &NodeControlLiveWorkflowBundleExportInput<'_>,
) -> Result<NodeControlLiveWorkflowBundleExport> {
    let ticket = parse_node_control_live_ticket(input.receiver_ticket_value)?;
    let admission = parse_node_control_live_peer_admission(input.peer_admission_value)?;
    let authority = parse_node_control_authority_grant(input.authority_grant_value)?;
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
    let bundle_ref = canonical_hash(&bundle_value)?;
    let bundle = NodeControlLiveWorkflowBundle {
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
    let receipt_ref = canonical_hash(&receipt_value)?;
    Ok(NodeControlLiveWorkflowBundleExport {
        bundle,
        receipt_ref,
        receipt_value,
        decision: decision.to_string(),
        diagnostics,
    })
}

pub fn verify_node_control_live_workflow_bundle(
    input: &NodeControlLiveWorkflowBundleVerifyInput<'_>,
) -> Result<NodeControlLiveWorkflowBundleVerify> {
    validate_live_workflow_bundle_verify_input(input)?;
    let bundle_ref = canonical_hash(input.bundle_value)?;
    let expected = live_workflow_bundle_expected_input_from_verify(input);
    let parsed = parse_node_control_live_workflow_bundle(input.bundle_value);
    let (ticket_ref, peer_admission_ref, authority_grant_ref, receipt_refs, diagnostics) = match parsed {
        Ok(bundle) => {
            let ticket = parse_node_control_live_ticket(&bundle.ticket_value)?;
            let admission = parse_node_control_live_peer_admission(&bundle.peer_admission_value)?;
            let authority = parse_node_control_authority_grant(&bundle.authority_grant_value)?;
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
    let receipt_ref = canonical_hash(&receipt_value)?;
    Ok(NodeControlLiveWorkflowBundleVerify {
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

pub fn gate_node_control_live_workflow_bundle(
    input: &NodeControlLiveWorkflowBundleGateInput<'_>,
) -> Result<NodeControlLiveWorkflowBundleGate> {
    let verify_input = live_workflow_bundle_verify_input_from_gate(input);
    let verified = verify_node_control_live_workflow_bundle(&verify_input)?;
    let expected = live_workflow_bundle_expected_input_from_verify(&verify_input);
    let mut diagnostics = verified.diagnostics.clone();
    let verify_receipt_ref = match input.verify_receipt_value {
        Some(value) => match parse_node_control_live_workflow_bundle_verify_receipt(value) {
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
                let receipt_ref = canonical_hash(value)?;
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
    let receipt_ref = canonical_hash(&receipt_value)?;
    Ok(NodeControlLiveWorkflowBundleGate {
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

pub async fn apply_node_control_live_workflow_bundle(
    input: &NodeControlLiveWorkflowBundleApplyInput<'_>,
) -> Result<NodeControlLiveWorkflowBundleApply> {
    validate_live_workflow_bundle_apply_input(input)?;
    ensure_state_layout(input.state_root)?;
    let verify_input = live_workflow_bundle_verify_input_from_apply(input);
    let verified = verify_node_control_live_workflow_bundle(&verify_input)?;
    let expected = live_workflow_bundle_expected_input_from_verify(&verify_input);
    let mut diagnostics = verified.diagnostics.clone();
    let gate_receipt_ref = match input.gate_receipt_value {
        Some(value) => match parse_node_control_live_workflow_bundle_gate_receipt(value) {
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
                let receipt_ref = canonical_hash(value)?;
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
    if input.should_send && input.request_value.is_none() {
        diagnostics.push("node control live workflow bundle apply send requested without a request".to_string());
    }
    let mut import_receipt_ref = None;
    let mut imported_refs = Vec::new();
    let mut envelope_ref = None;
    let mut operation_ref = None;
    let mut send_receipt_ref = None;
    let mut send_receipt_value = None;
    if diagnostics.is_empty() {
        let imported = import_node_control_live_workflow_bundle(&live_workflow_bundle_import_input_from_apply(input))?;
        import_receipt_ref = Some(imported.receipt_ref.clone());
        if imported.decision == "pass" {
            imported_refs = imported.imported_refs;
        } else {
            diagnostics.extend(imported.diagnostics);
        }
    }
    if diagnostics.is_empty()
        && let Some(request_value) = input.request_value
    {
        let bundle = parse_node_control_live_workflow_bundle(input.bundle_value)?;
        let authority = parse_node_control_authority_grant(&bundle.authority_grant_value)?;
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
        let send_input = NodeControlLiveSendInput {
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
            let sent = send_node_control_live_ingress(&send_input).await?;
            envelope_ref = Some(sent.envelope_ref.clone());
            operation_ref = Some(sent.operation_ref.clone());
            send_receipt_ref = Some(sent.send_receipt_ref.clone());
            let send_receipt = parse_node_control_live_send_receipt(&sent.send_receipt_value)?;
            if send_receipt.decision != "pass" {
                diagnostics.extend(send_receipt.diagnostics);
            }
            send_receipt_value = Some(sent.send_receipt_value);
        } else {
            let preflight = preflight_node_control_live_send(&send_input)?;
            envelope_ref = Some(preflight.envelope_ref);
            operation_ref = Some(preflight.operation_ref);
            if preflight.decision != "pass" {
                diagnostics.extend(preflight.diagnostics);
            }
        }
    }
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let mode = if input.should_send {
        "send"
    } else if input.request_value.is_some() {
        "dry-run"
    } else {
        "import"
    };
    let receipt_value = live_workflow_bundle_apply_receipt_value(&LiveWorkflowBundleApplyReceiptValueInput {
        decision,
        state_root: input.state_root,
        bundle_ref: &verified.bundle_ref,
        gate_receipt_ref: gate_receipt_ref.as_deref(),
        recomputed_verify_receipt_ref: &verified.receipt_ref,
        import_receipt_ref: import_receipt_ref.as_deref(),
        imported_refs: &imported_refs,
        mode,
        envelope_ref: envelope_ref.as_deref(),
        operation_ref: operation_ref.as_deref(),
        send_receipt_ref: send_receipt_ref.as_deref(),
        expected: &expected,
        diagnostics: &diagnostics,
    })?;
    let receipt_ref = canonical_hash(&receipt_value)?;
    import_node_artifact(input.state_root, &receipt_value)?;
    Ok(NodeControlLiveWorkflowBundleApply {
        bundle_ref: verified.bundle_ref,
        gate_receipt_ref,
        recomputed_verify_receipt_ref: verified.receipt_ref,
        import_receipt_ref,
        imported_refs,
        envelope_ref,
        operation_ref,
        send_receipt_ref,
        send_receipt_value,
        diagnostics,
        receipt_ref,
        receipt_value,
        decision: decision.to_string(),
    })
}

pub fn reconcile_node_control_live_workflow_bundle(
    input: &NodeControlLiveWorkflowBundleReconcileInput<'_>,
) -> Result<NodeControlLiveWorkflowBundleReconcile> {
    validate_live_workflow_bundle_reconcile_input(input)?;
    let apply = parse_node_control_live_workflow_bundle_apply_receipt(input.apply_receipt_value)?;
    let send = input.send_receipt_value.map(parse_node_control_live_send_receipt).transpose()?;
    let ingress = input.ingress_receipt_value.map(parse_node_control_ingress_receipt).transpose()?;
    let queue = input.queue_receipt_value.map(parse_node_control_queue_receipt).transpose()?;
    let control = input.control_receipt_value.map(node_runtime::parse_node_control_receipt).transpose()?;
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
    let receipt_ref = canonical_hash(&receipt_value)?;
    diagnostics.shrink_to_fit();
    Ok(NodeControlLiveWorkflowBundleReconcile {
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

pub fn export_node_control_live_workflow_bundle_ack(
    input: &NodeControlLiveWorkflowBundleAckExportInput<'_>,
) -> Result<NodeControlLiveWorkflowBundleAckExport> {
    let reconciled = reconcile_node_control_live_workflow_bundle(&NodeControlLiveWorkflowBundleReconcileInput {
        apply_receipt_value: input.apply_receipt_value,
        send_receipt_value: input.send_receipt_value,
        ingress_receipt_value: input.ingress_receipt_value,
        queue_receipt_value: input.queue_receipt_value,
        control_receipt_value: input.control_receipt_value,
        expected_envelope_ref: None,
        expected_operation_ref: None,
        expected_request_ref: None,
    })?;
    let reconcile = parse_node_control_live_workflow_bundle_reconcile_receipt(input.reconcile_receipt_value)?;
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
    let ack = parse_node_control_live_workflow_bundle_ack(&ack_value)?;
    let receipt_value = live_workflow_bundle_ack_export_receipt_value(&LiveWorkflowBundleAckExportReceiptValueInput {
        decision,
        ack: &ack,
        diagnostics: &diagnostics,
    })?;
    let receipt_ref = canonical_hash(&receipt_value)?;
    diagnostics.shrink_to_fit();
    Ok(NodeControlLiveWorkflowBundleAckExport {
        receiver_decision: ack.receiver_decision.clone(),
        ack,
        receipt_ref,
        receipt_value,
        decision: decision.to_string(),
        diagnostics,
    })
}

pub fn import_node_control_live_workflow_bundle_ack(
    input: &NodeControlLiveWorkflowBundleAckImportInput<'_>,
) -> Result<NodeControlLiveWorkflowBundleAckImport> {
    validate_live_workflow_bundle_ack_import_input(input)?;
    ensure_state_layout(input.state_root)?;
    let ack = parse_node_control_live_workflow_bundle_ack(input.ack_value)?;
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
    let receipt_ref = canonical_hash(&receipt_value)?;
    import_node_artifact(input.state_root, &receipt_value)?;
    diagnostics.shrink_to_fit();
    Ok(NodeControlLiveWorkflowBundleAckImport {
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

fn validate_live_workflow_bundle_reconcile_input(
    input: &NodeControlLiveWorkflowBundleReconcileInput<'_>,
) -> Result<()> {
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
    input: &NodeControlLiveWorkflowBundleReconcileInput<'_>,
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

fn live_workflow_bundle_reconcile_ingress_diagnostics(
    input: &NodeControlLiveWorkflowBundleReconcileInput<'_>,
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
    if let Some(expected) = input.expected_envelope_ref
        && ingress.envelope_ref != expected
    {
        diagnostics.push(format!(
            "node control live workflow bundle reconcile receiver envelope {} does not match expected {}",
            ingress.envelope_ref, expected
        ));
    }
    if let Some(expected) = artifacts.apply.envelope_ref.as_ref()
        && ingress.envelope_ref != *expected
    {
        diagnostics.push(format!(
            "node control live workflow bundle reconcile receiver envelope {} does not match apply {}",
            ingress.envelope_ref, expected
        ));
    }
    if let Some(send) = artifacts.send
        && ingress.envelope_ref != send.envelope_ref
    {
        diagnostics.push(format!(
            "node control live workflow bundle reconcile receiver envelope {} does not match send {}",
            ingress.envelope_ref, send.envelope_ref
        ));
    }
    if let Some(expected) = input.expected_operation_ref
        && ingress.operation_ref != expected
    {
        diagnostics.push(format!(
            "node control live workflow bundle reconcile receiver operation {} does not match expected {}",
            ingress.operation_ref, expected
        ));
    }
    if let Some(expected) = artifacts.apply.operation_ref.as_ref()
        && ingress.operation_ref != *expected
    {
        diagnostics.push(format!(
            "node control live workflow bundle reconcile receiver operation {} does not match apply {}",
            ingress.operation_ref, expected
        ));
    }
    if let Some(expected) = input.expected_request_ref
        && ingress.request_ref != expected
    {
        diagnostics.push(format!(
            "node control live workflow bundle reconcile receiver request {} does not match expected {}",
            ingress.request_ref, expected
        ));
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
    input: &NodeControlLiveWorkflowBundleAckExportInput<'_>,
    reconciled: &NodeControlLiveWorkflowBundleReconcile,
    reconcile: &NodeControlLiveWorkflowBundleReconcileReceipt,
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
    let ingress = input.ingress_receipt_value.map(parse_node_control_ingress_receipt).transpose()?;
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

fn validate_live_workflow_bundle_ack_import_input(
    input: &NodeControlLiveWorkflowBundleAckImportInput<'_>,
) -> Result<()> {
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
    input: &NodeControlLiveWorkflowBundleAckImportInput<'_>,
    ack: &NodeControlLiveWorkflowBundleAck,
) -> Result<Vec<String>> {
    let recomputed = reconcile_node_control_live_workflow_bundle(&NodeControlLiveWorkflowBundleReconcileInput {
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
        let ingress = parse_node_control_ingress_receipt(ingress_value)?;
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
    ack: &NodeControlLiveWorkflowBundleAck,
) -> Result<Vec<String>> {
    let mut imported_refs = Vec::with_capacity(8);
    imported_refs.push(import_node_artifact(state_root, &ack.apply_receipt_value)?);
    if let Some(value) = ack.send_receipt_value.as_ref() {
        imported_refs.push(import_node_artifact(state_root, value)?);
    }
    if let Some(value) = ack.ingress_receipt_value.as_ref() {
        imported_refs.push(import_node_artifact(state_root, value)?);
    }
    if let Some(value) = ack.queue_receipt_value.as_ref() {
        imported_refs.push(import_node_artifact(state_root, value)?);
    }
    if let Some(value) = ack.control_receipt_value.as_ref() {
        imported_refs.push(import_node_artifact(state_root, value)?);
    }
    imported_refs.push(import_node_artifact(state_root, &ack.reconcile_receipt_value)?);
    imported_refs.push(import_node_artifact(state_root, &ack.ack_value)?);
    Ok(imported_refs)
}

pub fn parse_node_control_live_workflow_bundle_apply_receipt(
    value: &IOValue,
) -> Result<NodeControlLiveWorkflowBundleApplyReceipt> {
    let fields = value
        .collect_simple_record("node-control-live-workflow-bundle-apply-receipt-v1", Some(15))
        .ok_or_else(|| {
            MoltenError::invalid_harness("expected <node-control-live-workflow-bundle-apply-receipt-v1 ...>")
        })?;
    require_schema(
        &fields[0],
        NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_APPLY_RECEIPT_SCHEMA,
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
    Ok(NodeControlLiveWorkflowBundleApplyReceipt {
        receipt_ref: canonical_hash(value)?,
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

pub fn parse_node_control_live_workflow_bundle_reconcile_receipt(
    value: &IOValue,
) -> Result<NodeControlLiveWorkflowBundleReconcileReceipt> {
    let fields = value
        .collect_simple_record("node-control-live-workflow-bundle-reconcile-receipt-v1", Some(13))
        .ok_or_else(|| {
            MoltenError::invalid_harness("expected <node-control-live-workflow-bundle-reconcile-receipt-v1 ...>")
        })?;
    require_schema(
        &fields[0],
        NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_RECONCILE_RECEIPT_SCHEMA,
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
    Ok(NodeControlLiveWorkflowBundleReconcileReceipt {
        receipt_ref: canonical_hash(value)?,
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

pub fn parse_node_control_ingress_receipt(value: &IOValue) -> Result<NodeControlIngressReceipt> {
    let fields = value
        .collect_simple_record("node-control-ingress-receipt-v1", Some(15))
        .ok_or_else(|| MoltenError::invalid_harness("expected <node-control-ingress-receipt-v1 ...>"))?;
    require_schema(&fields[0], NODE_CONTROL_INGRESS_RECEIPT_SCHEMA, "node control ingress receipt")?;
    let idempotency_receipt_ref = record_optional_ref_string(&fields[11], "idempotency")?;
    let queue_receipt_ref = record_optional_ref_string(&fields[12], "queue")?;
    let _checks = record_sequence_len(&fields[14], "checks")?;
    let decision = record_string(&fields[1], "decision")?;
    validate_decision(&decision)?;
    Ok(NodeControlIngressReceipt {
        receipt_ref: canonical_hash(value)?,
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

pub fn parse_node_control_queue_receipt(value: &IOValue) -> Result<NodeControlQueueReceipt> {
    let fields = value
        .collect_simple_record("node-control-queue-receipt-v1", Some(9))
        .ok_or_else(|| MoltenError::invalid_harness("expected <node-control-queue-receipt-v1 ...>"))?;
    require_schema(&fields[0], NODE_CONTROL_QUEUE_RECEIPT_SCHEMA, "node control queue receipt")?;
    let _checks = record_sequence_len(&fields[8], "checks")?;
    let decision = record_string(&fields[1], "decision")?;
    validate_decision(&decision)?;
    Ok(NodeControlQueueReceipt {
        receipt_ref: canonical_hash(value)?,
        decision,
        phase: record_string(&fields[2], "phase")?,
        operation: record_string(&fields[3], "operation")?,
        request_ref: record_ref_string(&fields[4], "request")?,
        location_ref: record_ref_string(&fields[6], "location")?,
        diagnostics: record_strings(&fields[7], "diagnostics")?,
    })
}

pub fn parse_node_control_live_workflow_bundle_verify_receipt(
    value: &IOValue,
) -> Result<NodeControlLiveWorkflowBundleVerifyReceipt> {
    let fields = value
        .collect_simple_record("node-control-live-workflow-bundle-verify-receipt-v1", Some(10))
        .ok_or_else(|| {
            MoltenError::invalid_harness("expected <node-control-live-workflow-bundle-verify-receipt-v1 ...>")
        })?;
    require_schema(
        &fields[0],
        NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_VERIFY_RECEIPT_SCHEMA,
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
    Ok(NodeControlLiveWorkflowBundleVerifyReceipt {
        receipt_ref: canonical_hash(value)?,
        decision,
        bundle_ref: record_ref_string(&fields[2], "bundle")?,
        ticket_ref,
        peer_admission_ref,
        authority_grant_ref,
        receipt_refs: record_ref_strings(&fields[6], "receipts")?,
        diagnostics: record_strings(&fields[8], "diagnostics")?,
    })
}

pub fn parse_node_control_live_workflow_bundle_gate_receipt(
    value: &IOValue,
) -> Result<NodeControlLiveWorkflowBundleGateReceipt> {
    let fields = value
        .collect_simple_record("node-control-live-workflow-bundle-gate-receipt-v1", Some(12))
        .ok_or_else(|| {
            MoltenError::invalid_harness("expected <node-control-live-workflow-bundle-gate-receipt-v1 ...>")
        })?;
    require_schema(
        &fields[0],
        NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_GATE_RECEIPT_SCHEMA,
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
    Ok(NodeControlLiveWorkflowBundleGateReceipt {
        receipt_ref: canonical_hash(value)?,
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

pub fn parse_node_control_live_workflow_bundle(value: &IOValue) -> Result<NodeControlLiveWorkflowBundle> {
    let fields = value
        .collect_simple_record("node-control-live-workflow-bundle-v1", Some(10))
        .ok_or_else(|| MoltenError::invalid_harness("expected <node-control-live-workflow-bundle-v1 ...>"))?;
    require_schema(&fields[0], NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_SCHEMA, "node control live workflow bundle")?;
    let ticket_value = record_value(&fields[1], "ticket")?;
    let peer_admission_value = record_value(&fields[2], "peer-admission")?;
    let authority_grant_value = record_value(&fields[3], "authority-grant")?;
    let receipt_values = record_values(&fields[4], "receipts")?;
    let ticket_ref = record_ref_string(&fields[5], "ticket-ref")?;
    let peer_admission_ref = record_ref_string(&fields[6], "peer-admission-ref")?;
    let authority_grant_ref = record_ref_string(&fields[7], "authority-grant-ref")?;
    let receipt_refs = record_ref_strings(&fields[8], "receipt-refs")?;
    let parsed_ticket = parse_node_control_live_ticket(&ticket_value)?;
    let parsed_admission = parse_node_control_live_peer_admission(&peer_admission_value)?;
    let parsed_authority = parse_node_control_authority_grant(&authority_grant_value)?;
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
    Ok(NodeControlLiveWorkflowBundle {
        bundle_ref: canonical_hash(value)?,
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

pub fn parse_node_control_live_workflow_bundle_ack(value: &IOValue) -> Result<NodeControlLiveWorkflowBundleAck> {
    let fields = value
        .collect_simple_record("node-control-live-workflow-bundle-ack-v1", Some(22))
        .ok_or_else(|| MoltenError::invalid_harness("expected <node-control-live-workflow-bundle-ack-v1 ...>"))?;
    require_schema(&fields[0], NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_ACK_SCHEMA, "node control live workflow bundle ack")?;
    let apply_receipt_value = record_value(&fields[1], "apply-receipt")?;
    let send_receipt_value = record_optional_value(&fields[2], "send-receipt")?;
    let ingress_receipt_value = record_optional_value(&fields[3], "ingress-receipt")?;
    let queue_receipt_value = record_optional_value(&fields[4], "queue-receipt")?;
    let control_receipt_value = record_optional_value(&fields[5], "control-receipt")?;
    let reconcile_receipt_value = record_value(&fields[6], "reconcile-receipt")?;
    let apply_receipt_ref = record_ref_string(&fields[7], "apply-ref")?;
    let send_receipt_ref = record_optional_ref_string(&fields[8], "send-ref")?;
    let ingress_receipt_ref = record_optional_ref_string(&fields[9], "ingress-ref")?;
    let queue_receipt_ref = record_optional_ref_string(&fields[10], "queue-ref")?;
    let control_receipt_ref = record_optional_ref_string(&fields[11], "control-ref")?;
    let reconcile_receipt_ref = record_ref_string(&fields[12], "reconcile-ref")?;
    let bundle_ref = record_ref_string(&fields[13], "bundle")?;
    let envelope_ref = record_optional_ref_string(&fields[14], "envelope")?;
    let operation_ref = record_optional_ref_string(&fields[15], "operation")?;
    let request_ref = record_optional_ref_string(&fields[16], "request")?;
    let receiver_decision = record_string(&fields[17], "receiver-decision")?;
    validate_decision(&receiver_decision)?;
    let receiver_diagnostics = record_strings(&fields[18], "receiver-diagnostics")?;
    let diagnostics = record_strings(&fields[19], "diagnostics")?;
    let _checks = record_sequence_len(&fields[20], "checks")?;
    let _member_refs = record_sequence_len(&fields[21], "member-refs")?;
    let apply = parse_node_control_live_workflow_bundle_apply_receipt(&apply_receipt_value)?;
    let reconcile = parse_node_control_live_workflow_bundle_reconcile_receipt(&reconcile_receipt_value)?;
    if let Some(value) = send_receipt_value.as_ref() {
        parse_node_control_live_send_receipt(value)?;
    }
    if let Some(value) = ingress_receipt_value.as_ref() {
        parse_node_control_ingress_receipt(value)?;
    }
    if let Some(value) = queue_receipt_value.as_ref() {
        parse_node_control_queue_receipt(value)?;
    }
    if let Some(value) = control_receipt_value.as_ref() {
        node_runtime::parse_node_control_receipt(value)?;
    }
    validate_member_ref(&apply.receipt_ref, &apply_receipt_ref, "ack apply receipt")?;
    validate_member_ref(&reconcile.receipt_ref, &reconcile_receipt_ref, "ack reconcile receipt")?;
    validate_optional_member_ref(send_receipt_value.as_ref(), send_receipt_ref.as_deref(), "ack send receipt")?;
    validate_optional_member_ref(
        ingress_receipt_value.as_ref(),
        ingress_receipt_ref.as_deref(),
        "ack ingress receipt",
    )?;
    validate_optional_member_ref(queue_receipt_value.as_ref(), queue_receipt_ref.as_deref(), "ack queue receipt")?;
    validate_optional_member_ref(
        control_receipt_value.as_ref(),
        control_receipt_ref.as_deref(),
        "ack control receipt",
    )?;
    if reconcile.apply_receipt_ref != apply_receipt_ref {
        return Err(MoltenError::invalid_harness("node control live workflow bundle ack apply ref mismatch"));
    }
    if reconcile.bundle_ref != bundle_ref {
        return Err(MoltenError::invalid_harness("node control live workflow bundle ack bundle ref mismatch"));
    }
    if reconcile.send_receipt_ref != send_receipt_ref {
        return Err(MoltenError::invalid_harness("node control live workflow bundle ack send ref mismatch"));
    }
    if reconcile.ingress_receipt_ref != ingress_receipt_ref {
        return Err(MoltenError::invalid_harness("node control live workflow bundle ack ingress ref mismatch"));
    }
    if reconcile.queue_receipt_ref != queue_receipt_ref {
        return Err(MoltenError::invalid_harness("node control live workflow bundle ack queue ref mismatch"));
    }
    if reconcile.control_receipt_ref != control_receipt_ref {
        return Err(MoltenError::invalid_harness("node control live workflow bundle ack control ref mismatch"));
    }
    if reconcile.envelope_ref != envelope_ref {
        return Err(MoltenError::invalid_harness("node control live workflow bundle ack envelope ref mismatch"));
    }
    if reconcile.operation_ref != operation_ref {
        return Err(MoltenError::invalid_harness("node control live workflow bundle ack operation ref mismatch"));
    }
    if reconcile.request_ref != request_ref {
        return Err(MoltenError::invalid_harness("node control live workflow bundle ack request ref mismatch"));
    }
    if reconcile.decision != receiver_decision {
        return Err(MoltenError::invalid_harness("node control live workflow bundle ack receiver decision mismatch"));
    }
    if reconcile.diagnostics != receiver_diagnostics {
        return Err(MoltenError::invalid_harness(
            "node control live workflow bundle ack receiver diagnostics mismatch",
        ));
    }
    Ok(NodeControlLiveWorkflowBundleAck {
        ack_ref: canonical_hash(value)?,
        ack_value: value.clone(),
        apply_receipt_ref,
        send_receipt_ref,
        ingress_receipt_ref,
        queue_receipt_ref,
        control_receipt_ref,
        reconcile_receipt_ref,
        bundle_ref,
        envelope_ref,
        operation_ref,
        request_ref,
        receiver_decision,
        receiver_diagnostics,
        diagnostics,
        apply_receipt_value,
        send_receipt_value,
        ingress_receipt_value,
        queue_receipt_value,
        control_receipt_value,
        reconcile_receipt_value,
    })
}

pub fn import_node_control_live_workflow_bundle(
    input: &NodeControlLiveWorkflowBundleImportInput<'_>,
) -> Result<NodeControlLiveWorkflowBundleImport> {
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
    let bundle = parse_node_control_live_workflow_bundle(input.bundle_value)?;
    let ticket = parse_node_control_live_ticket(&bundle.ticket_value)?;
    let admission = parse_node_control_live_peer_admission(&bundle.peer_admission_value)?;
    let authority = parse_node_control_authority_grant(&bundle.authority_grant_value)?;
    let mut diagnostics = live_workflow_bundle_import_diagnostics(input, &ticket, &admission, &authority);
    let mut imported_refs = Vec::with_capacity(bundle.receipt_values.len().saturating_add(5));
    let mut ticket_import_ref = None;
    let mut authority_import_ref = None;
    if diagnostics.is_empty() {
        let ticket_import = import_node_control_live_ticket(&NodeControlLiveTicketImportInput {
            state_root: input.state_root,
            ticket_value: &bundle.ticket_value,
            peer_admission_value: Some(&bundle.peer_admission_value),
            expected_node: input.expected_node,
            expected_topic: input.expected_topic,
            expected_endpoint: input.expected_endpoint,
            expected_peer: input.expected_peer,
            as_of_sequence: input.as_of_sequence,
        })?;
        let authority_import = import_node_control_authority_grant_checked(&NodeControlAuthorityGrantImportInput {
            state_root: input.state_root,
            grant_value: &bundle.authority_grant_value,
            expected_peer: input.expected_peer,
            expected_node: input.expected_node,
            expected_operations: input.expected_operations,
            expected_target_scope: input.expected_target_scope,
            expected_resource_scope: input.expected_resource_scope,
            as_of_epoch: input.as_of_epoch,
        })?;
        ticket_import_ref = Some(ticket_import.receipt_ref.clone());
        authority_import_ref = Some(authority_import.receipt_ref.clone());
        if ticket_import.decision != "pass" {
            diagnostics.extend(ticket_import.diagnostics);
        }
        if authority_import.decision != "pass" {
            diagnostics.extend(authority_import.diagnostics);
        }
        if diagnostics.is_empty() {
            imported_refs.extend(ticket_import.imported_refs);
            imported_refs.extend(authority_import.imported_refs);
            imported_refs.push(import_node_artifact(input.state_root, input.bundle_value)?);
            for receipt_value in &bundle.receipt_values {
                imported_refs.push(import_node_artifact(input.state_root, receipt_value)?);
            }
        }
    }
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let receipt_value = live_workflow_bundle_import_receipt_value(&LiveWorkflowBundleImportReceiptValueInput {
        decision,
        state_root: input.state_root,
        bundle: &bundle,
        ticket_import_ref: ticket_import_ref.as_deref(),
        authority_import_ref: authority_import_ref.as_deref(),
        imported_refs: &imported_refs,
        diagnostics: &diagnostics,
    })?;
    let receipt_ref = canonical_hash(&receipt_value)?;
    import_node_artifact(input.state_root, &receipt_value)?;
    Ok(NodeControlLiveWorkflowBundleImport {
        bundle_ref: bundle.bundle_ref,
        ticket_import_ref,
        authority_import_ref,
        imported_refs,
        diagnostics,
        receipt_ref,
        receipt_value,
        decision: decision.to_string(),
    })
}

fn validate_live_workflow_bundle_verify_input(input: &NodeControlLiveWorkflowBundleVerifyInput<'_>) -> Result<()> {
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

fn validate_live_workflow_bundle_apply_input(input: &NodeControlLiveWorkflowBundleApplyInput<'_>) -> Result<()> {
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
    input: &'a NodeControlLiveWorkflowBundleVerifyInput<'a>,
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
    input: &'a NodeControlLiveWorkflowBundleGateInput<'a>,
) -> NodeControlLiveWorkflowBundleVerifyInput<'a> {
    NodeControlLiveWorkflowBundleVerifyInput {
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
    input: &'a NodeControlLiveWorkflowBundleApplyInput<'a>,
) -> NodeControlLiveWorkflowBundleVerifyInput<'a> {
    NodeControlLiveWorkflowBundleVerifyInput {
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
    input: &'a NodeControlLiveWorkflowBundleApplyInput<'a>,
) -> NodeControlLiveWorkflowBundleImportInput<'a> {
    NodeControlLiveWorkflowBundleImportInput {
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
    input: &'a NodeControlLiveWorkflowBundleImportInput<'a>,
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
    input: &NodeControlLiveWorkflowBundleImportInput<'_>,
    ticket: &NodeControlLiveTicket,
    admission: &NodeControlLivePeerAdmission,
    authority: &NodeControlAuthorityGrant,
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
    ticket: &NodeControlLiveTicket,
    admission: &NodeControlLivePeerAdmission,
    authority: &NodeControlAuthorityGrant,
) -> Vec<String> {
    let mut diagnostics = live_workflow_bundle_binding_diagnostics(ticket, admission, authority);
    diagnostics.extend(live_ticket_expected_diagnostics(input, ticket, admission));
    diagnostics.extend(authority_grant_expected_diagnostics(input, authority));
    diagnostics
}

fn live_ticket_expected_diagnostics(
    input: &LiveWorkflowBundleExpectedInput<'_>,
    ticket: &NodeControlLiveTicket,
    admission: &NodeControlLivePeerAdmission,
) -> Vec<String> {
    live_ticket_import_diagnostics(
        &NodeControlLiveTicketImportInput {
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
    authority: &NodeControlAuthorityGrant,
) -> Vec<String> {
    authority_grant_import_diagnostics(
        &NodeControlAuthorityGrantImportInput {
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
    ticket: &NodeControlLiveTicket,
    admission: &NodeControlLivePeerAdmission,
    authority: &NodeControlAuthorityGrant,
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

fn live_workflow_bundle_receipt_refs(values: &[&IOValue]) -> Result<Vec<String>> {
    let owned_values = values.iter().map(|value| (**value).clone()).collect::<Vec<_>>();
    live_workflow_bundle_receipt_refs_from_values(&owned_values)
}

fn live_workflow_bundle_receipt_refs_from_values(values: &[IOValue]) -> Result<Vec<String>> {
    let mut refs = Vec::with_capacity(values.len());
    for value in values {
        refs.push(canonical_hash(value)?);
    }
    Ok(refs)
}

fn live_workflow_bundle_receipt_diagnostics(values: &[&IOValue]) -> Vec<String> {
    let mut diagnostics = Vec::with_capacity(values.len());
    for value in values {
        let kind = ledger::artifact_kind(value);
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
    input: &NodeControlLiveTicketImportInput<'_>,
    ticket: &NodeControlLiveTicket,
    admission: Option<&NodeControlLivePeerAdmission>,
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
    input: &NodeControlLiveTicketImportInput<'_>,
    ticket: &NodeControlLiveTicket,
    admission: &NodeControlLivePeerAdmission,
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
    input: &NodeControlAuthorityGrantImportInput<'_>,
    grant: &NodeControlAuthorityGrant,
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

fn live_ticket_import_receipt_value(input: &LiveTicketImportReceiptValueInput<'_>) -> Result<IOValue> {
    validate_decision(input.decision)?;
    let binding_status = if input.decision == "pass" { "pass" } else { "fail" };
    Ok(record("node-control-live-ticket-import-receipt-v1", vec![
        string(NODE_CONTROL_LIVE_TICKET_IMPORT_RECEIPT_SCHEMA),
        record("decision", vec![string(input.decision)]),
        record("state-root", vec![string(&state_root_profile_ref(input.state_root)?)]),
        record("ticket", vec![string(&input.ticket.ticket_ref)]),
        record("node", vec![string(&input.ticket.node_id)]),
        record("topic", vec![string(&input.ticket.topic)]),
        record("endpoint", vec![string(&input.ticket.live_endpoint_id)]),
        record("peer-admission", vec![optional_string(input.peer_admission_ref)]),
        record("peer", vec![optional_string(input.peer_id)]),
        record("as-of-sequence", vec![string(input.as_of_sequence.to_string())]),
        record("imported", vec![sequence(input.imported_refs.iter().map(string).collect())]),
        record("diagnostics", vec![sequence(input.diagnostics.iter().map(string).collect())]),
        record("checks", vec![sequence(vec![
            record("check", vec![string("ticket-kind-version"), string("pass")]),
            record("check", vec![string("ticket-topic-endpoint-bound"), string(binding_status)]),
            record("check", vec![string("peer-admission-kind-version"), string(binding_status)]),
            record("check", vec![string("import-receipt-is-not-authority"), string("pass")]),
            record("check", vec![string("provenance-still-required"), string("pass")]),
        ])]),
    ]))
}

fn authority_grant_import_receipt_value(input: &AuthorityGrantImportReceiptValueInput<'_>) -> Result<IOValue> {
    validate_decision(input.decision)?;
    let binding_status = if input.decision == "pass" { "pass" } else { "fail" };
    Ok(record("node-control-authority-grant-import-receipt-v1", vec![
        string(NODE_CONTROL_AUTHORITY_GRANT_IMPORT_RECEIPT_SCHEMA),
        record("decision", vec![string(input.decision)]),
        record("state-root", vec![string(&state_root_profile_ref(input.state_root)?)]),
        record("grant", vec![string(&input.grant.grant_ref)]),
        record("peer", vec![string(&input.grant.peer_id)]),
        record("node", vec![string(&input.grant.node_id)]),
        record("operations", vec![sequence(input.grant.operations.iter().map(string).collect())]),
        record("target-scope", vec![string(&input.grant.target_scope)]),
        record("resource-scope", vec![string(&input.grant.resource_scope)]),
        record("as-of-epoch", vec![string(input.as_of_epoch.to_string())]),
        record("imported", vec![sequence(input.imported_refs.iter().map(string).collect())]),
        record("diagnostics", vec![sequence(input.diagnostics.iter().map(string).collect())]),
        record("checks", vec![sequence(vec![
            record("check", vec![string("grant-kind-version"), string("pass")]),
            record("check", vec![string("peer-node-operation-scope-bound"), string(binding_status)]),
            record("check", vec![string("grant-fresh-and-unrevoked"), string(binding_status)]),
            record("check", vec![string("import-receipt-is-not-authority"), string("pass")]),
            record("check", vec![string("provenance-still-required"), string("pass")]),
        ])]),
    ]))
}

fn live_workflow_bundle_value(input: &LiveWorkflowBundleValueInput<'_>) -> Result<IOValue> {
    let binding_status = if input.diagnostics.is_empty() { "pass" } else { "fail" };
    let receipt_refs = live_workflow_bundle_receipt_refs(input.receipt_values)?;
    Ok(record("node-control-live-workflow-bundle-v1", vec![
        string(NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_SCHEMA),
        record("ticket", vec![(*input.ticket_value).clone()]),
        record("peer-admission", vec![(*input.admission_value).clone()]),
        record("authority-grant", vec![(*input.authority_value).clone()]),
        record("receipts", vec![sequence(
            input.receipt_values.iter().map(|value| (**value).clone()).collect(),
        )]),
        record("ticket-ref", vec![string(&input.ticket.ticket_ref)]),
        record("peer-admission-ref", vec![string(&input.admission.admission_ref)]),
        record("authority-grant-ref", vec![string(&input.authority.grant_ref)]),
        record("receipt-refs", vec![sequence(receipt_refs.iter().map(string).collect())]),
        record("checks", vec![sequence(vec![
            record("check", vec![string("ticket-kind-version"), string("pass")]),
            record("check", vec![string("peer-admission-kind-version"), string("pass")]),
            record("check", vec![string("authority-grant-kind-version"), string("pass")]),
            record("check", vec![string("ticket-admission-bound"), string(binding_status)]),
            record("check", vec![string("authority-grant-bound"), string(binding_status)]),
            record("check", vec![string("bundle-is-not-authority"), string("pass")]),
            record("check", vec![string("provenance-still-required"), string("pass")]),
        ])]),
    ]))
}

fn live_workflow_bundle_export_receipt_value(input: &LiveWorkflowBundleExportReceiptValueInput<'_>) -> Result<IOValue> {
    validate_decision(input.decision)?;
    let binding_status = if input.decision == "pass" { "pass" } else { "fail" };
    Ok(record("node-control-live-workflow-bundle-export-receipt-v1", vec![
        string(NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_EXPORT_RECEIPT_SCHEMA),
        record("decision", vec![string(input.decision)]),
        record("bundle", vec![string(&input.bundle.bundle_ref)]),
        record("ticket", vec![string(&input.bundle.ticket_ref)]),
        record("peer-admission", vec![string(&input.bundle.peer_admission_ref)]),
        record("authority-grant", vec![string(&input.bundle.authority_grant_ref)]),
        record("receipts", vec![sequence(input.bundle.receipt_refs.iter().map(string).collect())]),
        record("diagnostics", vec![sequence(input.diagnostics.iter().map(string).collect())]),
        record("checks", vec![sequence(vec![
            record("check", vec![string("bundle-kind-version"), string("pass")]),
            record("check", vec![string("bundle-member-bindings"), string(binding_status)]),
            record("check", vec![string("bundle-receipt-kinds"), string(binding_status)]),
            record("check", vec![string("bundle-is-not-authority"), string("pass")]),
            record("check", vec![string("provenance-still-required"), string("pass")]),
        ])]),
    ]))
}

fn live_workflow_bundle_verify_receipt_value(input: &LiveWorkflowBundleVerifyReceiptValueInput<'_>) -> Result<IOValue> {
    validate_decision(input.decision)?;
    let binding_status = if input.decision == "pass" { "pass" } else { "fail" };
    Ok(record("node-control-live-workflow-bundle-verify-receipt-v1", vec![
        string(NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_VERIFY_RECEIPT_SCHEMA),
        record("decision", vec![string(input.decision)]),
        record("bundle", vec![string(input.bundle_ref)]),
        record("ticket", vec![optional_string(input.ticket_ref)]),
        record("peer-admission", vec![optional_string(input.peer_admission_ref)]),
        record("authority-grant", vec![optional_string(input.authority_grant_ref)]),
        record("receipts", vec![sequence(input.receipt_refs.iter().map(string).collect())]),
        record("expected", vec![live_workflow_bundle_expected_value(input.expected)]),
        record("diagnostics", vec![sequence(input.diagnostics.iter().map(string).collect())]),
        record("checks", vec![sequence(vec![
            record("check", vec![string("bundle-kind-version"), string(binding_status)]),
            record("check", vec![string("bundle-member-bindings"), string(binding_status)]),
            record("check", vec![string("bundle-receipt-kinds"), string(binding_status)]),
            record("check", vec![string("expected-bindings"), string(binding_status)]),
            record("check", vec![string("verify-receipt-is-not-authority"), string("pass")]),
            record("check", vec![string("provenance-still-required"), string("pass")]),
        ])]),
    ]))
}

fn live_workflow_bundle_gate_receipt_value(input: &LiveWorkflowBundleGateReceiptValueInput<'_>) -> Result<IOValue> {
    validate_decision(input.decision)?;
    let gate_status = if input.decision == "pass" { "pass" } else { "fail" };
    Ok(record("node-control-live-workflow-bundle-gate-receipt-v1", vec![
        string(NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_GATE_RECEIPT_SCHEMA),
        record("decision", vec![string(input.decision)]),
        record("bundle", vec![string(input.bundle_ref)]),
        record("verify-receipt", vec![optional_string(input.verify_receipt_ref)]),
        record("recomputed-verify", vec![string(input.recomputed_verify_receipt_ref)]),
        record("ticket", vec![optional_string(input.ticket_ref)]),
        record("peer-admission", vec![optional_string(input.peer_admission_ref)]),
        record("authority-grant", vec![optional_string(input.authority_grant_ref)]),
        record("receipts", vec![sequence(input.receipt_refs.iter().map(string).collect())]),
        record("expected", vec![live_workflow_bundle_expected_value(input.expected)]),
        record("diagnostics", vec![sequence(input.diagnostics.iter().map(string).collect())]),
        record("checks", vec![sequence(vec![
            record("check", vec![string("bundle-verification"), string(gate_status)]),
            record("check", vec![string("verify-receipt-current"), string(gate_status)]),
            record("check", vec![string("expected-bindings"), string(gate_status)]),
            record("check", vec![string("gate-receipt-is-not-authority"), string("pass")]),
            record("check", vec![string("bundle-import-still-required"), string("pass")]),
            record("check", vec![string("provenance-still-required"), string("pass")]),
        ])]),
    ]))
}

fn live_workflow_bundle_apply_receipt_value(input: &LiveWorkflowBundleApplyReceiptValueInput<'_>) -> Result<IOValue> {
    validate_decision(input.decision)?;
    let apply_status = if input.decision == "pass" { "pass" } else { "fail" };
    Ok(record("node-control-live-workflow-bundle-apply-receipt-v1", vec![
        string(NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_APPLY_RECEIPT_SCHEMA),
        record("decision", vec![string(input.decision)]),
        record("state-root", vec![string(input.state_root.display().to_string())]),
        record("bundle", vec![string(input.bundle_ref)]),
        record("gate-receipt", vec![optional_string(input.gate_receipt_ref)]),
        record("recomputed-verify", vec![string(input.recomputed_verify_receipt_ref)]),
        record("import-receipt", vec![optional_string(input.import_receipt_ref)]),
        record("imported", vec![sequence(input.imported_refs.iter().map(string).collect())]),
        record("mode", vec![string(input.mode)]),
        record("envelope", vec![optional_string(input.envelope_ref)]),
        record("operation", vec![optional_string(input.operation_ref)]),
        record("send-receipt", vec![optional_string(input.send_receipt_ref)]),
        record("expected", vec![live_workflow_bundle_expected_value(input.expected)]),
        record("diagnostics", vec![sequence(input.diagnostics.iter().map(string).collect())]),
        record("checks", vec![sequence(vec![
            record("check", vec![string("bundle-verification"), string(apply_status)]),
            record("check", vec![string("gate-receipt-current"), string(apply_status)]),
            record("check", vec![string("bundle-imported"), string(apply_status)]),
            record("check", vec![string("send-preflight-or-dispatch"), string(apply_status)]),
            record("check", vec![string("apply-receipt-is-not-authority"), string("pass")]),
            record("check", vec![string("provenance-still-required"), string("pass")]),
        ])]),
    ]))
}

fn live_workflow_bundle_reconcile_receipt_value(
    input: &LiveWorkflowBundleReconcileReceiptValueInput<'_>,
) -> Result<IOValue> {
    validate_decision(input.decision)?;
    let reconcile_status = if input.decision == "pass" { "pass" } else { "fail" };
    Ok(record("node-control-live-workflow-bundle-reconcile-receipt-v1", vec![
        string(NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_RECONCILE_RECEIPT_SCHEMA),
        record("decision", vec![string(input.decision)]),
        record("apply-receipt", vec![string(input.apply_receipt_ref)]),
        record("bundle", vec![string(input.bundle_ref)]),
        record("send-receipt", vec![optional_string(input.send_receipt_ref)]),
        record("ingress-receipt", vec![optional_string(input.ingress_receipt_ref)]),
        record("queue-receipt", vec![optional_string(input.queue_receipt_ref)]),
        record("control-receipt", vec![optional_string(input.control_receipt_ref)]),
        record("envelope", vec![optional_string(input.envelope_ref)]),
        record("operation", vec![optional_string(input.operation_ref)]),
        record("request", vec![optional_string(input.request_ref)]),
        record("diagnostics", vec![sequence(input.diagnostics.iter().map(string).collect())]),
        record("checks", vec![sequence(vec![
            record("check", vec![string("apply-receipt-bound"), string(reconcile_status)]),
            record("check", vec![string("send-receipt-current"), string(reconcile_status)]),
            record("check", vec![string("receiver-ingress-bound"), string(reconcile_status)]),
            record("check", vec![string("durable-enqueue-or-deny"), string(reconcile_status)]),
            record("check", vec![string("control-dispatch-bound"), string(reconcile_status)]),
            record("check", vec![string("reconcile-receipt-is-not-authority"), string("pass")]),
            record("check", vec![string("provenance-still-required"), string("pass")]),
        ])]),
    ]))
}

fn live_workflow_bundle_ack_value(input: &LiveWorkflowBundleAckValueInput<'_>) -> Result<IOValue> {
    validate_decision(input.receiver_decision)?;
    Ok(record("node-control-live-workflow-bundle-ack-v1", vec![
        string(NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_ACK_SCHEMA),
        record("apply-receipt", vec![input.apply_receipt_value.clone()]),
        record("send-receipt", vec![optional_value(input.send_receipt_value)]),
        record("ingress-receipt", vec![optional_value(input.ingress_receipt_value)]),
        record("queue-receipt", vec![optional_value(input.queue_receipt_value)]),
        record("control-receipt", vec![optional_value(input.control_receipt_value)]),
        record("reconcile-receipt", vec![input.reconcile_receipt_value.clone()]),
        record("apply-ref", vec![string(input.apply_receipt_ref)]),
        record("send-ref", vec![optional_string(input.send_receipt_ref)]),
        record("ingress-ref", vec![optional_string(input.ingress_receipt_ref)]),
        record("queue-ref", vec![optional_string(input.queue_receipt_ref)]),
        record("control-ref", vec![optional_string(input.control_receipt_ref)]),
        record("reconcile-ref", vec![string(input.reconcile_receipt_ref)]),
        record("bundle", vec![string(input.bundle_ref)]),
        record("envelope", vec![optional_string(input.envelope_ref)]),
        record("operation", vec![optional_string(input.operation_ref)]),
        record("request", vec![optional_string(input.request_ref)]),
        record("receiver-decision", vec![string(input.receiver_decision)]),
        record("receiver-diagnostics", vec![sequence(input.receiver_diagnostics.iter().map(string).collect())]),
        record("diagnostics", vec![sequence(input.diagnostics.iter().map(string).collect())]),
        record("checks", vec![sequence(vec![
            record("check", vec![string("ack-member-refs-bound"), string("pass")]),
            record("check", vec![string("receiver-outcome-recorded"), string("pass")]),
            record("check", vec![string("ack-bundle-is-not-authority"), string("pass")]),
            record("check", vec![string("provenance-still-required"), string("pass")]),
        ])]),
        record("member-refs", vec![sequence(
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
            .map(string)
            .collect(),
        )]),
    ]))
}

fn live_workflow_bundle_ack_export_receipt_value(
    input: &LiveWorkflowBundleAckExportReceiptValueInput<'_>,
) -> Result<IOValue> {
    validate_decision(input.decision)?;
    let ack_status = if input.decision == "pass" { "pass" } else { "fail" };
    Ok(record("node-control-live-workflow-bundle-ack-export-receipt-v1", vec![
        string(NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_ACK_EXPORT_RECEIPT_SCHEMA),
        record("decision", vec![string(input.decision)]),
        record("ack", vec![string(&input.ack.ack_ref)]),
        record("bundle", vec![string(&input.ack.bundle_ref)]),
        record("apply-receipt", vec![string(&input.ack.apply_receipt_ref)]),
        record("send-receipt", vec![optional_string(input.ack.send_receipt_ref.as_deref())]),
        record("ingress-receipt", vec![optional_string(input.ack.ingress_receipt_ref.as_deref())]),
        record("queue-receipt", vec![optional_string(input.ack.queue_receipt_ref.as_deref())]),
        record("control-receipt", vec![optional_string(input.ack.control_receipt_ref.as_deref())]),
        record("reconcile-receipt", vec![string(&input.ack.reconcile_receipt_ref)]),
        record("envelope", vec![optional_string(input.ack.envelope_ref.as_deref())]),
        record("operation", vec![optional_string(input.ack.operation_ref.as_deref())]),
        record("request", vec![optional_string(input.ack.request_ref.as_deref())]),
        record("receiver-decision", vec![string(&input.ack.receiver_decision)]),
        record("receiver-diagnostics", vec![sequence(input.ack.receiver_diagnostics.iter().map(string).collect())]),
        record("diagnostics", vec![sequence(input.diagnostics.iter().map(string).collect())]),
        record("checks", vec![sequence(vec![
            record("check", vec![string("ack-bundle-kind-version"), string("pass")]),
            record("check", vec![string("receiver-evidence-packaged"), string(ack_status)]),
            record("check", vec![string("reconcile-receipt-current"), string(ack_status)]),
            record("check", vec![string("ack-export-is-not-authority"), string("pass")]),
            record("check", vec![string("provenance-still-required"), string("pass")]),
        ])]),
    ]))
}

fn live_workflow_bundle_ack_import_receipt_value(
    input: &LiveWorkflowBundleAckImportReceiptValueInput<'_>,
) -> Result<IOValue> {
    validate_decision(input.decision)?;
    let ack_status = if input.decision == "pass" { "pass" } else { "fail" };
    Ok(record("node-control-live-workflow-bundle-ack-import-receipt-v1", vec![
        string(NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_ACK_IMPORT_RECEIPT_SCHEMA),
        record("decision", vec![string(input.decision)]),
        record("state-root", vec![string(input.state_root.display().to_string())]),
        record("ack", vec![string(&input.ack.ack_ref)]),
        record("bundle", vec![string(&input.ack.bundle_ref)]),
        record("imported", vec![sequence(input.imported_refs.iter().map(string).collect())]),
        record("receiver-decision", vec![string(&input.ack.receiver_decision)]),
        record("receiver-diagnostics", vec![sequence(input.ack.receiver_diagnostics.iter().map(string).collect())]),
        record("diagnostics", vec![sequence(input.diagnostics.iter().map(string).collect())]),
        record("checks", vec![sequence(vec![
            record("check", vec![string("ack-bundle-kind-version"), string("pass")]),
            record("check", vec![string("ack-member-bindings"), string(ack_status)]),
            record("check", vec![string("sender-ledger-imported"), string(ack_status)]),
            record("check", vec![string("ack-import-is-not-authority"), string("pass")]),
            record("check", vec![string("provenance-still-required"), string("pass")]),
        ])]),
    ]))
}

fn live_workflow_bundle_expected_value(input: &LiveWorkflowBundleExpectedInput<'_>) -> IOValue {
    record("expected", vec![sequence(vec![
        record("node", vec![optional_string(input.expected_node)]),
        record("topic", vec![optional_string(input.expected_topic)]),
        record("endpoint", vec![optional_string(input.expected_endpoint)]),
        record("peer", vec![optional_string(input.expected_peer)]),
        record("operations", vec![sequence(input.expected_operations.iter().map(string).collect())]),
        record("target-scope", vec![optional_string(input.expected_target_scope)]),
        record("resource-scope", vec![optional_string(input.expected_resource_scope)]),
        record("as-of-sequence", vec![string(input.as_of_sequence.to_string())]),
        record("as-of-epoch", vec![string(input.as_of_epoch.to_string())]),
    ])])
}

fn live_workflow_bundle_import_receipt_value(input: &LiveWorkflowBundleImportReceiptValueInput<'_>) -> Result<IOValue> {
    validate_decision(input.decision)?;
    let binding_status = if input.decision == "pass" { "pass" } else { "fail" };
    Ok(record("node-control-live-workflow-bundle-import-receipt-v1", vec![
        string(NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_IMPORT_RECEIPT_SCHEMA),
        record("decision", vec![string(input.decision)]),
        record("state-root", vec![string(&state_root_profile_ref(input.state_root)?)]),
        record("bundle", vec![string(&input.bundle.bundle_ref)]),
        record("ticket", vec![string(&input.bundle.ticket_ref)]),
        record("peer-admission", vec![string(&input.bundle.peer_admission_ref)]),
        record("authority-grant", vec![string(&input.bundle.authority_grant_ref)]),
        record("ticket-import", vec![optional_string(input.ticket_import_ref)]),
        record("authority-import", vec![optional_string(input.authority_import_ref)]),
        record("imported", vec![sequence(input.imported_refs.iter().map(string).collect())]),
        record("diagnostics", vec![sequence(input.diagnostics.iter().map(string).collect())]),
        record("checks", vec![sequence(vec![
            record("check", vec![string("bundle-kind-version"), string("pass")]),
            record("check", vec![string("ticket-admission-imported"), string(binding_status)]),
            record("check", vec![string("authority-grant-imported"), string(binding_status)]),
            record("check", vec![string("bundle-receipt-imported"), string(binding_status)]),
            record("check", vec![string("bundle-import-is-not-authority"), string("pass")]),
            record("check", vec![string("provenance-still-required"), string("pass")]),
        ])]),
    ]))
}

pub fn node_control_supervisor_policy_value(input: &NodeControlSupervisorPolicyInput<'_>) -> Result<IOValue> {
    validate_supervisor_policy_bounds(input.max_restarts, "max restarts")?;
    validate_supervisor_policy_bounds(input.restart_window_ticks, "restart window ticks")?;
    validate_supervisor_policy_bounds(input.heartbeat_timeout_ticks, "heartbeat timeout ticks")?;
    validate_supervisor_policy_bounds(input.shutdown_drain_ticks, "shutdown drain ticks")?;
    validate_ingress_refs(input.policy_refs, "node control supervisor policy ref")?;
    validate_ingress_refs(input.evidence_refs, "node control supervisor evidence ref")?;
    Ok(record("node-control-supervisor-policy-v1", vec![
        string(NODE_CONTROL_SUPERVISOR_POLICY_SCHEMA),
        record("max-restarts", vec![string(input.max_restarts.to_string())]),
        record("restart-window-ticks", vec![string(input.restart_window_ticks.to_string())]),
        record("heartbeat-timeout-ticks", vec![string(input.heartbeat_timeout_ticks.to_string())]),
        record("shutdown-drain-ticks", vec![string(input.shutdown_drain_ticks.to_string())]),
        record("stale-lock-recovery", vec![string(if input.stale_lock_recovery { "allow" } else { "deny" })]),
        record("policy", vec![sequence(input.policy_refs.iter().map(string).collect())]),
        record("evidence", vec![sequence(input.evidence_refs.iter().map(string).collect())]),
        record("checks", vec![sequence(vec![
            record("check", vec![string("bounded-restarts"), string("pass")]),
            record("check", vec![string("bounded-heartbeat-timeout"), string("pass")]),
            record("check", vec![string("explicit-stale-lock-policy"), string("pass")]),
            record("check", vec![string("shutdown-drain-bound"), string("pass")]),
        ])]),
    ]))
}

pub fn parse_node_control_supervisor_policy(value: &IOValue) -> Result<NodeControlSupervisorPolicy> {
    let fields = value
        .collect_simple_record("node-control-supervisor-policy-v1", Some(9))
        .ok_or_else(|| MoltenError::invalid_harness("expected <node-control-supervisor-policy-v1 ...>"))?;
    require_schema(&fields[0], NODE_CONTROL_SUPERVISOR_POLICY_SCHEMA, "node control supervisor policy")?;
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
    Ok(NodeControlSupervisorPolicy {
        policy_ref: canonical_hash(value)?,
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

pub fn import_node_control_supervisor_policy(
    state_root: &Path,
    policy_value: &IOValue,
) -> Result<NodeControlSupervisorPolicy> {
    validate_state_root(state_root)?;
    ensure_state_layout(state_root)?;
    let policy = parse_node_control_supervisor_policy(policy_value)?;
    import_node_artifact(state_root, policy_value)?;
    Ok(policy)
}

fn parse_node_control_supervisor_receipt(value: &IOValue) -> Result<NodeControlSupervisorReceipt> {
    let fields = value
        .collect_simple_record("node-control-supervisor-receipt-v1", Some(9))
        .ok_or_else(|| MoltenError::invalid_harness("expected <node-control-supervisor-receipt-v1 ...>"))?;
    require_schema(&fields[0], NODE_CONTROL_SUPERVISOR_RECEIPT_SCHEMA, "node control supervisor receipt")?;
    Ok(NodeControlSupervisorReceipt {
        receipt_ref: canonical_hash(value)?,
        decision: record_string(&fields[1], "decision")?,
        operation: record_string(&fields[2], "operation")?,
        supervisor_policy_ref: record_optional_string(&fields[5], "policy")?,
        diagnostics: record_strings(&fields[7], "diagnostics")?,
        value: value.clone(),
    })
}

fn service_run_supervisor_policy_ref(value: &IOValue) -> Result<Option<String>> {
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

pub fn init_local_node(input: &NodeDaemonInitInput<'_>) -> Result<NodeDaemonInit> {
    validate_state_root(input.state_root)?;
    validate_node_id(input.node_id)?;
    ensure_state_layout(input.state_root)?;
    let policy_refs = vec![local_ref("node-policy", input.node_id)?];
    let identity_config = node_identity::NodeIdentityConfig {
        node_id: input.node_id.to_string(),
        display_name: input.node_id.to_string(),
        data_dir: input.state_root.join("identity"),
        explicit_key: None,
        allow_generate: true,
        allow_rotation: false,
        policy_refs: policy_refs.clone(),
    };
    let identity_resolution = node_identity::resolve_node_identity(&identity_config)?;
    let identity = identity_resolution
        .identity
        .ok_or_else(|| MoltenError::invalid_harness("node daemon identity resolution denied"))?;
    let adapters = default_adapter_bindings(input.state_root)?;
    let capability_refs = vec![local_ref("node-capability", input.node_id)?];
    let resource_refs = vec![local_ref("node-resource", input.node_id)?];
    let effect_profile_refs = vec![local_ref("node-effect-profile", input.node_id)?];
    let state_root_ref = state_root_profile_ref(input.state_root)?;
    let config_value = node_runtime::node_config_value(&node_runtime::ConfigValueInput {
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
    Ok(NodeDaemonInit {
        config_ref: canonical_hash(&config_value)?,
        identity_ref: identity.identity_ref,
        identity_receipt_ref: identity_resolution.receipt_ref,
        config_value,
        identity_receipt_value: identity_resolution.receipt_value,
    })
}

pub fn run_local_node(input: &NodeDaemonRunInput<'_>) -> Result<NodeDaemonRun> {
    ensure_state_layout(input.state_root)?;
    verify_restart_state(input.state_root)?;
    let config_value = read_preserves(&input.state_root.join(CONFIG_FILE))?;
    let identity_receipt = read_preserves(&input.state_root.join(IDENTITY_RECEIPT_FILE))?;
    let identity_receipt_ref = canonical_hash(&identity_receipt)?;
    let index_receipt_refs = index_receipt_refs(input.state_root)?;
    let resource_receipt_refs = resource_receipt_refs(input.state_root)?;
    let capability_receipt_refs = capability_receipt_refs(input.state_root)?;
    let version_refs = vec![local_ref("molten-binary-version", env!("CARGO_PKG_VERSION"))?];
    let source_gate_value = octet_gate::synthetic_clean_octet_gate_receipt_for_tests()?;
    let source_gate_ref = canonical_hash(&source_gate_value)?;
    let run = node_runtime::start_node_runtime(&node_runtime::NodeRuntimeStartInput {
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
    import_node_artifact(input.state_root, &run.startup_receipt.value)?;
    Ok(NodeDaemonRun {
        startup_ref,
        startup_value: run.startup_receipt.value,
        adapter_receipt_refs: run.adapter_receipts,
    })
}

pub fn status_local_node(input: &NodeDaemonStatusInput<'_>) -> Result<NodeDaemonStatus> {
    let request = status_request()?;
    status_local_node_with_request(input, &request)
}

fn status_local_node_with_request(
    input: &NodeDaemonStatusInput<'_>,
    request: &node_runtime::NodeControlRequest,
) -> Result<NodeDaemonStatus> {
    let startup_value = read_preserves(&input.state_root.join(STARTUP_FILE))?;
    let startup = node_runtime::parse_node_startup_receipt(&startup_value)?;
    let shutdown_ref = if input.state_root.join(SHUTDOWN_FILE).exists() {
        Some(canonical_hash(&read_preserves(&input.state_root.join(SHUTDOWN_FILE))?)?)
    } else {
        None
    };
    let status = if shutdown_ref.is_some() { "stopped" } else { "running" }.to_string();
    let health_value = node_runtime::node_health_receipt_value(&node_runtime::HealthReceiptValueInput {
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
    let health_ref = canonical_hash(&health_value)?;
    write_preserves(&input.state_root.join(HEALTH_FILE), &health_value)?;
    import_node_artifact(input.state_root, &health_value)?;
    let control_receipt_value = control_receipt_for_request(
        input.state_root,
        request,
        &startup.receipt_ref,
        std::slice::from_ref(&health_ref),
        &[],
    )?;
    let control_receipt_ref = canonical_hash(&control_receipt_value)?;
    write_preserves(&input.state_root.join(CONTROL_STATUS_FILE), &control_receipt_value)?;
    import_node_artifact(input.state_root, &control_receipt_value)?;
    Ok(NodeDaemonStatus {
        health_ref,
        control_receipt_ref,
        health_value,
        control_receipt_value,
        status,
    })
}

pub fn stop_local_node(input: &NodeDaemonStopInput<'_>) -> Result<NodeDaemonStop> {
    let request = shutdown_request()?;
    stop_local_node_with_request(input, &request)
}

fn stop_local_node_with_request(
    input: &NodeDaemonStopInput<'_>,
    request: &node_runtime::NodeControlRequest,
) -> Result<NodeDaemonStop> {
    let startup_value = read_preserves(&input.state_root.join(STARTUP_FILE))?;
    let startup = node_runtime::parse_node_startup_receipt(&startup_value)?;
    let mut shutdown_adapters = Vec::with_capacity(startup.adapters.len());
    for adapter in startup.adapters.iter().rev() {
        let binding = node_runtime::node_adapter_binding(&adapter.name, &adapter.receipt_ref)?;
        let value = node_runtime::node_adapter_lifecycle_receipt_value(&node_runtime::AdapterLifecycleReceiptInput {
            operation: "shutdown",
            decision: "pass",
            adapter: &binding,
            index_receipt_refs: &index_receipt_refs(input.state_root)?,
            resource_receipt_refs: &resource_receipt_refs(input.state_root)?,
            diagnostics: &[],
        })?;
        let receipt_ref = canonical_hash(&value)?;
        write_preserves(
            &input.state_root.join("receipts").join(format!("adapter-shutdown-{}.preserves", adapter.name)),
            &value,
        )?;
        import_node_artifact(input.state_root, &value)?;
        shutdown_adapters.push(node_runtime::NodeAdapterReceiptRef {
            name: adapter.name.clone(),
            receipt_ref,
        });
    }
    let index_refs = index_receipt_refs(input.state_root)?;
    let shutdown_value = node_runtime::node_shutdown_receipt_value(&node_runtime::ShutdownReceiptValueInput {
        decision: "pass",
        startup_receipt_ref: &startup.receipt_ref,
        adapter_receipts: &shutdown_adapters,
        drained_job_refs: &[],
        index_receipt_refs: &index_refs,
        diagnostics: &[],
    })?;
    let shutdown_ref = canonical_hash(&shutdown_value)?;
    write_preserves(&input.state_root.join(SHUTDOWN_FILE), &shutdown_value)?;
    import_node_artifact(input.state_root, &shutdown_value)?;
    let control_receipt_value = control_receipt_for_request(
        input.state_root,
        request,
        &startup.receipt_ref,
        std::slice::from_ref(&shutdown_ref),
        &[],
    )?;
    let control_receipt_ref = canonical_hash(&control_receipt_value)?;
    write_preserves(&input.state_root.join(CONTROL_STOP_FILE), &control_receipt_value)?;
    import_node_artifact(input.state_root, &control_receipt_value)?;
    remove_active_lock(input.state_root)?;
    Ok(NodeDaemonStop {
        shutdown_ref,
        control_receipt_ref,
        shutdown_value,
        control_receipt_value,
    })
}

pub fn submit_control_request(input: &NodeControlSubmitInput<'_>) -> Result<NodeControlSubmit> {
    validate_state_root(input.state_root)?;
    ensure_state_layout(input.state_root)?;
    let request = node_runtime::parse_node_control_request(input.request_value)?;
    import_node_artifact(input.state_root, input.request_value)?;
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
    let queue_receipt_ref = canonical_hash(&receipt_value)?;
    write_preserves(&queue_receipt_path(input.state_root, &request.request_ref), &receipt_value)?;
    import_node_artifact(input.state_root, &receipt_value)?;
    Ok(NodeControlSubmit {
        request_ref: request.request_ref,
        inbox_path,
        queue_receipt_ref,
        queue_receipt_value: receipt_value,
    })
}

pub fn dispatch_control_request(input: &NodeControlDispatchInput<'_>) -> Result<NodeControlDispatch> {
    validate_state_root(input.state_root)?;
    ensure_state_layout(input.state_root)?;
    require_active_lock(input.state_root)?;
    let request_path = match input.request_path {
        Some(path) => path.to_path_buf(),
        None => first_pending_control_request(input.state_root)?,
    };
    let request_value = read_preserves(&request_path)?;
    let request = node_runtime::parse_node_control_request(&request_value)?;
    import_node_artifact(input.state_root, &request_value)?;
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

pub fn run_control_loop(input: &NodeControlLoopInput<'_>) -> Result<NodeControlLoop> {
    validate_state_root(input.state_root)?;
    ensure_state_layout(input.state_root)?;
    let max_requests = validate_loop_request_limit(input.max_requests)?;
    require_active_lock(input.state_root)?;
    let startup = current_startup_receipt(input.state_root)?;
    let lock_value = read_preserves(&input.state_root.join(CONTROL_LOCK_FILE))?;
    let lock_ref = canonical_hash(&lock_value)?;
    let initial_diagnostics = Vec::new();
    let heartbeat_value = heartbeat_receipt_value(&HeartbeatReceiptValueInput {
        startup_receipt_ref: &startup.receipt_ref,
        lock_ref: &lock_ref,
        loop_sequence: 0,
        processed_count: 0,
        diagnostics: &initial_diagnostics,
    })?;
    let heartbeat_receipt_ref = canonical_hash(&heartbeat_value)?;
    write_preserves(&control_heartbeat_receipt_path(input.state_root, &heartbeat_receipt_ref), &heartbeat_value)?;
    import_node_artifact(input.state_root, &heartbeat_value)?;

    let mut processed_request_refs = Vec::with_capacity(max_requests);
    let mut dispatch_receipt_refs = Vec::with_capacity(max_requests);
    let mut diagnostics = Vec::new();
    let mut has_stopped = false;
    for _ in 0..max_requests {
        let Some(request_path) = next_pending_control_request(input.state_root)? else {
            break;
        };
        let dispatched = dispatch_control_request(&NodeControlDispatchInput {
            state_root: input.state_root,
            request_path: Some(&request_path),
        })?;
        let control = node_runtime::parse_node_control_receipt(&dispatched.control_receipt_value)?;
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
    let loop_receipt_ref = canonical_hash(&loop_value)?;
    write_preserves(&control_loop_receipt_path(input.state_root, &loop_receipt_ref), &loop_value)?;
    import_node_artifact(input.state_root, &loop_value)?;
    Ok(NodeControlLoop {
        loop_receipt_ref,
        loop_receipt_value: loop_value,
        heartbeat_receipt_ref,
        heartbeat_receipt_value: heartbeat_value,
        processed_request_refs,
        dispatch_receipt_refs,
        has_stopped,
    })
}

pub fn serve_node_control(input: &NodeControlServeInput<'_>) -> Result<NodeControlServe> {
    validate_state_root(input.state_root)?;
    validate_node_id(input.topic)?;
    ensure_state_layout(input.state_root)?;
    let max_ticks = validate_service_tick_limit(input.max_ticks)?;
    let max_requests_per_tick = validate_loop_request_limit(input.max_requests_per_tick)?;
    let supervisor_policy = input
        .supervisor_policy_value
        .map(|value| import_node_control_supervisor_policy(input.state_root, value))
        .transpose()?;
    let mut supervisor_receipt_refs = Vec::new();
    require_active_lock(input.state_root)?;
    let startup = current_startup_receipt(input.state_root)?;
    if input.state_root.join(CONTROL_SERVICE_LOCK_FILE).exists() {
        if let Some(policy) = supervisor_policy.as_ref()
            && policy.stale_lock_recovery
        {
            let lock_value = read_preserves(&input.state_root.join(CONTROL_SERVICE_LOCK_FILE))?;
            let stale_lock_ref = canonical_hash(&lock_value)?;
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
        } else {
            return denied_duplicate_service_run(input, &startup, supervisor_policy.as_ref(), &supervisor_receipt_refs);
        }
    }
    if let Some(policy) = supervisor_policy.as_ref() {
        let prior_runs = count_prior_supervised_service_runs(input.state_root, &policy.policy_ref)?;
        if prior_runs > policy.max_restarts {
            let diagnostics = vec![format!(
                "node control supervisor restart attempts {prior_runs} exceeded bound {}",
                policy.max_restarts
            )];
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
            let service_receipt_ref = canonical_hash(&receipt_value)?;
            write_preserves(&control_service_run_receipt_path(input.state_root, &service_receipt_ref), &receipt_value)?;
            import_node_artifact(input.state_root, &receipt_value)?;
            return Ok(NodeControlServe {
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
            });
        }
    }
    let identity = node_identity::parse_node_identity(&read_preserves(&input.state_root.join(IDENTITY_FILE))?)?;
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
    let service_lock_ref = canonical_hash(&lock_value)?;
    write_preserves(&input.state_root.join(CONTROL_SERVICE_LOCK_FILE), &lock_value)?;
    import_node_artifact(input.state_root, &lock_value)?;
    if let Some(policy) = supervisor_policy.as_ref() {
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

    let max_service_events = max_ticks.saturating_mul(max_requests_per_tick);
    let mut heartbeat_receipt_refs = Vec::with_capacity(max_ticks);
    let mut ingress_receipt_refs = Vec::with_capacity(max_service_events);
    let mut loop_receipt_refs = Vec::with_capacity(max_ticks);
    let mut processed_request_refs = Vec::with_capacity(max_service_events);
    let mut diagnostics = Vec::with_capacity(max_ticks.saturating_mul(2));
    let mut has_stopped = false;
    let mut ticks = 0_u64;

    for tick in 0..input.max_ticks {
        ticks = tick + 1;
        let heartbeat_value = service_heartbeat_receipt_value(&ServiceHeartbeatValueInput {
            startup_receipt_ref: &startup.receipt_ref,
            service_lock_ref: &service_lock_ref,
            tick,
            delivered_count: ingress_receipt_refs.len() as u64,
            processed_count: processed_request_refs.len() as u64,
            diagnostics: &diagnostics,
        })?;
        let heartbeat_ref = canonical_hash(&heartbeat_value)?;
        write_preserves(&control_service_heartbeat_path(input.state_root, &heartbeat_ref), &heartbeat_value)?;
        import_node_artifact(input.state_root, &heartbeat_value)?;
        heartbeat_receipt_refs.push(heartbeat_ref);

        let envelope_refs = match pending_ingress_envelope_refs(input.state_root, input.topic) {
            Ok(envelope_refs) => envelope_refs,
            Err(error) => {
                diagnostics.push(format!("node control service ingress scan failed: {error}"));
                break;
            }
        };
        for envelope_ref in envelope_refs {
            let delivered = match deliver_node_control_ingress(&NodeControlIngressDeliverInput {
                state_root: input.state_root,
                topic: input.topic,
                envelope_ref: &envelope_ref,
            }) {
                Ok(delivered) => delivered,
                Err(error) => {
                    diagnostics.push(format!("node control service ingress delivery {envelope_ref} failed: {error}"));
                    continue;
                }
            };
            let receipt = node_ingress_receipt_decision(&delivered.ingress_receipt_value)?;
            if receipt != "pass" {
                diagnostics
                    .push(format!("node control service ingress {} decision {}", delivered.envelope_ref, receipt));
            }
            ingress_receipt_refs.push(delivered.ingress_receipt_ref);
        }

        if !input.state_root.join(CONTROL_LOCK_FILE).exists() {
            has_stopped = true;
            break;
        }
        let loop_run = match run_control_loop(&NodeControlLoopInput {
            state_root: input.state_root,
            max_requests: input.max_requests_per_tick,
        }) {
            Ok(loop_run) => loop_run,
            Err(error) => {
                diagnostics.push(format!("node control service loop failed: {error}"));
                break;
            }
        };
        processed_request_refs.extend(loop_run.processed_request_refs.iter().cloned());
        loop_receipt_refs.push(loop_run.loop_receipt_ref);
        if loop_run.has_stopped || !input.state_root.join(CONTROL_LOCK_FILE).exists() {
            has_stopped = true;
            break;
        }
    }

    if !has_stopped {
        match has_pending_service_work(input.state_root, input.topic) {
            Ok(true) => diagnostics.push("node control service reached max ticks with pending work".to_string()),
            Ok(false) => {}
            Err(error) => diagnostics.push(format!("node control service pending-work scan failed: {error}")),
        }
    }
    if let Some(policy) = supervisor_policy.as_ref()
        && has_stopped
    {
        let mut shutdown_diagnostics = Vec::new();
        if ticks > policy.shutdown_drain_ticks {
            let diagnostic = format!(
                "node control shutdown drain ticks {ticks} exceeded supervisor bound {}",
                policy.shutdown_drain_ticks
            );
            diagnostics.push(diagnostic.clone());
            shutdown_diagnostics.push(diagnostic);
        }
        let receipt_ref = write_supervisor_receipt(input.state_root, &SupervisorReceiptValueInput {
            decision: if shutdown_diagnostics.is_empty() {
                "pass"
            } else {
                "deny"
            },
            operation: "shutdown-drain",
            startup_receipt_ref: &startup.receipt_ref,
            service_lock_ref: Some(&service_lock_ref),
            supervisor_policy_ref: Some(&policy.policy_ref),
            topic: input.topic,
            diagnostics: &shutdown_diagnostics,
        })?;
        supervisor_receipt_refs.push(receipt_ref);
    }
    remove_service_lock(input.state_root, &service_lock_ref)?;
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let receipt_value = service_run_receipt_value(&ServiceRunReceiptValueInput {
        decision,
        startup_receipt_ref: &startup.receipt_ref,
        service_lock_ref: Some(&service_lock_ref),
        topic: input.topic,
        max_ticks: input.max_ticks,
        max_requests_per_tick: input.max_requests_per_tick,
        ticks,
        heartbeat_receipt_refs: &heartbeat_receipt_refs,
        ingress_receipt_refs: &ingress_receipt_refs,
        loop_receipt_refs: &loop_receipt_refs,
        processed_request_refs: &processed_request_refs,
        has_stopped,
        supervisor_policy_ref: supervisor_policy.as_ref().map(|policy| policy.policy_ref.as_str()),
        supervisor_receipt_refs: &supervisor_receipt_refs,
        diagnostics: &diagnostics,
    })?;
    let service_receipt_ref = canonical_hash(&receipt_value)?;
    write_preserves(&control_service_run_receipt_path(input.state_root, &service_receipt_ref), &receipt_value)?;
    import_node_artifact(input.state_root, &receipt_value)?;
    Ok(NodeControlServe {
        service_receipt_ref,
        service_receipt_value: receipt_value,
        service_lock_ref: Some(service_lock_ref),
        heartbeat_receipt_refs,
        ingress_receipt_refs,
        loop_receipt_refs,
        processed_request_refs,
        supervisor_policy_ref: supervisor_policy.map(|policy| policy.policy_ref),
        supervisor_receipt_refs,
        ticks,
        has_stopped,
        decision: decision.to_string(),
    })
}

fn denied_duplicate_service_run(
    input: &NodeControlServeInput<'_>,
    startup: &node_runtime::NodeStartupReceipt,
    supervisor_policy: Option<&NodeControlSupervisorPolicy>,
    inherited_supervisor_receipt_refs: &[String],
) -> Result<NodeControlServe> {
    let lock_value = read_preserves(&input.state_root.join(CONTROL_SERVICE_LOCK_FILE))?;
    let service_lock_ref = canonical_hash(&lock_value)?;
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
    let service_receipt_ref = canonical_hash(&receipt_value)?;
    write_preserves(&control_service_run_receipt_path(input.state_root, &service_receipt_ref), &receipt_value)?;
    import_node_artifact(input.state_root, &receipt_value)?;
    Ok(NodeControlServe {
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
        let envelope = parse_node_control_ingress_envelope(&value)?;
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
    let current_ref = canonical_hash(&read_preserves(&path)?)?;
    if current_ref != service_lock_ref {
        return Err(MoltenError::invalid_harness("node control service lock changed during serve"));
    }
    fs::remove_file(path).map_err(MoltenError::from)
}

fn node_ingress_receipt_decision(value: &IOValue) -> Result<String> {
    let fields = value
        .collect_simple_record("node-control-ingress-receipt-v1", Some(15))
        .ok_or_else(|| MoltenError::invalid_harness("expected <node-control-ingress-receipt-v1 ...>"))?;
    require_schema(&fields[0], NODE_CONTROL_INGRESS_RECEIPT_SCHEMA, "node control ingress receipt")?;
    record_string(&fields[1], "decision")
}

pub fn node_control_ingress_envelope(
    input: &NodeControlIngressEnvelopeInput<'_>,
) -> Result<NodeControlIngressEnvelope> {
    node_control_ingress_envelope_for_transport(input, LOCAL_CONTROL_INGRESS_TRANSPORT, "iroh-local-ingress")
}

pub fn node_control_live_ingress_envelope(
    input: &NodeControlIngressEnvelopeInput<'_>,
) -> Result<NodeControlIngressEnvelope> {
    node_control_ingress_envelope_for_transport(input, LIVE_CONTROL_INGRESS_TRANSPORT, "live-iroh-gossip")
}

fn node_control_ingress_envelope_for_transport(
    input: &NodeControlIngressEnvelopeInput<'_>,
    transport: &str,
    transport_check: &str,
) -> Result<NodeControlIngressEnvelope> {
    let request = node_runtime::parse_node_control_request(input.request_value)?;
    validate_node_id(input.from_peer)?;
    validate_node_id(input.to_node)?;
    validate_node_id(input.topic)?;
    validate_node_id(transport)?;
    validate_ingress_refs(input.peer_bootstrap_refs, "node control ingress peer bootstrap ref")?;
    validate_ingress_refs(input.authority_refs, "node control ingress authority ref")?;
    validate_ingress_refs(input.policy_refs, "node control ingress policy ref")?;
    validate_ingress_refs(input.resource_refs, "node control ingress resource ref")?;
    validate_ingress_refs(input.evidence_refs, "node control ingress evidence ref")?;
    let scope_ref = delivery_idempotency::remote_topic_scope_ref(input.topic, input.to_node)?;
    let operation = delivery_idempotency::derive_operation_id(delivery_idempotency::OperationIdInput {
        scope_ref,
        producer: input.from_peer.to_string(),
        consumer: input.to_node.to_string(),
        sequence: input.sequence,
        intent: "node-control-ingress".to_string(),
        payload_ref: request.request_ref.clone(),
        policy_refs: input.policy_refs.to_vec(),
    })?;
    let value = ingress_envelope_value(input, &request, &operation.operation_ref, transport, transport_check)?;
    parse_node_control_ingress_envelope(&value)
}

pub async fn publish_node_control_live_ingress(
    input: &NodeControlLiveIngressPublishInput<'_>,
) -> Result<NodeControlLiveIngressPublish> {
    validate_node_id(input.node_id)?;
    let envelope = parse_node_control_ingress_envelope(input.envelope_value)?;
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
            .broadcast(canonical_bytes(&envelope.value)?.into())
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
    let transport_receipt_ref = canonical_hash(&receipt_value)?;
    Ok(NodeControlLiveIngressPublish {
        envelope_ref: envelope.envelope_ref,
        transport_receipt_ref,
        transport_receipt_value: receipt_value,
    })
}

pub fn receive_node_control_live_ingress_event(
    state_root: &Path,
    event: &iroh_gossip::api::Event,
    topic: &str,
    receiver_node: &str,
) -> Result<Option<NodeControlLiveIngressReceive>> {
    match event {
        iroh_gossip::api::Event::Received(message) => {
            receive_node_control_live_ingress_bytes(&NodeControlLiveIngressReceiveBytesInput {
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

pub fn receive_node_control_live_ingress_bytes(
    input: &NodeControlLiveIngressReceiveBytesInput<'_>,
) -> Result<NodeControlLiveIngressReceive> {
    validate_state_root(input.state_root)?;
    validate_node_id(input.topic)?;
    validate_node_id(input.receiver_node)?;
    validate_node_id(input.delivered_from)?;
    ensure_state_layout(input.state_root)?;
    let value = parse_canonical_bytes(input.bytes)?;
    let envelope = parse_node_control_ingress_envelope(&value)?;
    let mut diagnostics = live_receive_diagnostics(input, &envelope);
    write_preserves(&control_ingress_envelope_path(input.state_root, input.topic, &envelope.envelope_ref), &value)?;
    import_node_artifact(input.state_root, &value)?;
    let delivered = if diagnostics.is_empty() {
        deliver_node_control_ingress(&NodeControlIngressDeliverInput {
            state_root: input.state_root,
            topic: input.topic,
            envelope_ref: &envelope.envelope_ref,
        })?
    } else {
        denied_live_ingress_delivery(input.state_root, &envelope, &diagnostics)?
    };
    let ingress_decision = node_ingress_receipt_decision(&delivered.ingress_receipt_value)?;
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
    let transport_receipt_ref = canonical_hash(&receipt_value)?;
    write_preserves(
        &control_live_transport_receipt_path(input.state_root, &envelope.envelope_ref, "receive"),
        &receipt_value,
    )?;
    import_node_artifact(input.state_root, &receipt_value)?;
    Ok(NodeControlLiveIngressReceive {
        envelope_ref: envelope.envelope_ref,
        transport_receipt_ref,
        transport_receipt_value: receipt_value,
        ingress_receipt_ref: delivered.ingress_receipt_ref,
        ingress_receipt_value: delivered.ingress_receipt_value,
        has_enqueued: delivered.has_enqueued,
    })
}

pub async fn node_control_live_iroh_loopback(
    input: &NodeControlLiveLoopbackInput<'_>,
) -> Result<NodeControlLiveLoopback> {
    validate_state_root(input.state_root)?;
    ensure_state_layout(input.state_root)?;
    let envelope_input = NodeControlIngressEnvelopeInput {
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
    let envelope = node_control_live_ingress_envelope(&envelope_input)?;
    let topic_id = node_control_live_topic_id(input.topic);
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
    let published = publish_node_control_live_ingress(&NodeControlLiveIngressPublishInput {
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
    Ok(NodeControlLiveLoopback {
        envelope_ref: envelope.envelope_ref,
        publish_receipt_ref: published.transport_receipt_ref,
        publish_receipt_value: published.transport_receipt_value,
        receive_receipt_ref: received.transport_receipt_ref,
        receive_receipt_value: received.transport_receipt_value,
        ingress_receipt_ref: received.ingress_receipt_ref,
        has_enqueued: received.has_enqueued,
    })
}

pub fn preflight_node_control_live_send(input: &NodeControlLiveSendInput<'_>) -> Result<NodeControlLiveSendPreflight> {
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
    let ticket = parse_node_control_live_ticket(input.receiver_ticket_value)?;
    let envelope = node_control_live_ingress_envelope(&NodeControlIngressEnvelopeInput {
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
    Ok(NodeControlLiveSendPreflight {
        decision: decision.to_string(),
        envelope_ref: envelope.envelope_ref,
        operation_ref: envelope.operation_ref,
        receiver_ticket_ref: ticket.ticket_ref,
        diagnostics,
    })
}

pub async fn send_node_control_live_ingress(input: &NodeControlLiveSendInput<'_>) -> Result<NodeControlLiveSend> {
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
    let ticket = parse_node_control_live_ticket(input.receiver_ticket_value)?;
    let envelope = node_control_live_ingress_envelope(&NodeControlIngressEnvelopeInput {
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
    if let Some(operation_ref) = input.expected_operation_ref
        && operation_ref != envelope.operation_ref
    {
        let diagnostics = vec![format!(
            "node control live send operation-id {operation_ref} does not match derived {}",
            envelope.operation_ref
        )];
        return denied_node_control_live_send_with_diagnostics(DeniedLiveSendInput {
            input,
            ticket: &ticket,
            envelope,
            diagnostics,
            retry_receipt_refs: Vec::new(),
            retry_receipt_values: Vec::new(),
        });
    }
    let mut preflight_diagnostics = live_send_ticket_diagnostics(input, &ticket);
    if let Some(state_root) = input.state_root {
        preflight_diagnostics.extend(live_send_state_root_evidence_diagnostics(state_root, input, &envelope)?);
    }
    let receiver_addr = if ticket.address_refs.is_empty() {
        preflight_diagnostics.push(
            "node control live send ticket has no endpoint addresses; import a bound live ticket with live-ticket-import or use serve --live-ticket-out"
                .to_string(),
        );
        None
    } else {
        match live_ticket_endpoint_addr(&ticket) {
            Ok(addr) => Some(addr),
            Err(error) => {
                preflight_diagnostics.push(format!(
                    "node control live send ticket address unsupported or malformed: {error}; import a fresh live ticket with live-ticket-import"
                ));
                None
            }
        }
    };
    if !preflight_diagnostics.is_empty() {
        return denied_node_control_live_send_with_diagnostics(DeniedLiveSendInput {
            input,
            ticket: &ticket,
            envelope,
            diagnostics: preflight_diagnostics,
            retry_receipt_refs: Vec::new(),
            retry_receipt_values: Vec::new(),
        });
    }
    let receiver_addr = receiver_addr.ok_or_else(|| {
        MoltenError::invalid_harness("node control live send receiver address missing after preflight")
    })?;
    if let Some(state_root) = input.state_root
        && let Some(duplicate) = duplicate_node_control_live_send(input, state_root, &ticket, &envelope)?
    {
        return Ok(duplicate);
    }
    let attempt_capacity = usize::try_from(input.max_attempts)
        .map_err(|_| MoltenError::invalid_harness("node control live send attempts exceed usize capacity"))?;
    let mut retry_receipt_refs = Vec::with_capacity(attempt_capacity);
    let mut retry_receipt_values = Vec::with_capacity(attempt_capacity);
    let mut diagnostics = Vec::with_capacity(attempt_capacity);
    let mut published = None;
    for attempt in 1..=input.max_attempts {
        match attempt_node_control_live_send(input, &receiver_addr, &envelope).await? {
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
                    ticket: &ticket,
                    envelope: &envelope,
                    diagnostics: &attempt_diagnostics,
                })?;
                let retry_ref = canonical_hash(&retry_value)?;
                if let Some(state_root) = input.state_root {
                    write_preserves(&control_live_send_retry_receipt_path(state_root, &retry_ref), &retry_value)?;
                    import_node_artifact(state_root, &retry_value)?;
                }
                retry_receipt_refs.push(retry_ref);
                retry_receipt_values.push(retry_value);
            }
        }
    }
    let Some(published) = published else {
        return denied_node_control_live_send_with_diagnostics(DeniedLiveSendInput {
            input,
            ticket: &ticket,
            envelope,
            diagnostics,
            retry_receipt_refs,
            retry_receipt_values,
        });
    };
    let send_receipt_value = live_send_receipt_value(&LiveSendReceiptValueInput {
        decision: "pass",
        from_peer: input.from_peer,
        ticket: &ticket,
        envelope: &envelope,
        transport_receipt_ref: Some(&published.transport_receipt_ref),
        diagnostics: &[],
    })?;
    let send_receipt_ref = canonical_hash(&send_receipt_value)?;
    if let Some(state_root) = input.state_root {
        import_node_artifact(state_root, input.receiver_ticket_value)?;
        write_preserves(
            &control_ingress_envelope_path(state_root, &ticket.topic, &envelope.envelope_ref),
            &envelope.value,
        )?;
        import_node_artifact(state_root, &envelope.value)?;
        write_preserves(
            &control_live_transport_receipt_path(state_root, &envelope.envelope_ref, "send"),
            &published.transport_receipt_value,
        )?;
        import_node_artifact(state_root, &published.transport_receipt_value)?;
        write_preserves(&control_live_send_receipt_path(state_root, &send_receipt_ref), &send_receipt_value)?;
        import_node_artifact(state_root, &send_receipt_value)?;
    }
    Ok(NodeControlLiveSend {
        envelope_ref: envelope.envelope_ref,
        envelope_value: envelope.value,
        operation_ref: envelope.operation_ref,
        receiver_ticket_ref: ticket.ticket_ref,
        receiver_endpoint_id: ticket.live_endpoint_id,
        transport_receipt_ref: Some(published.transport_receipt_ref),
        transport_receipt_value: Some(published.transport_receipt_value),
        retry_receipt_refs,
        retry_receipt_values,
        duplicate_receipt_ref: None,
        duplicate_receipt_value: None,
        send_receipt_ref,
        send_receipt_value,
    })
}

async fn attempt_node_control_live_send(
    input: &NodeControlLiveSendInput<'_>,
    receiver_addr: &iroh::EndpointAddr,
    envelope: &NodeControlIngressEnvelope,
) -> Result<std::result::Result<NodeControlLiveIngressPublish, String>> {
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
    let topic_id = node_control_live_topic_id(&envelope.topic);
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
            let published = publish_node_control_live_ingress(&NodeControlLiveIngressPublishInput {
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

fn duplicate_node_control_live_send(
    input: &NodeControlLiveSendInput<'_>,
    state_root: &Path,
    ticket: &NodeControlLiveTicket,
    envelope: &NodeControlIngressEnvelope,
) -> Result<Option<NodeControlLiveSend>> {
    let transport_receipt_value = live_transport_receipt_value(&LiveTransportReceiptValueInput {
        operation: "publish",
        decision: "pass",
        node_id: input.from_peer,
        delivered_from: None,
        envelope,
        ingress_receipt_ref: None,
        diagnostics: &[],
    })?;
    let transport_receipt_ref = canonical_hash(&transport_receipt_value)?;
    let send_receipt_value = live_send_receipt_value(&LiveSendReceiptValueInput {
        decision: "pass",
        from_peer: input.from_peer,
        ticket,
        envelope,
        transport_receipt_ref: Some(&transport_receipt_ref),
        diagnostics: &[],
    })?;
    let send_receipt_ref = canonical_hash(&send_receipt_value)?;
    let send_path = control_live_send_receipt_path(state_root, &send_receipt_ref);
    if !send_path.exists() {
        return Ok(None);
    }
    let prior_send_value = read_preserves(&send_path)?;
    let prior_send = parse_node_control_live_send_receipt(&prior_send_value)?;
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
    let duplicate_receipt_ref = canonical_hash(&duplicate_receipt_value)?;
    write_preserves(
        &control_live_send_duplicate_receipt_path(state_root, &duplicate_receipt_ref),
        &duplicate_receipt_value,
    )?;
    import_node_artifact(state_root, &duplicate_receipt_value)?;
    Ok(Some(NodeControlLiveSend {
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

fn denied_node_control_live_send_with_diagnostics(denied: DeniedLiveSendInput<'_>) -> Result<NodeControlLiveSend> {
    let send_receipt_value = live_send_receipt_value(&LiveSendReceiptValueInput {
        decision: "deny",
        from_peer: denied.input.from_peer,
        ticket: denied.ticket,
        envelope: &denied.envelope,
        transport_receipt_ref: None,
        diagnostics: &denied.diagnostics,
    })?;
    let send_receipt_ref = canonical_hash(&send_receipt_value)?;
    if let Some(state_root) = denied.input.state_root {
        import_node_artifact(state_root, denied.input.receiver_ticket_value)?;
        write_preserves(
            &control_ingress_envelope_path(state_root, &denied.ticket.topic, &denied.envelope.envelope_ref),
            &denied.envelope.value,
        )?;
        import_node_artifact(state_root, &denied.envelope.value)?;
        write_preserves(&control_live_send_receipt_path(state_root, &send_receipt_ref), &send_receipt_value)?;
        import_node_artifact(state_root, &send_receipt_value)?;
    }
    Ok(NodeControlLiveSend {
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

pub fn parse_node_control_live_send_receipt(value: &IOValue) -> Result<NodeControlLiveSendReceipt> {
    let fields = value
        .collect_simple_record("node-control-live-send-receipt-v1", Some(13))
        .ok_or_else(|| MoltenError::invalid_harness("expected <node-control-live-send-receipt-v1 ...>"))?;
    require_schema(&fields[0], NODE_CONTROL_LIVE_SEND_RECEIPT_SCHEMA, "node control live send receipt")?;
    let transport_receipt_ref = record_optional_string(&fields[10], "transport-receipt")?;
    if let Some(reference) = transport_receipt_ref.as_ref() {
        validate_ingress_ref(reference, "node control live send transport receipt ref")?;
    }
    Ok(NodeControlLiveSendReceipt {
        receipt_ref: canonical_hash(value)?,
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

pub fn node_control_live_workflow_receipt(
    input: &NodeControlLiveWorkflowInput<'_>,
) -> Result<NodeControlLiveWorkflowReceipt> {
    if let Some(state_root) = input.state_root {
        validate_state_root(state_root)?;
        ensure_state_layout(state_root)?;
    }
    let ticket = parse_node_control_live_ticket(input.receiver_ticket_value)?;
    let admission = parse_node_control_live_peer_admission(input.peer_admission_value)?;
    let authority = parse_node_control_authority_grant(input.authority_grant_value)?;
    let send = parse_node_control_live_send_receipt(input.send_receipt_value)?;
    let service_receipt_ref = service_run_receipt_ref(input.service_receipt_value)?;
    let mut diagnostics = Vec::with_capacity(input.receive_receipt_values.len().saturating_add(8));
    if admission.ticket_ref != ticket.ticket_ref {
        diagnostics.push("node control live workflow admission does not bind receiver ticket".to_string());
    }
    if admission.decision != "pass" {
        diagnostics.push(format!("node control live workflow admission decision {}", admission.decision));
    }
    if authority.peer_id != admission.peer_id {
        diagnostics.push("node control live workflow authority grant peer does not match admission".to_string());
    }
    if authority.node_id != ticket.node_id {
        diagnostics.push("node control live workflow authority grant node does not match ticket".to_string());
    }
    if send.receiver_ticket_ref != ticket.ticket_ref {
        diagnostics.push("node control live workflow send receipt does not bind receiver ticket".to_string());
    }
    if send.from_peer != admission.peer_id {
        diagnostics.push("node control live workflow send peer does not match admission".to_string());
    }
    if send.to_node != ticket.node_id || send.topic != ticket.topic {
        diagnostics.push("node control live workflow send destination does not match ticket".to_string());
    }
    if send.decision != "pass" {
        diagnostics.push(format!("node control live workflow send decision {}", send.decision));
    }
    let mut receive_receipt_refs = Vec::with_capacity(input.receive_receipt_values.len());
    for receive_value in input.receive_receipt_values {
        let (receipt_ref, operation, envelope_ref) = live_transport_receipt_ref(receive_value)?;
        if operation != "receive" {
            diagnostics
                .push(format!("node control live workflow transport receipt operation {operation} is not receive"));
        }
        if envelope_ref != send.envelope_ref {
            diagnostics.push("node control live workflow receive envelope does not match send envelope".to_string());
        }
        receive_receipt_refs.push(receipt_ref);
    }
    if receive_receipt_refs.is_empty() {
        diagnostics.push("node control live workflow missing receive receipt".to_string());
    }
    let listener_receipt_ref = if let Some(listener_value) = input.listener_receipt_value {
        let (listener_ref, listener_transport_refs, listener_service_ref) = live_listener_receipt_refs(listener_value)?;
        for receive_ref in &receive_receipt_refs {
            if !listener_transport_refs.iter().any(|reference| reference == receive_ref) {
                diagnostics.push("node control live workflow listener does not bind receive receipt".to_string());
            }
        }
        if listener_service_ref != service_receipt_ref {
            diagnostics
                .push("node control live workflow listener service run does not match service receipt".to_string());
        }
        Some(listener_ref)
    } else {
        None
    };
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let receipt_value = live_workflow_receipt_value(&LiveWorkflowReceiptValueInput {
        decision,
        ticket: &ticket,
        admission: &admission,
        authority: &authority,
        send: &send,
        receive_receipt_refs: &receive_receipt_refs,
        listener_receipt_ref: listener_receipt_ref.as_deref(),
        service_receipt_ref: &service_receipt_ref,
        diagnostics: &diagnostics,
    })?;
    let receipt_ref = canonical_hash(&receipt_value)?;
    if let Some(state_root) = input.state_root {
        import_node_artifact(state_root, input.receiver_ticket_value)?;
        import_node_artifact(state_root, input.peer_admission_value)?;
        import_node_artifact(state_root, input.authority_grant_value)?;
        import_node_artifact(state_root, input.send_receipt_value)?;
        for receive_value in input.receive_receipt_values {
            import_node_artifact(state_root, receive_value)?;
        }
        if let Some(listener_value) = input.listener_receipt_value {
            import_node_artifact(state_root, listener_value)?;
        }
        import_node_artifact(state_root, input.service_receipt_value)?;
        write_preserves(&control_live_workflow_receipt_path(state_root, &receipt_ref), &receipt_value)?;
        import_node_artifact(state_root, &receipt_value)?;
    }
    Ok(NodeControlLiveWorkflowReceipt {
        receipt_ref,
        receipt_value,
        decision: decision.to_string(),
        diagnostics,
    })
}

fn service_run_receipt_ref(value: &IOValue) -> Result<String> {
    if let Some(fields) = value.collect_simple_record("node-control-service-run-receipt-v1", Some(17)) {
        require_schema(&fields[0], NODE_CONTROL_SERVICE_RUN_RECEIPT_SCHEMA, "node control service run receipt")?;
        return canonical_hash(value);
    }
    if let Some(fields) = value.collect_simple_record("node-control-service-run-receipt-v1", Some(15)) {
        require_schema(&fields[0], NODE_CONTROL_SERVICE_RUN_RECEIPT_SCHEMA, "node control service run receipt")?;
        return canonical_hash(value);
    }
    Err(MoltenError::invalid_harness("expected <node-control-service-run-receipt-v1 ...>"))
}

fn live_transport_receipt_ref(value: &IOValue) -> Result<(String, String, String)> {
    let fields = value
        .collect_simple_record("node-control-live-transport-receipt-v1", Some(11))
        .ok_or_else(|| MoltenError::invalid_harness("expected <node-control-live-transport-receipt-v1 ...>"))?;
    require_schema(&fields[0], NODE_CONTROL_LIVE_TRANSPORT_RECEIPT_SCHEMA, "node control live transport receipt")?;
    Ok((
        canonical_hash(value)?,
        record_string(&fields[1], "operation")?,
        record_ref_string(&fields[7], "envelope")?,
    ))
}

fn live_listener_receipt_refs(value: &IOValue) -> Result<(String, Vec<String>, String)> {
    let fields = value
        .collect_simple_record("node-control-live-listener-receipt-v1", Some(14))
        .ok_or_else(|| MoltenError::invalid_harness("expected <node-control-live-listener-receipt-v1 ...>"))?;
    require_schema(&fields[0], NODE_CONTROL_LIVE_LISTENER_RECEIPT_SCHEMA, "node control live listener receipt")?;
    Ok((
        canonical_hash(value)?,
        record_ref_strings(&fields[9], "transport-receipts")?,
        record_ref_string(&fields[11], "service-run")?,
    ))
}

pub async fn serve_node_control_live_listener(input: &NodeControlLiveServeInput<'_>) -> Result<NodeControlLiveServe> {
    validate_state_root(input.state_root)?;
    validate_node_id(input.topic)?;
    validate_listener_event_limit(input.max_events)?;
    validate_loop_request_limit(input.max_requests_per_tick)?;
    ensure_state_layout(input.state_root)?;
    let identity = node_identity::parse_node_identity(&read_preserves(&input.state_root.join(IDENTITY_FILE))?)?;
    let lookup = iroh::address_lookup::memory::MemoryLookup::new();
    let endpoint = live_gossip_endpoint(&lookup, Some(stable_live_endpoint_secret(&identity))).await?;
    let bound_endpoint_id = format!("iroh:{}", endpoint.id());
    let live_ticket = live_ticket_for_bound_endpoint(input.state_root, &identity, input.topic, &endpoint.addr())?;
    lookup.add_endpoint_info(endpoint.addr());
    let gossip = iroh_gossip::Gossip::builder().spawn(endpoint.clone());
    let router = iroh::protocol::Router::builder(endpoint).accept(iroh_gossip::ALPN, gossip.clone()).spawn();
    let mut topic = gossip
        .subscribe(node_control_live_topic_id(input.topic), Vec::new())
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

pub async fn node_control_live_serve_listener_loopback(
    input: &NodeControlLiveServeLoopbackInput<'_>,
) -> Result<NodeControlLiveServeLoopback> {
    validate_state_root(input.state_root)?;
    ensure_state_layout(input.state_root)?;
    let envelope_input = NodeControlIngressEnvelopeInput {
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
    let envelope = node_control_live_ingress_envelope(&envelope_input)?;
    let identity = node_identity::parse_node_identity(&read_preserves(&input.state_root.join(IDENTITY_FILE))?)?;
    let lookup = iroh::address_lookup::memory::MemoryLookup::new();
    let receiver_endpoint = live_gossip_endpoint(&lookup, Some(stable_live_endpoint_secret(&identity))).await?;
    let sender_endpoint = live_gossip_endpoint(&lookup, None).await?;
    let live_ticket =
        live_ticket_for_bound_endpoint(input.state_root, &identity, input.topic, &receiver_endpoint.addr())?;
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
    let topic_id = node_control_live_topic_id(input.topic);
    let mut receiver_topic = receiver_gossip.subscribe(topic_id, vec![sender_id]).await.map_err(|error| {
        MoltenError::invalid_harness(format!("live Iroh listener receiver subscribe failed: {error}"))
    })?;
    let sender_topic = sender_gossip
        .subscribe_and_join(topic_id, vec![receiver_id])
        .await
        .map_err(|error| MoltenError::invalid_harness(format!("live Iroh listener sender join failed: {error}")))?;
    let (sender, _unused_receiver) = sender_topic.split();
    let published = publish_node_control_live_ingress(&NodeControlLiveIngressPublishInput {
        sender: &sender,
        envelope_value: &envelope.value,
        node_id: input.from_peer,
    })
    .await?;
    let listener_input = NodeControlLiveServeInput {
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
        &identity.node_id,
        &identity.endpoint_id,
        &bound_endpoint_id,
    )
    .await?;
    listener.live_ticket_ref = Some(live_ticket.ticket_ref);
    listener.live_ticket_value = Some(live_ticket.value);
    receiver_router.shutdown().await.map_err(|error| {
        MoltenError::invalid_harness(format!("live Iroh listener receiver shutdown failed: {error}"))
    })?;
    sender_router
        .shutdown()
        .await
        .map_err(|error| MoltenError::invalid_harness(format!("live Iroh listener sender shutdown failed: {error}")))?;
    Ok(NodeControlLiveServeLoopback {
        envelope_ref: envelope.envelope_ref,
        publish_receipt_ref: published.transport_receipt_ref,
        listener,
    })
}

async fn serve_node_control_live_listener_with_topic(
    input: &NodeControlLiveServeInput<'_>,
    receiver: &mut iroh_gossip::api::GossipTopic,
    node_id: &str,
    logical_endpoint_id: &str,
    bound_endpoint_id: &str,
) -> Result<NodeControlLiveServe> {
    validate_listener_event_limit(input.max_events)?;
    validate_loop_request_limit(input.max_requests_per_tick)?;
    let event_capacity = usize::try_from(input.max_events)
        .map_err(|_| MoltenError::invalid_harness("node control live listener max events exceeds usize capacity"))?;
    let startup = current_startup_receipt(input.state_root)?;
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
                    receive_node_control_live_ingress_event(input.state_root, &event, input.topic, node_id)?
                {
                    transport_receipt_refs.push(received.transport_receipt_ref);
                }
            }
        }
        if !transport_receipt_refs.is_empty() {
            break;
        }
    }
    let service = serve_node_control(&NodeControlServeInput {
        state_root: input.state_root,
        topic: input.topic,
        max_ticks: 1,
        max_requests_per_tick: input.max_requests_per_tick,
        supervisor_policy_value: input.supervisor_policy_value,
    })?;
    if service.decision != "pass" {
        diagnostics.push(format!("node control live listener service drain decision {}", service.decision));
    }
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let receipt_value = live_listener_receipt_value(&ListenerReceiptValueInput {
        decision,
        startup_receipt_ref: &startup.receipt_ref,
        node_id,
        logical_endpoint_id,
        bound_endpoint_id,
        topic: input.topic,
        max_events: input.max_events,
        observed_events,
        transport_receipt_refs: &transport_receipt_refs,
        neighbor_events: &neighbor_events,
        service_receipt_ref: &service.service_receipt_ref,
        diagnostics: &diagnostics,
    })?;
    let listener_receipt_ref = canonical_hash(&receipt_value)?;
    write_preserves(&control_live_listener_receipt_path(input.state_root, &listener_receipt_ref), &receipt_value)?;
    import_node_artifact(input.state_root, &receipt_value)?;
    Ok(NodeControlLiveServe {
        listener_receipt_ref,
        listener_receipt_value: receipt_value,
        service,
        transport_receipt_refs,
        neighbor_events,
        observed_events,
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
) -> Result<NodeControlLiveIngressReceive> {
    for _ in 0..MAX_CONTROL_LIVE_LISTENER_EVENTS {
        let Some(event) = receiver.next().await else {
            return Err(MoltenError::invalid_harness("live Iroh receiver closed before node control envelope arrived"));
        };
        let event =
            event.map_err(|error| MoltenError::invalid_harness(format!("live Iroh receive failed: {error}")))?;
        if let Some(received) = receive_node_control_live_ingress_event(state_root, &event, topic, receiver_node)? {
            return Ok(received);
        }
    }
    Err(MoltenError::invalid_harness(
        "live Iroh receiver exceeded bounded event scan before node control envelope arrived",
    ))
}

fn stable_live_endpoint_secret(identity: &node_identity::NodeIdentity) -> iroh::SecretKey {
    let seed = blake3::hash(
        format!("molten.node-control.live.endpoint.v1:{}:{}", identity.node_id, identity.endpoint_id).as_bytes(),
    );
    iroh::SecretKey::from_bytes(seed.as_bytes())
}

fn stable_live_endpoint_id(identity: &node_identity::NodeIdentity) -> String {
    format!("iroh:{}", stable_live_endpoint_secret(identity).public())
}

fn live_ticket_address_refs(addr: &iroh::EndpointAddr) -> Vec<String> {
    addr.addrs.iter().map(ToString::to_string).collect()
}

fn live_ticket_for_bound_endpoint(
    state_root: &Path,
    identity: &node_identity::NodeIdentity,
    topic: &str,
    addr: &iroh::EndpointAddr,
) -> Result<NodeControlLiveTicket> {
    let address_refs = live_ticket_address_refs(addr);
    let value = node_control_live_ticket_value(&NodeControlLiveTicketInput {
        node_id: &identity.node_id,
        node_identity_ref: &identity.identity_ref,
        logical_endpoint_id: &identity.endpoint_id,
        live_endpoint_id: &format!("iroh:{}", addr.id),
        topic,
        address_refs: &address_refs,
        policy_refs: &identity.policy_refs,
        evidence_refs: &identity.receipt_refs,
    })?;
    let ticket = parse_node_control_live_ticket(&value)?;
    import_node_artifact(state_root, &value)?;
    Ok(ticket)
}

fn live_send_ticket_diagnostics(input: &NodeControlLiveSendInput<'_>, ticket: &NodeControlLiveTicket) -> Vec<String> {
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
    input: &NodeControlLiveSendInput<'_>,
    envelope: &NodeControlIngressEnvelope,
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

fn live_send_authority_grant_diagnostics(
    state_root: &Path,
    envelope: &NodeControlIngressEnvelope,
) -> Result<Vec<String>> {
    let mut diagnostics = Vec::with_capacity(envelope.authority_refs.len().saturating_add(2));
    let mut has_candidate_authority = false;
    let mut has_admitted_grant = false;
    for authority_ref in envelope
        .authority_refs
        .iter()
        .filter(|authority_ref| envelope.request.authority_refs.contains(*authority_ref))
    {
        has_candidate_authority = true;
        match read_node_ledger_artifact(state_root, authority_ref) {
            Ok(value) => match parse_node_control_authority_grant(&value) {
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

fn live_ticket_endpoint_addr(ticket: &NodeControlLiveTicket) -> Result<iroh::EndpointAddr> {
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

fn node_control_live_topic_id(topic: &str) -> iroh_gossip::TopicId {
    let digest = blake3::hash(format!("molten.node-control.live.topic.v1:{topic}").as_bytes());
    iroh_gossip::TopicId::from_bytes(*digest.as_bytes())
}

fn denied_live_ingress_delivery(
    state_root: &Path,
    envelope: &NodeControlIngressEnvelope,
    diagnostics: &[String],
) -> Result<NodeControlIngressDeliver> {
    let receipt_value = ingress_receipt_value(&IngressReceiptValueInput {
        decision: "deny",
        phase: "live-receive-deny",
        transport: &envelope.transport,
        envelope,
        idempotency_receipt_ref: None,
        queue_receipt_ref: None,
        diagnostics,
    })?;
    let ingress_receipt_ref = canonical_hash(&receipt_value)?;
    write_preserves(&control_ingress_receipt_path(state_root, &envelope.envelope_ref, "deliver"), &receipt_value)?;
    import_node_artifact(state_root, &receipt_value)?;
    Ok(NodeControlIngressDeliver {
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
    input: &NodeControlLiveIngressReceiveBytesInput<'_>,
    envelope: &NodeControlIngressEnvelope,
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

pub fn parse_node_control_ingress_envelope(value: &IOValue) -> Result<NodeControlIngressEnvelope> {
    let fields = value
        .collect_simple_record("node-control-ingress-envelope-v1", Some(15))
        .ok_or_else(|| MoltenError::invalid_harness("expected <node-control-ingress-envelope-v1 ...>"))?;
    require_schema(&fields[0], NODE_CONTROL_INGRESS_ENVELOPE_SCHEMA, "node control ingress envelope")?;
    let transport = record_string(&fields[1], "transport")?;
    let topic = record_string(&fields[2], "topic")?;
    let from_peer = record_string(&fields[3], "from-peer")?;
    let to_node = record_string(&fields[4], "to-node")?;
    let sequence = record_u64_string(&fields[5], "sequence")?;
    let operation_ref = record_ref_string(&fields[6], "operation")?;
    let request_ref = record_ref_string(&fields[7], "request-ref")?;
    let request_value = record_value(&fields[8], "request")?;
    let request = node_runtime::parse_node_control_request(&request_value)?;
    if request.request_ref != request_ref {
        return Err(MoltenError::invalid_harness("node control ingress embedded request ref mismatch"));
    }
    let peer_bootstrap_refs = record_ref_strings(&fields[9], "peer-bootstrap")?;
    let authority_refs = record_ref_strings(&fields[10], "authority")?;
    let policy_refs = record_ref_strings(&fields[11], "policy")?;
    let resource_refs = record_ref_strings(&fields[12], "resource")?;
    let evidence_refs = record_ref_strings(&fields[13], "evidence")?;
    let expected_scope = delivery_idempotency::remote_topic_scope_ref(&topic, &to_node)?;
    let expected_operation = delivery_idempotency::derive_operation_id(delivery_idempotency::OperationIdInput {
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
    Ok(NodeControlIngressEnvelope {
        envelope_ref: canonical_hash(value)?,
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

pub fn publish_node_control_ingress(input: &NodeControlIngressPublishInput<'_>) -> Result<NodeControlIngressPublish> {
    validate_state_root(input.state_root)?;
    ensure_state_layout(input.state_root)?;
    let envelope = parse_node_control_ingress_envelope(input.envelope_value)?;
    let envelope_path = control_ingress_envelope_path(input.state_root, &envelope.topic, &envelope.envelope_ref);
    write_preserves(&envelope_path, &envelope.value)?;
    import_node_artifact(input.state_root, &envelope.value)?;
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
    let receipt_ref = canonical_hash(&receipt_value)?;
    write_preserves(
        &control_ingress_receipt_path(input.state_root, &envelope.envelope_ref, "publish"),
        &receipt_value,
    )?;
    import_node_artifact(input.state_root, &receipt_value)?;
    Ok(NodeControlIngressPublish {
        envelope_ref: envelope.envelope_ref,
        envelope_path,
        receipt_ref,
        receipt_value,
    })
}

pub fn deliver_node_control_ingress(input: &NodeControlIngressDeliverInput<'_>) -> Result<NodeControlIngressDeliver> {
    validate_state_root(input.state_root)?;
    validate_node_id(input.topic)?;
    ensure_state_layout(input.state_root)?;
    let envelope_value =
        read_preserves(&control_ingress_envelope_path(input.state_root, input.topic, input.envelope_ref))?;
    let envelope = parse_node_control_ingress_envelope(&envelope_value)?;
    let mut diagnostics = ingress_pre_enqueue_diagnostics(input.state_root, input.topic, &envelope)?;
    let mut idempotency_receipt_ref = None;
    let mut queue_receipt_ref = None;
    let mut has_enqueued = false;
    if diagnostics.is_empty() {
        let idempotency_evidence_refs = ingress_idempotency_evidence_refs(&envelope);
        let scope_ref = delivery_idempotency::remote_topic_scope_ref(&envelope.topic, &envelope.to_node)?;
        let delivery = delivery_idempotency::check_delivery(delivery_idempotency::DeliveryCheckInput {
            root: &input.state_root.join(CONTROL_IDEMPOTENCY_DIR),
            scope_profile: delivery_idempotency::SCOPE_REMOTE_TOPIC,
            scope_ref: &scope_ref,
            producer: &envelope.from_peer,
            consumer: &envelope.to_node,
            sequence: envelope.sequence,
            intent: "node-control-ingress",
            payload_ref: &envelope.request.request_ref,
            policy_refs: &envelope.policy_refs,
            evidence_refs: &idempotency_evidence_refs,
            semantic_result_ref: Some(&envelope.request.request_ref),
            gap_policy: delivery_idempotency::GapPolicy::Deny,
        })?;
        idempotency_receipt_ref = Some(delivery.receipt.receipt_ref.clone());
        import_node_artifact(input.state_root, &delivery.receipt.value)?;
        if delivery.should_commit_side_effect {
            let submitted = submit_control_request(&NodeControlSubmitInput {
                state_root: input.state_root,
                request_value: &envelope.request.value,
            })?;
            queue_receipt_ref = Some(submitted.queue_receipt_ref);
            has_enqueued = true;
        } else if delivery.receipt.decision == "duplicate" {
            queue_receipt_ref = prior_queue_receipt_ref(input.state_root, &envelope.request.request_ref).ok();
        } else {
            diagnostics.extend(delivery.receipt.diagnostics.iter().cloned());
            diagnostics.push(format!("node control ingress idempotency decision {}", delivery.receipt.decision));
        }
    }
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let receipt_value = ingress_receipt_value(&IngressReceiptValueInput {
        decision,
        phase: if has_enqueued { "deliver" } else { "duplicate-or-deny" },
        transport: &envelope.transport,
        envelope: &envelope,
        idempotency_receipt_ref: idempotency_receipt_ref.as_deref(),
        queue_receipt_ref: queue_receipt_ref.as_deref(),
        diagnostics: &diagnostics,
    })?;
    let ingress_receipt_ref = canonical_hash(&receipt_value)?;
    write_preserves(
        &control_ingress_receipt_path(input.state_root, &envelope.envelope_ref, "deliver"),
        &receipt_value,
    )?;
    import_node_artifact(input.state_root, &receipt_value)?;
    Ok(NodeControlIngressDeliver {
        envelope_ref: envelope.envelope_ref,
        request_ref: envelope.request.request_ref,
        ingress_receipt_ref,
        ingress_receipt_value: receipt_value,
        idempotency_receipt_ref,
        queue_receipt_ref,
        has_enqueued,
    })
}

fn ingress_pre_enqueue_diagnostics(
    state_root: &Path,
    topic: &str,
    envelope: &NodeControlIngressEnvelope,
) -> Result<Vec<String>> {
    let mut diagnostics = Vec::new();
    if !matches!(envelope.transport.as_str(), LOCAL_CONTROL_INGRESS_TRANSPORT | LIVE_CONTROL_INGRESS_TRANSPORT) {
        diagnostics.push(format!("unsupported node control ingress transport {}", envelope.transport));
    }
    if envelope.topic != topic {
        diagnostics.push(format!("node control ingress topic {} does not match requested {topic}", envelope.topic));
    }
    let identity = node_identity::parse_node_identity(&read_preserves(&state_root.join(IDENTITY_FILE))?)?;
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

fn evaluate_live_peer_bootstrap(state_root: &Path, envelope: &NodeControlIngressEnvelope) -> Result<Vec<String>> {
    let mut diagnostics = Vec::with_capacity(envelope.peer_bootstrap_refs.len().saturating_add(1));
    let mut admitted_peer_ref = None;
    for peer_ref in envelope.peer_bootstrap_refs.iter() {
        match read_node_ledger_artifact(state_root, peer_ref) {
            Ok(value) => match parse_node_control_live_peer_admission(&value) {
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
    envelope: &NodeControlIngressEnvelope,
    admission: &NodeControlLivePeerAdmission,
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
    match read_node_ledger_artifact(state_root, &admission.ticket_ref) {
        Ok(value) => match parse_node_control_live_ticket(&value) {
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

fn evaluate_live_authority_delegation(state_root: &Path, envelope: &NodeControlIngressEnvelope) -> Result<Vec<String>> {
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
        match read_node_ledger_artifact(state_root, authority_ref) {
            Ok(value) => match parse_node_control_authority_grant(&value) {
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
    let receipt_ref = canonical_hash(&receipt_value)?;
    write_preserves(&control_authority_receipt_path(state_root, &envelope.envelope_ref), &receipt_value)?;
    import_node_artifact(state_root, &receipt_value)?;
    if decision == "deny" {
        diagnostics.push(format!("node control authority receipt {receipt_ref} denied"));
    }
    Ok(diagnostics)
}

fn authority_grant_diagnostics(
    envelope: &NodeControlIngressEnvelope,
    grant: &NodeControlAuthorityGrant,
) -> Vec<String> {
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

fn ingress_idempotency_evidence_refs(envelope: &NodeControlIngressEnvelope) -> Vec<String> {
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
    canonical_hash(&receipt)
}

fn prior_dispatch_for_request(
    state_root: &Path,
    request: &node_runtime::NodeControlRequest,
) -> Result<Option<NodeControlDispatch>> {
    let receipt_path = control_outbox_receipt_path(state_root, &request.request_ref);
    if !receipt_path.exists() {
        return Ok(None);
    }
    let archived_path = control_outbox_request_path(state_root, &request.request_ref);
    if archived_path.exists() {
        let archived_value = read_preserves(&archived_path)?;
        let archived_ref = canonical_hash(&archived_value)?;
        if archived_ref != request.request_ref {
            return Err(MoltenError::invalid_harness(
                "node control duplicate request conflicts with archived request evidence",
            ));
        }
    }
    let control_receipt_value = read_preserves(&receipt_path)?;
    let control = node_runtime::parse_node_control_receipt(&control_receipt_value)?;
    if control.request_ref != request.request_ref {
        return Err(MoltenError::invalid_harness("node control duplicate receipt conflicts with request ref"));
    }
    Ok(Some(NodeControlDispatch {
        operation: request.operation.clone(),
        request_ref: request.request_ref.clone(),
        control_receipt_ref: control.receipt_ref,
        control_receipt_value: control.value,
        subreceipt_refs: control.subreceipt_refs,
    }))
}

fn write_dispatch_queue_receipt(
    state_root: &Path,
    request: &node_runtime::NodeControlRequest,
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
    let queue_receipt_ref = canonical_hash(&queue_receipt)?;
    write_preserves(&dispatch_receipt_path(state_root, &request.request_ref), &queue_receipt)?;
    import_node_artifact(state_root, &queue_receipt)?;
    Ok(queue_receipt_ref)
}

fn dispatch_status_request(
    state_root: &Path,
    request: &node_runtime::NodeControlRequest,
) -> Result<NodeControlDispatch> {
    let status = status_local_node_with_request(&NodeDaemonStatusInput { state_root }, request)?;
    write_preserves(&control_outbox_receipt_path(state_root, &request.request_ref), &status.control_receipt_value)?;
    Ok(NodeControlDispatch {
        operation: request.operation.clone(),
        request_ref: request.request_ref.clone(),
        control_receipt_ref: status.control_receipt_ref,
        control_receipt_value: status.control_receipt_value,
        subreceipt_refs: vec![status.health_ref],
    })
}

fn dispatch_shutdown_request(
    state_root: &Path,
    request: &node_runtime::NodeControlRequest,
) -> Result<NodeControlDispatch> {
    let stop = stop_local_node_with_request(&NodeDaemonStopInput { state_root }, request)?;
    write_preserves(&control_outbox_receipt_path(state_root, &request.request_ref), &stop.control_receipt_value)?;
    Ok(NodeControlDispatch {
        operation: request.operation.clone(),
        request_ref: request.request_ref.clone(),
        control_receipt_ref: stop.control_receipt_ref,
        control_receipt_value: stop.control_receipt_value,
        subreceipt_refs: vec![stop.shutdown_ref],
    })
}

#[derive(Debug, Clone, Copy)]
struct NodeControlProvenanceInput<'a> {
    state_root: &'a Path,
    request: &'a node_runtime::NodeControlRequest,
    artifact_ref: &'a str,
    operation: &'a str,
    subreceipt_kind: &'a str,
}

fn evaluate_node_control_provenance(
    input: &NodeControlProvenanceInput<'_>,
) -> Result<provenance::ProvenanceEvaluation> {
    let mut provenance_diagnostics = Vec::with_capacity(input.request.evidence_refs.len().saturating_add(1));
    if input.request.evidence_refs.is_empty() {
        provenance_diagnostics.push("node control provenance evidence refs missing".to_string());
    }
    let mut provenance_values = Vec::with_capacity(input.request.evidence_refs.len());
    for evidence_ref in &input.request.evidence_refs {
        match read_node_ledger_artifact(input.state_root, evidence_ref) {
            Ok(value) => provenance_values.push(value),
            Err(error) => provenance_diagnostics
                .push(format!("node control provenance evidence {evidence_ref} not found in node ledger: {error}")),
        }
    }
    let evaluation = provenance::evaluate_provenance(&provenance::ProvenanceEvaluationInput {
        operation: input.operation,
        profile: "node-control",
        artifact_ref: input.artifact_ref,
        provenance_values: &provenance_values,
        prior_diagnostics: &provenance_diagnostics,
    })?;
    write_preserves(
        &control_operation_subreceipt_path(input.state_root, &input.request.request_ref, input.subreceipt_kind),
        &evaluation.receipt_value,
    )?;
    import_node_artifact(input.state_root, &evaluation.receipt_value)?;
    Ok(evaluation)
}

fn dispatch_install_request(
    state_root: &Path,
    request: &node_runtime::NodeControlRequest,
) -> Result<NodeControlDispatch> {
    let startup = current_startup_receipt(state_root)?;
    let mut diagnostics = side_effect_preflight_diagnostics(request);
    let Some(payload_ref) = request.payload_ref.as_deref() else {
        diagnostics.push("node control install requires payload ref".to_string());
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
    let payload_value = match read_node_ledger_artifact(state_root, payload_ref) {
        Ok(value) => value,
        Err(error) => {
            diagnostics.push(format!("node control install payload not found in node ledger: {error}"));
            return finalize_operation_dispatch(&OperationFinalizeInput {
                state_root,
                request,
                startup_receipt_ref: &startup.receipt_ref,
                subreceipt_refs: &[],
                diagnostics: &diagnostics,
            });
        }
    };
    let provenance = evaluate_node_control_provenance(&NodeControlProvenanceInput {
        state_root,
        request,
        artifact_ref: payload_ref,
        operation: "install",
        subreceipt_kind: "artifact-provenance",
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
    let schema_refs = match request.target_ref.as_ref() {
        Some(target_ref) => vec![target_ref.clone()],
        None => vec![local_ref("node-control-install-schema", &request.request_ref)?],
    };
    let extra_evidence_refs = if request.target_ref.is_some() { 3 } else { 2 };
    let mut evidence_refs =
        Vec::with_capacity(request.resource_refs.len() + request.evidence_refs.len() + extra_evidence_refs);
    evidence_refs.extend(request.resource_refs.iter().cloned());
    evidence_refs.extend(request.evidence_refs.iter().cloned());
    evidence_refs.push(provenance_receipt_refs[0].clone());
    evidence_refs.push(payload_ref.to_string());
    if let Some(target_ref) = request.target_ref.as_ref() {
        evidence_refs.push(target_ref.clone());
    }
    let install = match artifacts::install_artifact(&state_root.join("registry"), &artifacts::ArtifactInstallInput {
        kind: "node-control-artifact".to_string(),
        payload: payload_value,
        schema_refs,
        dependency_refs: Vec::new(),
        effect_manifest_ref: None,
        policy_refs: request.policy_refs.clone(),
        evidence_refs,
        installer_ref: request.request_ref.clone(),
        capability_refs: request.authority_refs.clone(),
    }) {
        Ok(install) => install,
        Err(error) => {
            diagnostics.push(format!("node control artifact install failed: {error}"));
            return finalize_operation_dispatch(&OperationFinalizeInput {
                state_root,
                request,
                startup_receipt_ref: &startup.receipt_ref,
                subreceipt_refs: &provenance_receipt_refs,
                diagnostics: &diagnostics,
            });
        }
    };
    let install_receipt_ref = canonical_hash(&install.receipt_value)?;
    write_preserves(
        &control_operation_subreceipt_path(state_root, &request.request_ref, "artifact-install"),
        &install.receipt_value,
    )?;
    import_node_artifact(state_root, &install.receipt_value)?;
    if install.decision == "pass" {
        import_node_artifact(state_root, &install.artifact.value)?;
    } else if install.missing_dependencies.is_empty() {
        diagnostics.push("node control artifact install denied".to_string());
    } else {
        diagnostics
            .extend(install.missing_dependencies.iter().map(|reference| format!("missing dependency {reference}")));
    }
    finalize_operation_dispatch(&OperationFinalizeInput {
        state_root,
        request,
        startup_receipt_ref: &startup.receipt_ref,
        subreceipt_refs: &[provenance.receipt_ref, install_receipt_ref],
        diagnostics: &diagnostics,
    })
}

fn dispatch_run_request(state_root: &Path, request: &node_runtime::NodeControlRequest) -> Result<NodeControlDispatch> {
    let startup = current_startup_receipt(state_root)?;
    let mut diagnostics = side_effect_preflight_diagnostics(request);
    let Some(execution_request_ref) = request.payload_ref.as_deref() else {
        diagnostics.push("node control run requires execution request payload ref".to_string());
        return finalize_operation_dispatch(&OperationFinalizeInput {
            state_root,
            request,
            startup_receipt_ref: &startup.receipt_ref,
            subreceipt_refs: &[],
            diagnostics: &diagnostics,
        });
    };
    let Some(admission_ref) = request.target_ref.as_deref() else {
        diagnostics.push("node control run requires admission receipt target ref".to_string());
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
    let execution_request_value = match read_node_ledger_artifact(state_root, execution_request_ref) {
        Ok(value) => value,
        Err(error) => {
            diagnostics.push(format!("node control run execution request not found in node ledger: {error}"));
            return finalize_operation_dispatch(&OperationFinalizeInput {
                state_root,
                request,
                startup_receipt_ref: &startup.receipt_ref,
                subreceipt_refs: &[],
                diagnostics: &diagnostics,
            });
        }
    };
    let execution_request = match job_dag::parse_job_execution_request_value(&execution_request_value) {
        Ok(execution_request) => execution_request,
        Err(error) => {
            diagnostics.push(format!("node control run execution request malformed: {error}"));
            return finalize_operation_dispatch(&OperationFinalizeInput {
                state_root,
                request,
                startup_receipt_ref: &startup.receipt_ref,
                subreceipt_refs: &[],
                diagnostics: &diagnostics,
            });
        }
    };
    let provenance = evaluate_node_control_provenance(&NodeControlProvenanceInput {
        state_root,
        request,
        artifact_ref: &execution_request.job_ref,
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
    let admission_receipt_value = match read_node_ledger_artifact(state_root, admission_ref) {
        Ok(value) => value,
        Err(error) => {
            diagnostics.push(format!("node control run admission receipt not found in node ledger: {error}"));
            return finalize_operation_dispatch(&OperationFinalizeInput {
                state_root,
                request,
                startup_receipt_ref: &startup.receipt_ref,
                subreceipt_refs: &provenance_receipt_refs,
                diagnostics: &diagnostics,
            });
        }
    };
    let execution = job_dag::execution_loopback(job_dag::ExecutionLoopbackInput {
        target_registry: &state_root.join("registry"),
        storage_root: &state_root.join("storage"),
        cache_root: &state_root.join("cache"),
        chunk_root: &state_root.join("chunks"),
        admission_receipt_value: &admission_receipt_value,
        request_value: &execution_request_value,
    })?;
    write_preserves(
        &control_operation_subreceipt_path(state_root, &request.request_ref, "job-execution"),
        &execution.receipt_value,
    )?;
    import_node_artifact(state_root, &execution.receipt_value)?;
    let mut subreceipt_refs = Vec::with_capacity(3);
    subreceipt_refs.push(provenance.receipt_ref);
    subreceipt_refs.push(execution.receipt_ref.clone());
    if let Some(run) = execution.run.as_ref() {
        let run_ref = canonical_hash(&run.receipt_value)?;
        write_preserves(
            &control_operation_subreceipt_path(state_root, &request.request_ref, "job-run"),
            &run.receipt_value,
        )?;
        import_node_artifact(state_root, &run.receipt_value)?;
        subreceipt_refs.push(run_ref);
    }
    diagnostics.extend(execution.diagnostics.iter().cloned());
    if execution.decision != "pass" && diagnostics.is_empty() {
        diagnostics.push("node control run execution denied".to_string());
    }
    finalize_operation_dispatch(&OperationFinalizeInput {
        state_root,
        request,
        startup_receipt_ref: &startup.receipt_ref,
        subreceipt_refs: &subreceipt_refs,
        diagnostics: &diagnostics,
    })
}

fn dispatch_gate_request(state_root: &Path, request: &node_runtime::NodeControlRequest) -> Result<NodeControlDispatch> {
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
    let gate_value = match read_node_ledger_artifact(state_root, gate_receipt_ref) {
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
    let validation = octet_gate::validate_octet_source_gate(&octet_gate::OctetSourceGateValidationInput {
        consumer: "node-control-gate".to_string(),
        subject_ref: subject_ref.to_string(),
        gate_receipt_value: Some(gate_value),
        source_scope: octet_gate::default_source_scope("node-control-gate")?,
    })?;
    write_preserves(
        &control_operation_subreceipt_path(state_root, &request.request_ref, "octet-source-gate"),
        &validation.value,
    )?;
    import_node_artifact(state_root, &validation.value)?;
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

fn finalize_operation_dispatch(input: &OperationFinalizeInput<'_>) -> Result<NodeControlDispatch> {
    let decision = if input.diagnostics.is_empty() { "pass" } else { "deny" };
    let operation_receipt = operation_receipt_value(&OperationReceiptValueInput {
        decision,
        request: input.request,
        diagnostics: input.diagnostics,
    })?;
    let operation_receipt_ref = canonical_hash(&operation_receipt)?;
    write_preserves(&control_operation_receipt_path(input.state_root, &input.request.request_ref), &operation_receipt)?;
    import_node_artifact(input.state_root, &operation_receipt)?;
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
    let control_receipt_ref = canonical_hash(&control_receipt)?;
    write_preserves(&control_outbox_receipt_path(input.state_root, &input.request.request_ref), &control_receipt)?;
    import_node_artifact(input.state_root, &control_receipt)?;
    Ok(NodeControlDispatch {
        operation: input.request.operation.clone(),
        request_ref: input.request.request_ref.clone(),
        control_receipt_ref,
        control_receipt_value: control_receipt,
        subreceipt_refs: all_subreceipt_refs,
    })
}

fn side_effect_preflight_diagnostics(request: &node_runtime::NodeControlRequest) -> Vec<String> {
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

fn read_node_ledger_artifact(state_root: &Path, artifact_ref: &str) -> Result<IOValue> {
    ledger::read_artifact(&state_root.join("ledger"), artifact_ref)
}

fn control_receipt_for_request(
    state_root: &Path,
    request: &node_runtime::NodeControlRequest,
    startup_receipt_ref: &str,
    subreceipt_refs: &[String],
    diagnostics: &[String],
) -> Result<IOValue> {
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
    node_runtime::node_control_receipt_value(&node_runtime::ControlReceiptValueInput {
        decision: final_decision,
        request,
        startup_receipt_ref,
        authority_receipt_refs: &authority_receipt_refs,
        resource_receipt_refs: &resource_receipt_refs,
        subreceipt_refs,
        diagnostics: &receipt_diagnostics,
    })
}

fn authority_receipt_value(input: &AuthorityReceiptValueInput<'_>) -> Result<IOValue> {
    validate_decision(input.decision)?;
    Ok(record("node-control-authority-receipt-v1", vec![
        string(NODE_CONTROL_AUTHORITY_RECEIPT_SCHEMA),
        record("decision", vec![string(input.decision)]),
        record("envelope", vec![string(&input.envelope.envelope_ref)]),
        record("request", vec![string(&input.envelope.request.request_ref)]),
        record("from-peer", vec![string(&input.envelope.from_peer)]),
        record("to-node", vec![string(&input.envelope.to_node)]),
        record("operation", vec![string(&input.envelope.request.operation)]),
        record("grant", vec![optional_string(input.grant_ref)]),
        record("diagnostics", vec![sequence(input.diagnostics.iter().map(string).collect())]),
        record("checks", vec![sequence(vec![
            record("check", vec![
                string("peer-node-bound"),
                string(if input.grant_ref.is_some() { "pass" } else { "fail" }),
            ]),
            record("check", vec![
                string("operation-scope-bound"),
                string(if input.grant_ref.is_some() { "pass" } else { "fail" }),
            ]),
            record("check", vec![string("revocation-checked-at-ingress"), string("pass")]),
            record("check", vec![string("transport-is-not-authority"), string("pass")]),
        ])]),
    ]))
}

fn live_listener_receipt_value(input: &ListenerReceiptValueInput<'_>) -> Result<IOValue> {
    validate_decision(input.decision)?;
    Ok(record("node-control-live-listener-receipt-v1", vec![
        string(NODE_CONTROL_LIVE_LISTENER_RECEIPT_SCHEMA),
        record("decision", vec![string(input.decision)]),
        record("startup", vec![string(input.startup_receipt_ref)]),
        record("node", vec![string(input.node_id)]),
        record("logical-endpoint", vec![string(input.logical_endpoint_id)]),
        record("bound-endpoint", vec![string(input.bound_endpoint_id)]),
        record("topic", vec![string(input.topic)]),
        record("max-events", vec![string(input.max_events.to_string())]),
        record("observed-events", vec![string(input.observed_events.to_string())]),
        record("transport-receipts", vec![sequence(input.transport_receipt_refs.iter().map(string).collect())]),
        record("neighbor-events", vec![sequence(input.neighbor_events.iter().map(string).collect())]),
        record("service-run", vec![string(input.service_receipt_ref)]),
        record("diagnostics", vec![sequence(input.diagnostics.iter().map(string).collect())]),
        record("checks", vec![sequence(vec![
            record("check", vec![string("live-iroh-listener"), string("pass")]),
            record("check", vec![string("receive-before-drain"), string("pass")]),
            record("check", vec![string("session-evidence-not-authority"), string("pass")]),
            record("check", vec![string("bounded-listener"), string("pass")]),
            record("check", vec![string("durable-inbox-boundary"), string("pass")]),
        ])]),
    ]))
}

fn live_transport_receipt_value(input: &LiveTransportReceiptValueInput<'_>) -> Result<IOValue> {
    validate_decision(input.decision)?;
    let has_peer_bootstrap = !input.envelope.peer_bootstrap_refs.is_empty();
    Ok(record("node-control-live-transport-receipt-v1", vec![
        string(NODE_CONTROL_LIVE_TRANSPORT_RECEIPT_SCHEMA),
        record("operation", vec![string(input.operation)]),
        record("decision", vec![string(input.decision)]),
        record("transport", vec![string(LIVE_CONTROL_INGRESS_TRANSPORT)]),
        record("topic", vec![string(&input.envelope.topic)]),
        record("node", vec![string(input.node_id)]),
        record("delivered-from", vec![optional_string(input.delivered_from)]),
        record("envelope", vec![string(&input.envelope.envelope_ref)]),
        record("ingress-receipt", vec![optional_string(input.ingress_receipt_ref)]),
        record("diagnostics", vec![sequence(input.diagnostics.iter().map(string).collect())]),
        record("checks", vec![sequence(vec![
            record("check", vec![string("canonical-envelope-ref"), string("pass")]),
            record("check", vec![string("live-iroh-gossip"), string("pass")]),
            record("check", vec![
                string("peer-bootstrap-before-enqueue"),
                string(if has_peer_bootstrap { "pass" } else { "fail" }),
            ]),
            record("check", vec![string("transport-is-not-authority"), string("pass")]),
            record("check", vec![string("durable-inbox-boundary"), string("pass")]),
        ])]),
    ]))
}

fn live_workflow_receipt_value(input: &LiveWorkflowReceiptValueInput<'_>) -> Result<IOValue> {
    validate_decision(input.decision)?;
    Ok(record("node-control-live-workflow-receipt-v1", vec![
        string(NODE_CONTROL_LIVE_WORKFLOW_RECEIPT_SCHEMA),
        record("decision", vec![string(input.decision)]),
        record("topic", vec![string(&input.ticket.topic)]),
        record("peer", vec![string(&input.admission.peer_id)]),
        record("node", vec![string(&input.ticket.node_id)]),
        record("receiver-ticket", vec![string(&input.ticket.ticket_ref)]),
        record("peer-admission", vec![string(&input.admission.admission_ref)]),
        record("authority-grant", vec![string(&input.authority.grant_ref)]),
        record("send-receipt", vec![string(&input.send.receipt_ref)]),
        record("receive-receipts", vec![sequence(input.receive_receipt_refs.iter().map(string).collect())]),
        record("listener-receipt", vec![optional_string(input.listener_receipt_ref)]),
        record("service-run", vec![string(input.service_receipt_ref)]),
        record("diagnostics", vec![sequence(input.diagnostics.iter().map(string).collect())]),
        record("checks", vec![sequence(vec![
            record("check", vec![
                string("ticket-admission-bound"),
                string(if input.admission.ticket_ref == input.ticket.ticket_ref {
                    "pass"
                } else {
                    "fail"
                }),
            ]),
            record("check", vec![
                string("authority-grant-bound"),
                string(
                    if input.authority.peer_id == input.admission.peer_id
                        && input.authority.node_id == input.ticket.node_id
                    {
                        "pass"
                    } else {
                        "fail"
                    },
                ),
            ]),
            record("check", vec![
                string("send-ticket-bound"),
                string(if input.send.receiver_ticket_ref == input.ticket.ticket_ref {
                    "pass"
                } else {
                    "fail"
                }),
            ]),
            record("check", vec![
                string("receive-before-service"),
                string(if input.receive_receipt_refs.is_empty() {
                    "fail"
                } else {
                    "pass"
                }),
            ]),
            record("check", vec![string("transport-is-not-authority"), string("pass")]),
            record("check", vec![string("durable-inbox-boundary"), string("pass")]),
        ])]),
    ]))
}

fn live_send_receipt_value(input: &LiveSendReceiptValueInput<'_>) -> Result<IOValue> {
    validate_decision(input.decision)?;
    let has_addresses = !input.ticket.address_refs.is_empty();
    let has_operation_mismatch = diagnostics_include(input.diagnostics, "operation-id");
    let has_supported_addresses = has_addresses
        && !diagnostics_include(input.diagnostics, "unsupported transport address")
        && !diagnostics_include(input.diagnostics, "address unsupported or malformed")
        && !diagnostics_include(input.diagnostics, "address parse failed")
        && !diagnostics_include(input.diagnostics, "endpoint parse failed");
    let has_expected_ticket_binding = !diagnostics_include(input.diagnostics, "ticket node")
        && !diagnostics_include(input.diagnostics, "ticket topic")
        && !diagnostics_include(input.diagnostics, "ticket endpoint");
    let has_state_root_evidence = !diagnostics_include(input.diagnostics, "sender state root")
        && !diagnostics_include(input.diagnostics, "peer admission refs missing")
        && !diagnostics_include(input.diagnostics, "authority grant refs missing");
    let has_transport_success = input.transport_receipt_ref.is_some();
    Ok(record("node-control-live-send-receipt-v1", vec![
        string(NODE_CONTROL_LIVE_SEND_RECEIPT_SCHEMA),
        record("decision", vec![string(input.decision)]),
        record("transport", vec![string(LIVE_CONTROL_INGRESS_TRANSPORT)]),
        record("topic", vec![string(&input.envelope.topic)]),
        record("from-peer", vec![string(input.from_peer)]),
        record("to-node", vec![string(&input.ticket.node_id)]),
        record("receiver-ticket", vec![string(&input.ticket.ticket_ref)]),
        record("receiver-endpoint", vec![string(&input.ticket.live_endpoint_id)]),
        record("receiver-addresses", vec![sequence(input.ticket.address_refs.iter().map(string).collect())]),
        record("envelope", vec![string(&input.envelope.envelope_ref)]),
        record("transport-receipt", vec![optional_string(input.transport_receipt_ref)]),
        record("diagnostics", vec![sequence(input.diagnostics.iter().map(string).collect())]),
        record("checks", vec![sequence(vec![
            record("check", vec![string("receiver-ticket-bound"), string("pass")]),
            record("check", vec![
                string("receiver-address-bound"),
                string(if has_addresses { "pass" } else { "fail" }),
            ]),
            record("check", vec![
                string("receiver-address-supported"),
                string(if has_supported_addresses { "pass" } else { "fail" }),
            ]),
            record("check", vec![
                string("receiver-ticket-expected"),
                string(if has_expected_ticket_binding { "pass" } else { "fail" }),
            ]),
            record("check", vec![
                string("operation-id-bound"),
                string(if has_operation_mismatch { "fail" } else { "pass" }),
            ]),
            record("check", vec![
                string("sender-state-root-evidence"),
                string(if has_state_root_evidence { "pass" } else { "fail" }),
            ]),
            record("check", vec![
                string("join-or-publish-succeeded"),
                string(if has_transport_success { "pass" } else { "fail" }),
            ]),
            record("check", vec![string("canonical-envelope-ref"), string("pass")]),
            record("check", vec![string("live-iroh-gossip"), string("pass")]),
            record("check", vec![string("transport-is-not-authority"), string("pass")]),
            record("check", vec![string("durable-inbox-boundary"), string("pass")]),
        ])]),
    ]))
}

fn live_send_retry_receipt_value(input: &LiveSendRetryReceiptValueInput<'_>) -> Result<IOValue> {
    validate_decision(input.decision)?;
    Ok(record("node-control-live-send-retry-receipt-v1", vec![
        string(NODE_CONTROL_LIVE_SEND_RETRY_RECEIPT_SCHEMA),
        record("decision", vec![string(input.decision)]),
        record("attempt", vec![string(input.attempt.to_string())]),
        record("max-attempts", vec![string(input.max_attempts.to_string())]),
        record("transport", vec![string(LIVE_CONTROL_INGRESS_TRANSPORT)]),
        record("topic", vec![string(&input.envelope.topic)]),
        record("from-peer", vec![string(input.from_peer)]),
        record("to-node", vec![string(&input.ticket.node_id)]),
        record("receiver-ticket", vec![string(&input.ticket.ticket_ref)]),
        record("receiver-endpoint", vec![string(&input.ticket.live_endpoint_id)]),
        record("envelope", vec![string(&input.envelope.envelope_ref)]),
        record("operation", vec![string(&input.envelope.operation_ref)]),
        record("diagnostics", vec![sequence(input.diagnostics.iter().map(string).collect())]),
        record("checks", vec![sequence(vec![
            record("check", vec![string("bounded-retry"), string("pass")]),
            record("check", vec![string("operation-id-bound"), string("pass")]),
            record("check", vec![string("transport-is-not-authority"), string("pass")]),
        ])]),
    ]))
}

fn live_send_duplicate_receipt_value(input: &LiveSendDuplicateReceiptValueInput<'_>) -> Result<IOValue> {
    Ok(record("node-control-live-send-duplicate-receipt-v1", vec![
        string(NODE_CONTROL_LIVE_SEND_DUPLICATE_RECEIPT_SCHEMA),
        record("decision", vec![string("pass")]),
        record("transport", vec![string(LIVE_CONTROL_INGRESS_TRANSPORT)]),
        record("topic", vec![string(&input.envelope.topic)]),
        record("from-peer", vec![string(input.from_peer)]),
        record("to-node", vec![string(&input.ticket.node_id)]),
        record("receiver-ticket", vec![string(&input.ticket.ticket_ref)]),
        record("receiver-endpoint", vec![string(&input.ticket.live_endpoint_id)]),
        record("envelope", vec![string(&input.envelope.envelope_ref)]),
        record("operation", vec![string(&input.envelope.operation_ref)]),
        record("prior-send-receipt", vec![string(input.prior_send_receipt_ref)]),
        record("diagnostics", vec![sequence(input.diagnostics.iter().map(string).collect())]),
        record("checks", vec![sequence(vec![
            record("check", vec![string("duplicate-side-effect-suppressed"), string("pass")]),
            record("check", vec![string("operation-id-bound"), string("pass")]),
            record("check", vec![string("prior-send-receipt-bound"), string("pass")]),
            record("check", vec![string("transport-is-not-authority"), string("pass")]),
        ])]),
    ]))
}

fn service_lock_value(input: &ServiceLockValueInput<'_>) -> Result<IOValue> {
    Ok(record("node-control-service-lock-v1", vec![
        string(NODE_CONTROL_SERVICE_LOCK_SCHEMA),
        record("state-root", vec![string(&state_root_profile_ref(input.state_root)?)]),
        record("startup", vec![string(input.startup_receipt_ref)]),
        record("node", vec![string(input.node_id)]),
        record("topic", vec![string(input.topic)]),
        record("max-ticks", vec![string(input.max_ticks.to_string())]),
        record("max-requests-per-tick", vec![string(input.max_requests_per_tick.to_string())]),
        record("service-run", vec![string(input.service_run_ref)]),
        record("profile", vec![string("local-supervised-node-control-v1")]),
        record("checks", vec![sequence(vec![
            record("check", vec![string("startup-bound"), string("pass")]),
            record("check", vec![string("single-active-service"), string("pass")]),
            record("check", vec![string("bounded-ticks"), string("pass")]),
            record("check", vec![string("not-authority-token"), string("pass")]),
        ])]),
    ]))
}

fn service_heartbeat_receipt_value(input: &ServiceHeartbeatValueInput<'_>) -> Result<IOValue> {
    Ok(record("node-control-service-heartbeat-receipt-v1", vec![
        string(NODE_CONTROL_SERVICE_HEARTBEAT_RECEIPT_SCHEMA),
        record("decision", vec![string(if input.diagnostics.is_empty() { "pass" } else { "deny" })]),
        record("startup", vec![string(input.startup_receipt_ref)]),
        record("service-lock", vec![string(input.service_lock_ref)]),
        record("tick", vec![string(input.tick.to_string())]),
        record("delivered-count", vec![string(input.delivered_count.to_string())]),
        record("processed-count", vec![string(input.processed_count.to_string())]),
        record("diagnostics", vec![sequence(input.diagnostics.iter().map(string).collect())]),
        record("checks", vec![sequence(vec![
            record("check", vec![string("service-lock-bound"), string("pass")]),
            record("check", vec![string("startup-bound"), string("pass")]),
            record("check", vec![string("monotonic-tick"), string("pass")]),
        ])]),
    ]))
}

fn supervisor_receipt_value(input: &SupervisorReceiptValueInput<'_>) -> Result<IOValue> {
    validate_decision(input.decision)?;
    Ok(record("node-control-supervisor-receipt-v1", vec![
        string(NODE_CONTROL_SUPERVISOR_RECEIPT_SCHEMA),
        record("decision", vec![string(input.decision)]),
        record("operation", vec![string(input.operation)]),
        record("startup", vec![string(input.startup_receipt_ref)]),
        record("service-lock", vec![optional_string(input.service_lock_ref)]),
        record("policy", vec![optional_string(input.supervisor_policy_ref)]),
        record("topic", vec![string(input.topic)]),
        record("diagnostics", vec![sequence(input.diagnostics.iter().map(string).collect())]),
        record("checks", vec![sequence(vec![
            record("check", vec![
                string("supervisor-policy-bound"),
                string(if input.supervisor_policy_ref.is_some() {
                    "pass"
                } else {
                    "fail"
                }),
            ]),
            record("check", vec![string("single-active-service"), string("pass")]),
            record("check", vec![string("bounded-restart-policy"), string("pass")]),
            record("check", vec![string("shutdown-drain-bound"), string("pass")]),
        ])]),
    ]))
}

fn service_run_receipt_value(input: &ServiceRunReceiptValueInput<'_>) -> Result<IOValue> {
    validate_decision(input.decision)?;
    Ok(record("node-control-service-run-receipt-v1", vec![
        string(NODE_CONTROL_SERVICE_RUN_RECEIPT_SCHEMA),
        record("decision", vec![string(input.decision)]),
        record("startup", vec![string(input.startup_receipt_ref)]),
        record("service-lock", vec![optional_string(input.service_lock_ref)]),
        record("topic", vec![string(input.topic)]),
        record("max-ticks", vec![string(input.max_ticks.to_string())]),
        record("max-requests-per-tick", vec![string(input.max_requests_per_tick.to_string())]),
        record("ticks", vec![string(input.ticks.to_string())]),
        record("heartbeats", vec![sequence(input.heartbeat_receipt_refs.iter().map(string).collect())]),
        record("ingress-receipts", vec![sequence(input.ingress_receipt_refs.iter().map(string).collect())]),
        record("loop-receipts", vec![sequence(input.loop_receipt_refs.iter().map(string).collect())]),
        record("processed-requests", vec![sequence(input.processed_request_refs.iter().map(string).collect())]),
        record("stopped", vec![string(if input.has_stopped { "true" } else { "false" })]),
        record("supervisor-policy", vec![optional_string(input.supervisor_policy_ref)]),
        record("supervisor-receipts", vec![sequence(input.supervisor_receipt_refs.iter().map(string).collect())]),
        record("diagnostics", vec![sequence(input.diagnostics.iter().map(string).collect())]),
        record("checks", vec![sequence(vec![
            record("check", vec![
                string("single-active-service"),
                string(if input.service_lock_ref.is_some() {
                    "pass"
                } else {
                    "fail"
                }),
            ]),
            record("check", vec![string("ingress-before-loop"), string("pass")]),
            record("check", vec![string("loop-reuse"), string("pass")]),
            record("check", vec![string("shutdown-stop-semantics"), string("pass")]),
            record("check", vec![string("bounded-ticks"), string("pass")]),
            record("check", vec![
                string("supervisor-policy-bound"),
                string(if input.supervisor_policy_ref.is_none() || !input.supervisor_receipt_refs.is_empty() {
                    "pass"
                } else {
                    "fail"
                }),
            ]),
        ])]),
    ]))
}

fn ingress_envelope_value(
    input: &NodeControlIngressEnvelopeInput<'_>,
    request: &node_runtime::NodeControlRequest,
    operation_ref: &str,
    transport: &str,
    transport_check: &str,
) -> Result<IOValue> {
    Ok(record("node-control-ingress-envelope-v1", vec![
        string(NODE_CONTROL_INGRESS_ENVELOPE_SCHEMA),
        record("transport", vec![string(transport)]),
        record("topic", vec![string(input.topic)]),
        record("from-peer", vec![string(input.from_peer)]),
        record("to-node", vec![string(input.to_node)]),
        record("sequence", vec![string(input.sequence.to_string())]),
        record("operation", vec![string(operation_ref)]),
        record("request-ref", vec![string(&request.request_ref)]),
        record("request", vec![request.value.clone()]),
        record("peer-bootstrap", vec![sequence(input.peer_bootstrap_refs.iter().map(string).collect())]),
        record("authority", vec![sequence(input.authority_refs.iter().map(string).collect())]),
        record("policy", vec![sequence(input.policy_refs.iter().map(string).collect())]),
        record("resource", vec![sequence(input.resource_refs.iter().map(string).collect())]),
        record("evidence", vec![sequence(input.evidence_refs.iter().map(string).collect())]),
        record("checks", vec![sequence(vec![
            record("check", vec![string("canonical-request-ref"), string("pass")]),
            record("check", vec![string("operation-id-bound"), string("pass")]),
            record("check", vec![string(transport_check), string("pass")]),
            record("check", vec![string("transport-is-not-authority"), string("pass")]),
        ])]),
    ]))
}

fn ingress_receipt_value(input: &IngressReceiptValueInput<'_>) -> Result<IOValue> {
    validate_decision(input.decision)?;
    let has_peer_bootstrap = !input.envelope.peer_bootstrap_refs.is_empty();
    let has_authority = !input.envelope.authority_refs.is_empty() && !input.envelope.request.authority_refs.is_empty();
    let has_policy = !input.envelope.policy_refs.is_empty() && !input.envelope.request.policy_refs.is_empty();
    let has_resource = !input.envelope.resource_refs.is_empty() && !input.envelope.request.resource_refs.is_empty();
    Ok(record("node-control-ingress-receipt-v1", vec![
        string(NODE_CONTROL_INGRESS_RECEIPT_SCHEMA),
        record("decision", vec![string(input.decision)]),
        record("phase", vec![string(input.phase)]),
        record("transport", vec![string(input.transport)]),
        record("topic", vec![string(&input.envelope.topic)]),
        record("from-peer", vec![string(&input.envelope.from_peer)]),
        record("to-node", vec![string(&input.envelope.to_node)]),
        record("sequence", vec![string(input.envelope.sequence.to_string())]),
        record("envelope", vec![string(&input.envelope.envelope_ref)]),
        record("operation", vec![string(&input.envelope.operation_ref)]),
        record("request", vec![string(&input.envelope.request.request_ref)]),
        record("idempotency", vec![optional_string(input.idempotency_receipt_ref)]),
        record("queue", vec![optional_string(input.queue_receipt_ref)]),
        record("diagnostics", vec![sequence(input.diagnostics.iter().map(string).collect())]),
        record("checks", vec![sequence(vec![
            record("check", vec![
                string("peer-bootstrap-bound"),
                string(if has_peer_bootstrap { "pass" } else { "fail" }),
            ]),
            record("check", vec![
                string("authority-before-enqueue"),
                string(if has_authority { "pass" } else { "fail" }),
            ]),
            record("check", vec![
                string("authority-delegation-before-enqueue"),
                string(if input.envelope.transport != LIVE_CONTROL_INGRESS_TRANSPORT || input.decision == "pass" {
                    "pass"
                } else {
                    "fail"
                }),
            ]),
            record("check", vec![
                string("policy-before-enqueue"),
                string(if has_policy { "pass" } else { "fail" }),
            ]),
            record("check", vec![
                string("resource-before-enqueue"),
                string(if has_resource { "pass" } else { "fail" }),
            ]),
            record("check", vec![
                string("delivery-idempotency-before-enqueue"),
                string(
                    if input.phase == "publish" || input.idempotency_receipt_ref.is_some() || input.decision == "deny" {
                        "pass"
                    } else {
                        "fail"
                    },
                ),
            ]),
            record("check", vec![string("durable-inbox-boundary"), string("pass")]),
        ])]),
    ]))
}

fn queue_receipt_value(input: &QueueReceiptValueInput<'_>) -> Result<IOValue> {
    validate_decision(input.decision)?;
    Ok(record("node-control-queue-receipt-v1", vec![
        string(NODE_CONTROL_QUEUE_RECEIPT_SCHEMA),
        record("decision", vec![string(input.decision)]),
        record("phase", vec![string(input.phase)]),
        record("operation", vec![string(input.operation)]),
        record("request", vec![string(input.request_ref)]),
        record("profile", vec![string("local-preserves-control-file-v1")]),
        record("location", vec![string(input.location_ref)]),
        record("diagnostics", vec![sequence(input.diagnostics.iter().map(string).collect())]),
        record("checks", vec![sequence(vec![
            record("check", vec![string("canonical-request-ref"), string("pass")]),
            record("check", vec![string("durable-control-profile"), string("pass")]),
            record("check", vec![string("explicit-state-root"), string("pass")]),
        ])]),
    ]))
}

fn operation_receipt_value(input: &OperationReceiptValueInput<'_>) -> Result<IOValue> {
    validate_decision(input.decision)?;
    Ok(record("node-control-operation-receipt-v1", vec![
        string(NODE_CONTROL_OPERATION_RECEIPT_SCHEMA),
        record("decision", vec![string(input.decision)]),
        record("operation", vec![string(&input.request.operation)]),
        record("request", vec![string(&input.request.request_ref)]),
        record("target", vec![optional_string(input.request.target_ref.as_deref())]),
        record("payload", vec![optional_string(input.request.payload_ref.as_deref())]),
        record("diagnostics", vec![sequence(input.diagnostics.iter().map(string).collect())]),
        record("checks", vec![sequence(vec![
            record("check", vec![string("operation-dispatch-explicit"), string("pass")]),
            record("check", vec![string("side-effects-receipted"), string("pass")]),
            record("check", vec![string("canonical-receipt"), string("pass")]),
        ])]),
    ]))
}

fn heartbeat_receipt_value(input: &HeartbeatReceiptValueInput<'_>) -> Result<IOValue> {
    Ok(record("node-control-heartbeat-receipt-v1", vec![
        string(NODE_CONTROL_HEARTBEAT_RECEIPT_SCHEMA),
        record("decision", vec![string(if input.diagnostics.is_empty() { "pass" } else { "deny" })]),
        record("startup", vec![string(input.startup_receipt_ref)]),
        record("lock", vec![string(input.lock_ref)]),
        record("loop-sequence", vec![string(input.loop_sequence.to_string())]),
        record("processed-count", vec![string(input.processed_count.to_string())]),
        record("profile", vec![string("local-preserves-control-loop-v1")]),
        record("diagnostics", vec![sequence(input.diagnostics.iter().map(string).collect())]),
        record("checks", vec![sequence(vec![
            record("check", vec![string("active-lock-bound"), string("pass")]),
            record("check", vec![string("heartbeat-is-receipted"), string("pass")]),
            record("check", vec![string("no-ambient-socket-authority"), string("pass")]),
        ])]),
    ]))
}

fn loop_receipt_value(input: &LoopReceiptValueInput<'_>) -> Result<IOValue> {
    validate_decision(input.decision)?;
    Ok(record("node-control-loop-receipt-v1", vec![
        string(NODE_CONTROL_LOOP_RECEIPT_SCHEMA),
        record("decision", vec![string(input.decision)]),
        record("startup", vec![string(input.startup_receipt_ref)]),
        record("heartbeat", vec![string(input.heartbeat_receipt_ref)]),
        record("max-requests", vec![string(input.max_requests.to_string())]),
        record("processed-requests", vec![sequence(input.processed_request_refs.iter().map(string).collect())]),
        record("dispatch-receipts", vec![sequence(input.dispatch_receipt_refs.iter().map(string).collect())]),
        record("stopped", vec![string(if input.has_stopped { "yes" } else { "no" })]),
        record("profile", vec![string("local-preserves-control-loop-v1")]),
        record("diagnostics", vec![sequence(input.diagnostics.iter().map(string).collect())]),
        record("checks", vec![sequence(vec![
            record("check", vec![string("bounded-request-loop"), string("pass")]),
            record("check", vec![string("deterministic-inbox-order"), string("pass")]),
            record("check", vec![string("idempotent-request-dispatch"), string("pass")]),
            record("check", vec![string("shutdown-stops-loop"), string("pass")]),
        ])]),
    ]))
}

pub fn node_daemon_summary(value: &IOValue) -> Result<String> {
    if let Ok(config) = node_runtime::parse_node_config(value) {
        return Ok(format!(
            "node config ref={} identity={} adapters={}",
            config.config_ref,
            config.node_identity_ref,
            config.adapters.len()
        ));
    }
    if let Ok(startup) = node_runtime::parse_node_startup_receipt(value) {
        return Ok(format!(
            "node startup decision={} receipt={} adapters={}",
            startup.decision,
            startup.receipt_ref,
            startup.adapters.len()
        ));
    }
    if let Ok(control) = node_runtime::parse_node_control_receipt(value) {
        return Ok(format!(
            "node control decision={} receipt={} request={}",
            control.decision, control.receipt_ref, control.request_ref
        ));
    }
    if let Ok(ingress) = parse_node_control_ingress_envelope(value) {
        return Ok(format!(
            "node control ingress envelope ref={} topic={} from={} to={} request={}",
            ingress.envelope_ref, ingress.topic, ingress.from_peer, ingress.to_node, ingress.request.request_ref
        ));
    }
    if let Some(fields) = value.collect_simple_record("node-control-ingress-receipt-v1", Some(15)) {
        require_schema(&fields[0], NODE_CONTROL_INGRESS_RECEIPT_SCHEMA, "node control ingress receipt")?;
        return Ok(format!(
            "node control ingress decision={} phase={} envelope={} request={}",
            record_string(&fields[1], "decision")?,
            record_string(&fields[2], "phase")?,
            record_string(&fields[8], "envelope")?,
            record_string(&fields[10], "request")?
        ));
    }
    if let Some(fields) = value.collect_simple_record("node-control-live-ticket-import-receipt-v1", Some(13)) {
        require_schema(
            &fields[0],
            NODE_CONTROL_LIVE_TICKET_IMPORT_RECEIPT_SCHEMA,
            "node control live ticket import receipt",
        )?;
        return Ok(format!(
            "node control live ticket import decision={} ticket={} imported={}",
            record_string(&fields[1], "decision")?,
            record_string(&fields[3], "ticket")?,
            record_sequence_len(&fields[10], "imported")?
        ));
    }
    if let Some(fields) = value.collect_simple_record("node-control-authority-grant-import-receipt-v1", Some(13)) {
        require_schema(
            &fields[0],
            NODE_CONTROL_AUTHORITY_GRANT_IMPORT_RECEIPT_SCHEMA,
            "node control authority grant import receipt",
        )?;
        return Ok(format!(
            "node control authority grant import decision={} grant={} imported={}",
            record_string(&fields[1], "decision")?,
            record_string(&fields[3], "grant")?,
            record_sequence_len(&fields[10], "imported")?
        ));
    }
    if let Ok(ticket) = parse_node_control_live_ticket(value) {
        return Ok(format!(
            "node control live ticket ref={} node={} topic={} endpoint={}",
            ticket.ticket_ref, ticket.node_id, ticket.topic, ticket.live_endpoint_id
        ));
    }
    if let Ok(admission) = parse_node_control_live_peer_admission(value) {
        return Ok(format!(
            "node control live peer admission decision={} peer={} node={} topic={}",
            admission.decision, admission.peer_id, admission.node_id, admission.topic
        ));
    }
    if let Ok(grant) = parse_node_control_authority_grant(value) {
        return Ok(format!(
            "node control authority grant ref={} peer={} node={} operations={}",
            grant.grant_ref,
            grant.peer_id,
            grant.node_id,
            grant.operations.join(",")
        ));
    }
    if let Some(fields) = value.collect_simple_record("node-control-authority-receipt-v1", Some(10)) {
        require_schema(&fields[0], NODE_CONTROL_AUTHORITY_RECEIPT_SCHEMA, "node control authority receipt")?;
        return Ok(format!(
            "node control authority decision={} envelope={} operation={} grant={}",
            record_string(&fields[1], "decision")?,
            record_string(&fields[2], "envelope")?,
            record_string(&fields[6], "operation")?,
            record_optional_string(&fields[7], "grant")?.unwrap_or_else(|| "none".to_string())
        ));
    }
    if let Some(fields) = value.collect_simple_record("node-control-live-listener-receipt-v1", Some(14)) {
        require_schema(&fields[0], NODE_CONTROL_LIVE_LISTENER_RECEIPT_SCHEMA, "node control live listener receipt")?;
        return Ok(format!(
            "node control live listener decision={} topic={} events={} service={}",
            record_string(&fields[1], "decision")?,
            record_string(&fields[6], "topic")?,
            record_string(&fields[8], "observed-events")?,
            record_string(&fields[11], "service-run")?
        ));
    }
    if let Some(fields) = value.collect_simple_record("node-control-live-workflow-receipt-v1", Some(14)) {
        require_schema(&fields[0], NODE_CONTROL_LIVE_WORKFLOW_RECEIPT_SCHEMA, "node control live workflow receipt")?;
        return Ok(format!(
            "node control live workflow decision={} peer={} node={} send={} service={}",
            record_string(&fields[1], "decision")?,
            record_string(&fields[3], "peer")?,
            record_string(&fields[4], "node")?,
            record_string(&fields[8], "send-receipt")?,
            record_string(&fields[11], "service-run")?
        ));
    }
    if let Some(fields) = value.collect_simple_record("node-control-live-workflow-bundle-v1", Some(10)) {
        require_schema(&fields[0], NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_SCHEMA, "node control live workflow bundle")?;
        return Ok(format!(
            "node control live workflow bundle ticket={} admission={} grant={} receipts={}",
            record_string(&fields[5], "ticket-ref")?,
            record_string(&fields[6], "peer-admission-ref")?,
            record_string(&fields[7], "authority-grant-ref")?,
            record_sequence_len(&fields[8], "receipt-refs")?
        ));
    }
    if let Some(fields) = value.collect_simple_record("node-control-live-workflow-bundle-export-receipt-v1", Some(9)) {
        require_schema(
            &fields[0],
            NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_EXPORT_RECEIPT_SCHEMA,
            "node control live workflow bundle export receipt",
        )?;
        return Ok(format!(
            "node control live workflow bundle export decision={} bundle={} receipts={}",
            record_string(&fields[1], "decision")?,
            record_string(&fields[2], "bundle")?,
            record_sequence_len(&fields[6], "receipts")?
        ));
    }
    if let Some(fields) = value.collect_simple_record("node-control-live-workflow-bundle-import-receipt-v1", Some(12)) {
        require_schema(
            &fields[0],
            NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_IMPORT_RECEIPT_SCHEMA,
            "node control live workflow bundle import receipt",
        )?;
        return Ok(format!(
            "node control live workflow bundle import decision={} bundle={} imported={}",
            record_string(&fields[1], "decision")?,
            record_string(&fields[3], "bundle")?,
            record_sequence_len(&fields[9], "imported")?
        ));
    }
    if let Some(fields) = value.collect_simple_record("node-control-live-workflow-bundle-verify-receipt-v1", Some(10)) {
        require_schema(
            &fields[0],
            NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_VERIFY_RECEIPT_SCHEMA,
            "node control live workflow bundle verify receipt",
        )?;
        return Ok(format!(
            "node control live workflow bundle verify decision={} bundle={} receipts={}",
            record_string(&fields[1], "decision")?,
            record_string(&fields[2], "bundle")?,
            record_sequence_len(&fields[6], "receipts")?
        ));
    }
    if let Some(fields) = value.collect_simple_record("node-control-live-workflow-bundle-gate-receipt-v1", Some(12)) {
        require_schema(
            &fields[0],
            NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_GATE_RECEIPT_SCHEMA,
            "node control live workflow bundle gate receipt",
        )?;
        return Ok(format!(
            "node control live workflow bundle gate decision={} bundle={} verify={}",
            record_string(&fields[1], "decision")?,
            record_string(&fields[2], "bundle")?,
            record_optional_string(&fields[3], "verify-receipt")?.unwrap_or_else(|| "none".to_string())
        ));
    }
    if let Some(fields) = value.collect_simple_record("node-control-live-workflow-bundle-apply-receipt-v1", Some(15)) {
        require_schema(
            &fields[0],
            NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_APPLY_RECEIPT_SCHEMA,
            "node control live workflow bundle apply receipt",
        )?;
        return Ok(format!(
            "node control live workflow bundle apply decision={} bundle={} mode={} send={}",
            record_string(&fields[1], "decision")?,
            record_string(&fields[3], "bundle")?,
            record_string(&fields[8], "mode")?,
            record_optional_string(&fields[11], "send-receipt")?.unwrap_or_else(|| "none".to_string())
        ));
    }
    if let Some(fields) =
        value.collect_simple_record("node-control-live-workflow-bundle-reconcile-receipt-v1", Some(13))
    {
        require_schema(
            &fields[0],
            NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_RECONCILE_RECEIPT_SCHEMA,
            "node control live workflow bundle reconcile receipt",
        )?;
        return Ok(format!(
            "node control live workflow bundle reconcile decision={} bundle={} envelope={} control={}",
            record_string(&fields[1], "decision")?,
            record_string(&fields[3], "bundle")?,
            record_optional_string(&fields[8], "envelope")?.unwrap_or_else(|| "none".to_string()),
            record_optional_string(&fields[7], "control-receipt")?.unwrap_or_else(|| "none".to_string())
        ));
    }
    if let Some(fields) = value.collect_simple_record("node-control-live-send-retry-receipt-v1", Some(14)) {
        require_schema(
            &fields[0],
            NODE_CONTROL_LIVE_SEND_RETRY_RECEIPT_SCHEMA,
            "node control live send retry receipt",
        )?;
        return Ok(format!(
            "node control live send retry decision={} attempt={}/{} envelope={}",
            record_string(&fields[1], "decision")?,
            record_string(&fields[2], "attempt")?,
            record_string(&fields[3], "max-attempts")?,
            record_string(&fields[10], "envelope")?
        ));
    }
    if let Some(fields) = value.collect_simple_record("node-control-live-send-duplicate-receipt-v1", Some(13)) {
        require_schema(
            &fields[0],
            NODE_CONTROL_LIVE_SEND_DUPLICATE_RECEIPT_SCHEMA,
            "node control live send duplicate receipt",
        )?;
        return Ok(format!(
            "node control live send duplicate operation={} prior={}",
            record_string(&fields[9], "operation")?,
            record_string(&fields[10], "prior-send-receipt")?
        ));
    }
    if let Some(fields) = value.collect_simple_record("node-control-live-send-receipt-v1", Some(13)) {
        require_schema(&fields[0], NODE_CONTROL_LIVE_SEND_RECEIPT_SCHEMA, "node control live send receipt")?;
        return Ok(format!(
            "node control live send decision={} from={} to={} ticket={} envelope={}",
            record_string(&fields[1], "decision")?,
            record_string(&fields[4], "from-peer")?,
            record_string(&fields[5], "to-node")?,
            record_string(&fields[6], "receiver-ticket")?,
            record_string(&fields[9], "envelope")?
        ));
    }
    if let Some(fields) = value.collect_simple_record("node-control-live-transport-receipt-v1", Some(11)) {
        require_schema(&fields[0], NODE_CONTROL_LIVE_TRANSPORT_RECEIPT_SCHEMA, "node control live transport receipt")?;
        return Ok(format!(
            "node control live transport operation={} decision={} envelope={} ingress={}",
            record_string(&fields[1], "operation")?,
            record_string(&fields[2], "decision")?,
            record_string(&fields[7], "envelope")?,
            record_optional_string(&fields[8], "ingress-receipt")?.unwrap_or_else(|| "none".to_string())
        ));
    }
    if let Ok(health) = node_runtime::parse_node_health_receipt(value) {
        return Ok(format!(
            "node health decision={} receipt={} replay={}",
            health.decision, health.receipt_ref, health.replay_status
        ));
    }
    if let Ok(shutdown) = node_runtime::parse_node_shutdown_receipt(value) {
        return Ok(format!(
            "node shutdown decision={} receipt={} adapters={}",
            shutdown.decision,
            shutdown.receipt_ref,
            shutdown.adapters.len()
        ));
    }
    if let Some(fields) = value.collect_simple_record("node-control-lock-v1", Some(6)) {
        return Ok(format!(
            "node control lock startup={} owner={}",
            record_string(&fields[2], "startup")?,
            record_string(&fields[3], "owner")?
        ));
    }
    if let Some(fields) = value.collect_simple_record("node-control-service-lock-v1", Some(10)) {
        return Ok(format!(
            "node control service lock startup={} topic={} max_ticks={}",
            record_string(&fields[2], "startup")?,
            record_string(&fields[4], "topic")?,
            record_string(&fields[5], "max-ticks")?
        ));
    }
    if let Some(fields) = value.collect_simple_record("node-control-service-heartbeat-receipt-v1", Some(9)) {
        return Ok(format!(
            "node control service heartbeat decision={} startup={} tick={}",
            record_string(&fields[1], "decision")?,
            record_string(&fields[2], "startup")?,
            record_string(&fields[4], "tick")?
        ));
    }
    if let Ok(policy) = parse_node_control_supervisor_policy(value) {
        return Ok(format!(
            "node control supervisor policy ref={} restarts={} stale_lock_recovery={}",
            policy.policy_ref, policy.max_restarts, policy.stale_lock_recovery
        ));
    }
    if let Ok(receipt) = parse_node_control_supervisor_receipt(value) {
        return Ok(format!(
            "node control supervisor decision={} operation={} policy={}",
            receipt.decision,
            receipt.operation,
            receipt.supervisor_policy_ref.unwrap_or_else(|| "none".to_string())
        ));
    }
    if let Some(fields) = value.collect_simple_record("node-control-service-run-receipt-v1", Some(17)) {
        return Ok(format!(
            "node control service run decision={} ticks={} heartbeats={} stopped={}",
            record_string(&fields[1], "decision")?,
            record_string(&fields[7], "ticks")?,
            record_sequence_len(&fields[8], "heartbeats")?,
            record_string(&fields[12], "stopped")?
        ));
    }
    if let Some(fields) = value.collect_simple_record("node-control-service-run-receipt-v1", Some(15)) {
        return Ok(format!(
            "node control service run decision={} ticks={} heartbeats={} stopped={}",
            record_string(&fields[1], "decision")?,
            record_string(&fields[7], "ticks")?,
            record_sequence_len(&fields[8], "heartbeats")?,
            record_string(&fields[12], "stopped")?
        ));
    }
    if let Some(fields) = value.collect_simple_record("node-control-queue-receipt-v1", Some(9)) {
        return Ok(format!(
            "node control queue decision={} phase={} request={}",
            record_string(&fields[1], "decision")?,
            record_string(&fields[2], "phase")?,
            record_string(&fields[4], "request")?
        ));
    }
    if let Some(fields) = value.collect_simple_record("node-control-operation-receipt-v1", Some(8)) {
        return Ok(format!(
            "node control operation decision={} operation={} request={}",
            record_string(&fields[1], "decision")?,
            record_string(&fields[2], "operation")?,
            record_string(&fields[3], "request")?
        ));
    }
    if let Some(fields) = value.collect_simple_record("node-control-heartbeat-receipt-v1", Some(9)) {
        return Ok(format!(
            "node control heartbeat decision={} startup={} processed={}",
            record_string(&fields[1], "decision")?,
            record_string(&fields[2], "startup")?,
            record_string(&fields[5], "processed-count")?
        ));
    }
    if let Some(fields) = value.collect_simple_record("node-control-loop-receipt-v1", Some(11)) {
        return Ok(format!(
            "node control loop decision={} startup={} processed={} stopped={}",
            record_string(&fields[1], "decision")?,
            record_string(&fields[2], "startup")?,
            record_sequence_len(&fields[5], "processed-requests")?,
            record_string(&fields[7], "stopped")?
        ));
    }
    if let Ok(summary) = provenance::provenance_summary(value) {
        return Ok(summary);
    }
    Err(MoltenError::invalid_harness("unsupported node daemon artifact for show"))
}

fn current_startup_receipt(state_root: &Path) -> Result<node_runtime::NodeStartupReceipt> {
    let startup_value = read_preserves(&state_root.join(STARTUP_FILE))?;
    node_runtime::parse_node_startup_receipt(&startup_value)
}

fn write_active_lock(state_root: &Path, startup_receipt_ref: &str) -> Result<()> {
    let lock_value = active_lock_value(state_root, startup_receipt_ref)?;
    write_preserves(&state_root.join(CONTROL_LOCK_FILE), &lock_value)?;
    import_node_artifact(state_root, &lock_value)?;
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
    require_schema(&fields[0], NODE_CONTROL_LOCK_SCHEMA, "node control lock")?;
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

fn active_lock_value(state_root: &Path, startup_receipt_ref: &str) -> Result<IOValue> {
    Ok(record("node-control-lock-v1", vec![
        string(NODE_CONTROL_LOCK_SCHEMA),
        record("state-root", vec![string(&state_root_profile_ref(state_root)?)]),
        record("startup", vec![string(startup_receipt_ref)]),
        record("owner", vec![string(&local_ref("node-control-owner", startup_receipt_ref)?)]),
        record("profile", vec![string("local-preserves-control-file-v1")]),
        record("checks", vec![sequence(vec![
            record("check", vec![string("startup-bound"), string("pass")]),
            record("check", vec![string("not-authority-token"), string("pass")]),
            record("check", vec![string("explicit-state-root"), string("pass")]),
        ])]),
    ]))
}

fn import_node_artifact(state_root: &Path, value: &IOValue) -> Result<String> {
    let imported = ledger::import_artifact(&state_root.join("ledger"), value)?;
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

fn archive_dispatched_request(state_root: &Path, request_path: &Path, request_value: &IOValue) -> Result<()> {
    let request_ref = canonical_hash(request_value)?;
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
    let receipt_ref = canonical_hash(&value)?;
    write_preserves(&control_supervisor_receipt_path(state_root, &receipt_ref), &value)?;
    import_node_artifact(state_root, &value)?;
    Ok(receipt_ref)
}

fn control_ingress_envelope_path(state_root: &Path, topic: &str, envelope_ref: &str) -> PathBuf {
    state_root
        .join(CONTROL_INGRESS_DIR)
        .join(topic)
        .join(format!("{}.envelope.preserves", ref_file_stem(envelope_ref)))
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

fn optional_string(value: Option<&str>) -> IOValue {
    match value {
        Some(value) => record("some", vec![string(value)]),
        None => record("none", Vec::new()),
    }
}

fn optional_value(value: Option<&IOValue>) -> IOValue {
    match value {
        Some(value) => record("some", vec![value.clone()]),
        None => record("none", Vec::new()),
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

fn record_optional_value(value: &preserves::Value<preserves::IOValue>, tag: &str) -> Result<Option<IOValue>> {
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

fn record_value(value: &preserves::Value<preserves::IOValue>, tag: &str) -> Result<IOValue> {
    let record_value = crate::preserves_rail::value_to_iovalue(value);
    let fields = record_value
        .collect_simple_record(tag, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{tag} value>")))?;
    Ok(crate::preserves_rail::value_to_iovalue(&fields[0]))
}

fn record_values(value: &preserves::Value<preserves::IOValue>, tag: &str) -> Result<Vec<IOValue>> {
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
    if reference.starts_with("blake3:") {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("{label} must be a blake3 ref")))
    }
}

fn validate_member_ref(actual: &str, expected: &str, label: &str) -> Result<()> {
    if actual == expected {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("{label} ref {actual} does not match {expected}")))
    }
}

fn validate_optional_member_ref(value: Option<&IOValue>, expected_ref: Option<&str>, label: &str) -> Result<()> {
    match (value, expected_ref) {
        (Some(value), Some(expected)) => validate_member_ref(&canonical_hash(value)?, expected, label),
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
        let startup = node_runtime::parse_node_startup_receipt(&startup_value)?;
        let shutdown_ref = canonical_hash(&read_preserves(&shutdown_path)?)?;
        let head_refs = vec![startup.receipt_ref.clone()];
        let health_value =
            node_runtime::node_restart_health_receipt_value(&node_runtime::RestartHealthReceiptValueInput {
                startup_receipt: &startup,
                shutdown_receipt_ref: Some(&shutdown_ref),
                index_receipt_refs: &index_receipt_refs(state_root)?,
                head_refs: &head_refs,
                open_job_refs: &[],
                diagnostics: &[],
            })?;
        let health = node_runtime::parse_node_health_receipt(&health_value)?;
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

fn default_adapter_bindings(state_root: &Path) -> Result<Vec<node_runtime::NodeAdapterBinding>> {
    let mut adapters = Vec::with_capacity(node_runtime::REQUIRED_RUNTIME_ADAPTERS.len());
    for name in node_runtime::REQUIRED_RUNTIME_ADAPTERS {
        let profile_ref =
            local_ref("node-adapter-profile", &format!("{}:{name}", state_root_profile_ref(state_root)?))?;
        adapters.push(node_runtime::node_adapter_binding(name, &profile_ref)?);
    }
    Ok(adapters)
}

fn status_request() -> Result<node_runtime::NodeControlRequest> {
    control_request("status")
}

fn shutdown_request() -> Result<node_runtime::NodeControlRequest> {
    control_request("shutdown")
}

fn control_request(operation: &str) -> Result<node_runtime::NodeControlRequest> {
    let authority_refs = vec![local_ref("node-control-authority", operation)?];
    let policy_refs = vec![local_ref("node-control-policy", operation)?];
    let resource_refs = vec![local_ref("node-control-resource", operation)?];
    let value = node_runtime::node_control_request_value(&node_runtime::ControlRequestValueInput {
        operation,
        target_ref: None,
        payload_ref: None,
        authority_refs: &authority_refs,
        policy_refs: &policy_refs,
        resource_refs: &resource_refs,
        evidence_refs: &[],
    })?;
    node_runtime::parse_node_control_request(&value)
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
    let grant_value = node_control_authority_grant_value(&NodeControlAuthorityGrantInput {
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
    let grant = import_node_control_authority_grant(state_root, &grant_value)?;
    Ok(vec![grant.grant_ref])
}

#[cfg(test)]
fn test_live_peer_bootstrap_refs(
    state_root: &Path,
    peer_id: &str,
    topic: &str,
    policy_refs: &[String],
) -> Result<Vec<String>> {
    let ticket = export_node_control_live_ticket(&NodeControlLiveTicketExportInput {
        state_root,
        topic,
        policy_refs,
        evidence_refs: &[],
    })?;
    let admission = admit_node_control_live_peer(&NodeControlLivePeerAdmitInput {
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
    let mut refs = Vec::with_capacity(node_runtime::REQUIRED_RUNTIME_ADAPTERS.len());
    for name in node_runtime::REQUIRED_RUNTIME_ADAPTERS {
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
    canonical_hash(&record("node-daemon-local-ref-v1", vec![string(kind), string(label)]))
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

fn write_preserves(path: &Path, value: &IOValue) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(MoltenError::from)?;
    }
    fs::write(path, to_text(value)?).map_err(MoltenError::from)
}

fn read_preserves(path: &Path) -> Result<IOValue> {
    let text = fs::read_to_string(path).map_err(MoltenError::from)?;
    parse_text(&text)
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
    use std::sync::atomic::AtomicU64;
    use std::sync::atomic::Ordering;

    use super::*;

    #[test]
    fn local_node_init_run_status_stop_and_restart_recovery_are_receipted() {
        let root = temp_dir("node-daemon-lifecycle");
        let init = init_local_node(&NodeDaemonInitInput {
            state_root: &root,
            node_id: "node:test",
        })
        .expect("init node");
        assert!(init.config_ref.starts_with("blake3:"));
        let run = run_local_node(&NodeDaemonRunInput { state_root: &root }).expect("run node");
        assert!(run.startup_ref.starts_with("blake3:"));
        assert_eq!(run.adapter_receipt_refs.len(), node_runtime::REQUIRED_RUNTIME_ADAPTERS.len());
        let status = status_local_node(&NodeDaemonStatusInput { state_root: &root }).expect("status node");
        assert_eq!(status.status, "running");
        let stop = stop_local_node(&NodeDaemonStopInput { state_root: &root }).expect("stop node");
        assert!(stop.shutdown_ref.starts_with("blake3:"));
        let stopped = status_local_node(&NodeDaemonStatusInput { state_root: &root }).expect("stopped status");
        assert_eq!(stopped.status, "stopped");
        let restarted = run_local_node(&NodeDaemonRunInput { state_root: &root }).expect("restart node");
        assert!(restarted.startup_ref.starts_with("blake3:"));
        let restarted_status =
            status_local_node(&NodeDaemonStatusInput { state_root: &root }).expect("restarted status");
        assert_eq!(restarted_status.status, "running");
        let stale = run_local_node(&NodeDaemonRunInput { state_root: &root }).expect_err("stale running state denied");
        assert!(stale.to_string().contains("previous startup has no clean shutdown receipt"));
        let startup = node_runtime::parse_node_startup_receipt(&run.startup_value).expect("startup parse");
        let restart = node_runtime::node_restart_health_receipt_value(&node_runtime::RestartHealthReceiptValueInput {
            startup_receipt: &startup,
            shutdown_receipt_ref: Some(&stop.shutdown_ref),
            index_receipt_refs: &index_receipt_refs(&root).expect("index refs"),
            head_refs: std::slice::from_ref(&run.startup_ref),
            open_job_refs: &[],
            diagnostics: &[],
        })
        .expect("restart health");
        let restart_health = node_runtime::parse_node_health_receipt(&restart).expect("parse health");
        assert_eq!(restart_health.decision, "pass");
    }

    #[test]
    fn ambient_current_directory_state_root_is_denied() {
        let denied = init_local_node(&NodeDaemonInitInput {
            state_root: Path::new("."),
            node_id: "node:test",
        })
        .expect_err("ambient state denied");
        assert!(denied.to_string().contains("ambient current directory"));
        let request = status_request().expect("status request");
        let control_denied = submit_control_request(&NodeControlSubmitInput {
            state_root: Path::new("."),
            request_value: &request.value,
        })
        .expect_err("ambient control denied");
        assert!(control_denied.to_string().contains("ambient current directory"));
    }

    #[test]
    fn control_inbox_dispatch_imports_receipts_and_denies_missing_operation_payloads() {
        let root = temp_dir("node-control-socket");
        init_local_node(&NodeDaemonInitInput {
            state_root: &root,
            node_id: "node:control",
        })
        .expect("init node");
        run_local_node(&NodeDaemonRunInput { state_root: &root }).expect("run node");
        let status_request = status_request().expect("status request");
        let submitted = submit_control_request(&NodeControlSubmitInput {
            state_root: &root,
            request_value: &status_request.value,
        })
        .expect("submit status");
        assert!(submitted.inbox_path.exists());
        let dispatched = dispatch_control_request(&NodeControlDispatchInput {
            state_root: &root,
            request_path: Some(&submitted.inbox_path),
        })
        .expect("dispatch status");
        assert_eq!(dispatched.operation, "status");
        let receipt =
            node_runtime::parse_node_control_receipt(&dispatched.control_receipt_value).expect("control receipt");
        assert_eq!(receipt.decision, "pass");
        assert_eq!(receipt.request_ref, status_request.request_ref);
        let kinds = ledger::list_artifacts(&root.join("ledger"))
            .expect("list ledger")
            .into_iter()
            .map(|entry| entry.artifact_kind)
            .collect::<Vec<_>>();
        assert!(kinds.iter().any(|kind| kind == "node-control-request"));
        assert!(kinds.iter().any(|kind| kind == "node-control-queue-receipt"));
        assert!(kinds.iter().any(|kind| kind == "node-health-receipt"));
        assert!(kinds.iter().any(|kind| kind == "node-control-receipt"));

        let target_ref = local_ref("install-target", "fixture").expect("target ref");
        let install_value = node_runtime::node_control_request_value(&node_runtime::ControlRequestValueInput {
            operation: "install",
            target_ref: Some(&target_ref),
            payload_ref: None,
            authority_refs: &status_request.authority_refs,
            policy_refs: &status_request.policy_refs,
            resource_refs: &status_request.resource_refs,
            evidence_refs: &[],
        })
        .expect("install request");
        let install_submitted = submit_control_request(&NodeControlSubmitInput {
            state_root: &root,
            request_value: &install_value,
        })
        .expect("submit install");
        let install_dispatch = dispatch_control_request(&NodeControlDispatchInput {
            state_root: &root,
            request_path: Some(&install_submitted.inbox_path),
        })
        .expect("dispatch install");
        let install_receipt =
            node_runtime::parse_node_control_receipt(&install_dispatch.control_receipt_value).expect("install receipt");
        assert_eq!(install_receipt.decision, "deny");
        assert!(install_receipt.diagnostics.iter().any(|diagnostic| diagnostic.contains("requires payload ref")));

        let missing_authority = node_runtime::node_control_request_value(&node_runtime::ControlRequestValueInput {
            operation: "status",
            target_ref: None,
            payload_ref: None,
            authority_refs: &[],
            policy_refs: &status_request.policy_refs,
            resource_refs: &status_request.resource_refs,
            evidence_refs: &[],
        })
        .expect("missing authority request");
        let missing_submitted = submit_control_request(&NodeControlSubmitInput {
            state_root: &root,
            request_value: &missing_authority,
        })
        .expect("submit missing authority");
        let missing_dispatch = dispatch_control_request(&NodeControlDispatchInput {
            state_root: &root,
            request_path: Some(&missing_submitted.inbox_path),
        })
        .expect("dispatch missing authority");
        let missing_receipt =
            node_runtime::parse_node_control_receipt(&missing_dispatch.control_receipt_value).expect("missing receipt");
        assert_eq!(missing_receipt.decision, "deny");
        assert!(missing_receipt.diagnostics.iter().any(|diagnostic| diagnostic.contains("authority refs missing")));

        let shutdown_request = shutdown_request().expect("shutdown request");
        let shutdown_submitted = submit_control_request(&NodeControlSubmitInput {
            state_root: &root,
            request_value: &shutdown_request.value,
        })
        .expect("submit shutdown");
        let shutdown_dispatch = dispatch_control_request(&NodeControlDispatchInput {
            state_root: &root,
            request_path: Some(&shutdown_submitted.inbox_path),
        })
        .expect("dispatch shutdown");
        let shutdown_receipt = node_runtime::parse_node_control_receipt(&shutdown_dispatch.control_receipt_value)
            .expect("shutdown receipt");
        assert_eq!(shutdown_receipt.decision, "pass");
        assert!(!root.join(CONTROL_LOCK_FILE).exists());

        let after_stop = dispatch_control_request(&NodeControlDispatchInput {
            state_root: &root,
            request_path: None,
        })
        .expect_err("dispatch requires lock");
        assert!(after_stop.to_string().contains("active node lock"));
    }

    #[test]
    fn control_loop_processes_queue_idempotently_and_stops_on_shutdown() {
        let root = temp_dir("node-control-loop");
        init_local_node(&NodeDaemonInitInput {
            state_root: &root,
            node_id: "node:loop",
        })
        .expect("init node");
        run_local_node(&NodeDaemonRunInput { state_root: &root }).expect("run node");
        let status_request = status_request().expect("status request");
        submit_control_request(&NodeControlSubmitInput {
            state_root: &root,
            request_value: &status_request.value,
        })
        .expect("submit status");
        let first_loop = run_control_loop(&NodeControlLoopInput {
            state_root: &root,
            max_requests: 1,
        })
        .expect("run one status request");
        assert_eq!(first_loop.processed_request_refs, vec![status_request.request_ref.clone()]);
        assert!(!first_loop.has_stopped);
        assert_eq!(ledger::artifact_kind(&first_loop.loop_receipt_value), "node-control-loop-receipt");
        assert_eq!(ledger::artifact_kind(&first_loop.heartbeat_receipt_value), "node-control-heartbeat-receipt");

        submit_control_request(&NodeControlSubmitInput {
            state_root: &root,
            request_value: &status_request.value,
        })
        .expect("resubmit duplicate status");
        let duplicate_loop = run_control_loop(&NodeControlLoopInput {
            state_root: &root,
            max_requests: 1,
        })
        .expect("run duplicate status request");
        assert_eq!(duplicate_loop.processed_request_refs, vec![status_request.request_ref.clone()]);
        assert_eq!(duplicate_loop.dispatch_receipt_refs, first_loop.dispatch_receipt_refs);

        let shutdown_request = shutdown_request().expect("shutdown request");
        submit_control_request(&NodeControlSubmitInput {
            state_root: &root,
            request_value: &shutdown_request.value,
        })
        .expect("submit shutdown");
        let shutdown_loop = run_control_loop(&NodeControlLoopInput {
            state_root: &root,
            max_requests: DEFAULT_CONTROL_LOOP_REQUESTS,
        })
        .expect("run shutdown request");
        assert!(shutdown_loop.has_stopped);
        assert!(!root.join(CONTROL_LOCK_FILE).exists());
        let after_stop = run_control_loop(&NodeControlLoopInput {
            state_root: &root,
            max_requests: 1,
        })
        .expect_err("stopped node loop denied");
        assert!(after_stop.to_string().contains("active node lock"));

        let kinds = ledger::list_artifacts(&root.join("ledger"))
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
        init_local_node(&NodeDaemonInitInput {
            state_root: &root,
            node_id: "node:duplicate",
        })
        .expect("init node");
        run_local_node(&NodeDaemonRunInput { state_root: &root }).expect("run node");
        let status_request = status_request().expect("status request");
        let submitted = submit_control_request(&NodeControlSubmitInput {
            state_root: &root,
            request_value: &status_request.value,
        })
        .expect("submit status");
        dispatch_control_request(&NodeControlDispatchInput {
            state_root: &root,
            request_path: Some(&submitted.inbox_path),
        })
        .expect("dispatch status");
        write_preserves(
            &control_outbox_request_path(&root, &status_request.request_ref),
            &record("tampered-node-control-request", vec![string("conflict")]),
        )
        .expect("tamper archived request");
        let duplicate = submit_control_request(&NodeControlSubmitInput {
            state_root: &root,
            request_value: &status_request.value,
        })
        .expect("resubmit duplicate");
        let denied = dispatch_control_request(&NodeControlDispatchInput {
            state_root: &root,
            request_path: Some(&duplicate.inbox_path),
        })
        .expect_err("conflicting duplicate denied");
        assert!(denied.to_string().contains("conflicts with archived request evidence"));
    }

    #[test]
    fn node_control_provenance_gate_denies_missing_and_tampered_evidence_before_side_effects() {
        let root = temp_dir("node-control-provenance");
        init_local_node(&NodeDaemonInitInput {
            state_root: &root,
            node_id: "node:provenance",
        })
        .expect("init node");
        run_local_node(&NodeDaemonRunInput { state_root: &root }).expect("run node");
        let authority_refs = vec![local_ref("node-control-authority", "provenance").expect("authority ref")];
        let policy_refs = vec![local_ref("node-control-policy", "provenance").expect("policy ref")];
        let resource_refs = vec![local_ref("node-control-resource", "provenance").expect("resource ref")];

        let payload_value = record("node-control-install-payload", vec![string("missing-provenance")]);
        let payload_ref = import_node_artifact(&root, &payload_value).expect("import payload");
        let missing_provenance_request =
            node_runtime::node_control_request_value(&node_runtime::ControlRequestValueInput {
                operation: "install",
                target_ref: None,
                payload_ref: Some(&payload_ref),
                authority_refs: &authority_refs,
                policy_refs: &policy_refs,
                resource_refs: &resource_refs,
                evidence_refs: &[],
            })
            .expect("missing provenance request");
        let submitted = submit_control_request(&NodeControlSubmitInput {
            state_root: &root,
            request_value: &missing_provenance_request,
        })
        .expect("submit missing provenance");
        let dispatch = dispatch_control_request(&NodeControlDispatchInput {
            state_root: &root,
            request_path: Some(&submitted.inbox_path),
        })
        .expect("dispatch missing provenance");
        let receipt =
            node_runtime::parse_node_control_receipt(&dispatch.control_receipt_value).expect("control receipt");
        assert_eq!(receipt.decision, "deny");
        assert!(receipt.subreceipt_refs.iter().any(|reference| reference.starts_with("blake3:")));
        assert!(receipt.diagnostics.iter().any(|diagnostic| diagnostic.contains("provenance evidence refs missing")));
        assert!(
            artifacts::list_artifacts(&root.join("registry"), Some("node-control-artifact"))
                .expect("list registry")
                .is_empty()
        );

        let queued_payload = record("node-control-install-payload", vec![string("queued-missing-provenance")]);
        let queued_payload_ref = import_node_artifact(&root, &queued_payload).expect("import queued payload");
        let queued_request = node_runtime::node_control_request_value(&node_runtime::ControlRequestValueInput {
            operation: "install",
            target_ref: None,
            payload_ref: Some(&queued_payload_ref),
            authority_refs: &authority_refs,
            policy_refs: &policy_refs,
            resource_refs: &resource_refs,
            evidence_refs: &[],
        })
        .expect("queued missing provenance request");
        let queued = node_runtime::parse_node_control_request(&queued_request).expect("queued request parse");
        submit_control_request(&NodeControlSubmitInput {
            state_root: &root,
            request_value: &queued_request,
        })
        .expect("submit queued missing provenance");
        let loop_result = run_control_loop(&NodeControlLoopInput {
            state_root: &root,
            max_requests: 1,
        })
        .expect("process queued missing provenance");
        assert_eq!(loop_result.processed_request_refs, vec![queued.request_ref.clone()]);
        let queued_receipt_value =
            read_preserves(&control_outbox_receipt_path(&root, &queued.request_ref)).expect("queued receipt value");
        let queued_receipt = node_runtime::parse_node_control_receipt(&queued_receipt_value).expect("queued receipt");
        assert_eq!(queued_receipt.decision, "deny");
        assert!(
            queued_receipt
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.contains("missing provenance evidence"))
        );

        let tampered_payload = record("node-control-install-payload", vec![string("tampered-provenance")]);
        let tampered_payload_ref = import_node_artifact(&root, &tampered_payload).expect("import tampered payload");
        let wrong_artifact_ref = local_ref("node-control-wrong-provenance-artifact", "tampered").expect("wrong ref");
        let wrong_provenance =
            provenance::synthetic_reviewed_provenance_record(&wrong_artifact_ref).expect("wrong provenance");
        let wrong_provenance_ref = import_node_artifact(&root, &wrong_provenance).expect("import wrong provenance");
        let tampered_evidence_refs = vec![wrong_provenance_ref];
        let tampered_request = node_runtime::node_control_request_value(&node_runtime::ControlRequestValueInput {
            operation: "install",
            target_ref: None,
            payload_ref: Some(&tampered_payload_ref),
            authority_refs: &authority_refs,
            policy_refs: &policy_refs,
            resource_refs: &resource_refs,
            evidence_refs: &tampered_evidence_refs,
        })
        .expect("tampered request");
        let tampered_submitted = submit_control_request(&NodeControlSubmitInput {
            state_root: &root,
            request_value: &tampered_request,
        })
        .expect("submit tampered provenance");
        let tampered_dispatch = dispatch_control_request(&NodeControlDispatchInput {
            state_root: &root,
            request_path: Some(&tampered_submitted.inbox_path),
        })
        .expect("dispatch tampered provenance");
        let tampered_receipt = node_runtime::parse_node_control_receipt(&tampered_dispatch.control_receipt_value)
            .expect("tampered receipt");
        assert_eq!(tampered_receipt.decision, "deny");
        assert!(
            tampered_receipt
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.contains("no provenance record matches"))
        );
        assert!(
            artifacts::list_artifacts(&root.join("registry"), Some("node-control-artifact"))
                .expect("list registry after tampered")
                .is_empty()
        );
    }

    #[test]
    fn node_control_ingress_enqueues_once_and_preserves_provenance_gate() {
        let root = temp_dir("node-control-ingress");
        init_local_node(&NodeDaemonInitInput {
            state_root: &root,
            node_id: "node:ingress",
        })
        .expect("init node");
        run_local_node(&NodeDaemonRunInput { state_root: &root }).expect("run node");
        let authority_refs = vec![local_ref("node-control-authority", "ingress").expect("authority ref")];
        let policy_refs = vec![local_ref("node-control-policy", "ingress").expect("policy ref")];
        let resource_refs = vec![local_ref("node-control-resource", "ingress").expect("resource ref")];
        let peer_bootstrap_refs = vec![local_ref("peer-bootstrap", "peer:operator").expect("bootstrap ref")];

        let payload_value = record("node-control-ingress-payload", vec![string("missing-provenance")]);
        let payload_ref = import_node_artifact(&root, &payload_value).expect("import payload");
        let request_value = node_runtime::node_control_request_value(&node_runtime::ControlRequestValueInput {
            operation: "install",
            target_ref: None,
            payload_ref: Some(&payload_ref),
            authority_refs: &authority_refs,
            policy_refs: &policy_refs,
            resource_refs: &resource_refs,
            evidence_refs: &[],
        })
        .expect("install request");
        let envelope = node_control_ingress_envelope(&NodeControlIngressEnvelopeInput {
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
        let published = publish_node_control_ingress(&NodeControlIngressPublishInput {
            state_root: &root,
            envelope_value: &envelope.value,
        })
        .expect("publish ingress");
        assert_eq!(ledger::artifact_kind(&published.receipt_value), "node-control-ingress-receipt");
        let delivered = deliver_node_control_ingress(&NodeControlIngressDeliverInput {
            state_root: &root,
            topic: DEFAULT_CONTROL_INGRESS_TOPIC,
            envelope_ref: &envelope.envelope_ref,
        })
        .expect("deliver ingress");
        assert!(delivered.has_enqueued);
        assert!(delivered.queue_receipt_ref.is_some());

        let duplicate = deliver_node_control_ingress(&NodeControlIngressDeliverInput {
            state_root: &root,
            topic: DEFAULT_CONTROL_INGRESS_TOPIC,
            envelope_ref: &envelope.envelope_ref,
        })
        .expect("duplicate ingress");
        assert!(!duplicate.has_enqueued);
        assert!(duplicate.idempotency_receipt_ref.is_some());

        let loop_result = run_control_loop(&NodeControlLoopInput {
            state_root: &root,
            max_requests: 1,
        })
        .expect("dispatch ingress request");
        assert_eq!(loop_result.processed_request_refs.len(), 1);
        let control_value = read_preserves(&control_outbox_receipt_path(&root, &delivered.request_ref))
            .expect("read ingress dispatch receipt");
        let control = node_runtime::parse_node_control_receipt(&control_value).expect("parse control receipt");
        assert_eq!(control.decision, "deny");
        assert!(control.diagnostics.iter().any(|diagnostic| diagnostic.contains("provenance evidence refs missing")));
    }

    #[test]
    fn node_control_live_workflow_bundle_reconcile_binds_receiver_evidence() {
        let root = temp_dir("node-control-live-workflow-reconcile");
        init_local_node(&NodeDaemonInitInput {
            state_root: &root,
            node_id: "node:reconcile",
        })
        .expect("init node");
        run_local_node(&NodeDaemonRunInput { state_root: &root }).expect("run node");
        let policy_refs = vec![local_ref("node-control-policy", "reconcile").expect("policy ref")];
        let resource_refs = vec![local_ref("node-control-resource", "reconcile").expect("resource ref")];
        let peer_bootstrap_refs =
            test_live_peer_bootstrap_refs(&root, "peer:reconcile", DEFAULT_CONTROL_INGRESS_TOPIC, &policy_refs)
                .expect("peer bootstrap");
        let authority_refs =
            test_live_authority_refs(&root, "peer:reconcile", "node:reconcile", "status", &policy_refs)
                .expect("authority refs");
        let request_value = node_runtime::node_control_request_value(&node_runtime::ControlRequestValueInput {
            operation: "status",
            target_ref: None,
            payload_ref: None,
            authority_refs: &authority_refs,
            policy_refs: &policy_refs,
            resource_refs: &resource_refs,
            evidence_refs: &[],
        })
        .expect("status request");
        let request = node_runtime::parse_node_control_request(&request_value).expect("request");
        let envelope = node_control_live_ingress_envelope(&NodeControlIngressEnvelopeInput {
            request_value: &request_value,
            from_peer: "peer:reconcile",
            to_node: "node:reconcile",
            topic: DEFAULT_CONTROL_INGRESS_TOPIC,
            sequence: 1,
            peer_bootstrap_refs: &peer_bootstrap_refs,
            authority_refs: &authority_refs,
            policy_refs: &policy_refs,
            resource_refs: &resource_refs,
            evidence_refs: &[],
        })
        .expect("live envelope");
        publish_node_control_ingress(&NodeControlIngressPublishInput {
            state_root: &root,
            envelope_value: &envelope.value,
        })
        .expect("publish envelope");
        let delivered = deliver_node_control_ingress(&NodeControlIngressDeliverInput {
            state_root: &root,
            topic: DEFAULT_CONTROL_INGRESS_TOPIC,
            envelope_ref: &envelope.envelope_ref,
        })
        .expect("deliver envelope");
        assert!(
            delivered.has_enqueued,
            "{}",
            to_text(&delivered.ingress_receipt_value).expect("ingress receipt text")
        );
        run_control_loop(&NodeControlLoopInput {
            state_root: &root,
            max_requests: 1,
        })
        .expect("dispatch request");
        let queue_value = read_preserves(&queue_receipt_path(&root, &delivered.request_ref)).expect("queue receipt");
        let control_value =
            read_preserves(&control_outbox_receipt_path(&root, &delivered.request_ref)).expect("control receipt");
        let control = node_runtime::parse_node_control_receipt(&control_value).expect("parse control");
        assert_eq!(control.decision, "pass");
        let imported_refs = Vec::new();
        let diagnostics = Vec::new();
        let expected_operations = vec!["status".to_string()];
        let expected = LiveWorkflowBundleExpectedInput {
            expected_node: Some("node:reconcile"),
            expected_topic: Some(DEFAULT_CONTROL_INGRESS_TOPIC),
            expected_endpoint: None,
            expected_peer: Some("peer:reconcile"),
            expected_operations: &expected_operations,
            expected_target_scope: Some("*"),
            expected_resource_scope: Some("*"),
            as_of_sequence: 1,
            as_of_epoch: 1,
        };
        let bundle_ref = local_ref("node-control-live-workflow-bundle", "reconcile").expect("bundle ref");
        let verify_ref = local_ref("node-control-live-workflow-bundle-verify", "reconcile").expect("verify ref");
        let apply_receipt_value = live_workflow_bundle_apply_receipt_value(&LiveWorkflowBundleApplyReceiptValueInput {
            decision: "pass",
            state_root: &root,
            bundle_ref: &bundle_ref,
            gate_receipt_ref: None,
            recomputed_verify_receipt_ref: &verify_ref,
            import_receipt_ref: None,
            imported_refs: &imported_refs,
            mode: "dry-run",
            envelope_ref: Some(&envelope.envelope_ref),
            operation_ref: Some(&envelope.operation_ref),
            send_receipt_ref: None,
            expected: &expected,
            diagnostics: &diagnostics,
        })
        .expect("apply receipt");
        let reconciled = reconcile_node_control_live_workflow_bundle(&NodeControlLiveWorkflowBundleReconcileInput {
            apply_receipt_value: &apply_receipt_value,
            send_receipt_value: None,
            ingress_receipt_value: Some(&delivered.ingress_receipt_value),
            queue_receipt_value: Some(&queue_value),
            control_receipt_value: Some(&control_value),
            expected_envelope_ref: Some(&envelope.envelope_ref),
            expected_operation_ref: Some(&envelope.operation_ref),
            expected_request_ref: Some(&delivered.request_ref),
        })
        .expect("reconcile");
        assert_eq!(reconciled.decision, "pass");
        assert_eq!(
            ledger::artifact_kind(&reconciled.receipt_value),
            "node-control-live-workflow-bundle-reconcile-receipt"
        );
        assert_eq!(reconciled.ingress_receipt_ref.as_deref(), Some(delivered.ingress_receipt_ref.as_str()));
        assert_eq!(reconciled.control_receipt_ref.as_deref(), Some(control.receipt_ref.as_str()));
        assert!(parse_node_control_authority_grant(&reconciled.receipt_value).is_err());
        assert!(
            to_text(&reconciled.receipt_value)
                .expect("reconcile text")
                .contains("reconcile-receipt-is-not-authority")
        );
        import_node_artifact(&root, &reconciled.receipt_value).expect("import reconcile receipt");
        let reconcile_authority_refs = vec![reconciled.receipt_ref.clone()];
        let reconcile_authority_request_value =
            node_runtime::node_control_request_value(&node_runtime::ControlRequestValueInput {
                operation: "status",
                target_ref: None,
                payload_ref: None,
                authority_refs: &reconcile_authority_refs,
                policy_refs: &[],
                resource_refs: &[],
                evidence_refs: &[],
            })
            .expect("reconcile authority request");
        let reconcile_authority_envelope = node_control_live_ingress_envelope(&NodeControlIngressEnvelopeInput {
            request_value: &reconcile_authority_request_value,
            from_peer: "peer:reconcile",
            to_node: "node:reconcile",
            topic: DEFAULT_CONTROL_INGRESS_TOPIC,
            sequence: 2,
            peer_bootstrap_refs: &[],
            authority_refs: &reconcile_authority_refs,
            policy_refs: &[],
            resource_refs: &[],
            evidence_refs: &[],
        })
        .expect("reconcile authority envelope");
        let reconcile_authority_diagnostics =
            live_send_authority_grant_diagnostics(&root, &reconcile_authority_envelope)
                .expect("reconcile authority diagnostics");
        assert!(reconcile_authority_diagnostics.iter().any(|value| value.contains("is not a grant")));
        assert!(
            reconcile_authority_diagnostics
                .iter()
                .any(|value| value.contains("authority delegation missing admitted grant"))
        );

        let missing_receiver =
            reconcile_node_control_live_workflow_bundle(&NodeControlLiveWorkflowBundleReconcileInput {
                apply_receipt_value: &apply_receipt_value,
                send_receipt_value: None,
                ingress_receipt_value: None,
                queue_receipt_value: None,
                control_receipt_value: None,
                expected_envelope_ref: Some(&envelope.envelope_ref),
                expected_operation_ref: Some(&envelope.operation_ref),
                expected_request_ref: Some(&delivered.request_ref),
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
        let wrong_reconcile =
            reconcile_node_control_live_workflow_bundle(&NodeControlLiveWorkflowBundleReconcileInput {
                apply_receipt_value: &apply_receipt_value,
                send_receipt_value: None,
                ingress_receipt_value: Some(&delivered.ingress_receipt_value),
                queue_receipt_value: Some(&queue_value),
                control_receipt_value: Some(&control_value),
                expected_envelope_ref: Some(&wrong_envelope),
                expected_operation_ref: Some(&envelope.operation_ref),
                expected_request_ref: Some(&delivered.request_ref),
            })
            .expect("wrong envelope reconcile");
        assert_eq!(wrong_reconcile.decision, "deny");
        assert!(wrong_reconcile.diagnostics.iter().any(|diagnostic| diagnostic.contains("does not match expected")));

        let denied_control = node_runtime::node_control_deny_receipt_value(
            &request,
            &local_ref("node-startup", "reconcile-deny").expect("startup ref"),
            "receiver denial propagated",
        )
        .expect("denied control");
        let denied_reconcile =
            reconcile_node_control_live_workflow_bundle(&NodeControlLiveWorkflowBundleReconcileInput {
                apply_receipt_value: &apply_receipt_value,
                send_receipt_value: None,
                ingress_receipt_value: Some(&delivered.ingress_receipt_value),
                queue_receipt_value: Some(&queue_value),
                control_receipt_value: Some(&denied_control),
                expected_envelope_ref: Some(&envelope.envelope_ref),
                expected_operation_ref: Some(&envelope.operation_ref),
                expected_request_ref: Some(&delivered.request_ref),
            })
            .expect("denied reconcile");
        assert_eq!(denied_reconcile.decision, "deny");
        assert!(
            denied_reconcile
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.contains("receiver denial propagated"))
        );

        let ack_export = export_node_control_live_workflow_bundle_ack(&NodeControlLiveWorkflowBundleAckExportInput {
            apply_receipt_value: &apply_receipt_value,
            send_receipt_value: None,
            ingress_receipt_value: Some(&delivered.ingress_receipt_value),
            queue_receipt_value: Some(&queue_value),
            control_receipt_value: Some(&control_value),
            reconcile_receipt_value: &reconciled.receipt_value,
        })
        .expect("ack export");
        assert_eq!(ack_export.decision, "pass");
        assert_eq!(ack_export.receiver_decision, "pass");
        assert_eq!(ledger::artifact_kind(&ack_export.ack.ack_value), "node-control-live-workflow-bundle-ack");
        assert_eq!(
            ledger::artifact_kind(&ack_export.receipt_value),
            "node-control-live-workflow-bundle-ack-export-receipt"
        );
        assert!(parse_node_control_authority_grant(&ack_export.ack.ack_value).is_err());
        assert!(to_text(&ack_export.ack.ack_value).expect("ack text").contains("ack-bundle-is-not-authority"));
        let ack_import_root = temp_dir("node-control-live-workflow-ack-import");
        init_local_node(&NodeDaemonInitInput {
            state_root: &ack_import_root,
            node_id: "node:ack-import",
        })
        .expect("init ack import root");
        let ack_import = import_node_control_live_workflow_bundle_ack(&NodeControlLiveWorkflowBundleAckImportInput {
            state_root: &ack_import_root,
            ack_value: &ack_export.ack.ack_value,
            expected_bundle_ref: Some(&bundle_ref),
            expected_envelope_ref: Some(&envelope.envelope_ref),
            expected_operation_ref: Some(&envelope.operation_ref),
            expected_request_ref: Some(&delivered.request_ref),
        })
        .expect("ack import");
        assert_eq!(ack_import.decision, "pass");
        assert!(ack_import.imported_refs.iter().any(|reference| reference == &ack_export.ack.ack_ref));
        assert_eq!(
            ledger::artifact_kind(&ack_import.receipt_value),
            "node-control-live-workflow-bundle-ack-import-receipt"
        );
        assert!(to_text(&ack_import.receipt_value).expect("ack import text").contains("ack-import-is-not-authority"));
        read_node_ledger_artifact(&ack_import_root, &ack_export.ack.ack_ref).expect("ack imported");
        read_node_ledger_artifact(&ack_import_root, &reconciled.receipt_ref).expect("reconcile imported");
        let wrong_ack_import =
            import_node_control_live_workflow_bundle_ack(&NodeControlLiveWorkflowBundleAckImportInput {
                state_root: &ack_import_root,
                ack_value: &ack_export.ack.ack_value,
                expected_bundle_ref: Some(&bundle_ref),
                expected_envelope_ref: Some(&wrong_envelope),
                expected_operation_ref: Some(&envelope.operation_ref),
                expected_request_ref: Some(&delivered.request_ref),
            })
            .expect("wrong ack import");
        assert_eq!(wrong_ack_import.decision, "deny");
        assert!(wrong_ack_import.diagnostics.iter().any(|value| value.contains("does not match expected")));

        let missing_ack_export =
            export_node_control_live_workflow_bundle_ack(&NodeControlLiveWorkflowBundleAckExportInput {
                apply_receipt_value: &apply_receipt_value,
                send_receipt_value: None,
                ingress_receipt_value: None,
                queue_receipt_value: None,
                control_receipt_value: None,
                reconcile_receipt_value: &missing_receiver.receipt_value,
            })
            .expect("missing ack export");
        assert_eq!(missing_ack_export.decision, "deny");
        assert!(
            missing_ack_export
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.contains("requires receiver ingress receipt"))
        );

        let denied_ack_export =
            export_node_control_live_workflow_bundle_ack(&NodeControlLiveWorkflowBundleAckExportInput {
                apply_receipt_value: &apply_receipt_value,
                send_receipt_value: None,
                ingress_receipt_value: Some(&delivered.ingress_receipt_value),
                queue_receipt_value: Some(&queue_value),
                control_receipt_value: Some(&denied_control),
                reconcile_receipt_value: &denied_reconcile.receipt_value,
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
        let denied_ack_import =
            import_node_control_live_workflow_bundle_ack(&NodeControlLiveWorkflowBundleAckImportInput {
                state_root: &ack_import_root,
                ack_value: &denied_ack_export.ack.ack_value,
                expected_bundle_ref: Some(&bundle_ref),
                expected_envelope_ref: Some(&envelope.envelope_ref),
                expected_operation_ref: Some(&envelope.operation_ref),
                expected_request_ref: Some(&delivered.request_ref),
            })
            .expect("denied ack import");
        assert_eq!(denied_ack_import.decision, "pass");
        assert_eq!(denied_ack_import.receiver_decision, "deny");
    }

    #[test]
    fn node_control_ingress_denies_missing_authority_before_enqueue() {
        let root = temp_dir("node-control-ingress-deny");
        init_local_node(&NodeDaemonInitInput {
            state_root: &root,
            node_id: "node:ingress-deny",
        })
        .expect("init node");
        run_local_node(&NodeDaemonRunInput { state_root: &root }).expect("run node");
        let request = status_request().expect("status request");
        let peer_bootstrap_refs = vec![local_ref("peer-bootstrap", "peer:operator").expect("bootstrap ref")];
        let policy_refs = vec![local_ref("node-control-policy", "ingress-deny").expect("policy ref")];
        let resource_refs = vec![local_ref("node-control-resource", "ingress-deny").expect("resource ref")];
        let envelope = node_control_ingress_envelope(&NodeControlIngressEnvelopeInput {
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
        publish_node_control_ingress(&NodeControlIngressPublishInput {
            state_root: &root,
            envelope_value: &envelope.value,
        })
        .expect("publish denied ingress");
        let delivered = deliver_node_control_ingress(&NodeControlIngressDeliverInput {
            state_root: &root,
            topic: DEFAULT_CONTROL_INGRESS_TOPIC,
            envelope_ref: &envelope.envelope_ref,
        })
        .expect("deliver denied ingress");
        assert!(!delivered.has_enqueued);
        let receipt_text = to_text(&delivered.ingress_receipt_value).expect("receipt text");
        assert!(receipt_text.contains("authority refs missing"));
        assert!(next_pending_control_request(&root).expect("pending request scan").is_none());
    }

    #[test]
    fn node_control_live_peer_ticket_admission_gates_bootstrap() {
        let root = temp_dir("node-control-live-peer-ticket");
        init_local_node(&NodeDaemonInitInput {
            state_root: &root,
            node_id: "node:live-ticket",
        })
        .expect("init node");
        run_local_node(&NodeDaemonRunInput { state_root: &root }).expect("run node");
        let policy_refs = vec![local_ref("node-control-policy", "live-ticket").expect("policy ref")];
        let resource_refs = vec![local_ref("node-control-resource", "live-ticket").expect("resource ref")];
        let peer_bootstrap_refs =
            test_live_peer_bootstrap_refs(&root, "peer:ticket", DEFAULT_CONTROL_INGRESS_TOPIC, &policy_refs)
                .expect("peer admission ref");
        let authority_refs = test_live_authority_refs(&root, "peer:ticket", "node:live-ticket", "status", &policy_refs)
            .expect("authority grant ref");
        let request_value = node_runtime::node_control_request_value(&node_runtime::ControlRequestValueInput {
            operation: "status",
            target_ref: None,
            payload_ref: None,
            authority_refs: &authority_refs,
            policy_refs: &policy_refs,
            resource_refs: &resource_refs,
            evidence_refs: &[],
        })
        .expect("status request");
        let admitted = node_control_live_ingress_envelope(&NodeControlIngressEnvelopeInput {
            request_value: &request_value,
            from_peer: "peer:ticket",
            to_node: "node:live-ticket",
            topic: DEFAULT_CONTROL_INGRESS_TOPIC,
            sequence: 1,
            peer_bootstrap_refs: &peer_bootstrap_refs,
            authority_refs: &authority_refs,
            policy_refs: &policy_refs,
            resource_refs: &resource_refs,
            evidence_refs: &[],
        })
        .expect("admitted envelope");
        publish_node_control_ingress(&NodeControlIngressPublishInput {
            state_root: &root,
            envelope_value: &admitted.value,
        })
        .expect("publish admitted");
        let delivered = deliver_node_control_ingress(&NodeControlIngressDeliverInput {
            state_root: &root,
            topic: DEFAULT_CONTROL_INGRESS_TOPIC,
            envelope_ref: &admitted.envelope_ref,
        })
        .expect("deliver admitted");
        assert!(delivered.has_enqueued);

        let denied = node_control_live_ingress_envelope(&NodeControlIngressEnvelopeInput {
            request_value: &request_value,
            from_peer: "peer:other-ticket",
            to_node: "node:live-ticket",
            topic: DEFAULT_CONTROL_INGRESS_TOPIC,
            sequence: 1,
            peer_bootstrap_refs: &peer_bootstrap_refs,
            authority_refs: &authority_refs,
            policy_refs: &policy_refs,
            resource_refs: &resource_refs,
            evidence_refs: &[],
        })
        .expect("denied envelope");
        publish_node_control_ingress(&NodeControlIngressPublishInput {
            state_root: &root,
            envelope_value: &denied.value,
        })
        .expect("publish denied");
        let denied_delivery = deliver_node_control_ingress(&NodeControlIngressDeliverInput {
            state_root: &root,
            topic: DEFAULT_CONTROL_INGRESS_TOPIC,
            envelope_ref: &denied.envelope_ref,
        })
        .expect("deliver denied");
        assert!(!denied_delivery.has_enqueued);
        let receipt_text = to_text(&denied_delivery.ingress_receipt_value).expect("receipt text");
        assert!(receipt_text.contains("peer peer:ticket does not match peer:other-ticket"));
    }

    #[test]
    fn node_control_live_ticket_and_authority_import_receipts_gate_bindings() {
        let receiver = temp_dir("node-control-live-import-receiver");
        let sender = temp_dir("node-control-live-import-sender");
        init_local_node(&NodeDaemonInitInput {
            state_root: &receiver,
            node_id: "node:live-import",
        })
        .expect("init receiver");
        run_local_node(&NodeDaemonRunInput { state_root: &receiver }).expect("run receiver");
        init_local_node(&NodeDaemonInitInput {
            state_root: &sender,
            node_id: "node:live-import-sender",
        })
        .expect("init sender");
        let policy_refs = vec![local_ref("node-control-policy", "live-import").expect("policy ref")];
        let ticket = export_node_control_live_ticket(&NodeControlLiveTicketExportInput {
            state_root: &receiver,
            topic: DEFAULT_CONTROL_INGRESS_TOPIC,
            policy_refs: &policy_refs,
            evidence_refs: &[],
        })
        .expect("export ticket");
        let admission = admit_node_control_live_peer(&NodeControlLivePeerAdmitInput {
            state_root: &receiver,
            ticket_value: &ticket.value,
            peer_id: "peer:live-import",
            sequence: 1,
            expires_at: Some(4),
            policy_refs: &policy_refs,
            evidence_refs: &[],
        })
        .expect("admit peer");
        let imported_ticket = import_node_control_live_ticket(&NodeControlLiveTicketImportInput {
            state_root: &sender,
            ticket_value: &ticket.value,
            peer_admission_value: Some(&admission.value),
            expected_node: Some("node:live-import"),
            expected_topic: Some(DEFAULT_CONTROL_INGRESS_TOPIC),
            expected_endpoint: Some(&ticket.live_endpoint_id),
            expected_peer: Some("peer:live-import"),
            as_of_sequence: 2,
        })
        .expect("import ticket");
        assert_eq!(imported_ticket.decision, "pass");
        assert_eq!(imported_ticket.imported_refs.len(), 2);
        assert_eq!(ledger::artifact_kind(&imported_ticket.receipt_value), "node-control-live-ticket-import-receipt");
        read_node_ledger_artifact(&sender, &ticket.ticket_ref).expect("ticket imported");
        read_node_ledger_artifact(&sender, &admission.admission_ref).expect("admission imported");

        let stale_ticket = import_node_control_live_ticket(&NodeControlLiveTicketImportInput {
            state_root: &sender,
            ticket_value: &ticket.value,
            peer_admission_value: Some(&admission.value),
            expected_node: Some("node:live-import"),
            expected_topic: Some(DEFAULT_CONTROL_INGRESS_TOPIC),
            expected_endpoint: Some(&ticket.live_endpoint_id),
            expected_peer: Some("peer:live-import"),
            as_of_sequence: 8,
        })
        .expect("stale ticket import receipt");
        assert_eq!(stale_ticket.decision, "deny");
        assert!(stale_ticket.imported_refs.is_empty());
        assert!(stale_ticket.diagnostics.iter().any(|value| value.contains("expired at sequence")));

        let operations = vec!["status".to_string()];
        let grant_value = node_control_authority_grant_value(&NodeControlAuthorityGrantInput {
            peer_id: "peer:live-import",
            node_id: "node:live-import",
            operations: &operations,
            target_scope: "*",
            resource_scope: "*",
            epoch: 1,
            expires_at: Some(4),
            policy_refs: &policy_refs,
            revocation_refs: &[],
            evidence_refs: &[],
        })
        .expect("grant value");
        let imported_grant = import_node_control_authority_grant_checked(&NodeControlAuthorityGrantImportInput {
            state_root: &sender,
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
        assert_eq!(ledger::artifact_kind(&imported_grant.receipt_value), "node-control-authority-grant-import-receipt");
        read_node_ledger_artifact(&sender, &imported_grant.grant_ref).expect("grant imported");

        let bad_operations = vec!["shutdown".to_string()];
        let denied_grant = import_node_control_authority_grant_checked(&NodeControlAuthorityGrantImportInput {
            state_root: &sender,
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
    fn node_control_live_workflow_bundle_import_export_gates_bindings() {
        let receiver = temp_dir("node-control-live-workflow-bundle-receiver");
        let staging_sender = temp_dir("node-control-live-workflow-bundle-staging");
        let bundle_sender = temp_dir("node-control-live-workflow-bundle-sender");
        init_local_node(&NodeDaemonInitInput {
            state_root: &receiver,
            node_id: "node:live-bundle",
        })
        .expect("init receiver");
        run_local_node(&NodeDaemonRunInput { state_root: &receiver }).expect("run receiver");
        init_local_node(&NodeDaemonInitInput {
            state_root: &staging_sender,
            node_id: "node:live-bundle-staging",
        })
        .expect("init staging sender");
        init_local_node(&NodeDaemonInitInput {
            state_root: &bundle_sender,
            node_id: "node:live-bundle-sender",
        })
        .expect("init bundle sender");
        let policy_refs = vec![local_ref("node-control-policy", "live-bundle").expect("policy ref")];
        let ticket = export_node_control_live_ticket(&NodeControlLiveTicketExportInput {
            state_root: &receiver,
            topic: DEFAULT_CONTROL_INGRESS_TOPIC,
            policy_refs: &policy_refs,
            evidence_refs: &[],
        })
        .expect("export ticket");
        let admission = admit_node_control_live_peer(&NodeControlLivePeerAdmitInput {
            state_root: &receiver,
            ticket_value: &ticket.value,
            peer_id: "peer:live-bundle",
            sequence: 1,
            expires_at: Some(8),
            policy_refs: &policy_refs,
            evidence_refs: &[],
        })
        .expect("admit peer");
        let operations = vec!["status".to_string()];
        let authority_value = node_control_authority_grant_value(&NodeControlAuthorityGrantInput {
            peer_id: "peer:live-bundle",
            node_id: "node:live-bundle",
            operations: &operations,
            target_scope: "*",
            resource_scope: "*",
            epoch: 1,
            expires_at: Some(8),
            policy_refs: &policy_refs,
            revocation_refs: &[],
            evidence_refs: &[],
        })
        .expect("authority grant value");
        let ticket_import = import_node_control_live_ticket(&NodeControlLiveTicketImportInput {
            state_root: &staging_sender,
            ticket_value: &ticket.value,
            peer_admission_value: Some(&admission.value),
            expected_node: Some("node:live-bundle"),
            expected_topic: Some(DEFAULT_CONTROL_INGRESS_TOPIC),
            expected_endpoint: Some(&ticket.live_endpoint_id),
            expected_peer: Some("peer:live-bundle"),
            as_of_sequence: 2,
        })
        .expect("ticket import");
        let authority_import = import_node_control_authority_grant_checked(&NodeControlAuthorityGrantImportInput {
            state_root: &staging_sender,
            grant_value: &authority_value,
            expected_peer: Some("peer:live-bundle"),
            expected_node: Some("node:live-bundle"),
            expected_operations: &operations,
            expected_target_scope: Some("*"),
            expected_resource_scope: Some("*"),
            as_of_epoch: 2,
        })
        .expect("authority import");
        let receipt_values = vec![&ticket_import.receipt_value, &authority_import.receipt_value];
        let exported = export_node_control_live_workflow_bundle(&NodeControlLiveWorkflowBundleExportInput {
            receiver_ticket_value: &ticket.value,
            peer_admission_value: &admission.value,
            authority_grant_value: &authority_value,
            receipt_values: &receipt_values,
        })
        .expect("export bundle");
        assert_eq!(exported.decision, "pass");
        assert_eq!(ledger::artifact_kind(&exported.bundle.bundle_value), "node-control-live-workflow-bundle");
        assert!(parse_node_control_authority_grant(&exported.bundle.bundle_value).is_err());
        let verified = verify_node_control_live_workflow_bundle(&NodeControlLiveWorkflowBundleVerifyInput {
            bundle_value: &exported.bundle.bundle_value,
            expected_node: Some("node:live-bundle"),
            expected_topic: Some(DEFAULT_CONTROL_INGRESS_TOPIC),
            expected_endpoint: Some(&ticket.live_endpoint_id),
            expected_peer: Some("peer:live-bundle"),
            expected_operations: &operations,
            expected_target_scope: Some("*"),
            expected_resource_scope: Some("*"),
            as_of_sequence: 2,
            as_of_epoch: 2,
        })
        .expect("verify bundle");
        assert_eq!(verified.decision, "pass");
        assert_eq!(ledger::artifact_kind(&verified.receipt_value), "node-control-live-workflow-bundle-verify-receipt");
        assert!(parse_node_control_authority_grant(&verified.receipt_value).is_err());
        assert!(
            to_text(&verified.receipt_value)
                .expect("verify receipt text")
                .contains("verify-receipt-is-not-authority")
        );
        let gated = gate_node_control_live_workflow_bundle(&NodeControlLiveWorkflowBundleGateInput {
            bundle_value: &exported.bundle.bundle_value,
            verify_receipt_value: Some(&verified.receipt_value),
            require_verify_receipt: true,
            expected_node: Some("node:live-bundle"),
            expected_topic: Some(DEFAULT_CONTROL_INGRESS_TOPIC),
            expected_endpoint: Some(&ticket.live_endpoint_id),
            expected_peer: Some("peer:live-bundle"),
            expected_operations: &operations,
            expected_target_scope: Some("*"),
            expected_resource_scope: Some("*"),
            as_of_sequence: 2,
            as_of_epoch: 2,
        })
        .expect("gate bundle");
        assert_eq!(gated.decision, "pass");
        assert_eq!(ledger::artifact_kind(&gated.receipt_value), "node-control-live-workflow-bundle-gate-receipt");
        assert_eq!(gated.verify_receipt_ref.as_deref(), Some(verified.receipt_ref.as_str()));
        assert!(parse_node_control_authority_grant(&gated.receipt_value).is_err());
        assert!(to_text(&gated.receipt_value).expect("gate receipt text").contains("gate-receipt-is-not-authority"));
        let missing_verify_gate = gate_node_control_live_workflow_bundle(&NodeControlLiveWorkflowBundleGateInput {
            bundle_value: &exported.bundle.bundle_value,
            verify_receipt_value: None,
            require_verify_receipt: true,
            expected_node: Some("node:live-bundle"),
            expected_topic: Some(DEFAULT_CONTROL_INGRESS_TOPIC),
            expected_endpoint: Some(&ticket.live_endpoint_id),
            expected_peer: Some("peer:live-bundle"),
            expected_operations: &operations,
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
        let malformed_verify_gate = gate_node_control_live_workflow_bundle(&NodeControlLiveWorkflowBundleGateInput {
            bundle_value: &exported.bundle.bundle_value,
            verify_receipt_value: Some(&exported.bundle.bundle_value),
            require_verify_receipt: true,
            expected_node: Some("node:live-bundle"),
            expected_topic: Some(DEFAULT_CONTROL_INGRESS_TOPIC),
            expected_endpoint: Some(&ticket.live_endpoint_id),
            expected_peer: Some("peer:live-bundle"),
            expected_operations: &operations,
            expected_target_scope: Some("*"),
            expected_resource_scope: Some("*"),
            as_of_sequence: 2,
            as_of_epoch: 2,
        })
        .expect("malformed verify gate receipt");
        assert_eq!(malformed_verify_gate.decision, "deny");
        assert!(malformed_verify_gate.diagnostics.iter().any(|value| value.contains("verify receipt parse failed")));
        let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().expect("apply runtime");
        let applied = runtime
            .block_on(apply_node_control_live_workflow_bundle(&NodeControlLiveWorkflowBundleApplyInput {
                state_root: &bundle_sender,
                bundle_value: &exported.bundle.bundle_value,
                gate_receipt_value: Some(&gated.receipt_value),
                is_gate_receipt_required: true,
                request_value: None,
                should_send: false,
                from_peer: None,
                sequence: 1,
                expected_operation_ref: None,
                expected_node: Some("node:live-bundle"),
                expected_topic: Some(DEFAULT_CONTROL_INGRESS_TOPIC),
                expected_endpoint: Some(&ticket.live_endpoint_id),
                expected_peer: Some("peer:live-bundle"),
                expected_operations: &operations,
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
            .expect("apply bundle");
        assert_eq!(applied.decision, "pass");
        assert_eq!(ledger::artifact_kind(&applied.receipt_value), "node-control-live-workflow-bundle-apply-receipt");
        assert!(applied.import_receipt_ref.is_some());
        assert!(applied.imported_refs.iter().any(|reference| reference == &exported.bundle.bundle_ref));
        assert!(parse_node_control_authority_grant(&applied.receipt_value).is_err());
        assert!(
            to_text(&applied.receipt_value)
                .expect("apply receipt text")
                .contains("apply-receipt-is-not-authority")
        );
        read_node_ledger_artifact(&bundle_sender, &exported.bundle.bundle_ref).expect("apply imported bundle");
        let missing_gate_apply_root = temp_dir("node-control-live-workflow-bundle-apply-missing-gate");
        init_local_node(&NodeDaemonInitInput {
            state_root: &missing_gate_apply_root,
            node_id: "node:live-bundle-apply-missing-gate",
        })
        .expect("init missing gate apply root");
        let missing_gate_apply = runtime
            .block_on(apply_node_control_live_workflow_bundle(&NodeControlLiveWorkflowBundleApplyInput {
                state_root: &missing_gate_apply_root,
                bundle_value: &exported.bundle.bundle_value,
                gate_receipt_value: None,
                is_gate_receipt_required: true,
                request_value: None,
                should_send: false,
                from_peer: None,
                sequence: 1,
                expected_operation_ref: None,
                expected_node: Some("node:live-bundle"),
                expected_topic: Some(DEFAULT_CONTROL_INGRESS_TOPIC),
                expected_endpoint: Some(&ticket.live_endpoint_id),
                expected_peer: Some("peer:live-bundle"),
                expected_operations: &operations,
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
            .expect("missing gate apply receipt");
        assert_eq!(missing_gate_apply.decision, "deny");
        assert!(missing_gate_apply.imported_refs.is_empty());
        assert!(missing_gate_apply.diagnostics.iter().any(|value| value.contains("requires a current gate receipt")));
        assert!(read_node_ledger_artifact(&missing_gate_apply_root, &exported.bundle.bundle_ref).is_err());
        let apply_send_root = temp_dir("node-control-live-workflow-bundle-apply-send");
        init_local_node(&NodeDaemonInitInput {
            state_root: &apply_send_root,
            node_id: "node:live-bundle-apply-send",
        })
        .expect("init apply send root");
        let apply_request_authority_refs = vec![exported.bundle.authority_grant_ref.clone()];
        let apply_request_value = node_runtime::node_control_request_value(&node_runtime::ControlRequestValueInput {
            operation: "status",
            target_ref: None,
            payload_ref: None,
            authority_refs: &apply_request_authority_refs,
            policy_refs: &[],
            resource_refs: &[],
            evidence_refs: &[],
        })
        .expect("apply send request");
        let apply_send = runtime
            .block_on(apply_node_control_live_workflow_bundle(&NodeControlLiveWorkflowBundleApplyInput {
                state_root: &apply_send_root,
                bundle_value: &exported.bundle.bundle_value,
                gate_receipt_value: Some(&gated.receipt_value),
                is_gate_receipt_required: true,
                request_value: Some(&apply_request_value),
                should_send: true,
                from_peer: None,
                sequence: 7,
                expected_operation_ref: None,
                expected_node: Some("node:live-bundle"),
                expected_topic: Some(DEFAULT_CONTROL_INGRESS_TOPIC),
                expected_endpoint: Some(&ticket.live_endpoint_id),
                expected_peer: Some("peer:live-bundle"),
                expected_operations: &operations,
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
            .expect("apply send receipt");
        assert_eq!(apply_send.decision, "deny");
        assert!(apply_send.import_receipt_ref.is_some());
        assert!(apply_send.send_receipt_ref.is_some());
        assert!(apply_send.diagnostics.iter().any(|value| value.contains("no endpoint addresses")));
        assert!(apply_send.send_receipt_value.is_some());
        let imported = import_node_control_live_workflow_bundle(&NodeControlLiveWorkflowBundleImportInput {
            state_root: &bundle_sender,
            bundle_value: &exported.bundle.bundle_value,
            expected_node: Some("node:live-bundle"),
            expected_topic: Some(DEFAULT_CONTROL_INGRESS_TOPIC),
            expected_endpoint: Some(&ticket.live_endpoint_id),
            expected_peer: Some("peer:live-bundle"),
            expected_operations: &operations,
            expected_target_scope: Some("*"),
            expected_resource_scope: Some("*"),
            as_of_sequence: 2,
            as_of_epoch: 2,
        })
        .expect("import bundle");
        assert_eq!(imported.decision, "pass");
        assert!(imported.imported_refs.iter().any(|reference| reference == &exported.bundle.bundle_ref));
        read_node_ledger_artifact(&bundle_sender, &ticket.ticket_ref).expect("bundle imported ticket");
        read_node_ledger_artifact(&bundle_sender, &admission.admission_ref).expect("bundle imported admission");
        read_node_ledger_artifact(&bundle_sender, &authority_import.grant_ref).expect("bundle imported authority");
        assert!(parse_node_control_authority_grant(&imported.receipt_value).is_err());
        assert!(
            to_text(&imported.receipt_value)
                .expect("import receipt text")
                .contains("bundle-import-is-not-authority")
        );

        let wrong_topic_root = temp_dir("node-control-live-workflow-bundle-wrong-topic");
        init_local_node(&NodeDaemonInitInput {
            state_root: &wrong_topic_root,
            node_id: "node:live-bundle-wrong-topic",
        })
        .expect("init wrong topic root");
        let wrong_topic = import_node_control_live_workflow_bundle(&NodeControlLiveWorkflowBundleImportInput {
            state_root: &wrong_topic_root,
            bundle_value: &exported.bundle.bundle_value,
            expected_node: Some("node:live-bundle"),
            expected_topic: Some("wrong-topic"),
            expected_endpoint: Some(&ticket.live_endpoint_id),
            expected_peer: Some("peer:live-bundle"),
            expected_operations: &operations,
            expected_target_scope: Some("*"),
            expected_resource_scope: Some("*"),
            as_of_sequence: 2,
            as_of_epoch: 2,
        })
        .expect("wrong topic receipt");
        assert_eq!(wrong_topic.decision, "deny");
        assert!(wrong_topic.imported_refs.is_empty());
        assert!(wrong_topic.diagnostics.iter().any(|value| value.contains("wrong-topic")));
        assert!(read_node_ledger_artifact(&wrong_topic_root, &exported.bundle.bundle_ref).is_err());
        let wrong_topic_verify = verify_node_control_live_workflow_bundle(&NodeControlLiveWorkflowBundleVerifyInput {
            bundle_value: &exported.bundle.bundle_value,
            expected_node: Some("node:live-bundle"),
            expected_topic: Some("wrong-topic"),
            expected_endpoint: Some(&ticket.live_endpoint_id),
            expected_peer: Some("peer:live-bundle"),
            expected_operations: &operations,
            expected_target_scope: Some("*"),
            expected_resource_scope: Some("*"),
            as_of_sequence: 2,
            as_of_epoch: 2,
        })
        .expect("wrong topic verify receipt");
        assert_eq!(wrong_topic_verify.decision, "deny");
        assert!(wrong_topic_verify.diagnostics.iter().any(|value| value.contains("wrong-topic")));
        let stale_verify_gate = gate_node_control_live_workflow_bundle(&NodeControlLiveWorkflowBundleGateInput {
            bundle_value: &exported.bundle.bundle_value,
            verify_receipt_value: Some(&wrong_topic_verify.receipt_value),
            require_verify_receipt: true,
            expected_node: Some("node:live-bundle"),
            expected_topic: Some(DEFAULT_CONTROL_INGRESS_TOPIC),
            expected_endpoint: Some(&ticket.live_endpoint_id),
            expected_peer: Some("peer:live-bundle"),
            expected_operations: &operations,
            expected_target_scope: Some("*"),
            expected_resource_scope: Some("*"),
            as_of_sequence: 2,
            as_of_epoch: 2,
        })
        .expect("stale verify gate receipt");
        assert_eq!(stale_verify_gate.decision, "deny");
        assert!(stale_verify_gate.diagnostics.iter().any(|value| value.contains("does not match recomputed")));
        let stale_gate_apply_root = temp_dir("node-control-live-workflow-bundle-apply-stale-gate");
        init_local_node(&NodeDaemonInitInput {
            state_root: &stale_gate_apply_root,
            node_id: "node:live-bundle-apply-stale-gate",
        })
        .expect("init stale gate apply root");
        let stale_gate_apply = runtime
            .block_on(apply_node_control_live_workflow_bundle(&NodeControlLiveWorkflowBundleApplyInput {
                state_root: &stale_gate_apply_root,
                bundle_value: &exported.bundle.bundle_value,
                gate_receipt_value: Some(&stale_verify_gate.receipt_value),
                is_gate_receipt_required: true,
                request_value: None,
                should_send: false,
                from_peer: None,
                sequence: 1,
                expected_operation_ref: None,
                expected_node: Some("node:live-bundle"),
                expected_topic: Some(DEFAULT_CONTROL_INGRESS_TOPIC),
                expected_endpoint: Some(&ticket.live_endpoint_id),
                expected_peer: Some("peer:live-bundle"),
                expected_operations: &operations,
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
            .expect("stale gate apply receipt");
        assert_eq!(stale_gate_apply.decision, "deny");
        assert!(stale_gate_apply.imported_refs.is_empty());
        assert!(stale_gate_apply.diagnostics.iter().any(|value| value.contains("decision deny")));
        assert!(read_node_ledger_artifact(&stale_gate_apply_root, &exported.bundle.bundle_ref).is_err());

        let wrong_peer_root = temp_dir("node-control-live-workflow-bundle-wrong-peer");
        init_local_node(&NodeDaemonInitInput {
            state_root: &wrong_peer_root,
            node_id: "node:live-bundle-wrong-peer",
        })
        .expect("init wrong peer root");
        let wrong_peer = import_node_control_live_workflow_bundle(&NodeControlLiveWorkflowBundleImportInput {
            state_root: &wrong_peer_root,
            bundle_value: &exported.bundle.bundle_value,
            expected_node: Some("node:live-bundle"),
            expected_topic: Some(DEFAULT_CONTROL_INGRESS_TOPIC),
            expected_endpoint: Some(&ticket.live_endpoint_id),
            expected_peer: Some("peer:other-live-bundle"),
            expected_operations: &operations,
            expected_target_scope: Some("*"),
            expected_resource_scope: Some("*"),
            as_of_sequence: 2,
            as_of_epoch: 2,
        })
        .expect("wrong peer receipt");
        assert_eq!(wrong_peer.decision, "deny");
        assert!(wrong_peer.imported_refs.is_empty());
        assert!(wrong_peer.diagnostics.iter().any(|value| value.contains("peer:other-live-bundle")));
        let wrong_peer_verify = verify_node_control_live_workflow_bundle(&NodeControlLiveWorkflowBundleVerifyInput {
            bundle_value: &exported.bundle.bundle_value,
            expected_node: Some("node:live-bundle"),
            expected_topic: Some(DEFAULT_CONTROL_INGRESS_TOPIC),
            expected_endpoint: Some(&ticket.live_endpoint_id),
            expected_peer: Some("peer:other-live-bundle"),
            expected_operations: &operations,
            expected_target_scope: Some("*"),
            expected_resource_scope: Some("*"),
            as_of_sequence: 2,
            as_of_epoch: 2,
        })
        .expect("wrong peer verify receipt");
        assert_eq!(wrong_peer_verify.decision, "deny");
        assert!(wrong_peer_verify.diagnostics.iter().any(|value| value.contains("peer:other-live-bundle")));

        let wrong_operation_root = temp_dir("node-control-live-workflow-bundle-wrong-operation");
        init_local_node(&NodeDaemonInitInput {
            state_root: &wrong_operation_root,
            node_id: "node:live-bundle-wrong-operation",
        })
        .expect("init wrong operation root");
        let wrong_operations = vec!["shutdown".to_string()];
        let wrong_operation = import_node_control_live_workflow_bundle(&NodeControlLiveWorkflowBundleImportInput {
            state_root: &wrong_operation_root,
            bundle_value: &exported.bundle.bundle_value,
            expected_node: Some("node:live-bundle"),
            expected_topic: Some(DEFAULT_CONTROL_INGRESS_TOPIC),
            expected_endpoint: Some(&ticket.live_endpoint_id),
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
        let wrong_operation_verify =
            verify_node_control_live_workflow_bundle(&NodeControlLiveWorkflowBundleVerifyInput {
                bundle_value: &exported.bundle.bundle_value,
                expected_node: Some("node:live-bundle"),
                expected_topic: Some(DEFAULT_CONTROL_INGRESS_TOPIC),
                expected_endpoint: Some(&ticket.live_endpoint_id),
                expected_peer: Some("peer:live-bundle"),
                expected_operations: &wrong_operations,
                expected_target_scope: Some("*"),
                expected_resource_scope: Some("*"),
                as_of_sequence: 2,
                as_of_epoch: 2,
            })
            .expect("wrong operation verify receipt");
        assert_eq!(wrong_operation_verify.decision, "deny");
        assert!(wrong_operation_verify.diagnostics.iter().any(|value| value.contains("operation shutdown")));

        let wrong_grant_ref = local_ref("authority-grant", "wrong-live-bundle").expect("wrong grant ref");
        let wrong_grant_bundle = record("node-control-live-workflow-bundle-v1", vec![
            string(NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_SCHEMA),
            record("ticket", vec![exported.bundle.ticket_value.clone()]),
            record("peer-admission", vec![exported.bundle.peer_admission_value.clone()]),
            record("authority-grant", vec![exported.bundle.authority_grant_value.clone()]),
            record("receipts", vec![sequence(exported.bundle.receipt_values.clone())]),
            record("ticket-ref", vec![string(&exported.bundle.ticket_ref)]),
            record("peer-admission-ref", vec![string(&exported.bundle.peer_admission_ref)]),
            record("authority-grant-ref", vec![string(&wrong_grant_ref)]),
            record("receipt-refs", vec![sequence(exported.bundle.receipt_refs.iter().map(string).collect())]),
            record("checks", vec![sequence(Vec::<IOValue>::new())]),
        ]);
        let wrong_grant_verify = verify_node_control_live_workflow_bundle(&NodeControlLiveWorkflowBundleVerifyInput {
            bundle_value: &wrong_grant_bundle,
            expected_node: Some("node:live-bundle"),
            expected_topic: Some(DEFAULT_CONTROL_INGRESS_TOPIC),
            expected_endpoint: Some(&ticket.live_endpoint_id),
            expected_peer: Some("peer:live-bundle"),
            expected_operations: &operations,
            expected_target_scope: Some("*"),
            expected_resource_scope: Some("*"),
            as_of_sequence: 2,
            as_of_epoch: 2,
        })
        .expect("wrong grant verify receipt");
        assert_eq!(wrong_grant_verify.decision, "deny");
        assert!(wrong_grant_verify.diagnostics.iter().any(|value| value.contains("authority grant ref mismatch")));

        import_node_artifact(&bundle_sender, &verified.receipt_value).expect("import verify receipt");
        let verify_authority_refs = vec![verified.receipt_ref.clone()];
        let verify_authority_request_value =
            node_runtime::node_control_request_value(&node_runtime::ControlRequestValueInput {
                operation: "status",
                target_ref: None,
                payload_ref: None,
                authority_refs: &verify_authority_refs,
                policy_refs: &[],
                resource_refs: &[],
                evidence_refs: &[],
            })
            .expect("verify authority request");
        let verify_authority_envelope = node_control_live_ingress_envelope(&NodeControlIngressEnvelopeInput {
            request_value: &verify_authority_request_value,
            from_peer: "peer:live-bundle",
            to_node: "node:live-bundle",
            topic: DEFAULT_CONTROL_INGRESS_TOPIC,
            sequence: 3,
            peer_bootstrap_refs: &[],
            authority_refs: &verify_authority_refs,
            policy_refs: &[],
            resource_refs: &[],
            evidence_refs: &[],
        })
        .expect("verify authority envelope");
        let verify_authority_diagnostics =
            live_send_authority_grant_diagnostics(&bundle_sender, &verify_authority_envelope)
                .expect("verify authority diagnostics");
        assert!(verify_authority_diagnostics.iter().any(|value| value.contains("is not a grant")));
        assert!(
            verify_authority_diagnostics
                .iter()
                .any(|value| value.contains("authority delegation missing admitted grant"))
        );
        import_node_artifact(&bundle_sender, &gated.receipt_value).expect("import gate receipt");
        let gate_authority_refs = vec![gated.receipt_ref.clone()];
        let gate_authority_request_value =
            node_runtime::node_control_request_value(&node_runtime::ControlRequestValueInput {
                operation: "status",
                target_ref: None,
                payload_ref: None,
                authority_refs: &gate_authority_refs,
                policy_refs: &[],
                resource_refs: &[],
                evidence_refs: &[],
            })
            .expect("gate authority request");
        let gate_authority_envelope = node_control_live_ingress_envelope(&NodeControlIngressEnvelopeInput {
            request_value: &gate_authority_request_value,
            from_peer: "peer:live-bundle",
            to_node: "node:live-bundle",
            topic: DEFAULT_CONTROL_INGRESS_TOPIC,
            sequence: 4,
            peer_bootstrap_refs: &[],
            authority_refs: &gate_authority_refs,
            policy_refs: &[],
            resource_refs: &[],
            evidence_refs: &[],
        })
        .expect("gate authority envelope");
        let gate_authority_diagnostics =
            live_send_authority_grant_diagnostics(&bundle_sender, &gate_authority_envelope)
                .expect("gate authority diagnostics");
        assert!(gate_authority_diagnostics.iter().any(|value| value.contains("is not a grant")));
        assert!(
            gate_authority_diagnostics
                .iter()
                .any(|value| value.contains("authority delegation missing admitted grant"))
        );
        let apply_authority_refs = vec![applied.receipt_ref.clone()];
        let apply_authority_request_value =
            node_runtime::node_control_request_value(&node_runtime::ControlRequestValueInput {
                operation: "status",
                target_ref: None,
                payload_ref: None,
                authority_refs: &apply_authority_refs,
                policy_refs: &[],
                resource_refs: &[],
                evidence_refs: &[],
            })
            .expect("apply authority request");
        let apply_authority_envelope = node_control_live_ingress_envelope(&NodeControlIngressEnvelopeInput {
            request_value: &apply_authority_request_value,
            from_peer: "peer:live-bundle",
            to_node: "node:live-bundle",
            topic: DEFAULT_CONTROL_INGRESS_TOPIC,
            sequence: 5,
            peer_bootstrap_refs: &[],
            authority_refs: &apply_authority_refs,
            policy_refs: &[],
            resource_refs: &[],
            evidence_refs: &[],
        })
        .expect("apply authority envelope");
        let apply_authority_diagnostics =
            live_send_authority_grant_diagnostics(&bundle_sender, &apply_authority_envelope)
                .expect("apply authority diagnostics");
        assert!(apply_authority_diagnostics.iter().any(|value| value.contains("is not a grant")));
        assert!(
            apply_authority_diagnostics
                .iter()
                .any(|value| value.contains("authority delegation missing admitted grant"))
        );

        let malformed_root = temp_dir("node-control-live-workflow-bundle-malformed");
        init_local_node(&NodeDaemonInitInput {
            state_root: &malformed_root,
            node_id: "node:live-bundle-malformed",
        })
        .expect("init malformed root");
        let malformed =
            record("node-control-live-workflow-bundle-v1", vec![string(NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_SCHEMA)]);
        assert!(
            import_node_control_live_workflow_bundle(&NodeControlLiveWorkflowBundleImportInput {
                state_root: &malformed_root,
                bundle_value: &malformed,
                expected_node: Some("node:live-bundle"),
                expected_topic: Some(DEFAULT_CONTROL_INGRESS_TOPIC),
                expected_endpoint: Some(&ticket.live_endpoint_id),
                expected_peer: Some("peer:live-bundle"),
                expected_operations: &operations,
                expected_target_scope: Some("*"),
                expected_resource_scope: Some("*"),
                as_of_sequence: 2,
                as_of_epoch: 2,
            })
            .is_err()
        );
        let malformed_verify = verify_node_control_live_workflow_bundle(&NodeControlLiveWorkflowBundleVerifyInput {
            bundle_value: &malformed,
            expected_node: Some("node:live-bundle"),
            expected_topic: Some(DEFAULT_CONTROL_INGRESS_TOPIC),
            expected_endpoint: Some(&ticket.live_endpoint_id),
            expected_peer: Some("peer:live-bundle"),
            expected_operations: &operations,
            expected_target_scope: Some("*"),
            expected_resource_scope: Some("*"),
            as_of_sequence: 2,
            as_of_epoch: 2,
        })
        .expect("malformed verify receipt");
        assert_eq!(malformed_verify.decision, "deny");
        assert!(malformed_verify.diagnostics.iter().any(|value| value.contains("parse failed")));
        let malformed_gate = gate_node_control_live_workflow_bundle(&NodeControlLiveWorkflowBundleGateInput {
            bundle_value: &malformed,
            verify_receipt_value: Some(&malformed_verify.receipt_value),
            require_verify_receipt: true,
            expected_node: Some("node:live-bundle"),
            expected_topic: Some(DEFAULT_CONTROL_INGRESS_TOPIC),
            expected_endpoint: Some(&ticket.live_endpoint_id),
            expected_peer: Some("peer:live-bundle"),
            expected_operations: &operations,
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
    fn node_control_live_send_reaches_bounded_listener() {
        let runtime = tokio::runtime::Builder::new_multi_thread().enable_all().build().expect("runtime");
        runtime.block_on(async {
            let root = temp_dir("node-control-live-send");
            init_local_node(&NodeDaemonInitInput {
                state_root: &root,
                node_id: "node:live-send",
            })
            .expect("init node");
            run_local_node(&NodeDaemonRunInput { state_root: &root }).expect("run node");
            let identity =
                node_identity::parse_node_identity(&read_preserves(&root.join(IDENTITY_FILE)).expect("identity"))
                    .expect("parse identity");
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
                .subscribe(node_control_live_topic_id(DEFAULT_CONTROL_INGRESS_TOPIC), Vec::new())
                .await
                .expect("receiver subscribe");
            let policy_refs = vec![local_ref("node-control-policy", "live-send").expect("policy ref")];
            let resource_refs = vec![local_ref("node-control-resource", "live-send").expect("resource ref")];
            let admission = admit_node_control_live_peer(&NodeControlLivePeerAdmitInput {
                state_root: &root,
                ticket_value: &live_ticket.value,
                peer_id: "peer:external-send",
                sequence: 1,
                expires_at: None,
                policy_refs: &policy_refs,
                evidence_refs: &[],
            })
            .expect("peer admission");
            let peer_bootstrap_refs = vec![admission.admission_ref.clone()];
            let authority_refs =
                test_live_authority_refs(&root, "peer:external-send", "node:live-send", "status", &policy_refs)
                    .expect("authority grant ref");
            let request_value = node_runtime::node_control_request_value(&node_runtime::ControlRequestValueInput {
                operation: "status",
                target_ref: None,
                payload_ref: None,
                authority_refs: &authority_refs,
                policy_refs: &policy_refs,
                resource_refs: &resource_refs,
                evidence_refs: &[],
            })
            .expect("status request");
            let send_input = NodeControlLiveSendInput {
                state_root: Some(&root),
                request_value: &request_value,
                receiver_ticket_value: &live_ticket.value,
                from_peer: "peer:external-send",
                sequence: 1,
                expected_operation_ref: None,
                expected_receiver_node: None,
                expected_topic: None,
                expected_endpoint: None,
                max_attempts: DEFAULT_CONTROL_LIVE_SEND_ATTEMPTS,
                peer_bootstrap_refs: &peer_bootstrap_refs,
                authority_refs: &authority_refs,
                policy_refs: &policy_refs,
                resource_refs: &resource_refs,
                evidence_refs: &[],
                join_timeout_ms: 10_000,
            };
            let sent = send_node_control_live_ingress(&send_input).await.expect("live send");
            assert_eq!(ledger::artifact_kind(&sent.send_receipt_value), "node-control-live-send-receipt");
            assert!(sent.transport_receipt_ref.is_some());
            assert_eq!(
                sent.operation_ref,
                parse_node_control_ingress_envelope(&sent.envelope_value).expect("envelope").operation_ref
            );
            let duplicate = send_node_control_live_ingress(&send_input).await.expect("duplicate live send");
            assert_eq!(duplicate.send_receipt_ref, sent.send_receipt_ref);
            assert!(duplicate.duplicate_receipt_ref.is_some());
            assert_eq!(
                ledger::artifact_kind(duplicate.duplicate_receipt_value.as_ref().expect("duplicate receipt")),
                "node-control-live-send-duplicate-receipt"
            );
            let listener_input = NodeControlLiveServeInput {
                state_root: &root,
                topic: DEFAULT_CONTROL_INGRESS_TOPIC,
                max_events: 8,
                event_timeout_ms: 1_000,
                max_requests_per_tick: 1,
                supervisor_policy_value: None,
            };
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
            assert_eq!(listener.service.decision, "pass");
            assert_eq!(listener.service.processed_request_refs.len(), 1);
            assert_eq!(listener.transport_receipt_refs.len(), 1);
            assert!(listener.observed_events > 0);
            let authority_value = read_node_ledger_artifact(&root, &authority_refs[0]).expect("authority value");
            let receive_values = listener
                .transport_receipt_refs
                .iter()
                .map(|reference| read_node_ledger_artifact(&root, reference).expect("receive receipt value"))
                .collect::<Vec<_>>();
            let receive_value_refs = receive_values.iter().collect::<Vec<_>>();
            let workflow = node_control_live_workflow_receipt(&NodeControlLiveWorkflowInput {
                state_root: Some(&root),
                receiver_ticket_value: &live_ticket.value,
                peer_admission_value: &admission.value,
                authority_grant_value: &authority_value,
                send_receipt_value: &sent.send_receipt_value,
                receive_receipt_values: &receive_value_refs,
                listener_receipt_value: Some(&listener.listener_receipt_value),
                service_receipt_value: &listener.service.service_receipt_value,
            })
            .expect("workflow receipt");
            assert_eq!(workflow.decision, "pass");
            assert_eq!(ledger::artifact_kind(&workflow.receipt_value), "node-control-live-workflow-receipt");
        });
    }

    #[test]
    fn node_control_live_authority_delegation_fails_closed() {
        struct Case<'a> {
            name: &'a str,
            grant_peer: Option<&'a str>,
            grant_node: &'a str,
            grant_operations: &'a [&'a str],
            target_ref: Option<&'a str>,
            target_scope: &'a str,
            resource_scope: &'a str,
            epoch: u64,
            expires_at: Option<u64>,
            revoked: bool,
            sequence: u64,
            expected: &'a str,
        }
        let cases = [
            Case {
                name: "unknown-grant",
                grant_peer: None,
                grant_node: "node:live-authority",
                grant_operations: &["status"],
                target_ref: None,
                target_scope: "*",
                resource_scope: "*",
                epoch: 1,
                expires_at: None,
                revoked: false,
                sequence: 1,
                expected: "not found",
            },
            Case {
                name: "wrong-peer",
                grant_peer: Some("peer:other"),
                grant_node: "node:live-authority",
                grant_operations: &["status"],
                target_ref: None,
                target_scope: "*",
                resource_scope: "*",
                epoch: 1,
                expires_at: None,
                revoked: false,
                sequence: 1,
                expected: "does not match peer:case",
            },
            Case {
                name: "wrong-op",
                grant_peer: Some("peer:case"),
                grant_node: "node:live-authority",
                grant_operations: &["shutdown"],
                target_ref: None,
                target_scope: "*",
                resource_scope: "*",
                epoch: 1,
                expires_at: None,
                revoked: false,
                sequence: 1,
                expected: "does not allow operation status",
            },
            Case {
                name: "wrong-target",
                grant_peer: Some("peer:case"),
                grant_node: "node:live-authority",
                grant_operations: &["status"],
                target_ref: Some("blake3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
                target_scope: "blake3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
                resource_scope: "*",
                epoch: 1,
                expires_at: None,
                revoked: false,
                sequence: 1,
                expected: "target scope",
            },
            Case {
                name: "wrong-resource",
                grant_peer: Some("peer:case"),
                grant_node: "node:live-authority",
                grant_operations: &["status"],
                target_ref: None,
                target_scope: "*",
                resource_scope: "blake3:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
                epoch: 1,
                expires_at: None,
                revoked: false,
                sequence: 1,
                expected: "resource scope",
            },
            Case {
                name: "expired",
                grant_peer: Some("peer:case"),
                grant_node: "node:live-authority",
                grant_operations: &["status"],
                target_ref: None,
                target_scope: "*",
                resource_scope: "*",
                epoch: 1,
                expires_at: Some(1),
                revoked: false,
                sequence: 2,
                expected: "expired at epoch 1",
            },
            Case {
                name: "revoked",
                grant_peer: Some("peer:case"),
                grant_node: "node:live-authority",
                grant_operations: &["status"],
                target_ref: None,
                target_scope: "*",
                resource_scope: "*",
                epoch: 1,
                expires_at: None,
                revoked: true,
                sequence: 1,
                expected: "has revocation refs",
            },
        ];
        for case in cases {
            let root = temp_dir(&format!("node-control-live-authority-{}", case.name));
            init_local_node(&NodeDaemonInitInput {
                state_root: &root,
                node_id: "node:live-authority",
            })
            .expect("init node");
            run_local_node(&NodeDaemonRunInput { state_root: &root }).expect("run node");
            let policy_refs = vec![local_ref("node-control-policy", case.name).expect("policy ref")];
            let resource_refs = vec![local_ref("node-control-resource", case.name).expect("resource ref")];
            let peer_bootstrap_refs =
                test_live_peer_bootstrap_refs(&root, "peer:case", DEFAULT_CONTROL_INGRESS_TOPIC, &policy_refs)
                    .expect("peer admission ref");
            let authority_refs = if let Some(grant_peer) = case.grant_peer {
                let operations =
                    case.grant_operations.iter().map(|operation| (*operation).to_string()).collect::<Vec<_>>();
                let revocation_refs = if case.revoked {
                    vec![local_ref("node-control-revocation", case.name).expect("revocation ref")]
                } else {
                    Vec::new()
                };
                let grant_value = node_control_authority_grant_value(&NodeControlAuthorityGrantInput {
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
                vec![
                    import_node_control_authority_grant(&root, &grant_value).expect("import authority grant").grant_ref,
                ]
            } else {
                vec![local_ref("node-control-authority", case.name).expect("authority ref")]
            };
            let request_value = node_runtime::node_control_request_value(&node_runtime::ControlRequestValueInput {
                operation: "status",
                target_ref: case.target_ref,
                payload_ref: None,
                authority_refs: &authority_refs,
                policy_refs: &policy_refs,
                resource_refs: &resource_refs,
                evidence_refs: &[],
            })
            .expect("status request");
            let envelope = node_control_live_ingress_envelope(&NodeControlIngressEnvelopeInput {
                request_value: &request_value,
                from_peer: "peer:case",
                to_node: "node:live-authority",
                topic: DEFAULT_CONTROL_INGRESS_TOPIC,
                sequence: case.sequence,
                peer_bootstrap_refs: &peer_bootstrap_refs,
                authority_refs: &authority_refs,
                policy_refs: &policy_refs,
                resource_refs: &resource_refs,
                evidence_refs: &[],
            })
            .expect("live envelope");
            publish_node_control_ingress(&NodeControlIngressPublishInput {
                state_root: &root,
                envelope_value: &envelope.value,
            })
            .expect("publish live envelope");
            let delivered = deliver_node_control_ingress(&NodeControlIngressDeliverInput {
                state_root: &root,
                topic: DEFAULT_CONTROL_INGRESS_TOPIC,
                envelope_ref: &envelope.envelope_ref,
            })
            .expect("deliver live envelope");
            assert!(!delivered.has_enqueued, "{} enqueued", case.name);
            let receipt_text = to_text(&delivered.ingress_receipt_value).expect("receipt text");
            assert!(receipt_text.contains(case.expected), "{} receipt: {receipt_text}", case.name);
            assert!(next_pending_control_request(&root).expect("pending request scan").is_none());
        }
    }

    #[tokio::test]
    async fn node_control_live_serve_listener_loopback_dispatches_through_service() {
        let root = temp_dir("node-control-live-listener");
        init_local_node(&NodeDaemonInitInput {
            state_root: &root,
            node_id: "node:live-listener",
        })
        .expect("init node");
        run_local_node(&NodeDaemonRunInput { state_root: &root }).expect("run node");
        let policy_refs = vec![local_ref("node-control-policy", "live-listener").expect("policy ref")];
        let authority_refs =
            test_live_authority_refs(&root, "peer:listener", "node:live-listener", "status", &policy_refs)
                .expect("authority grant ref");
        let resource_refs = vec![local_ref("node-control-resource", "live-listener").expect("resource ref")];
        let peer_bootstrap_refs =
            test_live_peer_bootstrap_refs(&root, "peer:listener", DEFAULT_CONTROL_INGRESS_TOPIC, &policy_refs)
                .expect("peer admission ref");
        let request_value = node_runtime::node_control_request_value(&node_runtime::ControlRequestValueInput {
            operation: "status",
            target_ref: None,
            payload_ref: None,
            authority_refs: &authority_refs,
            policy_refs: &policy_refs,
            resource_refs: &resource_refs,
            evidence_refs: &[],
        })
        .expect("status request");

        let loopback = node_control_live_serve_listener_loopback(&NodeControlLiveServeLoopbackInput {
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
            ledger::artifact_kind(&loopback.listener.listener_receipt_value),
            "node-control-live-listener-receipt"
        );
        assert_eq!(loopback.listener.service.decision, "pass");
        assert_eq!(loopback.listener.service.processed_request_refs.len(), 1);
        assert_eq!(loopback.listener.transport_receipt_refs.len(), 1);
        assert!(loopback.listener.observed_events >= 1);
    }

    #[tokio::test]
    async fn node_control_live_iroh_loopback_delivers_to_durable_inbox() {
        let root = temp_dir("node-control-live-iroh");
        init_local_node(&NodeDaemonInitInput {
            state_root: &root,
            node_id: "node:live-ingress",
        })
        .expect("init node");
        run_local_node(&NodeDaemonRunInput { state_root: &root }).expect("run node");
        let policy_refs = vec![local_ref("node-control-policy", "live-ingress").expect("policy ref")];
        let authority_refs = test_live_authority_refs(&root, "peer:live", "node:live-ingress", "status", &policy_refs)
            .expect("authority grant ref");
        let resource_refs = vec![local_ref("node-control-resource", "live-ingress").expect("resource ref")];
        let peer_bootstrap_refs =
            test_live_peer_bootstrap_refs(&root, "peer:live", DEFAULT_CONTROL_INGRESS_TOPIC, &policy_refs)
                .expect("peer admission ref");
        let request_value = node_runtime::node_control_request_value(&node_runtime::ControlRequestValueInput {
            operation: "status",
            target_ref: None,
            payload_ref: None,
            authority_refs: &authority_refs,
            policy_refs: &policy_refs,
            resource_refs: &resource_refs,
            evidence_refs: &[],
        })
        .expect("status request");

        let live = node_control_live_iroh_loopback(&NodeControlLiveLoopbackInput {
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
        assert_eq!(ledger::artifact_kind(&live.publish_receipt_value), "node-control-live-transport-receipt");
        assert_eq!(ledger::artifact_kind(&live.receive_receipt_value), "node-control-live-transport-receipt");

        let served = serve_node_control(&NodeControlServeInput {
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
    fn node_control_service_delivers_ingress_and_dispatches_through_loop() {
        let root = temp_dir("node-control-service-ingress");
        init_local_node(&NodeDaemonInitInput {
            state_root: &root,
            node_id: "node:service-ingress",
        })
        .expect("init node");
        run_local_node(&NodeDaemonRunInput { state_root: &root }).expect("run node");
        let authority_refs = vec![local_ref("node-control-authority", "service-ingress").expect("authority ref")];
        let policy_refs = vec![local_ref("node-control-policy", "service-ingress").expect("policy ref")];
        let resource_refs = vec![local_ref("node-control-resource", "service-ingress").expect("resource ref")];
        let peer_bootstrap_refs = vec![local_ref("peer-bootstrap", "peer:service").expect("bootstrap ref")];
        let request_value = node_runtime::node_control_request_value(&node_runtime::ControlRequestValueInput {
            operation: "status",
            target_ref: None,
            payload_ref: None,
            authority_refs: &authority_refs,
            policy_refs: &policy_refs,
            resource_refs: &resource_refs,
            evidence_refs: &[],
        })
        .expect("status request");
        let envelope = node_control_ingress_envelope(&NodeControlIngressEnvelopeInput {
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
        publish_node_control_ingress(&NodeControlIngressPublishInput {
            state_root: &root,
            envelope_value: &envelope.value,
        })
        .expect("publish ingress");

        let served = serve_node_control(&NodeControlServeInput {
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
        assert_eq!(ledger::artifact_kind(&served.service_receipt_value), "node-control-service-run-receipt");
        let control_value = read_preserves(&control_outbox_receipt_path(&root, &served.processed_request_refs[0]))
            .expect("read served control receipt");
        let control = node_runtime::parse_node_control_receipt(&control_value).expect("parse served control");
        assert_eq!(control.decision, "pass");
    }

    #[test]
    fn node_control_service_duplicate_lock_denies_before_side_effects() {
        let root = temp_dir("node-control-service-duplicate");
        init_local_node(&NodeDaemonInitInput {
            state_root: &root,
            node_id: "node:service-duplicate",
        })
        .expect("init node");
        run_local_node(&NodeDaemonRunInput { state_root: &root }).expect("run node");
        let startup = current_startup_receipt(&root).expect("startup");
        let identity =
            node_identity::parse_node_identity(&read_preserves(&root.join(IDENTITY_FILE)).expect("identity"))
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
        submit_control_request(&NodeControlSubmitInput {
            state_root: &root,
            request_value: &request.value,
        })
        .expect("submit pending request");

        let served = serve_node_control(&NodeControlServeInput {
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
        let text = to_text(&served.service_receipt_value).expect("service receipt text");
        assert!(text.contains("already active"));
    }

    #[test]
    fn node_control_supervisor_policy_recovers_stale_lock_and_bounds_shutdown() {
        let root = temp_dir("node-control-supervisor-policy");
        init_local_node(&NodeDaemonInitInput {
            state_root: &root,
            node_id: "node:supervisor-policy",
        })
        .expect("init node");
        run_local_node(&NodeDaemonRunInput { state_root: &root }).expect("run node");
        let startup = current_startup_receipt(&root).expect("startup");
        let identity =
            node_identity::parse_node_identity(&read_preserves(&root.join(IDENTITY_FILE)).expect("identity"))
                .expect("parse identity");
        let service_run_ref = local_ref("node-control-service-run", "stale").expect("service run ref");
        let stale_lock = service_lock_value(&ServiceLockValueInput {
            state_root: &root,
            startup_receipt_ref: &startup.receipt_ref,
            node_id: &identity.node_id,
            topic: DEFAULT_CONTROL_INGRESS_TOPIC,
            max_ticks: 1,
            max_requests_per_tick: 1,
            service_run_ref: &service_run_ref,
        })
        .expect("stale lock");
        write_preserves(&root.join(CONTROL_SERVICE_LOCK_FILE), &stale_lock).expect("write stale lock");
        let policy_refs = vec![local_ref("node-control-supervisor-policy", "recover").expect("policy ref")];
        let recover_policy = node_control_supervisor_policy_value(&NodeControlSupervisorPolicyInput {
            max_restarts: 1,
            restart_window_ticks: 1,
            heartbeat_timeout_ticks: 1,
            shutdown_drain_ticks: 1,
            stale_lock_recovery: true,
            policy_refs: &policy_refs,
            evidence_refs: &[],
        })
        .expect("recover policy");

        let recovered = serve_node_control(&NodeControlServeInput {
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
        let restart_once = serve_node_control(&NodeControlServeInput {
            state_root: &root,
            topic: DEFAULT_CONTROL_INGRESS_TOPIC,
            max_ticks: 1,
            max_requests_per_tick: 1,
            supervisor_policy_value: Some(&recover_policy),
        })
        .expect("allowed restart");
        assert_eq!(restart_once.decision, "pass");
        let restart_denied = serve_node_control(&NodeControlServeInput {
            state_root: &root,
            topic: DEFAULT_CONTROL_INGRESS_TOPIC,
            max_ticks: 1,
            max_requests_per_tick: 1,
            supervisor_policy_value: Some(&recover_policy),
        })
        .expect("bounded restart denial");
        assert_eq!(restart_denied.decision, "deny");
        assert_eq!(restart_denied.ticks, 0);
        let restart_denied_text = to_text(&restart_denied.service_receipt_value).expect("restart denial receipt text");
        assert!(restart_denied_text.contains("restart attempts"));

        let shutdown = shutdown_request().expect("shutdown request");
        submit_control_request(&NodeControlSubmitInput {
            state_root: &root,
            request_value: &shutdown.value,
        })
        .expect("submit shutdown");
        let tight_policy = node_control_supervisor_policy_value(&NodeControlSupervisorPolicyInput {
            max_restarts: 0,
            restart_window_ticks: 1,
            heartbeat_timeout_ticks: 1,
            shutdown_drain_ticks: 0,
            stale_lock_recovery: false,
            policy_refs: &policy_refs,
            evidence_refs: &[],
        })
        .expect("tight policy");
        let stopped = serve_node_control(&NodeControlServeInput {
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
        let text = to_text(&stopped.service_receipt_value).expect("service receipt text");
        assert!(text.contains("exceeded supervisor bound"));
    }

    #[test]
    fn node_control_service_heartbeats_continue_and_shutdown_stops() {
        let root = temp_dir("node-control-service-shutdown");
        init_local_node(&NodeDaemonInitInput {
            state_root: &root,
            node_id: "node:service-shutdown",
        })
        .expect("init node");
        run_local_node(&NodeDaemonRunInput { state_root: &root }).expect("run node");
        let idle = serve_node_control(&NodeControlServeInput {
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
        submit_control_request(&NodeControlSubmitInput {
            state_root: &root,
            request_value: &shutdown.value,
        })
        .expect("submit shutdown");
        let stopped = serve_node_control(&NodeControlServeInput {
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
        let root = temp_dir("node-control-operations");
        init_local_node(&NodeDaemonInitInput {
            state_root: &root,
            node_id: "node:ops",
        })
        .expect("init node");
        run_local_node(&NodeDaemonRunInput { state_root: &root }).expect("run node");
        let authority_refs = vec![local_ref("node-control-authority", "ops").expect("authority ref")];
        let policy_refs = vec![local_ref("node-control-policy", "ops").expect("policy ref")];
        let resource_refs = vec![local_ref("node-control-resource", "ops").expect("resource ref")];

        let payload_value = record("node-control-install-payload", vec![string("payload")]);
        let payload_ref = import_node_artifact(&root, &payload_value).expect("import payload");
        let payload_provenance =
            provenance::synthetic_reviewed_provenance_record(&payload_ref).expect("payload provenance");
        let payload_provenance_ref =
            import_node_artifact(&root, &payload_provenance).expect("import payload provenance");
        let install_evidence_refs = vec![payload_provenance_ref];
        let install_value = node_runtime::node_control_request_value(&node_runtime::ControlRequestValueInput {
            operation: "install",
            target_ref: None,
            payload_ref: Some(&payload_ref),
            authority_refs: &authority_refs,
            policy_refs: &policy_refs,
            resource_refs: &resource_refs,
            evidence_refs: &install_evidence_refs,
        })
        .expect("install request");
        let install_submitted = submit_control_request(&NodeControlSubmitInput {
            state_root: &root,
            request_value: &install_value,
        })
        .expect("submit install");
        let install_dispatch = dispatch_control_request(&NodeControlDispatchInput {
            state_root: &root,
            request_path: Some(&install_submitted.inbox_path),
        })
        .expect("dispatch install");
        let install_receipt =
            node_runtime::parse_node_control_receipt(&install_dispatch.control_receipt_value).expect("install receipt");
        assert_eq!(install_receipt.decision, "pass");
        let installed = artifacts::list_artifacts(&root.join("registry"), Some("node-control-artifact"))
            .expect("list installed artifacts");
        assert_eq!(installed.len(), 1);

        let gate_value = octet_gate::synthetic_clean_octet_gate_receipt_for_tests().expect("gate receipt");
        let gate_ref = import_node_artifact(&root, &gate_value).expect("import gate");
        let gate_target = local_ref("node-control-gate-target", "ops").expect("gate target");
        let gate_request = node_runtime::node_control_request_value(&node_runtime::ControlRequestValueInput {
            operation: "gate",
            target_ref: Some(&gate_target),
            payload_ref: Some(&gate_ref),
            authority_refs: &authority_refs,
            policy_refs: &policy_refs,
            resource_refs: &resource_refs,
            evidence_refs: &[],
        })
        .expect("gate request");
        let gate_submitted = submit_control_request(&NodeControlSubmitInput {
            state_root: &root,
            request_value: &gate_request,
        })
        .expect("submit gate");
        let gate_dispatch = dispatch_control_request(&NodeControlDispatchInput {
            state_root: &root,
            request_path: Some(&gate_submitted.inbox_path),
        })
        .expect("dispatch gate");
        let gate_receipt =
            node_runtime::parse_node_control_receipt(&gate_dispatch.control_receipt_value).expect("gate receipt");
        assert_eq!(gate_receipt.decision, "pass");
        assert!(gate_receipt.subreceipt_refs.iter().any(|reference| reference.starts_with("blake3:")));

        let job_fixture = install_node_job_fixture(&root);
        let execution_request_ref =
            import_node_artifact(&root, &job_fixture.execution_request).expect("import execution request");
        let admission_ref =
            import_node_artifact(&root, &job_fixture.admission_receipt).expect("import admission receipt");
        let job_provenance =
            provenance::synthetic_reviewed_provenance_record(&job_fixture.job_ref).expect("job provenance");
        let job_provenance_ref = import_node_artifact(&root, &job_provenance).expect("import job provenance");
        let run_evidence_refs = vec![job_provenance_ref];
        let run_request = node_runtime::node_control_request_value(&node_runtime::ControlRequestValueInput {
            operation: "run",
            target_ref: Some(&admission_ref),
            payload_ref: Some(&execution_request_ref),
            authority_refs: &authority_refs,
            policy_refs: &policy_refs,
            resource_refs: &resource_refs,
            evidence_refs: &run_evidence_refs,
        })
        .expect("run request");
        let run_submitted = submit_control_request(&NodeControlSubmitInput {
            state_root: &root,
            request_value: &run_request,
        })
        .expect("submit run");
        let run_dispatch = dispatch_control_request(&NodeControlDispatchInput {
            state_root: &root,
            request_path: Some(&run_submitted.inbox_path),
        })
        .expect("dispatch run");
        let run_receipt =
            node_runtime::parse_node_control_receipt(&run_dispatch.control_receipt_value).expect("run receipt");
        assert_eq!(run_receipt.decision, "pass");

        let kinds = ledger::list_artifacts(&root.join("ledger"))
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

    struct NodeJobFixture {
        execution_request: IOValue,
        admission_receipt: IOValue,
        job_ref: String,
    }

    fn install_node_job_fixture(root: &Path) -> NodeJobFixture {
        let registry = root.join("registry");
        let stage_schema = local_ref("node-job-stage-schema", "ops").expect("stage schema");
        let stage_policy = local_ref("node-job-stage-policy", "ops").expect("stage policy");
        let stage_evidence = local_ref("node-job-stage-evidence", "ops").expect("stage evidence");
        let stage_installer = local_ref("node-job-stage-installer", "ops").expect("stage installer");
        let stage_capability = local_ref("node-job-stage-capability", "ops").expect("stage capability");
        let source_stage = artifacts::install_artifact(&registry, &artifacts::ArtifactInstallInput {
            kind: "stage".to_string(),
            payload: job_dag::builtin_stage_operation_value("source").expect("source operation"),
            schema_refs: vec![stage_schema.clone()],
            dependency_refs: Vec::new(),
            effect_manifest_ref: None,
            policy_refs: vec![stage_policy.clone()],
            evidence_refs: vec![stage_evidence.clone()],
            installer_ref: stage_installer.clone(),
            capability_refs: vec![stage_capability.clone()],
        })
        .expect("install source stage");
        let map_stage = artifacts::install_artifact(&registry, &artifacts::ArtifactInstallInput {
            kind: "stage".to_string(),
            payload: job_dag::builtin_stage_operation_value("identity").expect("identity operation"),
            schema_refs: vec![stage_schema],
            dependency_refs: Vec::new(),
            effect_manifest_ref: None,
            policy_refs: vec![stage_policy],
            evidence_refs: vec![stage_evidence],
            installer_ref: stage_installer,
            capability_refs: vec![stage_capability],
        })
        .expect("install map stage");
        let source_node = job_dag::job_node_value(job_dag::NodeValueInput {
            id: "source",
            kind: "source",
            stage_artifact_ref: Some(&source_stage.artifact_ref),
            input_ports: &[],
            output_ports: &["out".to_string()],
            config: record("source", vec![record("values", vec![sequence(vec![string("node-job")])])]),
            effect_manifest_refs: &[],
            policy_refs: &[],
            evidence_refs: &[],
        })
        .expect("source node");
        let map_node = job_dag::job_node_value(job_dag::NodeValueInput {
            id: "map",
            kind: "map",
            stage_artifact_ref: Some(&map_stage.artifact_ref),
            input_ports: &["in".to_string()],
            output_ports: &["out".to_string()],
            config: record("op", vec![string("identity")]),
            effect_manifest_refs: &[],
            policy_refs: &[],
            evidence_refs: &[],
        })
        .expect("map node");
        let edge = job_dag::job_edge_value(job_dag::EdgeValueInput {
            from_node: "source",
            from_port: "out",
            to_node: "map",
            to_port: "in",
            schema_ref: None,
            partitioning: "single",
            materialization: "stream",
        })
        .expect("edge");
        let dag_value = job_dag::job_dag_value(job_dag::DagValueInput {
            nodes: vec![source_node, map_node],
            edges: vec![edge],
            output_roots: &["map".to_string()],
            schema_refs: &[],
            effect_manifest_refs: &[],
            policy_refs: &[],
            evidence_refs: &[],
        })
        .expect("dag value");
        let installed = job_dag::install_job_dag(&registry, &dag_value).expect("install job dag");
        let authority_ref = install_node_job_authority(&registry, &installed.job_ref);
        let gate_ref = install_node_clean_gate(&registry);
        let sync_ref = local_ref("node-job-sync", &installed.job_ref).expect("sync ref");
        let resource_refs = vec![local_ref("node-job-resource", &installed.job_ref).expect("resource ref")];
        let policy_refs = vec![local_ref("node-job-policy", &installed.job_ref).expect("policy ref")];
        let capability_refs = vec![authority_ref.clone()];
        let evidence_refs = vec![sync_ref.clone(), gate_ref];
        let admission_request = job_dag::job_admission_request_value(job_dag::AdmissionRequestValueInput {
            job_ref: &installed.job_ref,
            sync_ref: &sync_ref,
            stage_ids: &[],
            target_peer: "node:ops",
            policy_refs: &policy_refs,
            capability_refs: &capability_refs,
            evidence_refs: &evidence_refs,
            resource_refs: &resource_refs,
        })
        .expect("admission request");
        let admission = job_dag::admission_loopback(&registry, &admission_request).expect("admission loopback");
        assert_eq!(admission.plan.decision, "pass");
        let admission_ref = canonical_hash(&admission.receipt_value).expect("admission ref");
        let execution_request = job_dag::job_execution_request_value(job_dag::ExecutionRequestValueInput {
            job_ref: &installed.job_ref,
            admission_ref: &admission_ref,
            stage_ids: &admission.plan.stage_order,
            target_peer: "node:ops",
            storage_profile_ref: &local_ref("node-job-storage", &installed.job_ref).expect("storage ref"),
            cache_profile_ref: &local_ref("node-job-cache", &installed.job_ref).expect("cache ref"),
            chunk_profile_ref: &local_ref("node-job-chunks", &installed.job_ref).expect("chunks ref"),
            policy_refs: &policy_refs,
            capability_refs: &capability_refs,
            resource_refs: &resource_refs,
        })
        .expect("execution request");
        NodeJobFixture {
            execution_request,
            admission_receipt: admission.receipt_value,
            job_ref: installed.job_ref,
        }
    }

    fn install_node_job_authority(registry: &Path, job_ref: &str) -> String {
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
        let context_ref = canonical_hash(&context_value).expect("authority context ref");
        let install = artifacts::install_artifact(registry, &artifacts::ArtifactInstallInput {
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

    fn install_node_clean_gate(registry: &Path) -> String {
        let gate_value = octet_gate::synthetic_clean_octet_gate_receipt_for_tests().expect("clean gate");
        let gate_ref = canonical_hash(&gate_value).expect("gate ref");
        let install = artifacts::install_artifact(registry, &artifacts::ArtifactInstallInput {
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
        static TEMP_DIR_COUNTER: AtomicU64 = AtomicU64::new(0);
        let nonce = TEMP_DIR_COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("molten-{name}-{}-{nonce}", std::process::id()));
        if dir.exists() {
            fs::remove_dir_all(&dir).expect("remove stale temp dir");
        }
        fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }
}

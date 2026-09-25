
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
    pub inbox_entry: String,
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
    pub topology_profile_ref: Option<String>,
    pub transport_profile_ref: Option<String>,
    pub effective_max_attempts: u64,
    pub effective_join_timeout_ms: u64,
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

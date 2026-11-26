//! Public ingress, live publish, loopback, send, and listener outcome records.

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
    pub value: preserves::IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlIngressPublish {
    pub envelope_ref: String,
    pub envelope_path: std::path::PathBuf,
    pub receipt_ref: String,
    pub receipt_value: preserves::IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlIngressDeliver {
    pub envelope_ref: String,
    pub request_ref: String,
    pub ingress_receipt_ref: String,
    pub ingress_receipt_value: preserves::IOValue,
    pub idempotency_receipt_ref: Option<String>,
    pub queue_receipt_ref: Option<String>,
    pub has_enqueued: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlLiveIngressPublish {
    pub envelope_ref: String,
    pub transport_receipt_ref: String,
    pub transport_receipt_value: preserves::IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlLiveIngressReceive {
    pub envelope_ref: String,
    pub transport_receipt_ref: String,
    pub transport_receipt_value: preserves::IOValue,
    pub ingress_receipt_ref: String,
    pub ingress_receipt_value: preserves::IOValue,
    pub has_enqueued: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlLiveLoopback {
    pub envelope_ref: String,
    pub publish_receipt_ref: String,
    pub publish_receipt_value: preserves::IOValue,
    pub receive_receipt_ref: String,
    pub receive_receipt_value: preserves::IOValue,
    pub ingress_receipt_ref: String,
    pub has_enqueued: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlLiveSend {
    pub envelope_ref: String,
    pub envelope_value: preserves::IOValue,
    pub operation_ref: String,
    pub receiver_ticket_ref: String,
    pub receiver_endpoint_id: String,
    pub transport_receipt_ref: Option<String>,
    pub transport_receipt_value: Option<preserves::IOValue>,
    pub retry_receipt_refs: Vec<String>,
    pub retry_receipt_values: Vec<preserves::IOValue>,
    pub duplicate_receipt_ref: Option<String>,
    pub duplicate_receipt_value: Option<preserves::IOValue>,
    pub send_receipt_ref: String,
    pub send_receipt_value: preserves::IOValue,
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
    pub value: preserves::IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlLiveWorkflowReceipt {
    pub receipt_ref: String,
    pub receipt_value: preserves::IOValue,
    pub decision: String,
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
pub struct ControlLiveServe {
    pub listener_receipt_ref: String,
    pub listener_receipt_value: preserves::IOValue,
    pub service: super::outcomes::ControlServe,
    pub transport_receipt_refs: Vec<String>,
    pub neighbor_events: Vec<String>,
    pub observed_events: u64,
    pub bound_endpoint_id: String,
    pub live_ticket_ref: Option<String>,
    pub live_ticket_value: Option<preserves::IOValue>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlLiveServeLoopback {
    pub envelope_ref: String,
    pub publish_receipt_ref: String,
    pub listener: ControlLiveServe,
}

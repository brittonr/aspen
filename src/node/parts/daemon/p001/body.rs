
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
    pub receipt_value: Option<&'a IoValue>,
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
    pub receipt_value: &'a IoValue,
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
    pub identity_ref: &'a str,
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

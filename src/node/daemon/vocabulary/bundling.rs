//! Public inputs for live workflow bundle export, verify, gate, apply, reconcile, and ack.

#[derive(Debug, Clone, Copy)]
pub struct ControlLiveWorkflowInput<'a> {
    pub state_root: Option<&'a std::path::Path>,
    pub receiver_ticket_value: &'a preserves::IOValue,
    pub peer_admission_value: &'a preserves::IOValue,
    pub authority_grant_value: &'a preserves::IOValue,
    pub send_receipt_value: &'a preserves::IOValue,
    pub receive_receipt_values: &'a [&'a preserves::IOValue],
    pub listener_receipt_value: Option<&'a preserves::IOValue>,
    pub service_receipt_value: &'a preserves::IOValue,
}

#[derive(Debug, Clone, Copy)]
pub struct ControlLiveWorkflowBundleExportInput<'a> {
    pub receiver_ticket_value: &'a preserves::IOValue,
    pub peer_admission_value: &'a preserves::IOValue,
    pub authority_grant_value: &'a preserves::IOValue,
    pub receipt_values: &'a [&'a preserves::IOValue],
}

#[derive(Debug, Clone, Copy)]
pub struct ControlLiveWorkflowBundleVerifyInput<'a> {
    pub bundle_value: &'a preserves::IOValue,
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
    pub bundle_value: &'a preserves::IOValue,
    pub verify_receipt_value: Option<&'a preserves::IOValue>,
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
    pub state_root: &'a std::path::Path,
    pub bundle_value: &'a preserves::IOValue,
    pub receipt_value: Option<&'a preserves::IOValue>,
    pub is_gate_receipt_required: bool,
    pub request_value: Option<&'a preserves::IOValue>,
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
    pub topology_profile: Option<&'a super::offers::LiveTopologyProfile<'a>>,
    pub transport_profile: Option<&'a super::offers::LiveTransportProfile<'a>>,
    pub max_attempts: u64,
    pub join_timeout_ms: u64,
}

#[derive(Debug, Clone, Copy)]
pub struct ControlLiveWorkflowBundleReconcileInput<'a> {
    pub apply_receipt_value: &'a preserves::IOValue,
    pub send_receipt_value: Option<&'a preserves::IOValue>,
    pub ingress_receipt_value: Option<&'a preserves::IOValue>,
    pub queue_receipt_value: Option<&'a preserves::IOValue>,
    pub control_receipt_value: Option<&'a preserves::IOValue>,
    pub expected_envelope_ref: Option<&'a str>,
    pub expected_operation_ref: Option<&'a str>,
    pub expected_request_ref: Option<&'a str>,
}

#[derive(Debug, Clone, Copy)]
pub struct ControlLiveWorkflowBundleAckExportInput<'a> {
    pub apply_receipt_value: &'a preserves::IOValue,
    pub send_receipt_value: Option<&'a preserves::IOValue>,
    pub ingress_receipt_value: Option<&'a preserves::IOValue>,
    pub queue_receipt_value: Option<&'a preserves::IOValue>,
    pub control_receipt_value: Option<&'a preserves::IOValue>,
    pub reconcile_receipt_value: &'a preserves::IOValue,
}

#[derive(Debug, Clone, Copy)]
pub struct ControlLiveWorkflowBundleAckImportInput<'a> {
    pub state_root: &'a std::path::Path,
    pub ack_value: &'a preserves::IOValue,
    pub expected_bundle_ref: Option<&'a str>,
    pub expected_envelope_ref: Option<&'a str>,
    pub expected_operation_ref: Option<&'a str>,
    pub expected_request_ref: Option<&'a str>,
}

#[derive(Debug, Clone, Copy)]
pub struct ControlLiveWorkflowProtocolGateInput<'a> {
    pub bundle_value: &'a preserves::IOValue,
    pub receipt_value: &'a preserves::IOValue,
    pub apply_receipt_value: &'a preserves::IOValue,
    pub reconcile_receipt_value: &'a preserves::IOValue,
    pub ack_value: &'a preserves::IOValue,
    pub expected_envelope_ref: Option<&'a str>,
    pub expected_operation_ref: Option<&'a str>,
    pub expected_request_ref: Option<&'a str>,
}

#[derive(Debug, Clone, Copy)]
pub struct ControlLiveWorkflowBundleImportInput<'a> {
    pub state_root: &'a std::path::Path,
    pub bundle_value: &'a preserves::IOValue,
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


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
    pub identity_ref: String,
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

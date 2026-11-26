//! Public live workflow bundle records and their receipts.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlLiveWorkflowBundle {
    pub bundle_ref: String,
    pub bundle_value: preserves::IOValue,
    pub ticket_ref: String,
    pub peer_admission_ref: String,
    pub authority_grant_ref: String,
    pub receipt_refs: Vec<String>,
    pub ticket_value: preserves::IOValue,
    pub peer_admission_value: preserves::IOValue,
    pub authority_grant_value: preserves::IOValue,
    pub receipt_values: Vec<preserves::IOValue>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlLiveWorkflowBundleExport {
    pub bundle: ControlLiveWorkflowBundle,
    pub receipt_ref: String,
    pub receipt_value: preserves::IOValue,
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
    pub receipt_value: preserves::IOValue,
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
    pub receipt_value: preserves::IOValue,
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
pub struct ControlLiveWorkflowBundleApply {
    pub bundle_ref: String,
    pub gate_receipt_ref: Option<String>,
    pub recomputed_verify_receipt_ref: String,
    pub import_receipt_ref: Option<String>,
    pub imported_refs: Vec<String>,
    pub envelope_ref: Option<String>,
    pub operation_ref: Option<String>,
    pub send_receipt_ref: Option<String>,
    pub send_receipt_value: Option<preserves::IOValue>,
    pub diagnostics: Vec<String>,
    pub receipt_ref: String,
    pub receipt_value: preserves::IOValue,
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
    pub receipt_value: preserves::IOValue,
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
    pub ack_value: preserves::IOValue,
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
    pub apply_receipt_value: preserves::IOValue,
    pub send_receipt_value: Option<preserves::IOValue>,
    pub ingress_receipt_value: Option<preserves::IOValue>,
    pub queue_receipt_value: Option<preserves::IOValue>,
    pub control_receipt_value: Option<preserves::IOValue>,
    pub reconcile_receipt_value: preserves::IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlLiveWorkflowBundleAckExport {
    pub ack: ControlLiveWorkflowBundleAck,
    pub receipt_ref: String,
    pub receipt_value: preserves::IOValue,
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
    pub receipt_value: preserves::IOValue,
    pub decision: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlLiveWorkflowProtocolGate {
    pub session_id: String,
    pub install_receipt_ref: String,
    pub protocol_ref: String,
    pub receipt_ref: String,
    pub receipt_value: preserves::IOValue,
    pub decision: String,
    pub operation_count: usize,
    pub message_count: usize,
    pub diagnostics: Vec<String>,
    pub manifest_value: preserves::IOValue,
    pub install_receipt_value: preserves::IOValue,
    pub initial_state_values: Vec<preserves::IOValue>,
    pub operation_receipt_values: Vec<preserves::IOValue>,
    pub message_values: Vec<preserves::IOValue>,
    pub next_state_values: Vec<preserves::IOValue>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlLiveWorkflowBundleImport {
    pub bundle_ref: String,
    pub ticket_import_ref: Option<String>,
    pub authority_import_ref: Option<String>,
    pub imported_refs: Vec<String>,
    pub diagnostics: Vec<String>,
    pub receipt_ref: String,
    pub receipt_value: preserves::IOValue,
    pub decision: String,
}

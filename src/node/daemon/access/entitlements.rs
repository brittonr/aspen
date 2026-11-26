//! Public grant, ticket, peer admission, and lifecycle decision records.

#[derive(Debug, Clone, Copy)]
pub struct LiveWorkflowLifecycleInput<'a> {
    pub bundle: Option<&'a super::artifacts::ControlLiveWorkflowBundle>,
    pub gate: Option<&'a super::artifacts::ControlLiveWorkflowBundleGateReceipt>,
    pub apply: Option<&'a super::artifacts::ControlLiveWorkflowBundleApplyReceipt>,
    pub reconcile: Option<&'a super::artifacts::ControlLiveWorkflowBundleReconcileReceipt>,
    pub ack: Option<&'a super::artifacts::ControlLiveWorkflowBundleAck>,
    pub ack_import: Option<&'a super::artifacts::ControlLiveWorkflowBundleAckImport>,
    pub protocol_gate: Option<&'a super::artifacts::ControlLiveWorkflowProtocolGate>,
    pub expected_bundle_ref: Option<&'a str>,
    pub expected_envelope_ref: Option<&'a str>,
    pub expected_operation_ref: Option<&'a str>,
    pub expected_request_ref: Option<&'a str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiveWorkflowLifecycleDecision {
    pub decision: String,
    pub diagnostics: Vec<String>,
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
    pub value: preserves::IOValue,
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
    pub value: preserves::IOValue,
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
    pub value: preserves::IOValue,
}

#[derive(Debug, Clone, Copy)]
pub struct LiveTicketScopeInput<'a> {
    pub ticket: &'a ControlLiveTicket,
    pub admission: Option<&'a ControlLivePeerAdmission>,
    pub expected_node: Option<&'a str>,
    pub expected_topic: Option<&'a str>,
    pub expected_endpoint: Option<&'a str>,
    pub expected_peer: Option<&'a str>,
    pub as_of_sequence: u64,
    pub required_policy_refs: &'a [String],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiveTicketScopeDecision {
    pub decision: String,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlLiveTicketImport {
    pub decision: String,
    pub ticket_ref: String,
    pub peer_admission_ref: Option<String>,
    pub imported_refs: Vec<String>,
    pub diagnostics: Vec<String>,
    pub receipt_ref: String,
    pub receipt_value: preserves::IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlAuthorityGrantImport {
    pub decision: String,
    pub grant_ref: String,
    pub imported_refs: Vec<String>,
    pub diagnostics: Vec<String>,
    pub receipt_ref: String,
    pub receipt_value: preserves::IOValue,
}

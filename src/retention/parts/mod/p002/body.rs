
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteGcClearanceRequest {
    pub request_ref: String,
    pub requester_ref: String,
    pub peer_ref: String,
    pub object_ref: String,
    pub object_kind: String,
    pub retention_class: String,
    pub action: String,
    pub remote_ref: String,
    pub policy_ref: String,
    pub authority_ref: String,
    pub evidence_refs: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, Copy)]
pub struct RemoteGcClearanceResponseInput<'a, Root: ?Sized = Path> {
    pub root: &'a Root,
    pub request_value: &'a IoValue,
    pub evidence_refs: &'a [String],
    pub retained_refs: &'a [String],
    pub is_current: bool,
    pub revoked_refs: &'a [String],
    pub diagnostics: &'a [String],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteGcClearanceResponse {
    pub response_ref: String,
    pub decision: String,
    pub request_ref: String,
    pub request: RemoteGcClearanceRequest,
    pub clearance_ref: String,
    pub clearance: RemoteGcClearance,
    pub diagnostics: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, Copy)]
pub struct RemoteGcClearanceImportInput<'a, Root: ?Sized = Path> {
    pub root: &'a Root,
    pub request_value: &'a IoValue,
    pub response_value: &'a IoValue,
    pub expected_peer_ref: Option<&'a str>,
    pub expected_remote_ref: Option<&'a str>,
}

#[derive(Debug, Clone, Copy)]
pub struct RemoteGcClearanceImportValueInput<'a> {
    pub decision: &'a str,
    pub request_ref: &'a str,
    pub response_ref: &'a str,
    pub clearance_ref: Option<&'a str>,
    pub peer_ref: &'a str,
    pub remote_ref: &'a str,
    pub diagnostics: &'a [String],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteGcClearanceImport {
    pub import_ref: String,
    pub decision: String,
    pub request_ref: String,
    pub response_ref: String,
    pub clearance_ref: Option<String>,
    pub peer_ref: String,
    pub remote_ref: String,
    pub diagnostics: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, Copy)]
pub struct RemoteGcClearanceLiveLoopbackInput<'a, Root: ?Sized = Path> {
    pub root: &'a Root,
    pub requester_node_root: &'a Path,
    pub peer_node_root: &'a Path,
    pub requester_node_id: &'a str,
    pub peer_node_id: &'a str,
    pub topic: &'a str,
    pub request_sequence: u64,
    pub response_sequence: u64,
    pub requester_ref: &'a str,
    pub peer_ref: &'a str,
    pub object_ref: &'a str,
    pub object_kind: &'a str,
    pub retention_class: &'a str,
    pub action: &'a str,
    pub remote_ref: &'a str,
    pub policy_ref: &'a str,
    pub authority_ref: &'a str,
    pub retention_evidence_refs: &'a [String],
    pub response_evidence_refs: &'a [String],
    pub retained_refs: &'a [String],
    pub is_current: bool,
    pub revoked_refs: &'a [String],
    pub response_diagnostics: &'a [String],
    pub request_peer_bootstrap_refs: &'a [String],
    pub request_authority_refs: &'a [String],
    pub request_policy_refs: &'a [String],
    pub request_resource_refs: &'a [String],
    pub request_transport_evidence_refs: &'a [String],
    pub response_peer_bootstrap_refs: &'a [String],
    pub response_authority_refs: &'a [String],
    pub response_policy_refs: &'a [String],
    pub response_resource_refs: &'a [String],
    pub response_transport_evidence_refs: &'a [String],
}

#[derive(Debug, Clone, Copy)]
pub struct RemoteGcClearanceLiveRequestSendInput<'a, Root: ?Sized = Path> {
    pub root: &'a Root,
    pub requester_node_root: Option<&'a Path>,
    pub peer_ticket_value: &'a IoValue,
    pub requester_node_id: &'a str,
    pub peer_node_id: &'a str,
    pub topic: &'a str,
    pub sequence: u64,
    pub max_attempts: u64,
    pub join_timeout_ms: u64,
    pub requester_ref: &'a str,
    pub peer_ref: &'a str,
    pub object_ref: &'a str,
    pub object_kind: &'a str,
    pub retention_class: &'a str,
    pub action: &'a str,
    pub remote_ref: &'a str,
    pub policy_ref: &'a str,
    pub authority_ref: &'a str,
    pub retention_evidence_refs: &'a [String],
    pub peer_bootstrap_refs: &'a [String],
    pub authority_refs: &'a [String],
    pub policy_refs: &'a [String],
    pub resource_refs: &'a [String],
    pub transport_evidence_refs: &'a [String],
}

#[derive(Debug, Clone, Copy)]
pub struct RemoteGcClearanceLiveResponseSendInput<'a, Root: ?Sized = Path> {
    pub root: &'a Root,
    pub peer_node_root: Option<&'a Path>,
    pub requester_ticket_value: &'a IoValue,
    pub request_value: &'a IoValue,
    pub peer_node_id: &'a str,
    pub requester_node_id: &'a str,
    pub topic: &'a str,
    pub sequence: u64,
    pub max_attempts: u64,
    pub join_timeout_ms: u64,
    pub response_evidence_refs: &'a [String],
    pub retained_refs: &'a [String],
    pub is_current: bool,
    pub revoked_refs: &'a [String],
    pub response_diagnostics: &'a [String],
    pub peer_bootstrap_refs: &'a [String],
    pub authority_refs: &'a [String],
    pub policy_refs: &'a [String],
    pub resource_refs: &'a [String],
    pub transport_evidence_refs: &'a [String],
}

#[derive(Debug, Clone, Copy)]
pub struct RemoteGcClearanceLiveImportWorkflowInput<'a, Root: ?Sized = Path> {
    pub root: &'a Root,
    pub request_value: &'a IoValue,
    pub response_value: &'a IoValue,
    pub request_control_value: &'a IoValue,
    pub request_send_receipt_value: &'a IoValue,
    pub request_receive_receipt_value: &'a IoValue,
    pub request_ingress_ref: &'a str,
    pub response_control_value: &'a IoValue,
    pub response_send_receipt_value: &'a IoValue,
    pub response_receive_receipt_value: &'a IoValue,
    pub response_ingress_ref: &'a str,
    pub expected_peer_ref: Option<&'a str>,
    pub expected_remote_ref: Option<&'a str>,
}

#[derive(Debug, Clone, Copy)]
pub struct RemoteGcClearanceLiveWorkflowValueInput<'a> {
    pub request_value: &'a IoValue,
    pub response_value: &'a IoValue,
    pub import_value: &'a IoValue,
    pub request_control_ref: &'a str,
    pub request_publish_ref: &'a str,
    pub request_receive_ref: &'a str,
    pub request_ingress_ref: &'a str,
    pub response_control_ref: &'a str,
    pub response_publish_ref: &'a str,
    pub response_receive_ref: &'a str,
    pub response_ingress_ref: &'a str,
    pub transport_diagnostics: &'a [String],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteGcClearanceLiveWorkflow {
    pub workflow_ref: String,
    pub decision: String,
    pub request_ref: String,
    pub response_ref: String,
    pub import_ref: String,
    pub clearance_ref: Option<String>,
    pub peer_ref: String,
    pub remote_ref: String,
    pub request_live_refs: Vec<String>,
    pub response_live_refs: Vec<String>,
    pub diagnostics: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteGcClearanceLiveLoopback {
    pub request: RemoteGcClearanceRequest,
    pub response: RemoteGcClearanceResponse,
    pub import: RemoteGcClearanceImport,
    pub workflow: RemoteGcClearanceLiveWorkflow,
    pub request_publish_receipt_value: IoValue,
    pub request_receive_receipt_value: IoValue,
    pub response_publish_receipt_value: IoValue,
    pub response_receive_receipt_value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteGcClearanceLiveRequestSend {
    pub request: RemoteGcClearanceRequest,
    pub control_ref: String,
    pub control_value: IoValue,
    pub send: crate::node_daemon::ControlLiveSend,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteGcClearanceLiveResponseSend {
    pub response: RemoteGcClearanceResponse,
    pub control_ref: String,
    pub control_value: IoValue,
    pub send: crate::node_daemon::ControlLiveSend,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteGcClearanceLiveImportWorkflow {
    pub import: RemoteGcClearanceImport,
    pub workflow: RemoteGcClearanceLiveWorkflow,
    pub request_send_receipt_ref: String,
    pub response_send_receipt_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DestructiveAdmission {
    pub decision: String,
    pub diagnostics: Vec<String>,
    pub admitted_refs: Vec<String>,
    pub has_delete_authority: bool,
    pub has_remote_gc_clearance: bool,
}

pub struct DestructiveAdmissionInput<'a, Root: ?Sized = Path> {
    pub root: &'a Root,
    pub evidence: &'a DestructiveEvidence,
    pub object_ref: &'a str,
    pub object_kind: &'a str,
    pub retention_class: &'a str,
    pub action: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Receipt {
    pub receipt_ref: String,
    pub decision: String,
    pub action: String,
    pub object_ref: String,
    pub object_kind: String,
    pub retention_class: String,
    pub requester_ref: String,
    pub index_ref: String,
    pub pin_refs: Vec<String>,
    pub retained_refs: Vec<String>,
    pub remote_refs: Vec<String>,
    pub tombstone_ref: Option<String>,
    pub diagnostics: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tombstone {
    pub tombstone_ref: String,
    pub object_ref: String,
    pub object_kind: String,
    pub retention_class: String,
    pub action: String,
    pub receipt_ref: String,
    pub policy_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub value: IoValue,
}


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
    topology_profile_ref: Option<&'a str>,
    transport_profile_ref: Option<&'a str>,
    effective_max_attempts: u64,
    effective_join_timeout_ms: u64,
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
    pub profile_resolution_ref: String,
    pub config_value: IoValue,
    pub identity_receipt_value: IoValue,
    pub profile_resolution_value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Run {
    pub startup_ref: String,
    pub startup_value: IoValue,
    pub adapter_receipt_refs: Vec<crate::node_runtime::NodeAdapterReceiptRef>,
}

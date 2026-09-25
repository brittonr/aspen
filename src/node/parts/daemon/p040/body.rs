
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
    topology_profile_ref: Option<&'a str>,
    transport_profile_ref: Option<&'a str>,
    effective_max_attempts: Option<u64>,
    effective_join_timeout_ms: Option<u64>,
    diagnostics: &'a [String],
}

#[derive(Debug, Clone, Copy)]
struct LiveSendReceiptValueInput<'a> {
    decision: &'a str,
    from_peer: &'a str,
    ticket: &'a ControlLiveTicket,
    envelope: &'a ControlIngressEnvelope,
    transport_receipt_ref: Option<&'a str>,
    topology_profile_ref: Option<&'a str>,
    transport_profile_ref: Option<&'a str>,
    effective_max_attempts: u64,
    effective_join_timeout_ms: u64,
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

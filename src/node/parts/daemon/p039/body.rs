
#[derive(Debug, Clone, Copy)]
pub struct ControlLiveIngressReceiveBytesInput<'a> {
    pub state_root: &'a Path,
    pub topic: &'a str,
    pub receiver_node: &'a str,
    pub delivered_from: &'a str,
    pub bytes: &'a [u8],
}

#[derive(Debug, Clone, Copy)]
pub struct ControlLiveLoopbackInput<'a> {
    pub state_root: &'a Path,
    pub request_value: &'a IoValue,
    pub from_peer: &'a str,
    pub to_node: &'a str,
    pub topic: &'a str,
    pub sequence: u64,
    pub peer_bootstrap_refs: &'a [String],
    pub authority_refs: &'a [String],
    pub policy_refs: &'a [String],
    pub resource_refs: &'a [String],
    pub evidence_refs: &'a [String],
}

#[derive(Debug, Clone, Copy)]
pub struct ControlLiveServeInput<'a> {
    pub state_root: &'a Path,
    pub topic: &'a str,
    pub max_events: u64,
    pub event_timeout_ms: u64,
    pub max_requests_per_tick: u64,
    pub supervisor_policy_value: Option<&'a IoValue>,
}

#[derive(Debug, Clone, Copy)]
pub struct ControlLiveServeLoopbackInput<'a> {
    pub state_root: &'a Path,
    pub request_value: &'a IoValue,
    pub from_peer: &'a str,
    pub to_node: &'a str,
    pub topic: &'a str,
    pub sequence: u64,
    pub peer_bootstrap_refs: &'a [String],
    pub authority_refs: &'a [String],
    pub policy_refs: &'a [String],
    pub resource_refs: &'a [String],
    pub evidence_refs: &'a [String],
    pub max_requests_per_tick: u64,
}

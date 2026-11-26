//! Public inputs for local lifecycle, control, ingress, and live-serve requests.

#[derive(Debug, Clone, Copy)]
pub struct InitInput<'a> {
    pub state_root: &'a std::path::Path,
    pub node_id: &'a str,
}

#[derive(Debug, Clone, Copy)]
pub struct ProfileInitInput<'a> {
    pub state_root: &'a std::path::Path,
    pub node_id: &'a str,
    pub profile: &'a crate::node_profile_config::Checked,
    pub overrides: &'a crate::node_profile_config::Overrides,
}

#[derive(Debug, Clone, Copy)]
pub struct RunInput<'a> {
    pub state_root: &'a std::path::Path,
}

#[derive(Debug, Clone, Copy)]
pub struct StatusInput<'a> {
    pub state_root: &'a std::path::Path,
}

#[derive(Debug, Clone, Copy)]
pub struct StopInput<'a> {
    pub state_root: &'a std::path::Path,
}

#[derive(Debug, Clone, Copy)]
pub struct ControlSubmitInput<'a> {
    pub state_root: &'a std::path::Path,
    pub request_value: &'a preserves::IOValue,
}

#[derive(Debug, Clone, Copy)]
pub struct ControlDispatchInput<'a> {
    pub state_root: &'a std::path::Path,
    pub request_path: Option<&'a std::path::Path>,
}

#[derive(Debug, Clone, Copy)]
pub struct ControlDispatchEntryInput<'a> {
    pub state_root: &'a std::path::Path,
    pub request_entry: Option<&'a str>,
}

#[derive(Debug, Clone, Copy)]
pub struct ControlLoopInput<'a> {
    pub state_root: &'a std::path::Path,
    pub max_requests: u64,
}

#[derive(Debug, Clone, Copy)]
pub struct ControlServeInput<'a> {
    pub state_root: &'a std::path::Path,
    pub topic: &'a str,
    pub max_ticks: u64,
    pub max_requests_per_tick: u64,
    pub supervisor_policy_value: Option<&'a preserves::IOValue>,
}

#[derive(Debug, Clone, Copy)]
pub struct ControlSupervisorPolicyInput<'a> {
    pub max_restarts: u64,
    pub restart_window_ticks: u64,
    pub heartbeat_timeout_ticks: u64,
    pub shutdown_drain_ticks: u64,
    pub stale_lock_recovery: bool,
    pub policy_refs: &'a [String],
    pub evidence_refs: &'a [String],
}

#[derive(Debug, Clone, Copy)]
pub struct ControlIngressEnvelopeInput<'a> {
    pub request_value: &'a preserves::IOValue,
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
pub struct ControlIngressPublishInput<'a> {
    pub state_root: &'a std::path::Path,
    pub envelope_value: &'a preserves::IOValue,
}

#[derive(Debug, Clone, Copy)]
pub struct ControlIngressDeliverInput<'a> {
    pub state_root: &'a std::path::Path,
    pub topic: &'a str,
    pub envelope_ref: &'a str,
}

#[derive(Debug, Clone, Copy)]
pub struct ControlLiveIngressPublishInput<'a> {
    pub sender: &'a iroh_gossip::api::GossipSender,
    pub envelope_value: &'a preserves::IOValue,
    pub node_id: &'a str,
    pub topology_profile_ref: Option<&'a str>,
    pub transport_profile_ref: Option<&'a str>,
    pub effective_max_attempts: Option<u64>,
    pub effective_join_timeout_ms: Option<u64>,
}

#[derive(Debug, Clone, Copy)]
pub struct ControlLiveIngressReceiveBytesInput<'a> {
    pub state_root: &'a std::path::Path,
    pub topic: &'a str,
    pub receiver_node: &'a str,
    pub delivered_from: &'a str,
    pub bytes: &'a [u8],
}

#[derive(Debug, Clone, Copy)]
pub struct ControlLiveLoopbackInput<'a> {
    pub state_root: &'a std::path::Path,
    pub request_value: &'a preserves::IOValue,
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
    pub state_root: &'a std::path::Path,
    pub topic: &'a str,
    pub max_events: u64,
    pub event_timeout_ms: u64,
    pub max_requests_per_tick: u64,
    pub supervisor_policy_value: Option<&'a preserves::IOValue>,
}

#[derive(Debug, Clone, Copy)]
pub struct ControlLiveServeLoopbackInput<'a> {
    pub state_root: &'a std::path::Path,
    pub request_value: &'a preserves::IOValue,
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

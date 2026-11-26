//! Public inputs for live sends, transport profiles, tickets, grants, and peer admission.

#[derive(Debug, Clone, Copy)]
pub struct LiveTopologyProfile<'a> {
    pub profile_ref: &'a str,
    pub expected_node: &'a str,
    pub expected_peer: &'a str,
    pub expected_topic: &'a str,
    pub expected_endpoint: Option<&'a str>,
    pub allowed_alpns: &'a [&'a str],
    pub ticket_refs: &'a [String],
    pub peer_admission_refs: &'a [String],
    pub role: Option<&'a str>,
}

#[derive(Debug, Clone, Copy)]
pub struct LiveTransportProfile<'a> {
    pub profile_ref: &'a str,
    pub max_attempts: u64,
    pub join_timeout_ms: u64,
    pub publish_timeout_ms: u64,
    pub relay_preference: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiveProfilePreflight {
    pub decision: String,
    pub topology_profile_ref: Option<String>,
    pub transport_profile_ref: Option<String>,
    pub effective_max_attempts: u64,
    pub effective_join_timeout_ms: u64,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct LiveProfilePreflightInput<'a> {
    pub(super) send: &'a ControlLiveSendInput<'a>,
    pub(super) ticket: &'a super::entitlements::ControlLiveTicket,
    pub(super) envelope: &'a super::transits::ControlIngressEnvelope,
}

#[derive(Debug, Clone, Copy)]
pub struct ControlLiveSendInput<'a> {
    pub state_root: Option<&'a std::path::Path>,
    pub request_value: &'a preserves::IOValue,
    pub receiver_ticket_value: &'a preserves::IOValue,
    pub from_peer: &'a str,
    pub sequence: u64,
    pub expected_operation_ref: Option<&'a str>,
    pub expected_receiver_node: Option<&'a str>,
    pub expected_topic: Option<&'a str>,
    pub expected_endpoint: Option<&'a str>,
    pub topology_profile: Option<&'a LiveTopologyProfile<'a>>,
    pub transport_profile: Option<&'a LiveTransportProfile<'a>>,
    pub max_attempts: u64,
    pub peer_bootstrap_refs: &'a [String],
    pub authority_refs: &'a [String],
    pub policy_refs: &'a [String],
    pub resource_refs: &'a [String],
    pub evidence_refs: &'a [String],
    pub join_timeout_ms: u64,
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
    pub state_root: &'a std::path::Path,
    pub topic: &'a str,
    pub policy_refs: &'a [String],
    pub evidence_refs: &'a [String],
}

#[derive(Debug, Clone, Copy)]
pub struct ControlLiveTicketImportInput<'a> {
    pub state_root: &'a std::path::Path,
    pub ticket_value: &'a preserves::IOValue,
    pub peer_admission_value: Option<&'a preserves::IOValue>,
    pub expected_node: Option<&'a str>,
    pub expected_topic: Option<&'a str>,
    pub expected_endpoint: Option<&'a str>,
    pub expected_peer: Option<&'a str>,
    pub as_of_sequence: u64,
}

#[derive(Debug, Clone, Copy)]
pub struct ControlAuthorityGrantImportInput<'a> {
    pub state_root: &'a std::path::Path,
    pub grant_value: &'a preserves::IOValue,
    pub expected_peer: Option<&'a str>,
    pub expected_node: Option<&'a str>,
    pub expected_operations: &'a [String],
    pub expected_target_scope: Option<&'a str>,
    pub expected_resource_scope: Option<&'a str>,
    pub as_of_epoch: u64,
}

#[derive(Debug, Clone, Copy)]
pub struct ControlLivePeerAdmitInput<'a> {
    pub state_root: &'a std::path::Path,
    pub ticket_value: &'a preserves::IOValue,
    pub peer_id: &'a str,
    pub sequence: u64,
    pub expires_at: Option<u64>,
    pub policy_refs: &'a [String],
    pub evidence_refs: &'a [String],
}

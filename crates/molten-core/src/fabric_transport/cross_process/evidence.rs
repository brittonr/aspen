use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DistinctProcessParticipantEvidence {
    pub role: EndpointParticipantRole,
    pub invocation_ref: String,
    pub parent_start_ref: String,
    pub terminal_ref: String,
    pub cleanup_ref: String,
    pub descriptor_ref: String,
    pub profile_id: String,
    pub protocol_id: String,
    pub alpn: String,
    pub service_id: String,
    pub generation: u64,
    pub request_ref: String,
    pub payload_ref: String,
    pub acknowledgement_ref: String,
    pub parent_observed_start: bool,
    pub parent_observed_terminal: bool,
    pub parent_observed_exit: bool,
    pub automatic_retry_count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DistinctProcessTransportEvidenceInput {
    pub listener: DistinctProcessParticipantEvidence,
    pub client: DistinctProcessParticipantEvidence,
    pub handoff_ref: String,
    pub child_handles_distinct: bool,
    pub handoff_observed_before_client_start: bool,
    pub cleanup_succeeded: bool,
    pub same_process_loopback: bool,
    pub child_only_separation_claim: bool,
    pub default_readback_redacted: bool,
    pub payloads_excluded: bool,
    pub accepted_sessions: u64,
    pub max_sessions: u64,
    pub exchanged_bytes: u64,
    pub max_frame_bytes: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DistinctProcessEvidenceIssue {
    MalformedContentRef(&'static str),
    ParticipantRoleMismatch,
    DuplicateInvocation,
    DescriptorMismatch,
    ProfileMismatch,
    ProtocolMismatch,
    AlpnMismatch,
    ServiceMismatch,
    GenerationMismatch,
    RequestMismatch,
    PayloadMismatch,
    AcknowledgementMismatch,
    ParentStartMissing,
    ParentTerminalMissing,
    ParentExitMissing,
    ChildHandlesNotDistinct,
    HandoffOrderInvalid,
    CleanupFailed,
    SameProcessLoopbackInsufficient,
    ChildOnlyClaimInsufficient,
    DefaultReadbackLeaksLocators,
    PayloadEvidenceLeak,
    AutomaticRetryObserved,
    SessionResourceExceeded,
    FrameResourceExceeded,
    ZeroResourceBound(&'static str),
}

impl DistinctProcessEvidenceIssue {
    pub const fn code(self) -> &'static str {
        match self {
            Self::MalformedContentRef(_) => "malformed-content-ref",
            Self::ParticipantRoleMismatch => "participant-role-mismatch",
            Self::DuplicateInvocation => "duplicate-invocation",
            Self::DescriptorMismatch => "descriptor-mismatch",
            Self::ProfileMismatch => "profile-mismatch",
            Self::ProtocolMismatch => "protocol-mismatch",
            Self::AlpnMismatch => "alpn-mismatch",
            Self::ServiceMismatch => "service-mismatch",
            Self::GenerationMismatch => "generation-mismatch",
            Self::RequestMismatch => "request-mismatch",
            Self::PayloadMismatch => "payload-mismatch",
            Self::AcknowledgementMismatch => "acknowledgement-mismatch",
            Self::ParentStartMissing => "parent-start-missing",
            Self::ParentTerminalMissing => "parent-terminal-missing",
            Self::ParentExitMissing => "parent-exit-missing",
            Self::ChildHandlesNotDistinct => "child-handles-not-distinct",
            Self::HandoffOrderInvalid => "handoff-order-invalid",
            Self::CleanupFailed => "cleanup-failed",
            Self::SameProcessLoopbackInsufficient => "same-process-loopback-insufficient",
            Self::ChildOnlyClaimInsufficient => "child-only-claim-insufficient",
            Self::DefaultReadbackLeaksLocators => "default-readback-leaks-locators",
            Self::PayloadEvidenceLeak => "payload-evidence-leak",
            Self::AutomaticRetryObserved => "automatic-retry-observed",
            Self::SessionResourceExceeded => "session-resource-exceeded",
            Self::FrameResourceExceeded => "frame-resource-exceeded",
            Self::ZeroResourceBound(_) => "zero-resource-bound",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DistinctProcessTransportAssessment {
    pub admitted: bool,
    pub issues: Vec<DistinctProcessEvidenceIssue>,
}

// r[impl molten.fabric_transport.distinct_process_evidence]
// r[impl molten.fabric_transport.cross_process_validation]
pub fn assess_distinct_process_transport_evidence(
    input: &DistinctProcessTransportEvidenceInput,
) -> DistinctProcessTransportAssessment {
    let mut issues = Vec::new();
    validate_participant_refs(&input.listener, "listener", &mut issues);
    validate_participant_refs(&input.client, "client", &mut issues);
    validate_ref_for_evidence("handoff", &input.handoff_ref, &mut issues);
    if input.listener.role != EndpointParticipantRole::Listener || input.client.role != EndpointParticipantRole::Client
    {
        issues.push(DistinctProcessEvidenceIssue::ParticipantRoleMismatch);
    }
    if input.listener.invocation_ref == input.client.invocation_ref {
        issues.push(DistinctProcessEvidenceIssue::DuplicateInvocation);
    }
    compare_participant_bindings(&input.listener, &input.client, &mut issues);
    if !input.listener.parent_observed_start || !input.client.parent_observed_start {
        issues.push(DistinctProcessEvidenceIssue::ParentStartMissing);
    }
    if !input.listener.parent_observed_terminal || !input.client.parent_observed_terminal {
        issues.push(DistinctProcessEvidenceIssue::ParentTerminalMissing);
    }
    if !input.listener.parent_observed_exit || !input.client.parent_observed_exit {
        issues.push(DistinctProcessEvidenceIssue::ParentExitMissing);
    }
    if !input.child_handles_distinct {
        issues.push(DistinctProcessEvidenceIssue::ChildHandlesNotDistinct);
    }
    if !input.handoff_observed_before_client_start {
        issues.push(DistinctProcessEvidenceIssue::HandoffOrderInvalid);
    }
    if !input.cleanup_succeeded {
        issues.push(DistinctProcessEvidenceIssue::CleanupFailed);
    }
    if input.same_process_loopback {
        issues.push(DistinctProcessEvidenceIssue::SameProcessLoopbackInsufficient);
    }
    if input.child_only_separation_claim {
        issues.push(DistinctProcessEvidenceIssue::ChildOnlyClaimInsufficient);
    }
    if !input.default_readback_redacted {
        issues.push(DistinctProcessEvidenceIssue::DefaultReadbackLeaksLocators);
    }
    if !input.payloads_excluded {
        issues.push(DistinctProcessEvidenceIssue::PayloadEvidenceLeak);
    }
    if input.listener.automatic_retry_count != 0 || input.client.automatic_retry_count != 0 {
        issues.push(DistinctProcessEvidenceIssue::AutomaticRetryObserved);
    }
    if input.max_sessions == 0 {
        issues.push(DistinctProcessEvidenceIssue::ZeroResourceBound("max-sessions"));
    } else if input.accepted_sessions > input.max_sessions {
        issues.push(DistinctProcessEvidenceIssue::SessionResourceExceeded);
    }
    if input.max_frame_bytes == 0 {
        issues.push(DistinctProcessEvidenceIssue::ZeroResourceBound("max-frame-bytes"));
    } else if input.exchanged_bytes == 0 || input.exchanged_bytes > input.max_frame_bytes {
        issues.push(DistinctProcessEvidenceIssue::FrameResourceExceeded);
    }
    DistinctProcessTransportAssessment {
        admitted: issues.is_empty(),
        issues,
    }
}

fn validate_participant_refs(
    participant: &DistinctProcessParticipantEvidence,
    role_label: &'static str,
    issues: &mut Vec<DistinctProcessEvidenceIssue>,
) {
    for (field, reference) in [
        (role_label, participant.invocation_ref.as_str()),
        ("parent-start", participant.parent_start_ref.as_str()),
        ("terminal", participant.terminal_ref.as_str()),
        ("cleanup", participant.cleanup_ref.as_str()),
        ("descriptor", participant.descriptor_ref.as_str()),
        ("request", participant.request_ref.as_str()),
        ("payload", participant.payload_ref.as_str()),
        ("acknowledgement", participant.acknowledgement_ref.as_str()),
    ] {
        validate_ref_for_evidence(field, reference, issues);
    }
}

fn validate_ref_for_evidence(field: &'static str, reference: &str, issues: &mut Vec<DistinctProcessEvidenceIssue>) {
    if !crate::fabric::valid_blake3_ref(reference) {
        issues.push(DistinctProcessEvidenceIssue::MalformedContentRef(field));
    }
}

fn compare_participant_bindings(
    listener: &DistinctProcessParticipantEvidence,
    client: &DistinctProcessParticipantEvidence,
    issues: &mut Vec<DistinctProcessEvidenceIssue>,
) {
    if listener.descriptor_ref != client.descriptor_ref {
        issues.push(DistinctProcessEvidenceIssue::DescriptorMismatch);
    }
    if listener.profile_id != client.profile_id {
        issues.push(DistinctProcessEvidenceIssue::ProfileMismatch);
    }
    if listener.protocol_id != client.protocol_id {
        issues.push(DistinctProcessEvidenceIssue::ProtocolMismatch);
    }
    if listener.alpn != client.alpn {
        issues.push(DistinctProcessEvidenceIssue::AlpnMismatch);
    }
    if listener.service_id != client.service_id {
        issues.push(DistinctProcessEvidenceIssue::ServiceMismatch);
    }
    if listener.generation != client.generation {
        issues.push(DistinctProcessEvidenceIssue::GenerationMismatch);
    }
    if listener.request_ref != client.request_ref {
        issues.push(DistinctProcessEvidenceIssue::RequestMismatch);
    }
    if listener.payload_ref != client.payload_ref {
        issues.push(DistinctProcessEvidenceIssue::PayloadMismatch);
    }
    if listener.acknowledgement_ref != client.acknowledgement_ref {
        issues.push(DistinctProcessEvidenceIssue::AcknowledgementMismatch);
    }
}

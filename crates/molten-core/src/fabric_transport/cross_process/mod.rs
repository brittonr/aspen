//! Pure cross-process endpoint, listener, and session admission.
//!
//! This module owns typed connectivity artifacts and deterministic transition
//! decisions. It performs no network, clock, filesystem, process, or executor
//! operations.

use super::*;

mod endpoint;
mod evidence;
mod listener;
mod session;

pub use endpoint::*;
pub use evidence::*;
pub use listener::*;
pub use session::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrossProcessTransportIssue {
    InvalidTransportProfile,
    InvalidProtocolDescriptor,
    EndpointSchemaMismatch,
    EmptyField(&'static str),
    MalformedField(&'static str),
    MalformedContentRef(&'static str),
    ProfileAdapterMismatch,
    ProfileIdentityMismatch,
    ProtocolIdentityMismatch,
    AlpnMismatch,
    ExtensionIdentityMismatch,
    ServiceIdentityMismatch,
    GenerationMismatch,
    GenerationDidNotAdvance,
    EndpointIdentityMismatch,
    ListenerIdentityMismatch,
    PeerContextMismatch,
    ParticipantRoleMismatch,
    FramingProfileMismatch,
    InvalidValidityCohort,
    ValidityCohortNotStarted,
    ValidityCohortExpired,
    MissingLocator,
    LocatorLimitExceeded,
    DuplicateLocator,
    DuplicateLocatorClass,
    UnsupportedLocatorClass,
    UndeclaredLocatorClass,
    PrivateLocatorDisclosure,
    LocatorClassMismatch,
    SecretBearingLocator,
    RawHandleLocator,
    DefaultReadbackNotRedacted,
    InvalidResourceBound(&'static str),
    ResourceBoundExceeded(&'static str),
    MissingNonClaim(TransportNonClaim),
    RegistrationRevoked,
    TransportCapabilityRevoked,
    ProtocolCapabilityRevoked,
    ProfileRevoked,
    ListenerNotReady,
    ListenerLimitExceeded,
    DuplicateListener,
    InvalidListenerTransition {
        from: CrossProcessListenerPhase,
        command: CrossProcessListenerCommandKind,
    },
    ListenerReadinessIncomplete,
    ListenerSessionLimitExceeded,
    ListenerHasActiveSessions,
    ListenerHasNoActiveSessions,
    StaleListenerCallback,
    CleanupEvidenceRequired,
    InvalidSessionTransition {
        from: CrossProcessSessionPhase,
        command: CrossProcessSessionCommandKind,
    },
    StaleSessionCallback,
    SessionFrameLimitExceeded,
    SessionQueueLimitExceeded,
    SessionInflightLimitExceeded,
    SessionAccountingMismatch,
    SessionWorkRemains,
    CounterOverflow,
}

impl CrossProcessTransportIssue {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidTransportProfile => "invalid-transport-profile",
            Self::InvalidProtocolDescriptor => "invalid-protocol-descriptor",
            Self::EndpointSchemaMismatch => "endpoint-schema-mismatch",
            Self::EmptyField(_) => "empty-field",
            Self::MalformedField(_) => "malformed-field",
            Self::MalformedContentRef(_) => "malformed-content-ref",
            Self::ProfileAdapterMismatch => "profile-adapter-mismatch",
            Self::ProfileIdentityMismatch => "profile-identity-mismatch",
            Self::ProtocolIdentityMismatch => "protocol-identity-mismatch",
            Self::AlpnMismatch => "alpn-mismatch",
            Self::ExtensionIdentityMismatch => "extension-identity-mismatch",
            Self::ServiceIdentityMismatch => "service-identity-mismatch",
            Self::GenerationMismatch => "generation-mismatch",
            Self::GenerationDidNotAdvance => "generation-did-not-advance",
            Self::EndpointIdentityMismatch => "endpoint-identity-mismatch",
            Self::ListenerIdentityMismatch => "listener-identity-mismatch",
            Self::PeerContextMismatch => "peer-context-mismatch",
            Self::ParticipantRoleMismatch => "participant-role-mismatch",
            Self::FramingProfileMismatch => "framing-profile-mismatch",
            Self::InvalidValidityCohort => "invalid-validity-cohort",
            Self::ValidityCohortNotStarted => "validity-cohort-not-started",
            Self::ValidityCohortExpired => "validity-cohort-expired",
            Self::MissingLocator => "missing-locator",
            Self::LocatorLimitExceeded => "locator-limit-exceeded",
            Self::DuplicateLocator => "duplicate-locator",
            Self::DuplicateLocatorClass => "duplicate-locator-class",
            Self::UnsupportedLocatorClass => "unsupported-locator-class",
            Self::UndeclaredLocatorClass => "undeclared-locator-class",
            Self::PrivateLocatorDisclosure => "private-locator-disclosure",
            Self::LocatorClassMismatch => "locator-class-mismatch",
            Self::SecretBearingLocator => "secret-bearing-locator",
            Self::RawHandleLocator => "raw-handle-locator",
            Self::DefaultReadbackNotRedacted => "default-readback-not-redacted",
            Self::InvalidResourceBound(_) => "invalid-resource-bound",
            Self::ResourceBoundExceeded(_) => "resource-bound-exceeded",
            Self::MissingNonClaim(_) => "missing-non-claim",
            Self::RegistrationRevoked => "registration-revoked",
            Self::TransportCapabilityRevoked => "transport-capability-revoked",
            Self::ProtocolCapabilityRevoked => "protocol-capability-revoked",
            Self::ProfileRevoked => "profile-revoked",
            Self::ListenerNotReady => "listener-not-ready",
            Self::ListenerLimitExceeded => "listener-limit-exceeded",
            Self::DuplicateListener => "duplicate-listener",
            Self::InvalidListenerTransition { .. } => "invalid-listener-transition",
            Self::ListenerReadinessIncomplete => "listener-readiness-incomplete",
            Self::ListenerSessionLimitExceeded => "listener-session-limit-exceeded",
            Self::ListenerHasActiveSessions => "listener-has-active-sessions",
            Self::ListenerHasNoActiveSessions => "listener-has-no-active-sessions",
            Self::StaleListenerCallback => "stale-listener-callback",
            Self::CleanupEvidenceRequired => "cleanup-evidence-required",
            Self::InvalidSessionTransition { .. } => "invalid-session-transition",
            Self::StaleSessionCallback => "stale-session-callback",
            Self::SessionFrameLimitExceeded => "session-frame-limit-exceeded",
            Self::SessionQueueLimitExceeded => "session-queue-limit-exceeded",
            Self::SessionInflightLimitExceeded => "session-inflight-limit-exceeded",
            Self::SessionAccountingMismatch => "session-accounting-mismatch",
            Self::SessionWorkRemains => "session-work-remains",
            Self::CounterOverflow => "counter-overflow",
        }
    }
}

fn validate_token(field: &'static str, value: &str, issues: &mut Vec<CrossProcessTransportIssue>) {
    if value.is_empty() {
        issues.push(CrossProcessTransportIssue::EmptyField(field));
    } else if value.len() > MAX_TRANSPORT_TEXT_BYTES || !crate::fabric::valid_fabric_token(value) {
        issues.push(CrossProcessTransportIssue::MalformedField(field));
    }
}

fn validate_alpn(value: &str, issues: &mut Vec<CrossProcessTransportIssue>) {
    if value.is_empty() {
        issues.push(CrossProcessTransportIssue::EmptyField("alpn"));
    } else if value.len() > MAX_TRANSPORT_TEXT_BYTES
        || !value.bytes().all(|byte| byte.is_ascii_graphic() && byte != b' ')
    {
        issues.push(CrossProcessTransportIssue::MalformedField("alpn"));
    }
}

fn validate_ref(field: &'static str, value: &str, issues: &mut Vec<CrossProcessTransportIssue>) {
    if !crate::fabric::valid_blake3_ref(value) {
        issues.push(CrossProcessTransportIssue::MalformedContentRef(field));
    }
}

fn checked_increment(value: u64) -> Result<u64, CrossProcessTransportIssue> {
    value.checked_add(1).ok_or(CrossProcessTransportIssue::CounterOverflow)
}

fn checked_add(left: u64, right: u64) -> Result<u64, CrossProcessTransportIssue> {
    left.checked_add(right).ok_or(CrossProcessTransportIssue::CounterOverflow)
}

#[cfg(test)]
mod tests;

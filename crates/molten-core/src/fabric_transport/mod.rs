//! Pure transport-port contracts and deterministic session transition laws.
//!
//! This module owns no sockets, executors, clocks, randomness, or simulator
//! runtime. Adapter shells submit explicit commands and observed adapter facts.

mod transition;

use std::collections::BTreeMap;

pub use transition::*;

use crate::fabric::valid_blake3_ref;
use crate::fabric::valid_fabric_token;

pub const TRANSPORT_PROFILE_SCHEMA: &str = "molten.fabric.transport.profile.v1";
pub const TRANSPORT_PROTOCOL_SCHEMA: &str = "molten.fabric.transport.protocol.v1";
pub const TRANSPORT_COMMAND_SCHEMA: &str = "molten.fabric.transport.command.v1";
pub const TRANSPORT_EVENT_SCHEMA: &str = "molten.fabric.transport.event.v1";
pub const TRANSPORT_STATUS_SCHEMA: &str = "molten.fabric.transport.status.v1";

pub const MAX_TRANSPORT_TEXT_BYTES: usize = 256;
pub const MAX_TRANSPORT_COLLECTION_ITEMS: usize = 4_096;
const ADJACENT_PAIR_WIDTH: usize = 2;
const REQUIRED_NON_CLAIM_COUNT: usize = 9;

pub const REQUIRED_TRANSPORT_NON_CLAIMS: [TransportNonClaim; REQUIRED_NON_CLAIM_COUNT] = [
    TransportNonClaim::NoDurableDelivery,
    TransportNonClaim::NoExactlyOnce,
    TransportNonClaim::NoTransactionalMessaging,
    TransportNonClaim::NoGlobalOrdering,
    TransportNonClaim::NoAutomaticRetry,
    TransportNonClaim::NoMembership,
    TransportNonClaim::NoConsensus,
    TransportNonClaim::NoProtocolCompatibility,
    TransportNonClaim::NoApplicationAuthority,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TransportNonClaim {
    NoDurableDelivery,
    NoExactlyOnce,
    NoTransactionalMessaging,
    NoGlobalOrdering,
    NoAutomaticRetry,
    NoMembership,
    NoConsensus,
    NoProtocolCompatibility,
    NoApplicationAuthority,
}

impl TransportNonClaim {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NoDurableDelivery => "does-not-prove-durable-delivery",
            Self::NoExactlyOnce => "does-not-prove-exactly-once-delivery",
            Self::NoTransactionalMessaging => "does-not-prove-transactional-messaging",
            Self::NoGlobalOrdering => "does-not-prove-global-ordering",
            Self::NoAutomaticRetry => "does-not-provide-automatic-retry-safety",
            Self::NoMembership => "does-not-grant-membership",
            Self::NoConsensus => "does-not-prove-consensus",
            Self::NoProtocolCompatibility => "does-not-prove-protocol-compatibility",
            Self::NoApplicationAuthority => "does-not-grant-application-authority",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TransportAdapterKind {
    IrohLive,
    DeterministicSimulation,
}

impl TransportAdapterKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::IrohLive => "iroh-live",
            Self::DeterministicSimulation => "deterministic-simulation",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TransportCapability {
    BidirectionalStreams,
    UnidirectionalStreams,
    Datagrams,
}

impl TransportCapability {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::BidirectionalStreams => "bidirectional-streams",
            Self::UnidirectionalStreams => "unidirectional-streams",
            Self::Datagrams => "datagrams",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransportLimits {
    pub max_listeners: u64,
    pub max_sessions: u64,
    pub max_streams_per_session: u64,
    pub max_frame_bytes: u64,
    pub max_datagram_bytes: u64,
    pub max_queued_events: u64,
    pub max_queued_bytes: u64,
    pub max_inflight_bytes: u64,
    pub operation_deadline_ticks: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransportProfile {
    pub schema: String,
    pub profile_id: String,
    pub profile_ref: String,
    pub adapter_kind: TransportAdapterKind,
    pub capabilities: Vec<TransportCapability>,
    pub limits: TransportLimits,
    pub non_claims: Vec<TransportNonClaim>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FramingProfile {
    pub profile_id: String,
    pub profile_ref: String,
    pub max_frame_bytes: u64,
    pub length_prefix_bytes: u64,
    pub payload_hash_required: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListenerCleanupPolicy {
    Immediate,
    BoundedDrain { grace_ticks: u64 },
}

impl ListenerCleanupPolicy {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Immediate => "immediate",
            Self::BoundedDrain { .. } => "bounded-drain",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolDescriptor {
    pub schema: String,
    pub protocol_id: String,
    pub version: String,
    pub alpn: String,
    pub extension_id: String,
    pub service_id: String,
    pub generation: u64,
    pub listener_limit: u64,
    pub requested_capabilities: Vec<TransportCapability>,
    pub framing: FramingProfile,
    pub cleanup_policy: ListenerCleanupPolicy,
    pub registration_authority_ref: String,
    pub profile_ref: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtocolRegistrationPhase {
    Active,
    Draining,
}

impl ProtocolRegistrationPhase {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Draining => "draining",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegisteredProtocol {
    pub descriptor: ProtocolDescriptor,
    pub phase: ProtocolRegistrationPhase,
    pub latest_evidence_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopedTransportId {
    pub opaque_ref: String,
    pub service_id: String,
    pub generation: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeerIdentityRefs {
    pub transport_identity_ref: String,
    pub membership_ref: Option<String>,
    pub application_principal_ref: Option<String>,
    pub trust_decision_ref: Option<String>,
    pub capability_authority_ref: Option<String>,
    pub bootstrap_policy_ref: Option<String>,
}

impl PeerIdentityRefs {
    pub fn has_service_authority(&self) -> bool {
        let normal_admission = self.membership_ref.is_some()
            && self.application_principal_ref.is_some()
            && self.trust_decision_ref.is_some()
            && self.capability_authority_ref.is_some();
        normal_admission || self.bootstrap_policy_ref.is_some()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionDirection {
    Outbound,
    Inbound,
}

impl SessionDirection {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Outbound => "outbound",
            Self::Inbound => "inbound",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionPhase {
    Active,
    Draining,
    Closed,
    Cancelled,
    Failed,
}

impl SessionPhase {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Draining => "draining",
            Self::Closed => "closed",
            Self::Cancelled => "cancelled",
            Self::Failed => "failed",
        }
    }

    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Closed | Self::Cancelled | Self::Failed)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamDirection {
    Bidirectional,
    SendOnly,
    ReceiveOnly,
}

impl StreamDirection {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Bidirectional => "bidirectional",
            Self::SendOnly => "send-only",
            Self::ReceiveOnly => "receive-only",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamPhase {
    Open,
    SendHalfClosed,
    ReceiveHalfClosed,
    Closed,
    Reset,
    Cancelled,
}

impl StreamPhase {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::SendHalfClosed => "send-half-closed",
            Self::ReceiveHalfClosed => "receive-half-closed",
            Self::Closed => "closed",
            Self::Reset => "reset",
            Self::Cancelled => "cancelled",
        }
    }

    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Closed | Self::Reset | Self::Cancelled)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransportStream {
    pub id: ScopedTransportId,
    pub direction: StreamDirection,
    pub phase: StreamPhase,
    pub send_credit_bytes: u64,
    pub inflight_bytes: u64,
    pub next_send_sequence: u64,
    pub next_receive_sequence: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransportSession {
    pub id: ScopedTransportId,
    pub protocol_id: String,
    pub alpn: String,
    pub direction: SessionDirection,
    pub phase: SessionPhase,
    pub peer: PeerIdentityRefs,
    pub streams: BTreeMap<String, TransportStream>,
    pub queued_events: u64,
    pub queued_bytes: u64,
    pub inflight_bytes: u64,
    pub deadline_tick: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TransportCounters {
    pub registrations: u64,
    pub ownership_transfers: u64,
    pub sessions_opened: u64,
    pub streams_opened: u64,
    pub frames_submitted: u64,
    pub frames_received: u64,
    pub datagrams_submitted: u64,
    pub failures: u64,
    pub cancellations: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TransportState {
    pub protocols: BTreeMap<String, RegisteredProtocol>,
    pub sessions: BTreeMap<String, TransportSession>,
    pub counters: TransportCounters,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeliveryOutcome {
    NotAttempted,
    Pending,
    Delivered,
    NotDelivered,
    Uncertain,
}

impl DeliveryOutcome {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NotAttempted => "not-attempted",
            Self::Pending => "pending",
            Self::Delivered => "delivered",
            Self::NotDelivered => "not-delivered",
            Self::Uncertain => "uncertain",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetryDisposition {
    NotApplicable,
    HigherLevelPolicyRequired,
    UnsafeWithoutReconciliation,
}

impl RetryDisposition {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NotApplicable => "not-applicable",
            Self::HigherLevelPolicyRequired => "higher-level-policy-required",
            Self::UnsafeWithoutReconciliation => "unsafe-without-reconciliation",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportFailureClass {
    LocalRefusal,
    RemoteRefusal,
    Disconnect,
    Reset,
    Timeout,
    Partition,
    MalformedInput,
    Overload,
    Cancellation,
    AdapterFailure,
}

impl TransportFailureClass {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::LocalRefusal => "local-refusal",
            Self::RemoteRefusal => "remote-refusal",
            Self::Disconnect => "disconnect",
            Self::Reset => "reset",
            Self::Timeout => "timeout",
            Self::Partition => "partition",
            Self::MalformedInput => "malformed-input",
            Self::Overload => "overload",
            Self::Cancellation => "cancellation",
            Self::AdapterFailure => "adapter-failure",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportEventKind {
    ProtocolRegistered,
    ProtocolOwnershipTransferred,
    ListenerDraining,
    ListenerCleaned,
    SessionEstablished,
    StreamOpened,
    FrameSubmitted,
    FrameReceived,
    FrameAcknowledged,
    DatagramSubmitted,
    DatagramCompleted,
    CreditGranted,
    Backpressured,
    StreamHalfClosed,
    StreamClosed,
    SessionClosed,
    Cancelled,
    Failed,
}

impl TransportEventKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ProtocolRegistered => "protocol-registered",
            Self::ProtocolOwnershipTransferred => "protocol-ownership-transferred",
            Self::ListenerDraining => "listener-draining",
            Self::ListenerCleaned => "listener-cleaned",
            Self::SessionEstablished => "session-established",
            Self::StreamOpened => "stream-opened",
            Self::FrameSubmitted => "frame-submitted",
            Self::FrameReceived => "frame-received",
            Self::FrameAcknowledged => "frame-acknowledged",
            Self::DatagramSubmitted => "datagram-submitted",
            Self::DatagramCompleted => "datagram-completed",
            Self::CreditGranted => "credit-granted",
            Self::Backpressured => "backpressured",
            Self::StreamHalfClosed => "stream-half-closed",
            Self::StreamClosed => "stream-closed",
            Self::SessionClosed => "session-closed",
            Self::Cancelled => "cancelled",
            Self::Failed => "failed",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransportEvent {
    pub kind: TransportEventKind,
    pub operation_id: String,
    pub protocol_id: String,
    pub session_id: Option<String>,
    pub stream_id: Option<String>,
    pub generation: u64,
    pub sequence: Option<u64>,
    pub payload_ref: Option<String>,
    pub payload_bytes: u64,
    pub peer: Option<PeerIdentityRefs>,
    pub failure: Option<TransportFailureClass>,
    pub delivery: DeliveryOutcome,
    pub retry: RetryDisposition,
    pub terminal: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CancelTarget {
    Session(ScopedTransportId),
    Stream {
        session_id: ScopedTransportId,
        stream_id: ScopedTransportId,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransportCommand {
    Register {
        operation_id: String,
        descriptor: ProtocolDescriptor,
    },
    TransferOwnership {
        operation_id: String,
        descriptor: ProtocolDescriptor,
        prior_generation: u64,
        cleanup_evidence_ref: String,
    },
    BeginDrain {
        operation_id: String,
        alpn: String,
        service_id: String,
        generation: u64,
    },
    CleanupListener {
        operation_id: String,
        alpn: String,
        service_id: String,
        generation: u64,
        cleanup_evidence_ref: String,
    },
    OpenSession {
        operation_id: String,
        session_id: ScopedTransportId,
        alpn: String,
        direction: SessionDirection,
        peer: PeerIdentityRefs,
        observed_tick: u64,
        deadline_tick: u64,
    },
    OpenStream {
        operation_id: String,
        session_id: ScopedTransportId,
        stream_id: ScopedTransportId,
        direction: StreamDirection,
        initial_credit_bytes: u64,
    },
    SendFrame {
        operation_id: String,
        session_id: ScopedTransportId,
        stream_id: ScopedTransportId,
        payload_ref: String,
        payload_bytes: u64,
        observed_tick: u64,
    },
    ReceiveFrame {
        operation_id: String,
        session_id: ScopedTransportId,
        stream_id: ScopedTransportId,
        payload_ref: String,
        payload_bytes: u64,
        sequence: u64,
        observed_tick: u64,
    },
    AcknowledgeFrame {
        operation_id: String,
        session_id: ScopedTransportId,
        stream_id: ScopedTransportId,
        payload_bytes: u64,
    },
    SendDatagram {
        operation_id: String,
        session_id: ScopedTransportId,
        payload_ref: String,
        payload_bytes: u64,
        observed_tick: u64,
    },
    CompleteDatagram {
        operation_id: String,
        session_id: ScopedTransportId,
        payload_bytes: u64,
        delivered: bool,
    },
    GrantCredit {
        operation_id: String,
        session_id: ScopedTransportId,
        stream_id: ScopedTransportId,
        credit_bytes: u64,
    },
    HalfCloseStream {
        operation_id: String,
        session_id: ScopedTransportId,
        stream_id: ScopedTransportId,
        send_direction: bool,
    },
    CloseStream {
        operation_id: String,
        session_id: ScopedTransportId,
        stream_id: ScopedTransportId,
    },
    CloseSession {
        operation_id: String,
        session_id: ScopedTransportId,
    },
    Cancel {
        operation_id: String,
        target: CancelTarget,
    },
    FailSession {
        operation_id: String,
        session_id: ScopedTransportId,
        class: TransportFailureClass,
        delivery_definitive: bool,
    },
}

impl TransportCommand {
    pub fn generation(&self) -> u64 {
        match self {
            Self::Register { descriptor, .. } | Self::TransferOwnership { descriptor, .. } => descriptor.generation,
            Self::BeginDrain { generation, .. } | Self::CleanupListener { generation, .. } => *generation,
            Self::OpenSession { session_id, .. }
            | Self::OpenStream { session_id, .. }
            | Self::SendFrame { session_id, .. }
            | Self::ReceiveFrame { session_id, .. }
            | Self::AcknowledgeFrame { session_id, .. }
            | Self::SendDatagram { session_id, .. }
            | Self::CompleteDatagram { session_id, .. }
            | Self::GrantCredit { session_id, .. }
            | Self::HalfCloseStream { session_id, .. }
            | Self::CloseStream { session_id, .. }
            | Self::CloseSession { session_id, .. }
            | Self::FailSession { session_id, .. } => session_id.generation,
            Self::Cancel { target, .. } => match target {
                CancelTarget::Session(session_id) | CancelTarget::Stream { session_id, .. } => session_id.generation,
            },
        }
    }

    pub fn operation_id(&self) -> &str {
        match self {
            Self::Register { operation_id, .. }
            | Self::TransferOwnership { operation_id, .. }
            | Self::BeginDrain { operation_id, .. }
            | Self::CleanupListener { operation_id, .. }
            | Self::OpenSession { operation_id, .. }
            | Self::OpenStream { operation_id, .. }
            | Self::SendFrame { operation_id, .. }
            | Self::ReceiveFrame { operation_id, .. }
            | Self::AcknowledgeFrame { operation_id, .. }
            | Self::SendDatagram { operation_id, .. }
            | Self::CompleteDatagram { operation_id, .. }
            | Self::GrantCredit { operation_id, .. }
            | Self::HalfCloseStream { operation_id, .. }
            | Self::CloseStream { operation_id, .. }
            | Self::CloseSession { operation_id, .. }
            | Self::Cancel { operation_id, .. }
            | Self::FailSession { operation_id, .. } => operation_id,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportTransitionDecision {
    Applied,
    Backpressured,
}

impl TransportTransitionDecision {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Applied => "applied",
            Self::Backpressured => "backpressured",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransportTransition {
    pub next: TransportState,
    pub decision: TransportTransitionDecision,
    pub events: Vec<TransportEvent>,
    pub automatic_retry_count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OuterFrameObservation {
    pub payload_ref: String,
    pub payload_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransportIssue {
    ProfileSchemaMismatch,
    ProtocolSchemaMismatch,
    EmptyField(&'static str),
    MalformedField(&'static str),
    MalformedContentRef(&'static str),
    ZeroLimit(&'static str),
    DuplicateValue(&'static str),
    MissingCapability(TransportCapability),
    UnsupportedCapability(TransportCapability),
    MissingNonClaim(TransportNonClaim),
    ProfileMismatch,
    FramingBoundExceedsProfile,
    ListenerLimitExceeded,
    SessionLimitExceeded,
    StreamLimitExceeded,
    FrameLimitExceeded { actual: u64, maximum: u64 },
    DatagramLimitExceeded { actual: u64, maximum: u64 },
    QueueEventLimitExceeded,
    QueueByteLimitExceeded,
    InflightByteLimitExceeded,
    DeadlineExpired,
    DeadlineTooLarge,
    UnknownProtocol,
    DuplicateProtocol,
    ConflictingProtocolIdentity,
    ProtocolDraining,
    ProtocolNotDraining,
    StaleGeneration { active: u64, requested: u64 },
    GenerationDidNotAdvance,
    ServiceIdentityMismatch,
    ExtensionIdentityMismatch,
    RegistrationAuthorityMismatch,
    CleanupEvidenceRequired,
    ActiveSessionsRemain,
    UnknownSession,
    SessionTerminal(SessionPhase),
    UnknownStream,
    StreamTerminal(StreamPhase),
    WrongGenerationHandle,
    HandleServiceMismatch,
    DuplicateHandle,
    TransportIdentityWithoutServiceAuthority,
    StreamDirectionUnsupported,
    SendDirectionClosed,
    ReceiveDirectionClosed,
    SequenceMismatch { expected: u64, actual: u64 },
    CreditOverflow,
    CounterOverflow,
    PayloadRefMismatch,
    EmptyPayload,
    InvalidAcknowledgement,
    CollectionLimitExceeded,
}

pub fn validate_transport_profile(profile: &TransportProfile) -> Result<(), Vec<TransportIssue>> {
    let mut issues = Vec::new();
    if profile.schema != TRANSPORT_PROFILE_SCHEMA {
        issues.push(TransportIssue::ProfileSchemaMismatch);
    }
    validate_token("profile-id", &profile.profile_id, &mut issues);
    validate_ref("profile-ref", &profile.profile_ref, &mut issues);
    validate_unique("capabilities", &profile.capabilities, &mut issues);
    validate_unique("non-claims", &profile.non_claims, &mut issues);
    if !profile.capabilities.contains(&TransportCapability::BidirectionalStreams) {
        issues.push(TransportIssue::MissingCapability(TransportCapability::BidirectionalStreams));
    }
    for required in REQUIRED_TRANSPORT_NON_CLAIMS {
        if !profile.non_claims.contains(&required) {
            issues.push(TransportIssue::MissingNonClaim(required));
        }
    }
    validate_limits(&profile.limits, &mut issues);
    if issues.is_empty() { Ok(()) } else { Err(issues) }
}

pub fn validate_protocol_descriptor(
    profile: &TransportProfile,
    descriptor: &ProtocolDescriptor,
) -> Result<(), Vec<TransportIssue>> {
    let mut issues = validate_transport_profile(profile).err().unwrap_or_default();
    if descriptor.schema != TRANSPORT_PROTOCOL_SCHEMA {
        issues.push(TransportIssue::ProtocolSchemaMismatch);
    }
    validate_token("protocol-id", &descriptor.protocol_id, &mut issues);
    validate_token("protocol-version", &descriptor.version, &mut issues);
    validate_alpn(&descriptor.alpn, &mut issues);
    validate_token("extension-id", &descriptor.extension_id, &mut issues);
    validate_token("service-id", &descriptor.service_id, &mut issues);
    validate_positive("generation", descriptor.generation, &mut issues);
    validate_positive("listener-limit", descriptor.listener_limit, &mut issues);
    validate_unique("requested-capabilities", &descriptor.requested_capabilities, &mut issues);
    validate_ref("registration-authority-ref", &descriptor.registration_authority_ref, &mut issues);
    validate_ref("descriptor-profile-ref", &descriptor.profile_ref, &mut issues);
    if descriptor.profile_ref != profile.profile_ref {
        issues.push(TransportIssue::ProfileMismatch);
    }
    if descriptor.listener_limit > profile.limits.max_listeners {
        issues.push(TransportIssue::ListenerLimitExceeded);
    }
    for capability in &descriptor.requested_capabilities {
        if !profile.capabilities.contains(capability) {
            issues.push(TransportIssue::UnsupportedCapability(*capability));
        }
    }
    validate_framing(profile, &descriptor.framing, &mut issues);
    match descriptor.cleanup_policy {
        ListenerCleanupPolicy::Immediate => {}
        ListenerCleanupPolicy::BoundedDrain { grace_ticks } => {
            validate_positive("cleanup-grace-ticks", grace_ticks, &mut issues);
        }
    }
    if issues.is_empty() { Ok(()) } else { Err(issues) }
}

pub fn validate_outer_frame(
    profile: &TransportProfile,
    declared_ref: &str,
    actual_ref: &str,
    actual_bytes: u64,
) -> Result<OuterFrameObservation, Vec<TransportIssue>> {
    let mut issues = validate_transport_profile(profile).err().unwrap_or_default();
    validate_ref("declared-payload-ref", declared_ref, &mut issues);
    validate_ref("actual-payload-ref", actual_ref, &mut issues);
    if declared_ref != actual_ref {
        issues.push(TransportIssue::PayloadRefMismatch);
    }
    if actual_bytes == 0 {
        issues.push(TransportIssue::EmptyPayload);
    }
    if actual_bytes > profile.limits.max_frame_bytes {
        issues.push(TransportIssue::FrameLimitExceeded {
            actual: actual_bytes,
            maximum: profile.limits.max_frame_bytes,
        });
    }
    if issues.is_empty() {
        Ok(OuterFrameObservation {
            payload_ref: actual_ref.to_string(),
            payload_bytes: actual_bytes,
        })
    } else {
        Err(issues)
    }
}

fn validate_limits(limits: &TransportLimits, issues: &mut Vec<TransportIssue>) {
    for (field, value) in [
        ("max-listeners", limits.max_listeners),
        ("max-sessions", limits.max_sessions),
        ("max-streams-per-session", limits.max_streams_per_session),
        ("max-frame-bytes", limits.max_frame_bytes),
        ("max-datagram-bytes", limits.max_datagram_bytes),
        ("max-queued-events", limits.max_queued_events),
        ("max-queued-bytes", limits.max_queued_bytes),
        ("max-inflight-bytes", limits.max_inflight_bytes),
        ("operation-deadline-ticks", limits.operation_deadline_ticks),
    ] {
        validate_positive(field, value, issues);
    }
    if limits.max_frame_bytes > limits.max_queued_bytes || limits.max_frame_bytes > limits.max_inflight_bytes {
        issues.push(TransportIssue::FramingBoundExceedsProfile);
    }
}

fn validate_framing(profile: &TransportProfile, framing: &FramingProfile, issues: &mut Vec<TransportIssue>) {
    validate_token("framing-profile-id", &framing.profile_id, issues);
    validate_ref("framing-profile-ref", &framing.profile_ref, issues);
    validate_positive("framing-max-frame-bytes", framing.max_frame_bytes, issues);
    validate_positive("length-prefix-bytes", framing.length_prefix_bytes, issues);
    if framing.max_frame_bytes > profile.limits.max_frame_bytes {
        issues.push(TransportIssue::FramingBoundExceedsProfile);
    }
}

pub(crate) fn validate_scoped_id(id: &ScopedTransportId, issues: &mut Vec<TransportIssue>) {
    validate_ref("opaque-transport-id", &id.opaque_ref, issues);
    validate_token("transport-id-service", &id.service_id, issues);
    validate_positive("transport-id-generation", id.generation, issues);
}

pub(crate) fn validate_peer(peer: &PeerIdentityRefs, issues: &mut Vec<TransportIssue>) {
    validate_ref("transport-identity-ref", &peer.transport_identity_ref, issues);
    for (field, value) in [
        ("membership-ref", peer.membership_ref.as_deref()),
        ("application-principal-ref", peer.application_principal_ref.as_deref()),
        ("trust-decision-ref", peer.trust_decision_ref.as_deref()),
        ("capability-authority-ref", peer.capability_authority_ref.as_deref()),
        ("bootstrap-policy-ref", peer.bootstrap_policy_ref.as_deref()),
    ] {
        if let Some(value) = value {
            validate_ref(field, value, issues);
        }
    }
    if !peer.has_service_authority() {
        issues.push(TransportIssue::TransportIdentityWithoutServiceAuthority);
    }
}

pub(crate) fn validate_operation_id(value: &str, issues: &mut Vec<TransportIssue>) {
    validate_ref("operation-id", value, issues);
}

fn validate_token(field: &'static str, value: &str, issues: &mut Vec<TransportIssue>) {
    if value.is_empty() {
        issues.push(TransportIssue::EmptyField(field));
    } else if value.len() > MAX_TRANSPORT_TEXT_BYTES || !valid_fabric_token(value) {
        issues.push(TransportIssue::MalformedField(field));
    }
}

fn validate_alpn(value: &str, issues: &mut Vec<TransportIssue>) {
    if value.is_empty() {
        issues.push(TransportIssue::EmptyField("alpn"));
    } else if value.len() > MAX_TRANSPORT_TEXT_BYTES
        || !value.bytes().all(|byte| byte.is_ascii_graphic() && byte != b' ')
    {
        issues.push(TransportIssue::MalformedField("alpn"));
    }
}

fn validate_ref(field: &'static str, value: &str, issues: &mut Vec<TransportIssue>) {
    if !valid_blake3_ref(value) {
        issues.push(TransportIssue::MalformedContentRef(field));
    }
}

fn validate_positive(field: &'static str, value: u64, issues: &mut Vec<TransportIssue>) {
    if value == 0 {
        issues.push(TransportIssue::ZeroLimit(field));
    }
}

fn validate_unique<T: Ord>(field: &'static str, values: &[T], issues: &mut Vec<TransportIssue>) {
    let mut sorted = values.iter().collect::<Vec<_>>();
    sorted.sort();
    if sorted.windows(ADJACENT_PAIR_WIDTH).any(|pair| pair[0] == pair[1]) {
        issues.push(TransportIssue::DuplicateValue(field));
    }
    if values.len() > MAX_TRANSPORT_COLLECTION_ITEMS {
        issues.push(TransportIssue::CollectionLimitExceeded);
    }
}

#[cfg(test)]
mod tests;

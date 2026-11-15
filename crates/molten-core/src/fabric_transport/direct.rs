//! Clock-free admission for the explicitly selected direct-stream profile.
use super::*;

pub const DIRECT_LAN_STREAM_PROFILE: &str = "molten-iroh-direct-lan-stream-v1";
pub const MAX_DIRECT_FRAME_BYTES: u64 = 1_048_576;
pub const MAX_DIRECT_BUFFERED_BYTES: u64 = 67_108_864;
pub const MAX_DIRECT_DEADLINE_MS: u64 = 300_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectStreamBounds {
    pub max_connections: u64,
    pub max_frame_bytes: u64,
    pub max_total_bytes: u64,
    pub max_frames: u64,
    pub max_buffered_bytes: u64,
    pub idle_timeout_ms: u64,
    pub total_timeout_ms: u64,
    pub cleanup_timeout_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectPeerBinding {
    /// Bare, authenticated Iroh EndpointId, not an address hint.
    pub endpoint_id: String,
    pub peer_context_ref: String,
    pub service_id: String,
    pub alpn: String,
    pub generation: u64,
    pub remote_role: EndpointParticipantRole,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DirectStreamIssue {
    InvalidProfile,
    InvalidBounds,
    InvalidPeer,
    WrongPeer,
    WrongContext,
    WrongService,
    WrongAlpn,
    StaleGeneration,
    WrongRole,
    EarlyData,
}

impl DirectStreamIssue {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidProfile => "direct-profile-invalid",
            Self::InvalidBounds => "direct-bounds-invalid",
            Self::InvalidPeer => "direct-peer-invalid",
            Self::WrongPeer => "direct-peer-mismatch",
            Self::WrongContext => "direct-context-mismatch",
            Self::WrongService => "direct-service-mismatch",
            Self::WrongAlpn => "direct-alpn-mismatch",
            Self::StaleGeneration => "direct-generation-mismatch",
            Self::WrongRole => "direct-role-mismatch",
            Self::EarlyData => "direct-early-data-denied",
        }
    }
}

// r[impl molten.fabric_transport.direct_stream_bounds]
pub fn validate_direct_stream_bounds(
    profile: &TransportProfile,
    protocol: &ProtocolDescriptor,
    bounds: &DirectStreamBounds,
) -> Result<(), DirectStreamIssue> {
    if profile.profile_id != DIRECT_LAN_STREAM_PROFILE
        || profile.adapter_kind != TransportAdapterKind::IrohLive
        || validate_transport_profile(profile).is_err()
        || validate_protocol_descriptor(profile, protocol).is_err()
    {
        return Err(DirectStreamIssue::InvalidProfile);
    }
    // This profile deliberately serializes one admitted operation/connection and
    // one bidirectional stream. No hidden task or transfer queue is created.
    let required_buffers = bounds.max_frame_bytes.checked_mul(6).and_then(|n| n.checked_add(1_024));
    if bounds.max_connections != 1
        || profile.limits.max_sessions != 1
        || profile.limits.max_streams_per_session != 1
        || bounds.max_frame_bytes == 0
        || bounds.max_frame_bytes > MAX_DIRECT_FRAME_BYTES
        || bounds.max_frame_bytes > protocol.framing.max_frame_bytes
        || bounds.max_frame_bytes > profile.limits.max_queued_bytes
        || bounds.max_frame_bytes > profile.limits.max_inflight_bytes
        || bounds.max_total_bytes < bounds.max_frame_bytes
        || bounds.max_frames == 0
        || bounds.max_frames > (u64::MAX - 2) / 2
        || required_buffers.is_none_or(|n| n > bounds.max_buffered_bytes)
        || bounds.max_buffered_bytes > MAX_DIRECT_BUFFERED_BYTES
        || bounds.idle_timeout_ms == 0
        || bounds.idle_timeout_ms > bounds.total_timeout_ms
        || bounds.total_timeout_ms > MAX_DIRECT_DEADLINE_MS
        || bounds.cleanup_timeout_ms == 0
        || bounds.cleanup_timeout_ms > MAX_DIRECT_DEADLINE_MS
    {
        return Err(DirectStreamIssue::InvalidBounds);
    }
    Ok(())
}

// r[impl molten.fabric_transport.direct_lan]
pub fn validate_direct_peer(
    inbound: &GatedInbound,
    expected: &DirectPeerBinding,
    protocol: &ProtocolDescriptor,
    established: bool,
) -> Result<(), DirectStreamIssue> {
    if !established {
        return Err(DirectStreamIssue::EarlyData);
    }
    let observed = inbound.observation();
    if expected.endpoint_id.is_empty()
        || expected.endpoint_id.len() > MAX_TRANSPORT_TEXT_BYTES
        || !valid_blake3_ref(&expected.peer_context_ref)
    {
        return Err(DirectStreamIssue::InvalidPeer);
    }
    if observed.peer_identity != expected.endpoint_id {
        return Err(DirectStreamIssue::WrongPeer);
    }
    if observed.peer_context_ref != expected.peer_context_ref {
        return Err(DirectStreamIssue::WrongContext);
    }
    if observed.service_id != expected.service_id || expected.service_id != protocol.service_id {
        return Err(DirectStreamIssue::WrongService);
    }
    if observed.alpn != expected.alpn || expected.alpn != protocol.alpn {
        return Err(DirectStreamIssue::WrongAlpn);
    }
    if observed.generation != expected.generation || expected.generation != protocol.generation {
        return Err(DirectStreamIssue::StaleGeneration);
    }
    if observed.role != expected.remote_role {
        return Err(DirectStreamIssue::WrongRole);
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectStreamRequest {
    pub request_ref: String,
    pub operation_ref: String,
    pub resource_ref: String,
    pub generation: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DirectStreamTerminal {
    Clean,
    Denied,
    Cancelled,
    Timeout,
    Disconnected,
    Malformed,
    OverBound,
}
impl DirectStreamTerminal {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Clean => "clean",
            Self::Denied => "denied",
            Self::Cancelled => "cancelled",
            Self::Timeout => "timeout",
            Self::Disconnected => "disconnected",
            Self::Malformed => "malformed",
            Self::OverBound => "over-bound",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DirectStreamPath {
    DirectLoopback,
    DirectIp,
    Unavailable,
}
impl DirectStreamPath {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::DirectLoopback => "direct-loopback",
            Self::DirectIp => "direct-ip",
            Self::Unavailable => "unavailable",
        }
    }
}

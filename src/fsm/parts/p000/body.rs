type IoValue = preserves::IOValue;
type Result<T> = crate::error::Result<T>;

const PEER_PROFILE_SCHEMA: &str = "molten.peer-profile.v1";
const PEER_SESSION_SCHEMA: &str = "molten.peer-session.v1";
const PEER_TRANSITION_SCHEMA: &str = "molten.peer-session-transition-receipt.v1";
const MAX_PEER_SESSION_DIAGNOSTICS: usize = 16;
const MAX_PEER_SESSION_GUARD_REFS: usize = 16;
const _: () = assert!(MAX_PEER_SESSION_DIAGNOSTICS > 0);
const _: () = assert!(MAX_PEER_SESSION_GUARD_REFS > 0);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StateKind {
    Discovered,
    Invited,
    Handshaking,
    Negotiated,
    Admitted,
    Connected,
    Expired,
    Revoked,
    Quarantined,
}

impl StateKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Discovered => "discovered",
            Self::Invited => "invited",
            Self::Handshaking => "handshaking",
            Self::Negotiated => "negotiated",
            Self::Admitted => "admitted",
            Self::Connected => "connected",
            Self::Expired => "expired",
            Self::Revoked => "revoked",
            Self::Quarantined => "quarantined",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventKind {
    Invite,
    HandshakeStart,
    NegotiationPass,
    Admit,
    Connect,
    Expire,
    Revoke,
    Quarantine,
    Recover,
}

impl EventKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Invite => "invite",
            Self::HandshakeStart => "handshake-start",
            Self::NegotiationPass => "negotiation-pass",
            Self::Admit => "admit",
            Self::Connect => "connect",
            Self::Expire => "expire",
            Self::Revoke => "revoke",
            Self::Quarantine => "quarantine",
            Self::Recover => "recover",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Profile {
    pub peer_ref: String,
    pub endpoint: String,
    pub scope: String,
    pub resource_ref: String,
    pub freshness_tick: u64,
    pub revocation_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record {
    pub peer_ref: String,
    pub session_ref: String,
    pub topic: String,
    pub state: StateKind,
    pub bootstrap_refs: Vec<String>,
    pub capability_refs: Vec<String>,
    pub authority_refs: Vec<String>,
    pub policy_refs: Vec<String>,
    pub resource_refs: Vec<String>,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransitionInput {
    pub prior: Record,
    pub event: EventKind,
    pub target: StateKind,
    pub observed_topic: String,
    pub at_tick: u64,
    pub required_bootstrap_ref: Option<String>,
    pub required_authority_ref: Option<String>,
    pub required_recovery_ref: Option<String>,
    pub revocation_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransitionDecision {
    pub decision: String,
    pub prior_state: StateKind,
    pub event: EventKind,
    pub target_state: StateKind,
    pub next_state: StateKind,
    pub before_state_ref: String,
    pub after_state_ref: String,
    pub guard_refs: Vec<String>,
    pub diagnostics: Vec<String>,
    pub session: Record,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransitionReceipt {
    pub decision: String,
    pub session: Record,
    pub diagnostics: Vec<String>,
    pub value: IoValue,
    pub receipt_ref: String,
}

#[derive(Clone, Copy)]
struct TransitionView<'a> {
    prior_state: StateKind,
    event: EventKind,
    target: StateKind,
    prior_topic: &'a str,
    observed_topic: &'a str,
    at_tick: u64,
    bootstrap_refs: &'a [String],
    authority_refs: &'a [String],
    required_bootstrap_ref: Option<&'a String>,
    required_authority_ref: Option<&'a String>,
    required_recovery_ref: Option<&'a String>,
    revocation_ref: Option<&'a String>,
}

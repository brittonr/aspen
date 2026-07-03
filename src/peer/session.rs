type IoValue = preserves::IOValue;
type Result<T> = crate::error::Result<T>;

const PEER_PROFILE_SCHEMA: &str = "molten.peer-profile.v1";
const PEER_SESSION_SCHEMA: &str = "molten.peer-session.v1";
const PEER_TRANSITION_SCHEMA: &str = "molten.peer-session-transition-receipt.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeerSessionStateKind {
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

impl PeerSessionStateKind {
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeerProfile {
    pub peer_ref: String,
    pub endpoint: String,
    pub scope: String,
    pub resource_ref: String,
    pub freshness_tick: u64,
    pub revocation_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeerSession {
    pub peer_ref: String,
    pub session_ref: String,
    pub topic: String,
    pub state: PeerSessionStateKind,
    pub bootstrap_refs: Vec<String>,
    pub capability_refs: Vec<String>,
    pub authority_refs: Vec<String>,
    pub policy_refs: Vec<String>,
    pub resource_refs: Vec<String>,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeerTransitionInput {
    pub prior: PeerSession,
    pub target: PeerSessionStateKind,
    pub observed_topic: String,
    pub at_tick: u64,
    pub required_bootstrap_ref: Option<String>,
    pub required_authority_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeerTransitionReceipt {
    pub decision: String,
    pub session: PeerSession,
    pub diagnostics: Vec<String>,
    pub value: IoValue,
    pub receipt_ref: String,
}

pub fn peer_profile_value(profile: &PeerProfile) -> IoValue {
    crate::preserves_rail::record("peer-profile-v1", vec![
        string(PEER_PROFILE_SCHEMA),
        field("peer-ref", &profile.peer_ref),
        field("endpoint", &profile.endpoint),
        field("scope", &profile.scope),
        field("resource-ref", &profile.resource_ref),
        crate::preserves_rail::record("freshness-tick", vec![crate::preserves_rail::u64_value(profile.freshness_tick)]),
        list_field("revocation-refs", &profile.revocation_refs),
        list_field("evidence-refs", &profile.evidence_refs),
    ])
}

pub fn peer_session_value(session: &PeerSession) -> IoValue {
    crate::preserves_rail::record("peer-session-v1", vec![
        string(PEER_SESSION_SCHEMA),
        field("peer-ref", &session.peer_ref),
        field("session-ref", &session.session_ref),
        field("topic", &session.topic),
        field("state", session.state.as_str()),
        list_field("bootstrap-refs", &session.bootstrap_refs),
        list_field("capability-refs", &session.capability_refs),
        list_field("authority-refs", &session.authority_refs),
        list_field("policy-refs", &session.policy_refs),
        list_field("resource-refs", &session.resource_refs),
        list_field("diagnostics", &session.diagnostics),
    ])
}

pub fn apply_peer_transition(input: &PeerTransitionInput) -> Result<PeerTransitionReceipt> {
    let mut diagnostics = Vec::new();
    if input.observed_topic != input.prior.topic {
        diagnostics.push("wrong topic for peer session".to_string());
    }
    if input.target == PeerSessionStateKind::Admitted
        && missing(input.required_bootstrap_ref.as_ref(), &input.prior.bootstrap_refs)
    {
        diagnostics.push("missing bootstrap admission".to_string());
    }
    if input.target == PeerSessionStateKind::Connected
        && missing(input.required_authority_ref.as_ref(), &input.prior.authority_refs)
    {
        diagnostics.push("missing authority grant".to_string());
    }
    if input.target == PeerSessionStateKind::Expired && input.at_tick == 0 {
        diagnostics.push("stale ticket requires nonzero expiry tick".to_string());
    }
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let mut session = input.prior.clone();
    if decision == "pass" {
        session.state = input.target;
    }
    session.diagnostics = diagnostics.clone();
    let value = transition_value(decision, &session, &diagnostics);
    let receipt_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(PeerTransitionReceipt {
        decision: decision.to_string(),
        session,
        diagnostics,
        value,
        receipt_ref,
    })
}

pub fn peer_session_as_authority_denial(session_ref: &str, operation: &str) -> Result<PeerTransitionReceipt> {
    crate::preserves_rail::validate_content_ref(session_ref)?;
    let diagnostics = vec![format!(
        "peer session {session_ref} is transport state, not authority for {operation}"
    )];
    let session = PeerSession {
        peer_ref: session_ref.to_string(),
        session_ref: session_ref.to_string(),
        topic: operation.to_string(),
        state: PeerSessionStateKind::Quarantined,
        bootstrap_refs: Vec::new(),
        capability_refs: Vec::new(),
        authority_refs: Vec::new(),
        policy_refs: Vec::new(),
        resource_refs: Vec::new(),
        diagnostics: diagnostics.clone(),
    };
    let value = transition_value("deny", &session, &diagnostics);
    let receipt_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(PeerTransitionReceipt {
        decision: "deny".to_string(),
        session,
        diagnostics,
        value,
        receipt_ref,
    })
}

fn transition_value(decision: &str, session: &PeerSession, diagnostics: &[String]) -> IoValue {
    crate::preserves_rail::record("peer-session-transition-receipt-v1", vec![
        string(PEER_TRANSITION_SCHEMA),
        field("decision", decision),
        field("peer-ref", &session.peer_ref),
        field("session-ref", &session.session_ref),
        field("state", session.state.as_str()),
        list_field("diagnostics", diagnostics),
        field("evidence-only", "transport-does-not-grant-authority"),
    ])
}

fn missing(required: Option<&String>, available: &[String]) -> bool {
    required.is_some_and(|reference| !available.iter().any(|value| value == reference))
}

fn field(label: &'static str, value: &str) -> IoValue {
    crate::preserves_rail::record(label, vec![string(value)])
}

fn list_field(label: &'static str, values: &[String]) -> IoValue {
    crate::preserves_rail::record(label, vec![crate::preserves_rail::sequence(values.iter().map(string).collect())])
}

fn string(value: impl AsRef<str>) -> IoValue {
    crate::preserves_rail::string(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONNECT_TICK: u64 = 4;

    #[test]
    fn peer_lifecycle_reaches_connected_with_required_evidence() {
        let bootstrap_ref = test_ref("bootstrap");
        let authority_ref = test_ref("authority");
        let prior = session(PeerSessionStateKind::Admitted, vec![bootstrap_ref.clone()], vec![authority_ref.clone()]);
        let receipt = apply_peer_transition(&PeerTransitionInput {
            prior,
            target: PeerSessionStateKind::Connected,
            observed_topic: "node-control".to_string(),
            at_tick: CONNECT_TICK,
            required_bootstrap_ref: Some(bootstrap_ref),
            required_authority_ref: Some(authority_ref),
        })
        .expect("transition");
        assert_eq!(receipt.decision, "pass");
        assert_eq!(receipt.session.state, PeerSessionStateKind::Connected);
    }

    #[test]
    fn wrong_topic_and_missing_authority_deny() {
        let prior = session(PeerSessionStateKind::Admitted, vec![test_ref("bootstrap")], Vec::new());
        let receipt = apply_peer_transition(&PeerTransitionInput {
            prior,
            target: PeerSessionStateKind::Connected,
            observed_topic: "wrong-topic".to_string(),
            at_tick: CONNECT_TICK,
            required_bootstrap_ref: None,
            required_authority_ref: Some(test_ref("authority")),
        })
        .expect("transition");
        assert_eq!(receipt.decision, "deny");
        assert!(receipt.diagnostics.iter().any(|diagnostic| diagnostic.contains("wrong topic")));
        assert!(receipt.diagnostics.iter().any(|diagnostic| diagnostic.contains("missing authority")));
    }

    #[test]
    fn connected_session_is_not_authority() {
        let receipt = peer_session_as_authority_denial(&test_ref("session"), "publish").expect("denial");
        assert_eq!(receipt.decision, "deny");
        assert!(receipt.diagnostics[0].contains("not authority"));
    }

    fn session(state: PeerSessionStateKind, bootstrap_refs: Vec<String>, authority_refs: Vec<String>) -> PeerSession {
        PeerSession {
            peer_ref: test_ref("peer"),
            session_ref: test_ref("session"),
            topic: "node-control".to_string(),
            state,
            bootstrap_refs,
            capability_refs: Vec::new(),
            authority_refs,
            policy_refs: vec![test_ref("policy")],
            resource_refs: vec![test_ref("resource")],
            diagnostics: Vec::new(),
        }
    }

    fn test_ref(label: &str) -> String {
        crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("peer-session-test-ref", vec![string(
            label,
        )]))
        .expect("test ref")
    }
}

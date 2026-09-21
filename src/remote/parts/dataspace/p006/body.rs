
const MAX_REMOTE_SESSIONS: usize = 1_024;
const MAX_SESSION_RETRACTIONS: usize = 4_096;
const _: () = assert!(MAX_REMOTE_SESSIONS > 0);
const _: () = assert!(MAX_SESSION_RETRACTIONS > 0);

const SESSION_STATE_OPEN: &str = "open";
const SESSION_STATE_CLOSED: &str = "closed";
const SESSION_CLOSE_CAUSE_DISCONNECT: &str = "disconnect";
const SESSION_CLOSE_CAUSE_STOP: &str = "stop";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoteSessionState {
    Open,
    Closed,
}

impl RemoteSessionState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Open => SESSION_STATE_OPEN,
            Self::Closed => SESSION_STATE_CLOSED,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteSession {
    pub session_ref: String,
    pub owner: String,
    pub receiver_peer: String,
    pub topic: String,
    pub generation: u64,
    pub state: RemoteSessionState,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RemoteSessionRegistry {
    sessions: std::collections::BTreeMap<String, RemoteSession>,
}

impl RemoteSessionRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn open_session(&mut self, receiver_peer: &str, topic: &str, generation: u64) -> Result<RemoteSession> {
        validate_name(receiver_peer, "receiver peer")?;
        validate_name(topic, "topic")?;
        let session_ref = remote_session_ref(receiver_peer, topic, generation)?;
        if let Some(existing) = self.sessions.get(&session_ref) {
            return Err(MoltenError::invalid_harness(format!(
                "remote dataspace session identity {session_ref} is already recorded as {} and cannot be reused",
                existing.state.as_str()
            )));
        }
        ensure_count_at_most(self.sessions.len(), MAX_REMOTE_SESSIONS, "remote dataspace sessions")?;
        let session = RemoteSession {
            owner: remote_session_owner(receiver_peer, topic, generation),
            session_ref,
            receiver_peer: receiver_peer.to_owned(),
            topic: topic.to_owned(),
            generation,
            state: RemoteSessionState::Open,
        };
        self.sessions.insert(session.session_ref.clone(), session.clone());
        Ok(session)
    }

    pub fn session(&self, session_ref: &str) -> Option<&RemoteSession> {
        self.sessions.get(session_ref)
    }

    fn mark_closed(&mut self, session_ref: &str) {
        if let Some(session) = self.sessions.get_mut(session_ref)
            && session.state == RemoteSessionState::Open
        {
            session.state = RemoteSessionState::Closed;
        }
    }
}

pub fn remote_session_ref(receiver_peer: &str, topic: &str, generation: u64) -> Result<String> {
    canonical_hash(&record("remote-dataspace-session-v1", vec![
        string(crate::preserves_rail::REMOTE_DATASPACE_SESSION_SCHEMA),
        record("receiver-peer", vec![string(receiver_peer)]),
        record("topic", vec![string(topic)]),
        record("generation", vec![u64_value(generation)]),
    ]))
}

pub fn remote_session_owner(receiver_peer: &str, topic: &str, generation: u64) -> String {
    format!("session:{receiver_peer}/{topic}:{generation}")
}

fn u64_value(value: u64) -> IoValue {
    crate::preserves_rail::u64_value(value)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionAdmission {
    Applied(SessionApplied),
    Denied(SessionDenial),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionApplied {
    pub session_ref: String,
    pub owner: String,
    pub events: Vec<RuntimeEvent>,
    pub admission_receipt_value: IoValue,
    pub applied_assertion_value: Option<IoValue>,
    pub turn_journal_context_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionDenial {
    pub session_ref: String,
    pub admission_receipt_value: IoValue,
    pub diagnostics: Vec<String>,
}

pub fn apply_delivered_envelope_owned_by(
    state: &mut RuntimeState,
    envelope: &Envelope,
    owner: &str,
) -> Result<Vec<RuntimeEvent>> {
    validate_envelope_identity(envelope)?;
    let payload = RuntimeValue::new(envelope.payload.clone())?;
    let step = remote_operation_step(envelope, owner, payload)?;
    Ok(state.apply_step(&step))
}

fn remote_operation_step(envelope: &Envelope, owner: &str, payload: RuntimeValue) -> Result<RuntimeStep> {
    Ok(match envelope.operation {
        Operation::Assert => RuntimeStep::Assert {
            actor: owner.to_owned(),
            value: payload,
        },
        Operation::Retract => RuntimeStep::Retract {
            actor: owner.to_owned(),
            value: payload,
        },
        Operation::Observe => RuntimeStep::Observe {
            actor: owner.to_owned(),
            pattern: payload,
        },
        Operation::Message => RuntimeStep::Send {
            from: owner.to_owned(),
            to: format!("{}:inbox", envelope.to_peer),
            body: payload,
        },
    })
}

pub fn admit_and_apply_delivered_envelope_for_session(
    state: &mut RuntimeState,
    sessions: &RemoteSessionRegistry,
    session_ref: &str,
    delivery: &Delivery,
    evidence: &DeliveryEvidence,
) -> Result<SessionAdmission> {
    let transport_receipt_ref = canonical_hash(&delivery.receipt_value)?;
    if let Some(diagnostics) = declared_owner_diagnostics(sessions, session_ref) {
        return Ok(SessionAdmission::Denied(SessionDenial {
            session_ref: session_ref.to_owned(),
            admission_receipt_value: deny_admission_receipt_value(
                &delivery.envelope,
                &transport_receipt_ref,
                diagnostics.clone(),
            ),
            diagnostics,
        }));
    }
    let session = sessions.session(session_ref).ok_or_else(|| {
        MoltenError::invalid_harness(format!("declared owner session is unknown: {session_ref}"))
    })?;
    let owner = session.owner.clone();
    validate_delivery_evidence(&delivery.envelope, evidence)?;
    let turn_journal_context_ref_value = turn_journal_context_ref(delivery)?;
    let mut turn_context_refs = vec![turn_journal_context_ref_value.clone()];
    turn_context_refs.push(transport_receipt_ref.clone());
    let admission_receipt_value = remote_admission_receipt_value(AdmissionReceiptInput {
        decision: "pass",
        envelope: &delivery.envelope,
        transport_receipt_ref: &transport_receipt_ref,
        evidence,
        turn_context_refs: &turn_context_refs,
        diagnostics: Vec::new(),
    });
    let events = apply_delivered_envelope_owned_by(state, &delivery.envelope, &owner)?;
    let applied_assertion_value = (delivery.envelope.operation == Operation::Assert)
        .then(|| applied_assertion_value(AppliedAssertionInput {
            owner: &owner,
            session_ref,
            session_state: RemoteSessionState::Open,
            envelope: &delivery.envelope,
            payload: &delivery.envelope.payload,
        }))
        .transpose()?;
    Ok(SessionAdmission::Applied(SessionApplied {
        session_ref: session_ref.to_owned(),
        owner,
        events,
        admission_receipt_value,
        applied_assertion_value,
        turn_journal_context_ref: turn_journal_context_ref_value,
    }))
}

fn declared_owner_diagnostics(sessions: &RemoteSessionRegistry, session_ref: &str) -> Option<Vec<String>> {
    match sessions.session(session_ref) {
        None => Some(vec![format!("declared owner session is unknown: {session_ref}")]),
        Some(session) if session.state == RemoteSessionState::Closed => {
            Some(vec![format!("declared owner session is closed: {session_ref}")])
        }
        Some(_) => None,
    }
}

struct AppliedAssertionInput<'a> {
    owner: &'a str,
    session_ref: &'a str,
    session_state: RemoteSessionState,
    envelope: &'a Envelope,
    payload: &'a IoValue,
}

fn applied_assertion_value(input: AppliedAssertionInput<'_>) -> Result<IoValue> {
    let value = RuntimeValue::new(input.payload.clone())?;
    let assertion = crate::runtime::RuntimeAssertion {
        actor: input.owner.to_owned(),
        value,
    };
    Ok(record("remote-dataspace-applied-assertion-v1", vec![
        string(crate::preserves_rail::REMOTE_DATASPACE_APPLIED_ASSERTION_SCHEMA),
        record("assertion-ref", vec![string(&assertion.assertion_ref()?)]),
        record("owner", vec![string(input.owner)]),
        record("session-ref", vec![string(input.session_ref)]),
        record("session-state", vec![string(input.session_state.as_str())]),
        record("envelope-ref", vec![string(&input.envelope.envelope_ref)]),
        record("operation", vec![string(input.envelope.operation.as_str())]),
        record("payload-ref", vec![string(&canonical_hash(input.payload)?)]),
        record("checks", vec![sequence(vec![
            record("check", vec![string("owner-is-receiving-session"), string("pass")]),
            record("check", vec![string("no-delivery-completeness-claim"), string("pass")]),
        ])]),
    ]))
}

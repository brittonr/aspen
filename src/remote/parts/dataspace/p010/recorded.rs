// Capability-authenticated local/recorded remote session owner.
// r[impl molten.remote_dataspace.recorded_session_source]
const RECORDED_SESSIONS: redb::TableDefinition<&str, &[u8]> =
    redb::TableDefinition::new("remote_recorded_sessions_v1");
const MAX_RECORDED_SESSION_BYTES: usize = 4_194_304;
const MAX_RECORDED_OBSERVATIONS: usize = 4_096;
// A fixed close observation (two 71-byte refs, fixed payload and bounded sequence)
// fits within this reserve, so a full live history can still retire its generation.
const RECORDED_CLOSE_RESERVE_BYTES: usize = 1_024;

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RecordedSessionData {
    sender: String,
    receiver: String,
    topic: String,
    generation: u64,
    session_ref: String,
    sender_identity: String,
    receiver_identity: String,
    closed: bool,
    observations: Vec<RecordedObservation>,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RecordedObservation {
    sequence: u64,
    predecessor: String,
    operation: String,
    payload: Vec<u8>,
    content_refs: Vec<String>,
    source_envelope_ref: Option<String>,
    observation_ref: String,
}

struct RecordedTransitionInput<'a> {
    sender: &'a str,
    topic: &'a str,
    generation: u64,
    operation: &'a str,
    payload: &'a IoValue,
    content_refs: &'a [String],
    source_envelope_ref: Option<&'a str>,
}

/// The receiver's admitted local recording capability. Not a network admission.
/// There is deliberately no API to import a caller-carried snapshot or ref list.
pub struct RecordedSessionStore {
    database: redb::Database,
    receiver: String,
    receiver_identity: String,
    receiver_root: crate::node_state::DirectoryRoot,
}

/// A generation-bound borrow of the live receiver owner, not a cached snapshot.
pub struct SenderSessionView<'a> {
    store: &'a RecordedSessionStore,
    key: String,
    generation: u64,
    sender_root: crate::node_state::DirectoryRoot,
}

pub struct RecordedSessionCommit {
    pub session_ref: String,
    pub generation: u64,
    pub observation_ref: String,
    pub receipt_value: IoValue,
}

pub enum RecordedSessionOutcome {
    Committed(RecordedSessionCommit),
    Denied { receipt_value: IoValue, diagnostics: Vec<String> },
}

fn require_recorded_commit(outcome: RecordedSessionOutcome) -> Result<RecordedSessionCommit> {
    match outcome {
        RecordedSessionOutcome::Committed(commit) => Ok(commit),
        RecordedSessionOutcome::Denied { receipt_value, .. } => Err(recorded_error(
            crate::preserves_rail::to_text(&receipt_value)?)),
    }
}

/// The session coordinates a recorded denial receipt binds to.
struct RecordedDenialInput<'a> {
    sender: &'a str,
    receiver: &'a str,
    topic: &'a str,
    generation: u64,
    payload: &'a IoValue,
    source_envelope_ref: Option<&'a str>,
}

fn recorded_denial(input: &RecordedDenialInput<'_>, diagnostic: String) -> Result<RecordedSessionOutcome> {
    let receipt_value = record("remote-recorded-session-denial-v1", vec![
        string("deny"), string(input.sender), string(input.receiver), string(input.topic), u64_value(input.generation),
        string(canonical_hash(input.payload)?), sequence(vec![string(&diagnostic)]),
        sequence(input.source_envelope_ref.into_iter().map(string).collect()),
    ]);
    Ok(RecordedSessionOutcome::Denied { receipt_value, diagnostics: vec![diagnostic] })
}

fn recorded_error(error: impl std::fmt::Display) -> MoltenError {
    MoltenError::invalid_harness(format!("recorded remote session: {error}"))
}

fn recorded_identity(root: &crate::node_state::DirectoryRoot) -> Result<crate::node_identity::Identity> {
    let text = root.read_to_string(&crate::node_state::RelativePath::parse("identity.preserves")?, crate::node_state::MAX_NODE_STATE_FILE_BYTES)?;
    crate::node_identity::parse_identity(&crate::preserves_rail::parse_text(&text)?)
}

fn recorded_key(sender: &str, receiver: &str, topic: &str) -> Result<String> {
    canonical_hash(&record("remote-recorded-session-key-v1", vec![string(sender), string(receiver), string(topic)]))
}

fn observation_value(data: &RecordedSessionData, observation: &RecordedObservation) -> Result<IoValue> {
    if let Some(reference) = &observation.source_envelope_ref {
        validate_content_ref(reference)?;
    }
    Ok(record("remote-recorded-session-observation-v1", vec![
        string(&data.session_ref),
        u64_value(data.generation),
        u64_value(observation.sequence),
        string(&observation.predecessor),
        string(&observation.operation),
        parse_canonical_bytes(&observation.payload)?,
        sequence(observation.content_refs.iter().map(string).collect()),
        sequence(observation.source_envelope_ref.iter().map(string).collect()),
    ]))
}

fn replay_recorded_state(data: &RecordedSessionData) -> Result<RuntimeState> {
    if data.observations.is_empty() || data.observations.len() > MAX_RECORDED_OBSERVATIONS {
        return Err(recorded_error("missing or oversized observation history"));
    }
    let binding = canonical_hash(&record("remote-recorded-session-binding-v1", vec![
        string(&data.sender), string(&data.receiver), string(&data.topic), u64_value(data.generation),
        string(&data.sender_identity), string(&data.receiver_identity),
    ]))?;
    if binding != data.session_ref {
        return Err(recorded_error("session identity binding mismatch"));
    }
    let mut state = RuntimeState::new(0);
    let owner = format!("recorded-session:{}", data.session_ref);
    let mut predecessor = data.session_ref.clone();
    let mut is_closed = false;
    for (index, observation) in data.observations.iter().enumerate() {
        if is_closed || observation.sequence != index as u64 || observation.predecessor != predecessor {
            return Err(recorded_error("closed history, sequence gap, or predecessor mismatch"));
        }
        let value = observation_value(data, observation)?;
        if canonical_hash(&value)? != observation.observation_ref {
            return Err(recorded_error("observation hash mismatch"));
        }
        let payload = parse_canonical_bytes(&observation.payload)?;
        if index == 0 && observation.operation != "open" {
            return Err(recorded_error("history does not start with receiver admission"));
        }
        match observation.operation.as_str() {
            "open" if index == 0 => {}
            "assert" => {
                semantic_payload_references(&payload)?;
                state.apply_step_with_predicate_receipt(&RuntimeStep::Assert { actor: owner.clone(), value: RuntimeValue::new(payload)? })?;
            }
            "retract" => {
                state.apply_step_with_predicate_receipt(&RuntimeStep::Retract { actor: owner.clone(), value: RuntimeValue::new(payload)? })?;
            }
            "message" => {
                let introduced = IntroducedReferences::derive(&state, &owner,
                    &[data.sender_identity.clone(), data.receiver_identity.clone(), data.session_ref.clone()]);
                let unknown = introduced.unknown(&message_references(&payload, &observation.content_refs)?);
                if !unknown.is_empty() {
                    return Err(recorded_error(format!("unintroduced references in committed message: {}", unknown.join(", "))));
                }
                state.apply_step_with_predicate_receipt(&RuntimeStep::Send { from: owner.clone(), to: format!("{}:inbox", data.receiver),
                    body: RuntimeValue::new(payload)? })?;
            }
            "observe" => {
                state.apply_step_with_predicate_receipt(&RuntimeStep::Observe { actor: owner.clone(), pattern: RuntimeValue::new(payload)? })?;
            }
            "close" => {
                is_closed = true;
                state.cleanup_actor_scope(&owner)?;
            }
            _ => return Err(recorded_error("invalid observation transition")),
        }
        predecessor = observation.observation_ref.clone();
    }
    if is_closed != data.closed {
        return Err(recorded_error("close marker mismatch"));
    }
    Ok(state)
}

fn replay_recorded(data: &RecordedSessionData) -> Result<IntroducedReferences> {
    let state = replay_recorded_state(data)?;
    if data.closed {
        return Ok(IntroducedReferences::default());
    }
    Ok(IntroducedReferences::derive(&state, &format!("recorded-session:{}", data.session_ref),
        &[data.sender_identity.clone(), data.receiver_identity.clone(), data.session_ref.clone()]))
}

fn decode_recorded(bytes: &[u8]) -> Result<RecordedSessionData> {
    if bytes.len() > MAX_RECORDED_SESSION_BYTES { return Err(recorded_error("recording byte bound exceeded")); }
    serde_json::from_slice(bytes).map_err(recorded_error)
}

fn encode_recorded(data: &RecordedSessionData) -> Result<Vec<u8>> {
    let bytes = serde_json::to_vec(data).map_err(recorded_error)?;
    let limit = if data.closed { MAX_RECORDED_SESSION_BYTES } else { MAX_RECORDED_SESSION_BYTES - RECORDED_CLOSE_RESERVE_BYTES };
    if bytes.len() > limit { return Err(recorded_error("recording byte bound exceeded")); }
    Ok(bytes)
}

fn append_recorded(data: &mut RecordedSessionData, operation: &str, payload: &IoValue, content_refs: &[String], source_envelope_ref: Option<&str>) -> Result<RecordedSessionCommit> {
    let limit = if operation == "close" { MAX_RECORDED_OBSERVATIONS } else { MAX_RECORDED_OBSERVATIONS - 1 };
    if data.observations.len() >= limit { return Err(recorded_error("observation bound exceeded")); }
    let mut observation = RecordedObservation {
        sequence: data.observations.len() as u64,
        predecessor: data.observations.last().map_or_else(|| data.session_ref.clone(), |prior| prior.observation_ref.clone()),
        operation: operation.to_owned(), payload: canonical_bytes(payload)?, observation_ref: String::new(),
        content_refs: content_refs.to_vec(),
        source_envelope_ref: source_envelope_ref.map(str::to_owned),
    };
    let receipt_value = observation_value(data, &observation)?;
    observation.observation_ref = canonical_hash(&receipt_value)?;
    let observation_ref = observation.observation_ref.clone();
    data.observations.push(observation);
    data.closed = operation == "close";
    let state_ref = replay_recorded_state(data)?.snapshot().snapshot_ref()?;
    let receipt_value = record("remote-recorded-session-commit-v1", vec![receipt_value, string(state_ref)]);
    Ok(RecordedSessionCommit { session_ref: data.session_ref.clone(), generation: data.generation, observation_ref, receipt_value })
}

impl SenderSessionView<'_> {
    fn read_current(&self) -> Result<RecordedSessionData> {
        use redb::ReadableDatabase;
        let read = self.store.database.begin_read().map_err(recorded_error)?;
        let table = read.open_table(RECORDED_SESSIONS).map_err(recorded_error)?;
        let entry = table.get(self.key.as_str()).map_err(recorded_error)?
            .ok_or_else(|| recorded_error("unknown session"))?;
        let data = decode_recorded(entry.value())?;
        let receiver = recorded_identity(&self.store.receiver_root)?;
        let sender = recorded_identity(&self.sender_root)?;
        if receiver.identity_ref != data.receiver_identity || sender.identity_ref != data.sender_identity {
            return Err(recorded_error("initialized node identity changed; readmission required"));
        }
        if recorded_key(&data.sender, &data.receiver, &data.topic)? != self.key {
            return Err(recorded_error("recorded key binding mismatch"));
        }
        if data.generation != self.generation || data.closed || data.receiver_identity != self.store.receiver_identity {
            return Err(recorded_error("stale, closed, or identity-replaced session"));
        }
        Ok(data)
    }

    fn introduced_for(&self, from_peer: &str, to_peer: &str, topic: &str) -> Result<IntroducedReferences> {
        let data = self.read_current()?;
        if from_peer != data.sender || to_peer != data.receiver || topic != data.topic {
            return Err(recorded_error("envelope peer/topic binding mismatch"));
        }
        replay_recorded(&data)
    }

    /// Read-only actual receiver runtime state for this current generation.
    /// Possessing a snapshot cannot construct a sender view or establish refs.
    pub fn snapshot(&self) -> Result<crate::runtime::RuntimeSnapshot> {
        let data = self.read_current()?;
        Ok(replay_recorded_state(&data)?.snapshot())
    }
}

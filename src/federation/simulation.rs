pub mod adversarial;

/// Recorded, verified boundary observations, not invented crypto or admission outcomes.
pub enum Input {
    Inventory {
        record: Box<super::envelope::network::VerifiedRecord>,
        now: u64,
        requested: bool,
    },
    Transition {
        event: molten_core::federation::SessionEvent,
        generation: u64,
        now: u64,
    },
    Resume,
}
#[derive(Clone)]
pub struct Replay<'a> {
    pub manifest_ref: &'a str,
    pub bounds: &'a molten_core::federation::PullBounds,
    pub peers: &'a [molten_core::federation::PeerHint],
    pub local: &'a [molten_core::federation::InventoryEntry],
    pub generation: u64,
    pub session: Option<molten_core::federation::Session>,
    pub last_query: Option<u64>,
    pub resource_history: Vec<molten_core::federation::ResourceHistory>,
}

/// Drives the production planner, transition laws, and recovery codec with explicit logical inputs.
/// Signature faults must be denied by the real crypto adapter before an Inventory input can exist.
pub fn replay(state: &mut Replay<'_>, inputs: Vec<Input>) -> crate::error::Result<preserves::IOValue> {
    if inputs.len() > molten_core::federation::MAX_FEDERATION_ENTRIES {
        return Err(crate::error::Failure::invalid_harness("federation replay exceeds event bound"));
    }
    let mut events = Vec::with_capacity(inputs.len());
    for input in inputs {
        let observation = input_value(&input);
        let outcome = match input {
            Input::Inventory { record, now, requested } => inventory(state, &record, now, requested),
            Input::Transition { event, generation, now } => transition(state, &event, generation, now),
            Input::Resume => {
                if let Some(session) = &state.session {
                    state.session = Some(super::lifecycle::checkpoint::decode(
                        &super::lifecycle::checkpoint::encode(session),
                        state.bounds,
                    )?);
                }
                state.resource_history = super::lifecycle::resources::decode(&super::lifecycle::resources::encode(
                    &state.resource_history,
                )?)?;
                Ok("resumed".to_string())
            }
        };
        let (decision, detail) = match outcome {
            Ok(detail) => ("accepted", detail),
            Err(detail) => ("denied", detail),
        };
        events.push(crate::preserves_rail::record("federation-replay-event-v1", vec![
            observation,
            crate::preserves_rail::string(decision),
            crate::preserves_rail::string(detail),
            match &state.session {
                Some(session) => super::lifecycle::checkpoint::encode(session),
                None => crate::preserves_rail::sequence(Vec::new()),
            },
            super::lifecycle::resources::encode(&state.resource_history)?,
        ]));
    }
    Ok(crate::preserves_rail::record("federation-deterministic-trace-v1", vec![
        crate::preserves_rail::u64_value(state.generation),
        crate::preserves_rail::sequence(events),
        crate::preserves_rail::string("same-core deterministic regression evidence; not live transport or authority"),
    ]))
}
fn inventory(
    state: &mut Replay<'_>,
    record: &super::envelope::network::VerifiedRecord,
    now: u64,
    requested: bool,
) -> Result<String, String> {
    let inventory = super::envelope::records::verified_inventory(record, state.bounds)
        .map_err(|_| "record-rejected".to_string())?;
    let plan = molten_core::federation::plan_pull(&molten_core::federation::PullInput {
        bounds: state.bounds,
        configured: state.peers,
        inventory: &inventory,
        local: state.local,
        generation: state.generation,
        now,
        last_query: state.last_query,
        resource_history: &state.resource_history,
        active_sessions: usize::from(state.session.as_ref().is_some_and(|session| {
            !matches!(
                session.phase,
                molten_core::federation::SessionPhase::Committed | molten_core::federation::SessionPhase::Denied
            )
        })),
        receiver_requested: requested,
    })
    .map_err(|issue| format!("{issue:?}"))?;
    let session = super::lifecycle::planned_session(state.manifest_ref, state.generation, plan, record)
        .map_err(|_| "session-identity-rejected".to_string())?;
    if state.session.as_ref().is_some_and(|existing| {
        existing.session_ref == session.session_ref
            && !matches!(
                existing.phase,
                molten_core::federation::SessionPhase::Committed | molten_core::federation::SessionPhase::Denied
            )
    }) {
        return Err("duplicate-active-session".to_string());
    }
    state.resource_history =
        molten_core::federation::reserve_resources(&state.resource_history, &session.plan, state.bounds, now)
            .map_err(|issue| format!("{issue:?}"))?;
    state.last_query = Some(now);
    state.session = Some(session);
    Ok("planned".to_string())
}
fn transition(
    state: &mut Replay<'_>,
    event: &molten_core::federation::SessionEvent,
    generation: u64,
    now: u64,
) -> Result<String, String> {
    let session = state.session.as_ref().ok_or_else(|| "unknown-session".to_string())?;
    let next = molten_core::federation::advance_session(session, event, generation, now, state.bounds)
        .map_err(|issue| format!("{issue:?}"))?;
    if matches!(event, molten_core::federation::SessionEvent::BeginFetch) && next.attempts > 1 {
        state.resource_history =
            molten_core::federation::reserve_resources(&state.resource_history, &next.plan, state.bounds, now)
                .map_err(|issue| format!("{issue:?}"))?;
    }
    state.session = Some(next);
    Ok("advanced".to_string())
}

fn input_value(input: &Input) -> preserves::IOValue {
    match input {
        Input::Inventory { record, now, requested } => crate::preserves_rail::record("inventory-input", vec![
            crate::preserves_rail::string(&record.verification().outcome_ref),
            crate::preserves_rail::u64_value(*now),
            crate::preserves_rail::bool_value(*requested),
        ]),
        Input::Resume => crate::preserves_rail::record("resume-input", Vec::new()),
        Input::Transition { event, generation, now } => crate::preserves_rail::record("transition-input", vec![
            crate::preserves_rail::u64_value(*generation),
            crate::preserves_rail::u64_value(*now),
            event_value(event),
        ]),
    }
}
fn event_value(event: &molten_core::federation::SessionEvent) -> preserves::IOValue {
    let (kind, references): (&str, Vec<&str>) = match event {
        molten_core::federation::SessionEvent::BeginFetch => ("begin-fetch", Vec::new()),
        molten_core::federation::SessionEvent::Verified { head_ref, evidence_ref } => {
            ("verified", vec![head_ref, evidence_ref])
        }
        molten_core::federation::SessionEvent::Committed { head_ref, evidence_ref } => {
            ("committed", vec![head_ref, evidence_ref])
        }
        molten_core::federation::SessionEvent::Denied { evidence_ref } => ("denied", vec![evidence_ref]),
        molten_core::federation::SessionEvent::Admitted {
            head_ref,
            prerequisites,
        } => ("admitted", vec![
            head_ref,
            &prerequisites.schema_ref,
            &prerequisites.policy_ref,
            &prerequisites.capability_ref,
            &prerequisites.provenance_ref,
            &prerequisites.resource_ref,
            &prerequisites.retention_ref,
            &prerequisites.import_ref,
        ]),
    };
    crate::preserves_rail::record(
        kind,
        references
            .into_iter()
            .map(|reference| {
                crate::preserves_rail::string(if molten_core::federation::canonical_ref(reference) {
                    reference
                } else {
                    "invalid-reference"
                })
            })
            .collect(),
    )
}

/// Exact equality at the production inventory-plan boundary, not merely an allowed/denied label.
pub fn compare_live_plan(
    template: &Replay<'_>,
    record: &super::envelope::network::VerifiedRecord,
    now: u64,
    live: &molten_core::federation::Session,
) -> crate::error::Result<preserves::IOValue> {
    let mut state = (*template).clone();
    let trace = replay(&mut state, vec![Input::Inventory {
        record: Box::new(record.clone()),
        now,
        requested: true,
    }])?;
    if state.session.as_ref() != Some(live) {
        return Err(crate::error::Failure::invalid_harness("live and deterministic federation session plans differ"));
    }
    let status = super::lifecycle::status::session_value(live)?;
    Ok(crate::preserves_rail::record("federation-live-plan-comparison-v1", vec![
        crate::preserves_rail::string(&live.session_ref),
        crate::preserves_rail::string(crate::preserves_rail::canonical_hash(&trace)?),
        crate::preserves_rail::string(crate::preserves_rail::canonical_hash(&status)?),
        crate::preserves_rail::string(
            "equal production plan/session/status projection; not content or import completion evidence",
        ),
    ]))
}

/// Events must be independently derived from actual DAG, local gate, and import receipts.
pub struct RecordedRun<'a> {
    pub template: Replay<'a>,
    pub inventory: super::envelope::network::VerifiedRecord,
    pub events: Vec<molten_core::federation::SessionEvent>,
    pub now: u64,
    pub live: molten_core::federation::Session,
}

pub fn compare_recorded_run(run: RecordedRun<'_>) -> crate::error::Result<preserves::IOValue> {
    let invalid =
        || crate::error::Failure::invalid_harness("recorded federation terminal trace differs from live effects");
    if run.live.phase != molten_core::federation::SessionPhase::Committed
        || run.events.is_empty()
        || run.events.len() >= molten_core::federation::MAX_FEDERATION_ENTRIES
    {
        return Err(invalid());
    }
    let guard = run
        .events
        .iter()
        .rev()
        .find(|event| matches!(event, molten_core::federation::SessionEvent::Committed { .. }))
        .cloned()
        .ok_or_else(invalid)?;
    let mut state = run.template;
    let mut inputs = Vec::with_capacity(run.events.len().saturating_add(1));
    inputs.push(Input::Inventory {
        record: Box::new(run.inventory),
        now: run.now,
        requested: true,
    });
    inputs.extend(run.events.into_iter().map(|event| Input::Transition {
        event,
        generation: state.generation,
        now: run.now,
    }));
    let trace = replay(&mut state, inputs)?;
    let fields = trace.collect_simple_record("federation-deterministic-trace-v1", Some(3)).ok_or_else(invalid)?;
    for event in fields[1].collect_sequence().ok_or_else(invalid)?.iter() {
        let parts = event.collect_simple_record("federation-replay-event-v1", Some(5)).ok_or_else(invalid)?;
        if parts[1].as_string().as_deref() != Some("accepted") {
            return Err(invalid());
        }
    }
    if state.session.as_ref() != Some(&run.live) {
        return Err(invalid());
    }
    let status = super::lifecycle::status::session_value(state.session.as_ref().ok_or_else(invalid)?)?;
    let live_status = super::lifecycle::status::session_value(&run.live)?;
    if crate::preserves_rail::canonical_bytes(&status)? != crate::preserves_rail::canonical_bytes(&live_status)? {
        return Err(invalid());
    }
    let terminal = state.session.clone();
    let resources = state.resource_history.clone();
    if transition(&mut state, &guard, run.live.generation, run.now).err().as_deref() != Some("Terminal")
        || state.session != terminal
        || state.resource_history != resources
    {
        return Err(invalid());
    }
    Ok(crate::preserves_rail::record("federation-recorded-terminal-comparison-v1", vec![
        crate::preserves_rail::string(&run.live.session_ref),
        trace,
        crate::preserves_rail::string(crate::preserves_rail::canonical_hash(&status)?),
        crate::preserves_rail::string(
            "recorded live effects replay identically; duplicate terminal commit denied without mutation",
        ),
    ]))
}

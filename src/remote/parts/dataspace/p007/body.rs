#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClosedRemoteSession {
    pub session_ref: String,
    pub owner: String,
    pub retraction_events: Vec<RuntimeEvent>,
    pub cleanup: RuntimeScopeCleanup,
    pub cleanup_decision: String,
    pub cleanup_receipt_value: IoValue,
    pub cleanup_receipt_ref: String,
    pub before_state_ref: String,
    pub after_state_ref: String,
}

pub fn close_remote_session(
    state: &mut RuntimeState,
    sessions: &mut RemoteSessionRegistry,
    session_ref: &str,
    cause: &str,
) -> Result<ClosedRemoteSession> {
    let session = sessions
        .session(session_ref)
        .ok_or_else(|| MoltenError::invalid_harness(format!("unknown remote dataspace session {session_ref}")))?
        .clone();
    let owner = session.owner.clone();
    let before = state.snapshot();
    let before_state_ref = before.snapshot_ref()?;
    let mut retracted_assertion_refs = Vec::new();
    let mut retraction_events = Vec::new();
    if session.state == RemoteSessionState::Open {
        let owned: Vec<crate::runtime::RuntimeAssertion> = before
            .assertions
            .iter()
            .filter(|assertion| assertion.actor == owner)
            .cloned()
            .collect();
        ensure_count_at_most(owned.len(), MAX_SESSION_RETRACTIONS, "session retraction values")?;
        retracted_assertion_refs = owned
            .iter()
            .map(|assertion| assertion.assertion_ref())
            .collect::<Result<Vec<_>>>()?;
        for assertion in owned {
            extend_bounded(
                &mut retraction_events,
                state.apply_step(&RuntimeStep::Retract {
                    actor: owner.clone(),
                    value: assertion.value,
                }),
                MAX_SESSION_RETRACTIONS,
                "session retraction events",
            )?;
        }
        retracted_assertion_refs.sort();
    }
    let mut cleanup = state.cleanup_actor_scope(&owner)?;
    cleanup.assertion_refs.extend(retracted_assertion_refs);
    cleanup.assertion_refs.sort();
    cleanup.assertion_refs.dedup();
    sessions.mark_closed(session_ref);
    let after = state.snapshot();
    let after_state_ref = after.snapshot_ref()?;
    let cleanup_receipt = crate::lifecycle::scope_cleanup_receipt(&crate::lifecycle::ScopeCleanupInput {
        entity_kind: crate::lifecycle::EntityKind::Session,
        entity_id: &owner,
        cause,
        before: &before,
        after_cleanup: &after,
        cleanup: &cleanup,
        live_ref_refs: &[session_ref.to_owned()],
        resource_refs: &[],
        evidence_refs: &[],
        logical_step: before.logical_time,
    })?;
    Ok(ClosedRemoteSession {
        session_ref: session_ref.to_owned(),
        owner,
        retraction_events,
        cleanup,
        cleanup_decision: cleanup_receipt.decision.clone(),
        cleanup_receipt_value: cleanup_receipt.value,
        cleanup_receipt_ref: cleanup_receipt.receipt_ref,
        before_state_ref,
        after_state_ref,
    })
}

pub fn close_remote_session_for_disconnect(
    state: &mut RuntimeState,
    sessions: &mut RemoteSessionRegistry,
    session_ref: &str,
) -> Result<ClosedRemoteSession> {
    close_remote_session(state, sessions, session_ref, SESSION_CLOSE_CAUSE_DISCONNECT)
}

pub fn close_remote_session_for_stop(
    state: &mut RuntimeState,
    sessions: &mut RemoteSessionRegistry,
    session_ref: &str,
) -> Result<ClosedRemoteSession> {
    close_remote_session(state, sessions, session_ref, SESSION_CLOSE_CAUSE_STOP)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionReplayOutcome {
    pub events: Vec<RuntimeEvent>,
    pub diagnostics: Vec<String>,
    pub is_diagnostic_only: bool,
}

pub fn replay_delivery_log_for_session(
    state: &mut RuntimeState,
    sessions: &RemoteSessionRegistry,
    session_ref: &str,
    log: &DeliveryLog,
) -> Result<SessionReplayOutcome> {
    if !log.replayable {
        return Err(MoltenError::invalid_harness(
            "remote dataspace delivery log is non-replayable and cannot satisfy deterministic replay",
        ));
    }
    let owner = match sessions.session(session_ref) {
        None => {
            return Ok(SessionReplayOutcome {
                events: Vec::new(),
                diagnostics: vec![format!(
                    "declared owner session is unknown: {session_ref}; replay stays diagnostic and the peer must re-assert"
                )],
                is_diagnostic_only: true,
            });
        }
        Some(session) if session.state == RemoteSessionState::Closed => {
            return Ok(SessionReplayOutcome {
                events: Vec::new(),
                diagnostics: vec![format!(
                    "declared owner session is closed: {session_ref}; replay stays diagnostic and the peer must re-assert"
                )],
                is_diagnostic_only: true,
            });
        }
        Some(session) => session.owner.clone(),
    };
    let mut events = Vec::new();
    for delivery in &log.entries {
        let delivered = apply_delivered_envelope_owned_by(state, &delivery.envelope, &owner)?;
        extend_bounded(&mut events, delivered, MAX_REPLAY_EVENTS, "remote replay events")?;
    }
    Ok(SessionReplayOutcome {
        events,
        diagnostics: Vec::new(),
        is_diagnostic_only: false,
    })
}

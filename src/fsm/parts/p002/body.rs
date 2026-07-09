fn transition_view(input: &TransitionInput) -> TransitionView<'_> {
    TransitionView {
        prior_state: input.prior.state,
        event: input.event,
        target: input.target,
        prior_topic: &input.prior.topic,
        observed_topic: &input.observed_topic,
        at_tick: input.at_tick,
        bootstrap_refs: &input.prior.bootstrap_refs,
        authority_refs: &input.prior.authority_refs,
        required_bootstrap_ref: input.required_bootstrap_ref.as_ref(),
        required_authority_ref: input.required_authority_ref.as_ref(),
        required_recovery_ref: input.required_recovery_ref.as_ref(),
        revocation_ref: input.revocation_ref.as_ref(),
    }
}

fn collect_transition_diagnostics(
    input: TransitionView<'_>,
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) -> Result<()> {
    if input.observed_topic != input.prior_topic {
        push_peer_diagnostic(diagnostics, "wrong topic for peer session")?;
    }
    if !transition_allowed(input.prior_state, input.event, input.target) {
        push_peer_diagnostic(diagnostics, "peer session transition is not in reviewed table")?;
    }
    if is_terminal_or_quarantined(input.prior_state) && input.event != EventKind::Recover {
        push_peer_diagnostic(diagnostics, "terminal or quarantined peer session requires recovery transition")?;
    }
    if input.event == EventKind::Admit && missing(input.required_bootstrap_ref, input.bootstrap_refs) {
        push_peer_diagnostic(diagnostics, "missing bootstrap admission")?;
    }
    if input.event == EventKind::Connect && missing(input.required_authority_ref, input.authority_refs) {
        push_peer_diagnostic(diagnostics, "missing authority grant")?;
    }
    if input.event == EventKind::Connect && input.required_authority_ref.is_none() {
        push_peer_diagnostic(diagnostics, "transport observation is not peer authority")?;
    }
    if input.event == EventKind::Expire && input.at_tick == 0 {
        push_peer_diagnostic(diagnostics, "stale ticket requires nonzero expiry tick")?;
    }
    if input.event == EventKind::Revoke && input.revocation_ref.is_none() {
        push_peer_diagnostic(diagnostics, "revocation transition requires revocation evidence")?;
    }
    if input.event == EventKind::Recover && input.required_recovery_ref.is_none() {
        push_peer_diagnostic(diagnostics, "recovery transition requires recovery evidence")?;
    }
    Ok(())
}

fn collect_transition_guard_refs(
    input: TransitionView<'_>,
    guard_refs: &mut impl crate::bounded::VecSink<String>,
) -> Result<()> {
    if let Some(reference) = input.required_bootstrap_ref {
        push_peer_guard_ref(guard_refs, reference)?;
    }
    if let Some(reference) = input.required_authority_ref {
        push_peer_guard_ref(guard_refs, reference)?;
    }
    if let Some(reference) = input.required_recovery_ref {
        push_peer_guard_ref(guard_refs, reference)?;
    }
    if let Some(reference) = input.revocation_ref {
        push_peer_guard_ref(guard_refs, reference)?;
    }
    Ok(())
}

fn transition_allowed(
    prior: StateKind,
    event: EventKind,
    target: StateKind,
) -> bool {
    matches!(
        (prior, event, target),
        (StateKind::Discovered, EventKind::Invite, StateKind::Invited)
            | (
                StateKind::Invited,
                EventKind::HandshakeStart,
                StateKind::Handshaking
            )
            | (
                StateKind::Handshaking,
                EventKind::NegotiationPass,
                StateKind::Negotiated
            )
            | (StateKind::Negotiated, EventKind::Admit, StateKind::Admitted)
            | (StateKind::Admitted, EventKind::Connect, StateKind::Connected)
            | (StateKind::Connected, EventKind::Expire, StateKind::Expired)
            | (StateKind::Connected, EventKind::Revoke, StateKind::Revoked)
            | (_, EventKind::Quarantine, StateKind::Quarantined)
            | (StateKind::Expired, EventKind::Recover, StateKind::Invited)
            | (StateKind::Quarantined, EventKind::Recover, StateKind::Invited)
    )
}

fn is_terminal_or_quarantined(state: StateKind) -> bool {
    matches!(
        state,
        StateKind::Expired | StateKind::Revoked | StateKind::Quarantined
    )
}

fn transition_decision_value(decision: &TransitionDecision) -> IoValue {
    crate::preserves_rail::record("peer-session-transition-receipt-v1", vec![
        string(PEER_TRANSITION_SCHEMA),
        field("decision", &decision.decision),
        field("peer-ref", &decision.session.peer_ref),
        field("session-ref", &decision.session.session_ref),
        field("prior-state", decision.prior_state.as_str()),
        field("event", decision.event.as_str()),
        field("target-state", decision.target_state.as_str()),
        field("next-state", decision.next_state.as_str()),
        field("before-state", &decision.before_state_ref),
        field("after-or-preserved-state", &decision.after_state_ref),
        list_field("guard-refs", &decision.guard_refs),
        list_field("diagnostics", &decision.diagnostics),
        field("evidence-only", "transport-does-not-grant-authority"),
        checks_value(&[
            "reviewed-transition-table",
            "deny-preserves-prior-state",
            "transport-not-authority",
        ]),
    ])
}

fn transition_value(decision: &str, session: &Record, diagnostics: &[String]) -> IoValue {
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

fn checks_value(checks: &[&str]) -> IoValue {
    crate::preserves_rail::record("checks", vec![crate::preserves_rail::sequence(
        checks
            .iter()
            .map(|name| crate::preserves_rail::record("check", vec![string(name), string("pass")]))
            .collect(),
    )])
}

fn push_peer_diagnostic(diagnostics: &mut impl crate::bounded::VecSink<String>, diagnostic: &str) -> Result<()> {
    crate::bounded::push_bounded(
        diagnostics,
        diagnostic.to_string(),
        MAX_PEER_SESSION_DIAGNOSTICS,
        "peer session diagnostics",
    )
}

fn push_peer_guard_ref(guard_refs: &mut impl crate::bounded::VecSink<String>, reference: &str) -> Result<()> {
    crate::preserves_rail::validate_content_ref(reference)?;
    crate::bounded::push_bounded(
        guard_refs,
        reference.to_string(),
        MAX_PEER_SESSION_GUARD_REFS,
        "peer session guard refs",
    )
}

fn string(value: impl AsRef<str>) -> IoValue {
    crate::preserves_rail::string(value)
}

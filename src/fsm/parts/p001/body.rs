pub fn profile_value(profile: &Profile) -> IoValue {
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

pub fn record_value(session: &Record) -> IoValue {
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

pub fn apply_transition(input: &TransitionInput) -> Result<TransitionReceipt> {
    let decision = transition_decision(input)?;
    let value = transition_decision_value(&decision);
    let receipt_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(TransitionReceipt {
        decision: decision.decision,
        session: decision.session,
        diagnostics: decision.diagnostics,
        value,
        receipt_ref,
    })
}

pub fn transition_decision(input: &TransitionInput) -> Result<TransitionDecision> {
    let view = transition_view(input);
    let mut diagnostics = Vec::new();
    let mut guard_refs = Vec::new();
    collect_transition_guard_refs(view, &mut guard_refs)?;
    collect_transition_diagnostics(view, &mut diagnostics)?;
    let is_pass = diagnostics.is_empty();
    let next_state = if is_pass { input.target } else { input.prior.state };
    let mut session = input.prior.clone();
    session.state = next_state;
    session.diagnostics = diagnostics.clone();
    let before_state_ref = crate::preserves_rail::canonical_hash(&record_value(&input.prior))?;
    let after_state_ref = crate::preserves_rail::canonical_hash(&record_value(&session))?;
    Ok(TransitionDecision {
        decision: if is_pass { "pass" } else { "deny" }.to_string(),
        prior_state: input.prior.state,
        event: input.event,
        target_state: input.target,
        next_state,
        before_state_ref,
        after_state_ref,
        guard_refs,
        diagnostics,
        session,
    })
}

pub fn record_as_authority_denial(session_ref: &str, operation: &str) -> Result<TransitionReceipt> {
    crate::preserves_rail::validate_content_ref(session_ref)?;
    let diagnostics = vec![format!(
        "peer session {session_ref} is transport state, not authority for {operation}"
    )];
    let session = Record {
        peer_ref: session_ref.to_string(),
        session_ref: session_ref.to_string(),
        topic: operation.to_string(),
        state: StateKind::Quarantined,
        bootstrap_refs: Vec::new(),
        capability_refs: Vec::new(),
        authority_refs: Vec::new(),
        policy_refs: Vec::new(),
        resource_refs: Vec::new(),
        diagnostics: diagnostics.clone(),
    };
    let value = transition_value("deny", &session, &diagnostics);
    let receipt_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(TransitionReceipt {
        decision: "deny".to_string(),
        session,
        diagnostics,
        value,
        receipt_ref,
    })
}

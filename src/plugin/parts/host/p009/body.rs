
fn plugin_lifecycle_guard_refs(guards: &PluginLifecycleGuardSnapshot<'_>) -> Result<Vec<String>> {
    let mut refs = Vec::new();
    for reference in [
        guards.install_ref,
        guards.permission_ref,
        guards.activation_ref,
        guards.hostcall_ref,
        guards.health_ref,
        guards.removal_ref,
        guards.upgrade_ref,
        guards.negotiation_ref,
        guards.compatibility_ref,
        guards.recovery_ref,
    ]
    .into_iter()
    .flatten()
    {
        validate_ref(reference, "plugin lifecycle FSM guard ref")?;
        refs.push_limited(reference.to_string(), MAX_PLUGIN_REFS, "plugin lifecycle FSM guard refs")?;
    }
    Ok(refs)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PluginLifecycleFsmDecisionValueInput<'a> {
    decision: &'a str,
    prior_state: PluginLifecycleState,
    event: PluginLifecycleEvent,
    next_state: PluginLifecycleState,
    manifest_ref: &'a str,
    guard_refs: &'a [String],
    side_effect_class: &'a str,
    authority_closed: bool,
    diagnostics: &'a [String],
}

fn plugin_lifecycle_fsm_decision_value(input: PluginLifecycleFsmDecisionValueInput<'_>) -> Result<IoValue> {
    validate_ref(input.manifest_ref, "plugin lifecycle FSM manifest ref")?;
    validate_refs(input.guard_refs, "plugin lifecycle FSM guard ref")?;
    Ok(record("plugin-lifecycle-fsm-decision-v1", vec![
        string(PLUGIN_LIFECYCLE_FSM_DECISION_SCHEMA),
        record("decision", vec![string(input.decision)]),
        record("prior-state", vec![string(input.prior_state.as_str())]),
        record("event", vec![string(input.event.as_str())]),
        record("next-state", vec![string(input.next_state.as_str())]),
        record("manifest", vec![string(input.manifest_ref)]),
        record("guards", vec![refs_sequence(input.guard_refs)]),
        record("side-effect", vec![string(input.side_effect_class)]),
        record("authority-closed", vec![bool_value(input.authority_closed)]),
        record("diagnostics", vec![strings_sequence(input.diagnostics)]),
        checks_value(&[
            ("reviewed-transition-table", PLUGIN_DECISION_PASS),
            ("explicit-guard-facts", PLUGIN_DECISION_PASS),
            ("authority-closure-bound", PLUGIN_DECISION_PASS),
            ("no-ambient-authority", PLUGIN_DECISION_PASS),
        ]),
    ]))
}

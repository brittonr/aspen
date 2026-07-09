
const PLUGIN_LIFECYCLE_FSM_DECISION_SCHEMA: &str = "molten.plugin.lifecycle-fsm-decision.v1";
const PLUGIN_LIFECYCLE_SIDE_EFFECT_NONE: &str = "none";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginLifecycleState {
    ManifestDeclared,
    Installed,
    Permitted,
    Active,
    Healthy,
    Degraded,
    Removed,
    Upgraded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginLifecycleEvent {
    CompleteTrace,
    Activate,
    Hostcall,
    Upgrade,
    Remove,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PluginLifecycleGuardBooleans {
    install_passes: bool,
    permission_passes: bool,
    activation_passes: bool,
    hostcall_passes: bool,
    health_passes: bool,
    removal_passes: bool,
    upgrade_passes: bool,
    negotiation_passes: bool,
    compatibility_passes: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PluginLifecycleGuardSnapshot<'a> {
    install_ref: Option<&'a str>,
    permission_ref: Option<&'a str>,
    activation_ref: Option<&'a str>,
    hostcall_ref: Option<&'a str>,
    health_ref: Option<&'a str>,
    removal_ref: Option<&'a str>,
    upgrade_ref: Option<&'a str>,
    negotiation_ref: Option<&'a str>,
    compatibility_ref: Option<&'a str>,
    recovery_ref: Option<&'a str>,
    booleans: PluginLifecycleGuardBooleans,
}

fn plugin_lifecycle_guard_snapshot<'a>(
    input: &'a PluginLifecycleStateInput<'a>,
    booleans: PluginLifecycleGuardBooleans,
) -> PluginLifecycleGuardSnapshot<'a> {
    PluginLifecycleGuardSnapshot {
        install_ref: input.install.map(|receipt| receipt.receipt_ref.as_str()),
        permission_ref: input.permission.map(|receipt| receipt.receipt_ref.as_str()),
        activation_ref: input.activation.map(|receipt| receipt.receipt_ref.as_str()),
        hostcall_ref: input.hostcall.map(|receipt| receipt.receipt_ref.as_str()),
        health_ref: input.health.map(|receipt| receipt.receipt_ref.as_str()),
        removal_ref: input.removal.map(|receipt| receipt.receipt_ref.as_str()),
        upgrade_ref: input.upgrade.map(|receipt| receipt.receipt_ref.as_str()),
        negotiation_ref: input.negotiation.map(|receipt| receipt.receipt_ref.as_str()),
        compatibility_ref: input.compatibility.map(|receipt| receipt.receipt_ref.as_str()),
        recovery_ref: input.recovery_receipt_ref,
        booleans,
    }
}

fn plugin_lifecycle_transition_decision(
    kind: PluginLifecycleEvaluationKind,
    manifest_ref: &str,
    guards: PluginLifecycleGuardSnapshot<'_>,
    diagnostics: Vec<String>,
) -> Result<PluginLifecycleStateDecision> {
    validate_ref(manifest_ref, "plugin lifecycle FSM manifest ref")?;
    let prior_state = plugin_lifecycle_prior_state(kind, &guards);
    let event = plugin_lifecycle_event(kind);
    let is_transition_admitted = plugin_lifecycle_transition_allows(prior_state, event, &guards);
    let decision = if is_transition_admitted && diagnostics.is_empty() {
        PLUGIN_DECISION_PASS
    } else {
        PLUGIN_DECISION_DENY
    };
    let next_state = plugin_lifecycle_next_state(prior_state, event, decision == PLUGIN_DECISION_PASS, &guards);
    let side_effect_class = plugin_lifecycle_side_effect_class(event, decision == PLUGIN_DECISION_PASS).to_string();
    let guard_refs = plugin_lifecycle_guard_refs(&guards)?;
    let is_authority_closed = guards.booleans.removal_passes || next_state == PluginLifecycleState::Removed;
    let value = plugin_lifecycle_fsm_decision_value(PluginLifecycleFsmDecisionValueInput {
        decision,
        prior_state,
        event,
        next_state,
        manifest_ref,
        guard_refs: &guard_refs,
        side_effect_class: &side_effect_class,
        authority_closed: is_authority_closed,
        diagnostics: &diagnostics,
    })?;
    let is_side_effect_authorized = decision == PLUGIN_DECISION_PASS && side_effect_class != PLUGIN_LIFECYCLE_SIDE_EFFECT_NONE;
    Ok(PluginLifecycleStateDecision {
        decision: decision.to_string(),
        diagnostics,
        side_effect_authorized: is_side_effect_authorized,
        authority_closed: is_authority_closed,
        prior_state,
        event,
        next_state,
        guard_refs,
        side_effect_class,
        value,
    })
}

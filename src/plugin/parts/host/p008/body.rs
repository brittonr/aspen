
fn plugin_lifecycle_transition_allows(
    prior_state: PluginLifecycleState,
    event: PluginLifecycleEvent,
    guards: &PluginLifecycleGuardSnapshot<'_>,
) -> bool {
    let is_base_ready = guards.booleans.install_passes && guards.booleans.permission_passes;
    match event {
        PluginLifecycleEvent::CompleteTrace => {
            is_base_ready
                && guards.booleans.activation_passes
                && guards.booleans.hostcall_passes
                && guards.booleans.health_passes
                && guards.booleans.removal_passes
                && guards.booleans.upgrade_passes
                && guards.booleans.negotiation_passes
                && guards.booleans.compatibility_passes
        }
        PluginLifecycleEvent::Activate => {
            matches!(prior_state, PluginLifecycleState::Permitted)
                && guards.booleans.activation_passes
                && guards.booleans.negotiation_passes
        }
        PluginLifecycleEvent::Hostcall => {
            matches!(prior_state, PluginLifecycleState::Active | PluginLifecycleState::Healthy)
                && guards.booleans.hostcall_passes
                && guards.booleans.health_passes
                && guards.booleans.negotiation_passes
                && !guards.booleans.removal_passes
        }
        PluginLifecycleEvent::Upgrade => {
            matches!(prior_state, PluginLifecycleState::Active | PluginLifecycleState::Healthy)
                && guards.booleans.upgrade_passes
                && guards.booleans.health_passes
                && guards.booleans.compatibility_passes
                && !guards.booleans.removal_passes
        }
        PluginLifecycleEvent::Remove => {
            matches!(prior_state, PluginLifecycleState::Active | PluginLifecycleState::Healthy)
                && guards.booleans.removal_passes
        }
    }
}

fn plugin_lifecycle_prior_state(
    kind: PluginLifecycleEvaluationKind,
    guards: &PluginLifecycleGuardSnapshot<'_>,
) -> PluginLifecycleState {
    match kind {
        PluginLifecycleEvaluationKind::CompleteTrace => PluginLifecycleState::ManifestDeclared,
        PluginLifecycleEvaluationKind::ActivationRequest => plugin_lifecycle_pre_activation_state(guards),
        PluginLifecycleEvaluationKind::HostcallRequest => plugin_lifecycle_active_state(guards),
        PluginLifecycleEvaluationKind::UpgradeRequest => plugin_lifecycle_active_state(guards),
        PluginLifecycleEvaluationKind::RemovalRequest => plugin_lifecycle_active_state(guards),
    }
}

fn plugin_lifecycle_pre_activation_state(guards: &PluginLifecycleGuardSnapshot<'_>) -> PluginLifecycleState {
    if guards.booleans.permission_passes {
        return PluginLifecycleState::Permitted;
    }
    if guards.booleans.install_passes {
        return PluginLifecycleState::Installed;
    }
    PluginLifecycleState::ManifestDeclared
}

fn plugin_lifecycle_active_state(guards: &PluginLifecycleGuardSnapshot<'_>) -> PluginLifecycleState {
    if guards.booleans.removal_passes {
        return PluginLifecycleState::Removed;
    }
    if guards.booleans.health_passes {
        return PluginLifecycleState::Healthy;
    }
    if guards.health_ref.is_some() {
        return PluginLifecycleState::Degraded;
    }
    if guards.booleans.activation_passes {
        return PluginLifecycleState::Active;
    }
    plugin_lifecycle_pre_activation_state(guards)
}

fn plugin_lifecycle_event(kind: PluginLifecycleEvaluationKind) -> PluginLifecycleEvent {
    match kind {
        PluginLifecycleEvaluationKind::CompleteTrace => PluginLifecycleEvent::CompleteTrace,
        PluginLifecycleEvaluationKind::ActivationRequest => PluginLifecycleEvent::Activate,
        PluginLifecycleEvaluationKind::HostcallRequest => PluginLifecycleEvent::Hostcall,
        PluginLifecycleEvaluationKind::UpgradeRequest => PluginLifecycleEvent::Upgrade,
        PluginLifecycleEvaluationKind::RemovalRequest => PluginLifecycleEvent::Remove,
    }
}

fn plugin_lifecycle_next_state(
    prior_state: PluginLifecycleState,
    event: PluginLifecycleEvent,
    transition_passes: bool,
    guards: &PluginLifecycleGuardSnapshot<'_>,
) -> PluginLifecycleState {
    if !transition_passes {
        return prior_state;
    }
    match event {
        PluginLifecycleEvent::CompleteTrace => plugin_lifecycle_complete_trace_state(prior_state, guards),
        PluginLifecycleEvent::Activate => PluginLifecycleState::Active,
        PluginLifecycleEvent::Hostcall => prior_state,
        PluginLifecycleEvent::Upgrade => PluginLifecycleState::Upgraded,
        PluginLifecycleEvent::Remove => PluginLifecycleState::Removed,
    }
}

fn plugin_lifecycle_complete_trace_state(
    prior_state: PluginLifecycleState,
    guards: &PluginLifecycleGuardSnapshot<'_>,
) -> PluginLifecycleState {
    if guards.booleans.upgrade_passes {
        return PluginLifecycleState::Upgraded;
    }
    if guards.booleans.removal_passes {
        return PluginLifecycleState::Removed;
    }
    prior_state
}

fn plugin_lifecycle_side_effect_class(event: PluginLifecycleEvent, transition_passes: bool) -> &'static str {
    if !transition_passes {
        return PLUGIN_LIFECYCLE_SIDE_EFFECT_NONE;
    }
    match event {
        PluginLifecycleEvent::CompleteTrace => "trace-replay",
        PluginLifecycleEvent::Activate => "plugin-activation",
        PluginLifecycleEvent::Hostcall => "hostcall",
        PluginLifecycleEvent::Upgrade => "upgrade-cutover",
        PluginLifecycleEvent::Remove => "authority-closing-removal",
    }
}

impl PluginLifecycleState {
    fn as_str(self) -> &'static str {
        match self {
            PluginLifecycleState::ManifestDeclared => "manifest-declared",
            PluginLifecycleState::Installed => "installed",
            PluginLifecycleState::Permitted => "permitted",
            PluginLifecycleState::Active => "active",
            PluginLifecycleState::Healthy => "healthy",
            PluginLifecycleState::Degraded => "degraded",
            PluginLifecycleState::Removed => "removed",
            PluginLifecycleState::Upgraded => "upgraded",
        }
    }
}

impl PluginLifecycleEvent {
    fn as_str(self) -> &'static str {
        match self {
            PluginLifecycleEvent::CompleteTrace => "complete-trace",
            PluginLifecycleEvent::Activate => "activate",
            PluginLifecycleEvent::Hostcall => "hostcall",
            PluginLifecycleEvent::Upgrade => "upgrade",
            PluginLifecycleEvent::Remove => "remove",
        }
    }
}

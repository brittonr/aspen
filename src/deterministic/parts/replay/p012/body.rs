fn first_divergence(
    expected: &ReplayRunParts,
    actual: &ReplayRunParts,
    variant: ReplayFixtureVariant,
) -> ReplayDivergenceKind {
    if variant == ReplayFixtureVariant::MissingRecordedEffect {
        return ReplayDivergenceKind::LiveEffect;
    }
    first_divergence_between_parts(expected, actual)
}

fn first_divergence_between_parts(expected: &ReplayRunParts, actual: &ReplayRunParts) -> ReplayDivergenceKind {
    if expected.identity_ref != actual.identity_ref {
        return ReplayDivergenceKind::Identity;
    }
    if expected.scheduler_ref != actual.scheduler_ref {
        return ReplayDivergenceKind::Scheduler;
    }
    if expected.input_ref != actual.input_ref {
        return ReplayDivergenceKind::Input;
    }
    if expected.effect_request_ref != actual.effect_request_ref {
        return ReplayDivergenceKind::EffectRequest;
    }
    if expected.effect_response_ref != actual.effect_response_ref {
        return ReplayDivergenceKind::EffectResponse;
    }
    if expected.policy_decision_ref != actual.policy_decision_ref {
        return ReplayDivergenceKind::PolicyDecision;
    }
    if expected.action_ref != actual.action_ref {
        return ReplayDivergenceKind::Action;
    }
    if expected.receipt_ref != actual.receipt_ref {
        return ReplayDivergenceKind::Receipt;
    }
    if expected.output_ref != actual.output_ref {
        return ReplayDivergenceKind::Output;
    }
    if expected.after_state_ref != actual.after_state_ref {
        return ReplayDivergenceKind::StateHash;
    }
    ReplayDivergenceKind::None
}

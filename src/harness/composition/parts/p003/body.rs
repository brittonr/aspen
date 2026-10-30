// The property evaluation for the service recovery composition.
//
// Each scenario asserts the properties that apply to it, so a regression in
// one mechanism fails the scenarios that depend on it.

/// The outcome of one property in one scenario.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropertyOutcome {
    pub property: ServiceProperty,
    pub holds: bool,
    pub detail: String,
}

/// Evaluate the properties that apply to one scenario.
pub fn evaluate_properties(
    scenario: FailureScenario,
    facts: &ScenarioFacts,
) -> Vec<PropertyOutcome> {
    match scenario {
        FailureScenario::MemberFailureWithDependentRestart => vec![property_outcome(
            ServiceProperty::UnaffectedServicesContinue,
            facts.sibling_progressed,
            "the sibling outside the recovery group kept working",
        )],
        FailureScenario::DelayedPreRestartEventDelivery => vec![property_outcome(
            ServiceProperty::StaleInstancesLoseInfluence,
            !facts.stale_delivery_acted,
            "a delayed pre-restart delivery did not act on the replacement instance",
        )],
        FailureScenario::DurableWorkReadmission => vec![property_outcome(
            ServiceProperty::DurableWorkIsReadmitted,
            facts.durable_work_readmitted && !facts.durable_work_lost,
            "durable work was readmitted under a current claim and nothing was silently lost",
        )],
        FailureScenario::UncertainEffectSurfacing => vec![property_outcome(
            ServiceProperty::UncertainEffectsAreNotRepeated,
            !facts.uncertain_effect_repeated && !facts.durable_work_lost,
            "an uncertain external effect surfaced as unknown and was not blindly repeated",
        )],
        FailureScenario::QueueOverload => vec![property_outcome(
            ServiceProperty::UnaffectedServicesContinue,
            facts.sibling_progressed && facts.overload_denied,
            "the overload was denied explicitly while the sibling kept working",
        )],
        FailureScenario::RestartStormToEscalation => vec![property_outcome(
            ServiceProperty::StormsTerminateWithinWindow,
            facts.storm_terminated,
            "the restart storm reached a terminal outcome inside the admitted window",
        )],
        FailureScenario::CpuSaturationUnderTimeSlicing => vec![property_outcome(
            ServiceProperty::UnaffectedServicesContinue,
            facts.sibling_progressed && facts.saturating_work_bounded,
            "the sibling completed between the saturating callback's slices, and the callback ended within its budget",
        )],
        FailureScenario::GenerationUpgradeWithStaleWorkRejection => generation_upgrade_outcomes(facts),
    }
}

/// The properties asserted by a generation upgrade, in their established order.
fn generation_upgrade_outcomes(facts: &ScenarioFacts) -> Vec<PropertyOutcome> {
    vec![
        property_outcome(
            ServiceProperty::StaleInstancesLoseInfluence,
            facts.upgrade_completed && !facts.stale_generation_acted,
            "the drained/checkpointed/transformed replacement activated and old-generation work did not act",
        ),
        property_outcome(
            ServiceProperty::DurableWorkIsReadmitted,
            facts.durable_work_readmitted && !facts.durable_work_lost,
            "upgraded durable work was explicitly readmitted without losing its logical identity",
        ),
        property_outcome(
            ServiceProperty::UnaffectedServicesContinue,
            facts.sibling_progressed,
            "the independent sibling progressed throughout generation replacement",
        ),
    ]
}

fn property_outcome(property: ServiceProperty, holds: bool, detail: &str) -> PropertyOutcome {
    PropertyOutcome {
        property,
        holds,
        detail: detail.to_string(),
    }
}

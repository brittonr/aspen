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
    let mut outcomes = Vec::new();
    let mut record = |property: ServiceProperty, holds: bool, detail: String| {
        outcomes.push(PropertyOutcome {
            property,
            holds,
            detail,
        });
    };
    match scenario {
        FailureScenario::MemberFailureWithDependentRestart => {
            record(
                ServiceProperty::UnaffectedServicesContinue,
                facts.sibling_progressed,
                "the sibling outside the recovery group kept working".to_string(),
            );
        }
        FailureScenario::DelayedPreRestartEventDelivery => {
            record(
                ServiceProperty::StaleInstancesLoseInfluence,
                !facts.stale_delivery_acted,
                "a delayed pre-restart delivery did not act on the replacement instance".to_string(),
            );
        }
        FailureScenario::DurableWorkReadmission => {
            record(
                ServiceProperty::DurableWorkIsReadmitted,
                facts.durable_work_readmitted && !facts.durable_work_lost,
                "durable work was readmitted under a current claim and nothing was silently lost".to_string(),
            );
        }
        FailureScenario::UncertainEffectSurfacing => {
            record(
                ServiceProperty::UncertainEffectsAreNotRepeated,
                !facts.uncertain_effect_repeated && !facts.durable_work_lost,
                "an uncertain external effect surfaced as unknown and was not blindly repeated".to_string(),
            );
        }
        FailureScenario::QueueOverload => {
            record(
                ServiceProperty::UnaffectedServicesContinue,
                facts.sibling_progressed && facts.overload_denied,
                "the overload was denied explicitly while the sibling kept working".to_string(),
            );
        }
        FailureScenario::RestartStormToEscalation => {
            record(
                ServiceProperty::StormsTerminateWithinWindow,
                facts.storm_terminated,
                "the restart storm reached a terminal outcome inside the admitted window".to_string(),
            );
        }
        FailureScenario::CpuSaturationUnderTimeSlicing => {
            record(
                ServiceProperty::UnaffectedServicesContinue,
                facts.sibling_progressed && facts.saturating_work_bounded,
                "the sibling completed between the saturating callback's slices, and the callback ended within its budget"
                    .to_string(),
            );
        }
        FailureScenario::GenerationUpgradeWithStaleWorkRejection => {
            record(
                ServiceProperty::StaleInstancesLoseInfluence,
                !facts.stale_generation_acted,
                "work from the previous generation did not act after the upgrade".to_string(),
            );
        }
    }
    outcomes
}

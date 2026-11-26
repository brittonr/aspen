//! The service roles, the asserted properties, and the property evaluation.
//!
//! Each scenario asserts the properties that apply to it, so a regression in
//! one mechanism fails the scenarios that depend on it.

use alloc::string::ToString;

/// The three services in the composition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ServiceRole {
    /// The durable stateful service the dependent worker needs.
    Stateful,
    /// The dependent worker, in the same recovery group and ordered after the
    /// stateful service.
    DependentWorker,
    /// The independent sibling, outside the recovery group.
    Sibling,
}

impl ServiceRole {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Stateful => "stateful",
            Self::DependentWorker => "dependent-worker",
            Self::Sibling => "sibling",
        }
    }
}

/// The properties the composition asserts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ServiceProperty {
    /// Unaffected services continue while a member fails and recovers.
    UnaffectedServicesContinue,
    /// Stale instances lose influence.
    StaleInstancesLoseInfluence,
    /// Durable logical work is not silently lost and is readmitted under a
    /// current claim.
    DurableWorkIsReadmitted,
    /// Uncertain external effects are surfaced as unknown outcomes and not
    /// blindly repeated.
    UncertainEffectsAreNotRepeated,
    /// Restart storms terminate or escalate within the admitted window.
    StormsTerminateWithinWindow,
}

impl ServiceProperty {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::UnaffectedServicesContinue => "unaffected-services-continue",
            Self::StaleInstancesLoseInfluence => "stale-instances-lose-influence",
            Self::DurableWorkIsReadmitted => "durable-work-is-readmitted",
            Self::UncertainEffectsAreNotRepeated => "uncertain-effects-are-not-repeated",
            Self::StormsTerminateWithinWindow => "storms-terminate-within-window",
        }
    }
}

/// The outcome of one property in one scenario.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropertyOutcome {
    pub property: ServiceProperty,
    pub holds: bool,
    pub detail: alloc::string::String,
}

/// Evaluate the properties that apply to one scenario.
// r[impl molten.testing.service_recovery_composition]
pub fn evaluate_properties(
    scenario: crate::vocabulary::FailureScenario,
    facts: &crate::vocabulary::ScenarioFacts,
) -> alloc::vec::Vec<PropertyOutcome> {
    match scenario {
        crate::vocabulary::FailureScenario::MemberFailureWithDependentRestart => alloc::vec![property_outcome(
            ServiceProperty::UnaffectedServicesContinue,
            facts.sibling_progressed,
            "the sibling outside the recovery group kept working",
        )],
        crate::vocabulary::FailureScenario::DelayedPreRestartEventDelivery => alloc::vec![property_outcome(
            ServiceProperty::StaleInstancesLoseInfluence,
            !facts.stale_delivery_acted,
            "a delayed pre-restart delivery did not act on the replacement instance",
        )],
        crate::vocabulary::FailureScenario::DurableWorkReadmission => alloc::vec![property_outcome(
            ServiceProperty::DurableWorkIsReadmitted,
            facts.durable_work_readmitted && !facts.durable_work_lost,
            "durable work was readmitted under a current claim and nothing was silently lost",
        )],
        crate::vocabulary::FailureScenario::UncertainEffectSurfacing => alloc::vec![property_outcome(
            ServiceProperty::UncertainEffectsAreNotRepeated,
            !facts.uncertain_effect_repeated && !facts.durable_work_lost,
            "an uncertain external effect surfaced as unknown and was not blindly repeated",
        )],
        crate::vocabulary::FailureScenario::QueueOverload => alloc::vec![property_outcome(
            ServiceProperty::UnaffectedServicesContinue,
            facts.sibling_progressed && facts.overload_denied,
            "the overload was denied explicitly while the sibling kept working",
        )],
        crate::vocabulary::FailureScenario::RestartStormToEscalation => alloc::vec![property_outcome(
            ServiceProperty::StormsTerminateWithinWindow,
            facts.storm_terminated,
            "the restart storm reached a terminal outcome inside the admitted window",
        )],
        crate::vocabulary::FailureScenario::CpuSaturationUnderTimeSlicing => alloc::vec![property_outcome(
            ServiceProperty::UnaffectedServicesContinue,
            facts.sibling_progressed && facts.saturating_work_bounded,
            "the sibling completed between the saturating callback's slices, and the callback ended within its budget",
        )],
        crate::vocabulary::FailureScenario::GenerationUpgradeWithStaleWorkRejection => {
            generation_upgrade_outcomes(facts)
        }
    }
}

/// The properties asserted by a generation upgrade, in their established order.
fn generation_upgrade_outcomes(facts: &crate::vocabulary::ScenarioFacts) -> alloc::vec::Vec<PropertyOutcome> {
    alloc::vec![
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

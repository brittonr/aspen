//! The scenario vocabulary, the variant vocabulary, and the facts one scenario records.

/// The scenarios the composition runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureScenario {
    MemberFailureWithDependentRestart,
    DelayedPreRestartEventDelivery,
    DurableWorkReadmission,
    UncertainEffectSurfacing,
    QueueOverload,
    RestartStormToEscalation,
    GenerationUpgradeWithStaleWorkRejection,
    CpuSaturationUnderTimeSlicing,
}

impl FailureScenario {
    /// Every scenario, in the order the composition runs them.
    pub const ALL: [Self; 8] = [
        Self::MemberFailureWithDependentRestart,
        Self::DelayedPreRestartEventDelivery,
        Self::DurableWorkReadmission,
        Self::UncertainEffectSurfacing,
        Self::QueueOverload,
        Self::RestartStormToEscalation,
        Self::GenerationUpgradeWithStaleWorkRejection,
        Self::CpuSaturationUnderTimeSlicing,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::MemberFailureWithDependentRestart => "member-failure-with-dependent-restart",
            Self::DelayedPreRestartEventDelivery => "delayed-pre-restart-event-delivery",
            Self::DurableWorkReadmission => "durable-work-readmission",
            Self::UncertainEffectSurfacing => "uncertain-effect-surfacing",
            Self::QueueOverload => "queue-overload",
            Self::RestartStormToEscalation => "restart-storm-to-escalation",
            Self::GenerationUpgradeWithStaleWorkRejection => "generation-upgrade-with-stale-work-rejection",
            Self::CpuSaturationUnderTimeSlicing => "cpu-saturation-under-time-slicing",
        }
    }
}

/// The deliberately broken composition variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScenarioVariant {
    /// The composition as the landed mechanisms behave.
    Correct,
    /// The restart no longer advances the incarnation, so stale work can act.
    FencingRemoved,
    /// The group restart budget is reset on every replacement.
    GroupBudgetReset,
    /// A saturating callback's yields are ignored, so it runs to its end
    /// before the scheduler selects anything else.
    SlicingDisabled,
    /// The replacement activates without materializing the admitted state transformation.
    MigrationSkipped,
}

impl ScenarioVariant {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Correct => "correct",
            Self::FencingRemoved => "fencing-removed",
            Self::GroupBudgetReset => "group-budget-reset",
            Self::SlicingDisabled => "slicing-disabled",
            Self::MigrationSkipped => "migration-skipped",
        }
    }
}

/// What one scenario observed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScenarioFacts {
    pub sibling_progressed: bool,
    pub stale_delivery_acted: bool,
    pub durable_work_readmitted: bool,
    pub durable_work_lost: bool,
    pub uncertain_effect_repeated: bool,
    pub overload_denied: bool,
    pub storm_terminated: bool,
    pub stale_generation_acted: bool,
    pub upgrade_completed: bool,
    pub dependent_restarted_after_dependency: bool,
    pub restarted_member_incarnation_advanced: bool,
    /// A saturating callback ended within its execution budget.
    pub saturating_work_bounded: bool,
}

impl ScenarioFacts {
    /// The empty observation set: every property is unproven until a scenario
    /// records it.
    pub fn empty() -> Self {
        Self {
            sibling_progressed: false,
            stale_delivery_acted: false,
            durable_work_readmitted: false,
            durable_work_lost: false,
            uncertain_effect_repeated: false,
            overload_denied: false,
            storm_terminated: false,
            stale_generation_acted: false,
            upgrade_completed: false,
            dependent_restarted_after_dependency: false,
            restarted_member_incarnation_advanced: false,
            saturating_work_bounded: false,
        }
    }
}

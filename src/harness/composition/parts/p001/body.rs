// The scenario vocabulary, the variant vocabulary, the facts one scenario
// records, and the shared fixtures for the service recovery composition.


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
}

impl FailureScenario {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::MemberFailureWithDependentRestart => "member-failure-with-dependent-restart",
            Self::DelayedPreRestartEventDelivery => "delayed-pre-restart-event-delivery",
            Self::DurableWorkReadmission => "durable-work-readmission",
            Self::UncertainEffectSurfacing => "uncertain-effect-surfacing",
            Self::QueueOverload => "queue-overload",
            Self::RestartStormToEscalation => "restart-storm-to-escalation",
            Self::GenerationUpgradeWithStaleWorkRejection => "generation-upgrade-with-stale-work-rejection",
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
}

impl ScenarioVariant {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Correct => "correct",
            Self::FencingRemoved => "fencing-removed",
            Self::GroupBudgetReset => "group-budget-reset",
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
    pub dependent_restarted_after_dependency: bool,
    pub restarted_member_incarnation_advanced: bool,
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
            dependent_restarted_after_dependency: false,
            restarted_member_incarnation_advanced: false,
        }
    }
}

pub(crate) const INITIAL_GENERATION: u64 = 1;
pub(crate) const MAX_RESTART_ATTEMPTS: u64 = 2;
pub(crate) const REQUEST_BYTES: u64 = 32;
pub(crate) const RESTART_HISTORY_CAPACITY: usize = 3;
const PROFILE_LIMIT: u64 = 16;
const OPERATION_BYTE_LIMIT: u64 = 128;
const NAMESPACE_BYTE_LIMIT: u64 = 1_024;

/// The idle resource usage every lifecycle transition in the composition uses.
pub(crate) fn idle_usage() -> ResourceUsage {
    ResourceUsage {
        concurrent_callbacks: 0,
        queued_events: 0,
        inflight_bytes: 0,
        open_streams: 0,
        timers: 0,
        effect_requests: 0,
    }
}

pub(crate) fn running_state() -> Result<LifecycleState, crate::error::Failure> {
    let installed = plan_lifecycle_transition(
        &LifecycleState::absent(),
        &LifecycleEvent::simple(LifecycleEventKind::Install, INITIAL_GENERATION, 0),
        idle_usage(),
        MAX_RESTART_ATTEMPTS,
    )
    .map_err(|issues| crate::error::Failure::invalid_harness(format!("install denied: {issues:?}")))?;
    let mut state = installed;
    for kind in [
        LifecycleEventKind::Admit,
        LifecycleEventKind::BeginInitialize,
        LifecycleEventKind::InitializeSucceeded,
        LifecycleEventKind::BeginStart,
        LifecycleEventKind::StartSucceeded,
    ] {
        state = plan_lifecycle_transition(
            &state,
            &LifecycleEvent::simple(kind, state.generation, state.incarnation),
            idle_usage(),
            MAX_RESTART_ATTEMPTS,
        )
        .map_err(|issues| crate::error::Failure::invalid_harness(format!("transition denied: {issues:?}")))?;
    }
    Ok(state)
}

pub(crate) fn failure_event(state: &LifecycleState) -> LifecycleEvent {
    LifecycleEvent {
        kind: LifecycleEventKind::Failure,
        generation: state.generation,
        incarnation: state.incarnation,
        next_generation: None,
        checkpoint_ref: None,
        failure_class: Some(FailureClass::Retryable),
    }
}

pub(crate) fn restart(state: &LifecycleState) -> Result<LifecycleState, crate::error::Failure> {
    let failed = plan_lifecycle_transition(
        state,
        &failure_event(state),
        idle_usage(),
        MAX_RESTART_ATTEMPTS,
    )
    .map_err(|issues| crate::error::Failure::invalid_harness(format!("the member fails: {issues:?}")))?;
    let restarting = plan_lifecycle_transition(
        &failed,
        &LifecycleEvent::simple(LifecycleEventKind::BeginRestart, state.generation, state.incarnation),
        idle_usage(),
        MAX_RESTART_ATTEMPTS,
    )
    .map_err(|issues| crate::error::Failure::invalid_harness(format!("the member restarts: {issues:?}")))?;
    let starting = plan_lifecycle_transition(
        &restarting,
        &LifecycleEvent::simple(LifecycleEventKind::BeginStart, state.generation, restarting.incarnation),
        idle_usage(),
        MAX_RESTART_ATTEMPTS,
    )
    .map_err(|issues| crate::error::Failure::invalid_harness(format!("the member starts: {issues:?}")))?;
    plan_lifecycle_transition(
        &starting,
        &LifecycleEvent::simple(LifecycleEventKind::StartSucceeded, state.generation, starting.incarnation),
        idle_usage(),
        MAX_RESTART_ATTEMPTS,
    )
    .map_err(|issues| crate::error::Failure::invalid_harness(format!("the member runs again: {issues:?}")))
}

pub(crate) fn facade_state(phase: &str, incarnation: u64) -> FacadeRuntimeState {
    FacadeRuntimeState {
        generation: INITIAL_GENERATION,
        incarnation,
        phase: phase.to_string(),
        state_ref: Some(HASH_C.to_string()),
        turn_sequence: 1,
        postponed: Vec::new(),
        completed: false,
    }
}

pub(crate) fn durable_profile() -> molten_core::fabric_durability::DurableStateProfile {
    molten_core::fabric_durability::DurableStateProfile {
        schema: molten_core::fabric_durability::DURABLE_STATE_PROFILE_SCHEMA.to_string(),
        profile_id: molten_core::fabric_durability::DurableAdapterKind::DeterministicSimulation.as_str().to_string(),
        profile_ref: HASH_A.to_string(),
        adapter_kind: molten_core::fabric_durability::DurableAdapterKind::DeterministicSimulation,
        supported_levels: vec![molten_core::fabric_durability::DurabilityLevel::Buffered, molten_core::fabric_durability::DurabilityLevel::ProcessLoss],
        max_namespaces: PROFILE_LIMIT,
        max_log_records: PROFILE_LIMIT,
        max_ordered_entries: PROFILE_LIMIT,
        max_operation_bytes: OPERATION_BYTE_LIMIT,
        max_namespace_bytes: NAMESPACE_BYTE_LIMIT,
        max_batch_operations: PROFILE_LIMIT,
        max_snapshots: PROFILE_LIMIT,
        max_effect_transactions: PROFILE_LIMIT,
        non_claims: molten_core::fabric_durability::REQUIRED_DURABILITY_NON_CLAIMS.to_vec(),
    }
}

pub(crate) fn durable_descriptor() -> molten_core::fabric_durability::DurableNamespaceDescriptor {
    molten_core::fabric_durability::DurableNamespaceDescriptor {
        schema: molten_core::fabric_durability::DURABLE_STATE_NAMESPACE_SCHEMA.to_string(),
        profile_ref: HASH_A.to_string(),
        adapter_id: "composition-adapter".to_string(),
        namespace_id: "composition-namespace".to_string(),
        generation: INITIAL_GENERATION,
        value_schema_ref: HASH_B.to_string(),
        atomicity_domain: molten_core::fabric_durability::AtomicityDomain {
            domain_id: "composition-domain".to_string(),
            adapter_id: "composition-adapter".to_string(),
            namespace_id: "composition-namespace".to_string(),
            generation: INITIAL_GENERATION,
            object_classes: vec![molten_core::fabric_durability::DurableObjectClass::EffectTransaction],
            max_operations: PROFILE_LIMIT,
            max_bytes: OPERATION_BYTE_LIMIT,
            supported_levels: vec![molten_core::fabric_durability::DurabilityLevel::Buffered, molten_core::fabric_durability::DurabilityLevel::ProcessLoss],
        },
        retention_authority_ref: Some(HASH_D.to_string()),
        quota_bytes: NAMESPACE_BYTE_LIMIT,
    }
}

pub(crate) fn effect_profile() -> molten_core::fabric_durability::EffectTransactionProfile {
    molten_core::fabric_durability::EffectTransactionProfile {
        durable_reservation: true,
        exclusive: true,
        expiring: true,
        idempotent_commit: true,
        compensating_abort: true,
    }
}

pub(crate) fn resource_envelope() -> ResourceEnvelope {
    ResourceEnvelope {
        max_concurrent_callbacks: 2,
        max_queued_events: 2,
        max_inflight_bytes: 4_096,
        max_open_streams: 1,
        max_timers: 2,
        max_effect_requests: 2,
        callback_deadline_ticks: 16,
        shutdown_grace_ticks: 32,
        max_restart_attempts: MAX_RESTART_ATTEMPTS,
        overload_policy: OverloadPolicy::Reject,
    }
}

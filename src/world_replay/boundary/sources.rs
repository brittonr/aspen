//! The runtime source catalog: every existing nondeterminism path, bound to a
//! declared replay source.
//!
//! Hostcalls come from `RuntimeHostcall`, effect ports from `EffectKind`,
//! asynchronous deliveries from `SchedulerChoiceKind`, scheduler decisions from
//! the scheduler replay policy, and the clock and entropy stream from the
//! virtual time profile. Each classification is an exhaustive match, so a new
//! variant cannot reach replay without a handling decision.

use molten_core::fabric_simulation::SchedulerChoiceKind;
use molten_core::fabric_time::SchedulerReplayPolicy;
use molten_core::planning::EffectKind;
use molten_core::world_replay::*;

use crate::error::Failure;
use crate::error::Result;
use crate::runtime::RuntimeHostcall;

pub const CLOCK_VIRTUAL_TIME_SOURCE: &str = "clock:virtual-time";
pub const ENTROPY_SEEDED_STREAM_SOURCE: &str = "entropy:seeded-stream";
pub const SCHEDULER_DECISION_SOURCE: &str = "scheduler:decision";

const RUNTIME_HOSTCALLS: [RuntimeHostcall; 4] = [
    RuntimeHostcall::Send,
    RuntimeHostcall::Subscribe,
    RuntimeHostcall::BlobGet,
    RuntimeHostcall::BlobPut,
];
const EFFECT_KINDS: [EffectKind; 7] = [
    EffectKind::StoreWrite,
    EffectKind::StoreDelete,
    EffectKind::TransportSend,
    EffectKind::ExecuteWorker,
    EffectKind::ClockRead,
    EffectKind::ReceiptWrite,
    EffectKind::RegistryRead,
];
const ASYNC_CHOICES: [SchedulerChoiceKind; 6] = [
    SchedulerChoiceKind::Runnable,
    SchedulerChoiceKind::MessageDelivery,
    SchedulerChoiceKind::TimerFire,
    SchedulerChoiceKind::StorageCompletion,
    SchedulerChoiceKind::ProcessLifecycle,
    SchedulerChoiceKind::FaultActivation,
];

const fn source(
    kind: ReplaySourceKind,
    handling: ReplaySourceHandling,
    is_external_mutation: bool,
) -> (ReplaySourceKind, ReplaySourceHandling, bool) {
    (kind, handling, is_external_mutation)
}

/// The source of a runtime hostcall: its capability name.
pub fn hostcall_source(hostcall: RuntimeHostcall) -> ReplaySource {
    use ReplaySourceHandling::RecordedObservation;
    let (kind, handling, is_external_mutation) = match hostcall {
        RuntimeHostcall::Send | RuntimeHostcall::Subscribe | RuntimeHostcall::BlobPut => {
            source(ReplaySourceKind::Hostcall, RecordedObservation, true)
        }
        RuntimeHostcall::BlobGet => source(ReplaySourceKind::Hostcall, RecordedObservation, false),
    };
    row(hostcall.capability_name(), kind, handling, is_external_mutation)
}

/// The source of a typed effect port.
pub fn effect_source(effect: EffectKind) -> ReplaySource {
    use ReplaySourceHandling::RecordedObservation;
    use ReplaySourceHandling::Simulated;
    let (identifier, (kind, handling, is_external_mutation)) = match effect {
        EffectKind::StoreWrite => {
            ("effect:store-write", source(ReplaySourceKind::EffectPort, RecordedObservation, true))
        }
        EffectKind::StoreDelete => {
            ("effect:store-delete", source(ReplaySourceKind::EffectPort, RecordedObservation, true))
        }
        EffectKind::TransportSend => {
            ("effect:transport-send", source(ReplaySourceKind::EffectPort, RecordedObservation, true))
        }
        EffectKind::ExecuteWorker => {
            ("effect:execute-worker", source(ReplaySourceKind::EffectPort, RecordedObservation, true))
        }
        EffectKind::ClockRead => ("effect:clock-read", source(ReplaySourceKind::Clock, Simulated, false)),
        EffectKind::ReceiptWrite => {
            ("effect:receipt-write", source(ReplaySourceKind::EffectPort, RecordedObservation, true))
        }
        EffectKind::RegistryRead => {
            ("effect:registry-read", source(ReplaySourceKind::ExternalRead, RecordedObservation, false))
        }
    };
    row(identifier, kind, handling, is_external_mutation)
}

/// The source of one asynchronous delivery or scheduler choice kind.
pub fn async_source(choice: SchedulerChoiceKind, policy: SchedulerReplayPolicy) -> ReplaySource {
    use ReplaySourceHandling::RecordedObservation;
    use ReplaySourceHandling::Simulated;
    let (kind, handling, is_external_mutation) = match choice {
        SchedulerChoiceKind::Runnable => source(ReplaySourceKind::SchedulerDecision, scheduler_handling(policy), false),
        SchedulerChoiceKind::MessageDelivery
        | SchedulerChoiceKind::StorageCompletion
        | SchedulerChoiceKind::ProcessLifecycle => source(ReplaySourceKind::AsyncEvent, RecordedObservation, false),
        SchedulerChoiceKind::TimerFire | SchedulerChoiceKind::FaultActivation => {
            source(ReplaySourceKind::AsyncEvent, Simulated, false)
        }
    };
    row(&format!("async:{}", choice.as_str()), kind, handling, is_external_mutation)
}

const fn scheduler_handling(policy: SchedulerReplayPolicy) -> ReplaySourceHandling {
    match policy {
        SchedulerReplayPolicy::Deterministic => ReplaySourceHandling::Deterministic,
        SchedulerReplayPolicy::RecordedChoiceRequired => ReplaySourceHandling::RecordedObservation,
    }
}

fn row(
    identifier: &str,
    kind: ReplaySourceKind,
    handling: ReplaySourceHandling,
    is_external_mutation: bool,
) -> ReplaySource {
    ReplaySource {
        source_id: identifier.to_string(),
        kind,
        handling,
        is_external_mutation,
        is_sensitive: false,
    }
}

/// Every runtime source, in canonical identifier order.
// r[impl molten.world_replay_boundary.inventory]
pub fn runtime_replay_sources(policy: SchedulerReplayPolicy) -> Vec<ReplaySource> {
    let mut sources = Vec::with_capacity(RUNTIME_HOSTCALLS.len() + EFFECT_KINDS.len() + ASYNC_CHOICES.len() + 3);
    sources.extend(RUNTIME_HOSTCALLS.into_iter().map(hostcall_source));
    sources.extend(EFFECT_KINDS.into_iter().map(effect_source));
    sources.extend(ASYNC_CHOICES.into_iter().map(|choice| async_source(choice, policy)));
    sources.push(row(CLOCK_VIRTUAL_TIME_SOURCE, ReplaySourceKind::Clock, ReplaySourceHandling::Simulated, false));
    sources.push(row(ENTROPY_SEEDED_STREAM_SOURCE, ReplaySourceKind::Entropy, ReplaySourceHandling::Simulated, false));
    sources.push(row(
        SCHEDULER_DECISION_SOURCE,
        ReplaySourceKind::SchedulerDecision,
        scheduler_handling(policy),
        false,
    ));
    sources.sort_by(|left, right| left.source_id.cmp(&right.source_id));
    sources
}

/// The identities a runtime inventory binds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeInventoryInput<'a> {
    pub profile_ref: &'a str,
    pub adapter_cohort_ref: &'a str,
    pub protection_profile_ref: &'a str,
    pub opaque_diagnostic_cohort_ref: Option<&'a str>,
    pub scheduler_policy: SchedulerReplayPolicy,
    /// Sources whose recorded values must be stored protected.
    pub sensitive_source_ids: &'a [&'a str],
}

/// Build and identify the closed runtime inventory of one replay profile.
// r[impl molten.world_replay_boundary.inventory]
pub fn runtime_replay_inventory(input: &RuntimeInventoryInput<'_>) -> Result<ReplaySourceInventory> {
    let mut sources = runtime_replay_sources(input.scheduler_policy);
    for source in &mut sources {
        source.is_sensitive = input.sensitive_source_ids.contains(&source.source_id.as_str());
    }
    let mut inventory = ReplaySourceInventory {
        schema: REPLAY_SOURCE_INVENTORY_SCHEMA.to_string(),
        inventory_ref: String::new(),
        profile_ref: input.profile_ref.to_string(),
        adapter_cohort_ref: input.adapter_cohort_ref.to_string(),
        protection_profile_ref: input.protection_profile_ref.to_string(),
        opaque_diagnostic_cohort_ref: input.opaque_diagnostic_cohort_ref.map(ToString::to_string),
        sources,
    };
    inventory.inventory_ref = identify_replay_source_inventory(&inventory)
        .map_err(|issue| Failure::invalid_harness(format!("replay source inventory identity failed: {issue:?}")))?;
    let blockers = validate_replay_source_inventory(&inventory);
    if !blockers.is_empty() {
        return Err(Failure::invalid_harness(format!("replay source inventory is not closed: {blockers:?}")));
    }
    Ok(inventory)
}

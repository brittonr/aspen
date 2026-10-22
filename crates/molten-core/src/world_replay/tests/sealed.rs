//! A complete observation boundary for the replay fixtures.
//!
//! The inventory covers each handling class and an unreached unsupported
//! hostcall. The ledger records one sensitive external read, two asynchronous
//! deliveries whose wall-clock arrivals run opposite to their logical order,
//! and one sealed external send.

use super::super::*;
use super::fixture::digest;

pub(crate) const CLOCK: &str = "clock:virtual-time";
pub(crate) const ENTROPY: &str = "entropy:seeded-stream";
pub(crate) const SCHEDULER: &str = "scheduler:decision";
pub(crate) const DELIVERY: &str = "async:message-delivery";
pub(crate) const SEND: &str = "effect:transport-send";
pub(crate) const REGISTRY: &str = "read:registry";
pub(crate) const BLOB_PUT: &str = "hostcall:blob-put";

fn source(
    source_id: &str,
    kind: ReplaySourceKind,
    handling: ReplaySourceHandling,
    is_external_mutation: bool,
    is_sensitive: bool,
) -> ReplaySource {
    ReplaySource {
        source_id: source_id.to_string(),
        kind,
        handling,
        is_external_mutation,
        is_sensitive,
    }
}

pub(crate) fn sealed_inventory(trace: &WorldTransitionTrace) -> ReplaySourceInventory {
    let mut sources = vec![
        source(CLOCK, ReplaySourceKind::Clock, ReplaySourceHandling::Simulated, false, false),
        source(ENTROPY, ReplaySourceKind::Entropy, ReplaySourceHandling::Simulated, false, false),
        source(SCHEDULER, ReplaySourceKind::SchedulerDecision, ReplaySourceHandling::Deterministic, false, false),
        source(DELIVERY, ReplaySourceKind::AsyncEvent, ReplaySourceHandling::RecordedObservation, false, false),
        source(SEND, ReplaySourceKind::EffectPort, ReplaySourceHandling::RecordedObservation, true, false),
        source(REGISTRY, ReplaySourceKind::ExternalRead, ReplaySourceHandling::RecordedObservation, false, true),
        source(BLOB_PUT, ReplaySourceKind::Hostcall, ReplaySourceHandling::Unsupported, true, false),
    ];
    sources.sort_by(|left, right| left.source_id.cmp(&right.source_id));
    reidentify_inventory(ReplaySourceInventory {
        schema: REPLAY_SOURCE_INVENTORY_SCHEMA.to_string(),
        inventory_ref: String::new(),
        profile_ref: trace.profile.profile_ref.as_str().to_string(),
        adapter_cohort_ref: digest("adapter-cohort"),
        protection_profile_ref: digest("protection-profile"),
        opaque_diagnostic_cohort_ref: Some(digest("chaoscontrol-native-cohort")),
        sources,
    })
}

pub(crate) fn reidentify_inventory(mut inventory: ReplaySourceInventory) -> ReplaySourceInventory {
    inventory.inventory_ref = identify_replay_source_inventory(&inventory).expect("inventory identity");
    inventory
}

pub(crate) fn slot(
    step_position: u64,
    logical_position: u64,
    source_id: &str,
    operation: &str,
) -> ReplayObservationSlot {
    ReplayObservationSlot {
        step_position,
        logical_position,
        source_id: source_id.to_string(),
        operation_id: operation.to_string(),
        request_ref: digest(&format!("{operation}-request")),
    }
}

pub(crate) fn record(
    inventory: &ReplaySourceInventory,
    slot: ReplayObservationSlot,
    arrival_observation: Option<u64>,
) -> RecordedReplayObservation {
    let is_sensitive = inventory.source(&slot.source_id).is_some_and(|source| source.is_sensitive);
    let protection = if is_sensitive {
        WorldReplayMemberProtection::Ciphertext {
            descriptor_ref: digest("observation-encryption"),
        }
    } else {
        WorldReplayMemberProtection::Public
    };
    reidentify_observation(RecordedReplayObservation {
        observation_ref: String::new(),
        outcome: ReplayObservationOutcome::Result {
            result_ref: digest(&format!("{}-result", slot.operation_id)),
            byte_length: 32,
        },
        slot,
        protection,
        protection_profile_ref: inventory.protection_profile_ref.clone(),
        adapter_cohort_ref: inventory.adapter_cohort_ref.clone(),
        arrival_observation,
    })
}

pub(crate) fn reidentify_observation(mut observation: RecordedReplayObservation) -> RecordedReplayObservation {
    observation.observation_ref = identify_recorded_replay_observation(&observation).expect("observation identity");
    observation
}

pub(crate) fn reidentify_ledger(mut ledger: ReplayObservationLedger) -> ReplayObservationLedger {
    ledger.ledger_ref = identify_replay_observation_ledger(&ledger).expect("ledger identity");
    ledger
}

pub(crate) fn sealed_boundary(trace: &WorldTransitionTrace) -> WorldReplayObservationBoundary {
    let inventory = sealed_inventory(trace);
    let first = trace.steps[0].position;
    let second = trace.steps[1].position;
    let slots = vec![
        slot(first, 0, REGISTRY, "read-1"),
        slot(first, 1, DELIVERY, "delivery-1"),
        slot(second, 0, SEND, "send-1"),
        slot(second, 1, DELIVERY, "delivery-2"),
    ];
    let arrivals = [None, Some(900), None, Some(100)];
    let observations = slots
        .iter()
        .cloned()
        .zip(arrivals)
        .map(|(slot, arrival)| record(&inventory, slot, arrival))
        .collect();
    let mut reached_sources = vec![CLOCK, ENTROPY, SCHEDULER, DELIVERY, SEND, REGISTRY]
        .into_iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    reached_sources.sort();
    let ledger = reidentify_ledger(ReplayObservationLedger {
        schema: REPLAY_OBSERVATION_LEDGER_SCHEMA.to_string(),
        ledger_ref: String::new(),
        inventory_ref: inventory.inventory_ref.clone(),
        trace_ref: trace.trace_ref.clone(),
        reached_sources,
        slots,
        observations,
    });
    WorldReplayObservationBoundary {
        inventory,
        ledger,
        opaque_diagnostics: Vec::new(),
    }
}

/// The capsule members a boundary adds to the fixture closure.
pub(crate) fn with_boundary_closure(
    additional: &[WorldReplayClosureRequirement],
    boundary: &WorldReplayObservationBoundary,
) -> Vec<WorldReplayClosureRequirement> {
    let mut requirements = additional.to_vec();
    requirements.extend(required_world_replay_boundary_closure(boundary));
    requirements
}

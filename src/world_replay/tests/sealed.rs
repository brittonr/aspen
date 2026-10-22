use std::collections::BTreeMap;

use molten_core::fabric_time::SchedulerReplayPolicy;
use molten_core::world_replay::*;

use super::super::*;
use super::fixture::*;
use super::ports::*;
use super::run::run_with_transitions;

const REGISTRY_READ: &str = "effect:registry-read";
const TRANSPORT_SEND: &str = "effect:transport-send";
const MESSAGE_DELIVERY: &str = "async:message-delivery";

/// The fixture boundary over the real runtime catalog: a protected registry
/// read in the first step, then a delivery and a sealed send in the second.
pub(super) fn fixture_boundary(trace: &WorldTransitionTrace) -> WorldReplayObservationBoundary {
    let adapter_cohort_ref = digest("replay-adapter-cohort");
    let protection_profile_ref = digest("replay-protection");
    let inventory = runtime_replay_inventory(&RuntimeInventoryInput {
        profile_ref: trace.profile.profile_ref.as_str(),
        adapter_cohort_ref: &adapter_cohort_ref,
        protection_profile_ref: &protection_profile_ref,
        opaque_diagnostic_cohort_ref: None,
        scheduler_policy: SchedulerReplayPolicy::Deterministic,
        sensitive_source_ids: &[REGISTRY_READ],
    })
    .expect("runtime inventory");
    let (first, second) = (trace.steps[0].position, trace.steps[1].position);
    let slots = vec![
        fixture_slot(first, 0, REGISTRY_READ, "registry-read-1"),
        fixture_slot(second, 0, MESSAGE_DELIVERY, "delivery-1"),
        fixture_slot(second, 1, TRANSPORT_SEND, "send-1"),
    ];
    let observations = slots.iter().map(|slot| fixture_observation(&inventory, slot.clone())).collect();
    let mut reached_sources = [
        CLOCK_VIRTUAL_TIME_SOURCE,
        ENTROPY_SEEDED_STREAM_SOURCE,
        SCHEDULER_DECISION_SOURCE,
        REGISTRY_READ,
        MESSAGE_DELIVERY,
        TRANSPORT_SEND,
    ]
    .map(ToString::to_string)
    .to_vec();
    reached_sources.sort();
    let mut ledger = ReplayObservationLedger {
        schema: REPLAY_OBSERVATION_LEDGER_SCHEMA.to_string(),
        ledger_ref: String::new(),
        inventory_ref: inventory.inventory_ref.clone(),
        trace_ref: trace.trace_ref.clone(),
        reached_sources,
        slots,
        observations,
    };
    ledger.ledger_ref = identify_replay_observation_ledger(&ledger).expect("ledger identity");
    WorldReplayObservationBoundary {
        inventory,
        ledger,
        opaque_diagnostics: Vec::new(),
    }
}

fn fixture_slot(step_position: u64, logical_position: u64, source_id: &str, operation: &str) -> ReplayObservationSlot {
    ReplayObservationSlot {
        step_position,
        logical_position,
        source_id: source_id.to_string(),
        operation_id: operation.to_string(),
        request_ref: digest(&format!("{operation}-request")),
    }
}

fn fixture_observation(inventory: &ReplaySourceInventory, slot: ReplayObservationSlot) -> RecordedReplayObservation {
    let is_sensitive = inventory.source(&slot.source_id).is_some_and(|source| source.is_sensitive);
    let mut observation = RecordedReplayObservation {
        observation_ref: String::new(),
        outcome: ReplayObservationOutcome::Result {
            result_ref: digest(&format!("{}-result", slot.operation_id)),
            byte_length: 16,
        },
        slot,
        protection: if is_sensitive {
            WorldReplayMemberProtection::Ciphertext {
                descriptor_ref: digest("replay-observation-key"),
            }
        } else {
            WorldReplayMemberProtection::Public
        },
        protection_profile_ref: inventory.protection_profile_ref.clone(),
        adapter_cohort_ref: inventory.adapter_cohort_ref.clone(),
        arrival_observation: None,
    };
    observation.observation_ref = identify_recorded_replay_observation(&observation).expect("observation identity");
    observation
}

/// The demands a faithful transition makes, by step.
pub(super) fn faithful_demands(
    boundary: &WorldReplayObservationBoundary,
) -> BTreeMap<u64, Vec<ReplayObservationDemand>> {
    let mut demands = BTreeMap::<u64, Vec<ReplayObservationDemand>>::new();
    for slot in &boundary.ledger.slots {
        demands.entry(slot.step_position).or_default().push(ReplayObservationDemand {
            step_position: slot.step_position,
            logical_position: slot.logical_position,
            source_id: slot.source_id.clone(),
            operation_id: slot.operation_id.clone(),
            request_ref: slot.request_ref.clone(),
        });
    }
    demands
}

// r[verify molten.world_replay_boundary.inventory]
#[test]
fn the_runtime_catalog_binds_every_hostcall_effect_and_async_kind_once() {
    let sources = runtime_replay_sources(SchedulerReplayPolicy::RecordedChoiceRequired);
    let identifiers = sources.iter().map(|source| source.source_id.as_str()).collect::<Vec<_>>();
    let mut unique = identifiers.clone();
    unique.dedup();
    assert_eq!(identifiers, unique, "canonical order without duplicates");
    for expected in [
        "hostcall:send",
        "hostcall:subscribe",
        "hostcall:blob-get",
        "hostcall:blob-put",
        "effect:store-write",
        "effect:clock-read",
        "async:message-delivery",
        "async:timer-fire",
        CLOCK_VIRTUAL_TIME_SOURCE,
        ENTROPY_SEEDED_STREAM_SOURCE,
        SCHEDULER_DECISION_SOURCE,
    ] {
        assert!(identifiers.contains(&expected), "{expected} is declared");
    }
    assert_eq!(sources.len(), 4 + 7 + 6 + 3, "every hostcall, effect kind, async kind, clock, entropy, scheduler");
    for source in sources.iter().filter(|source| source.is_external_mutation) {
        assert_eq!(source.handling, ReplaySourceHandling::RecordedObservation, "{} is sealed", source.source_id);
    }
    let scheduler = sources.iter().find(|source| source.source_id == SCHEDULER_DECISION_SOURCE).expect("scheduler");
    assert_eq!(scheduler.handling, ReplaySourceHandling::RecordedObservation, "recorded choices are observations");
}

// r[verify molten.world_replay_boundary.effect_sealing]
// r[verify molten.world_replay_boundary.verification]
#[test]
fn replay_returns_sealed_observations_and_never_repeats_the_send() {
    let fixture = fixture(WorldReplayProfileKind::Logical);
    let mut transitions = Transitions::demanding(faithful_demands(&fixture.request.boundary));
    let outcome = run_with_transitions(&fixture, &mut transitions).expect("sealed replay");
    assert_eq!(outcome.receipt.decision, WorldReplayReceiptDecision::Replayed);
    let served = fixture
        .request
        .boundary
        .ledger
        .observations
        .iter()
        .map(|observation| observation.observation_ref.clone());
    assert_eq!(transitions.served, served.collect::<Vec<_>>(), "observations are served in logical order");
    assert_eq!(outcome.receipt.boundary.sealed_effects, 1, "the recorded send is returned sealed");
    assert_eq!(outcome.receipt.boundary.repeated_effect_attempts, 0);
    assert_eq!(outcome.receipt.boundary.claims, vec![ReplayBoundaryClaim::LogicalReplayForProfile]);
    assert_eq!(outcome.plan.ledger_ref, fixture.request.boundary.ledger.ledger_ref);
}

// r[verify molten.world_replay_boundary.effect_sealing]
#[test]
fn an_adapter_that_attempts_the_original_effect_fails_the_replay() {
    let fixture = fixture(WorldReplayProfileKind::Logical);
    let mut transitions = Transitions::demanding(faithful_demands(&fixture.request.boundary));
    transitions.attempt_effect = Some(TRANSPORT_SEND.to_string());
    let error = run_with_transitions(&fixture, &mut transitions).expect_err("an attempted effect fails replay");
    assert!(error.to_string().contains("denied the original effect"), "{error}");
}

// r[verify molten.world_replay_boundary.completeness]
// r[verify molten.world_replay_boundary.ordering]
#[test]
fn an_unknown_reordered_misbound_or_skipped_demand_fails_the_replay() {
    let fixture = fixture(WorldReplayProfileKind::Logical);
    let second = fixture.request.trace.steps[1].position;
    let mut cases = Vec::new();
    let mut unknown = faithful_demands(&fixture.request.boundary);
    unknown.get_mut(&second).expect("second step")[0].source_id = "hostcall:undeclared".to_string();
    cases.push(("unknown source", unknown));
    let mut reordered = faithful_demands(&fixture.request.boundary);
    reordered.get_mut(&second).expect("second step").reverse();
    cases.push(("out of logical order", reordered));
    let mut misbound = faithful_demands(&fixture.request.boundary);
    misbound.get_mut(&second).expect("second step")[1].request_ref = digest("another-request");
    cases.push(("another request", misbound));
    let mut skipped = faithful_demands(&fixture.request.boundary);
    skipped.get_mut(&second).expect("second step").pop();
    cases.push(("unconsumed", skipped));
    for (expected, demands) in cases {
        let mut transitions = Transitions::demanding(demands);
        let error = run_with_transitions(&fixture, &mut transitions).expect_err(expected);
        assert!(error.to_string().contains(expected), "{expected}: {error}");
    }
}

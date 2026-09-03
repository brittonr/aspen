use molten_core::addressable_actor::*;

use super::super::*;
use super::support::*;

const SIMULATION_EFFECT_COUNT: usize = 2;

// r[verify molten.addressable_actor.verification]
#[test]
fn deterministic_simulation_replays_sleep_wake_lifecycle_exactly() {
    let profile = profile();
    let actor_key = actor_key();
    let reason = message_wake();
    let wake_ref = identify_actor_wake(&reason);
    let steps = vec![
        ActorSimulationStep {
            operation_id: "wake-message".to_string(),
            logical_tick: INITIAL_TICK,
            operation: ActorOperation::Wake { reason },
        },
        ActorSimulationStep {
            operation_id: "start-complete".to_string(),
            logical_tick: INITIAL_TICK,
            operation: ActorOperation::StartSucceeded { wake_ref },
        },
    ];
    let script = vec![ActorEffectDisposition::Succeeded; SIMULATION_EFFECT_COUNT];
    let first = simulate_actor_sequence(
        &profile,
        &actor_key,
        &reference("system-extension-manifest"),
        &reference("placement"),
        &steps,
        &script,
    )
    .expect("first simulation");
    let second = simulate_actor_sequence(
        &profile,
        &actor_key,
        &reference("system-extension-manifest"),
        &reference("placement"),
        &steps,
        &script,
    )
    .expect("second simulation");

    assert_eq!(first.receipt_refs, second.receipt_refs);
    assert_eq!(first.final_state, second.final_state);
    assert_eq!(first.final_state.state.phase, ActorPhase::Running);
    assert!(first.deterministic);
    assert!(!first.authorizes_production);
}

#[test]
fn simulation_preserves_unknown_effect_without_automatic_follow_up() {
    let profile = profile();
    let actor_key = actor_key();
    let steps = [ActorSimulationStep {
        operation_id: "wake-message".to_string(),
        logical_tick: INITIAL_TICK,
        operation: ActorOperation::Wake { reason: message_wake() },
    }];
    let report = simulate_actor_sequence(
        &profile,
        &actor_key,
        &reference("system-extension-manifest"),
        &reference("placement"),
        &steps,
        &[ActorEffectDisposition::Unknown],
    )
    .expect("unknown simulation");

    assert_eq!(report.effect_observations.len(), 1);
    assert_eq!(report.final_state.state.phase, ActorPhase::Degraded);
    assert!(report.final_state.state.unknown_effect_ref.is_some());
    assert!(!report.authorizes_production);
}

//! Bounded Hegel property for sealed replay observations: a denied demand never consumes the slot
//! it named, and any denial fails its step even when the transition carries on.

use molten_core::world_replay::*;

use super::super::*;
use super::fixture::*;
use super::sealed::faithful_demands;

const MAX_MISBOUND_DEMANDS: u32 = 3;

// r[verify molten.world_commit.replay_sealed_observation_and_full_comparison]
#[hegel::test(test_cases = 32)]
fn hegel_misbound_demands_never_consume_a_slot_and_always_fail_the_step(tc: hegel::TestCase) {
    let fixture = fixture(WorldReplayProfileKind::Logical);
    let first = fixture.request.trace.steps[0].position;
    let bound = faithful_demands(&fixture.request.boundary).get(&first).expect("first step")[0].clone();
    let misbound_count = tc.draw(hegel::generators::integers::<u32>().min_value(0).max_value(MAX_MISBOUND_DEMANDS));
    let is_operation_mismatch = tc.draw(hegel::generators::booleans());

    let mut observations =
        SealedReplayObservations::new(&fixture.request.boundary, &fixture.request.trace).expect("sealed");
    for attempt in 0..misbound_count {
        let mut misbound = bound.clone();
        if is_operation_mismatch {
            misbound.operation_id = format!("operation:misbound-{attempt}");
        } else {
            misbound.request_ref = digest(&format!("misbound-request-{attempt}"));
        }
        observations.observe(&misbound).expect_err("a misbound demand is denied");
    }
    assert_eq!(observations.demand_violations(), misbound_count);

    let served = observations.observe(&bound).expect("the slot stays pending for its true demand");
    assert_eq!(served.slot.request_ref, bound.request_ref);
    assert_eq!(observations.finish_step(first).is_ok(), misbound_count == 0);
}

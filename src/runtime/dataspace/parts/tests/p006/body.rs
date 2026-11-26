// Bounded Hegel properties for recorded clock and random responses and for force-run demand
// precedence. They reuse the fixtures of the determinism regressions in `p005`.

const PROPERTY_MAX_BOUNDARY_OFFSET: u64 = 3;
const PROPERTY_MAX_RANDOM_UPPER: u64 = 1_000;
const PROPERTY_MAX_DEMANDS: usize = 6;

/// A recorded clock value near the current time or near the restorable maximum, where regression
/// and exhaustion live.
fn draw_recorded_clock_value(tc: &hegel::TestCase, logical_time: u64) -> u64 {
    let offset = tc.draw(hegel::generators::integers::<u64>().min_value(0).max_value(PROPERTY_MAX_BOUNDARY_OFFSET));
    match tc.draw(hegel::generators::integers::<u8>().min_value(0).max_value(2)) {
        0 => logical_time.saturating_sub(offset),
        1 => logical_time.saturating_add(offset),
        _ => u64::MAX - offset,
    }
}

#[hegel::test(test_cases = 64)]
fn hegel_recorded_clock_responses_never_regress_or_exhaust_logical_time(tc: hegel::TestCase) {
    // r[verify molten.runtime_spine.dataspace_determinism_and_rollback_integrity]
    let is_near_maximum = tc.draw(hegel::generators::booleans());
    let logical_time = if is_near_maximum {
        u64::MAX - 2 - tc.draw(hegel::generators::integers::<u64>().min_value(0).max_value(PROPERTY_MAX_BOUNDARY_OFFSET))
    } else {
        tc.draw(hegel::generators::integers::<u64>().min_value(0).max_value(1_000_000))
    };
    let recorded = draw_recorded_clock_value(&tc, logical_time);
    let mut state = determinism_state_with_counters(logical_time, 0);
    let request = state
        .begin_effect_for_step(&Step::Clock {
            actor: "clock-actor".into(),
        })
        .expect("checked sequence")
        .expect("clock request");
    let before = state.snapshot();

    let replayed = recorded_effect_response_transition(&before, &request, recorded);
    let is_admissible = recorded >= logical_time && recorded <= u64::MAX - 2;
    assert_eq!(replayed.is_ok(), is_admissible, "recorded {recorded} at logical time {logical_time}");
    if let Ok(replayed) = replayed {
        assert_eq!(replayed.after.logical_time, recorded + 1);
        RuntimeState::from_snapshot(replayed.after).expect("an admitted replay stays restorable");
    }
}

#[hegel::test(test_cases = 64)]
fn hegel_recorded_random_values_replay_only_inside_their_range(tc: hegel::TestCase) {
    // r[verify molten.runtime_spine.dataspace_determinism_and_rollback_integrity]
    let upper = tc.draw(hegel::generators::integers::<u64>().min_value(1).max_value(PROPERTY_MAX_RANDOM_UPPER));
    let offset = tc.draw(hegel::generators::integers::<u64>().min_value(0).max_value(PROPERTY_MAX_BOUNDARY_OFFSET));
    let recorded = if tc.draw(hegel::generators::booleans()) {
        upper.saturating_sub(offset + 1)
    } else {
        upper + offset
    };
    let mut state = RuntimeState::new(DETERMINISM_SEED);
    let request = state
        .begin_effect_for_step(&Step::Random {
            actor: "random-actor".into(),
            upper,
        })
        .expect("checked sequence")
        .expect("random request");
    let before = state.snapshot();

    let replayed = recorded_effect_response_transition(&before, &request, recorded);
    assert_eq!(replayed.is_ok(), recorded < upper, "recorded {recorded} for range 0..{upper}");
}

fn property_demand(kind: ServiceDemandKind, index: usize) -> Assertion {
    Assertion {
        actor: DETERMINISM_DEMANDER_REF.to_string(),
        value: ServiceDemandAssertion {
            service_ref: DETERMINISM_SERVICE_REF.to_string(),
            kind,
            demander_ref: DETERMINISM_DEMANDER_REF.to_string(),
            operation_identity: format!("operation:property-{index}"),
        }
        .to_assertion_value()
        .expect("demand value"),
        facet: None,
    }
}

#[hegel::test(test_cases = 64)]
fn hegel_force_run_outranks_dependency_gated_in_every_order(tc: hegel::TestCase) {
    // r[verify molten.runtime_spine.dataspace_determinism_and_rollback_integrity]
    let demand_count = tc.draw(hegel::generators::integers::<usize>().min_value(1).max_value(PROPERTY_MAX_DEMANDS));
    let mut assertions = Vec::with_capacity(demand_count);
    let mut has_force_run = false;
    for index in 0..demand_count {
        let is_force_run = tc.draw(hegel::generators::booleans());
        has_force_run |= is_force_run;
        let kind = if is_force_run {
            ServiceDemandKind::ForceRun
        } else {
            ServiceDemandKind::DependencyGated
        };
        assertions.push(property_demand(kind, index));
    }

    let demands = ServiceDemandSet::derive(&assertions).expect("demand set");
    let expected = if has_force_run {
        vec![DETERMINISM_SERVICE_REF.to_string()]
    } else {
        Vec::new()
    };
    assert_eq!(demands.force_run_service_refs(), expected);
}

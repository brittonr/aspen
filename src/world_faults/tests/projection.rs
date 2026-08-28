use molten_core::world_faults::*;

use super::support::TEST_SOURCE_REVISION;

// r[verify molten.world_faults.profile]
#[test]
fn nickel_profile_is_an_exact_checked_projection_of_the_rust_profile() {
    let value = serde_json::from_str::<serde_json::Value>(include_str!(
        "../../../config/world-faults/generated/local-deterministic.json"
    ))
    .expect("generated world fault profile JSON");
    let profile = standard_world_fault_profile(TEST_SOURCE_REVISION).expect("Rust world fault profile");
    assert_eq!(json_string(&value, "schema"), profile.schema);
    assert_eq!(json_string(&value, "profile_name"), profile.profile_name);
    assert_eq!(json_string(&value, "source_revision"), profile.source_revision);
    assert_eq!(json_string(&value, "inventory_ref"), profile.inventory_ref);
    assert_eq!(
        json_string(&value, "rust_profile_ref"),
        identify_world_fault_profile(&profile).expect("Rust profile identity")
    );
    assert_limits(&value, profile.limits);
    assert_adapters(&value, &profile.adapters);
    assert_cases(&value, &profile.cases);
    assert_schedules(&value, &profile.schedules);
    assert_eq!(
        value
            .get("unexplained_numeric_thresholds")
            .and_then(serde_json::Value::as_array)
            .expect("unexplained thresholds")
            .len(),
        0
    );
    assert_eq!(value.get("claims_independent_witness").and_then(serde_json::Value::as_bool), Some(false));
    assert_eq!(value.get("claims_physical_power_loss").and_then(serde_json::Value::as_bool), Some(false));
}

fn assert_limits(value: &serde_json::Value, expected: WorldFaultLimits) {
    let limits = value.get("limits").expect("limits");
    assert_eq!(json_usize(limits, "max_cases"), expected.max_cases);
    assert_eq!(json_usize(limits, "max_schedules"), expected.max_schedules);
    assert_eq!(json_usize(limits, "max_schedule_steps"), expected.max_schedule_steps);
    assert_eq!(json_usize(limits, "max_adapters"), expected.max_adapters);
    assert_eq!(json_usize(limits, "max_observations"), expected.max_observations);
    assert_eq!(json_usize(limits, "max_unsupported_rows"), expected.max_unsupported_rows);
    assert_eq!(json_u64(limits, "max_restarts"), u64::from(expected.max_restarts));
}

fn assert_adapters(value: &serde_json::Value, expected_adapters: &[WorldFaultAdapterBinding]) {
    let adapters = value.get("adapters").and_then(serde_json::Value::as_array).expect("adapters");
    assert_eq!(adapters.len(), expected_adapters.len());
    for expected in expected_adapters {
        let observed = adapters
            .iter()
            .find(|adapter| json_string(adapter, "adapter_id") == expected.adapter_id)
            .expect("projected adapter");
        assert_eq!(json_string(observed, "owner"), expected.owner.as_str());
        assert_eq!(json_string(observed, "profile"), expected.profile);
        assert_eq!(json_string(observed, "implementation_ref"), expected.implementation_ref);
        assert_eq!(json_string(observed, "semantic_phase_map_ref"), expected.semantic_phase_map_ref);
    }
}

fn assert_cases(value: &serde_json::Value, expected_cases: &[WorldFaultCase]) {
    let cases = value.get("cases").and_then(serde_json::Value::as_array).expect("cases");
    assert_eq!(cases.len(), expected_cases.len());
    for expected in expected_cases {
        let observed =
            cases.iter().find(|case| json_string(case, "case_id") == expected.case_id).expect("projected case");
        assert_eq!(json_string(observed, "mutation"), expected.mutation.as_str());
        assert_eq!(json_string(observed, "operation_id"), expected.operation_id);
        assert_eq!(json_string(observed, "phase"), expected.phase.as_str());
        assert_eq!(json_string(observed, "adapter_id"), expected.adapter_id);
        assert_eq!(json_u64(observed, "expected_generation"), expected.expected_generation);
        assert_eq!(json_string(observed, "pre_state_ref"), expected.pre_state_ref);
        assert_eq!(json_string(observed, "expected_decision"), expected.expected_decision.as_str());
    }
}

fn assert_schedules(value: &serde_json::Value, expected_schedules: &[ConcurrentSchedule]) {
    let schedules = value.get("schedules").and_then(serde_json::Value::as_array).expect("schedules");
    assert_eq!(schedules.len(), expected_schedules.len());
    for expected in expected_schedules {
        let observed = schedules
            .iter()
            .find(|schedule| json_string(schedule, "schedule_id") == expected.schedule_id)
            .expect("projected schedule");
        assert_eq!(json_string(observed, "mutation"), expected.mutation.as_str());
        let steps = observed.get("steps").and_then(serde_json::Value::as_array).expect("schedule steps");
        assert_eq!(steps.len(), expected.steps.len());
        for expected_step in &expected.steps {
            let observed_step = steps
                .iter()
                .find(|step| {
                    json_u64(step, "position") == u64::from(expected_step.position)
                        && json_string(step, "operation_id") == expected_step.operation_id
                })
                .expect("projected schedule step");
            assert_eq!(json_string(observed_step, "mutation"), expected_step.mutation.as_str());
            assert_eq!(json_u64(observed_step, "expected_generation"), expected_step.expected_generation);
            assert_eq!(json_string(observed_step, "pre_state_ref"), expected_step.pre_state_ref);
            assert_eq!(json_string(observed_step, "interleaving"), expected_step.interleaving.as_str());
            assert_eq!(json_string(observed_step, "node_id"), expected_step.node_id);
            assert_eq!(json_u64(observed_step, "node_generation"), expected_step.node_generation);
        }
    }
}

fn json_string<'a>(value: &'a serde_json::Value, field: &str) -> &'a str {
    value.get(field).and_then(serde_json::Value::as_str).expect("JSON string field")
}

fn json_u64(value: &serde_json::Value, field: &str) -> u64 {
    value.get(field).and_then(serde_json::Value::as_u64).expect("JSON u64 field")
}

fn json_usize(value: &serde_json::Value, field: &str) -> usize {
    usize::try_from(json_u64(value, field)).expect("JSON usize field")
}

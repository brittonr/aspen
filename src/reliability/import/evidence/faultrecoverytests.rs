// Fault-stage, final-read, and recovery bounds regressions.

#[test]
fn missing_endpoint_stalled_recovery_and_contradictory_final_read_block_valid() {
    let (mut expected, mut evidence) = support::fixture();
    let saved = evidence.recovery.pop().expect("endpoint");
    assert_eq!(super::import(&expected, &evidence).err(), Some(super::Issue::Recovery));
    evidence.recovery.push(saved);
    evidence.recovery[0].stable_membership = false;
    assert_eq!(super::import(&expected, &evidence).err(), Some(super::Issue::Recovery));
    evidence.recovery[0].stable_membership = true;
    evidence.recovery[0].elapsed_ms = 1001;
    assert_eq!(super::import(&expected, &evidence).err(), Some(super::Issue::Recovery));
    evidence.recovery[0].elapsed_ms = 0;
    let original = evidence.recovery[1].value.clone();
    let alternate = if original == serde_json::json!({"type":"integer","value":1}) {
        2
    } else {
        1
    };
    evidence.recovery[0].value = serde_json::json!({"type":"integer","value":alternate});
    let attempt = evidence.recovery[0].attempt_id.clone();
    let completion = evidence.history["events"]
        .as_array_mut()
        .expect("events")
        .iter_mut()
        .find(|e| e["kind"] == "complete" && e["attempt_id"] == attempt)
        .expect("completion");
    completion["output"] = evidence.recovery[0].value.clone();
    evidence.report["history_id"] = serde_json::json!(
        crate::live_reliability::checker::semantic_history_id(
            &serde_json::from_value(evidence.history.clone()).expect("history")
        )
        .expect("id")
    );
    expected.checked = support::checked(&evidence);
    assert_eq!(super::import(&expected, &evidence).err(), Some(super::Issue::Recovery));
}

#[test]
fn independently_selected_stability_and_final_read_deadlines_are_enforced() {
    let (mut expected, mut evidence) = support::fixture();
    assert_eq!(super::import(&expected, &evidence).expect("in-bound observation").status, "bounded-valid");
    expected.fault.profile.stable_membership_ms = 2;
    assert_eq!(super::import(&expected, &evidence).err(), Some(super::Issue::Recovery));
    expected.fault.profile.stable_membership_ms = 1;

    let events = evidence.history["events"].as_array_mut().expect("events");
    for event in &mut events[3..] {
        let time = event["controller_time_ns"].as_u64().expect("controller time");
        event["controller_time_ns"] = serde_json::json!(time + 2_000_000);
    }
    for read in &mut evidence.recovery {
        read.elapsed_ms = 2;
    }
    evidence.report["history_id"] = serde_json::json!(
        crate::live_reliability::checker::semantic_history_id(
            &serde_json::from_value(evidence.history.clone()).expect("history")
        )
        .expect("id")
    );
    expected.checked = support::checked(&evidence);
    expected.fault.profile.final_read_deadline_ms = 3;
    assert_eq!(super::import(&expected, &evidence).expect("in-bound deadline").status, "bounded-valid");
    expected.fault.profile.final_read_deadline_ms = 2;
    assert_eq!(super::import(&expected, &evidence).err(), Some(super::Issue::Recovery));
}

#[test]
fn selected_fault_without_observed_effect_and_teardown_gap_are_rejected() {
    let (mut expected, mut evidence) = support::fixture();
    let effect = support::reference("effect");
    expected.fault.profile.fault = crate::live_reliability::fault::Mechanism::ProcessRestart {
        endpoint: expected.admitted.profile().endpoints[0].clone(),
        down_ms: 1,
    };
    expected.fault.effect_ref = Some(effect.clone());
    assert_eq!(super::import(&expected, &evidence).err(), Some(super::Issue::Fault));
    evidence.faults.push(super::FaultObservation {
        id: "restart".into(),
        mechanism: "process-restart".into(),
        selected: 0,
        applied: 1,
        observed: 2,
        healed: 3,
        recovered: 4,
        effect_ref: effect,
    });
    assert_eq!(super::import(&expected, &evidence).err(), Some(super::Issue::Fault));
    evidence.faults.clear();
    expected.fault = support::no_fault();
    evidence.teardown = false;
    assert_eq!(super::import(&expected, &evidence).err(), Some(super::Issue::Teardown));
}

#[test]
fn observed_partition_heal_and_recovery_remain_bounded_observations() {
    let (mut expected, mut evidence) = support::fixture();
    let effect = support::reference("effect");
    expected.fault.profile.fault = crate::live_reliability::fault::Mechanism::TemporaryPartition {
        left: expected.endpoints.iter().take(1).cloned().collect(),
        right: expected.endpoints.iter().skip(1).cloned().collect(),
        duration_ms: 1,
    };
    expected.fault.effect_ref = Some(effect.clone());
    let mut events: Vec<_> = ["selected", "applied", "observed"]
        .iter()
        .map(|phase| {
            serde_json::json!({"kind":"fault","fault_attempt_id":"partition","phase":phase,
            "effect_record_ref":if *phase == "selected" { None } else { Some(effect.clone()) },
            "event_index":0,"controller_time_ns":0})
        })
        .collect();
    for phase in ["heal", "recovery"] {
        events.push(serde_json::json!({"kind":"lifecycle","phase":phase,"event_index":0,"controller_time_ns":0}));
    }
    events.extend(evidence.history["events"].as_array().expect("events").iter().cloned());
    for (index, event) in events.iter_mut().enumerate() {
        event["event_index"] = serde_json::json!(index);
        let nanos = if index < 5 {
            index as u64 * 1_000_000
        } else {
            5_000_000 + event["controller_time_ns"].as_u64().expect("controller time")
        };
        event["controller_time_ns"] = serde_json::json!(nanos);
    }
    evidence.history["events"] = serde_json::json!(events);
    evidence.history["completeness"]["applied_faults"] = serde_json::json!(1);
    evidence.history["completeness"]["observed_faults"] = serde_json::json!(1);
    evidence.report["completeness"] = evidence.history["completeness"].clone();
    evidence.report["history_id"] = serde_json::json!(
        crate::live_reliability::checker::semantic_history_id(
            &serde_json::from_value(evidence.history.clone()).expect("history")
        )
        .expect("id")
    );
    evidence.faults.push(super::FaultObservation {
        id: "partition".into(),
        mechanism: "temporary-partition".into(),
        selected: 0,
        applied: 1,
        observed: 2,
        healed: 3,
        recovered: 4,
        effect_ref: effect,
    });
    for read in &mut evidence.recovery {
        read.ready_event += 5;
        read.stable_membership_event += 5;
    }
    expected.checked = support::checked(&evidence);
    assert_eq!(super::import(&expected, &evidence).expect("recovered partition").status, "bounded-valid");
    let admitted = expected.fault.clone();
    expected.fault = support::no_fault();
    assert_eq!(super::import(&expected, &evidence).err(), Some(super::Issue::Fault));
    expected.fault = admitted.clone();
    expected.fault.effect_ref = Some(support::reference("another-provider-effect"));
    assert_eq!(super::import(&expected, &evidence).err(), Some(super::Issue::Fault));
    expected.fault = admitted.clone();
    expected.fault.profile.heal_deadline_ms = 1;
    assert_eq!(super::import(&expected, &evidence).err(), Some(super::Issue::Fault));
    expected.fault = admitted;
    evidence.non_claims.remove("release");
    assert_eq!(super::import(&expected, &evidence).err(), Some(super::Issue::Claim));
}

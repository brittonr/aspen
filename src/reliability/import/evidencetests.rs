//! Admission regressions use in-memory, prevalidated checker observations; no checker is mocked as
//! an executable.

#[path = "evidence/support.rs"]
mod support;

// r[verify molten.testing.live_reliability_evidence]
#[test]
fn complete_valid_run_has_canonical_external_receipt_without_authority() {
    let (expected, evidence) = support::fixture();
    let receipt = super::import(&expected, &evidence).expect("complete run");
    assert_eq!(receipt.status, "bounded-valid");
    assert_eq!(receipt.receipt_ref, crate::preserves_rail::canonical_hash(&receipt.value).expect("canonical ref"));
    assert_eq!(
        crate::live_reliability::comparison::admit_claim("release"),
        Err(crate::live_reliability::comparison::Issue::Claim)
    );
}

// r[verify molten.testing.live_reliability_validation]
#[test]
fn invalid_stale_or_lost_write_read_retains_failure_witness() {
    let (mut expected, mut evidence) = support::fixture();
    let read = evidence.recovery.last_mut().expect("last endpoint");
    read.value = serde_json::json!({"type":"integer","value":2});
    let attempt = read.attempt_id.clone();
    let completion = evidence.history["events"]
        .as_array_mut()
        .expect("events")
        .iter_mut()
        .find(|e| e["kind"] == "complete" && e["attempt_id"] == attempt)
        .expect("completion");
    completion["output"] = read.value.clone();
    evidence.report["verdict"] = serde_json::json!("invalid");
    evidence.report["invalid"] =
        serde_json::json!({"remaining_attempt_ids":[attempt],"model_state":"null","failure_class":"stale-read"});
    evidence.report["history_id"] = serde_json::json!(
        crate::live_reliability::checker::semantic_history_id(
            &serde_json::from_value(evidence.history.clone()).expect("history")
        )
        .expect("history id")
    );
    expected.checked = support::checked(&evidence);
    let receipt = super::import(&expected, &evidence).expect("bounded failure retained");
    assert_eq!(receipt.verdict, "invalid");
    assert_eq!(receipt.status, "external-failure");
}

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
fn changed_source_artifact_unexecuted_report_and_duplicate_operations_fail_closed() {
    let (mut expected, mut evidence) = support::fixture();
    evidence.identities.insert("artifact".into(), support::reference("stale artifact"));
    assert_eq!(super::import(&expected, &evidence).err(), Some(super::Issue::Identity));
    evidence.identities = expected.identities.clone();
    evidence.report["verdict"] = serde_json::json!("invalid");
    assert_eq!(super::import(&expected, &evidence).err(), Some(super::Issue::Checker));
    evidence.report = expected.checked.report.clone();
    let events = evidence.history["events"].as_array_mut().expect("events");
    events.push(events[2].clone());
    expected.checked = support::checked(&evidence);
    assert_eq!(super::import(&expected, &evidence).err(), Some(super::Issue::History));
}

#[test]
fn selected_fault_without_observed_effect_and_teardown_gap_are_rejected() {
    let (expected, mut evidence) = support::fixture();
    evidence.faults.push(super::FaultObservation {
        id: "restart".into(),
        mechanism: "process-restart".into(),
        selected: 0,
        applied: 1,
        observed: 2,
        healed: 3,
        recovered: 4,
        effect_ref: support::reference("effect"),
    });
    assert_eq!(super::import(&expected, &evidence).err(), Some(super::Issue::Fault));
    evidence.faults.clear();
    evidence.teardown = false;
    assert_eq!(super::import(&expected, &evidence).err(), Some(super::Issue::Teardown));
}

#[test]
fn unknown_bound_and_reference_disagreement_never_promote() {
    let (mut expected, mut evidence) = support::fixture();
    evidence.report["verdict"] = serde_json::json!("unknown");
    evidence.report["unknown"] = serde_json::json!({"class":"max_states","bound_name":"max_states","bound_value":100,"observed":101,"message":"bound"});
    expected.checked = support::checked(&evidence);
    assert_eq!(super::import(&expected, &evidence).expect("unknown retained").status, "inconclusive");
    evidence.reference_report = Some(evidence.report.clone());
    evidence.reference_report.as_mut().expect("reference")["verdict"] = serde_json::json!("valid");
    evidence.identities.insert("reference-checker".into(), support::reference("reference checker"));
    expected.identities = evidence.identities.clone();
    let mut reference_checked = support::checked(&evidence);
    reference_checked.report = evidence.reference_report.clone().expect("reference");
    reference_checked.checker_ref = evidence.identities["reference-checker"].clone();
    expected.reference_checked = Some(reference_checked);
    assert_eq!(super::import(&expected, &evidence).expect("disagreement retained").status, "checker-disagreement");
}

// r[verify molten.testing.live_reliability_claim_boundary]
#[test]
fn all_four_profiles_keep_their_own_scope_and_no_substitution() {
    let profiles: Vec<_> = [
        crate::live_reliability::comparison::Environment::Simulation,
        crate::live_reliability::comparison::Environment::NixosVm,
        crate::live_reliability::comparison::Environment::ChaoscontrolKvm,
        crate::live_reliability::comparison::Environment::OnixosLive,
    ]
    .into_iter()
    .map(|environment| crate::live_reliability::comparison::ProfileEvidence {
        environment,
        receipt_ref: support::reference("receipt"),
        corpus_ref: support::reference("corpus"),
        invariant_ref: support::reference("invariant"),
        adapter_ref: support::reference("adapter"),
        fault_mechanism: "none".into(),
        time_source: "controller".into(),
        checker_ref: support::reference("checker"),
        verdict: "valid".into(),
        non_claims: super::NON_CLAIMS.iter().map(|v| (*v).into()).collect(),
    })
    .collect();
    let comparison = crate::live_reliability::comparison::compare(&profiles).expect("distinct scopes");
    assert_eq!(comparison.profiles, profiles);
    assert!(!comparison.authority);
    let mut duplicate = profiles.clone();
    duplicate[3].environment = duplicate[0].environment;
    assert_eq!(
        crate::live_reliability::comparison::compare(&duplicate).err(),
        Some(crate::live_reliability::comparison::Issue::DuplicateEnvironment)
    );
}

#[test]
fn observed_partition_heal_and_recovery_remain_bounded_observations() {
    let (mut expected, mut evidence) = support::fixture();
    let effect = support::reference("effect");
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
        event["controller_time_ns"] = serde_json::json!(index);
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
        read.ready_event = 5;
        read.stable_membership_event = 6;
    }
    expected.checked = support::checked(&evidence);
    assert_eq!(super::import(&expected, &evidence).expect("recovered partition").status, "bounded-valid");
    evidence.non_claims.remove("release");
    assert_eq!(super::import(&expected, &evidence).err(), Some(super::Issue::Claim));
}

#[test]
fn valid_disagreement_with_missing_witness_or_overbound_report_fails_closed() {
    let (mut expected, mut evidence) = support::fixture();
    evidence.report["verdict"] = serde_json::json!("invalid");
    expected.checked = support::checked(&evidence);
    assert_eq!(super::import(&expected, &evidence).err(), Some(super::Issue::Witness));
    evidence.report["verdict"] = serde_json::json!("valid");
    expected.checked = support::checked(&evidence);
    expected.max_witness_bytes = 1;
    assert_eq!(super::import(&expected, &evidence).err(), Some(super::Issue::Witness));
}
